---
id: SPR-006
title: "Agenda, timers, lifecycle queues, event bus, invariants"
type: how-to
status: ARCHIVED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001, VER-SPR-006, VER-SPR-006-TEST]
created: 2026-10-16
updated: 2026-10-16
version: 1.2.1
---
> [!NOTE]
> **ARCHIVED BASELINE:** Traditional sprint-based tracking has been superseded by Specification-Driven Development (SDD v2.0). All future deliverables are tracked as Spec Waves in `.spec/ARCHITECT_BACKLOG.md`.

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
- **Status:** [x] done (`56a3b9c`, `e606389`: `rh-core::agenda`, `rh_ids::EntityRef`; `tests/agenda.rs`, 256 cases × 300 operations against a model)

### T-02: Timers
- **Deliverable:** Insert, cancel and re-sample by dependency scope (epoch-keyed)
- **Acceptance:** Timer fires at its own tick in every mode
- **Status:** [x] done (`2781fe9`: `StepCtx::schedule`/`cancel`, `SystemModel::fire`/`resample_timer`, `Engine::submit_input`; `tests/agenda_events.rs`: every mode, scoped re-sampling by epoch, refusals)

### T-03: Lifecycle command queues
- **Deliverable:** Typed commands applied in `(phase, PersonId)` order by owners
- **Acceptance:** A non-owner cannot apply a command directly
- **Status:** [x] done (`2781fe9`: `rh-core::command`, `AccessSpec::sends`/`applies`, registration rule `command`; `tests/agenda_events.rs`, `rh-registry/tests/real_specs.rs`)

### T-04: Event bus
- **Deliverable:** `EventDraft` → EventId in emission order `(tick, phase, seq)`; causal edges
- **Acceptance:** V-DET-12: EventIds strictly increase; no event before the current instant
- **Status:** [x] done (`8fae55e`, `2781fe9`: `rh-events`, the per-level merge, rule `event-kind`; V-DET-12 over 16 random worlds and on the golden event log `tests/golden/life_world_events.txt`, with a back-dated positive control)

### T-05: Invariant framework
- **Deliverable:** Per-system invariants run in Bookkeeping; a violation is fatal with a crash artifact (GOV-004 §4)
- **Acceptance:** An injected violation produces a crash artifact that replays
- **Status:** [x] done (`2781fe9`: `rh-core::crash`, invariants in `Bookkeeping`, `Engine::replay`; `tests/invariants.rs` writes `artifacts/crash/<timeline>_<tick>.json` and replays it; committed artifact `tests/golden/crash_stock_seed3.json` with an altered-instant positive control)

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-006` |

## Developer notes (AGT-002-CORE, 2026-10-16)

Decisions taken in the sprint. **Architect ruling, 2026-10-16: D1–D8 accepted** (D5 at 70%, to be revisited if a real system needs shared event kinds; D7 on condition that the default is a `SimError` that poisons the engine, not a panic, which it is; D3 is an engine API, documented in `05.1` §2 next to the merge rule).

| # | Decision | Why | Confidence |
|---|---|---|---|
| D1 | Within a phase, due timers run in a **timer pass** before the firing bindings (canonical order; each owner once, at the level of its first binding in the phase, taking its timers in key order; restated over bindings after SPR-005 DEF-7) | A timer fires at its own tick, often between pulse boundaries; one pass keeps the canonical order of SPR-005 | high |
| D2 | Within one system's buffers the merge takes cancellations, timers, events, then commands | A command can then carry the `EventId` of the event that caused it (the death path of `05.1` §2 rule 2) | high |
| D3 | `DraftRef`: a step names its own earlier event drafts; the merge resolves them to `EventId`s (draft-local causes, a command's cause) | A step cannot know an id before the merge (ruling 1), yet consequences must cite their cause (ruling 4) | medium: a mechanism the spec does not list (ADR-0033 §5); it is the smallest that makes causal edges usable |
| D4 | Queue key `(tick, phase, PersonId, seq)`; the owner receives its whole queue the next time it runs; untaken commands stay queued | Ruling 5's `(phase, PersonId)`, led by the submitting instant (a queue spans instants) and closed by the merge counter (unique keys) | high |
| D5 | One registered emitter per event kind (rule `event-kind`) | The single-writer principle applied to events; the specs agree today | medium |
| D6 | The re-sample key includes the owner's `system_id`: `H(seed, salt, system_id, epoch, purpose, entity, t_input, k)` | The full layout of `05.1` §6 item 2; ADR-0025 item 3 lists the other fields | high |
| D7 | `SystemModel` gains provided `fire` and `resample_timer`, defaulting to a loud error | Additive: no SPR-005 signature or generated skeleton changes | high |
| D8 | The crash artifact's JSON is written and read in `rh-core` by hand, and also carries the branch salt and the timeline | No new dependency for plane A; replay needs both | high |

Carried forward:

| Item | To | Why |
|---|---|---|
| `StepCtx::rng` still draws at epoch 0; only re-sample draws carry the entity's epoch | BCK-002 A-29 (Architect), due before the first material input that touches a drawing system | `EntityKey` is an untyped `u64`; keying every draw by epoch needs a kind-tagged key in `StepCtx::rng` |
| A keyframe must hold the command queues, the entity epochs, the queued inputs and the event counter, not only the agenda | SPR-007 (passed on by the Architect) | All are plane-A state between instants (ruling 7 names the agenda) |
| Material-input scopes are explicit entity sets; the host expands a household, settlement or region | the input journal (SPR-007) and the ledger | `rh-core` has no membership tables yet |
| `EventLog::hash` is a stub fold, not yet part of a combined state hash | SPR-007 | Merkle hashes replace both stubs |

Goldens: SPR-005's `stub_world_hashes.txt` is unchanged. New: `crates/rh-core/tests/golden/life_world_events.txt` (`RH_CORE_WRITE_LIFE_GOLDEN=1`) and `crates/rh-core/tests/golden/crash_stock_seed3.json` (`RH_CORE_WRITE_CRASH_GOLDEN=1`).

### Tester interim on 3e71091: rulings and fixes (2026-10-16)

| Finding | Ruling | Fix |
|---|---|---|
| DEF-1 (low): a panicking step leaves no crash artifact | defect | caught at dispatch as a poisoning `SimError::Panic`, with an artifact; tested at 1 and 4 threads |
| DEF-2 (medium): an artifact after material inputs records journal position 0 and replays as `NoFailure` | defect | the artifact records the inputs applied; replay refuses a non-zero position with `ReplayError::JournalRequired` until SPR-007 |
| G1: firing order across owners at one `(t, phase)` | accepted as built | written into `05.1` §4.3; pinned by a test |
| G2: a sibling timer in the same slot cannot be cancelled by another's `fire` | one slot fires together; a cancellation takes effect from the next slot; "dead and acted" settles at the lifecycle queue (BCK-002 A-30) | written into `05.1` §4.3; a test pins it |
| G3: re-sample draws key on the raw entity id only | defect | keyed on `resample_entity_id(EntityRef)` (kind tag and raw id) |
| G4: `EventLog::hash` depends on the drain cadence | defect | a running fold in `EventId` order that draining does not reset |
| G5: a sender whose command owner is unregistered passes registration | defect | refused by rule `command` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [x] All tasks meet their acceptance criteria.
- [x] `bin/check_all.sh` passes; the report is attached.
- [x] Tester verdicts are PASS.
- [x] Architect audit `VER-SPR-006` is PASS.
- [x] No open DEF- against this sprint.
- [x] Merge route matches the CI-detected change class.
