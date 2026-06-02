# ADR-0001 – Cargo Workspace as Project Structure
**Date:** 2026-06-02
**Status:** Accepted
**Author:** Christian / SetScallywag

---

## Context

A monolithic `main.rs` approach leads to poor testability and high refactoring cost as a project grows (lesson learned from a prior project). Squelch should enforce modularity from day one to remain approachable for external contributors.

---

## Decision

Squelch is organized as a **Cargo workspace** with multiple independent crates.

---

## Rationale

A workspace enforces clear API boundaries between crates. Each crate has a single responsibility and can be tested independently. New functional areas can be added as separate crates without touching existing code. A shared `Cargo.lock` ensures consistent dependency versions across all crates.

---

## Alternatives Considered

| Option | Why rejected |
|--------|-------------|
| Single crate (monolith) | Poor testability, high refactoring cost |
| Separate repositories per crate | Too much overhead for an early-stage project |

---

## Consequences

**Positive:**
- Clear separation of responsibilities
- Individual crates testable in isolation (`cargo test -p squelch-audio`)
- New crates (alternative backends, plugins) easy to add post-MVP
- External contributors can navigate the codebase more easily

**Negative / Risks:**
- Higher initial setup cost compared to `cargo new`
- Dependencies between crates must be declared explicitly

---

## Related ADRs

- ADR-0002 – egui as UI Framework
- ADR-0003 – Matrix as Signaling Backend
- ADR-0004 – MIT License
