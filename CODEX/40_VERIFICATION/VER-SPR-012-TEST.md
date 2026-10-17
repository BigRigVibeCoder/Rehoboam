---
id: VER-SPR-012-TEST
title: "Determinism and statistics tester evidence for SPR-012"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing, determinism, validation]
related: [SPR-012, SPR-011, VER-SPR-011-TEST, VER-SPR-008-TEST, AGT-003-DET, AGT-003-STAT, ADR-0025, ADR-0026]
created: 2026-10-17
updated: 2026-10-17
version: 1.0.0
---

> **BLUF (head `9e5a6c25510748bfba6bc5d6a50305494cddd7ec`, PR #56): T-05, T-06, T-07 and T-08 meet their acceptance criteria, and so do the A-47 and A-49 items. I found no defect.** `bin/check_all.sh` is **358 of 358** on a clean clone of the head, and pytest passes (624) under the three CI event shapes. My own implementation of M2's rung-0 algorithm (S8 to S12), written from the spec on the repository's independent RNG reference, predicts the death instant of **every one of 33,711 persons** in the runs I compared, including 783 scoped re-samples, on both calendars and at 1, 2, 4 and 8 threads. The fixed-point schedule agrees with my own `fractions` reference on all 3,091 committed golden vectors, on 105,472 further random and edge cases, and on all 4,259,841 `Q16_16` inputs for the q(15) band identity. The 32-seed q(15) check passes against the Architect's conditional null in every cohort I ran (5 cohorts of 8 to 32 seeds, 88 seeds in all). **Seven contract gaps, none a defect.** The one that matters: `resample_timer`'s draw wiring is pinned by no test at all, solo's ladder included. Two mutants of it survive every gate, and my differential catches both (GAP-1). A second gap is structural: thread-count invariance on `solo` cannot fail, because no scheduler level ever holds two jobs (GAP-2).

Tested head: `9e5a6c25510748bfba6bc5d6a50305494cddd7ec` on `sprint/SPR-012-solo` (includes a merge of `main` at `67a78c0`). Probes ran on a clean clone and on `git archive` copies of that SHA, each with its own absolute `CARGO_TARGET_DIR`; the Developer's tree was never touched. The probe plan was written from the contracts (SPR-012 T-05 to T-08, SPR-011 rulings 3 to 17, the frozen M2 spec 0.2.0 `[algorithm.mortality_rung0]`, `specs/demography/`, R12 §9, `05.1` §2, §4.2, §6, §9, `05.2` §1, §4, §6, ADR-0025, ADR-0026, `docs/09` V-DET-01/02/05/12/19, V-INV-01/08/11, V-DATA-04/05, V-SCALE-03, V-GOLD-01, V-SF-02) before the branch was opened. The Architect's rulings of 2026-10-17 were applied when they arrived: the Table 19 e₀ is the recomputed one, the q(15) null is the conditional one, and K = 1 means within the spec's 2^10 units. Tools: rustc 1.97.0 (pinned), cargo-nextest 0.9.148, cargo-deny 0.20.2, valgrind 3.22.0, iai-callgrind-runner 0.16.1, a 4-core x86-64 host. My probes live in the scratchpad (`PROBE_PLAN_SPR-012.md` and `probes/`); none is committed. This file is the only thing I committed.

## 1. Verdicts

| Task | Verdict | Basis |
|---|---|---|
| T-05 the schedule in fixed point | **PASS** | 3,091 of 3,091 golden lines and 105,472 further cases equal my reference; the q(15) band, K's monotonicity and the e₀ recomputation hold on every `Q16_16` input (§2.1) |
| T-06 `rh-worldgen` | **PASS** | 70 worlds, 4,657 founders: ids, sex, ages, birth ticks and all 19 columns equal my oracle; forced and double collisions; the baseline's 19 world-column roots reproduce from my genesis (§2.2) |
| T-07 `M2.demography` | **PASS with GAP-1, GAP-3** | Every death instant of 33,711 persons predicted exactly (§2.3); q(15) inside the conditional band in all cohorts (§2.4); V-DATA-04 over 1,000 years (§2.5) |
| T-08 `rh-cli run <world>`, the `solo` ladder | **PASS with GAP-2, GAP-4** | Ladders equal across 21 monthly 1,000-year runs, 1 to 64 threads, two builds; resume equals the continuous run; the baseline gate's controls hold; `--strict` refuses `solo` (§2.6) |
| A-47 (spec loaders in system crates) | **PASS with GAP-5** | 10 planted forms fail or pass as the rule says (§2.7) |
| A-49 (SPR-011 gate and test gaps) | **PASS** | GAP-1, 2, 3 and 9 are killed by mutants; GAP-4 and 6 are documented (§2.7) |
| `bin/check_all.sh` whole | **PASS** | 358 of 358; report time summed 914 s serial (§2.8) |
| pytest under CI's variables | **PASS** | 624 passed under `pull_request`, `push` and `workflow_dispatch` shapes |
| `smoke` unchanged | **PASS** | `worlds/smoke` is identical to `main`; the 1,000-year yearly ladder is byte-equal to its baseline from the portable and the `target-cpu=native` builds |

## 2. Results

### 2.1 T-05: the schedule (AGT-003-STAT)

My reference is `probes/det_ref.py`: Python integers and `fractions`, written from the spec text (S0 to S11), not from `bin/gen_m2_reference.py` or `selftest_m2_rung0.py`. It reproduces the spec's seven `check_values` and puts `Q16_16(23.441086) = 1536235`, `Q16_16(36.569855) = 2396642`, and K at e₀ = 43.45915 at 2^32 − 569.

| Probe | Result |
|---|---|
| The committed `m2_schedule_v1.txt` against my reference, line by line (71 standard, 25 schedule, 25 grid, 424 survivor, 1,888 monthly q, 336 day, 315 re-sample lines; Table 19 checked separately) | 0 mismatches |
| Differential against the Rust API, 105,472 cases: 166 `solve_k`, 82,458 `survivors_at`, 16,848 `interval_q`, 3,000 `death_day`, 3,000 `resample_day`. Inputs: 150 random e₀ bit patterns plus both bounds ±1, both band edges ±1, the seven T-05 values; both calendars; every grid age in minutes ±1; ages past 100 years; draws 0, 1, 2^63, 2^64 − 1 and random; months of 28 to 31 days. Out-of-range e₀ and negative ages | 0 mismatches; out-of-range e₀ and negative ages are errors, not wraps |
| Exhaustive sweep of all 4,259,841 `Q16_16` e₀ in [20, 85] | K is non-increasing; q(15) ∈ [0.35, 0.55] (exact integer comparison) for exactly the contiguous bits [1536235, 2396642] (860,408 values); the worst error of the recomputed e₀ is 1,128 · 2^-34 years (6.6 · 10^-8 years, bound 0.06) |
| Table 19 | The golden file's inputs equal the e₀ I recompute from each α row (6 of 6, bits equal) and the worst difference from the published l per 10,000 is 0.513 (bound 1). With the published two-decimal e₀ the worst is 14.9; per the Architect's ruling that is not the criterion, and the Developer's file does not use it |
| K at the standard's own e₀ (43.45915) | K differs from 2^32 by 569 units, inside the spec's 2^10 bound |

### 2.2 T-06: `rh-worldgen` (AGT-003-DET)

The oracle is `probes/t06_oracle.py`. It recomputes ids, draws, sex, ages and birth ticks with `bin/gen_rng_golden.py`, the repository's independent reference (SPR-002), and never calls the Rust RNG.

| Probe | Result |
|---|---|
| 66 worlds, 4,625 founders: the four real manifests (`solo`, `household`, `hamlet`, `uz_job`) and 62 synthetic worlds (1 to 400 founders; seeds 0, 1, 2^64 − 1 and random; both calendars; roles, sexes and ages mixed; an empty world) | all 19 columns equal; ids unique and ascending as `u64` |
| 4 forced collisions (4 seeds, ids `founder(seed, 0/2/1/1/1/5)` pre-issued, so founder 1 needs two rehashes), 24 rehash steps in all; each genesis run three times | equal to my oracle; identical on every run |
| Ruling-10 order on the real manifests | cast, then children and servants, then the rest of the population. `household` 2 cast + 3 children; `hamlet` and `uz_job` 6 cast, 10 children, 30 servants, 14 commoners |
| `solo`: Ada | female, birth tick −13,236,200, so aged 25.53 years at tick 0 |
| The baseline's rung-1 table roots | 19 of 19 world-column roots in `solo_yearly_200y.ladder` reproduce from my genesis values and the repository's independent XXH3 and plane-hash reference |
| Keyframe cuts anywhere (V-INV-11, V-DET-05): 24 worlds, both calendars, 10 to 59 founders aged 90 to 99, cuts at random minutes in the first 1.25 years (9 with death timers pending, all 24 with queued inputs) | the loaded engine equals the cut state, and equals the continuous run at every instant to year 8 |
| Founders aged 0 | birth ticks in (−1 year, 0]; age 0 with offset 0 gives birth tick 0, not negative (a boundary, probability 1 in 518,400 per founder; not a defect) |

### 2.3 T-07: the system, differential (AGT-003-DET)

`probes/t07_oracle.py` reimplements S8 to S12 and ADR-0025's re-sample for one person at a time on the independent RNG reference (system id `M2.demography`, purpose `death_time`, entity `person.id`, epoch, `t`, `k`; the re-sample draws under `h64_words(tag("rehoboam/rh-core/resample-entity/v1"), [1, id])` at the epoch after the input). It predicts each person's death instant from the genesis tables and the month starts, then compares with the engine's `DEATH` events, life-course exits and ledger. Every run passed with 0 mismatches.

| Set | Worlds | Persons | Re-samples (dropped / re-timed) | Notes |
|---|---|---|---|---|
| Random worlds, 0 to 3 inputs | 40 | 553 | 1 | 1 to 60 founders, all e₀ extremes, ages 0 to 99, both calendars, 1 and 4 threads |
| Old founders, inputs near month starts and day starts | 250 | 9,960 | 401 (244 / 157) | 7 inputs at the timer's own instant, 1 on the month's last day |
| Several inputs at one instant, overlapping scopes | 60 | 2,400 | 324 (146 / 178) | 14 ties at the timer's instant |
| 2,000 founders, ages 0 to 255, at 1, 2 and 8 threads, both calendars | 8 | 12,018 | 0 | the traces at different thread counts are byte-equal |
| 3,000 founders aged 0, 1,000 years, both calendars (V-DATA-04) | 2 | 6,000 | 0 | §2.5 |
| Two earlier, smaller re-sample sets | 70 | 2,780 | 57 | same checks; kept for the total |

The checks inside each comparison, all passing: one k = 0 draw per living person per month whatever q (the predicted instants only agree if every draw is keyed as stated); no death at a Month-pulse instant and every death on a day start; `DEATH` has the deceased as its only subject, no causes, payload `[1]`; `EXTINCTION` exactly once, right after the last `DEATH`, caused by it, and the last event of the run; event ids strictly increase and ticks do not decrease; every life-course row is closed once with kind `death`, the death instant and cause `natural`; the ledger is empty; the run continues to its end after extinction. A material input at a month start raises the person's epoch before that month's draw, and my oracle's reading (the draw at that instant uses the new epoch) matches the engine. Deaths in the same instant come out in ascending `PersonId` order in all 21,495 same-instant groups (largest 35). The day of death is uniform over the month's day starts other than the pulse: in 123,690 deaths of the q(15) cohort, days 1 to 29 hold 4,161 to 4,405 each (χ² = 20.4, df 28), and day 0 holds none.

Refusals (read from the code and run by the Developer's `alpha_and_beta_other_than_one_are_refused_by_name`; mutants D01 and D02 below): `brass_alpha` set, β ≠ 1 including 1.00002 (65,537 bits), and e₀ outside [20, 85] are refused with named errors.

### 2.4 T-07: q(15) over seeds (AGT-003-STAT)

The null is the Architect's conditional form: deaths before age 15 against Σ(1 − l(15)/l(xᵢ)) over the genesis ages xᵢ ∈ [0, 1) of the founders (read from the run's own genesis tables), the exact Poisson-binomial variance, and a 99.9% two-sided band from the exact distribution (`probes/qband_oracle.py`, `probes/q15_run.py`). My exact month-and-day-start discretisation differs from this null by at most 1.4 · 10^-5 per founder, so I used the Architect's form. 10,000 founders aged 0 per seed, seeds of my own (not the Developer's 1000 to 1031).

