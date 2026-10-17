---
id: REV-SPR-016
title: "Independent review of the fixed-point exp/ln ruling and M2 F1 mortality (PR #51)"
type: record
status: OPEN
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-016, SPR-015, REV-SPR-015, EVO-001, BCK-002]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF:** Round 1 on PR #51, at `d69691d` plus the delta to `9f8d3f9`: **approve after changes.** No blockers. Four majors before the fold, and one more in the delta. The `exp`/`ln` core is sound: an independent re-implementation written from the `05.1` text alone matched the reference bit for bit on 83k inputs, and the full 3.6M-input `--exhaustive` run gave only faithful results. At β = 1 the schedule equals rung 0 bit for bit (48k random points, ages 1 to 90). The Architect's rulings are in §0. The review ran in a session on a different model from the drafter's.

# REV-SPR-016: independent review of exp/ln and M2 F1 mortality

## 0. Architect rulings, round 1 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R1 | MAJ-1 | Accepted. The frozen `death_time` `draws` text is kept verbatim. Any new draw is stated in new text, or through the births spec's amendment record (REV-SPR-015 R1). | 9/10 |
| R2 | MAJ-2 | Accepted. Certain death must not be claimed where rounding reaches it first. State the exact rule (q = 2^32 if and only if l(x + D) = 0), give the age at which survivorship first rounds to 0 as a function of (K, β) with check values, and keep 120 only as the hard cap. | 8/10 |
| R3 | MAJ-3 | Fixed by M10 in `9f8d3f9`. The newborn's first Month-pulse draw covers [0, x + D). Accepted at the drafter's 7/10: one draw per person per month, no new purpose, founders unchanged. | 7/10 |
| R4 | D-MAJ-1 | Accepted. M12 states the newborn rule inside a re-sample as one formula, with a check value and a reference case for each of the three readings the reviewer found, so only one survives. | 8/10 |
| R5 | MIN-1, MIN-2, MIN-3 | Accepted. Correct the error-budget sum, the survivor-error figure, the `exp` argument range and D-SPR016-1's month count. State that `--exhaustive` covers the Q16_16 grid and that Q32_32 inputs are sampled. Answer to the drafter's Q1: the sweep joins `bin/golden_nightly.sh`. | 8/10 |
| R6 | MIN-4 | Accepted, and treated as more than wording: a 31 % drop in monthly q at 90 is a visible artefact (a dip in deaths at 90). Join the tail so that monthly q is continuous across 90 within a stated tolerance (≤ 2 %), with a reference check. | 8/10 |
| R7 | MIN-5 | Accepted. Each control isolates exactly one check, and deleting that check must make the control pass. | 9/10 |
| R8 | MIN-6 | `05.1`: whichever of PR #49 and PR #51 merges second takes v1.11. The new `[[reads]]` entry carries EVO-001's typed `neutral` once EVO-001 is on `main` (a `rate` read). | 9/10 |
| R9 | MIN-7, MIN-8, the nits, D-MIN-2 | Accepted, with the reviewer's fixes. D-MIN-2: the reference handles every valid calendar, the 365-day calendar included. | 9/10 |
| R10 | D-MIN-1 | Accepted. Rename the steps with a prefix that clashes with neither system ids (M1–M9) nor D-items, for example `MR0`–`MR13`. | 9/10 |
| R11 | Drafter's A1–A11 | A1 to A7, A9 and A11 accepted. A8, the density modifier, is accepted as PRJ-002's SPR-016 row names it (two Registry parameters; the capacity is above every P1a world). A9's column name `household.food_entitlement_ratio` is provisional until M4 has a spec (P1b). A10 is accepted: `brass_alpha` stays refused in P1a. D-SPR016-1 to 4 are design values, ruled here; none is a director item. | 7/10 |
| R12 | Drafter's Q2 to Q5 | Q2: no Q16_16 named forms until a consumer needs them. Q3: as A8. Q4: Bourgeois-Pichat (1951) and Thatcher, Kannisto and Vaupel (1998) join the R12 batch-3 list; both tables stay replaceable constants. Q5: follow EVO-001's `neutral` when it lands (R8). | 8/10 |

## 1. The review (round 1, independent reviewer)

