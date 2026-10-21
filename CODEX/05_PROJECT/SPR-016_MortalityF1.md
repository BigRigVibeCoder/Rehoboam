---
id: SPR-016
title: "M2 F1 mortality: fixed-point exp and ln, Brass at any (alpha, beta), the infant sub-schedule, the old-age tail and the density modifier; the 0.3.1 bump (P1.3, part b)"
type: how-to
status: ARCHIVED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-002, PRJ-002, REV-SPR-016, REV-SPR-016-FOLD, REV-SPR-015, SPR-010, SPR-011, SPR-012, SPR-014, SPR-015, SPR-027, SPR-028, SPR-030, ADR-0006, ADR-0024, ADR-0025, ADR-0033]
created: 2026-10-17
updated: 2026-10-17
version: 0.2.0
---
> [!NOTE]
> **ARCHIVED BASELINE:** Traditional sprint-based tracking has been superseded by Specification-Driven Development (SDD v2.0). All future deliverables are tracked as Spec Waves in `.spec/ARCHITECT_BACKLOG.md`.

> **BLUF:** Builds `05.1` §6 item 4's fixed-point `exp`, `ln` and `prob_from_hazard` on a new `Q32_32` type in `rh_core::fixed`, bit-equal to `specs/numerics/exp_ln_v0.py`'s golden file, and the M2 spec's `[algorithm.mortality_f1]` MR0–MR13 in `rh-sys-demography`: Brass at any (α, β) on a 438-knot schedule, the infant sub-schedule below one year, the logistic old-age tail to 120, the density modifier (inert in every P1a world), the newborn's first draw and the deferral-consistent re-sample. At β = 1, under capacity, ages 1 to 90, the F1 schedule reproduces rung 0 bit for bit (the reference's check 1). `rh-manifest` accepts any β in the Registry bounds and compiles the two new parameters, refusing a non-integer `density_capacity` as MR2 says. SPR-012's draws golden is folded into the F1 model (W19). The frozen M2 moves from 0.3.0 to **0.3.1** (the interim rule out).
>
> **Two baselines move** (`household`, `hamlet`): newborns' and infants' first months and anyone past 90 − D draw against a different q. **`solo` is predicted unchanged** (Ada is 25 at genesis and dies at 57) and `smoke` has no M2; either moving is a defect. Re-recorded in the final `[C3]` commit by `bin/regen_baseline.sh`, announced to the director before the merge. SPR-012's q(15) founders test keeps its band and gets a re-derived null (W18).
>
> - **Developer:** AGT-002-CORE, with AGT-002-TOOLS for the references' additions and the Python-side parity (one session may hold both).
> - **Testers:** AGT-003-DET (bit-equality, the independent model, the divergence), AGT-003-STAT (q(15), V-SF-02b) and AGT-003-IF (the C2 surfaces: the 0.3.1 bump, `compile`'s refusals, `ResampleCtx::living_count()`; W24). One session may hold them, on a different model from the Developer's.
> - **Size and class:** **about 3.5 agent-days** (W25). **C3**: a model change that moves two goldens, and new determinism machinery (a fixed-point transcendental procedure; `05.1` §6 item 4: "Changing a constant, the step count, the working precision, a domain or a rounding is a C3 change with a new format version"). It contains C2: the 0.3.1 bump, `rh-manifest`'s compile-level `count` refusal, an additive `ResampleCtx` method with its `05.1` §2 rule 2 text.
> - **Status:** PLANNING, revision 1: the Architect's rulings on the independent review are applied (W18–W28, W42, D-3, D-6). It needs the director's ruling on A-48 (see "Director items") and the plan approval (D54; GOV-005 §2). **Merges after SPR-015** (PRJ-002); T-01 and T-02 may start at once.

# SPR-016 — M2 F1 mortality (WP P1.3, part b)

- **Roadmap item:** `docs/11` P1.3 (b) · **Phase / rung:** P1a, rungs 0–2
- **Depends on:**
  - **SPR-012 merged** (code): `rh-sys-demography`'s rung-0 schedule and draws, `bin/gen_m2_reference.py`, `bin/gen_m2_draws_reference.py` with its five re-sample cases and two mutants, the altered-golden controls, the q(15) test (`crates/rh-worldgen/tests/m2_system.rs`).
  - **SPR-015 merged before T-03's newborn cases, T-04 and T-05:** the births that make newborns exist, the promoted 0.3.0 this sprint bumps, the Registry carrying `density_capacity` and `density_mortality_slope`, SPR-015's differential model (`bin/gen_m2_f1_draws_reference.py`), into which T-03 folds the deaths. T-01 and T-02 depend on SPR-010 and SPR-012 only (PRJ-002 §4 wave 2: "machinery with no consumer yet").
  - Independent of SPR-014's and SPR-020's content beyond `main` having moved (T-05 re-measures from whatever baselines are on `main`).
- **Binding contracts:**
  - `05.1` §6 item 4 (v1.10/v1.11: `Q32_32`, the functions, domains, steps E1–E6 and L1–L6, the rounding and the error bound, format `rehoboam/fixed/exp-ln/v0`; as corrected at this plan: the Rust tests read the reference's golden directly); `specs/numerics/exp_ln_v0.py` and `specs/numerics/golden/exp_ln_v0.txt` (9,377 lines: `const`, `exp`, `ln`, `hazard`).
  - `[algorithm.mortality_f1]` MR0–MR13 of the M2 spec (0.3.0 on `main` after SPR-015), its eight `check_values` and the facts below them; `[algorithm.parameters_f1]` P1–P2 (the interim rule this sprint removes); INV-M2-20, INV-M2-21; `specs/demography/brass_general_standard.toml`, `infant_sub_schedule.toml`; `specs/demography/selftest_m2_f1_mortality.py` and `golden/m2_f1_mortality.txt` (`schedule`, `knot`, `q`, `qnew`, `qresample`, `modifier`, `qmod`).
  - `05.1` §2 rule 2 (the trait `resample_timer(&self, &TimerEntry, &ResampleCtx) -> Result<Option<SimTime>, SimError>`; **this sprint's T-03 adds the `living_count()` text**, the Architect's wording in the PR), §6 items 2, 4, 9; `05.5` §3.2 (authored numbers); `05.2` §1.
  - ADR-0006, ADR-0025 item 3, ADR-0033; `docs/09` V-DET-09's method (golden vectors at both optimisation levels), V-DET-12, V-DET-14, V-DET-19, V-INV-08, V-SF-02, V-SF-02b, V-WC-04, V-GOLD-01.
- **System spec:** M2 goes from frozen 0.3.0 to **frozen 0.3.1** in this PR (T-04; an Architect-approved C2 change inside the C3 PR). Nothing algorithmic changes (the specs table's "0.3.1 content" row).
- **Rulings (binding):**
  - REV-SPR-016 R1–R14, above all R1 (the frozen `draws` text untouched), R2 (certain death from the zero knot), R3 and R4 (the newborn's first draw and its re-sample formula), R5 (the error budgets, the sweep's counts), R6 and R13 (the mid-interval anchor kept; the sawtooth stays), R7 (one control, one check), R9 (N at the Inputs phase start), R11 (A8 the density modifier; A10 `brass_alpha` refused), R12 (no named `Q16_16` forms until a consumer needs them);
  - REV-SPR-016-FOLD R1 (P1 names six parameters), R2 (the M4 read deferred), R4 (`type = "count"`), R6 (the promotion checklist), and R3 as withdrawn by D-1 (0.3.0 promoted in SPR-015; this sprint bumps to 0.3.1);
  - the design values DV-SPR016-1 to 4 (the Architect's, companion §8.3);
  - **the Architect's rulings on the independent review of this plan (2026-10-17): W18–W28, W42, D-3, D-6** (the specs table marks each row they decide).
- **Director items:** **BCK-002 A-48** (the smooth interpolant: a model change; see "Director items"). D40 and D41 are approved and untouched.
- **Requirements covered:** DET-3 (fixed point: `Q32_32` and the transcendentals, the one procedure); DET-2 (keyed RNG: no new purpose); SIM-5 (hazards and binomial draws at any (α, β)); DATA-8 (death records unchanged in form); DET-13 (the re-sample never back-dates).

## Specs this sprint needs

| Spec | Exists? | Gap | Architect ruling | Needed by |
|---|---|---|---|---|
| **BCK-002 A-48**, the C¹ logit interpolant into monthly knots from age 1 | "Decided at SPR-016's planning" (REV-SPR-016 R13) | A model change with no exact reference, no spec text and no golden; it trades the β = 1 bit-equality with rung 0, on which checks 1 and 7 and the whole verification strategy of this sprint rest | **Not built here; a director item** (the recommendation: defer to a P1b C3 sprint with its own spec round, after SPR-028's harness; 7/10). The reviewer found the deferral sound | — |
| **Where the Rust vectors come from, for `exp`/`ln` and for the schedule** | `05.1`: "SPR-016 extends `bin/gen_fixed_vectors.py` with it"; the rung-0 precedent copies vectors into `crates/rh-sys-demography/tests/golden/` through `gen_m2_reference.py` (W26) | A second copy can drift from the reference's golden; two tasks with two policies | **Ruled (Architect, 2026-10-17; 6.5/10; D-6, W26): one policy.** The Rust tests read the two references' committed golden files directly: `specs/numerics/golden/exp_ln_v0.txt` and `specs/demography/golden/m2_f1_mortality.txt`. Where a vector family the engine needs is missing (pseudo-random `(e0, β)` points, `ell` samples at arbitrary minutes on both calendars, the newborn and over-capacity draws), the Python reference gains it and regenerates its own golden (a tool change inside the PR; the reference stays the oracle). `gen_m2_reference.py` is not extended for F1, and `05.1` §6 item 4's sentence is corrected (done at this plan) | T-01, T-02 |
| **The Rust nightly sweep** | `--exhaustive` is Python-only (`bin/golden_nightly.sh`); Rust has no exhaustive tier | The Rust port is proven only on the committed vectors | Proposed (7/10): `exp_ln_v0.py --emit N --seed S` writes N pseudo-random `Q32_32` vectors of both functions (not committed); an ignored Rust test reads them and checks bit-equality; `golden_nightly.sh` runs 2^20 of each nightly, the PR tier 2^14 through the same path | T-01 |
| **`type = "count"` parameters** | MR2: "the manifest literal must be an integer and a non-integer literal is a load error naming the parameter, since the manifest validator checks bounds only"; REV-SPR-016-FOLD R4: `--rust` emits `u64` | My draft made both validators refuse a non-integer, which departs from MR2 and misreads V-WC-04 (it compares `validate`, not `compile`) (W20) | **Ruled (Architect, 2026-10-17; 8/10; W20): keep MR2.** The validator checks bounds only; `rh_manifest::compile` refuses a non-integer `count` literal by name; a Rust compile-level test with a fixture manifest covers it; the `brass_alpha` parity line is dropped (its refusal is `compile`'s too); no Registry type change | T-03 |
| **MR12's N in a re-sample** | MR12: "N is the living count at the start of the Inputs phase of `t_in`, before the input's own effects, counted once per input" | **Gap.** `resample_timer` sees no world (`ResampleCtx { key, purpose, entity, t_input, epoch }`), so M2 cannot count N there, and the spec says nothing is stored | **Ruled (Architect, 2026-10-17; 7/10; D-3, W21): `ResampleCtx::living_count()`, an additive core-only read, listed as a `05.1` §2 rule 2 edit and worded exactly as MR12 is:** `rh-core` counts the ledger's `alive = 1` rows at the start of the Inputs phase of `t_in`, before the input's own effects, once per input, and hands it to every re-sample of that input. Hash-neutral (every P1a world is under capacity, so q'' = q_r whatever N is); a planted over-capacity fixture world exercises it | T-03 |
| **Construction errors by name** | MR0: "a refusal during construction is a load error naming the step"; MR8: "a construction error naming MR8" | `DemographyError { AlphaRefused, BetaNotOne, Schedule(ScheduleError) }` already exists in `rh-sys-demography` (W22) | **Ruled (Architect, 2026-10-17; 8.5/10; W22): extend `DemographyError`** with `Construction { step, reason }` (and drop `BetaNotOne`, since β is accepted); no new `SimError` variant. `rh-cli run` surfaces it as a load error. A test reaches each MR8 precondition through a test-only schedule hook outside the Registry box | T-02 |
| **The baseline prediction** | MR13's header: "a founder under one year old, a child born since SPR-015 (its first months) or a person aged over 90 − D at a pulse draws against a different q; nothing else" | Which worlds move | Proposed (7/10), found sound by the reviewer: `household` and `hamlet` move; `solo` is unchanged; `smoke` is unchanged; no `registry_version` bump is needed. The bisect's first divergent month names a person in one of the three classes; a divergence naming anyone else is a defect | T-05 |
| **V-SF-02's q(15) test** | SPR-012's test (`crates/rh-worldgen/tests/m2_system.rs:338–375`): 10,000 founders aged U[0, 1) at genesis, 32 seeds, a pooled z against a null derived from rung 0's `l` | **"Unchanged at β = 1" is false** (W18): a founder aged x in (0, 1) at genesis faces the infant knots, not rung 0's linear `l`; the reviewer's `q15_shift.py` gives a mean death probability by 15 of 0.34688 under rung 0 and 0.30646 under F1 at e0 30, β 1, a pooled z of about −50 against the old null. V-SF-02's band itself is unchanged: l(0), l(1) and l(15) are rung 0's, and a birth cohort's q(15) does not move | **Ruled (Architect, 2026-10-17; 9/10; W18): T-03 re-derives the test's null from MR9's `l` on [0, 1) and states the new z values**; the band and the pooled-z form stay | T-03 |
| **V-SF-02b's report tier** | `docs/09` V-SF-02b: l(65)/l(15) inside the range (α, β) implies ± 0.03; the spec's facts give 0.481, 0.315, 0.139 at e0 30 for β 0.64, 1, 1.64 | Not run anywhere today | Proposed (7/10): a release test on 10,000 founders aged 15 at β ∈ {0.64, 1, 1.64}, 32 seeds, pooled, reported; nightly | T-03 |
| **A world above capacity** | DV-SPR016-1: inert in every P1a world | Nothing on `main` exercises m_d > 1 | Proposed (8/10): a test manifest (not a golden world) with `density_capacity: 10` and 30 founders; the differential model carries its m_d trajectory | T-03 |
| **SPR-012's rung-0 draws golden** | `gen_m2_draws_reference.py`: six fixture worlds whose founders are aged 50 to 100 years, with certain death in the month a person reaches 100 under rung 0's S8 (w6's founders at 99.95 give the `last_day` c = 0 case); `draws_golden.rs` with `mutant-resample-hit-k1` and `mutant-resample-day-k0` | **Breaks under F1** (W19): the tail runs to 120, so no death is certain at 100 | **Ruled (Architect, 2026-10-17; 7.5/10; W19): fold.** `gen_m2_draws_reference.py` and `draws_golden.rs` move to the F1 model (SPR-015's `gen_m2_f1_draws_reference.py` and `f1_draws_golden.rs`, which gain MR7, MR8, MR10–MR12) and keep the five re-sample cases (`last_day`, `duplicate`, `mid_month`, `own_instant`, `no_pending`) and both mutants; the `last_day` case moves to a world whose founders reach the F1 zero knot (e0 20, β 1.65, founders at 109.4 years, or the cap at 120). **There is no test-only rung-0 path in plane A.** `CLAUDE.md`'s lines follow | T-03 |
| **The 0.3.1 content** | REV-SPR-016-FOLD R3's interim rule is frozen text after SPR-015 | What the bump may change | **Ruled (Architect, 2026-10-17; 8/10; W23): P1 loses its interim sentences and its table version becomes `m2-f1-parameters-v2`; `not_yet` item 2 (the interim newborn exposure) leaves; `not_yet` item 7's "a decision at SPR-016's planning" reads "deferred by the director (D54) to a P1b sprint after SPR-028"; INV-M2-20 and 21 lose the "apply from SPR-016" clause and the comment above them; the header's lifecycle paragraph and `[fidelity_ladder]` follow. Nothing in MR0–MR13, S0–S12, F, N, H or L changes. No `registry_version` bump: the Registry does not change** | T-04 |
| **Compile-fail controls** | SPR-010's controls are fixture crates under `tests/lint_fixtures/compile_fail/fixed_point/` (`float-cast`, `mix-q8-q16`, …), not doctests (W27) | — | **Ruled (Architect, 2026-10-17; 8.5/10; W27): new fixture crates there** (`mix-q32-q16`, `q32-float-cast`, `q32-float-from`, `q32-into-prob`), counted by `check_all`'s compile-fail gate | T-01 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C3**: a model change (the F1 schedule) that moves `state_hash` on `household` and `hamlet`, and new determinism machinery in `rh_core::fixed` (format `rehoboam/fixed/exp-ln/v0`). It contains C2: M2 0.3.1, `rh-manifest`'s compile-level `count` refusal, `ResampleCtx::living_count()` with its `05.1` text |
| Planes touched (ADR-0024) | **A:** `rh-core` (`fixed`: `Q32_32`, `exp`, `ln_ratio`, `prob_from_hazard`; `ResampleCtx`), `rh-sys-demography` (`schedule.rs`, the draws, `DemographyError`). **Tool:** `rh-manifest` (`compile`), `bin/gen_m2_f1_draws_reference.py` (the fold), `specs/numerics/exp_ln_v0.py` (`--emit`), `specs/demography/selftest_m2_f1_mortality.py` (the added vector families), `bin/golden_nightly.sh`, `nightly.yml`, `tests/lint_fixtures/compile_fail/fixed_point/`. **B, C:** none |
| Owned columns written | None new. M2's `person.*` and `life_course.*` exit fields as before, through `apply_death` (MR13 = S12 then H4) |
| Reads (via `rh-contracts`) | None new. The M4 entitlement read is deferred (REV-SPR-016-FOLD R2); the factor is exactly 1 |
| RNG purposes added | **None.** `death_time` keeps discriminant 1, the first entry of `[[rng]]`, with its frozen key and `draws`; MR10–MR12 change only the probability its k = 0 draw is compared against (R1) |
| Golden-hash effect | **`household` and `hamlet` move; `solo` and `smoke` do not.** Re-recorded by `bin/regen_baseline.sh` in the final `[C3]` commit, announced to the director before the merge; the divergence record is T-05's |
| Fixed-point quantities | `Q32_32` (new): `exp`, `ln_ratio`, `prob_from_hazard`, `widen`, `narrow_saturating`, `Prob32::from_q32`; `i128`/`u128` internals at 64 fraction bits; `lambda_i`, `w_i`, `m1`, `m2`, `mu`, `a`, `b`, `y_j`, `S_j` as MR1–MR8 state; `m_d` as `Q16_16`; every division `rhe`, exactly once |
| Determinism lints (GOV-003 §2, §6) | Must pass. No float enters `rh-core` (the `float-api` rule and clippy's `disallowed_methods` on `std` transcendentals cover it); compile-fail fixture crates for implicit `Q32_32` mixing (W27); no new plane-A dependency; no interior mutability (W31) |
| Tests that gate this sprint | V-DET-09's method (golden vectors at opt-levels 0 and 3) for `exp`/`ln` and the schedule, V-DET-12, V-DET-14 (the MR12 re-sample), V-DET-19, V-DET-01, V-DET-02, V-DET-05, V-INV-08 (INV-M2-20, 21), V-SF-02 (the founders test with its re-derived null, W18), V-WC-04 (unchanged: `validate` checks bounds only), V-GOLD-01 (two new baselines; `solo` unchanged), V-PERF-04 and V-PERF-11, plus `bin/check_all.sh`. V-SF-02b is reported |
| Docs updated in the same sprint | `bin/check_all.sh`, `python/tests/test_check_all_inventory.py` (same commit); `CLAUDE.md` (the folded draws golden's lines; the new gates); `05.1` §2 rule 2 (`living_count()`, the Architect's text) and §6 item 4 (the version note); `05.0` §6 (`Q32_32` in `rh-core::fixed`); the M2 spec 0.3.1 and companion §8; `next/README.md` (the follow-ups list); `docs/09` V-SF-02 (the founders test's null), V-SF-02b and V-INV-08 method lines; `worlds/*/README.md`; BCK-002 A-48 (the director's ruling) |

**Rules carried from P0–SPR-015 (every task):**
- **Property tests vary world size and seed:** at least three sizes and eight seeds, fixed-seed SplitMix64; no new crate.
- **A positive control fails for the right reason;** each control isolates one check (REV-SPR-016 R7): deleting that check makes the control pass.
- **A test input exercises the path it claims, and a probe asserts it:** the input landed while a newborn's first-draw timer was pending; a world was over capacity at the pulse; a founder was over 90 − D at a draw; the `last_day` case really had no day start left.
- **Python tests read no CI environment variable.**
- **Statistics only where SPR-012 already gates them** (the founders' q(15) with its re-derived null and stated power); everything new is reported.
- **CI time (W42):** against `e5082d0`'s core 7:58 and golden 10:21 (golden the tighter part), each part stays at or under 12 minutes (15 hard), reported; T-05 states this sprint's added time per part after wave 4 and what leaves which part. A third PR-tier job is the director's call (D54).
- **Merge `origin/main` when main moves,** and re-run both parts.
- **Every new gate is in exactly one `check_all.sh` part,** with the inventory test updated in the same commit.
- **Baselines are regenerated only by `bin/regen_baseline.sh`,** in the final `[C3]` commit; test goldens by their own tools. **Mutants never make a baseline** (`mutant-*` features; BCK-002 A-52).
- **One commit per task.**

## Tasks

T-01 and T-02 start at once (no dependency on SPR-015). T-03's newborn cases and the fold, T-04 and T-05 land after SPR-015 merges and `origin/main` is merged in.

#### T-01: `Q32_32` and the transcendentals in `rh_core::fixed` (`05.1` §6 item 4; REV-SPR-016 R5, R12; D-6; W27)
- **Deliverable:** `crates/rh-core/src/fixed/q32.rs` and `translog.rs`:
  - `Q32_32` with SPR-010's rules (saturating `add`/`sub`/`mul`, one rounding, no float conversion under any feature, no implicit mixing), `Q16_16::widen`, `Q32_32::narrow_saturating`, `Prob32::from_q32` (refused outside [0, 1]);
  - `exp(y: Q32_32) -> Result<Q32_32, FixedError>`: E1–E6 exactly, the 32 constants `C_k`, `LN2_64`, `INV_LN2_62` as `const`s; `ExpDomain` above 16; 0 below −23;
  - `ln_ratio(n: u64, d: u64) -> Result<Q32_32, FixedError>`: L1–L6; `LnDomain` outside `0 < n, d < 2^63`;
  - `prob_from_hazard(h: Q32_32) -> Result<Prob32, FixedError>`: `2^32 − exp(−h).bits`, `HazardDomain` below 0;
  - no `logistic`, no `pow` (R12: a power is `exp(β · ln(·))` at the call site, rounded once from 48 to 32 fraction bits);
  - the compile-fail fixture crates `mix-q32-q16`, `q32-float-cast`, `q32-float-from`, `q32-into-prob` under `tests/lint_fixtures/compile_fail/fixed_point/` (W27);
  - `exp_ln_v0.py --emit N --seed S` (the specs table).
- **Acceptance:**
  - **The golden file, read directly (D-6).** `crates/rh-core/tests/exp_ln_golden.rs` reads `specs/numerics/golden/exp_ln_v0.txt`: every `const`, `exp`, `ln` and `hazard` line reproduced bit for bit, in debug and in `--release`; the file's version line is `rehoboam/fixed/exp-ln/v0` or the test refuses it. The altered-vector control (`tests/lint_fixtures/golden/altered_exp_ln_v0.txt`) fails by name.
  - **Properties (SplitMix64, ≥ 10⁴ cases):** `exp` and `ln_ratio` monotone on their domains; `exp(0) = 2^32`; `ln_ratio(n, n) = 0`; `prob_from_hazard(0) = 0`, `= 2^32` for every `h ≥ 23 · 2^32`, non-decreasing; `exp(ln_ratio(n, d))` within 2 ulp of `n · 2^32 / d` on the Brass standard's 69 odds; each domain refused by name.
  - **Against the reference, beyond the committed vectors:** 2^14 `--emit` vectors in the PR tier and 2^20 nightly, bit-equal.
  - **Compile-fail controls:** the four fixture crates fail to compile for their one reason each, counted by `check_all`'s compile-fail gate; `fixed-ok` still compiles.
  - **Lints.** `lint_forbidden`'s float rule and clippy's transcendental ban are green on `rh-core`; V-PERF-04 and V-PERF-11 are unchanged (nothing runs on the per-person path).
- **Status:** [ ] not started

#### T-02: The F1 schedule at construction: MR1–MR9 (REV-SPR-016 R2, R6, R13; INV-M2-21; D-6; W22)
- **Deliverable:** `rh-sys-demography/src/schedule.rs` gains the F1 compile beside rung 0's:
  - MR1's 69 `lambda_i` as compiled constants; MR2's parameters (`E`, `B`, `K_cap`, `S_d`); MR3's odds grid, the rational form at `B = 65536`, else `exp(rhe(B · lambda_i / 2^16))`; MR4's survivors in both forms; MR5's bisection on the standard grid; MR6; MR7's thirteen infant knots from `infant_sub_schedule.toml`; MR8's tail (m1, m2, mu, a, b, the 361 knots, the forced monotonicity, the zero knot) with its three preconditions as `DemographyError::Construction { step: "MR8", … }` (W22); MR9's 438-knot table and `l(x)`.
  - **The reference's golden gains what the engine test needs** (D-6): `schedule`/`knot` lines at the four Registry corners and 32 pseudo-random `(e0, β)` points beyond the eight check-value points, and `ell` samples at pseudo-random minutes on both calendars, including ages under 1 and over 90; `selftest_m2_f1_mortality.py` writes them and `--check` pins them. No copy is made: `crates/rh-sys-demography/tests/schedule_f1_golden.rs` reads `specs/demography/golden/m2_f1_mortality.txt`.
- **Acceptance:**
  - **Bit-equality** with every `schedule`, `knot` and `ell` line, debug and release; the altered-line control (`tests/lint_fixtures/golden/stale_m2_f1_mortality.txt`'s pattern, one knot off) fails by name.
  - **Rung 0 is the β = 1 case:** at `B = 65536`, for the seven rung-0 check values and 64 pseudo-random e0, the F1 compile's `K`, `L_0..L_70` and `l(x)` at 10⁴ sampled minutes in [Y, 90Y] on both calendars equal the rung-0 path's and `m2_schedule_v1.txt`'s. A planted `mutant-beta-one-power` (the power form at β = 1) fails this by name (the Rust twin of the reference's control).
  - **INV-M2-21** at every compiled point: 438 knots strictly increasing in age, non-increasing in `l`, `l(90) = L_66`, `l(x) = 0` from the zero knot and at every `x ≥ 120Y`.
  - **Corners.** The Registry box compiles with no error at (20, 0.6), (20, 1.65), (85, 0.6), (85, 1.65); the zero knots are the spec's `l_zero_twelfths` (1314 at (20, 1.65), 1361 at (60, 1.65), 1381 at (85, 1.65), 1408 at (30, 1.3), 1440 elsewhere).
  - **Refusals by name:** `brass_alpha` set (exists, `compile`); each MR8 precondition through the test hook as `Construction`; an `exp` domain error during construction surfaces naming MR3 or MR8.
  - **Reported:** the time of one compile (the 69 `exp`/`ln` pairs and the 361 tail knots), and the F1 schedule's own trapezoid e0 against E at the eight points (the reference's check 8 figures: −0.06 years at e0 30, +0.5 at e0 85).
- **Status:** [ ] not started

#### T-03: The monthly probability, the modifier, the draws, the parameters, the fold and the q(15) null: MR10–MR12, MR2 (REV-SPR-016 R3, R4, R9, R11; D-3; W18, W19, W20, W21; INV-M2-20; V-DET-14; V-SF-02; V-SF-02b)
- **Deliverable:**
  - **MR10:** `q` from MR9's `l`; the newborn rule for `b > t_(m−1)` at `m ≥ 1`: `q = 2^32 − l(x + D)`.
  - **MR11:** `N` from M2's own `alive` column once per pulse before any draw; `m_d`; `q' = min(2^32, rhe(q · m_d / 2^16))`; the entitlement factor exactly 1 (`q'' = q'`), its form present and unit-tested through `f = exp(rhe(eps · ln_ratio(w, 65536) / 2^16))` on the clamp range, so that P1.5a turns it on without a new procedure.
  - **MR12:** S9–S11 on `q''`; the re-sample: `q_r` by S11's conditional for an ordinary person, and `2^32 − rhe(l(x) · l(x1) / l(x0))` for a person whose pending timer was drawn under the newborn rule; N from **`ResampleCtx::living_count()`** (D-3): the ledger's `alive = 1` rows at the start of the Inputs phase of `t_in`, before the input's own effects, counted once per input (MR12's words); nothing stored; the `05.1` §2 rule 2 text (the Architect's wording).
  - **`rh-manifest` (W20):** `brass_beta` accepted anywhere in the bounds (`DemographyError::BetaNotOne` removed); `DEMOGRAPHY_READS` becomes the six parameters of P1; `compile` refuses a non-integer `count` literal by name (`density_capacity: 1500.5` → "must be an integer count"); `manifest_v0.py` is unchanged (bounds only, as MR2 says).
  - **The fold (W19):** `bin/gen_m2_f1_draws_reference.py` (SPR-015's model) gains MR7, MR8, MR10, MR11 and MR12 and absorbs `gen_m2_draws_reference.py`'s six worlds re-chosen for F1: founders aged 0.3–0.9 years (the infant knots), 85–95 (the tail), a world at (20, 1.65) with founders at 109.4 years (the zero knot: the `last_day` c = 0 case), newborns through births, inputs that scope a newborn's pending first-draw timer, a world of 30 founders with `density_capacity: 10`, both calendars; the five re-sample cases (`last_day`, `duplicate`, `mid_month`, `own_instant`, `no_pending`) each covered and asserted; `draws_golden.rs` and its two mutants (`mutant-resample-hit-k1`, `mutant-resample-day-k0`) move into `f1_draws_golden.rs`; `gen_m2_draws_reference.py`, its golden and its `--power` gate are retired (A-53's larger re-sample world is carried into the fold). No test-only rung-0 draw path remains in plane A.
  - **The q(15) founders test (W18):** `m2_system.rs`'s null is re-derived from MR9's `l` on [0, 1) (a founder aged x faces `1 − l(15)/l(x)` with the infant knots), with the new expected mean and the new pooled-z figures stated in the test and in `docs/09` V-SF-02's method line.
- **Acceptance:**
  - **Bit-equality** with the folded draws golden (every death, birth and marriage instant, debug and release); the altered-line control fails by name; the five re-sample cases are each hit (asserted).
  - **Planted mutants, each failing the golden by name:** the two carried mutants; `mutant-newborn-s8` (rung 0's S8 for a newborn), `mutant-resample-recharge` (`2^32 − l(x1)`), `mutant-resample-conditional` (S11's conditional for a newborn), `mutant-density-symmetric` (`m_d = N / K_cap` below capacity).
  - **INV-M2-20** on random worlds under capacity: every `q''` equals MR10's `q` bit for bit (a test-only probe of the step's probabilities); on the over-capacity fixture `q' > q` exactly as `m_d` says, and `m_d` is capped at 4.
  - **`ResampleCtx::living_count()`** equals the ledger's living count at the Inputs phase start, before the input's effects, on random worlds; a keyframe round trip holds; thread invariance at 1, 2, 4 and N on the fixture worlds.
  - **V-SF-02 (W18):** the 10,000-founder test passes against its re-derived null at β = 1 with the pooled z and its power printed; the old null, kept as a planted control, fails by about z = −50 (the reviewer's figure: 0.34688 against 0.30646).
  - **V-SF-02b reported** (nightly, release): l(65)/l(15) of 10,000 founders aged 15 at β ∈ {0.64, 1, 1.64}, 32 seeds, pooled, against the schedule's own 0.481, 0.315, 0.139 at e0 30 within ± 0.03.
  - **`compile` (W20):** a fixture manifest with `density_capacity: 1500.5` is refused by `rh_manifest::compile` naming the parameter and passes `manifest_v0.py validate` and the Rust validator alike (bounds only); `density_capacity: 1500` compiles; `brass_beta: 1.3` compiles; `brass_alpha: 0.2` is refused by `compile` as today.
- **Status:** [ ] not started

#### T-04: The 0.3.1 bump (Architect, C2 inside the C3 PR; REV-SPR-016-FOLD R6; W23)
- **Deliverable:** `specs/systems/M2.demography.odd.toml` 0.3.1 with exactly the changes of the specs table's "0.3.1 content" row; the skeleton regenerated (`SPEC_VERSION = "0.3.1"`, nothing else); the companion §8 and `next/README.md` updated; the interim-rule code of SPR-015 T-03 removed.
- **Acceptance:** `lint_odd.py` passes; `gen_skeletons.py --check` passes and a diff of the skeleton shows only the version line; `selftest_m2_f1_mortality.py --check` and `selftest_m2_f1.py` pass against 0.3.1 (their check values are untouched); M2 registers against 0.3.1 under rule `spec`; the two stale-value controls still fail by name; `gen_manifest.py --check` passes.
- **Status:** [ ] not started

#### T-05: The two baselines, the divergence record, the gates and CI time (W42)
- **Deliverable:**
  - **The final commit,** subject ending `[C3]`: `household` and `hamlet` regenerated by `bin/regen_baseline.sh`; `registry_version` and the manifest pins unchanged (the Registry did not change).
  - **The divergence record,** in the PR body and in `VER-SPR-016`, per world: the bisect's first divergent month on a monthly ladder, the person it names and which of the three classes they fall in; the differing tables at every yearly rung.
  - **The gates:** part `core`: `exp_ln_golden` (debug), the 2^14 `--emit` comparison, the compile-fail fixture crates, `schedule_f1_golden` (debug) and its control, `f1_draws_golden` (debug) with its control and one carried mutant, the `compile` fixture, the `ResampleCtx` test, the re-derived q(15) null's unit test. Part `golden`: the two moved ladders, the unchanged `solo` and `smoke` ladders, the q(15) founders test in release as today. Nightly: `exp_ln_golden` and `schedule_f1_golden` in release, the other five mutants, the 2^20 sweep, V-SF-02b. The inventory test in the same commit.
  - The docs of the compliance table.
- **Acceptance:**
  - **Robust form.** `solo` and `smoke` are byte-identical at 1 and N threads, headers included. For `household` and `hamlet`, the first divergent month names a person under one year old at the pulse, a person born since the previous pulse, or a person over 90 − D; from that month on every table may differ. A first divergence naming anyone else, or `solo` moving, is a defect.
  - **Ladders.** The PR-tier ladders equal the new baselines at 1 and N threads; the nightly monthly ladders at 1, 2, 4 and N × 3 on the head; the parallelism floors hold or are restated.
  - **Channel A.** On `household` and `hamlet` every line before the first divergent month is byte-identical to SPR-015's segments.
  - **Performance.** V-PERF-04 and V-PERF-11 within budget (the F1 compile runs once at construction; the per-person path gains one multiply for `m_d` and one comparison for the newborn rule). If a recorded count moves, the tool re-baselines in a `[perf-baseline]` commit with a reason line.
  - **CI time (W42).** Against `e5082d0`'s core 7:58 and golden 10:21, after wave 4's additions, this sprint adds **about +0.7 minute to part core** (the debug goldens and one mutant) and **about +0.0 minute to part golden** (the two ladders are already run; the q(15) test is already there). **What leaves for the nightly tier:** from part golden, the release runs of `exp_ln_golden` and `schedule_f1_golden` (V-DET-09's opt-level-3 clause is kept nightly, as `rh-sample`'s stats tier is) and V-SF-02b; from part core, five of the six mutant rebuilds and the 2^20 sweep. Both parts at 12 minutes or less, reported.
- **Status:** [ ] not started

## Director items, and what changes under the alternative

| Item | Recommended | Under the alternative |
|---|---|---|
| **BCK-002 A-48: the smooth (C¹) logit interpolant from age 1** (REV-SPR-016 R6, R13; "decided at SPR-016's planning"). Recommendation: **defer; not built in SPR-016 (7/10)** | The F1 schedule keeps rung 0's linear-`l` interpolation between grid points (MR9 = S7's rule), with its monthly-q teeth (at e0 30, β 1: under 7% from 50 to 72.5, −11.5% to −36% from 75 to 87.5, −31% at the join at 90, −57% at age 2; the reference prints them). Why: (1) the β = 1 bit-equality with rung 0 is the proof this sprint rests on (check 1, the schedule golden); (2) A-48 has no spec text, no exact reference and no golden, so GOV-005 §2 forbids building it; (3) its effect is judged by SPR-028's harness on the age-at-death distribution, which does not exist yet; (4) the sprint is at 3.5 agent-days. A-48 becomes a P1b C3 sprint with its own spec round, after SPR-028; the spec's `not_yet` item 7 says so at the 0.3.1 bump (W23) | **Build it here:** a spec round first (a new `[algorithm.*]` table or an `[[amends]]` of S7 under a new `mortality_rung0` version, the reference's PCHIP-on-the-logit knots, a golden), then the code. Rung 0's equality at β = 1 is lost wherever the interpolant applies (ages 1 to 90), so `solo` moves too, the founders' q(15) null moves again, and the verification strategy becomes "equal to the new reference" alone. About +1.5 agent-days and a second review round |
| **D40 and D41** | Unchanged. Nothing in this sprint touches `tfr`, the marriage pattern or the V-SF-24/25 gating | — |

## Out of scope (explicit)
- **A-48** (above). **Sex-specific schedules**, **era-state shifts of α** (`medical_level`, P1b), **maternal mortality** (F2), **minute-level death times** (A-35): the spec's `not_yet`.
- **The M4 entitlement read** and `eps_f`, `eps_m` (P1.5a; REV-SPR-016-FOLD R2). The form of MR11's factor is present and unit-tested; it is never evaluated in P1a.
- **A settlement capacity** (M1 F0 / M3): `density_capacity` is the world's one capacity; `kingdom_10k.yaml`'s `density_capacity: 10000` is SPR-030's first C1 change (REV-SPR-030 R1a).
- **The demography bands as gates** (V-SF-01, V-SF-02b; SPR-028). V-SF-02 stays the gate it is, with its re-derived null.
- **Named `Q16_16` forms of `ln`** until a consumer needs them (R12 Q2).
- **The births slice** (SPR-015), which this sprint builds on; this sprint changes only its differential model (the fold).

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | T-01, T-02, T-03, T-05, written from `05.1` §6 item 4, MR0–MR13 and ADR-0025 item 3 before reading the code:<ul><li>their own implementation of E1–E6 and L1–L6 from the `05.1` text (the independent review did this in Python; the Tester repeats it against the Rust on 10⁵ inputs);</li><li>their own MR1–MR10 implementation against the Rust at the eight check values and 64 box points, and the β = 1 equality on both calendars;</li><li>their own S8–S12 + MR10–MR12 model (the SPR-012 Tester's model, extended) predicting every death instant of the fixture worlds, the five re-sample cases and the over-capacity world included;</li><li>the six mutants killed for their reasons, plus three of their own;</li><li>debug and release bit-equality; thread invariance; keyframe round trips;</li><li>the divergence re-measured and checked against the robust form (`solo` and `smoke` unchanged);</li><li>the nightly ladders and the 2^20 sweep on the head</li></ul> | `VER-SPR-016-TEST` |
| AGT-003-STAT | T-03's bands, written from `docs/09` V-SF-02 and V-SF-02b and the spec's facts:<ul><li>the re-derived q(15) null (W18) recomputed independently from MR9's `l`, its pooled z and power, the old null's failure reproduced;</li><li>V-SF-02b's l(65)/l(15) at the three β with intervals, INSUFFICIENT stated honestly;</li><li>the realised e0 of the F1 schedule against E at the design points;</li><li>no band adjusted</li></ul> | As above |
| AGT-003-IF (W24) | The C2 surfaces, written from `05.1` §2 rule 2's new text, MR2 and the 0.3.1 content row:<ul><li>`ResampleCtx::living_count()`: counted once per input, before the input's effects, equal across thread counts, absent from every keyframe;</li><li>`compile`'s refusals with fixtures of their own (a non-integer `count`, `brass_alpha`, β out of bounds) and `validate`'s bounds-only behaviour on the same files (V-WC-04 parity unchanged);</li><li>registration against 0.3.1 and the skeleton diff;</li><li>the 0.3.1 text against the content row: nothing algorithmic changed</li></ul> | As above |

## Merge route

- **Class and conditions.** C3, merged by the Architect after the Tester passes, the audit and the announcement of the two baseline changes to the director, who may veto (GOV-005 §6). The PR title and the final commit's subject end `[C3]`. The 32-seed CRN-paired ensemble is not built yet (SPR-030), so the divergence record and the Tester's model are the C3 evidence (W28). GOV-005 §6's sampler clause (the nightly tier green on the head before a C3 merge of `rh-sample` or its reference) does not apply by its letter (`rh-core::fixed`, not `rh-sample`), but its reason does: **the nightly 2^20 sweep and the monthly ladders run on the PR head before the merge** (the Architect's rule for this sprint, 8/10).
- **One C3 open for merge at a time (GOV-005 §4.1).** This sprint merges **after SPR-015** (PRJ-002 §3's merge-order edge): its PR is opened as a draft when T-01 lands and stays a draft until SPR-015 is on `main`; then `origin/main` is merged, the fold lands on SPR-015's model, T-03's newborn cases, T-04 and T-05 land, and the divergence is measured from SPR-015's baselines.
- **If SPR-015 slips past this sprint's readiness** (T-01, T-02 and the non-newborn part of T-03 done): the PR waits. It does not merge first: it would promote 0.3.0 itself, carry the interim rule it exists to remove, have no newborn to test MR10 on, and have no F1 model to fold the draws golden into.
- **Shared files.** With SPR-015: `rh-sys-demography`, the M2 spec, the draws model, `check_all.sh`, the inventory test, `CLAUDE.md`. With SPR-027: `check_all.sh`, the inventory test, `CLAUDE.md` only. Whichever merges second resolves the textual conflicts.

## Blockers

| # | Blocker | Status |
|---|---|---|
| 1 | SPR-015 not merged (T-03's newborn cases and the fold, T-04, T-05) | Open |
| 2 | The Architect's rulings of the specs table | **Closed:** ruled 2026-10-17 (W18–W28, D-3, D-6); the sweep, the baseline prediction, the V-SF-02b tier and the over-capacity world rows stay as proposed |
| 3 | The director's ruling on A-48 (recommended: defer) | Open (D54) |
| 4 | The director's plan approval (D54; GOV-005 §2) | Open |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS (DET, STAT, IF).
- [ ] Architect audit `VER-SPR-016` is PASS, with the divergence record of T-05 and the nightly sweep on the head.
- [ ] No open DEF- against this sprint.
- [ ] The merge route matches the CI-detected change class (C3); the two baseline changes were announced to the director first; `solo` and `smoke` are unchanged.
- [ ] M2 0.3.1 is frozen on `main` with no interim rule left; `05.1` §2 rule 2 states `living_count()` and §6 item 4 names the vectors' source; the rung-0 draws golden is folded; BCK-002 A-48 carries the director's ruling.
