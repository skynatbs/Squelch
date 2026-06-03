# Squelch – Workspace Documentation
**Version:** 0.2
**Status:** In development (Phase 3 complete)
**Date:** 2026-06-02

---

## 1. Overview

Squelch is organized as a **Cargo workspace** with five independent crates. Each crate
has a single responsibility and minimal dependencies on its siblings. The structure
enforces clean separation between the signal path (Matrix), the transport (WebRTC),
the audio stack (cpal + Opus), and the application shell (egui).

---

## 2. Directory Structure

```
squelch/
├── Cargo.toml                  # Workspace root — all dependency versions live here
├── Cargo.lock
├── .gitignore
├── Justfile                    # Build, test, lint, git recipes
├── README.md
│
├── docs/
│   ├── mvp.md                  # MVP scope, success criteria, open questions
│   ├── workspace.md            # This file
│   └── adr/                    # Architecture Decision Records
│       ├── adr-0000-template.md
│       ├── adr-0001-cargo-workspace.md
│       ├── adr-0002-egui.md
│       ├── adr-0003-matrix-signaling.md
│       ├── adr-0004-license.md
│       └── adr-0005-communication-model.md
│
├── spikes/                     # Throwaway experiments — validated before building
│   ├── 001-str0m-webrtc/       # ✓ VALIDATED: str0m ICE+DTLS+Audio P2P
│   ├── 002-matrix-signaling/   # ✓ VALIDATED: Matrix to-device SDP/ICE exchange
│   └── 003-audio-mixing/       # ✓ VALIDATED: cpal dual-channel PTT-gated mixing
│
├── squelch-core/               # Shared types, squad model, error types
│   └── src/
│       ├── lib.rs
│       ├── error.rs            # SquelchError (Audio, Signaling, WebRtc, Squad)
│       └── squad.rs            # Squad, Member, Duo, Role, leadership transfer
│
├── squelch-matrix/             # Matrix signaling backend
│   └── src/
│       ├── lib.rs
│       ├── client.rs           # MatrixClient: login, room, sync loop, SyncHandle
│       ├── error.rs            # MatrixError
│       ├── event_types.rs      # io.squelch.* to-device event type constants
│       └── signaling.rs        # SdpMessage, IceCandidate, SignalingEvent
│
├── squelch-webrtc/             # WebRTC peer connections (str0m)
│   └── src/
│       ├── lib.rs
│       ├── error.rs            # WebRtcError
│       ├── peer.rs             # PeerConnection, PeerRole, SDP/ICE, run loop
│       └── mesh.rs             # PeerMesh: manages all connections for one player
│
├── squelch-audio/              # Microphone capture, Opus codec, mixing (cpal)
│   └── src/
│       ├── lib.rs
│       ├── error.rs            # AudioError
│       ├── ptt.rs              # PttState (AtomicBool, Clone, thread-safe)
│       └── pipeline.rs         # AudioPipeline: cpal streams, Opus, PTT routing
│
└── squelch-app/                # Entry point, egui UI, global PTT hotkey
    └── src/
        └── main.rs             # Binary: squelch
```

---

## 3. Dependency Graph

```
squelch-app
    ├── squelch-audio
    │       └── squelch-core
    ├── squelch-webrtc
    │       ├── squelch-audio
    │       │       └── squelch-core
    │       ├── squelch-matrix
    │       │       └── squelch-core
    │       └── squelch-core
    └── squelch-matrix
            └── squelch-core
```

**Rule:** `squelch-core` has no internal dependencies. `squelch-matrix` and
`squelch-audio` only depend on `squelch-core`. `squelch-webrtc` depends on both.
`squelch-app` wires everything together. No circular dependencies. No crate imports
`squelch-app`.

---

## 4. Workspace `Cargo.toml`

All dependency versions are declared once in `[workspace.dependencies]`. Individual
crates reference them with `{ workspace = true }` — no duplicate version pins.