Reviewer: an independent review for the Architect, 2026-10-17. Scope: `132e17d..d69691d` (4 commits), then the delta
`d69691d..9f8d3f9` in its own section. Copies: `src/` = `d69691d`, `src2/` = `9f8d3f9`, `births/` = `origin/arch/M2-F1-births`
(`421f2e1`). I did not modify the repository. My scripts are in `scripts/`.

## BLUF

**Approve after changes. Nothing blocks; four MAJ findings need fixing before the fold.**

- **The exp/ln core is sound.** I re-implemented E1–E6 and L1–L6 from the `05.1` text alone. It matches the reference
  bit for bit on 45k `exp` and 38k `ln` inputs. Every result is faithful, including the full `--exhaustive` sweep (3.6M
  inputs).
- **β = 1 reproduces rung 0 bit for bit:** K, the grid, and l(x) and q at 48k random (x, D) on [1, 90 − D] years,
  checked directly against `selftest_m2_rung0.py`.

What fails:
- the fold of `death_time`'s `draws` text, which the births next-mode lint rejects (reproduced);
- the claimed certain death at 120, which is false over about half of the Registry box;
- in `d69691d`, a newborn's unhazarded first part-month. `9f8d3f9` fixes this but adds an ambiguous re-sample rule.

The error-bound proof does not add up: its terms sum to 133 and 164 units of 2^-65, against the claimed 128. The
headline bounds (faithful, ≤ ¾ ulp) still hold.

---

## Findings on `d69691d`

### MAJ-1: folding the `death_time` `draws` change fails the births next-mode lint
- **Where:** `specs/demography/mortality_f1.toml:41-46` (`[[rng_changes]]`); `9f8d3f9`: `:48-53`.
- **Problem.** The fold rewrites the frozen `death_time` `draws` lines: k = 0 now reads "on the month's modified Prob32
  probability q'...", and k = 1 reads "unchanged (F12 = S9...)". The births branch's `lint_odd.py --next` requires
  frozen purposes to keep the same `key`, `resample_on_fork` and **`draws`** (`births/specs/systems/lint_odd.py:201`;
  `specs/systems/next/README.md` table row `[[rng]]`).
- **Evidence.** I applied the `9f8d3f9` `draws` to `births/specs/systems/next/M2.demography.odd.toml`
  (`scripts/simulate_fold.py`) and ran the lint: `ERROR ... frozen RNG purpose death_time changed draws`, `FAIL`. The
  unmodified births tree passes.
- **Fix.** Keep the frozen `draws` text verbatim. It is already generic: "the month's Prob32 probability". Put q'/q''
  in `[algorithm.mortality_f1]` M12 only. Confidence 9/10.

### MAJ-2: certain death at 120, and "q = 2^32 iff x + D ≥ 120 Y", are false for β above about 1.2
- **Where:** `mortality_f1.toml:157` (F8 "T_360 = 0: death is certain in the month a person reaches 120"), `:159`
  (F10 "q = 2^32 iff x + D >= 120 Y"), `:144` and `:121` (D-SPR016-3 "a hard cap at 120"). The same text is at `9f8d3f9`
  `:163`, `:177`, `:179` and `:140`.
- **Problem.** The tail knots are `rhe(L_66 S_j / 2^32)`. When l(90) is small and the tail steep, T_j rounds to 0
  months or years before 120, so l(x) = 0 and q = 2^32 earlier. The real maximum age is a rounding artefact that the
  text never states. Check 4 tests only l(120 Y) = 0, never l(x) > 0 below 120.
- **Evidence** (`scripts/mort_checks.py`): survivorship reaches 0 at 109.5 years (e0 20, β 1.65), 110.5 (30, 1.65),
  113.4 (60, 1.65; a design point and a check-value row), 115.1 (85, 1.65) and 119.8 (30, 1.2). At e0 30, β 1 it holds,
  with T_359 = 15 bits. The behavioural effect is negligible (l < 2^-33), but an implementer who encodes the "iff" as an
  assert will trip it.
- **Fix.** State "death is certain from the first knot where T_j = 0, at the latest at 120". Drop the "iff" or make it
  "if". Change D-SPR016-3 to match. Optionally report the zero age per corner in the reference. Confidence 8/10.

