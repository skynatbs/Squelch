# ADR-0008 – Lobby-First UX Model
**Date:** 2026-06-03
**Status:** Accepted
**Author:** Christian / SetScallywag

---

## Context

The previous Team Setup model had an explicit "Start Session" button that transitioned
from a setup screen to a running screen. This created unnecessary friction and was
unintuitive compared to how players think about communication tools.

The reference model is Discord: you join a voice channel and you're immediately in it.
No explicit "start" step required.

---

## Decision

Squelch uses a **Lobby-First model**:

1. When a player joins the room they immediately enter the **Lobby**.
2. In the Lobby, all unassigned players hear each other via open mic.
3. Players assign themselves to squads (or leaders assign others).
4. Once assigned to a squad, the player **leaves the Lobby audio** and joins their
   **Squad open mic** instead.
5. The room view always shows the full picture: who is in the Lobby, who is in
   which squad, who is the squad leader.

---

## Lobby Model

```
Room: !abc:matrix.org

Lobby (open mic — all unassigned players)
  🔊 @alice    🔊 @bob    🔊 @carol

Squad: Alpha         Squad: Bravo
  ★ @dave              ★ @eve
    @frank               @grace
  [Leave Squad]        [Leave Squad]

[+ Join Alpha]  [+ Join Bravo]   ← shown to unassigned players
[Create Squad]                   ← available to anyone
```

---

## Assignment Rules

- Any player can assign **themselves** to any squad
- A squad **leader** can assign **other** players to their own squad
- Any player can click **"Leave Squad"** to return to the Lobby
- "Disband Squad" (leader only) removes the squad and returns all members to Lobby

---

## Audio Routing

| Where | Hears |
|-------|-------|
| Lobby | All other Lobby members (open mic) |
| Squad | All squad members (open mic) + Leader Net if leader |
| Between squads | Leader Net only (PTT) |

---

## Leave Session vs. Disband

- **Leave Session** — I leave the room. Room persists. Others stay. I return to the
  Squad Setup screen where I can rejoin with the same Room ID.
- **Disband Squad** — Leader-only. Sends `io.squelch.disband` to all members.
  Everyone leaves the room and clears their local room config.

---

## UI Screens (revised)

```
Login → Squad Setup (room ID input) → Room View (lobby + squads, always running)
                                           ↑
                                    [Leave Session] → back to Squad Setup
```

No "Start Session" button. The room is live immediately on join.

---

## Consequences

**Positive:**
- No cognitive overhead — join → you're in
- Familiar model (Discord-like)
- Squad assignment is live and reversible
- Lobby acts as a natural coordination space before operations

**Negative / Risks:**
- Lobby audio with many players can get loud (same problem Discord has)
- Players must remember to join their squad before the operation starts
  (convention, not enforced — acceptable for a small group)

---

## Related ADRs

- ADR-0007 – Squad Open Mic (unaffected — still the model within squads)
- ADR-0006 – Room Lifecycle (unaffected)
- ADR-0005 – Leader Net (unaffected)
