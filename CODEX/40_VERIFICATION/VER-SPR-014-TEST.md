---
id: VER-SPR-014-TEST
title: "Determinism and contracts tester evidence for SPR-014"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing, determinism]
related: [SPR-014, EVO-001, REV-EVO-001, VER-SPR-013-TEST, AGT-003-DET, AGT-003-IF, ADR-0024, ADR-0025, ADR-0033]
created: 2026-10-17
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: SPR-014 meets its acceptance criteria on every task, on the final head `d1e79e4`.** My first pass (on `921d372`) found two defects, both now fixed and verified: DEF-1 (medium, the `rh-contracts` drift test did not test the accessors) and DEF-2 (low, a widow's `person.spouse` was not cleared). Nine gaps remain, none blocking; four of them are mutants that survive every test, with their reachability named in §3. The independent divergence record agrees with the Developer's: against `8f7edb4`, **only `rh-core/engine.meta` differs, at every rung of `solo`, `household` and `hamlet`** (yearly and monthly, 1 and 4 threads), **only 20 `KEYFRAME.state_hash` fields per world differ in channel A**, `smoke` is byte-identical, and the ladders and channel A of `d1e79e4` are **byte-identical to those of `921d372`**. `bin/check_all.sh` is 556 of 556 on a clean clone of `d1e79e4`. Of 58 mutants I planted, the head's gates kill 53; the five survivors are named in §4.

Tested heads (both on `sprint/SPR-014-lifecycle-queues`, PR #83):
- `921d372354eb1f92ebe8868e9fcf22591c759225`, the head the Architect sent first: the full pass, the nightly tier, my own `alloc-record`, the first mutation matrix, DEF-1 and DEF-2 found.
- `d1e79e4c6d3de5406d217bb45a365a1392b01ce6`, the final head (a fast-forward on `921d372`, which merges `main` `3e2e6ef`): `check_all`, area H again, DEF-1 and DEF-2, the adopted tests, the survivors. The only code changes since `921d372` are `rh-contracts` (`08a17fe`), `apply_death` (`2c5624c`, 8 lines) and tests (`72c10e0`, `d1e79e4`); no file under `worlds/` or `crates/rh-bench/baselines/` changed. I did **not** re-run the nightly tier on `d1e79e4` (§5).

Base for every divergence measure: `8f7edb4` (SPR-013). Tester model: Sonnet 5.5 (the Developer's is a different one, GOV-005 §4.2, as AGT-003-DET §3 requires). One session held both roles (AGT-003-DET and AGT-003-IF).

Method: every run used a `git worktree` of the SHA with its own `CARGO_TARGET_DIR`; the Developer's tree was never touched. The probe plan was written from `05.1` §2 rules 2 and 4, §4.3, §9, `EVO-001`, `REV-EVO-001` R1 to R19, `envelope_v0.md` §2, `docs/09` and ADR-0025 before I read any implementation; the Architect accepted it, added the same-instant natural and commanded death case (area E), ruled the cursor-only rebaseline (area H) and the channel-A classification. My oracles are mine: a stdlib per-table ladder comparer (`ladcmp.py`), a stdlib channel-A classifier (`chacmp.py`) written before I looked at `bin/ladder_divergence.py`, a stdlib decoder of the hashed rows written from the layout text (`enc_check.py`), and black-box Rust probes that reuse the Developer's stub harness (`tests/support`) for plumbing only. The probes are on the review branch `review/SPR-014-tester-probes` (`tester-probes/SPR-014/`, commit `b299792`); the Developer adapted them as tests in `72c10e0` and `d1e79e4`.

## 1. Verdicts

| Task | Verdict | Basis |
|---|---|---|
| T-01 delivery to the one draining binding | **PASS** | The inbox reaches the `drained_by` binding and no other job; a timer pass drains nothing; key order; partial takes keep both orders; DRAINS refusals 4 of 4 (§2.1) |
| T-02 event-typed slots | **PASS with GAP-1, GAP-2** | Every `Draft` is rewritten at the merge; my independent row decoder agrees with the layout; -1 for none is now pinned; hash-neutral when empty (§2.2) |
| T-03 dead-person drop, queue age | **PASS with GAP-3** | Five points hold, the ledger-less and no-row cases are now tested; the counter changes no hash; the age invariant trips at `period` and not before (§2.3) |
| T-04 absent owners | **PASS** | The four real-spec worlds, neutral lookups, the refusals; 2 of 2 mutants killed (§2.4) |
| T-05 M2 drain, row insert, single death path | **PASS with GAP-4, GAP-5** (DEF-2 closed) | Same consequence set as a hazard death at 1, 2, 4 and 8 threads; the Architect's same-instant case holds; the spouse is cleared by command and by M2's timer (§2.5) |
| T-06 `rh-contracts` v0 | **PASS** (DEF-1 closed) | One macro list generates the accessors, `COLUMNS` and `ACCESSORS`; 10 of 10 planted faults killed (§2.6) |
| T-07 A-50 | **PASS with GAP-6, GAP-7** | 40 dodge variants of the `env!` rule: no bypass; the draw-kind check holds at all four entry points, the re-sample's included (§2.7) |
| T-08 ladders, gates, docs | **PASS with GAP-8, GAP-9** | Area H as ruled, on both heads; `check_all` 556 of 556; nightly tier (on `921d372`) passes (§2.8) |
| A-60 slice (`1bd05a6`) | **PASS** | Byte-identical before and after on every ladder and channel A; the narrowing is exactly as documented (§2.9) |
| `bin/check_all.sh` whole | **PASS** | 556 of 556 on a clean clone of `d1e79e4`; 546 of 546 on `921d372` (§2.8) |

## 2. Results

### 2.1 T-01: delivery to the one draining binding (AGT-003-DET)

| Probe | Result |
|---|---|
| `v_det_21_delivery_to_the_one_draining_binding` (the Developer's, 3 sizes x 8 seeds x 1, 2, 4, 8 threads), with both same-instant and cross-instant counts asserted non-zero | passes; the D1 control (`mutant-deliver-every-job`) fails with "delivered to a non-draining job" (in `check_all`) |
| My P-A6/A7: one owner, two kinds, `Fast` drained at Day/Agenda and `Slow` at Day/Culture, a sender at Vital sending both for five persons in scrambled order | passes at 1 and 4 threads: 30 `Fast` at Agenda, 30 `Slow` at Culture, 0 in the wrong job, key order strict, queue empty at the end. The Developer adopted the case as `command::tests::a_partial_take_across_two_kinds_keeps_both_orders` |
| DRAINS refusals: I changed `X4`'s generated `DRAINS` four ways and ran the smoke registration | a changed phase, a changed pulse, a missing entry and an extra entry are each refused under rule `spec`, naming the command and both bindings (`drains command Settle at Day/Culture, but its spec's [[applies]] says drained_by = Day/Activity`, and the three others) |
| Mutants | A2 (timer pass takes the queue), A3 (any phase of the pulse drains), A5 (wrong sort) killed by the Developer's tests on `921d372`. A6 (a partial take returns the commands reversed) and A7 (the commands left behind are dropped) survived them, were killed by my P-A6/A7, and are **killed by the adopted test on `d1e79e4`** |

### 2.2 T-02: event-typed slots (AGT-003-DET)

| Probe | Result |
|---|---|
| Developer's `v_det_21_event_slots_carry_the_merged_id` and keyframe round trips at three sizes | pass; B1 (payload fields not rewritten), B2 (timer cause not rewritten), B3 (command cause not rewritten) all killed |
| **Independent row decoder** (`tester_encoding.rs` dumps the hashed rows, `enc_check.py` decodes them from the layout in `05.1` and `envelope_v0.md` §2 only): a world with event id 0 as a cause, an absent cause, and `Event(Known 0)` and `Event(None)` payload fields | `engine.commands`: the `Known` field is `0` (not -1, not none), the absent one is `-1`, and an absent cause adds no `opt` tail; `agenda.timers`: 60 bytes with the `u8 1 ‖ u64 0` tail, **51 bytes with no tail when absent** (R15) |
| Mutants of the encoding | `u8 0` for an absent cause: killed by `rh_snapshot_hash_v1.txt` at `agenda.timers` (the Developer's control) and by my decoder (timer row length 52); cause as id + 1, as `u32`, and before the tombstone byte: killed by `golden`. **None encoded as 0 instead of -1 survived every test on `921d372` and was killed only by my decoder; the adopted `hashed_rows_encode_event_zero_none_and_the_cause_tail_as_the_layout_says` and `an_empty_event_slot_encodes_as_minus_one` kill it on `d1e79e4`** |
| `rh_snapshot_hash_v1.txt`, `gen_hash_golden.py --check`, `rh-snapshot --test golden` | unchanged vectors; pass |
| Smoke's `engine.commands` rows | byte-identical: the smoke ladder equals the base's at 1 and 4 threads (§2.8) |
| Refusals (P-B3): in one step, a wrong field count, a value outside its type, an integer in an `EventRef` slot, an event in an integer slot, a draft that was never emitted (as a field and as the cause), an undeclared kind and a key person that differs from `key_field`, each ignored, then one good command; against a world that sends only the good command | each is refused with its own error variant and field name (`EventSlot` names `ev` and `n`); tables, queues, the event counter and the drop counters are **identical** between the two worlds |
| `EventRef` opacity, as compile-fail doctests in a scratch copy | `as u64`, `u64::from`, `.into()`, `Hash`, `Ord` and use as an `EntityKey` (a draw key) each fail to compile; the same snippets with `==` and `Debug` compile. Nothing in the repository pins this (GAP-2) |
| The re-sample order: `Timer`, `TimerEntry` and `Arg` lost `Ord` and `Hash`, and the re-sample now sorts by `AgendaKey` | Equal to the old order by construction: the old `TimerEntry` order was `(key, timer)` and `key` carries the unique `seq`. The re-sample tests (the Developer's and mine on real M2) pass, and the monthly ladders of the three rungs equal the base's table by table except `engine.meta` (§2.8) |

### 2.3 T-03: the dead-person drop, the owner's drops, the queue age (AGT-003-DET)

| Probe | Result |
|---|---|
| Developer's `v_inv_15_drops_are_counted_and_never_applied` (the script of five persons) | passes; C1 (core drops `key = "any"`), C7 (`dead_key` counted as `dead`), C8 and C9 (not counted) killed |
| **P-C2**: a ledger-less world with a `key = "living"` command | all 15 commands reach the owner; the drop counter is empty. On `921d372` mutant C2 (drop in a ledger-less world) survived every test, because the smoke world's commands are `key = "any"`; **killed on `d1e79e4`** by `probes::key_living_drops_nothing_without_a_ledger` |
| **P-C4**: a `Kill` for a person with **no ledger row** (died on day 2, compacted at its `Bookkeeping`) | dropped `dead_key`, the living `Kill` applied, the `key = "any"` `Bury` of the absent person applied. Mutant C4 (a missing row counts as living) survived on `921d372`; **killed on `d1e79e4`** by `probes::a_key_person_with_no_ledger_row_is_dropped_as_dead_key` |
| Queue age: the Developer's control (a skipped drain) and C5 (`>` instead of `>=`) | killed. The message names the owner, the kind, the sender and the age (`has waited 1440 minutes`) |
| **P-C8**, a `Month`-pulse drainer in a 365-day calendar, a command sent on 1 February after the drain, the 1 March drain skipped | trips at 1 March at 40,320 minutes, the length of February (28 days): GAP-3 |
| Counter outside every plane hash | the Developer's `lifecycle.rs` observer-invariance test passes; the smoke run prints `vitals: dropped nothing`; the three rungs and smoke ladders equal the base's |

### 2.4 T-04: absent owners (AGT-003-IF)

| Probe | Result |
|---|---|
| `v_inv_04_absent_owners_on_the_real_specs`, run with the Developer's table | `{M2}` and `{M2, M7}` register (M7's reads of M6 and T1 dropped with neutral 0); `{M2, T1}` is refused by name; `{M2, M7, T1}` registers, order `[M2, M2, M7, T1]`, every `life_course.*` read is `same_instant` and none is in the lag-snapshot set; T1's `BurnHoldings` to M4 is reported with the exact message |
| A send to a registered owner that does not apply the command | refused: `A1.send: submits command Nope to B1.own, which does not declare that it applies it` (rule `command`) |
| Mutants | D1 (an identity read dropped instead of refused), D2 (the neutral replaced by the type's zero), S3 (an absent-owner send not discarded) all killed |

### 2.5 T-05: M2's `Day`/`Vital` drain, the row insert, the single death path (AGT-003-DET)

| Probe | Result |
|---|---|
| Developer's `death_path.rs` (real M2, X7 as M7, X1 as T1), re-run by me at **1, 2, 4 and 8 threads** (a copy that reads the thread count from the environment) | the same assertions hold at every thread count, on both heads |
| **The Architect's case**: a natural and a commanded death of one person in one slot | passes: M2's natural timer fires first, the sender's fire finds the person dead and drops its timer (`dropped{X1.minds, timer.1, dead}` = 1), one `DEATH`. The commanded path cannot reach `apply_death` here, because M2's timer pass precedes every sender's (name order), so the drop is the sender's |
| **P-E2c** (real M2): X7 and X1 both murder one victim on one day; both timers fire in the one `Vital` timer pass and both `ScheduleDeath` reach M2's single drain | one `DEATH`, one closed life-course row, the second command dropped by the owner (`dropped{M2.demography, ScheduleDeath, dead}` = 1), in 3 seeds. Mutant M3 (a non-idempotent `apply_death`) survived on `921d372`; **killed on `d1e79e4`** by `two_commanded_deaths_of_one_person_in_one_slot_apply_once` |
| **P-E2d**: a victim murdered two days before its pending natural timer | the natural timer later fires for a person with no ledger row and M2 drops it (`dropped{M2.demography, timer.1, dead}` = 1); one `DEATH`. Mutant M6 survived on `921d372`; **killed on `d1e79e4`** by `a_natural_timer_after_a_commanded_death_is_dropped_as_dead` |
| Row insert (the Developer's `insert.rs`) | E1 (unsorted), E2 (other owners' columns zero instead of the default), E3 (a duplicate accepted) killed |
| **DEF-2, on `d1e79e4`** (mine, real M2): A is seeded married to B; B dies **by command** (P-E5, a murder) and **by M2's own natural timer** (P-E5b, with a partner chosen not to die in the window) | in both, after B's row is compacted A's `person.spouse` is 0. The adopted `a_death_clears_the_survivors_spouse_by_command_and_by_timer` passes too. Mutant S1 (no clearing) is killed by it. **DEF-2 is closed** |
| The fix's guard (clear only a pointer that points back) | mutant S2 (clear unconditionally) **survives every test**, mine included (GAP-5) |
| `Marry` | applied both ways and dropped for a dead party by reading; no test applies one (mutants M4, M5 survive: GAP-4); it also overwrites an existing spouse without clearing the former spouse's pointer (GAP-5) |

### 2.6 T-06: `rh-contracts` v0 (AGT-003-IF)

On `921d372` the drift test compared a `COLUMNS` constant with the spec, and the 19 accessors were separate lines with their own column literal and decode table: 7 of 10 faults I planted in `m2.rs` survived (DEF-1). My 19-accessor consumer test (six rows, a distinct value in every column, expected values written from the spec) passed on `921d372`, so the accessors were right, and killed six of the seven.

On `d1e79e4` (`08a17fe`) one macro list generates the 19 accessors, `COLUMNS` and `ACCESSORS`, and `consumer.rs` has `every_accessor_decodes_its_spec_column_on_seeded_rows`. I re-ran all ten faults:

| Fault | `d1e79e4` |
|---|---|
| F1 `father` reads `person.mother` | killed (`every_m2_accessor_matches_the_spec`) |
| F2 `lc_father` reads `life_course.mother` | killed (`the_enums_follow_their_code_lists`) |
| F3 `SEXES` reversed | killed (`every_accessor_decodes_its_spec_column_on_seeded_rows`) |
| F4 the `DeathCause` decode table swaps `Murder` and `Battle` | killed (seeded rows) |
| F5 the `ORIGIN` list reordered | killed (`every_m2_accessor_matches_the_spec`) |
| F6 `alive` inverted | killed (the real-M2 consumer test) |
| F7 every time doubled | killed (seeded rows) |
| F8 an out-of-range enum code wraps instead of failing | killed (seeded rows) |
| F9 `lc_birth_tick` reads `person.birth_tick` | killed (`every_m2_accessor_matches_the_spec`) |
| F10 `lc_sex` reads `person.sex` | killed (seeded rows) |

**DEF-1 is closed.** `lint_arch` passes for `rh-contracts` (plane A, contract); the crate holds no M2 logic.

### 2.7 T-07: A-50 (AGT-003-IF)

| Probe | Result |
|---|---|
| 40 planted variants of the `env!` rule (`lint_forbidden.py --scan`): `env!`, `option_env!`, `std::`, `core::`, `::std::` paths; spaces, tabs and newlines inside `env ! (`; `{}` and `[]` invocations; `concat!`, `stringify!`, raw strings, `\u{}` escapes; `include_str!(env!())`, `include_bytes!(concat!(env!(...)))`; `macro_rules!` wrappers (literal and parameter); `#[cfg(test)]` modules; a comment inside the arguments; `env!(env!("CARGO_PKG_NAME"))`; a second `env!("HOME")` on the line of a permitted one | every dodge that reads a non-package variable is flagged; **no bypass found**. `CARGO_PKG_VERSION` with `env!` and `option_env!` passes; text in comments, doc comments, block comments and strings, and `my_env!`/`env_x!`, pass |
| Conservative refusals | `env!("CARGO_PKG_VERSION", "msg")`, a trailing comma and an escaped `CARGO_PKG_` are refused (the rule accepts exactly one plain literal) |
| Over-wide allowance | `env!("CARGO_PKG_NAME_X_HOME")` **passes**: the allowance is the pattern `CARGO_PKG_[A-Z_]+`, not the set Cargo defines (GAP-6) |
| Draw-kind check (the Developer's `a_draw_of_the_wrong_entity_kind_is_refused_by_name`) | K1 (`rng`) and K2 (`stream`) killed (`chance` goes through `rng`) |
| The re-sample's kind check (the sprint names `ResampleCtx`) | not exercised on `921d372` (mutant K3 survived); my P-G (a region timer under a person purpose, an input scoping the region) is refused by name (`by_person ... region`). **Killed on `d1e79e4`** by `probes::the_resample_checks_the_draw_kind` |
| An event-keyed draw under a person-keyed purpose | refused by name (`by_person ... event`, my P-K4, passes on `d1e79e4`), but **no repository test pins it**: mutant K4 (events accepted) survives every shipped test and is killed by P-K4 (GAP-7) |

### 2.8 T-08: ladders, channel A, gates (AGT-003-DET)

**Independent divergence record (area H, as ruled), on `921d372` and again on `d1e79e4`.** I built `8f7edb4`, `921d372` and `d1e79e4` as release binaries and ran, for `solo`, `household` and `hamlet`, 200 years, yearly and monthly ladders at 1 and 4 threads, and the run directory of each (channel A); for `smoke`, 1,000 years yearly at 1 and 4 threads. `ladcmp.py` compares every table's root at every rung.

| World | Rungs compared (yearly / monthly) | Tables that differ from the base | Rungs where `engine.meta` differs |
|---|---|---|---|
| `solo` | 200 / 2,400 | **`rh-core/engine.meta` only** | 200 of 200 / 2,400 of 2,400 |
| `household` | 200 / 2,400 | **`rh-core/engine.meta` only** | 200 of 200 / 2,400 of 2,400 |
| `hamlet` | 200 / 2,400 | **`rh-core/engine.meta` only** | 198 of 200 / 2,397 of 2,400 |
| `smoke` | 1,000 yearly, 1 and 4 threads | none: **byte-identical** to the base and to the committed baseline | 0 |

- The result is the same for `921d372` and `d1e79e4`, and **every ladder of `d1e79e4` is byte-identical to the same ladder of `921d372`** (12 ladders per world, plus `smoke`). Ladders at 1 thread equal those at 4 threads byte for byte; the same for the base.
- **The head's committed baselines equal my recomputation** (`solo`, `household`, `hamlet` yearly, 1 thread) on both heads.
- **Hamlet's equal rungs** (yearly rungs 34 and 62; monthly rungs 408 = 0033-12, 640 = 0053-04 and 744 = 0061-12): the Developer says these are where an instant already fell on the last day's start. From the base's channel A, `DEATH` and `LIFE_EXIT` events fall on the last day start of a month in **exactly months 408, 640 and 744** and in no other. Confirmed.
- **Channel A** (`chacmp.py`, then `canonical` dumps): solo 2,428 lines, household 2,443, hamlet 2,605 against the base: **20 `KEYFRAME.state_hash` fields differ per world and nothing else**. Every `STEP_DIGEST` is byte-identical, `state_hash` included (the Architect's ruling allowed `STEP_DIGEST.state_hash` to differ too; it does not). Controls: a changed tick, and a changed `observation_hash`, are each flagged as an unexplained line. The canonical dumps of `d1e79e4` equal those of `921d372`.
- The `[C3]` regeneration commit is `ae03dcd` (not the PR's final commit: GAP-8). `git diff 8f7edb4 d1e79e4 -- worlds` touches only the three `*_yearly_200y.ladder` baselines; `smoke`'s is untouched; `git diff 921d372 d1e79e4 -- worlds crates/rh-bench` is empty. The V-PERF-04 instruction baseline is untouched.

**`bin/check_all.sh` whole.** On a clean clone of `921d372` (4 vCPU, while my other probes ran): **546 of 546**, 2,455 s (part core 1,176 s for checks 1 to 446; part golden 1,279 s for 447 to 546), 348 of the checks positive controls. On a clean clone of `d1e79e4`: **556 of 556**, 2,213 s on this machine (part core 456 checks, 1,178 s; part golden 100 checks, 1,035 s), no failure. The Architect reports the CI run on `d1e79e4` (`39086593824`): core 15:04, golden 8:59 (GAP-9).

**Performance** (measured by me with the base and `921d372` release binaries, 3 runs each, 1 thread): `solo` 200 years **0.065 s -> 0.67 s**, `household` 0.068 -> 0.70, `hamlet` 0.09 -> 0.80 (about 10x, as the Developer reports); `smoke` 1,000 years 55.2 s -> 57.6 s (+4.4%; the Developer reports +3.7%). V-PERF-04 (`gate`, `repeatability`, `integrity`) and V-PERF-11 (`alloc-gate`, repeatability and integrity) pass in `check_all` on both heads. I also ran `perf_gate.py alloc-record` myself on `921d372` in a clean worktree: it rewrites `crates/rh-bench/baselines/allocations_v1.txt` **byte-identically** to the committed file (empty `git diff`), so the committed budget is the tool's output. `d1e79e4` changes no file under `crates/rh-bench/`, and its V-PERF-11 gate passes against the recorded budget.

**Nightly tier** (`bin/golden_nightly.sh`, on `921d372`; exit 0). The smoke world, 1,000 sim-years, 12 monthly ladders equal at 1, 2, 4 and 4 threads (N = 4 here) x 3 repeats, each with at least 5% parallel levels (730,000 of 10,323,633 levels, 7%); the yearly cut equals the committed baseline. The same for `solo`, `household` and `hamlet` (12 monthly ladders each of 1,000 years, equal; the yearly cut of a 200-year monthly run equals the committed baseline). V-SCALE-01/04 on the three rungs (replay equal at 4 threads, one `EXTINCTION`, chronicle consistent past it); the V-INV-02 debug soak on `household` and `hamlet` with 0 invariant violations; V-DET-11 on 64 injected perturbations named exactly (month, table, row); the sifter twin on `solo` and `hamlet`; exp/ln v0 exhaustive; the fixture worlds and CRN pairing. **I did not re-run it on `d1e79e4`**: the changes since are the contract crate, 8 lines of `apply_death` that do nothing in a world with no spouse, and tests, and every ladder and channel-A dump of `d1e79e4` equals `921d372`'s.

**Parallelism floors.** Only `smoke` carries `--require-parallel 5` (and passes). `household` and `hamlet` are one-system worlds checked with `--expect-serial`, which still passes: M2's daily instants add single-job levels, so no floor needed restating.

### 2.9 The A-60 slice, `1bd05a6` (AGT-003-DET)

| Claim | Result |
|---|---|
| Byte identity before (`ae03dcd`) and after (`1bd05a6`) | release binaries of both commits run on `solo`, `household` and `hamlet` (yearly and monthly ladders at 1 and 4 threads, and the run directory), and `smoke` 1,000 years at 1 and 4 threads: **every ladder, every channel-A canonical dump (2,427, 2,442 and 2,604 lines) and every `stream/` directory is identical** |
| `draws_golden`, the `m2_system` outputs, the tail snapshots | pass at the heads in `check_all`; I did not run them at `ae03dcd` |
| A job **with** work that changes a guarded value without changing a row count is still refused by name | mutants G1 (`||` -> `&&`), G2 (a timer job treated as light), G3 (an inbox ignored) and G4 (a non-drain-only binding with an empty inbox treated as light) are all killed by the Developer's `guard.rs` |
| The narrowing is no wider than documented | the only light job is a `drain_only` binding with an empty inbox and no due timer (`engine.rs` `ledger_guards`); a timer job and a non-drain-only binding are always full |
| `AccessSpec::drain_only` and rule `spec` | a drain-only binding that drains nothing is refused at registration under `[spec]` (my P-A60 passes) |
| The allocation re-record is the tool's | `check_all` ("a changed allocation baseline equals the tool's output exactly") passes, and a one-allocation-off control fails; the two re-record commits carry `[perf-baseline]` and a `perf-baseline-reason:` line and touch only `allocations_v1.txt`; my own `alloc-record` run reproduces it byte for byte |

## 3. Defects and gaps

### DEF-1 (medium): the `rh-contracts` drift test did not test the accessors (T-06 acceptance). CLOSED on `d1e79e4`

Contract: SPR-014 T-06, "A test compares every accessor's column, type and enum codes with M2's spec"; `docs/13` §4.3. On `921d372`, 7 of 10 planted faults survived `cargo nextest run -p rh-contracts`. On `d1e79e4`, all 10 are killed (§2.6).

### DEF-2 (low): a survivor's `person.spouse` was not cleared when the spouse died (M2 0.2.3). CLOSED on `d1e79e4`

Contract: `M2.demography.odd.toml` 0.2.3, `[[owns]] person.spouse`, "cleared on the spouse's death". On `921d372`, `apply_death` never wrote `person.spouse`: A's pointer kept naming a compacted B. On `d1e79e4` it clears the survivor's pointer when it points back, by command and by timer (§2.5).

### Gaps (test strength and contract text; none is a defect)

Closed by the adopted tests since the first pass: a partial take across two kinds, `key = "living"` in a ledger-less world, a person with no ledger row, `None` encoded as -1, the re-sample's kind check, M2's double commanded death and its late natural timer. What remains:

- **GAP-1: `DraftRef` is `pub struct DraftRef(pub usize)`.** `05.1` rule 4 says a `DraftRef` cannot be read as an integer; the field is readable and forgeable (a scratch doctest compiles `let n: usize = d.0;`). `EventRef` itself is opaque. `submit` refuses a draft index beyond the step's emitted count, not another draft of the same step. Inherited from SPR-006; low. Reachable by any system's code.
- **GAP-2: "an integer in an `EventRef` slot is refused ... at `schedule` by name" cannot happen.** A timer's `cause` is an `EventRef`, so an integer is unrepresentable: refused by the type system, with no runtime error to test. The acceptance wording should say so, and no compile-fail doctest in the repository pins the opacity (I ran eight in a scratch copy: all as expected).
- **GAP-3: the queue-age period is measured at the command's `tick`, the text says at `t`.** `05.1` §9: "the calendar's length of that pulse at *t*"; the code (`check_queue_age`) uses the pulse period containing `tick`. They differ only for `Month` and `Season` drainers in a 365-day calendar (P-C8: tripped at 1 March at 28 days; a March-length period, 31 days, would trip a month later). The code trips at the first missed run, which is what the text means ("a violation means a skipped drain"). The text should say "containing `tick`". Architect's call. Reachable only in a 365-day world.
- **GAP-4: `Marry` is untested, and M7 is unobserved.** Mutants M4 (`Marry` one-way) and M5 (`Marry` ignoring a dead party) survive every test: no test applies a `Marry`. **Not reachable in a P1a world**: the only sender of `Marry` is T1, which no world builds. Mutant M7 (the ledger's `death_cause` written as 0) survives because the dead row leaves at `Bookkeeping`; **reachable only by a system that reads `person.death_cause` between `Vital` and `Bookkeeping`** (M7 at `Polity`, T1 at `Agenda`, neither built).
- **GAP-5: the spouse fix's guard is untested, and `Marry` can leave a stale pointer.** Mutant S2 (clear the survivor's pointer without checking it points back) survives every test. The guard matters once `Marry` can overwrite a spouse without clearing the former spouse's pointer, which the current `marry` does (it writes both new pointers and nothing else); then B's death must not clear A's pointer to C. **Not reachable in P1a** (no `Marry` sender); a test needs a seeded asymmetric pair. The overwrite is SPR-015's N7.
- **GAP-6: the A-50 allowance is a pattern, not Cargo's set.** `env!("CARGO_PKG_NAME_X_HOME")` passes. An externally set `CARGO_PKG_X_HOME` could be read at build time. Low; an explicit list of the names Cargo defines closes it. Reachable by a plane-A author, not by a world.
- **GAP-7: an event-keyed draw under a kind-declared purpose is refused but untested.** Mutant K4 survives every shipped test. **Reachable**: `EventId` is an `EntityKey`, so a step can draw with one; the code refuses it by name today (my P-K4 passes). A five-line test closes it: draw `c.rng(&ByPerson::P, EventId::from_raw(3), 0)` under a `person`-keyed purpose and expect the error naming `event`.
- **GAP-8: the ladders were regenerated in `ae03dcd`, not in the final commit.** `CLAUDE.md`: "only in a C3 PR's final commit, whose subject ends with `[C3]`". Commits followed (a merge of `main`, the A-60 code change, an allocation re-record, then the DEF fixes). I verified the head's committed baselines equal a recomputation on both heads, so nothing is stale. SPR-014's T-08 status line read "in progress" on `921d372`.
- **GAP-9: cost.** The part-core CI time on `d1e79e4` (15:04, the Architect's figure, not measured by me) is above the sprint's 12-minute target and over the 15-minute hard limit by 4 seconds; the Architect says PR #89 moves about 295 s out of core before the audit. The rungs run about 10x slower headless (measured by me, §2.8). Accepted and filed as A-60; neither is a defect.

## 4. Mutation matrix

58 mutants planted in scratch copies of the heads (never in the Developer's tree): 56 on `921d372`, and the two spouse mutants (S1, S2) on `d1e79e4`. "On `921d372`" is the Developer's gates alone (`cargo nextest -p <crate>`) at that head; "On `d1e79e4`" is the head's gates, which include the adopted probes. I re-ran on `d1e79e4` the 19 mutants that had survived, the ten contract faults and two spouse mutants; the 37 that the first-pass gates already killed were not re-run (their tests are unchanged and only grew).

| Area | Mutants | Killed on `921d372` | Survivors on `921d372` | Killed on `d1e79e4` | Survive on `d1e79e4` |
|---|---|---|---|---|---|
| Delivery (A2, A3, A5, A6, A7) | 5 | 3 | A6, A7 | 5 | 0 |
| Slots and encoding (B1, B2, B3, B5, B6, B7, B8, S1, S2, S3) | 10 | 9 | B5 | 10 | 0 |
| Drop and age (C1, C2, C4, C5, C7, C8, C9) | 7 | 5 | C2, C4 | 7 | 0 |
| Absent owners (D1, D2) | 2 | 2 | | 2 | 0 |
| Row insert (E1, E2, E3) | 3 | 3 | | 3 | 0 |
| Guard (G1 to G4) | 4 | 4 | | 4 | 0 |
| A-50 (K1 to K4) | 4 | 2 | K3, K4 | 3 | K4 |
| M2 death path (M1 to M7) | 7 | 2 | M3 to M7 | 4 | M4, M5, M7 |
| Spouse clearing (S1, S2; new) | 2 | | | 1 | S2 |
| `rh-contracts` (F1 to F10) | 10 | 3 | 7 | 10 | 0 |
| DRAINS in generated skeletons | 4 | 4 | | 4 | 0 |
| **Total** | **58** | **37** | **19** | **53** | **5** (K4, M4, M5, M7, S2) |

Also killed by the Developer's controls in `check_all` and re-confirmed: the D1 control (inbox to every job) and the `u8 0` absent-cause encoder.

The five that survive the head's gates (the Architect's "five survivors"; F8, the sixth of the first pass's survivors, is now killed): K4, M4, M5, M7 and the new S2. Reachability is in §3: K4 reachable and correct today (GAP-7); M4, M5 and S2 unreachable in P1a (GAP-4, GAP-5); M7 reachable only by a same-instant reader that no world builds (GAP-4).

## 5. What was not run

- **No composed fuzz over the whole feature set** (row insert x drop x compaction x keyframe x re-sample x absent-owner send x threads in one random world). I composed pairs only: the real-M2 death path at 1, 2, 4 and 8 threads with senders, drops, a timer cause, a re-sample and now the spouse clearing; the Developer's keyframe round trips; the monthly ladders at 1 and 4 threads. The sprint's property tests vary size and seed, and their composition is not mine.
- **No fuzz of the row decoder for truncated tails** (remaining lengths 0, 2, 9 and 11 for SPR-020). My decoder checks the two lengths SPR-014 writes (51 and 60 bytes); SPR-020 re-verifies the others (the sprint's "Merge route").
- **The nightly tier on `d1e79e4`** (§2.8).
- **`draws_golden`, `m2_system` and the tail snapshots at `ae03dcd`** (§2.9).
- **`Marry`**: no test of mine applies one (GAP-4).
- **`--threads N` beyond 4** for the ladders (this machine has 4 vCPUs): thread counts 1, 2, 4 and 8 on the Rust probes; 1 and 4 on the ladders; the nightly tier adds 2.
- **The workflow files under `.github/`**: `check_all` runs the Python and lint gates; I read no workflow.
- The probe sources are on `review/SPR-014-tester-probes`; two additions after that push (P-E5b, the natural-timer spouse case, and P-K4, the event-keyed draw) are in my scratchpad only, as a Tester pushes once.
- Tool note: my first `cargo install cargo-nextest@0.9 cargo-deny@0.20` failed (a version specifier it does not accept) and the first `check_all` run stopped at its tools check; I reinstalled with `--version '^0.9'` and `'^0.20'` and re-ran.
