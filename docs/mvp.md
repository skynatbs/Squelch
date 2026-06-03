# Squelch – MVP Definition
**Version:** 0.2
**Status:** Concept phase
**Date:** 2026-06-02

---

## 1. Vision

Squelch is an open-source desktop app for gaming squads that enables structured voice communication without channel switching. Inspired by military radio nets and the communication model of the game *Squad*: your duo hears each other permanently, squad leaders coordinate via PTT on the Leader Net. No Discord channel switching, no interruption of gameplay.

---

## 2. Target Audience

**Primary (MVP):** 4-player gaming squads with two duos (e.g. Grayzone Warfare).
**Secondary (post-MVP):** Larger groups with 3+ teams (e.g. Star Citizen org operations).
**Out of scope for MVP:** Broadcast streaming, session recording, mobile clients.

---

## 3. Communication Model

```
Within a squad:   N:N open mic — all members hear each other permanently
                  Works for any squad size (2–8+ players)

Between leaders:  Leader Net — PTT, all squad leaders simultaneously
```

### Concrete example (Star Citizen operation)

```
Squad "Support"    (2):  alice, bob                          [open mic]
Squad "Fighter"    (6):  carol, dave, eve, frank, grace, heidi [open mic]
Squad "Dropships"  (2):  ivan, julia                         [open mic]
Squad "Ground-1"   (4):  kevin, linda, mike, nancy            [open mic]
Squad "Ground-2"   (4):  oscar, patricia, quinn, roger        [open mic]
Squad "Ground-3"   (4):  sarah, thomas, uma, victor           [open mic]

Leader Net (PTT):        alice, carol, ivan, kevin, oscar, sarah
```

**Leader rules:**
- First player to create the squad is automatically its leader
- Leadership is transferable via a single click in the app
- Only the current leader can initiate squad disband (ADR-0006)

---

## 4. MVP Scope

### 4.1 Features (In Scope)

#### Team & Squad Management
- Create a team room (one Matrix room for the entire operation)
- Create named squads within the team room
- Join an existing team room via room ID
- Team overview: all squads, all members, online status, leader indicator
- Leader assignment with transfer functionality (per squad)
- Squad disband by leader (ADR-0006)

#### Audio Channels
- **Squad open mic:** always-on N:N audio between all members of the same squad
  (any squad size — 2 to 8+ players)
- **Leader Net:** PTT key broadcasts to all other squad leaders simultaneously
- Simultaneous mixing of squad open mic + incoming leader net without artifacts

#### Audio Stack
- Microphone capture and playback via cpal
- WebRTC P2P audio streams — audio never passes through a server
- Opus encode/decode (48kHz, 24kbps voice)
- Resampling from device rate to 48kHz (linear, sufficient for voice)
- STUN/TURN for NAT traversal (internet gaming)
- Global PTT hotkey (works even when the game window has focus)
- Configurable PTT key (saved to `~/.config/squelch/config.toml`)

#### Signaling
- Matrix as signaling and discovery backend
- Matrix account required (any homeserver — matrix.org, self-hosted, etc.)
- WebRTC offer/answer + ICE via Matrix to-device events (`io.squelch.*`)
- Disband protocol via `io.squelch.disband` (ADR-0006)

#### Platforms
- **Windows** (primary — main gaming platform for most players)
- **Linux** (secondary — development platform)
- macOS explicitly **not** in MVP

---

### 4.2 Features (Out of Scope – Post-MVP)

- More than 2 duos / 3+ teams (Star Citizen scaling)
- Mobile app (iOS / Android)
- Session recording and playback
- Noise suppression / noise cancellation (RNNoise etc.)
- LAN mode / mDNS discovery
- Push-to-talk via joystick / HOTAS button
- Self-hosted TURN server
- End-to-end encryption via Matrix E2EE (WebRTC DTLS-SRTP is sufficient)

---

## 5. Technical Stack (confirmed)

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Language | **Rust** | Performance, safety, team expertise |
| UI | **egui / eframe** | Pure Rust, no WebView, minimal — fits the utility character |
| Tray + Hotkey | **tray-icon + global-hotkey** | Rust-native, no Tauri required |
| Signaling | **Matrix** (matrix-rust-sdk) | Federated, no own server, open source |
| Audio I/O | **cpal** | Cross-platform, Rust-native |
| Audio P2P | **WebRTC** (str0m — validated in Spike 001) | P2P, DTLS-SRTP encrypted |
| NAT Traversal | **STUN/TURN** | Standard WebRTC, public servers usable |
| Build | **Cargo Workspace** | Clear crate boundaries, good testability |
| License | **MIT** | Maximum openness, community-friendly |

---

## 6. Architecture Principles

1. **Audio never passes through a third-party server** — P2P via WebRTC only
2. **No Squelch server infrastructure** — Matrix federation makes it unnecessary
3. **Cargo workspace from day one** — clear crate boundaries, good testability
4. **No `.unwrap()` in production code** — `thiserror` / `anyhow`
5. **Transparently documented** — every architectural decision in `docs/adr/`
6. **Pure Rust** — no second tech stack, no JavaScript

---

## 7. Planned Crate Structure

```
squelch/
├── squelch-core/       # Shared types, config, error types, squad model
├── squelch-matrix/     # Matrix client, signaling, room management
├── squelch-webrtc/     # WebRTC peer connections, audio streams (str0m)
├── squelch-audio/      # Microphone capture, mixer (duo + leader net), cpal
└── squelch-app/        # Entry point, egui UI, tray icon, global PTT hotkey
```

---

## 7. MVP Success Criteria

The MVP is complete when the following user stories are fulfilled:

1. **As a player** I can create a team room and invite others — without registering
   an account with Squelch (any Matrix account works).
2. **As a squad member** I hear all my squad mates permanently without pressing
   any button, regardless of squad size (2–8 players).
3. **As a squad leader** I can reach all other squad leaders simultaneously by
   holding a configurable key — even while the game has focus.
4. **As any player** I see a team overview: all squads, their members, and who
   the leaders are — so everyone knows squad composition at a glance.
5. **As a regular player** I have no access to the Leader Net — coordination stays
   with the leaders.
6. **As a contributor** I can understand the project and trace decisions because
   everything is documented in `docs/adr/`.

---

## 9. Non-Functional Requirements

- **Latency:** < 50ms end-to-end audio
- **Startup time:** < 3 seconds
- **Memory usage:** < 100MB RAM at idle
- **No Squelch cloud dependency** — Matrix server is chosen by the user

---

## 10. Open Questions (Spike Phase)

- [ ] **MatrixRTC:** How much does matrix-rust-sdk handle for WebRTC signaling? (Spike 002)
- [ ] **Audio mixing:** Simultaneous mixing of duo channel + leader net without artifacts (Spike 003)
- [ ] **TURN server:** Which public TURN servers are suitable for gaming latency?
