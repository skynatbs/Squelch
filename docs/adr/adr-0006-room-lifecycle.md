# ADR-0006 – Squad Room Lifecycle
**Date:** 2026-06-02
**Status:** Accepted
**Author:** Christian / SetScallywag

---

## Context

Squelch uses a Matrix room as the signaling hub for each squad. A decision was
needed on how long that room should exist, who can remove it, and how cleanup
is handled without creating permanent abandoned rooms on third-party homeservers.

Two concerns drove this decision:

1. **Resource respect:** Public homeservers are often run by volunteers. Thousands
   of abandoned rooms from one-time squads would be an unreasonable burden.
2. **Permission mismatch:** Matrix room admin rights are tied to the account that
   created the room (Power Level 100). Squelch's leadership concept (transferable
   via a click) is an app-level concept that Matrix knows nothing about. Binding
   "disband" rights to the Matrix admin would create situations where a transferred
   leader cannot clean up a room they don't own on the Matrix level.

---

## Decision

Squelch uses **persistent squad rooms with explicit collective cleanup**:

1. **`Start Squad` (create):** Creates a new Matrix room and stores the room ID in
   `~/.config/squelch/config.toml`. Members are invited once.
2. **`Join Session` (rejoin):** On subsequent sessions, clients join the existing
   stored room ID — no new room, no sharing a new ID every time.
3. **`Leave Session`:** The local client leaves the current audio session.
   The Matrix room remains. Members can rejoin on the next session.
4. **`Disband Squad`:** Triggers collective cleanup — see below.

---

## Disband Protocol

Disband is a **squad-leader-only action** enforced at the Squelch app level
(not at the Matrix Power Level level). When the current leader presses
"Disband Squad":

```
Leader sends:  io.squelch.disband  (to-device message to all members)
All clients:   receive the event
               → leave the Matrix room
               → delete room ID from local config
               → return to the setup screen
```

The Matrix room becomes empty once all members have left. Empty rooms are
eventually cleaned up by the homeserver (behavior varies by implementation,
but all major servers reclaim abandoned rooms over time).

**No Matrix admin action is required.** The room dies naturally when all
members leave — consistent with how Matrix federation works.

---

## Rationale

This approach solves both concerns cleanly:

- **Resource respect:** Rooms are cleaned up actively, not left to accumulate.
- **Permission clarity:** Disband rights live entirely in Squelch's app model
  (squad leader), independent of who holds Matrix Power Level 100. Leadership
  can be transferred freely without worrying about Matrix room ownership.
- **Resilience:** If the original room creator leaves the squad, the room
  lifecycle is not blocked — the leader can still initiate disband.
- **Simplicity:** No Matrix admin API calls needed. Every client just calls
  `leave_room()` on receiving the disband event.

---

## Alternatives Considered

| Option | Why rejected |
|--------|-------------|
| Ephemeral rooms (create/delete each session) | ID must be shared before every session — friction |
| Persistent rooms, no explicit cleanup | Abandoned rooms accumulate on third-party servers |
| Disband = Matrix admin deletes room | Tied to room creator, not transferable with leadership |
| Disband requires unanimous vote | Too complex for MVP; trust the leader |

---

## UI Consequences

The `Running` screen will expose two distinct actions:

```
[ Leave Session ]    — I leave temporarily; room stays; I can rejoin later
[ Disband Squad ]    — shown only to the current leader
                       sends io.squelch.disband to all members
                       all clients leave and clear their local config
```

---

## New Signaling Event

A new to-device event type is added:

```
io.squelch.disband
  payload: { "room_id": "!abc:example.org", "reason": "leader_initiated" }
```

Only accepted from the user who currently holds the leader role in the
squad state. Clients ignore disband events from non-leaders.

---

## Consequences

**Positive:**
- Clean resource usage on public homeservers
- No dependency on Matrix Power Level for squad management
- Leadership transfer works seamlessly
- Simple implementation: one new event type, one new button

**Negative / Risks:**
- If a leader disconnects permanently without disbanding, remaining members
  must each leave manually to clean up (or elect a new leader who can disband)
- `io.squelch.disband` is unencrypted in MVP — a rogue actor with Matrix account
  access could spoof it (acceptable risk for a gaming tool; E2EE is post-MVP)

---

## Related ADRs

- ADR-0003 – Matrix as Signaling Backend
- ADR-0005 – Communication Model: Duo Channels + Leader Net