```toml
[workspace.dependencies]
# Error handling
thiserror = "2"
anyhow    = "1"

# Serialization
serde      = { version = "1", features = ["derive"] }
serde_json = "1"

# Async
tokio = { version = "1", features = ["full"] }

# Logging
tracing            = "0.1"
tracing-subscriber = "0.3"

# Audio
cpal    = "0.15"
ringbuf = "0.3"
opus    = "0.3"

# WebRTC
str0m = "0.20"

# Matrix
matrix-sdk        = { version = "0.18", ... }
matrix-sdk-common = "0.18"
ruma              = { version = "0.16", features = ["client-api-c"] }

# UI
eframe = "0.31"
egui   = "0.31"
```

---

## 5. Crate Details

---

### 5.1 `squelch-core`

**Responsibility:** Shared types, squad model, and error types. No network calls,
no audio I/O. Pure data and logic. Every other crate depends on this one.

**Key types:**

```rust
// squad.rs
pub struct Squad {
    pub members: Vec<Member>,
    pub duos:    Vec<Duo>,
}

pub struct Member {
    pub id:   MemberId,  // Matrix user ID
    pub role: Role,      // Member | Leader
}

pub struct Duo {
    pub members: [MemberId; 2],
}

impl Squad {
    pub fn is_leader(&self, id: &str) -> bool { ... }
    pub fn duo_partner<'a>(&'a self, id: &str) -> Option<&'a MemberId> { ... }
    pub fn transfer_leadership(&mut self, from: &str, to: &str) -> bool { ... }
}

// error.rs
pub enum SquelchError {
    Audio(String),
    Signaling(String),
    WebRtc(String),
    Squad(String),
}
```

---

### 5.2 `squelch-matrix`

**Responsibility:** Matrix client — login, squad room management, and WebRTC
signaling via unencrypted `io.squelch.*` to-device events.

**Architecture:** `MatrixClient::start_sync()` spawns a background Tokio task
that calls `sync_once` in a loop. Incoming signaling events are dispatched as
`SignalingEvent` values through an `mpsc` channel. Dropping the returned
`SyncHandle` aborts the background task (RAII).

**Key types:**

```rust
// client.rs
pub struct MatrixClient { ... }

impl MatrixClient {
    pub async fn login(config: &MatrixConfig) -> Result<Self, MatrixError>;
    pub async fn create_squad_room(&self, name: &str, invite: &[OwnedUserId])
        -> Result<OwnedRoomId, MatrixError>;
    pub async fn join_room(&self, room_id: &OwnedRoomId) -> Result<(), MatrixError>;
    pub async fn send_sdp_offer(&self, target_user, target_device, payload)
        -> Result<(), MatrixError>;
    pub async fn send_sdp_answer(&self, ...) -> Result<(), MatrixError>;
    pub async fn send_ice_candidate(&self, ...) -> Result<(), MatrixError>;
    pub fn start_sync(&self) -> (SyncHandle, mpsc::Receiver<SignalingEvent>);
}

pub struct SyncHandle(JoinHandle<()>); // aborts task on Drop

// signaling.rs
pub struct SdpMessage    { pub call_id, room_id, sdp }
pub struct IceCandidate  { pub call_id, room_id, candidate, sdp_m_line_index }

pub enum SignalingEvent {
    SdpOffer    { from: String, payload: SdpMessage },
    SdpAnswer   { from: String, payload: SdpMessage },
    IceCandidate { from: String, payload: IceCandidate },
}
```

**Implementation notes:**
- Matrix homeserver is user-supplied (any federated server — matrix.org, self-hosted)
- SQLite session store for persistent login
- `ProcessedToDeviceEvent::PlainText` for unencrypted signaling (MVP)
- `io.squelch.*` event types are custom and do not interfere with Matrix standard events

---

### 5.3 `squelch-webrtc`

