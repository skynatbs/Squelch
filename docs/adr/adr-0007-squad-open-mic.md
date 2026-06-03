# ADR-0007 – Squad Open Mic replaces Duo Channel
**Date:** 2026-06-03
**Status:** Accepted
**Supersedes:** ADR-0005 (partially)
**Author:** Christian / SetScallywag

---

## Context

ADR-0005 introduced a "Duo Channel" — a fixed pair of two players within a squad
who hear each other permanently. This was based on the assumption that squads always
consist of exactly 4 players split into 2 pairs.

A concrete Star Citizen example revealed this assumption is wrong:

```
Support:    2 players   — all hear each other
Fighter:    6 players   — all hear each other
Dropships:  2 players   — all hear each other
Ground-1:   4 players   — all hear each other
Ground-2:   4 players   — all hear each other
Ground-3:   4 players   — all hear each other
```

Squads can have any number of members (2–8 in practice). The communication
pattern inside a squad is always the same: **everyone hears everyone, always**.
There is no sub-grouping within a squad.

The "Duo" concept was an over-specification that would have forced artificial
constraints on squad composition.

---

## Decision

**The Duo Channel is replaced by Squad Open Mic.**

Every member of a squad is connected to every other member of the same squad via
an always-on open mic WebRTC connection. Squad membership defines the Open Mic
group — no further sub-grouping.

The Leader Net (PTT between squad leaders across all squads) is unchanged.

---

## Revised Communication Model

```
Within a squad:   N:N open mic — all members hear each other permanently
                  (N = squad size, any number ≥ 2)

Between leaders:  Leader Net — PTT, connects all squad leaders simultaneously
```

### Concrete example

```
Operation: OP Nightfall (Star Citizen)

Squad "Support"   (2):  alice ↔ bob                     [open mic]
Squad "Fighter"   (6):  carol, dave, eve, frank,
                        grace, heidi                     [open mic, full mesh]
Squad "Dropships" (2):  ivan ↔ julia                    [open mic]
Squad "Ground-1"  (4):  kevin, linda, mike, nancy        [open mic]
Squad "Ground-2"  (4):  oscar, patricia, quinn, roger    [open mic]
Squad "Ground-3"  (4):  sarah, thomas, uma, victor       [open mic]

Leader Net (PTT): alice, carol, ivan, kevin, oscar, sarah
```

---

## Data Model Changes

### Before (ADR-0005)

```rust
pub struct Squad {
    pub members: Vec<Member>,
    pub duos:    Vec<Duo>,       // ← removed
}

pub struct Duo {                 // ← removed
    pub members: [MemberId; 2],
}
```

### After (ADR-0007)

```rust
pub struct Team {
    pub name:   String,
    pub squads: Vec<Squad>,
}

pub struct Squad {
    pub name:    String,
    pub members: Vec<Member>,
    // No sub-grouping. All members share open mic.
    // Leader is identified by Role::Leader in Member.
}

pub struct Member {
    pub id:          MemberId,   // Matrix user ID
    pub display_name: String,    // human-readable name
    pub role:        Role,       // Member | Leader
}

pub enum Role {
    Member,
    Leader,
}
```

The `Duo` struct is removed entirely.

---

## Team Overview UI

Every player sees the full team structure. Each squad is displayed as a group
with its members and their roles clearly visible:

```
Operation: OP Nightfall                     Room: !abc:matrix.org  [📋]

Squad: Support            (2/2 online)
  ★ alice:matrix.org      Leader
    bob:matrix.org

Squad: Fighter            (6/6 online)
  ★ carol:matrix.org      Leader
    dave / eve / frank / grace / heidi

Squad: Ground-1           (4/4 online)
  ★ kevin:matrix.org      Leader
    linda / mike / nancy
...
```

This gives every player immediate situational awareness:
- Which squad am I in?
- Who is my squad leader?
- Which leaders are active on the Leader Net?

---

## WebRTC Topology Changes

### Before

Each player maintained 1 WebRTC connection (to their duo partner) + N connections
to other duo leaders.

### After

Each player maintains a connection to **every other member of their squad** (full
mesh within the squad) plus **one connection per other squad leader** (for the
Leader Net).

```
Fighter squad member (non-leader):
  → 5 open mic connections (to other 5 squad members)
  → 0 leader net connections

Fighter squad leader:
  → 5 open mic connections (to squad members)
  → 5 leader net connections (to other 5 squad leaders)
```

Maximum connections for a 6-person squad leader in a 6-squad operation:
  5 (squad) + 5 (leader net) = 10 WebRTC connections

This is well within RFC limits and typical usage. str0m handles each connection
independently in its own thread.

---

## Rationale

The original Duo design was chosen for simplicity in MVP. The Star Citizen example
shows it was unnecessarily restrictive. Squad Open Mic is:

- **More natural:** "My squad hears each other" is how players think
- **More flexible:** Works for 2-person squads up to 8-person squads unchanged
- **No worse technically:** WebRTC mesh scales linearly; str0m handles it cleanly

The only cost is more WebRTC connections for larger squads, which is expected and
manageable.

---

## What ADR-0005 Retains

ADR-0005's core decisions remain valid:

- Leader Net (PTT between squad leaders) — **unchanged**
- Leader assignment: first member is leader, transferable — **unchanged**
- Disband protocol — see ADR-0006 — **unchanged**

Only the "Duo Channel" concept is superseded.

---

## Consequences

**Positive:**
- Supports squads of any size (2–8+ players)
- Simpler mental model: squad = open mic group, no sub-structure
- Flexible team composition for any game (Grayzone, Star Citizen, etc.)

**Negative / Risks:**
- More WebRTC connections for large squads (e.g. 6-player Fighter squad: 5 connections
  per member instead of 1) — acceptable, well within practical limits
- `squelch-core` data model needs updating (remove `Duo`, add `Team`, expand `Squad`)
- Audio routing logic changes: instead of "route to duo partner", route to all squad members
- UI needs a Team Overview screen (replacing the current simple squad setup)

---

## Related ADRs

- ADR-0005 – Communication Model (partially superseded — Leader Net sections remain valid)
- ADR-0006 – Squad Room Lifecycle (unaffected)
- ADR-0003 – Matrix as Signaling Backend (unaffected)