### MAJ-3: a newborn carries no hazard from birth to its first Month pulse (the seam with births)
- **Where:** F10/F12 (`mortality_f1.toml:159-161`) against births F9 (a row is inserted at a day start inside a month).
- **Problem.** S8's q covers [x, x + D) with x = t_m − b > 0, so [0, x) is never hazarded. With F7's front-loaded
  infant table (q = 0.0903 in month 1 at e0 30), a newborn misses about half of that month. That is roughly 4.5 points of
  q0 = 0.283, so realised q(0) is about 0.25 and V-SF-02's q(15) shifts. No file owns this gap.
- **Status.** **Resolved in `9f8d3f9`** by M10's newborn rule and `[seam]`. See D-MAJ-1 for what the fix introduces.
  Confidence 8/10.

### MIN-1: the error-budget arithmetic does not reach 2^-58; the "exhaustive" tier covers only a 2^-16 lattice and never runs
- **Where:** `05.1` §6 item 4, `docs/05_architecture/05.1_simulation_core.md:245`; `specs/numerics/exp_ln_v0.py:37-38`.
- **Problem and evidence.**
  - **The stated terms do not sum to the claim.**
    - For `exp`: 32·2^-64 + 34·2^-65 + 32·2^-65 + 2^-64 + 2^-65 = 133·2^-65 = 2^-57.95.
    - For `ln`: 33·2^-64 + 32·2^-65 + 63·2^-65 + 2^-64 + 2^-65 = 164·2^-65 = 2^-57.64.
    - The claim is 2^-58 = 128·2^-65 in both cases.
  - **So "ln within ½ ulp + 2^-26 ulp" is not proven.** The stated terms give ½ + 2^-25.6 ulp.
  - **The headline bounds survive.**
    - `exp` excess at y = 16: 0.138 ulp, so ≤ ¾ ulp holds.
    - Measured worst internal error: 2^-59.97 relative (`exp`, exhaustive) and 2^-60.18 absolute (`ln`).
    - Worst `exp` error: 0.5146 ulp.
    - Unfaithful results: none.
  - **`--exhaustive` is not exhaustive for the inputs M2 uses.**
    - It covers Q16_16-granular y and ln(n / 2^16), while M3 and M8 call `exp` and `ln_ratio` on arbitrary Q32_32 values
      and operands.
    - It checks 2,555,905 `exp` inputs, not the 2,097,153 the text states (that count is [−16, 16]).
    - It is not wired into `.github/workflows/nightly.yml` or `bin/golden_nightly.sh`, so the nightly tier never runs.
- **Fix.**
  - State the bound as "within 2^-57.5", with ln ≤ ½ + 2^-25 ulp. The ¾ ulp bound for exp is unchanged.
  - Correct the input count.
  - Wire `--exhaustive` into the nightly job, and add a random-Q32_32 sample to it.
  - Confidence 9/10.

