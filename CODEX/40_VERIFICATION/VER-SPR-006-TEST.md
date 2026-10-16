---
id: VER-SPR-006-TEST
title: "Determinism tester evidence for SPR-006"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing, determinism]
related: [SPR-006, VER-SPR-006, AGT-003-DET]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** The determinism gates of SPR-006 pass on the final head `b3da78a`: ids, agenda state and tables are identical at 1, 2, 3, 4, 7 and 8 threads, across three registration orders, under injected scheduling jitter and when `advance_until` is chopped into random chunks with the thread count changed between calls; the agenda matches an independent sorted-vector oracle over 300 seeds; V-DET-12 holds on a 1,202-event log; an injected violation is fatal, writes a GOV-004 §4 artifact and replays to the same failure across thread-count crossings. `bin/check_all.sh` is green on the head, and 23 of the 24 gate mutations I tried turn it red. **Six defects were found, five were fixed during the sprint and re-verified on the final head; one is open (DEF-6, low): nothing pins the 1,000-record cap of the crash artifact.** Seven contract gaps: four closed by rulings and fixes, three open with recommendations (none blocks G0).

Tested commit: `b3da78a` on `sprint/SPR-006-agenda-events-invariants` (PR #26), on `main` `b0cf3e2`. Earlier heads were probed too: `3e71091` (first), `1c301ec` (after the SPR-005 restack), `0a93bcb`; the intermediate results are in §5. All probing ran on `git archive` copies with a fresh `CARGO_TARGET_DIR` per copy; the repository tree was only read. The probe plan was fixed from the contracts before any implementation was read (scratchpad `probe_plan.md`); the implementation was read afterwards to aim mutations. The Tester runs on a model different from the Developer's (GOV-005 §4.2). Probe suite (a `rh-core` integration test, 28 tests; kept in the scratchpad, not committed), runners `runprobes.sh` and `mut3.py`; `cargo-nextest` 0.9.148 and `cargo-deny` 0.20.2 installed with `cargo install --locked`.

## 1. Verdicts

| Gate | Verdict | Basis |
|---|---|---|
| T-01 agenda order (ruling 2) | **PASS; GAP-6 open** | 300 seeds × 400 random operations (insert at `i64::MIN`/`MAX`, negative times, all 5 entity kinds, `u64::MAX` ids; cancel, double cancel, pop) against a sorted-vector oracle on the key `(time, phase, kind tag, raw id, seq)`: pop sequence, `len()` and `seq` assignment equal at every step; the restored state drains in oracle order. The kind tag orders before the integer. |
| T-02 timers and re-sampling (ruling 3; V-DET-14) | **PASS after DEF-3** | A re-drawn value equals an independently computed `H(seed, salt, system_id, epoch, purpose, entity, t_input, k)` at epochs 1 and 2; epoch counts inputs per entity (two inputs at one instant give epochs 1 and 2); an entity with no timers still counts; a timer due at the input's tick is re-drawn before it can fire; a re-draw at the input instant, same phase or earlier, is refused. |
| Canonical merge (ruling 1) | **PASS** | Six events and timers per instant from six systems on one level, 3 registration orders × threads {1, 2, 3, 4, 7, 8}, 1–4 ms sleeps that depend on id and instant: events and agenda state identical in all 18 runs. Systems with two bindings in one phase plus timers (the SPR-005 DEF-7 restack): identical events, agenda and table hash at 1, 2, 4, 8 threads and both registration orders; the maximum number of concurrent jobs of that system is 1. |
| T-03 lifecycle queues (ruling 5) | **PASS after GAP-5** | Two senders, three persons each, in 3 thread counts × 2 registration orders: the owner receives `(tick, phase, PersonId, seq)` order every time. A second applier, in any of 3 registration orders, is refused at registration with all three violations. A sender whose owner is not registered is now refused (was accepted and failed at run time). |
| T-04 event bus and V-DET-12 | **PASS** | 1,202 events over 200 days at 3 threads, each from steps and timers: ids consecutive; `(tick, phase, id)` strictly increasing; every cause has a smaller id; a draft-local cause resolves; every outcome event carries a tick strictly after the instant that scheduled it. Back-dating is impossible by construction (the draft has no tick) and refused by the log. |
| T-05 invariants and crash artifact (ruling 6; GOV-004 §4) | **PASS after DEF-1, DEF-2, DEF-4; DEF-6 open** | An injected violation stops the run at that instant (4 earlier instants completed, none after); a later `advance_until` returns the same error and runs nothing; the artifact has all the GOV-004 §4 fields and the replay produces a byte-identical artifact for thread crossings 1→1, 8→1, 1→8, 4→2. 1,500 failing instants leave exactly 1,000 diagnostic records, oldest first, the last one the `FATAL` line. Hostile text (quotes, backslashes, control characters, `\u0000`, emoji, U+2028, a forged `"seed"` field) round-trips through the hand-written JSON writer and reader; 8 malformed documents (missing and renamed fields, overflowing integers, trailing text) are refused. |
| Observer invariance of the new machinery | **PASS** | An extra Hour-bound observer system, a run split into random chunks and the thread count changed between calls give the same events and agenda as the plain run. |
| `bin/check_all.sh` | **PASS** | Green on `3e71091`, `1c301ec` and `b3da78a` (baseline run of each mutation batch), 144 checks on the first; the gates named in SPR-006 are in the run and each has a positive control. |
| Mutation of the gate (§4) | **PASS with one survivor** | 23 of 24 mutants turn the gate red; DEF-6 survived; M0 (no change) green. |

## 2. Results (condensed; one row per probe family)

| Probe | Contract clause | Expected | Actual | Verdict |
|---|---|---|---|---|
| A9 oracle (above) | ruling 2 | pop order is a function of the keys | 300 of 300 seeds equal | PASS |
| A3 entity order | ruling 2 | kind tag, then integer | World < Person < Household, so Person(`u64::MAX`) sorts before Household(1) | PASS |
| A10 restore | ruling 7 | a plain sorted dump | `restore` accepts unsorted `entries`; a dump that carries a tombstone on the top entry restores to a state that differs from its input (the top tombstone is normalised away) | GAP-6 |
| B (18 runs) | ruling 1 | ids independent of threads, order, timing | identical | PASS |
| B-DEF-7 (8 runs) | SPR-005 restack | a system's bindings never run at once | max concurrency 1; identical output | PASS |
| C V-DET-12 | ruling 4 | see §1 | 1,202 events | PASS |
| D2/D5 | ADR-0025 items 3, 4 | epoch per entity; input before a same-tick timer | epochs 2 and 1; the old timer never fires, the new one fires at the re-drawn tick | PASS |
| D redraw hash | ruling 3 | value equals the epoch-keyed hash | equal at epochs 1, 2 (with the entity id derived as the `rh-core` docs state, `resample_entity_id`) | PASS (was DEF-3) |
| D past input | V-DET-13 | `PastInput` | `PastInput { at, next }`, nothing queued | PASS |
| E1 queue order | ruling 5 | `(tick, phase, PersonId, seq)` | 18 of 18 runs equal | PASS |
| E3 registration | ruling 5 | non-owner refused | refused in 3 of 3 orders | PASS |
| E8 backlog | 05.1 §2 rule 2 | queue drained | an owner that never calls `take_commands` accumulates 101 commands in 100 days with no invariant firing | GAP-7 |
| F2/F5 crash | ruling 6 | fatal, replays | see §1 | PASS |
| F panics | GOV-004 §2 | artifact for every fatal error | step, fire, `resample_timer`, `invariants`: `SimError::Panic` with an artifact, 1 and 4 threads | PASS (was DEF-1, DEF-4) |
| F replay with inputs | GOV-004 §4 | a replay that cannot be exact must say so | `ReplayError::JournalRequired { position: 1 }` | PASS (was DEF-2) |
| G1 firing order | 05.1 §4.3 | documented | two owners, one slot: fire in canonical binding order, not agenda key order; now documented in 05.1 §4.3 | GAP-1 closed |
| G2 same-slot cancel | ADR-0025 item 4 | documented | a firing timer cannot cancel a sibling due in its own slot (`not pending`); documented as "simultaneous" in 05.1 §4.3 with a pinning test | GAP-2 closed |
| D1 order | ruling D1 | timers before steps | `A.owner fire @1440` precedes both steps at 1440 | PASS (was DEF-5) |
| Hash and drain | 05.1 §6 item 6 | hash independent of consumer | equal after the fix | GAP-4 closed |
| Cancel-heavy | 05.1 §4.3 | no unbounded growth | 100,000 timers cancelled in 0.49 s; the dump holds 0 entries when cancelled in top-first order | PASS (see Not run for random order) |

## 3. Defects

Reproductions run from the repository root of a `git archive` copy; "stub" is the `Stub` of `crates/rh-core/tests/support`.

**DEF-1 (low, fixed in 071b289): a panic in a step left no crash artifact.** Repro on `3e71091`: a stub whose step does `panic!()`, 4 threads, `catch_unwind` around `advance_until`; `crash_artifact().is_some()` was false. Re-verified on `b3da78a` (1 and 4 threads): `SimError::Panic`, artifact present, engine poisoned.

**DEF-2 (medium, fixed in d8ed6bc): an artifact from a run with a material input claimed to replay.** Repro on `3e71091`: schedule a timer, `submit_input` for its entity, trip an invariant later; the artifact has `journal_position` 0 and `replay` on a fresh engine returned `NoFailure`. Now the position is the count of applied inputs and replay returns `JournalRequired`.

**DEF-3 (medium, fixed in 0a93bcb): the gate did not pin the epoch in the re-sample key.** Repro on `1c301ec`: change `SystemKey::new(.., e.system_id, epoch)` to `0` in `engine.rs`; `bin/check_all.sh` stayed green (144). My probe (a re-sample handler records `ctx.epoch()` and `ctx.draw(0)`; compare with `SystemKey::new(7, salt, system_id("A.x"), epoch).stream_key(purpose, entity).draw(t_input, 0)`) fails on the mutant and passes on the code. The same mutant now dies in nextest.

**DEF-4 (low, fixed in af086ce): the DEF-1 fix covered only `step` and `fire`.** Repro on `0a93bcb`: a stub whose `resample_timer`, or whose `invariants`, panics, 1 thread: `advance_until` unwound with no artifact. On `b3da78a` both return an error with an artifact; removing the guard from either call, or from `step`, turns nextest red (M22–M24).

**DEF-5 (medium, fixed in b3da78a): nothing pinned the ruling "timers due at `(t, phase)` fire before any system steps".** Repro on `1c301ec`: in `engine.rs` push the timer pass after the binding pass; `bin/check_all.sh` stayed green. My probe (an owner with a timer due at 1440 and a step at 1440, plus a second system) fails on the mutant. On `b3da78a` the same mutant dies in nextest (M19).

**DEF-6 (low, open): the 1,000-record cap is not tested.** Repro on `b3da78a`: set `DIAGNOSTIC_RECORDS` to `1_001` in `crates/rh-core/src/crash.rs`; `bin/check_all.sh` stays green. GOV-004 §4 says "the last 1,000 diagnostic records". The probe (`f3_diagnostics_cap_1000_and_order`: a violation at the 1,500th instant; `len == 1000`, last record contains `FATAL`) fails on the mutant. Recommendation: add that test to `tests/invariants.rs`; it takes 0.04 s. Confidence: high.

## 4. Mutation matrix on `bin/check_all.sh` (head `b3da78a`)

M0 (no change) is green. "Gate" is the first red check of the fail-fast run.

| # | Mutation | First red gate |
|---|---|---|
| M1 | cancel does not tombstone (a cancelled timer fires) | nextest |
| M2 | merge walks the level's jobs in reverse | nextest |
| M3 | event tick = instant − 1 | nextest |
| M4 | event log never refuses a back-dated append | nextest |
| M5 | re-sample key uses epoch 0 | nextest |
| M6 | re-sample entity id = raw id only (G3 reverted) | nextest |
| M7 | an input does not bump the epoch | nextest |
| M8 | command queue key drops the `PersonId` | nextest |
| M9 | a violated invariant is ignored | nextest |
| M10 | crash artifact tick + 1 | nextest |
| M11 | `HashMap` iteration in the agenda | `lint_forbidden` (V-DET-19) |
| M12 | `schedule()` accepts the current instant and phase | nextest |
| M13 | replay ignores the error text | nextest |
| M14 | `pop_due` ignores the phase | nextest |
| M15 | a second applier of a command is accepted | nextest |
| M16 | one line of the golden event log changed | nextest |
| M17 | the SPR-006 gate line deleted from `check_all.sh` | pytest (a Python test failed; I did not trace which) |
| M18 | every test in `tests/invariants.rs` ignored | the T-05 replay positive control (a control that expects a failure now sees none) |
| M19 | timer pass after the binding pass (D1 reversed) | nextest |
| M20 | a past material input accepted | nextest |
| M21 | `DIAGNOSTIC_RECORDS` = 1,001 | **none: green** (DEF-6) |
| M22–M24 | `catch_unwind` guard removed from `resample_timer`, `invariants`, `step` | nextest |

Mutations that tripped `cargo fmt` or clippy first are not counted: they were re-written to be clean (the runner formats after patching).

## 5. History across heads

| Head | What changed for the Tester | Result |
|---|---|---|
| `3e71091` | first look; check_all green (144); all gating probes pass | DEF-1, DEF-2, GAP-1…5 reported; the matrix started |
| `1c301ec` | SPR-005 DEF-7 restack (a binding is the scheduling unit; the timer pass walks nodes) | two-binding probe passes; **DEF-3 and DEF-5 found** by mutants that survived |
| `0a93bcb` | rebased on `main`; fixes for DEF-1, 2, 3, G3, G4, G5; A-28 fail-safe (`previous_pulse` and Hour `start_of_day` reads poison the engine) | all earlier probes pass; **DEF-4 found** |
| `b3da78a` | `guard()` around every call into system code; D1 test | DEF-4, DEF-5 verified fixed; **DEF-6 found** |

## 6. Contract gaps

| # | Gap | Recommendation | Confidence | Status |
|---|---|---|---|---|
| GAP-1 | At one `(t, phase)` timers fire in canonical binding order of their owners, not in agenda key order, though ruling 2 calls pop order "a pure function of the keys". | Document that the key orders each owner's timers only. | High | Closed: 05.1 §4.3 |
| GAP-2 | A firing timer cannot cancel a sibling due in its own `(t, phase)` slot, so a person's same-slot death and marriage both fire. | Settle at lifecycle application (BCK-002 A-30). | High | Closed: documented, pinning test |
| GAP-3 | `H`'s `entity_id` is the raw integer, so `Person:5` and `Household:5` shared a stream. | Tag the kind. | High | Closed: `resample_entity_id` |
| GAP-4 | `EventLog::hash()` was a function of the held events, so a consumer's drain cadence changed it. | A running fold. | High | Closed |
| GAP-5 | A sender whose owner is not registered was accepted at registration (ruling 5 says registration) and died at run time with a misleading text. | Refuse at registration. | Medium: a partial world may be legitimate while owners lack code | Closed: rule `command` |
| GAP-6 | `Agenda::restore` accepts an unsorted `entries` list and a tombstone on the top entry, and `state(restore(x))` can differ from `x`. SPR-007 will hash the dump. | `restore` should validate the canonical form (sorted keys, no top tombstone) and refuse otherwise. | Medium | Open |
| GAP-7 | A command queue that its owner never drains grows without bound and no invariant notices (101 in 100 days). | An invariant on the age or length of each queue, with the owner's declared pulse as the allowance. | Medium | Open |
| GAP-8 | `StepCtx::rng` still draws at epoch 0 (BCK-002 A-29) and its `entity_id` is the raw id, which is untagged for kinds without a typed id (Household, Settlement). | Tag the kind in the draw key when those kinds get ids (C3 for the RNG layout). | Medium | Open, carried forward |

## 7. What was not run

- **Cross-platform and the nightly tier.** Linux x86-64 only; no Windows or macOS; no nightly property sweep. V-DET-12 ran over one busy world and the Developer's 16 random worlds, not 10^5 worlds.
- **Long horizons.** The 3 repeats × 50 years ladder of V-DET-01/02 belongs to the golden-run sprint; here 120–200 simulated days per probe and the Developer's two-year golden.
- **Timers near `SimTime::MAX` end to end.** The agenda is probed at `i64::MIN`/`MAX`; an engine run to `i64::MAX` steps every bound pulse and cannot finish.
- **A cancellation-heavy agenda in random order.** Only the top-first order was measured (0 leftover entries); tombstones deep in the heap stay in the dump until their time (a keyframe-size question for SPR-007).
- **`kill -9` during a crash-artifact write.** `rh-core` has no I/O; the host writes the file and no host exists yet.
- **Mutations inside the merge** (D2: cancellations → timers → events → commands within one system) were not made; the order is exercised by the ladder but not mutated.
- **Viewer and console fuzz** (V-DET-06/07): no console exists yet.