| e₀ | Seeds | In band | Mean z (se) | sd z | Σz² (df) |
|---|---|---|---|---|---|
| 30 | 7000 to 7031 | 32 of 32 | +0.15 (0.18) | 0.93 | 27.5 (32) |
| 23.441086 (band's low edge) | 7200 to 7231 | 32 of 32 | −0.06 (0.18) | 0.95 | 28.1 (32) |
| 23.441086 | 7100 to 7107 | 8 of 8 | −1.03 (0.35) | 0.66 | 11.6 (8) |
| 36.569855 (high edge) | 7100 to 7107 | 8 of 8 | −0.70 (0.35) | 0.48 | 5.5 (8) |
| 60 | 7100 to 7107 | 8 of 8 | −0.32 (0.35) | 0.82 | 5.5 (8) |

The 8-seed cohort at the low edge leaned low (mean z −1.03, about 2.9 se); a fresh 32-seed cohort at the same e₀ gives −0.06. I take the first as chance and report it rather than hide it. Non-ergodicity: every seed is inside its band, so the share of seeds in band is 100%. The plain null 1 − l(15) = 0.444 (4,443 deaths) fails every one of the 32 seeds at e₀ = 30, for the stated reason: the observed rate is about 0.347.

**Power of the per-seed band** (`probes/q15_power.py`, shift of the null mean under a mutated schedule, in units of the per-seed sd; "pooled" is the z of the mean over 32 seeds):

| Mutated schedule | e₀ = 23.4 | e₀ = 30 | e₀ = 36.6 | Caught by the per-seed 99.9% band? |
|---|---|---|---|---|
| β = 1.05 | −0.52 sd | −0.76 sd | −0.98 sd | no (needs 3.29 sd); pooled z −2.9 to −5.6 |
| β = 0.95 | +0.53 sd | +0.77 sd | +1.00 sd | no; pooled +3.0 to +5.6 |
| β = 1.02 | −0.21 sd | −0.30 sd | −0.39 sd | no; pooled −1.2 to −2.2 |
| the grid shifted by one step | −5.6 sd | −5.2 sd | −4.7 sd | yes (almost every seed) |

So the test is blind to a slope error of 5% in the system, and the golden vectors are what guard the schedule against it (GAP-3).

### 2.5 T-07: V-DATA-04 over 1,000 years

Two worlds of 3,000 founders aged 0 (360-day and 365-day calendars, 1,000 years each, 1 and 4 threads): 3,000 life-course rows each, each `PersonId` once, the same set as the genesis ledger; 3,000 distinct `DEATH` subjects; 3,000 rows with exit kind `death`; the ledger empty; `EXTINCTION` once (years 93.6 and 96.8); the engine ran to year 1,000 with the hashes taken. A death record exists if and only if the person is absent from the ledger.

### 2.6 T-08: `rh-cli run <world>`, ladders, the baseline gate

| Probe | Result |
|---|---|
| `solo`, 200 years yearly | byte-equal to `worlds/solo/baselines/solo_yearly_200y.ladder`; header carries `engine_build=rh-golden-0.0.0-built:M2.demography`, the registry version, the manifest SHA-256, the seed, the journal digest and `built M2.demography not_built M1.regions,M3.agriculture,M4.economy,S5.daily_life` |
| 21 monthly ladders of 1,000 years at 1, 2, 3, 4, 8, 16 and 64 threads × 3 | all byte-equal (13.5 MB each) |
| Builds | debug at 4 threads, release, and release with `-C target-cpu=native` (200 years yearly, and 1,000 years monthly) all equal; the native build also reproduces the `smoke` baseline |
| The yearly cut of the 200-year monthly ladder | byte-equal to the baseline |
| Resume from the keyframes of years 0, 10, 20, 30, 50 and 190 (journaled, at 4 threads) | every resumed rung equals the continuous run's |
| `--strict solo` | refused, exit 2, naming M1, M3, M4 and S5; `--strict smoke` runs |
| Other arguments | `--years 0`, `--years 100000`, `--seed 2^64 − 1` run; `--years -5`, `--ladder daily`, a world name with `/` or `..` are refused; `--leak-thread` is refused on `solo` |
| `bin/regen_baseline.sh` | regenerates both baselines with zero diff; an unknown world is refused |
| `check_baseline.py`, `has_c3_tag` | 300,000 random strings against my own implementation of "the trailing tag group holds `[C3]`": 0 differences |
| `check_baseline.py` on a scratch repository (push and pull-request forms): a baseline changed in a `[C3]` commit passes; unchanged passes; changed under `docs: the [C3] rule` fails; a pull request without `[C3]` fails; a mismatching recomputation fails even with `[C3]`; a CRLF recomputation fails; a missing baseline fails; an all-zero `before` is "not checked". One behaviour is by design but worth knowing: a pushed range holding one `[C3]` commit and a later unmarked baseline change passes (`any`) | as stated |
| SHA-256 (`rh-golden`, hand-written) against `hashlib` on 390 lengths: 0 to 300, 2^p − 2 to 2^p + 2 for p = 6 to 20, the 55/56/57/63/64/65/119 to 129 boundaries, 1,000,000 and 4,194,359 bytes | 0 mismatches |
| Where the golden `solo` run stands | Ada dies at tick 10,664,640 (year 20.6) with the committed input and at tick 27,796,320 (year 53.6) without it; the input (year 10.3) finds no pending timer, so it changes her draws only through the epoch (GAP-1) |

V-SCALE-03: 1,000 years of `solo` take 213 ms at one thread, 12,002 instants (17.7 µs each); the gate reports the time and asserts nothing (GAP-4). V-PERF-04: the Developer reports CI run 286 at 7 min 31 s (core) and 7 min 40 s (golden); I did not inspect the run. My serial `check_all` totals 914 s.

### 2.7 A-47 and A-49

A-47 (`bin/lint_forbidden.py` rule `spec-loader`; planted into a copy of `rh-sys-demography/src/lib.rs`): a direct path call, a grouped import with a rename, `registry_system_ids`, a raw identifier (`r#load_specs`), a function pointer, a macro body and a `#[cfg(test)]` module each FAIL; the name in a doc comment, a line comment and a string literal passes (no false positive). Renaming the package to `rh_sys_demography`, `m2-demography`, `rhsys-demography` or `Rh-sys-demography` switches the rule off in `lint_forbidden`, but `lint_arch` then fails on the metadata rule ("only crates named rh-sys-* are kind system"), so the two gates together hold. The rule is a list of two names (GAP-5).

A-49: GAP-1 (every workspace member on exactly one engine list): a new crate on neither list, and `rh-bench` on both lists, each fail `test_every_workspace_member_is_on_exactly_one_engine_list`. GAP-2 (the deny-list): deleting `hyper`, and deleting `tts`, each fail `test_the_whole_deny_list_equals_the_committed_golden_list`. GAP-3 (`process::` alternative): dropping it fails `process_glob_std`. GAP-9 (the exponent error reported once): removing the `continue` fails `an_exponent_past_the_bound_is_reported_once` (M01). GAP-4 and GAP-6 are documented in `tests/lint_fixtures/README.md`.

### 2.8 `bin/check_all.sh` whole

A clean clone of `9e5a6c2` with a real `origin/main` (`67a78c0`), `CARGO_TARGET_DIR` absolute and fresh: **358 PASS, 0 FAIL**, exit 0, report written. The gates the Developer listed are present: the schedule reference and its control, `schedule_golden`, genesis, V-INV-11, `m2_system` in debug and release (q(15) over seeds 1000 to 1031 with z = 3.290527, V-DATA-04), `bernoulli_only` with its planted Poisson control, the A-47 control, the GAP-3 control, solo at 1 and N threads compared, V-GOLD-01 with its two controls, the `--strict` control, the V-SCALE-03 report, and `rh-golden`'s `solo` tests. I ran the whole tier once, not `--part core` and `--part golden` separately.

## 3. Defects

None. No DEF-n was opened. The accepted deviations the Architect listed (M2's world-level tests in `rh-worldgen/tests`, the loud failure for a dead victim's timer, the hand-written SHA-256, solo's 200-year PR ladder) are not reported as defects; the SHA-256 was probed as a mutation target (§2.6, §5).

Observations, not defects: the argument-error text of `rh-cli run <world>` says "run smoke: …" for any world (`--years -5` on `solo`); `--threads 0` is accepted and printed as "0 thread(s)" with the same hash as one thread.

## 4. Contract gaps

| # | Gap | Recommendation | Confidence |
|---|---|---|---|
| GAP-1 | **`resample_timer`'s draw wiring is pinned by no test.** Mutants "the Bernoulli takes `ctx.draw(1)`" (D11) and "the day takes `ctx.draw(0)`" (D12) survive the debug and release nextest suites and `solo`'s ladder. The `a_material_input_redraws…` test checks properties (later day start, inside the month, outsiders untouched), not the draws; the `resample_day` golden vectors cover the day arithmetic, not the glue; and solo's committed input (tick 5,326,927) lands 10 years before Ada's death, with no timer pending, so the golden run never cancels and re-draws a timer (ruling 15 asks only for epoch-keyed draws in the run, which holds). My differential catches both mutants (154 and 178 mismatching worlds of 250) | Add a differential or exact-value test of `resample_timer` (my 401 cases are reusable), and move solo's input into a month where Ada has a pending death, so the golden ladder covers ADR-0025 item 3. Changing the input moves the `solo` baseline, a `[C3]` change **Disposition (Architect, 2026-10-17): accepted.** The Developer adds (1) a differential golden test of M2's draws (a stdlib generator on `gen_rng_golden.py` and the rung-0 reference computes every death instant in small fixture worlds whose inputs land on pending deaths, on the c = 0 day, at a timer's own instant, and as duplicates; a Rust test asserts the engine equals it; a planted k-swap control must fail) and (2) solo's input moved into Ada's natural death month, a new `[C3]` baseline. Phase 3 verifies the delta: D11 and D12 must die, and my oracle must agree with the new golden file | 80% |
| GAP-2 | **Thread-count invariance on `solo` cannot fail.** The run reports 2,401 scheduler levels, 0 with two or more jobs: one system, one binding. `golden_nightly.sh` says so ("no parallel floor"). M2's loop over persons is sequential, so no thread count reaches the history. V-DET-01/02 on `solo`, `household` and `hamlet` therefore show only repeat invariance. (The same finding was DEF-2 for `smoke` in `VER-SPR-008-TEST`; `smoke` fixed it with a floor, which cannot exist here) | Record in `docs/09` V-DET-01 that M2-only worlds carry no thread evidence and `smoke` carries it; the first parallel loop in a system crate (chunking over persons) must bring its own thread test | 85% |
| GAP-3 | **The q(15) check cannot see a slope error.** A schedule at β = 1.05 shifts the expected count by 0.5 to 1.0 per-seed sd (§2.4), so a per-seed 99.9% band passes it; only the 5% pooled mean over 32 seeds would fail (z −2.9 to −5.6). A one-grid-step shift is caught. The golden vectors guard the schedule itself, so the check guards the system's use of it | Add a pooled test on the 32 seeds (the absolute mean z below about 3.5, with the same fixed seeds) | 70% |
| GAP-4 | **V-SCALE-03 is reported, not asserted, and not at daily cadence.** `solo` runs 12,002 Month-pulse instants in 213 ms; no Day binding exists until S5. The criterion ("1,000 years at daily cadence in ≤ 10 s; fixed overhead ≤ 1 ms per tick") is met by a wide margin on the cadence that exists | Assert ≤ 10 s and ≤ 1 ms per instant in the gate, and say in `docs/09` that daily cadence waits for S5 | 75% |
| GAP-5 | **The A-47 rule is a list of two function names.** A new loader in `rh-registry` or a wrapper such as `Ownership::from_dir` is not caught; the `rh-sys-` prefix test depends on `lint_arch`'s metadata rule. The structural alternative A-47 named (move the loaders into a tool crate) removes the question | Move `load_specs` and `registry_system_ids` out of `rh-registry` when P1.1a next touches it, or forbid the `rh_registry::ownership` module path in system crates | 60% |
| GAP-6 | **A `fail:` control with an empty expected text accepts any failure.** `check_all.sh` with `"fail:K at e0 bits 1966080"` turned into `"fail:"`, or `"fail:BASELINE MISMATCH"` into `"fail:"`, passes `test_check_all_inventory.py` and `test_check_all.py` (CA7, CA8). The run would then accept a control that fails for the wrong reason | Make the inventory test refuse an empty `fail:` text | 55% |
| GAP-7 | **`solo` is a one-person world.** Its ladder cannot see an error that needs two persons (visit order, a skipped row, a shared stream); `household` and `hamlet` baselines arrive with SPR-013. Until then multi-person behaviour is guarded by the `m2_system` tests and, in this report, by my differential. A reversed visit order is invisible even to the multi-person tests, but it is also invisible to the history (the agenda orders same-instant timers by key; D16 below), so that one is harmless | Keep the `m2_system` tests in the by-name gates (they are) and land the two other baselines with SPR-013 | 70% |

## 5. Mutation matrix

Each mutant is one edit to the Developer's code, run against the repository's own tests (`cargo nextest`, the packages named), then the survivors in release and against the `solo` baseline. "Ladder" is the `solo` 200-year yearly ladder compared with the committed baseline.

| Mutant | Outcome |
|---|---|
| S01 `rhe` half up; S02 `rhe` floors; S03 ties to odd; S04 `rhe_signed` half up | killed (`rounding_ties_go_to_even`, golden, spec check values) |
| S05 bisection 39 steps; S07 `>=` becomes `>`; S09 survivor numerator 2^63; S10 trapezoid drops the last interval; S12 `x >= A_70` becomes `>` | killed (golden vectors, spec check values, Table 19) |
| S13 `interval_q` floors; S14 `l(x) = 0` gives probability 0; S15 `death_day` may land on the next pulse; S16 `death_day` may land on this pulse; S17 re-sample includes the current day; S19 re-sample count + 1 | killed (golden vectors) |
| S06 41 bisection steps; S08 `K_HI` = 2^39; S11 boundary `a < x`; S18 `first <= t_end` | survive; **equivalent**: after 40 steps `hi − lo = 1`, so a 41st step changes nothing; K for e₀ ≥ 20 is below 2^39; at a grid age the interpolation returns the same value from either side; `c = 0` either way |
| D01 `brass_alpha` not refused; D02 β refused only below 1 | killed |
| D05 Bernoulli at k = 1; D09 `EXTINCTION` on any dead row; D10 `EXTINCTION` without its cause; D13 exit kind not written; D14 alive flag not cleared | killed |
| D06 `exit_tick` not written | killed in release (the q(15) test) and by the ladder |
| D04 the day drawn at k = 2; D08 `DEATH` payload empty | survive the nextest suites in debug and release; **killed only by the `solo` ladder** (Ada's one death) |
| D11 the re-sample Bernoulli from `draw(1)`; D12 the re-sample day from `draw(0)` | **survive everything** (GAP-1); killed by my differential |
| D03 no Bernoulli draw when q = 0; D07 `person.death_cause` written 0; D15 `person.death_tick` not written; D16 persons visited in descending id | survive; **equivalent at rung 0**: q is never 0 (l is strictly decreasing); the dead row is compacted before any reader (M7 and T1 read it from P1.13); same-instant timers are ordered by key, not by insertion (checked on 392 same-instant groups under the mutant) |
| W01 birthday offset 0; W02 child range 0 to 19; W03 servant range 14 to 60; W04 sex threshold 2^62; W05 founder k from 1; W06 rows unsorted; W07 positive birth ticks; W08 owner check ignored; W09 `alive` column 0; W10 origin not founder | killed |
| W11 | an identity edit (my mistake); not counted |
| M01 the exponent error reported twice (A-49 GAP-9) | killed |
| SH1 padding to 55 mod 64; SH2 length + 8; SH3 length little-endian; SH4 Σ1 rotation; SH5 Maj; SH6 Ch; SH7 K[0]; SH8 K[63]; SH9 H0[7]; SH10 schedule loop to 63; SH11 σ0 shift; SH12 marker 0x00; SH13 σ1 shift | all 13 killed (CAVP vectors, and the manifest-pin tests) |
| CA1 drop the hand-edit control; CA3 drop the q(15) release gate; CA4 drop the solo ladder comparison; CA9 a trailing `or true` on a solo gate; CA12 drop a binary from the by-name list | killed (`test_check_all_inventory.py`) |
| CA7, CA8 a `fail:` control with empty text | survive (GAP-6) |
| CA2, CA6, CA10 a control's `fail:` turned into `pass:`; CA11 a test filter that matches nothing | not caught by pytest, but each would turn the run red when executed (the command's exit status or the expected output would mismatch); I did not run `check_all` per mutant |
| A-47 rule: 7 code forms, 3 non-code forms, 4 package renames | §2.7 |
| A-49: new crate on neither list; on both lists; `hyper` removed; `tts` removed; `process::` alternative dropped | each killed |

Totals: 59 code mutants (19 schedule, 16 system, 10 worldgen, 1 manifest, 13 SHA-256; W11 not counted): 47 killed by the repository's tests, 2 killed only by the `solo` ladder (D04, D08), 8 equivalent or unobservable at rung 0 (S06, S08, S11, S18, D03, D07, D15, D16), 2 surviving every gate (D11, D12). 11 `check_all` edits applied (CA5's pattern did not match and was not applied), 5 lint mutants, 14 A-47 variants.

## 6. What was not run

- **CI run 286** and the Developer's CI timings: not inspected. My serial `check_all` took 914 s on this host.
- `bin/check_all.sh --part core` and `--part golden` as separate runs: the whole tier ran once.
- The cross-machine check (`bin/cross_machine.sh`, needs the director's laptop and `adjutant`).
- The nightly tiers: `bin/golden_nightly.sh` end to end (I ran its `solo` section by hand: 21 monthly ladders and the yearly cut), the 10^6-draw `rh-ids` stream sweep, the sampler statistics at 64×, the 94,000-load keyframe sweep, V-DET-11 × 64.
- A 365-day calendar for the 10,000-founder q(15) cohorts (the cohorts use the 360-day calendar; the 365-day calendar is covered by the exact-instant differential and the 1,000-year audit).
- `household` and `hamlet` runs through `rh-cli` (no `worlds/` directories until SPR-013); their manifests compile and generate (§2.2).
- The full `check_all` for each `check_all` mutant.
- Valgrind and the instruction-count gates run only inside the whole `check_all` pass.
