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
Duo A (P1 + P2):   always-on, hear each other permanently
Duo B (P3 + P4):   always-on, hear each other permanently

Leader Net:        P1 presses PTT  →  P3 hears them  (and vice versa)
                   Leaders only, not all players
```

**Leader rules:**
- First player in the squad is automatically the leader
- Leadership is transferable via a single click in the app
- Each duo has exactly one leader

---

## 4. MVP Scope

### 4.1 Features (In Scope)

#### Squad Management
- Create a squad via Matrix room
- Join a squad via invite link / room ID
- Duo assignment (who is paired with whom)
- Leader assignment with transfer functionality

#### Audio Channels
- **Duo channel:** always-on open mic between the 2 duo members
- **Leader Net:** PTT key broadcasts to all other leaders simultaneously
- Simultaneous mixing of both channels without artifacts

#### Audio Stack
- Microphone capture and playback via cpal
- WebRTC P2P audio streams (audio never passes through a server)
- STUN/TURN for NAT traversal (internet gaming)
- Global PTT hotkey (works even when the game window has focus)
- Configurable PTT key

#### Signaling
- Matrix as signaling and discovery backend
- Matrix account required (existing accounts usable)
- WebRTC offer/answer via Matrix events (MatrixRTC / MSC3401)

#### UI (minimal)
- Squad setup: create / join
- Member list with duo assignment and leader indicator
- Leader transfer via click
- PTT key configuration
- After setup: tray icon, app runs in background

#### Platforms
- **Windows** (primary — main gaming platform)
- **Linux** (secondary)
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

## 8. MVP Success Criteria

The MVP is complete when the following user stories are fulfilled:

1. **As a player** I can create a squad and invite three friends — without registering an account with Squelch.
2. **As a duo member** I hear my teammate permanently without pressing any button.
3. **As a squad leader** I can reach all other leaders simultaneously by holding a configurable key — even while the game has focus.
4. **As a regular player** I have no access to the Leader Net — coordination stays with the leaders.
5. **As a contributor** I can understand the project and trace decisions because everything is documented in `docs/adr/`.

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
