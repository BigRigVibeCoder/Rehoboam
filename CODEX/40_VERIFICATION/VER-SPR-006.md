---
id: VER-SPR-006
title: "Audit of SPR-006"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-006, VER-SPR-006-TEST]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** PASS. SPR-006 adds scheduled outcomes to the core:
> - an agenda of timers, keyed by `(time, phase, entity, seq)`, with lazy cancellation and epoch-keyed re-sampling (ADR-0025);
> - lifecycle command queues applied by their owners;
> - an event bus whose EventIds follow emission order and refuse back-dating (V-DET-12);
> - per-system invariants in Bookkeeping, whose violation writes a crash artifact that replays (GOV-004 §4).
>
> Everything a parallel step emits goes into per-system buffers that merge in canonical order before any id is assigned, so results are identical at every thread count. The Tester found 3 defects and 5 gaps; all are fixed, ruled or carried to a named backlog item. Change class C3. The Architect merges (GOV-005 §6).

# VER-SPR-006 — Audit of SPR-006

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh` | PASS (144 checks) | Developer and Tester at `b3da78a`; Architect at `0a93bcb`, `b3da78a` and the audit commit |
| CI on GitHub-hosted runners | PASS at `b3da78a` (run 38894836509) | PR #26 |
| Change class | C3: determinism machinery (event order and the agenda are part of the hash contract); no golden world exists yet | — |

## 2. Tester verdicts
| Round | Head | Result |
|---|---|---|
| 1 | `3e71091` (stacked on SPR-005 `f89c791`) | Determinism held on every probe: the thread, registration-order and jitter ladder; agenda against an oracle on 300 seeds; queue order; V-DET-12 on a 1,202-event log; crash replay across thread counts. DEF-1 (low): a panic left no artifact. DEF-2 (medium): an artifact from a run with inputs could not replay. Gaps G1 to G5 |
| 1b | `1c301ec` (restacked on SPR-005 `fc635dc`) | Restack probes PASS: two bindings plus timers in one phase are identical at 1, 2, 4 and 8 threads and in both registration orders, and one system never has two jobs at once. DEF-3 (medium): a re-sample key mutated to epoch 0 survived every gate |
| 2 | `0a93bcb` → `b3da78a` (rebased onto main `b0cf3e2`; report `5301760`) | **PASS.** DEF-4 and DEF-5 fixed at `b3da78a`. Every determinism gate passes: threads 1, 2, 3, 4, 7 and 8 × 3 registration orders plus jitter; the agenda oracle on 300 seeds; V-DET-12 on 1,202 events; crash replay across thread crossings; observer and chunking invariance. 23 of 24 gate mutants die. DEF-6 (low, an untested artifact cap) is carried to SPR-007 T-02. GAP-6 to GAP-8 are open and do not block G0 |

## 3. Rulings (Architect)
| # | Question | Ruling |
|---|---|---|
| Pre-sprint 1–7 | The merge, the agenda key, re-sampling, EventIds, the queues, the crash artifact, a serialisable agenda | SPR-006 v1.1.0 |
| D1 | Timers versus bindings within a phase | Timers due at (t, phase) fire before any binding. Each owner fires once, at the level of its first binding, in key order |
| D2 | One system's merge order | Cancellations, then timers, then events, then commands |
| D3 | Causes inside one step | `DraftRef`, resolved to EventIds at the merge (`05.1` §2) |
| D4 | The queue key | `(tick, phase, PersonId, seq)` |
| D5 | Shared event kinds | One emitter per kind, enforced at registration. Confidence 70%; revisit if a real system needs a shared kind |
| D6 | The re-sample key | The full layout of `05.1` §6 item 2, including `system_id` |
| D7 | `fire` and `resample_timer` defaults | A poisoning `SimError`, never a panic |
| D8 | The crash artifact | Hand-written JSON in `rh-core`, plus salt and timeline, with no wall time |
| G1 | Firing order across owners | Key order within an owner; canonical binding order across owners (`05.1` §4.3) |
| G2 | Cancellations within one slot | One slot fires together, against one state; a cancellation takes effect from the next slot. "Dead and acted" conflicts are settled at lifecycle application (BCK-002 A-30) |
| A-28 first step | Inexact lags | `previous_pulse`, and `start_of_day` on an Hour binding, poison the engine from construction with `Unsupported`. Accepted deviation: the engine poisons rather than `Engine::new` returning a `Result`, so SPR-005's API stays unchanged |

## 4. Defects and gaps: disposition
| Item | Disposition |
|---|---|
| DEF-1 a panic in a step left no crash artifact | Fixed (`071b289`): `SimError::Panic` with an artifact, at 1 and 4 threads |
| DEF-2 an artifact from a run with inputs could not replay | Fixed (`d8ed6bc`): the journal position is the count of applied inputs, and replay refuses a non-zero position with `JournalRequired` until SPR-007 wires the journal in |
| DEF-3 the re-sample epoch was not pinned by any test | Fixed (`0a93bcb`): an independent redraw test at epochs 1 and 2 for two entity kinds |
| G3 re-sample keyed on the raw entity id | Fixed (`a75c333`): a tagged hash of (kind, raw) |
| G4 the event-log hash depended on drain cadence | Fixed (`556af68`): a running fold in EventId order |
| G5 a sender with an unregistered owner passed registration | Fixed (`95fa148`): rule `command` |
| G1, G2 | Ruled and documented (`9d97e97`); a test pins them |
| DEF-4 the DEF-1 fix covered only `step` and `fire` | Fixed (`af086ce`): one `guard` around every call into system code |
| DEF-5 nothing pinned D1 | Fixed (`b3da78a`): a timer's write is seen by a `same_instant` reader in the same slot; the test dies when the passes are reversed |
| DEF-6 (low) the GOV-004 §4 cap of 1,000 diagnostic records is untested | Carried to SPR-007 T-02, which rewrites the artifact format |
| GAP-6 `Agenda::restore` accepts non-canonical dumps | Carried to SPR-007 T-02: restore validates the canonical form (sorted keys, no top tombstone) and refuses otherwise; `state(restore(x)) == x` is tested |
| GAP-7 an undrained command queue grows silently | BCK-002 A-31: an invariant on each queue's age or length, with the owner's pulse as the allowance; due with the first real lifecycle command (M2, P1a) |
| GAP-8 `StepCtx::rng` at epoch 0 | BCK-002 A-29 |

**Escapes found later:** none so far.

## 5. Acceptance criteria
| Task | Criterion | Met |
|---|---|---|
| T-01 | Deterministic order with ties; property test | Yes (256 cases × 300 operations against a model; the Tester's oracle on 300 seeds) |
| T-02 | A timer fires at its own tick in every mode | Yes (every mode; epoch-keyed re-sampling pinned independently) |
| T-03 | A non-owner cannot apply a command | Yes (rule `command`, at registration) |
| T-04 | V-DET-12: EventIds strictly increase; no event before the current instant | Yes (16 random worlds; a golden event log; a back-dated control) |
| T-05 | An injected violation produces a crash artifact that replays | Yes (a committed artifact, an altered-instant control, panics included) |

## 6. Carried forward
- SPR-007 T-02:
  - a keyframe holds the agenda, the command queues, the epochs, the queued inputs and the event counter, and `state_hash` covers them;
  - the crash artifact gets the journal head and its last keyframe;
  - DEF-6 (a test pinning the artifact cap);
  - GAP-6 (a canonical `restore`).
- BCK-002 A-28 (exact lags), A-29 (draws keyed by entity epoch), A-30 (same-slot conflicts at lifecycle application), A-31 (queue backlog invariant; filed in this audit).

## 7. Verdict
| Field | Value |
|---|---|
| Verdict | **PASS** |
| Merge route | The Architect merges (GOV-005 §6) |
| Escapes found later | — |
