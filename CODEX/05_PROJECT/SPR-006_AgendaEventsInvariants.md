---
id: SPR-006
title: "Agenda, timers, lifecycle queues, event bus, invariants"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001]
created: 2026-10-16
updated: 2026-10-16
version: 1.1.0
---

> **BLUF:** Add scheduled outcomes as timers, lifecycle command queues, the event bus with emission-order EventIds, and the invariant framework. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-006 — Agenda, timers, lifecycle queues, event bus, invariants

- **Roadmap item:** `docs/11` P0.4 (part b) · **Phase:** P0 (gate G0)
- **Depends on:** SPR-005 · **Plan approved by the director 2026-10-16** (P0 batch)
- **Binding contracts:** ADR-0025 (scheduled outcomes, no back-dating); `05.1` §4.3 (agenda), §2 (lifecycle queues), §9 (invariants)
- **Requirements covered:** DET-13, SIM-2

## Architect rulings, 2026-10-16 (binding; written before the sprint starts)
1. **One canonical merge for everything a step emits.** Systems in one dependency level may run in parallel (SPR-005). Each step writes into its own buffers: agenda inserts and cancellations, lifecycle commands and event drafts. At the end of each level, `rh-core` merges the buffers in the canonical order of `05.1` §2 rule 4 (as amended by SPR-005: levels, then Registry id), and only then assigns `seq` values and EventIds. The thread count must never reach an id. The gating tests run at 1, 2, 4 and 8 threads.
2. **The agenda key** is `(time, phase, entity, seq)`.
   - `entity` is a typed key with a total order: an entity-kind tag first, then the id's integer value.
   - `seq` is a `u64` from one counter per timeline, assigned at the merge (ruling 1) and never reused.
   - Keys are unique, so pop order is a pure function of the keys; nothing relies on `BinaryHeap`'s order for equal keys.
   - Cancellation is by `TimerId` with lazy deletion (a tombstone checked at pop). A cancelled timer never fires.
3. **Re-sampling (ADR-0025).** In the `Inputs` phase, pending timers inside a material input's dependency scope are cancelled and re-drawn with the key `H(seed, branch_salt, purpose, entity, t_input, epoch)`, where `epoch` counts the material inputs that have touched the entity. P0 has no input journal (SPR-007), so the API and a test with a synthetic input are enough.
4. **EventIds and no back-dating.**
   - EventIds follow emission order `(tick, phase, seq)` and strictly increase.
   - An event's tick is the current instant; an outcome in the future is a timer, not an event.
   - A causal edge points only to an earlier EventId.
5. **Lifecycle command queues** are owned by `rh-core` and applied in `(phase, PersonId)` order by the owning system. `AccessSpec` gains lifecycle commands and event types (carried forward from SPR-005), so a non-owner is refused at registration, not at run time.
6. **The crash artifact in P0** (GOV-004 §4) carries:
   - the history tuple: build, Registry version, manifest hash, seed;
   - the journal position (0 until SPR-007's journal);
   - the failing instant and invariant;
   - the last diagnostic records.

   "Last keyframe" reads `none` until SPR-007. Replay re-runs from the tuple to the instant and must hit the same violation. Artifacts go to `artifacts/crash/`, which is not committed.
7. **SPR-007 runs alongside this sprint.** Its keyframes hold this sprint's pending timers, so it merges after this sprint. Keep the agenda's state serialisable: a plain sorted dump of `(key, timer)` pairs plus the counter and the tombstones.

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C3 (determinism machinery) |
| Planes touched (ADR-0024) | A |
| Owned columns written | core tables only |
| RNG purposes | none |
| Golden-hash effect | event order becomes part of the hash contract |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-DET-12 (no back-dating property test); invariant framework self-test |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: Agenda heap
- **Deliverable:** Binary-heap queue keyed by `(time, phase, entity_id, seq)`
- **Acceptance:** Deterministic order with ties; property test
- **Status:** [ ] not started

### T-02: Timers
- **Deliverable:** Insert, cancel and re-sample by dependency scope (epoch-keyed)
- **Acceptance:** Timer fires at its own tick in every mode
- **Status:** [ ] not started

### T-03: Lifecycle command queues
- **Deliverable:** Typed commands applied in `(phase, PersonId)` order by owners
- **Acceptance:** A non-owner cannot apply a command directly
- **Status:** [ ] not started

### T-04: Event bus
- **Deliverable:** `EventDraft` → EventId in emission order `(tick, phase, seq)`; causal edges
- **Acceptance:** V-DET-12: EventIds strictly increase; no event before the current instant
- **Status:** [ ] not started

### T-05: Invariant framework
- **Deliverable:** Per-system invariants run in Bookkeeping; a violation is fatal with a crash artifact (GOV-004 §4)
- **Acceptance:** An injected violation produces a crash artifact that replays
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-006` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-006` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
