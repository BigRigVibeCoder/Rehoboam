# ADR-0024 — Three-plane state model: authoritative, observation and narrative

**Status:** Accepted (director, 2026-10-16) · **Date:** 2026-10-15 · **Amends:** ADR-0006, ADR-0007, ADR-0008

## Context

In baseline v1.0, *observing* the world could still change it. The red-team review (RT-C1) found six ways this happened:

1. **Pins in hashed state.** Pins were triggered by viewing, talking or opening an inspector. A pin set a bit in the hashed ledger.
2. **Pinned persons promoted to T1.** Pinned persons became T1 actors, so they gained cognition and emitted events.
3. **Pins renumbered events.** Pinned participants gained event rows, and each new row renumbered later EventIds.
4. **Pins changed memory layout.** Pins kept dead rows in the live ledger, which shifted chunk membership.
5. **Lazy catch-up.** Catch-up of pinned actors happened at observer-determined times.
6. **Sift feedback.** Sift matches also pinned people.

None of this was journaled, so seed + journal could not reproduce a run. That breaks DET-1, DET-6 and success criterion S3 from the first console slice onwards.

## Decision

World state is partitioned into three planes, each with its own hash.

| Plane | Contents | Written by | Hash |
|---|---|---|---|
| **A — Authoritative** | Ledger, life-course log, region and polity tables, named actors, beliefs, agenda and timers, chronicle *events* | Simulation systems and **journaled inputs only** | `state_hash` |
| **B — Observation** | Pins, facts ledger (what you were shown), bubble and view caches, camera, hydrated presentation state | Clients, console, 3D bubble, LLM facade | `observation_hash` |
| **C — Narrative** | Sift matches (partial and complete), significance components, retention and visibility tiers, ratings, chronicles and prose | Sifter, significance jobs, Narrative Compiler | `narrative_hash`, a pure function of A's event stream plus pattern/compiler versions |

Rules:

1. **Plane-A code never reads planes B or C.** Nothing in B or C may change:
   - which events A emits;
   - the EventIds A assigns;
   - which rows A retains;
   - which actors A schedules;
   - any A value.
2. **The only path from B or C into A is a journaled material input**, such as a player action, a player speech act, an Oracle verb or an accepted LLM choice.
3. **T1 (named-actor) membership comes only from simulation criteria or journaled inputs.** The simulation criteria are: office, great-house rank, material involvement, Registry-declared notability, and promotion by a journaled input. Pinning does **not** promote a person to T1.
4. **No presentation components on authoritative entities.** Authoritative iteration over `bevy_ecs` entities is sorted by `PersonId`, never by archetype order.
5. **No lazy catch-up of authoritative state.** Everything in A advances on the simulation's own schedule.
6. **Bubble NPCs never originate material events.** Material events come from the simulation's own scheduling or from journaled player inputs. NPC responses in conversation are deterministic functions of A plus the player's journaled inputs. The LLM only realises prose (`05.3` §8).
7. **Re-sifting is a feature.** Any timeline can be re-sifted with any pattern-library version without touching A.

## Consequences

- DET-6 (observer invariance) becomes testable and binding from P2, through console fuzzing of pins, facts and inspections with `state_hash` unchanged.
- Golden baselines cover plane A only. Pattern, significance and compiler iteration (plane C) never churns them, which supports S4.
- The facts ledger is plane B: you saw it, so presentation must honour it, but the simulation never reads it. Facts that must be consistent with the 3D world are *behavioural flavour*, derived in plane A (ADR-0026).

## Evidence

RT-C1 and RT-C4 (red-team review, 2026-10-15); R01 §3 (observer independence); R03 §4 (unsynced guard).
