# ADR-0003 – Matrix as Signaling Backend
**Date:** 2026-06-02
**Status:** Accepted
**Author:** Christian / SetScallywag

---

## Context

Squelch needs a way for players to find each other and negotiate WebRTC connections (signaling). The solution must work over the internet, must not require Squelch to operate its own server infrastructure, and must fit the open-source character of the project.

---

## Decision

Squelch uses the **Matrix protocol** (via `matrix-rust-sdk`) exclusively as a signaling and discovery layer. Audio data flows directly between peers via WebRTC — never through a Matrix server.

---

## Rationale

Matrix is federated: users can use any public homeserver (e.g. matrix.org) or run their own. The project is not dependent on any central Squelch infrastructure. The protocol is established, well-documented, and `matrix-rust-sdk` is production-ready. MatrixRTC (MSC3401) is designed exactly for WebRTC signaling over Matrix.

---

## Alternatives Considered

| Option | Why rejected |
|--------|-------------|
| Custom Axum signaling server | Requires permanent Squelch infrastructure, single point of failure |
| WebRTC via LAN/mDNS | No internet support, NAT traversal problems |
| XMPP/Jingle | Smaller ecosystem, fewer active Rust libraries |
| Manual peer ID exchange (QR code) | Too much friction for the user |

---

## Consequences

**Positive:**
- No Squelch server infrastructure required
- Federated: no vendor lock-in, no single point of failure
- Users can run their own homeserver
- Audio data never leaves the P2P channel
- Natural fit for the Matrix community

**Negative / Risks:**
- Matrix account required (low barrier, but present)
- matrix-rust-sdk is a large dependency — learning curve
- MatrixRTC still relatively new, API may evolve

---

## Related ADRs

- ADR-0001 – Cargo Workspace as Project Structure
- ADR-0004 – MIT License
