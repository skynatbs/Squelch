# Squelch

> Tactical voice communication for gaming squads.

Squelch is an open-source desktop app for small gaming groups (4-8 players) that need structured audio channels without the friction of manually switching channels in Discord.

## The Problem

When playing in a squad of 4+, communication gets chaotic. Open mic for everyone means crosstalk. Switching Discord channels mid-game breaks flow. Games that solve this (squad nets, platoon nets) show it works — but only in-game.

## The Concept

Inspired by military radio nets:

- **Team channel** — always-on open mic between your duo/trio
- **All channel** — PTT (push-to-talk) key broadcasts to the full squad
- No channel switching. No friction.

## Architecture

| Layer | Technology | Role |
|-------|-----------|------|
| Signaling | Matrix (matrix-rust-sdk) | Squad discovery, room management, WebRTC handshake |
| Audio | WebRTC (P2P) | Direct audio streams, never through a third-party server |
| App | Tauri 2 + Rust | Desktop app, global PTT hotkey |
| Audio mixing | cpal | Software mixer: team channel (always-on) + all channel (PTT) |

Matrix is used exclusively as a meeting point — audio data flows directly between peers via WebRTC. Users can bring their own Matrix homeserver or use any public one (matrix.org etc).

## Status

Concept / Spike phase.

## License

To be determined (likely MIT or Apache 2.0).
