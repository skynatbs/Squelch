# Squelch — First Multiplayer Test Guide

> **For the test session with 4 players on 2026-06-03**

---

## What this test validates

- All 4 players can log in with their own Matrix accounts
- One player creates the room, 3 others join with the Room ID
- Audio pipeline starts on each machine (mic active)
- WebRTC connections are attempted between players (ICE/SDP exchange via Matrix)
- PTT key works on each machine

> **Note:** Audio between players requires NAT traversal (STUN). If players are on
> the same LAN this is trivial. Over the internet, ICE usually succeeds within 5–10s.
> The SDP Answer routing (Phase 5b) is partially implemented — you may not hear each
> other yet, but you can verify signaling events arrive (check the logs).

---

## Prerequisites

### Every player needs

1. **A Matrix account** — any homeserver works:
   - Free registration: https://app.element.io (uses matrix.org)
   - Or any other homeserver (asra.gr, your own, etc.)

2. **Rust toolchain** (for building from source)
   - Linux/macOS: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
   - Windows: download rustup from https://rustup.rs

3. **On Linux:** ALSA dev headers
   ```bash
   sudo apt install libasound2-dev pkg-config   # Debian/Ubuntu
   sudo pacman -S alsa-lib pkg-config           # Arch/CachyOS
   ```

4. **On Windows:** Visual Studio Build Tools with C++ workload
   - Download from https://visualstudio.microsoft.com/visual-cpp-build-tools/
   - Select "Desktop development with C++"

---

## Build & run

```bash
# Clone the repo
git clone https://github.com/skynatbs/Squelch.git
cd Squelch

# Build and run (first build takes 2–3 minutes)
cargo run --bin squelch --release
```

> Use `--release` for better audio performance.

---

## Test procedure

### Player 1 (session host — SetScallywag)

1. Start Squelch
2. Enter your Matrix credentials:
   - Homeserver: `https://matrix.org` (or your homeserver)
   - Username: your Matrix username (without `@` and domain)
   - Password: your password
3. Click **Sign in** — wait for "Logged in as @..."
4. Leave the **Room ID** field empty
5. Set your **PTT Key** (default: `CapsLock`)
6. Click **Create Squad**
7. **Copy the Room ID** that appears (looks like `!abc123:matrix.org`)
8. Share it with the other 3 players (Discord, chat, etc.)

### Players 2, 3, 4

1. Start Squelch
2. Sign in with your own Matrix credentials
3. Paste the **Room ID** into the Room ID field
4. Set your PTT Key
5. Click **Join Squad**

---

## What to check

| What | Expected |
|------|----------|
| `🎙 Microphone active` shown | ✓ Audio pipeline started |
| Status: "Joined squad room." | ✓ Matrix room joined |
| Room ID visible and copyable | ✓ |
| `● LEADER NET — TRANSMITTING` on CapsLock hold | ✓ PTT works |
| Logs show `SDP offer sent` | ✓ WebRTC signaling started |
| Logs show `received SDP offer` on other machines | ✓ Matrix signaling works |

---

## Logs

To see detailed logs, run with:

```bash
RUST_LOG=squelch=debug,squelch_matrix=debug,squelch_webrtc=debug,warn \
  cargo run --bin squelch --release
```

Key log messages to look for:

```
INFO  squelch_audio::pipeline: audio pipeline started        ← audio OK
INFO  squelch: Logged in as @you:matrix.org                  ← login OK
INFO  squelch: SDP offer sent                                ← signaling OK
INFO  squelch: received SDP offer                            ← other side OK
INFO  squelch: ICE state Connected                           ← WebRTC connected
```

---

## Known limitations (Phase 5 — in progress)

- **SDP Answer routing:** When you receive a SDP answer from a peer, it is not yet
  fed back to the active `PeerConnection` (Phase 5b TODO). This means ICE/DTLS
  may not complete. Audio may not flow yet.
- **ICE candidate exchange** is also pending Phase 5b.
- **Team overview** (seeing all squads and members) is Phase 5 UI work.
- **Windows:** Build has not been tested yet. Please report any Windows-specific errors.

---

## Reporting issues

Please note down:
1. Your OS and version
2. The exact error message or log line
3. At what step it failed

Open an issue at https://github.com/skynatbs/Squelch/issues or report in the session.

---

## Session info

- **Room ID:** `!kAizKeKeeQKSXNozVl:matrix.org`
- **Homeserver:** matrix.org (players can use any homeserver)
- **PTT default:** CapsLock
