# ADR-0005 – Communication Model: Duo Channels + Leader Net
**Date:** 2026-06-02
**Status:** Partially superseded by ADR-0007
**Author:** Christian / SetScallywag

> **Note:** The "Duo Channel" concept has been superseded by ADR-0007 (Squad Open Mic).
> The Leader Net sections of this ADR remain valid and in effect.

---

## Context

A squad of 4+ players needs structured communication. A single open channel for everyone leads to chaos — players talk over each other, signal-to-noise ratio collapses. Discord solves this through manual channel switching, which causes too much friction during gameplay. An "all-PTT" button available to every player does not solve the problem either, since gamers — unlike military personnel — rarely maintain the necessary radio discipline.

The game *Squad* (Offworld Industries) has solved this convincingly with a multi-tier channel approach: local channel, squad channel, and a command net for squad leaders only.

---

## Decision

Squelch implements **two channel tiers**:

1. **Duo Channel (Team Channel):** Always-on open mic between the 2 players of a duo. No button, no thought required.
2. **Leader Net:** PTT channel exclusive to squad leaders. A leader presses PTT and reaches all other leaders simultaneously. Regular players have no access to the Leader Net.

**Leader assignment:** The first player in the squad is automatically the leader. Leadership can be transferred to another member with a single click in the app.

---

## Rationale

This model solves the discipline problem through mechanics rather than convention: regular players *cannot* burden the Leader Net regardless of their excitement level. At the same time, communication within the duo remains natural and frictionless. The Leader Net scales elegantly: with 3 teams (Star Citizen), leaders A, B, and C communicate via PTT without the 6 other players being affected.

---

## Alternatives Considered

| Option | Why rejected |
|--------|-------------|
| All-PTT for every player | Lack of player discipline makes this chaotic |
| Duo channels only, no cross-team channel | Leaders cannot coordinate |
| All-PTT for leaders only (broadcast) | Better, but one-directional — Leader Net enables dialogue |
| Configurable by squad | Too much complexity for MVP; a sensible default is sufficient |

---

## Consequences

**Positive:**
- Discipline is structurally enforced, not convention-dependent
- Duo channel is always active — no button, no effort
- Leader Net scales to any number of teams (Star Citizen, larger groups)
- Clear mental model: "I talk to my duo OR to the other leaders"

**Negative / Risks:**
- Every group needs at least one assigned leader
- A forgotten leader transfer can block coordination (transfer must be frictionless)

---

## Related ADRs

- ADR-0001 – Cargo Workspace as Project Structure
- ADR-0003 – Matrix as Signaling Backend
