# Squelch

> Tactical voice communication for gaming squads.

Squelch is an open-source desktop app for small gaming groups (4–8 players) that need structured audio channels without the friction of manually switching channels in Discord.

## The Problem

When playing in a squad of 4+, communication gets chaotic. Open mic for everyone means crosstalk. Switching Discord channels mid-game breaks flow. Games that solve this (squad nets, platoon nets) show it works — but only in-game.

## The Concept

Inspired by military radio nets and the communication model of the game *Squad*:

- **Duo channel** — always-on open mic between the 2 players of a duo. No button, no friction.
- **Leader Net** — PTT key exclusively for squad leaders. One leader reaches all other leaders simultaneously. Regular players have no access.

No channel switching. No interruption of gameplay.

## Architecture

| Layer | Technology | Role |
|-------|-----------|------|
| Signaling | Matrix (matrix-rust-sdk) | Squad discovery, room management, WebRTC handshake |
| Audio | WebRTC P2P (str0m) | Direct audio streams, never through a third-party server |
| App | egui / eframe (Rust) | Desktop app, global PTT hotkey |
| Audio mixing | cpal | Software mixer: duo channel (always-on) + leader net (PTT) |

Matrix is used exclusively as a meeting point — audio data flows directly between peers via WebRTC. Users can bring their own Matrix homeserver or use any public one (matrix.org etc).

## Status

Spike / concept phase.

## License

MIT — see [LICENSE](LICENSE).
