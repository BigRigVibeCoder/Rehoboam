---
id: VER-SPR-016-TEST
title: "Determinism and contracts tester evidence for SPR-016"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing, determinism]
related: [SPR-016, SPR-010, SPR-012, REV-SPR-016, REV-SPR-016-FOLD, VER-SPR-012-TEST, VER-SPR-013-TEST, AGT-003-DET, AGT-003-IF, ADR-0006, ADR-0024]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: Stage 1 (T-01, T-02) meets its acceptance criteria. I found no defect and five test-strength gaps (two of them informational), none of which blocks the merge.** `bin/check_all.sh` is **548 of 548** on a clean clone of `a00c64b`. My own Python oracles, written from the contracts before I read any Rust, agree with the Rust on every input I tried: 2,097,152 `--emit` vectors and 299,998 of my own for `exp`, `ln_ratio` and `prob_from_hazard` (debug and release, 0 mismatches), and 288 schedule points (8 check values, 4 corners, 64 box points, 64 β = 1 points, both calendars; 202,752 `l(x)` samples; all 438 knots of each). Of 28 mutants I planted in the Rust, 21 died (3 of them only under my vectors or the sweeps, which exposes two gaps in the committed golden); of the 7 that lived, 6 are equivalent or unkillable by any input and 1 is a guard that no input reaches. This is stage 1 only. T-03 to T-05 (MR10–MR12, the fold, the 0.3.1 bump, the baselines' move) are not covered here.

Tested head: `a00c64b8e2e28e7de4225e6ff390879af7b992e8` on `sprint/SPR-016-mortality-f1` (draft PR #88), the head the Architect named. It is `a7b99e4` (T-02) and `8298bbc` (T-01) plus a merge of `main` at `3e2e6ef`.

Method: every probe ran on a clean clone or a copy of the SHA with its own `CARGO_TARGET_DIR`; the Developer's tree was never touched. I wrote the oracles from `05.1` §6 item 4 and the M2 spec's `[algorithm.mortality_f1]` (with `brass_general_standard.toml`, `infant_sub_schedule.toml`) before reading the Rust. They import nothing from `specs/numerics/exp_ln_v0.py` or `selftest_m2_f1_mortality.py`. The scratch crate that ran my vectors against `rh-core` and `rh-sys-demography` depends on them by path and sits outside the repository. No runner was available, so nothing here is a CI result; the Architect's CI run `39084610402` is a separate record.

## 1. Verdicts

| Task | Verdict | Basis |
|---|---|---|
| T-01 `Q32_32`, `exp`, `ln_ratio`, `prob_from_hazard` | **PASS with GAP-1** | Golden, 2^14 and 2^20 sweeps, my 3 × 10^5 vectors and tie set all bit-equal, debug and release; four compile-fail crates each fail for their own reason (§2.1) |
| T-02 the F1 schedule, MR1–MR9 | **PASS with GAP-3, GAP-4, GAP-5** | 288 points against my MR1–MR9; β = 1 equals rung 0 on both calendars; INV-M2-21; corners; refusals by name (§2.2) |
| Developer's deviation 1 (`2 · max(1, n/d)` ulp) | **Confirmed** | Proved and measured independently (§2.3) |
| `bin/check_all.sh` whole | **PASS** | 548 of 548 on a clean clone of `a00c64b` (§2.4) |
| Baselines (`smoke`, `solo`, `household`, `hamlet`) | **PASS, unchanged** | No file under `worlds/` or `artifacts/` differs from `main`; all four ladders equal their baselines (§2.4) |
| V-PERF-04, V-PERF-11 | **PASS, unchanged** | No count moved; no baseline changed (§2.4) |
| Defects | **None** | |

## 2. Results

### 2.1 T-01: `exp`, `ln_ratio`, `prob_from_hazard`, `Q32_32` (AGT-003-DET)

| Probe | Oracle | Result |
|---|---|---|
| **My E1–E6 and L1–L6** (`oracle_explog.py`, Python integers, constants from `decimal` at 80 digits) against the committed golden `specs/numerics/golden/exp_ln_v0.txt` | the `05.1` text | **9,374 of 9,374 lines equal**, the 34 constants included. So the golden is confirmed against the contract by a second implementation, not only against its own generator |
| My 3 × 10^5 vectors (`exp`, `ln`, `hazard`; 10^5 each): both domain edges, `±1` around `16`, `-23`, `i64::MIN/MAX`, every binade edge `n = d·2^e ± 1` for `e` in `[-62, 62]`, neighbours of `k·ln 2` and of each `C_k`, a Q16_16-granular grid, random full-width pairs. Run through a scratch crate on `rh-core` | my oracle | **299,998 of 299,998 equal in debug and in release** (2 lines have an operand ≥ 2^64, which the `u64` API cannot express) |
| **128 constructed E6 ties** (`vectors_ties.txt`): inputs whose 64-fraction-bit result is exactly half-way, 64 with an even quotient and 64 with an odd one, found by search in `y` ∈ [12, 16] | my oracle | 128 of 128 equal, debug and release |
| **The 2^20 sweep, on the head:** `exp_ln_v0.py --emit 1048576` (seed 1592649754): 2,097,152 vectors. (a) the Developer's ignored test `emitted_vectors_are_bit_equal`, debug and `--release`; (b) my scratch crate, debug and release; (c) the file itself re-checked by my oracle | my oracle, and the reference | **0 mismatches in all five runs.** The reference's emitted values equal my independent ones (a check the Developer's test cannot make) |
| 2^14 PR-tier path (`check_all`) | the reference | PASS, in the 548 |
| Properties (Developer's `exp_ln_golden.rs`: monotone, `exp(0) = 2^32`, `ln_ratio(n, n) = 0`, hazard 0 at 0 and 1 from 23, refusals by name) | | 7 of 7 pass in debug and in release |
| **Compile-fail crates** (`fixed-ok`, `mix-q32-q16`, `q32-float-cast`, `q32-float-from`, `q32-into-prob`), built alone | rustc's message | `fixed-ok` compiles. The other four each fail with one error and a different reason: `E0308` mismatched types (`Q32_32` against `Q16_16`), `E0605` non-primitive cast `Q32_32` as `f64`, `E0308` (`Q32_32` against `f64`), `E0277` the trait bound `Prob32: From<Q32_32>` is not satisfied |
| **Each control isolates its check** (REV-SPR-016 R7): I added the missing impl to `Q32_32` and re-checked the fixture | | `impl From<Q32_32> for Prob32` makes `q32-into-prob` compile; `impl PartialEq<Q16_16> for Q32_32` makes `mix-q32-q16` compile; `impl From<f64> for Q32_32` makes `q32-float-from` compile. Deleting each check flips its control, so each gate would fail. `q32-float-cast` cannot be defeated by an impl (a cast of a struct is a language error) |
| Code read against the contract | | `translog.rs` follows E1–E6 and L1–L6 line for line: the arithmetic shift in E2 and E5 is the floor the text states; E6 and L6 round through `shr_round_half_even`; the L2 comparison `n >> e < d` is equivalent to `n < d · 2^e` because `d` is an integer; `prob_from_hazard` cannot overflow (`-h` is in `(-2^63, 0]`). The 32 `C_k`, `LN2_64` and `INV_LN2_62` equal mine |
| Lints, perf | | `lint_forbidden` and clippy pass in `check_all`; nothing runs on the per-person path, and V-PERF-04 and V-PERF-11 did not move (§2.4) |

### 2.2 T-02: the F1 schedule at construction (AGT-003-DET)

| Probe | Oracle | Result |
|---|---|---|
| **My MR1–MR9** (`oracle_mr.py`) at the eight check values | the spec's `check_values` | 8 of 8: `K`, `tail_a`, `tail_b` and the zero knot all equal |
| My MR1–MR9 against the golden's 4,380 `knot` lines (the 10 design and corner points) | the reference's golden | 4,380 of 4,380 |
| **Rust against my oracle**, 144 points × 2 calendars = 288 compiles: the 8 check values, the 4 corners and 4 more edge points, 64 pseudo-random `(e0, β)` in the Registry box, 64 pseudo-random `e0` at β = 1. Each compared on `K`, `tail_a`, `tail_b`, `mu`, the zero knot, all 438 knots, and `l(x)` at 704 minutes: 0 and 1 minute either side of 1 Y, 90 Y and 120 Y, around the zero knot, 300 uniform, 100 under 1 Y, 100 over 90 Y, and every seventh knot age ±1 | my oracle | **288 of 288 equal; 202,752 `l(x)` samples equal; 0 mismatches, debug and release.** No point raised an MR8 precondition |
| The 69 compiled-in `LAMBDA` constants | my `ln_ratio(10^4 − s_i, s_i)` | 69 of 69 equal |
| **β = 1 is rung 0** (the plan's wording: 7 + 64 e0, `K`, `L_0..L_70`, `l(x)` at 10^4 minutes in [Y, 90Y], both calendars) | (a) my Python rung-0 S1–S7; (b) the Rust rung-0 `Schedule` | (a) `K`, all 71 `L_i` and `l(x)` at 128,000 minutes equal at 71 e0 on both calendars; (b) 136 β = 1 compiles in the Rust: `K`, grid and `l(x)` at every sampled minute in [Y, 90Y] equal rung 0's |
| INV-M2-21 (my oracle, 71 e0 × 2 calendars; the Developer's test, 110 compiled points) | | 438 knots strictly increasing in age and non-increasing in `l`, `l(90) = L_66`, `l(120 Y) = 0`: holds at every point |
| `mutant-beta-one-power` (the plan's mutant) | | Fails three tests: `rung_0_is_the_beta_one_case` ("rung 0 differs at beta = 1: K at e0 bits 1310720"), `every_lambda_schedule_knot_and_ell_line_is_reproduced` ("K of schedule 30 1: the reference gives 9582773732, rh-sys-demography 9582773733") and the check-value test. Fails by name, for the reason the plan gives |
| Corners (the Registry box compiles with no error; zero knots `l_zero_twelfths` 1314, 1361, 1381, 1408, 1440) | the spec | All hold in my 288 and in the Developer's test (`inv_m2_21_holds_at_every_compiled_point_and_the_corners_compile`) |
| Refusals by name: `brass_alpha`; MR2 (β out of bounds); MR3 (`exp` refusal); MR8 ×6 through the `Tail::from_anchor` hook | | `each_refusal_names_its_step` passes: the error is `DemographyError::Construction { step, .. }` and the text names the step |
| **Reported (T-02):** the F1 schedule's own trapezoid e0 against E at the eight points | the spec's check 8 | −0.0516, −0.0598, −0.0386, −0.0689, −0.0031, **+0.5241**, −0.0493, −0.0216 years at (30,1), (30,0.8), (30,1.3), (23.44,1), (60,1.65), (85,0.6), (20,1.65), (85,1.65). The spec says −0.06 at e0 30 and +0.5 at e0 85: consistent |
| **Reported (T-02):** the time of one compile | | The Developer's tests do not print it (GAP-5). I measured 0.23–0.30 ms per compile in a release build and 0.75–0.80 ms in a debug build, at three points of the box, on a loaded 4-core container; one compile runs once at construction |

### 2.3 The Developer's deviation 1, checked independently

The acceptance wording (`exp(ln_ratio(n, d))` within 2 ulp of `n · 2^32 / d`) cannot hold for odds above about 4: `ln`'s absolute error of up to ½ ulp (2^-33) becomes a relative error of 2^-33 in `exp`, which is `(n/d)/2` ulp at the result. The Developer's test uses `2 · max(1, n/d)` ulp.

- **Analytic bound.** The error is at most `(n/d)/2` (from `ln`'s ½ ulp and 2^-25 ulp excess) plus `¾` (`exp`'s excess) plus `½` (the target `n · 2^32 / d` is not an integer), that is `(n/d)/2 + 1.25` ulp. For `n/d ≥ 1` that is at most `1.75 · (n/d)`; for `n/d < 1` at most `1.75`. Both are under `2 · max(1, n/d)`.
- **Measured** (exact rationals, my oracle). On the 69 standard odds the worst `error / max(1, n/d)` is 0.625 (4465/5535). The Developer's test reports 4,603 ulp at the largest odds (9999/1), against `(n/d)/2 ≈ 5,000`. On 200,000 random pairs with `n, d < 2^30` the worst ratio is 0.985, and the worst `error − (n/d)/2` is 0.4995 ulp, under the analytic 1.25.

**The bound is correct and the deviation is sound.** I accept it.

### 2.4 Whole `bin/check_all.sh`, baselines, performance

- **`bin/check_all.sh` with no flag, on a clean clone of `a00c64b`: 548 of 548 PASS** (`artifacts/check_report.md`: "Verdict: PASS (548 checks)"). It needs `origin/main` to exist as a ref for the baseline-integrity gates; a local clone has none, so I set it to the real `origin/main` (`3e2e6ef`, the merge base). A first run without it stopped at the `[perf-baseline]` gate (a property of the clone, not of the head); the second run is the one reported.
- **Baselines.** `git diff 39e213b a00c64b -- worlds artifacts` is empty. The `smoke` 1,000-year ladder (1 and N threads, parallel levels at least 5%), and the `solo`, `household` and `hamlet` 200-year ladders (1 and N threads; `--expect-serial` on the last two) all equal their committed baselines; the hand-edit and not-[C3] controls fail as they must. Stage 1 is hash-neutral, as the plan predicts and as T-02's header says (nothing calls the F1 schedule yet).
- **V-PERF-04 and V-PERF-11.** `benches 7 and 8 within 10%` and `repeatability` PASS; the allocation budget at 1 and 4 threads, its repeatability and its integrity controls PASS; no baseline file is in the diff.
- **New gates, all PASS in part core:** `exp_ln_golden` (debug); its altered-vector control; the 2^14 emit comparison; the four compile-fail crates and `fixed-ok`; `schedule_f1_golden` (debug) and both its controls (one knot one bit off, and `mutant-beta-one-power`); `exp_ln_v0.py --check` and its stale control; `selftest_m2_f1_mortality.py --check` with its four controls.

## 3. Mutation matrix, the Rust

I planted each mutant in a copy of the head and ran (D) the Developer's golden test for that file (`exp_ln_golden`, no emit file, or `schedule_f1_golden`) and (M) my vectors through the scratch crate. "Dies" means the run fails.

| Mutant | D | M | Verdict |
|---|---|---|---|
| E2 floor → division (truncates toward zero) | dies | dies | killed |
| E4 `r >= C_k` → `r > C_k` | lives | lives | equivalent: `r == C_k` has probability about 2^-60 |
| E4 `p >> k` → rounded shift | lives | dies | killed only beyond the golden: by the 2^14 sweep (2 of 32,768 vectors) and the 2^20 sweep (128 of 2,097,152); see GAP-1 |
| E5 omitted | dies | dies | killed |
| E5 floor → truncation toward zero | lives | lives | equivalent in practice: `r < 0` only within 2^-59, and the difference is 2^-64 against a midpoint |
| E6 round → floor | dies | dies | killed |
| **E6 round-half-even → half-up** | **lives** | dies (ties) | **killed only by my tie set and the 2^20 sweep (9 of 2,097,152); the golden and the 2^14 sweep do not catch it: GAP-1** |
| E1 refusal at `y ≥ 16` | dies | dies | killed |
| E1 zero below −24 instead of −23 | lives | lives | equivalent: the E2–E6 steps give 0 there (`05.1` says so) |
| L2 correction removed | dies | dies | killed |
| L4 `t <= m` → `t < m` | lives | lives | equivalent: `t == m` has probability about 2^-64 |
| L5 residual removed | dies | dies | killed |
| L6 round → floor | dies | dies | killed |
| L6 round-half-even → half-up | lives | lives | unkillable by vectors (a tie needs the low 32 bits of the 64-bit result to equal 2^31, about 2^-32 per input); L6 calls the same `shr_round_half_even` as E6, which E6's tie vectors do exercise: accepted on the code read |
| `prob_from_hazard` survival minus 1 | dies | dies | killed |
| `prob_from_hazard` saturation off (survival 0 → 1) | dies | dies | killed |
| MR3 product `rhe` → floor | dies | dies | killed |
| MR8 forced non-increasing (`min`) removed | lives | lives | **not exercised: GAP-3** |
| MR8 `m1`, `m2` rhe → floor | dies | dies | killed |
| MR8 `mu` rhe → floor | dies | dies | killed |
| MR8 cap not zero (`j` up to 360) | dies | dies | killed |
| MR7 infant `rhe` → floor | dies | dies | killed |
| MR9 interpolation `rhe_signed` → floor | dies | dies | killed |
| **MR9 cap one minute early** (`x >= 120 Y − 1`) | **lives** | dies | **the golden has no sample next to the cap: GAP-4** |
| MR9 interval search `<=` → `<` | lives | lives | equivalent: `l` is continuous at a knot, so both intervals give the same value |
| MR4 power form `rhe` → floor | dies | dies | killed |
| MR2 β upper bound 108,134 → 108,135 | dies | lives | killed by the Developer's refusal test (my oracle does not probe the bounds) |
| `mutant-beta-one-power` (the plan's) | dies | dies | killed, by name (§2.2) |

**Tally.** 28 mutants, the plan's `mutant-beta-one-power` and 27 of mine. **21 die** under at least one of the two runs. Three of those 21 die only under my vectors or the sweeps, not under the Developer's committed golden test: E4 with a rounded shift, E6 half-up and the MR9 cap one minute early; they are GAP-1 and GAP-4. **7 survive both runs:** six are equivalent or unkillable by any input (E4 `>`, E5 truncation, E1 at −24, L4 `<`, L6 half-up, MR9 `<`; GAP-2) and one is a guard no input has reached (MR8's forced non-increasing step; GAP-3).

## 4. Defects and gaps

No defect. Five gaps, in test strength and reporting, none a contract violation.

### GAP-1 (low): the committed golden does not exercise round-half-even; the 2^14 PR sweep barely does
Two mutants of `translog.rs` survive `exp_ln_golden` without an emit file: E6 rounding half-up and E4 with a rounded shift. The 2^14 emit sweep kills the second (2 vectors of 32,768) and not the first (0 vectors of 16,384 `exp` inputs separate half-even from half-up; 9 do in 2^20). So the contract's central rounding rule (E6 and L6: "half to even") is tested at PR tier only by 2 lucky vectors, and E6's tie case only nightly. **Recommendation:** add tie vectors to the reference (`exp_ln_v0.py`): my search finds ties in a few seconds (`y` ∈ [12, 16], `32 − n` ≤ 20, 64 with an even quotient and 64 with an odd one; `vectors_ties.txt`), so the golden itself would kill E6 half-up. This is a tool change inside SPR-016, as D-6 allows.

### GAP-2 (info): six mutants cannot be killed by any input
E4 `>`, E5 truncation, E1 at −24, L4 `<`, L6 half-up and MR9 `<` (§3) are equivalent or unreachable. I checked each by reading the code and its probability argument. This is a statement about the procedure, not the tests. No action; recorded so that "21 of 28" is not read as a defect count.

### GAP-3 (low): MR8's forced monotonicity has never been exercised
`T_j = min(T_(j−1), …)` ("faithful rounding is not monotone below 2^-27") never fires: on the 2,882-point grid (e0 20 to 85 in 0.5 steps, β 0.6 to 1.65 in 0.05) the unforced sequence is already non-increasing, and so it is on 4,000 random synthetic `(a, b, L_66)` tails. Removing the `min` changes no golden line and no sample of mine. Either the guard is unreachable (then the spec's sentence is a belt-and-braces rule and could say so), or a reachable case exists that none of us found. **Recommendation:** the Architect decides whether the guard is stated as unreachable in the spec, or whether the reference gets a synthetic `Tail::knots` case that forces it.

### GAP-4 (low): the golden's `ell` samples stop short of the 120 Y cap
`ell` samples include ages under 1 and over 90 but none within a minute of 120 Y. A cap placed one minute early (`x >= 120 Y − 1`) passes `schedule_f1_golden`; my `120 Y − 1` sample kills it. **Recommendation:** add `120 Y − 1`, `120 Y`, the zero knot ±1 and the knot ages ±1 to `ell_ages` in `selftest_m2_f1_mortality.py`.

### GAP-5 (info): "reported" items are not printed
The plan asks T-02 to report the time of one compile. The Developer's tests print the realised e0 figures (and they match the spec) but not the time. I measured it (§2.2). No action beyond printing it from the test if the Architect wants it on record.

## 5. Not covered

- **Stage 2:** MR10–MR12 (the monthly `q`, the newborn rule, the re-sample, the density modifier), `ResampleCtx::living_count()`, the `rh-manifest` refusals, the fold of the draws golden, the 0.3.1 bump, the moved baselines, the divergence record, the CI time. The five mutants named in T-03 and the model of S8–S12 + MR10–MR12 are mine to build then.
- **The monthly ladders and `golden_nightly.sh` as a whole** were not run. I ran the two nightly pieces that concern stage 1 by hand: the 2^20 `--emit` sweep (debug and release) and the release run of `exp_ln_golden`; I did not run the release run of `schedule_f1_golden` through `golden_nightly.sh` (it passes under `check_all` in debug, and my harness passes in release).
- **Thread invariance and keyframes:** nothing in stage 1 is on the per-person path, so there is nothing for them to vary; they matter from T-03.
- **The other STAT and IF items** (the q(15) null, V-SF-02b, `compile`'s refusals, registration against 0.3.1) belong to T-03 and T-04.
- **GitHub Actions:** no runner; `check_all` was run locally.
