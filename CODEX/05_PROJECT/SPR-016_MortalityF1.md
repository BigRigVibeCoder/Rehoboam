---
id: SPR-016
title: "M2 F1 mortality: fixed-point exp and ln, Brass at any (alpha, beta), the infant sub-schedule, the old-age tail and the density modifier; the 0.3.1 bump (P1.3, part b)"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-002, PRJ-002, REV-SPR-016, REV-SPR-016-FOLD, REV-SPR-015, SPR-010, SPR-011, SPR-012, SPR-014, SPR-015, SPR-028, SPR-030, ADR-0006, ADR-0024, ADR-0025, ADR-0033]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF:** Builds `05.1` §6 item 4's fixed-point `exp`, `ln` and `prob_from_hazard` on a new `Q32_32` type in `rh_core::fixed`, bit-equal to `specs/numerics/exp_ln_v0.py`'s golden file, and the M2 spec's `[algorithm.mortality_f1]` MR0–MR13 in `rh-sys-demography`: Brass at any (α, β) on a 438-knot schedule, the infant sub-schedule below one year, the logistic old-age tail to 120, the density modifier (inert in every P1a world), the newborn's first draw and the deferral-consistent re-sample. At β = 1, under capacity, ages 1 to 90, the F1 schedule reproduces rung 0 bit for bit (the reference's check 1). `rh-manifest` accepts any β in the Registry bounds and the two new parameters. The frozen M2 moves from 0.3.0 to **0.3.1** (the interim rule out).
>
> **Two baselines move** (`household`, `hamlet`): newborns' and infants' first months and anyone past 90 − D draw against a different q. **`solo` is predicted unchanged** (Ada is 25 at genesis and dies at 57) and `smoke` has no M2; either moving is a defect. Re-recorded in the final `[C3]` commit by `bin/regen_baseline.sh`, announced to the director before the merge.
>
> - **Developer:** AGT-002-CORE, with AGT-002-TOOLS for the generators and the Python-side parity (one session may hold both).
> - **Testers:** AGT-003-DET (bit-equality, the independent model, the divergence) and AGT-003-STAT (q(15), V-SF-02b). One session may hold both, on a different model from the Developer's.
> - **Size and class:** about 3 agent-days. **C3**: a model change that moves two goldens, and new determinism machinery (a fixed-point transcendental procedure, `05.1`: "a C3 change with a new format version"). It contains C2: the 0.3.1 bump, `rh-manifest`'s `count` parameters, an additive `ResampleCtx` method.
> - **Status:** PLANNING. It needs the Architect's rulings in the specs table, the director's ruling on A-48 (see "Director items") and the plan approval (D54; GOV-005 §2). **Merges after SPR-015** (PRJ-002); T-01 and T-02 may start at once.

# SPR-016 — M2 F1 mortality (WP P1.3, part b)

- **Roadmap item:** `docs/11` P1.3 (b) · **Phase / rung:** P1a, rungs 0–2
- **Depends on:**
  - **SPR-012 merged** (code): `rh-sys-demography`'s rung-0 schedule and draws, `bin/gen_m2_reference.py`, `bin/gen_m2_draws_reference.py`, the altered-golden controls.
  - **SPR-015 merged before T-03's newborn cases, T-04 and T-05:** the births that make newborns exist, the promoted 0.3.0 this sprint bumps, the Registry carrying `density_capacity` and `density_mortality_slope`, and SPR-015's differential model, which T-03 extends. T-01 and T-02 depend on SPR-010 and SPR-012 only (PRJ-002 §4 wave 2: "machinery with no consumer yet").
  - Independent of SPR-014's and SPR-020's content beyond `main` having moved (T-05 re-measures from whatever baselines are on `main`).
- **Binding contracts:**
  - `05.1` §6 item 4 (v1.10/v1.11: `Q32_32`, the functions, domains, steps E1–E6 and L1–L6, the rounding and the error bound, format `rehoboam/fixed/exp-ln/v0`); `specs/numerics/exp_ln_v0.py` and `specs/numerics/golden/exp_ln_v0.txt` (9,377 lines).
  - `[algorithm.mortality_f1]` MR0–MR13 of the M2 spec (0.3.0 on `main` after SPR-015), its eight `check_values` and the facts below them; `[algorithm.parameters_f1]` P1–P2 (the interim rule this sprint removes); INV-M2-20, INV-M2-21; `specs/demography/brass_general_standard.toml`, `infant_sub_schedule.toml`; `specs/demography/selftest_m2_f1_mortality.py` and `golden/m2_f1_mortality.txt`.
  - `05.1` §2 rule 2 (the trait `resample_timer(&self, &TimerEntry, &ResampleCtx) -> Result<Option<SimTime>, SimError>`), §6 items 2, 4, 9; `05.5` §3.2 (authored numbers; the `count` type); `05.2` §1.
  - ADR-0006, ADR-0025 item 3, ADR-0033; `docs/09` V-DET-09's method (golden vectors at both optimisation levels), V-DET-12, V-DET-14, V-DET-19, V-INV-08, V-SF-02, V-SF-02b, V-WC-04, V-GOLD-01.
- **System spec:** M2 goes from frozen 0.3.0 to **frozen 0.3.1** in this PR (T-04; an Architect-approved C2 change inside the C3 PR). Nothing algorithmic changes: the interim sentences leave P1, `not_yet` item 2 leaves, INV-M2-20 and 21 lose their "apply from SPR-016" clause.
- **Rulings (binding):**
  - REV-SPR-016 R1–R14, above all R1 (the frozen `draws` text untouched), R2 (certain death from the zero knot), R3 and R4 (the newborn's first draw and its re-sample formula), R5 (the error budgets, the sweep's counts), R6 and R13 (the mid-interval anchor kept; the sawtooth stays), R7 (one control, one check), R9 (N at the Inputs phase start), R11 (A8 the density modifier; A10 `brass_alpha` refused), R12 (no named `Q16_16` forms until a consumer needs them);
  - REV-SPR-016-FOLD R1 (P1 names six parameters), R2 (the M4 read deferred), R4 (`type = "count"`), R6 (the promotion checklist), and R3 as changed by SPR-015's specs table (0.3.0 promoted there; this sprint bumps to 0.3.1);
  - the design values DV-SPR016-1 to 4 (the Architect's, companion §8.3);
  - the Architect's rulings at this plan (the specs table).
- **Director items:** **BCK-002 A-48** (the smooth interpolant: a model change; see "Director items"). D40 and D41 are approved and untouched.
- **Requirements covered:** DET-3 (fixed point: `Q32_32` and the transcendentals, the one procedure); DET-2 (keyed RNG: no new purpose); SIM-5 (hazards and binomial draws at any (α, β)); DATA-8 (death records unchanged in form); DET-13 (the re-sample never back-dates).

## Specs this sprint needs

| Spec | Exists? | Gap | Proposed Architect ruling | Needed by |
|---|---|---|---|---|
| **BCK-002 A-48**, the C¹ logit interpolant into monthly knots from age 1 | "Decided at SPR-016's planning" (REV-SPR-016 R13) | A model change with no exact reference, no spec text and no golden; it trades the β = 1 bit-equality with rung 0, on which checks 1 and 7 and the whole verification strategy of this sprint rest | **Not built here.** A director item (a model change): the recommendation is to decide it after SPR-028's harness shows the age-at-death distribution with and without the teeth, as a P1b C3 sprint with its own spec round (reference, golden, a new `[algorithm.*]` table version). 7/10 | — |
| **Where the `exp`/`ln` Rust vectors come from** | `05.1`: "SPR-016 extends `bin/gen_fixed_vectors.py` with it" | A second copy of the vectors can drift from the reference's golden file | **The Rust golden test reads `specs/numerics/golden/exp_ln_v0.txt` directly** (the committed output of the one reference), as `rh-sys-demography`'s tests read `m2_schedule_v1.txt`; `gen_fixed_vectors.py` is not extended, and `05.1`'s sentence is corrected to say so. 7/10. Alternative: follow `05.1` literally and have the generator copy the vectors into `crates/rh-core/tests/golden/` with a `--check` | T-01 |
| **The Rust nightly sweep** | `--exhaustive` is Python-only (`bin/golden_nightly.sh`); Rust has no exhaustive tier | The Rust port is proven only on the 9,377 committed vectors | **`exp_ln_v0.py --emit N --seed S` writes N pseudo-random `Q32_32` vectors of both functions (not committed); an ignored Rust test reads them and checks bit-equality; `golden_nightly.sh` runs 2^20 of each nightly** (7/10). The PR tier runs 2^14 through the same path |
| **`type = "count"` parameters in the validators** | REV-SPR-016-FOLD R4: `lint_odd.py --rust` emits `u64`; MR2: a non-integer manifest literal is a load error naming the parameter | `rh_manifest::compile` and `manifest_v0.py` type every parameter as a decimal with bounds | **Both validators learn the `count` type from the spec table: an integer literal within integer bounds, a non-integer literal refused by name; a parity fixture both ways (V-WC-04)** (8/10). The Registry's `density_capacity = [1, 10000000]` already has integer bounds | T-03 |
| **MR12's N in a re-sample** | MR12: "N is the living count at the start of the Inputs phase of `t_in`, before the input's own effects, counted once per input" | **Gap.** `resample_timer` sees no world (`ResampleCtx { key, purpose, entity, t_input, epoch }`), so M2 cannot count N there, and the spec says nothing is stored | **`ResampleCtx::living_count()`: `rh-core` counts the ledger's `alive = 1` rows once at the start of the Inputs phase (the compaction pass's own core-only read) and hands it to every re-sample of that instant** (7/10). Additive, hash-neutral (every P1a world is under capacity, so q'' = q_r whatever N is); a planted over-capacity fixture world exercises it. Alternative: M2 stores the pulse's m_d in a world scalar, which is a new owned column and an amendment of "nothing is stored" | T-03 |
| **Construction errors by name** | MR0: "a refusal during construction is a load error naming the step"; MR8: "a construction error naming MR8" | Which error type and where | **`Demography::new` returns `SimError::Construction { system, step, reason }`, surfaced by `rh-cli run` as a load error; a test reaches each MR8 precondition through a test-only schedule hook outside the Registry box** (8/10) | T-02 |
| **The baseline prediction** | MR13's header: "a founder under one year old, a child born since SPR-015 (its first months) or a person aged over 90 − D at a pulse draws against a different q; nothing else" | Which worlds move | **`household` and `hamlet` move; `solo` is unchanged; `smoke` is unchanged. The bisect's first divergent month names a person in one of the three classes; a divergence naming anyone else is a defect** (7/10) | T-05 |
| **V-SF-02b's report tier** | `docs/09` V-SF-02b: l(65)/l(15) inside the range (α, β) implies ± 0.03; the spec's facts give 0.481, 0.315, 0.139 at e0 30 for β 0.64, 1, 1.64 | Not run anywhere today | **A release test on 10,000 founders aged 15 at β ∈ {0.64, 1, 1.64}, 32 seeds, pooled, reported; nightly** (7/10). V-SF-02's q(15) band test (SPR-012) stays in the PR tier and must be unchanged at β = 1 | T-03 |
| **A world above capacity** | DV-SPR016-1: inert in every P1a world | Nothing on `main` exercises m_d > 1 | **A test manifest (not a golden world) with `density_capacity: 10` and 30 founders; the differential model carries its m_d trajectory** (8/10) | T-03 |
| **The 0.3.1 content** | REV-SPR-016-FOLD R3's interim rule is frozen text after SPR-015 | What the bump may change | **P1 loses its interim sentences and its table version becomes `m2-f1-parameters-v2`; `not_yet` item 2 leaves; INV-M2-20 and 21 lose the "apply from SPR-016" clause and the comment above them; the header's lifecycle paragraph is updated. Nothing in MR0–MR13, S0–S12, F, N, H or L changes. No `registry_version` bump: the Registry does not change** (8/10) | T-04 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C3**: a model change (the F1 schedule) that moves `state_hash` on `household` and `hamlet`, and new determinism machinery in `rh_core::fixed` (format `rehoboam/fixed/exp-ln/v0`). It contains C2: M2 0.3.1, `rh-manifest`'s `count` type, `ResampleCtx::living_count()` |
| Planes touched (ADR-0024) | **A:** `rh-core` (`fixed`: `Q32_32`, `exp`, `ln_ratio`, `prob_from_hazard`; `ResampleCtx`), `rh-sys-demography` (`schedule.rs`, the draws). **Tool:** `rh-manifest`, `bin/gen_m2_reference.py`, `bin/gen_m2_f1_draws_reference.py`, `specs/numerics/exp_ln_v0.py` (`--emit`), `specs/manifest/manifest_v0.py`, `bin/golden_nightly.sh`, `nightly.yml`. **B, C:** none |
| Owned columns written | None new. M2's `person.*` and `life_course.*` exit fields as before, through `apply_death` (MR13 = S12 then H4) |
| Reads (via `rh-contracts`) | None new. The M4 entitlement read is deferred (REV-SPR-016-FOLD R2); the factor is exactly 1 |
| RNG purposes added | **None.** `death_time` keeps position 0 with its frozen key and `draws`; MR10–MR12 change only the probability its k = 0 draw is compared against (R1) |
| Golden-hash effect | **`household` and `hamlet` move; `solo` and `smoke` do not.** Re-recorded by `bin/regen_baseline.sh` in the final `[C3]` commit, announced to the director before the merge; the divergence record is T-05's |
| Fixed-point quantities | `Q32_32` (new): `exp`, `ln_ratio`, `prob_from_hazard`, `widen`, `narrow_saturating`, `Prob32::from_q32`; `i128`/`u128` internals at 64 fraction bits; `lambda_i`, `w_i`, `m1`, `m2`, `mu`, `a`, `b`, `y_j`, `S_j` as MR1–MR8 state; `m_d` as `Q16_16`; every division `rhe`, exactly once |
| Determinism lints (GOV-003 §2, §6) | Must pass. No float enters `rh-core` (the `float-api` rule and clippy's `disallowed_methods` on `std` transcendentals already guard `rh-sample`; the same clippy configuration covers `rh-core`); compile-fail controls for implicit `Q32_32` mixing (SPR-010's pattern); no new plane-A dependency |
| Tests that gate this sprint | V-DET-09's method (golden vectors at opt-levels 0 and 3) for `exp`/`ln` and the schedule, V-DET-12, V-DET-14 (the MR12 re-sample), V-DET-19, V-DET-01, V-DET-02, V-DET-05, V-INV-08 (INV-M2-20, 21), V-SF-02 (unchanged at β = 1), V-WC-04 (the `count` fixture), V-GOLD-01 (two new baselines; `solo` unchanged), V-PERF-04 and V-PERF-11, plus `bin/check_all.sh`. V-SF-02b is reported |
| Docs updated in the same sprint | `bin/check_all.sh`, `python/tests/test_check_all_inventory.py` (same commit); `CLAUDE.md` (the new gates; the mortality reference's line now reads the promoted spec); `05.1` §6 item 4 (the vectors' source; version note); `05.0` §6 (`Q32_32` in `rh-core::fixed`); the M2 spec 0.3.1 and companion §8; `next/README.md` (the follow-ups list); `docs/09` V-SF-02b and V-INV-08 method lines; `worlds/*/README.md`; BCK-002 A-48 (routed to the director) |

**Rules carried from P0–SPR-015 (every task):**
- **Property tests vary world size and seed:** at least three sizes and eight seeds, fixed-seed SplitMix64; no new crate.
- **A positive control fails for the right reason;** each control isolates one check (REV-SPR-016 R7): deleting that check makes the control pass.
- **A test input exercises the path it claims, and a probe asserts it:** the input landed while a newborn's first-draw timer was pending; a world was over capacity at the pulse; a founder was over 90 − D at a draw.
- **Python tests read no CI environment variable.**
- **Statistics only where SPR-012 already gates them** (the pooled q(15) at β = 1, its power stated); everything new is reported.
- **CI time:** each part at or under 12 minutes (15 hard), reported; T-05 says what moves to the nightly tier.
- **Merge `origin/main` when main moves,** and re-run both parts.
- **Every new gate is in exactly one `check_all.sh` part,** with the inventory test updated in the same commit.
- **Baselines are regenerated only by `bin/regen_baseline.sh`,** in the final `[C3]` commit; test goldens by their own tools. **Mutants never make a baseline** (`mutant-*` features; BCK-002 A-52).
- **One commit per task.**

## Tasks

T-01 and T-02 start at once (no dependency on SPR-015). T-03's newborn cases, T-04 and T-05 land after SPR-015 merges and `origin/main` is merged in.

#### T-01: `Q32_32` and the transcendentals in `rh_core::fixed` (`05.1` §6 item 4; REV-SPR-016 R5, R12)
- **Deliverable:** `crates/rh-core/src/fixed/q32.rs` and `translog.rs`:
  - `Q32_32` with SPR-010's rules (saturating `add`/`sub`/`mul`, one rounding, no float conversion under any feature, no implicit mixing), `Q16_16::widen`, `Q32_32::narrow_saturating`, `Prob32::from_q32` (refused outside [0, 1]);
  - `exp(y: Q32_32) -> Result<Q32_32, FixedError>`: E1–E6 exactly, the 32 constants `C_k`, `LN2_64`, `INV_LN2_62` as `const`s; `ExpDomain` above 16; 0 below −23;
  - `ln_ratio(n: u64, d: u64) -> Result<Q32_32, FixedError>`: L1–L6; `LnDomain` outside `0 < n, d < 2^63`;
  - `prob_from_hazard(h: Q32_32) -> Result<Prob32, FixedError>`: `2^32 − exp(−h).bits`, `HazardDomain` below 0;
  - no `logistic`, no `pow` (R12: a power is `exp(β · ln(·))` at the call site, rounded once from 48 to 32 fraction bits).
- **Acceptance:**
  - **The golden file.** `crates/rh-core/tests/exp_ln_golden.rs` reads `specs/numerics/golden/exp_ln_v0.txt`: every constant and every vector reproduced bit for bit, in debug and in `--release`; the file's version line is `rehoboam/fixed/exp-ln/v0` or the test refuses it. The altered-vector control (`tests/lint_fixtures/golden/altered_exp_ln_v0.txt`) fails by name.
  - **Properties (SplitMix64, ≥ 10⁴ cases):** `exp` and `ln_ratio` monotone on their domains; `exp(0) = 2^32`; `ln_ratio(n, n) = 0`; `prob_from_hazard(0) = 0`, `= 2^32` for every `h ≥ 23 · 2^32`, non-decreasing; `exp(ln_ratio(n, d))` within 2 ulp of `n · 2^32 / d` on the Brass standard's 69 odds; each domain refused by name.
  - **Against the reference, beyond the committed vectors:** 2^14 `--emit` vectors in the PR tier and 2^20 nightly (the specs table), bit-equal.
  - **Compile-fail controls:** `Q32_32 + Q16_16`, `Q32_32 as f64`, `f64 as Q32_32` do not compile (SPR-010's doctest pattern, counted by `check_all`).
  - **Lints.** `lint_forbidden`'s float rule and clippy's transcendental ban are green on `rh-core`; V-PERF-04 and V-PERF-11 are unchanged (nothing runs on the per-person path).
- **Status:** [ ] not started

#### T-02: The F1 schedule at construction: MR1–MR9 (REV-SPR-016 R2, R6, R13; INV-M2-21)
- **Deliverable:** `rh-sys-demography/src/schedule.rs` gains the F1 compile beside rung 0's:
  - MR1's 69 `lambda_i` as compiled constants; MR2's parameters (`E`, `B`, `K_cap`, `S_d`); MR3's odds grid, the rational form at `B = 65536`, else `exp(rhe(B · lambda_i / 2^16))`; MR4's survivors in both forms; MR5's bisection on the standard grid; MR6; MR7's thirteen infant knots from `infant_sub_schedule.toml`; MR8's tail (m1, m2, mu, a, b, the 361 knots, the forced monotonicity, the zero knot) with its three preconditions as `SimError::Construction` naming MR8; MR9's 438-knot table and `l(x)`.
  - **`bin/gen_m2_reference.py` extended** (importing `selftest_m2_f1_mortality.py`'s functions: one reference): a `schedule_f1` section with, for the eight check-value points, the four Registry corners and pseudo-random `(e0, β)`: `K`, `a`, `b`, the zero knot, all 438 knots; `ell` samples on both calendars including ages under 1 and over 90; the `q` and `qresample` cases of T-03. Written to `crates/rh-sys-demography/tests/golden/m2_f1_schedule_v1.txt`, read by `tests/schedule_f1_golden.rs`.
- **Acceptance:**
  - **Bit-equality** with every line, debug and release; the altered-line control fails by name (`K at e0 bits … beta …`).
  - **Rung 0 is the β = 1 case:** at `B = 65536`, for the seven rung-0 check values and 64 pseudo-random e0, the F1 compile's `K`, `L_0..L_70` and `l(x)` at 10⁴ sampled minutes in [Y, 90Y] on both calendars equal the rung-0 path's and `m2_schedule_v1.txt`'s. A planted `mutant-beta-one-power` (the power form at β = 1) fails this by name (the Rust twin of the reference's control).
  - **INV-M2-21** at every compiled point: 438 knots strictly increasing in age, non-increasing in `l`, `l(90) = L_66`, `l(x) = 0` from the zero knot and at every `x ≥ 120Y`.
  - **Corners.** The Registry box compiles with no error at (20, 0.6), (20, 1.65), (85, 0.6), (85, 1.65); the zero knots are the spec's `l_zero_twelfths` (1314 at (20, 1.65), 1361 at (60, 1.65), 1381 at (85, 1.65), 1408 at (30, 1.3), 1440 elsewhere).
  - **Refusals by name:** `brass_alpha` set (exists); each MR8 precondition through the test hook; an `exp` domain error during construction surfaces as a load error naming MR3 or MR8.
  - **Reported:** the time of one compile (the 69 `exp`/`ln` pairs and the 361 tail knots), and the F1 schedule's own trapezoid e0 against E at the eight points (the reference's check 8 figures: −0.06 years at e0 30, +0.5 at e0 85).
- **Status:** [ ] not started

#### T-03: The monthly probability, the modifier, the draws and the parameters: MR10–MR12, MR2 (REV-SPR-016 R3, R4, R9, R11; REV-SPR-016-FOLD R4; INV-M2-20; V-DET-14; V-SF-02; V-SF-02b)
- **Deliverable:**
  - **MR10:** `q` from MR9's `l`; the newborn rule for `b > t_(m−1)` at `m ≥ 1`: `q = 2^32 − l(x + D)`.
  - **MR11:** `N` from M2's own `alive` column once per pulse before any draw; `m_d`; `q' = min(2^32, rhe(q · m_d / 2^16))`; the entitlement factor exactly 1 (`q'' = q'`), its form present and tested through a unit test of `f = exp(rhe(eps · ln_ratio(w, 65536) / 2^16))` on the clamp range, so that P1.5a turns it on without a new procedure.
  - **MR12:** S9–S11 on `q''`; the re-sample: `q_r` by S11's conditional for an ordinary person, and `2^32 − rhe(l(x) · l(x1) / l(x0))` for a person whose pending timer was drawn under the newborn rule; `N` from `ResampleCtx::living_count()` (the specs table; `rh-core`); nothing stored.
  - **`rh-manifest` and `manifest_v0.py`:** `brass_beta` accepted anywhere in the bounds; the `count` type (an integer literal, integer bounds, a non-integer refused by name); `density_capacity` and `density_mortality_slope` compiled with their defaults; the compiled `WorldSpec` carries six M2 parameters.
  - **`bin/gen_m2_f1_draws_reference.py` extended** (SPR-015's independent model gains MR7, MR8, MR10, MR11, MR12): fixture worlds with founders aged 0.3–0.9 years, founders aged 85–95 and, at (20, 1.65), aged 105 (the zero knot at 109.5), newborns through births, inputs that scope a newborn's pending first-draw timer, a world of 30 founders with `density_capacity: 10`, both calendars. It writes every death instant and the `m_d` of every pulse of the over-capacity world.
- **Acceptance:**
  - **Bit-equality** with the draws golden (every death instant, debug and release); the altered-line control fails by name.
  - **Planted mutants, each failing the golden by name:** `mutant-newborn-s8` (rung 0's S8 for a newborn: the uncovered first part-month), `mutant-resample-recharge` (`2^32 − l(x1)`: the re-charged reading), `mutant-resample-conditional` (S11's conditional for a newborn: the deferred hazard dropped), `mutant-density-symmetric` (`m_d = N / K_cap` below capacity: the over-capacity world's and a dying world's draws move).
  - **INV-M2-20** on random worlds under capacity: every `q''` equals MR10's `q` bit for bit (a test-only probe of the step's probabilities); on the over-capacity fixture `q' > q` exactly as `m_d` says, and `m_d` is capped at 4.
  - **`ResampleCtx::living_count()`** equals the ledger's living count at the Inputs phase start, before the input's effects, on random worlds; a keyframe round trip holds; thread invariance at 1, 2, 4 and N on the fixture worlds.
  - **V-SF-02 unchanged:** SPR-012's 10,000-founder q(15) test (32 seeds, pooled z) passes at β = 1 with the F1 schedule in place, with the same pass line as today.
  - **V-SF-02b reported** (nightly, release): l(65)/l(15) of 10,000 founders aged 15 at β ∈ {0.64, 1, 1.64}, 32 seeds, pooled, against the schedule's own 0.481, 0.315, 0.139 at e0 30 within ± 0.03.
  - **Parity (V-WC-04):** a fixture with `density_capacity: 1500.5` is refused by both validators naming the parameter; `density_capacity: 1500` passes both; `brass_beta: 1.3` passes both; `brass_alpha: 0.2` is refused by both.
- **Status:** [ ] not started

#### T-04: The 0.3.1 bump (Architect, C2 inside the C3 PR; REV-SPR-016-FOLD R3 as changed, R6)
- **Deliverable:** `specs/systems/M2.demography.odd.toml` 0.3.1 with exactly the changes of the specs table's "0.3.1 content" row; the skeleton regenerated (`SPEC_VERSION = "0.3.1"`, nothing else); the companion §8 and `next/README.md` updated; the interim-rule code of SPR-015 T-02 removed.
- **Acceptance:** `lint_odd.py` passes; `gen_skeletons.py --check` passes and a diff of the skeleton shows only the version line; `selftest_m2_f1_mortality.py --check` and `selftest_m2_f1.py` pass against 0.3.1 (their check values are untouched); M2 registers against 0.3.1 under rule `spec`; the two stale-value controls still fail by name; `gen_manifest.py --check` passes.
- **Status:** [ ] not started

#### T-05: The two baselines, the divergence record, the gates and CI time
- **Deliverable:**
  - **The final commit,** subject ending `[C3]`: `household` and `hamlet` regenerated by `bin/regen_baseline.sh`; `registry_version` and the manifest pins unchanged (the Registry did not change).
  - **The divergence record,** in the PR body and in `VER-SPR-016`, per world: the bisect's first divergent month on a monthly ladder, the person it names and which of the three classes they fall in; the differing tables at every yearly rung.
  - **The gates:** part `core`: `exp_ln_golden` (debug), the PR-tier `--emit` comparison, the compile-fail controls, `schedule_f1_golden` and its control, the draws golden with its four mutants, the `count` parity fixture, the `ResampleCtx` test. Part `golden`: `exp_ln_golden` in release (as `rh-sample`'s), `schedule_f1_golden` in release, the two moved ladders, the unchanged `solo` and `smoke` ladders, the q(15) test as today. Nightly: the 2^20 sweep, V-SF-02b. The inventory test in the same commit.
  - The docs of the compliance table.
- **Acceptance:**
  - **Robust form.** `solo` and `smoke` are byte-identical at 1 and N threads, headers included. For `household` and `hamlet`, the first divergent month names a person under one year old at the pulse, a person born since the previous pulse, or a person over 90 − D; from that month on every table may differ. A first divergence naming anyone else, or `solo` moving, is a defect.
  - **Ladders.** The PR-tier ladders equal the new baselines at 1 and N threads; the nightly monthly ladders at 1, 2, 4 and N × 3 on the head; the parallelism floors hold or are restated.
  - **Channel A.** On `household` and `hamlet` every line before the first divergent month is byte-identical to SPR-015's segments.
  - **Performance.** V-PERF-04 and V-PERF-11 within budget (the F1 compile runs once at construction; the per-person path gains one multiply for `m_d` and one comparison for the newborn rule). If a recorded count moves, the tool re-baselines in a `[perf-baseline]` commit with a reason line.
  - **CI time.** Both parts at 12 minutes or less, reported. Estimate: core +1 min (the vector files, the schedule and draws goldens), golden +0.5 min (two release golden tests). If a part exceeds 12 minutes, the release `exp_ln_golden` run moves to the nightly by the Architect's ruling recorded in the audit, as SPR-017 T-09 provides for.
- **Status:** [ ] not started

## Director items, and what changes under the alternative

| Item | Recommended | Under the alternative |
|---|---|---|
| **BCK-002 A-48: the smooth (C¹) logit interpolant from age 1** (REV-SPR-016 R6, R13; "decided at SPR-016's planning"). Recommendation: **defer; not built in SPR-016 (7/10)** | The F1 schedule keeps rung 0's linear-`l` interpolation between grid points (MR9 = S7's rule), with its monthly-q teeth (at e0 30, β 1: under 7% from 50 to 72.5, −11.5% to −36% from 75 to 87.5, −31% at the join at 90, −57% at age 2; the reference prints them). Why: (1) the β = 1 bit-equality with rung 0 is the proof this sprint rests on (check 1, the schedule golden, V-SF-02 unchanged); (2) A-48 has no spec text, no exact reference and no golden, so GOV-005 §2 forbids building it; (3) its effect is judged by SPR-028's harness on the age-at-death distribution, which does not exist yet; (4) the sprint is at 3 agent-days. A-48 becomes a P1b C3 sprint with its own spec round, after SPR-028 | **Build it here:** a spec round first (a new `[algorithm.*]` table or an `[[amends]]` of S7 under a new `mortality_rung0` version, the reference's PCHIP-on-the-logit knots, a golden), then the code. Rung 0's equality at β = 1 is lost wherever the interpolant applies (ages 1 to 90), so `solo` moves too, V-SF-02's q(15) band moves slightly, and the verification strategy becomes "equal to the new reference" alone. About +1.5 agent-days and a second review round. The teeth are real but small in absolute q (about 10⁻³ a month in childhood); the harness will show whether they matter for the saga |
| **D40 and D41** | Unchanged. Nothing in this sprint touches `tfr`, the marriage pattern or the V-SF-24/25 gating | — |

## Out of scope (explicit)
- **A-48** (above). **Sex-specific schedules**, **era-state shifts of α** (`medical_level`, P1b), **maternal mortality** (F2), **minute-level death times** (A-35): the spec's `not_yet`.
- **The M4 entitlement read** and `eps_f`, `eps_m` (P1.5a; REV-SPR-016-FOLD R2). The form of MR11's factor is present and unit-tested; it is never evaluated in P1a.
- **A settlement capacity** (M1 F0 / M3): `density_capacity` is the world's one capacity; `kingdom_10k.yaml`'s `density_capacity: 10000` is SPR-030's first C1 change (REV-SPR-030 R1a).
- **The demography bands as gates** (V-SF-01, V-SF-02b; SPR-028). V-SF-02 stays the gate it is.
- **Named `Q16_16` forms of `ln`** until a consumer needs them (R12 Q2).
- **The births slice** (SPR-015), which this sprint builds on and does not change.

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | T-01, T-02, T-03, T-05, written from `05.1` §6 item 4, MR0–MR13 and ADR-0025 item 3 before reading the code:<ul><li>their own implementation of E1–E6 and L1–L6 from the `05.1` text (the independent review did this in Python; the Tester repeats it against the Rust on 10⁵ inputs);</li><li>their own MR1–MR10 implementation against the Rust at the eight check values and 64 box points, and the β = 1 equality on both calendars;</li><li>their own S8–S12 + MR10–MR12 model (the SPR-012 Tester's model, extended) predicting every death instant of the fixture worlds, re-samples and the over-capacity world included;</li><li>the four mutants killed for their reasons, plus three of their own;</li><li>debug and release bit-equality; thread invariance; keyframe round trips;</li><li>the divergence re-measured and checked against the robust form (`solo` and `smoke` unchanged);</li><li>the nightly ladders and the 2^20 sweep on the head</li></ul> | `VER-SPR-016-TEST` |
| AGT-003-STAT | T-03's bands, written from `docs/09` V-SF-02 and V-SF-02b and the spec's facts:<ul><li>the q(15) band unchanged at β = 1 (the pooled z and its power, as SPR-012 stated them);</li><li>V-SF-02b's l(65)/l(15) at the three β with intervals, INSUFFICIENT stated honestly;</li><li>the realised e0 of the F1 schedule against E at the design points;</li><li>no band adjusted</li></ul> | As above |

## Merge route

- **Class and conditions.** C3, merged by the Architect after the Tester passes, the audit and the announcement of the two baseline changes to the director, who may veto (GOV-005 §6). The PR title and the final commit's subject end `[C3]`. GOV-005 §6's sampler clause (the nightly tier green on the head before a C3 merge of `rh-sample` or its reference) does not apply by its letter (`rh-core::fixed`, not `rh-sample`), but its reason does: **the nightly 2^20 sweep and the monthly ladders run on the PR head before the merge** (the Architect's rule for this sprint, 8/10).
- **One C3 open for merge at a time (GOV-005 §4.1).** This sprint merges **after SPR-015** (PRJ-002 §3's merge-order edge): its PR is opened as a draft when T-01 lands and stays a draft until SPR-015 is on `main`; then `origin/main` is merged, the differential model is rebased on SPR-015's, T-03's newborn cases, T-04 and T-05 land, and the divergence is measured from SPR-015's baselines.
- **If SPR-015 slips past this sprint's readiness** (T-01, T-02 and the non-newborn part of T-03 done): the PR waits. It does not merge first: it would promote 0.3.0 itself, carry the interim rule it exists to remove, and have no newborn to test MR10 on.
- **Shared files.** With SPR-015: `rh-sys-demography`, the M2 spec, the two generators, `check_all.sh`, the inventory test, `CLAUDE.md`. With SPR-027: `check_all.sh`, the inventory test, `CLAUDE.md` only. Whichever merges second resolves the textual conflicts.

## Blockers

| # | Blocker | Status |
|---|---|---|
| 1 | SPR-015 not merged (T-03's newborn cases, T-04, T-05) | Open |
| 2 | The Architect's rulings of the specs table, above all the vectors' source (before T-01), `ResampleCtx::living_count()` and the `count` type (before T-03) | Open |
| 3 | The director's ruling on A-48 (recommended: defer) | Open |
| 4 | The director's plan approval (D54; GOV-005 §2) | Open |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS (DET, STAT).
- [ ] Architect audit `VER-SPR-016` is PASS, with the divergence record of T-05 and the nightly sweep on the head.
- [ ] No open DEF- against this sprint.
- [ ] The merge route matches the CI-detected change class (C3); the two baseline changes were announced to the director first; `solo` and `smoke` are unchanged.
- [ ] M2 0.3.1 is frozen on `main` with no interim rule left; `05.1` §6 item 4 names the vectors' source; BCK-002 A-48 carries the director's ruling.