### MIN-2: the `05.1` "why this is enough" analysis leaves out exp's quantisation of small w_i; the M3 argument range is wrong
- **Where:** `05.1:246` ("a grid survivor L_i moves by at most 0.6 of one 2^-32 step"); `mortality_f1.toml:152` (F3 "exp's
  argument is in [-1.1, 15.2]").
- **Evidence.**
  - I measured |L_i − 2^32 / (1 + K O_i^β)| at the compiled k, with a 60-digit oracle. The worst case is **1.458 bits**
    (e0 20, β 1.65, i = 3), against the 0.5 + 0.6 claimed. The analysis covers the error in y. It misses `exp`'s
    absolute ¾-ulp error on W_1 ≈ 0.057, which is about 13·2^-32 relative.
  - The argument range of M3 is [−2.861, 15.197]: λ_1 = −1.734 at β = 1.65. That is within the domain.
  - The material effect is about 1 Prob32 step.
- **Fix.** Restate the bound as ≤ 1.5 bits. Add the w-quantisation term. Correct the range to [−2.9, 15.2].
  Confidence 8/10.

### MIN-3: D-SPR016-1's evidence for the director is wrong in one place, and the alternative is mis-costed
- **Where:** `mortality_f1.toml:110` and F11 `:160`; `9f8d3f9` `:129` and `:180`.
- **Evidence and problems.**
  - **"q < 0.02 ... excludes only the first two months of life".** At e0 30, β 1, q > 0.02 for the **first six
    months** (ages 0–0.42 years). It is also intermittently above 0.02 from 84.4 years, on rung 0's linear-l sawtooth,
    and continuously from 87.9.
  - **At m = 4, the >1% error region** is ages 0 to 1.9 and every age from 69.4.
  - **"A proportional-hazards form needs exp and ln per person-month."** The same proportional-hazards form applied
    to the 438 knots once per month (l^m, recomputed only when m_d changes) costs 438 exp/ln pairs per month, whatever
    N is.
  - **The default capacity is 10M,** so the term is inert in every P1a world. Listing V-SF-01 in `earns_its_place_by`
    therefore has no support until a manifest sets a capacity.
- **Fix.** Correct the numbers. Add the knot-level alternative. Say plainly that the modifier is inert in P1a.
  Confidence 8/10.

### MIN-4: D-SPR016-3's "continuous in level and in hazard at 90" is false for the realised monthly q
- **Where:** `mortality_f1.toml:122`; `9f8d3f9:141`.
- **Evidence.** At e0 30, β 1 the monthly q for the three months before 90 is 0.0364, 0.0377 and 0.0392. At 90 it is
  0.0271, then 0.0272: a 31% drop. The cause is rung 0's linear interpolation of l, which inflates the hazard at the end
  of each 2.5-year interval. The tail's own first-month hazard matches mu within 0.1–0.5% at every corner.
- **Fix.** Say "continuous in level; the hazard matches Brass's mid-interval hazard at 90, and the sawtooth of linear l
  below 90 is rung 0's". Confidence 9/10.

### MIN-5: the `shares-unnormalised` control does not prove the continuity check works
- **Where:** `bin/check_all.sh:847`; `selftest_m2_f1_mortality.py:500-504`.
- **Evidence.** I deleted the continuity clause (`knot_l[12] == grid[1]`) in a copy and re-ran
  `--control shares-unnormalised`. Check 5 still fails, through `shares[-1] == Q16`, and check 1 fails as well. The gate
  label says "must FAIL continuity at age 1".
- **Fix.** Plant a table that keeps c_12 = 65536 but breaks continuity some other way. Alternatively, have the control
  bypass the precondition and match a distinct message. Confidence 8/10.

### MIN-6: the `05.1` version collides with EVO-001, and the read lacks EVO-001's typed `neutral`
- **Where:** `05.1:12`; `mortality_f1.toml:53-58`; `9f8d3f9:62-67`.
- **Evidence.**
  - EVO-001 claims the same version: `origin/arch/EVO-001-lifecycle-queues:CODEX/60_EVOLUTION/EVO-001_LifecycleQueuesP1a.md:70`
    reads "v1.10 (2026-10-17): EVO-001 (SPR-014)", and `:146` reads "amends 05.1 to v1.10 on acceptance".
  - EVO-001's `lint_odd` requires a machine-readable `neutral`, typed by the column, on every foreign read. It is an
    error on a frozen spec.
    - At `d69691d` the neutral is a string, `"1.0 (q16_16 65536)..."`.
    - At `9f8d3f9` it is only in `why`.
  - The changelog's claim that "PRJ-002 expected [exp/ln] as v1.9" has no match in PRJ-002.
- **Fix.**
  - The Architect assigns v1.10 and v1.11 by merge order: whichever merges second renumbers.
  - Add `neutral = 1.0`.
  - Drop the PRJ-002 remark.
- Confidence 9/10.

### MIN-7: the density term in a re-sample has no stated phase for N
- **Where:** F12, `mortality_f1.toml:161` ("F11's m taken from the living count at the input's instant"); `9f8d3f9`
  M12 `:181`.
- **Problem.** N is defined only at the Month pulse. At t_in the text does not say which point in the instant counts:
  the Inputs phase, before or after the input's own effects, and once per input or once per person. The reference
  computes no re-sample q at all.
- **Fix.** Use the N of the pending month's pulse (store nothing, recount nothing), or state the exact point. Add a
  re-sample q vector. Confidence 7/10.

### MIN-8: the fold does not amend `[design]`, which the density term contradicts
- **Where:** `births/specs/systems/next/M2.demography.odd.toml:456`.
- **Problem.** That line reads `scale_behaviour = "Identical at every N: every per-person draw depends only on that
  person's state ..., not on N"`. M11 makes q depend on N, and the fold changes neither `scale_behaviour` nor
  `interaction`.
- **Fix.** Add a `[design]` change to the fold. Confidence 8/10.

### NIT
- **NIT-1.** Check 8's `E0_TOL_HIGH` (1 year for e0 > 60) never runs, because no design point is above 60. The toml's
  "+0.5 at e0 = 85" goes unasserted; I measured +0.524.
- **NIT-2.** "A person who lives past 90 draws against a different q" (`:146`) should say "aged over 90 − D at the
  pulse".
- **NIT-3.** Both references write the committed golden file when run without `--check`. That includes a `--control`
  run that unexpectedly passed.
- **NIT-4.** The docstrings of `exp_ln_v0.py` check 2 (it says a finite Σ C_j, while the code uses the exact infinite
  tail) and check 3 (a 2^-24 window, while the code's window adds the budget) differ from the code.
- **NIT-5.** The names `widen_q32` and `narrow_q16` differ from `rh_core::fixed`'s existing `widen()` and
  `narrow_saturating()`.
- **NIT-6.** F11's widths are unstated: S_d (N − K_cap) has no named integer type.

---

## The delta `d69691d..9f8d3f9`: newborn rule (M10), `[seam]`, step rename M0–M13, INV-M2-13/14

**Resolved:**
- MAJ-3 (the newborn gap).
- The `d69691d` seam clashes with the births next file:
  - the fold targeted the frozen file;
  - INV-M2-06/07 collided with births' genealogy invariants;
  - step prefix F collided with births' F0–F10;
  - `not_yet` was written against the frozen list.
- `[seam]` answers who owns what. Mortality owns the newborn's first draw and infant death. Maternal death is not in F1;
  when it arrives it becomes a new mortality purpose, which leaves the CRN rule untouched. `apply_death` stays the single
  path: S12, then H4. The reference, golden file, controls, pytest inventory, ruff and mypy all pass at `9f8d3f9`.

**M10 against the brief:**
- **CRN.** The rule changes only the value of q. A newborn gets exactly one k = 0 draw at each Month pulse it is alive
  at, and none in its birth month (it is not alive at that pulse). The frozen rule ("one per living person per month")
  holds.
- **β = 1 for founders.** Founders have b ≤ 0 ≤ t_(m−1) for every m ≥ 1, so the newborn rule never applies to them.
  `d69691d`'s rung-0 equality on [1, 90] years is unchanged.
- **Integer exactness.** q = rhe(2^32 (2^32 − l(x + D)) / 2^32) = 2^32 − l(x + D) exactly, because l(0) = I_0 = 2^32.
  The result is in [0, 2^32] and is at least the S8 value.

### D-MAJ-1: "the newborn rule" in a re-sample is undefined, and its obvious reading double-counts
- **Where:** M12, `src2/specs/demography/mortality_f1.toml:181` ("q'' computed from M10 on x0 = t_in − b, x1 = t_end − b
  (and the newborn rule when b > t_(m−1))").
- **Problem.** For a newborn whose first-draw timer is pending, three readings give different q':
  - q' = 2^32 − l(x1). This re-charges [x, x0), which the child has already survived.
  - S11's conditional (l(x0) − l(x1)) / l(x0). This drops the deferred [0, x) hazard.
  - 1 − l(x) l(x1) / l(x0), the reading consistent with deferral.
- **Evidence** (`scripts/delta_checks.py`; born 20 days before the pulse, input 25 days into the month):
  0.1130 / 0.0064 / about 0.066. The reference implements no re-sample q, so no vector pins it.
- **Fix.** State the formula, recommending the deferral-consistent one, and add `qresample` vectors. Confidence 7/10.

### D-MIN-1: step names M0–M13 collide with system ids M1.regions to M8.war
- **Where:** `src2/specs/demography/mortality_f1.toml:4`, `:121`; `src2/docs/05_architecture/05.1_simulation_core.md:239`.
- **Evidence.**
  - `[seam]` says "a newborn is on M7's infant knots", where M7 is the step, while the births file says "M7 on_birth",
    where M7.dynasty is the system.
  - M11 reads "M4's household entitlement", the system, while M3 says "M4 uses its rational form", the step.
  - `05.1` now cites "`mortality_f1` M4" in a document where M4 means the economy throughout.
- **Fix.** Use a prefix no system or sibling table uses, such as `D0–D13` or `MT0–MT13`. Confidence 8/10.

### D-MIN-2: the reference asserts a newborn is under one month old, which the spec does not require and the 365-day calendar breaks
- **Where:** `src2/specs/demography/selftest_m2_f1_mortality.py:356` (`assert 0 <= x < month`).
- **Evidence.** A child born at Jan 2 00:00 has its first pulse on Feb 1: x = 30 days and D = 28 days. M10 gives
  q = 520511466. The reference raises `AssertionError`.
- **Fix.** Assert 0 < x ≤ D_(m−1) − 1440, using the previous month's length. Confidence 9/10.

### D-NIT
- **D-NIT-1.** Rename leftovers in the comments: "M7-F9" and "M2-F9" (`selftest_m2_f1_mortality.py`, section header
  and `compile_schedule` docstring).
- **D-NIT-2.** M11's entitlement text has no lower clamp. With w = 0, `ln_ratio(0, ·)` hits `LnDomain` at run time, while
  `beyond_range` says the ratio is clamped at 0.3.
- **D-NIT-3.** `05.1` v1.10 still says exp/ln are evaluated "at construction, on a fixed grid". M11 runs them once per
  household per month from P1.5a.
- **Still open at `9f8d3f9`:** MAJ-1, MAJ-2, MIN-1 to MIN-8, and NIT-1 to NIT-6 (NIT-2 now reads "aged over 90 − D").

---

## What I checked and found sound
- **exp/ln are integer-only,** with no float and no platform dependence. Python `>>` is floor, the same as an i128
  arithmetic shift. Every width is below 2^128 (L3: n 2^(64−e) < d 2^65).
- **The constants match a 90-digit oracle** and the pinned values. The convergence induction is correct.
- **The domains are refused by name:** `ExpDomain`, `LnDomain`, `HazardDomain`. `exp` returns 0 below −23.
- **Rounding** is half to even, consistent with SPR-010's `shr_round_half_even`, which already works on i128.
- **E2's n is exact** on the whole domain: r stayed in [0, ln 2) at every boundary probe.
- **`exp` and `ln_ratio` are monotone** at the ranges used: the step between adjacent true values far exceeds twice the
  error.
- **The infant knots:** I_0 = 2^32 and I_12 = L_1. The table matches Bourgeois-Pichat at 60 digits. It is flagged [U]
  and raised as a director item.
- **The tail anchor:** l(90) = L_66 exactly. The convexity condition behind the m2 > m1 proof holds for the standard
  (Δλ 0.8035 < 0.8513). Over an 11 × 10 sweep of the box, mu ≤ 0.5461 and b ≤ 0.1677, so 30b ≤ 16 holds.
- **The density term:** m_d is bounded in [1, 4], is exactly 1 at or below capacity, and gives q' = q there.
- **Citations:** R12 §9 (p. 59, 86, 95) and R09 §2 (infant mortality a quarter to a third, d(1 + x), the entitlement
  form) are present.
- **Gates:**
  - The six new gates are all in `--part core`, and `test_check_all_inventory.py` passes.
  - ruff and mypy are clean at both commits.
  - Both controls and both stale fixtures fail with exit codes 2 and 1.
- **Governance.**
  - Nothing amends `docs/03`, an ADR or the roadmap.
  - The Registry additions (density_capacity, density_mortality_slope) are raised as D-SPR016-1, not made. The
    remaining D-SPR016 items carry a recommendation and a confidence.
  - The `05.1` text and the spec are the Architect's to approve.

## What I did not run
- `bin/check_all.sh` and any cargo or Rust build. No Rust exists for SPR-016.
- The births branch's own `selftest_m2_f1.py`.
- A merged births + mortality fold beyond the `draws` simulation.
- Ages and inputs outside the samples named above. In particular, `exp` and `ln` were not run exhaustively over Q32_32,
  which is infeasible in Python.