**Responsibility:** WebRTC peer connections and audio transport. One
`PeerConnection` (wrapping str0m's `Rtc`) per remote peer. Each connection
runs its own event loop in a dedicated `std::thread` (str0m is sync, not async).

**Architecture:** `PeerMesh` manages all connections for one local player. For
a 4-player squad each player maintains 3 `PeerConnection` instances. Audio flows
as encoded **Opus bytes** (`Vec<u8>`) in both directions — encoding/decoding
happens in `squelch-audio`, not here. WebRTC is purely the transport.

**Key types:**

```rust
// peer.rs
pub struct PeerConnection {
    pub remote_id:   String,
    pub role:        PeerRole,      // Offerer | Answerer
    pub audio_out_tx: mpsc::Sender<Vec<u8>>,  // Opus bytes from remote → squelch-audio
    pub audio_in_tx:  mpsc::Sender<Vec<u8>>,  // Opus bytes from squelch-audio → remote
    // inner: Arc<Mutex<Inner>> holds Rtc + SdpPendingOffer + Mid
}

impl PeerConnection {
    pub fn new(remote_id, role) -> Result<(Self, Receiver<Vec<u8>>), WebRtcError>;
    pub fn create_offer(&self) -> Result<String, WebRtcError>;     // SDP string
    pub fn accept_offer(&self, sdp: &str) -> Result<String, WebRtcError>;
    pub fn accept_answer(&self, sdp: &str) -> Result<(), WebRtcError>;
    pub fn add_ice_candidate(&self, candidate: &str) -> Result<(), WebRtcError>;
    pub fn run(self, socket: UdpSocket, shutdown_rx: oneshot::Receiver<()>);
}

pub enum PeerRole { Offerer, Answerer }

// mesh.rs
pub struct PeerMesh { peers: HashMap<String, PeerConnection> }

impl PeerMesh {
    pub fn add_peer(&mut self, remote_id, role)
        -> Result<Receiver<Vec<u8>>, WebRtcError>;
    pub fn get(&self, remote_id: &str) -> Option<&PeerConnection>;
}
```

**SDP/ICE state machine:**
```
Offerer:  create_offer() ──[SDP via Matrix]──→ remote
          accept_answer() ←─[SDP via Matrix]── remote
          add_ice_candidate() ←─[via Matrix]── remote (trickle ICE)

Answerer: accept_offer() ←─[SDP via Matrix]── remote
          [return answer] ──[SDP via Matrix]──→ remote
          add_ice_candidate() ←─[via Matrix]── remote
```

**Implementation notes:**
- `SdpPendingOffer` is stored in `Inner` between `create_offer` and `accept_answer`
- `0.0.0.0` is rejected by str0m as an ICE host candidate; the run loop should
  bind to a resolved interface address in production
- Single-mutation invariant: after every `writer.write()` call, `poll_output` must
  be drained to `Output::Timeout` before the next mutation

---

### 5.4 `squelch-audio`

**Responsibility:** Microphone capture, Opus encoding, PTT-gated channel routing,
remote peer audio decoding, and output mixing via cpal.

**Architecture:**

```
cpal input thread
  f32 PCM (device rate, N channels)
    → downmix to mono
    → accumulate 960 samples (20ms @ 48kHz)
    → Opus encode
    → duo_opus_tx   (always)
    → leader_opus_tx (only when PTT active)

squelch-webrtc
  → audio_in_tx per peer → PeerConnection run loop → str0m → UDP

squelch-webrtc
  → UDP → str0m → PeerConnection → audio_out_tx
    → remote mpsc::Receiver<Vec<u8>>
      → Opus decode (decoder pool, one per peer)
      → mix (sum + clamp to [-1.0, 1.0])
        → cpal output thread → speakers
```

**Key types:**

```rust
// ptt.rs
pub struct PttState(pub(crate) Arc<AtomicBool>);  // Clone, Debug, Send

impl PttState {
    pub fn new() -> Self;
    pub fn press(&self);      // PTT key down
    pub fn release(&self);    // PTT key up
    pub fn is_active(&self) -> bool;
}

// pipeline.rs
pub struct AudioConfig {
    pub ptt: PttState,
}

pub struct AudioHandles {
    pub duo_opus_tx:    mpsc::Receiver<Vec<u8>>,   // Opus bytes → duo peer WebRTC
    pub leader_opus_tx: mpsc::Receiver<Vec<u8>>,   // Opus bytes → leader peers WebRTC
    pub add_remote_fn: Box<dyn FnMut() -> mpsc::Sender<Vec<u8>> + Send>,
}

pub struct AudioPipeline { ... }  // keeps cpal streams alive — drop to stop

impl AudioPipeline {
    pub fn start(cfg: AudioConfig)
        -> Result<(AudioPipeline, AudioHandles), AudioError>;
}

// Codec helpers (used by squelch-webrtc integration tests)
pub fn encode_frame(encoder: &mut Encoder, pcm: &[f32], out: &mut Vec<u8>)
    -> Result<usize, AudioError>;
pub fn decode_packet(decoder: &mut Decoder, packet: &[u8], out: &mut [f32])
    -> Result<usize, AudioError>;

pub const OPUS_FRAME_SAMPLES: usize = 960;  // 20ms at 48kHz
```

**Implementation notes:**
- Opus frames are always 960 samples (20ms at 48kHz) — the Opus spec requirement
- PTT gating happens in the input callback via `AtomicBool::load(Relaxed)` — no Mutex
  in the hot path, one frame of lag is imperceptible (< 1ms)
- The decoder pool grows dynamically as remote peers are added via `add_remote_fn`
- Stereo → mono downmix: channel average. Mono → stereo upmix: duplicate channel.
- Proper resampling (for devices not running at 48kHz) is post-MVP

---

### 5.5 `squelch-app`

**Responsibility:** Entry point, egui UI, global PTT hotkey, tray icon. Wires all
crates together. No business logic — only initialization and event routing.

```rust
// main.rs — Phase 4 (not yet implemented)
fn main() {
    // 1. Load config from disk
    // 2. Build PttState (shared between hotkey handler and audio pipeline)
    // 3. Start AudioPipeline::start()
    // 4. Login to Matrix (MatrixClient::login)
    // 5. Start sync loop (start_sync)
    // 6. Launch egui window for squad setup
    // 7. On setup complete: minimize to tray
    // 8. Register global PTT hotkey (global-hotkey crate)
}
```

---

## 6. Conventions

- **No `.unwrap()` in production code** — always `?` or explicit error handling
- **No `pub use *` exports** — explicit API boundaries per crate
- **Every public function has a doc comment** (`///`)
- **Every module has unit tests** at the bottom of the file (`#[cfg(test)]`)
- **All dependencies versioned in `[workspace.dependencies]`** — crate `Cargo.toml`
  only uses `{ workspace = true }`
- **All workspace crates have `publish = false`**
- **Commit messages follow conventional commits** (`feat:`, `fix:`, `chore:`, `docs:`)
- **Use `just commit "message"`** — never `git commit` directly
- **Audio callbacks are lock-free** — no Mutex in cpal input/output callbacks;
  use `AtomicBool`, channels, and `try_recv`/`try_send`
- **Opus bytes (`Vec<u8>`) are the interface between squelch-webrtc and squelch-audio**
  — WebRTC is transport-only, codec logic stays in squelch-audio

---

## 7. Running the Project

```bash
# Build everything
just build

# Run all tests
just test

# All quality gates (clippy + fmt + test)
just check

# Run a spike
just run-spike 001-str0m-webrtc

# Format code
just fmt

# Commit (stages all, then commits)
just commit "feat: your message"

# Push to GitHub
just push
```

---

## 8. Architecture Decisions

All architectural decisions are documented in `docs/adr/`:

| ADR | Decision | Status |
|-----|----------|--------|
| 0001 | Cargo Workspace as project structure | Accepted |
| 0002 | egui as UI framework | Accepted |
| 0003 | Matrix as signaling backend | Accepted |
| 0004 | MIT License | Proposed |
| 0005 | Communication model: Duo channels + Leader Net | Accepted |

---

## 9. What's Next

| Phase | Crate | Status |
|-------|-------|--------|
| Phase 1 | `squelch-matrix` — login, room, signaling | ✅ Done |
| Phase 2 | `squelch-webrtc` — PeerConnection + PeerMesh | ✅ Done |
| Phase 3 | `squelch-audio` — cpal pipeline + Opus | ✅ Done |
| Phase 4 | `squelch-app` — egui UI + global hotkey | 🔲 Next |
| Post-MVP | Star Citizen scaling (3+ teams) | 🔲 Planned |
| Post-MVP | Noise suppression (RNNoise) | 🔲 Planned |
| Post-MVP | E2EE signaling via Matrix E2EE | 🔲 Planned |
