# ADR-0004 – MIT License
**Date:** 2026-06-02
**Status:** Proposed
**Author:** Christian / SetScallywag

---

## Context

Squelch is to be released as an open-source project. The license choice affects which dependencies may be used, how the community can contribute, and whether commercial use is permitted.

---

## Decision

Squelch is released under the **MIT License**.

---

## Rationale

MIT is the most permissive and most widely used open-source license in the Rust ecosystem. It allows unrestricted use, modification, and redistribution — including in commercial contexts. This maximizes the potential user base and lowers the barrier for external contributions. All planned dependencies (matrix-rust-sdk: Apache-2.0, egui: MIT, str0m: MIT) are compatible with MIT.

---

## Alternatives Considered

| Option | Why rejected |
|--------|-------------|
| Apache-2.0 | Also permissive, but more complex — MIT is sufficient for this use case |
| GPL-3.0 | Copyleft forces open source on derivatives — restricts commercial use |
| AGPL-3.0 | Too restrictive for an app used in commercial gaming contexts |
| Proprietary | Contradicts the community-driven nature of the project |

---

## Consequences

**Positive:**
- Maximum openness for users and contributors
- Compatible with all planned dependencies
- Simple to understand, no compliance complexity

**Negative / Risks:**
- No obligation to contribute improvements back
- Commercial use without reciprocation is possible (consciously accepted)

---

## Related ADRs

- ADR-0003 – Matrix as Signaling Backend
