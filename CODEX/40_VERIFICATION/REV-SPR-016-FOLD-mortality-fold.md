---
id: REV-SPR-016-FOLD
title: "Independent review of the fold of M2 F1 mortality into the next M2 (PR #59)"
type: record
status: OPEN
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-016, SPR-015, REV-SPR-016, REV-SPR-015, SPR-020, SPR-019]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF: round 1 on `6c33f10` approves after changes, with no blocker, 3 majors, 6 minors and 5 nits.**
> - **The fold itself is faithful.** MR0–MR13, the table version and the 8 check values are byte-identical to `mortality_f1.toml` as merged (PR #51). The golden vectors are bit-identical except for the header line, the lock regenerates byte for byte, and the RNG reuse is sound.
> - **The majors are about the surrounding text:**
>   - P1 contradicts S2 and MR2;
>   - the M4 read's neutral cannot be typed once the spec is frozen;
>   - the freeze point of 0.3.0, which now carries mortality that SPR-015 does not build.
> - **Rulings:** R1–R6 in §0.
> - **Reviewer:** a session on a different model from the drafter's.

# REV-SPR-016-FOLD: independent review of the mortality fold

## 0. Architect rulings, round 1 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R1 | MAJ-1 | **Accepted.** Rewrite P1, the normative home of S2:<ul><li>name the six parameters with their types (`density_capacity` a u64 count);</li><li>F1 accepts any β within the bounds;</li><li>α stays refused at F1 (DV-SPR016-2);</li><li>drop "only in its last sentence"</li></ul> | 9/10 |
| R2 | MAJ-2 | **Option (a): defer the M4 read.** Defer the `household.food_entitlement_ratio` read, its in-coupling and its isolation forcing to M4's first spec (P1.5a). MR11 keeps the entitlement form, stated as a factor of exactly 1 until the read exists. `next/README.md` records the follow-up. This replaces the drafter's decision 4 | 8/10 |
| R3 | MAJ-3 | **Both.**<ul><li>0.3.0 is promoted (frozen) in SPR-016's PR, which merges after SPR-015's: the header and `next/README.md` say so.</li><li>P1 states the interim rule SPR-015 builds against: until SPR-016's code lands, F1 runs `mortality_rung0` for everyone, newborns included, and `rh-manifest` refuses β ≠ 1.</li><li>INV-M2-20 and 21 apply from SPR-016.</li><li>Restore births' MIN-7 `not_yet` entry as part of the interim rule</li></ul> | 7/10 |
| R4 | MIN-1 to MIN-6 | Accepted:<ul><li>MR8 and MR10 point at the companion's sections, not at removed tables;</li><li>correct the 27–36 % claim to the reference's figures (under 7 % at 55–72.5, −57 % at age 2);</li><li>amend the frozen `e0` meaning through an `[[amends]]` record;</li><li>declare `density_capacity` as an integer count, and fix the skeleton generator's mapping if it cannot express one;</li><li>restore the fertility density term in `not_yet`;</li><li>§8.2 of the companion marks round-2-refuted claims as refuted</li></ul> | 8/10 |
| R5 | NIT-1 to NIT-5 | Accepted | 9/10 |
| R6 | The drafter's decisions and questions | **Decisions:** 1 (retire `mortality_f1.toml`), 2 (the reference reads the next spec), 3 (no new purpose), 5 and 6 accepted; 4 is superseded by R2.<br>**Questions:**<ul><li>(a) the meaning text is enough;</li><li>(b) leave the dead text for the promotion PR, but add both items to `next/README.md`'s promotion checklist</li></ul> | 8/10 |

## 1. The review (round 1, independent reviewer)

Reviewer: independent and adversarial, on a different model from the drafter's. Copy reviewed: `$R/src`, a git archive of `6c33f10`. `$R` = the Architect's scratchpad `fold_review/`.

Reviewer: independent (adversarial), 2026-10-17. Paths below are in the PR's tree. `next M2` is
`specs/systems/next/M2.demography.odd.toml`.

## BLUF

**Approve after changes. Nothing blocks; three MAJ findings need fixing before 0.3.0 can freeze.**

The fold itself is faithful:
- MR0 to MR13, `version` and all 8 `check_values` are byte-identical to the base's `mortality_f1.toml`.
- The golden body is bit-identical; only line 1 changed.
- The lock regenerates byte for byte, and the self-check line is exact.
- The RNG is sound.

The defects are in the text around the table:
- **The parameter table contradicts the new mortality text.** Births' `[algorithm.parameters_f1]` P1 is the normative home of S2. It was not updated, so it now contradicts S2, MR2 and DV-SPR016-2.
- **The M4 read fails the lint once the spec freezes.** Its neutral cannot be typed. That is a warning while the spec is a draft and an ERROR once it is frozen, which I reproduced by simulating the promotion.
- **The spec still freezes too early.** It still freezes inside SPR-015's PR, now with mortality that SPR-015 does not implement. The fold also removed births' "until SPR-016" interim wording.

## Findings

### MAJ-1: P1 (the normative home of S2) contradicts S2, MR2 and `brass_alpha`
- **Where.** next M2 `:624` (`[algorithm.parameters_f1]` P1).
- **Problem.** P1 still says:
  - "F1 reads e0 and brass_beta as rung 0 (S2 is amended only in its last sentence)";
  - "rh_manifest::compile fills each of the four";
  - "brass_alpha is refused at load by name as at rung 0 (SPR-016 lifts it)".

  The amended S2 (`:542`), MR2 (`:695`) and the `brass_alpha` meaning (`:749`) now say otherwise:
  - F1 accepts any `brass_beta` in the bounds;
  - F1 compiles six parameters, adding `density_capacity` and `density_mortality_slope`;
  - `brass_alpha` stays refused at F1 (DV-SPR016-2, REV-SPR-016 R11 A10).
- **Evidence.**
  - `docs/05_architecture/systems/M2.demography_F1.md` §6, row R1: "S2 amended with `[algorithm.parameters_f1]` as its normative home".
  - The diff touches the S2 amendment and MR2, but not P1.
- **Fix.**
  - Rewrite P1. It should name the six parameters with their types (`density_capacity` as a u64 count), accept any β in the bounds at F1, and keep α refused at F1 under DV-SPR016-2.
  - Drop "only in its last sentence".
- **Confidence.** 9/10.

### MAJ-2: the M4 read's neutral cannot be typed; the frozen-mode lint errs at promotion
- **Where.** next M2 `:388-397` (`[[reads]]`), `:952-963` (coupling), `[isolation]`.
- **Problem.**
  - 05.1 §2 rule 2 types a rate read's neutral by the owner's `[[owns]]` or by the Registry `[era_state]`. M4 has neither.
  - `lint_odd.py:836` turns that diagnostic into an ERROR when `status = "frozen"`.
  - The new comment mentions only the "no spec yet" warning.
  - The column name is provisional. If M4's spec names it differently, 05.1 §2 refuses the world: "a read of a column its registered owner does not write stays refused". Frozen M2 would then need a C2 bump just to rename the read.
- **Evidence.**
  - `--next` mode prints two new WARNs for this read (12 warnings become 14).
  - I simulated step 4 of `next/README.md`: copied the next file over the frozen one with `status = "frozen"` and merged the delta's params and defaults. The plain lint then printed `ERROR M2.demography.odd.toml: neutral of household.food_entitlement_ratio cannot be typed`.
- **Fix (the Architect's choice).**
  - (a) Recommended: defer the read, its coupling and its forcing to M4's first spec (P1.5a). MR11's entitlement text stays. The term is inert in P1a, and R8's typed neutral cannot be met without M4.
  - (b) Alternatively, declare the column's type somewhere the lint reads it (an M4 stub, or a Registry entry). Record the follow-up in `next/README.md`, as for S6's `household.head`.
- **Confidence.** 8/10.

### MAJ-3: 0.3.0 still freezes in SPR-015's PR, but it now contains mortality that SPR-015 does not implement
- **Where.** next M2 `:20-22` (the lifecycle sentence, unchanged).
- **Problem.**
  - `M2.demography_F1.md` §8.1 says "SPR-015 runs rung 0's S8 for newborns until SPR-016 merges".
  - The fold removed births' interim wording: "F1 accepts exactly 1 until SPR-016" in `brass_beta`'s meaning, and the MIN-7 `not_yet` entry.
  - Once SPR-015's PR freezes 0.3.0, SPR-015's code breaks the frozen text:
    - INV-M2-20 (q'' equals MR10's q) fails for newborns;
    - `brass_beta` "any value in the Registry bounds" is false, because `rh-manifest` refuses β ≠ 1.
- **Fix.** Either freeze 0.3.0 in SPR-016's PR (header and `next/README.md`), or state the interim rule in P1: until SPR-016's code lands, F1 runs `mortality_rung0` and refuses β ≠ 1.
- **Confidence.** 7/10.

### MIN-1: MR8 and MR10 point at tables that no longer exist
- **Where.** next M2 `:701` (MR8: "see [join_at_90] below") and `:703` (MR10: "the seam with births, [seam]").
- **Problem.** Neither table exists in this file. Both moved to `M2.demography_F1.md` §8.2 and §8.1.
- **Evidence.** A search of the next M2 finds neither `[join_at_90]` nor `[seam]`.
- **Fix.** Point both at the companion doc. The table is new text, and no reference reads the step text.
- **Confidence.** 9/10.

### MIN-2: "27 to 36 percent at every grid boundary above 50" is false
- **Where.** next M2 `:67` (`not_yet`) and `:688-689` (table header). MR8 at `:701` carries the same claim from #51.
- **Problem.** The size of the monthly-q steps is misstated, and the childhood steps are left out.
- **Evidence.** The reference at e0 30, β 1 gives these steps in monthly q:

  | Age | Step |
  |---|---|
  | 50 | +6.9 % |
  | 55 to 72.5 | under 7 % |
  | 75 | −11.5 % |
  | 77.5 | −21.9 % |
  | 80 | −19.7 % |
  | 82.5 to 87.5 | −27 % to −36 % |
  | 2 | −57 % |
  | 3 | −43 % |

  The childhood steps are why BCK-002 A-48 starts at age 1, and A-48 states the numbers correctly.
- **Fix.** Use A-48's figures.
- **Confidence.** 9/10.

### MIN-3: the frozen meaning of `e0` is stale at F1 and was not amended
- **Where.** next M2 `:744` (`params.e0.meaning`).
- **Problem.** The meaning says e0 is "converted to the Brass level K = exp(2 alpha) at beta = 1 by [algorithm.mortality_rung0] S5". MR5 converts at any β. `brass_beta`'s meaning was amended for the same reason; this one was not.
- **Fix.** Add a 19th `[[amends]]`. The lock and the self-check line then read 19.
- **Confidence.** 6/10.

### MIN-4: `density_capacity` is a u64 count, but the tools make it Q16_16
- **Where.** next M2 `:776-780` and MR2 `:695`.
- **Problem.**
  - `lint_odd.py:1031` (`--rust`, which `gen_skeletons` uses) types every `[[params]]` entry as `Fixed`, which is `Q16_16` with a maximum of 32767.
  - MR2's "as SPR-011 T-04 does" (`to_q16_16`) cannot compile the default either.
  - The manifest validator checks bounds only, so a literal such as 1500.5 passes. MR2 states no rule for it.
- **Evidence.** `gen_skeletons.py --out` on the simulated promotion emits `pub density_capacity: Fixed`, which cannot hold the default 10,000,000.
- **Fix.**
  - Add a parameter type to `[[params]]`, honoured by the generator and by `compile`.
  - Have MR2 state that a non-integer literal is a load error naming the parameter.
- **Confidence.** 8/10.

### MIN-5: the fertility side of births' density and entitlement seam was dropped
- **Where.** next M2 `:62` (`not_yet`) and `:763` (`eps_f` meaning).
- **Problem.**
  - Births' entry "Density and entitlement modifiers of fertility and mortality" is replaced by one that drops the fertility density term, R09 §2's b·(1 − x).
  - Fertility's entitlement factor on M exists only in prose (`not_yet`, the reads comment, the coupling's `via`). No F-step states it.
  - `eps_f`'s meaning still says "the hook is stubbed in SPR-016".
- **Fix.**
  - Keep a `not_yet` line for fertility's density term.
  - Say that `eps_f`'s hook is the shared read and that its step comes in P1.5a.
- **Confidence.** 6/10.

### MIN-6: §8.2 calls the drafter's whole recommendation "accepted", including claims that were refuted
- **Where.** `docs/05_architecture/systems/M2.demography_F1.md:284-290`.
- **Problem.**
  - "Recommendation, accepted" keeps the claim that ages 1 to 50 "have one-year intervals, where the teeth are small". Round 2 refuted it: −57 % at age 2.
  - It also keeps the logit-linear remedy, which R13 rejected.
  - A missing full stop runs the A-48 sentence into the previous one ("…above that The Architect's backlog item…").
- **Fix.**
  - Mark only the mid-interval anchor as accepted.
  - Strike the childhood claim and the logit-linear remedy, or label them as superseded by R13.
- **Confidence.** 8/10.

### NITs
- **NIT-1 (INV-M2-20).** The parenthetical at `:877` ("rungs 0-2 draw as rung 0 does, newborns' first month aside") ignores ages under 1 (MR7) and over 90 − D (MR8). It was carried from #51.
- **NIT-2 (stale descriptions of S2).**
  - The table comment at `:680` ("untouched here") and the header's amendment list at `:17` do not reflect that this PR changes S2's amendment text.
  - The PR body's "mortality_rung0 table is byte-identical" is false for S2. The change is a legal amendment, still under `m2-rung0-v1.1`; acceptable, because nothing has been built against v1.1.
- **NIT-3 (no control for a stale mortality check value).** Nothing checks a stale mortality check value in the next spec, unlike births' `m2_f1_stale_control`.
  - I mutated K in a copy, and check 3 FAILs, so the reference does read the next spec.
  - Add the control.
- **NIT-4 (the reference's spec paths).**
  - In `selftest_m2_f1_mortality.py:71-73`, `MAIN_SPEC` duplicates `SPEC_M2`.
  - If `next/` is emptied before promotion, the fallback raises a `KeyError` traceback instead of a FAIL.
- **NIT-5 (errors in the PR body).**
  - It says "one Q16_16 factor on M (MR11)", but MR11's density term multiplies q, not M.
  - It says "births emits the lifecycle request", but §8.1 gives maternal death to mortality's own purpose, applied through `apply_death`.
  - It says "medical_level … declared in the seam", but it is in `not_yet` and `era_state`.

### The drafter's open questions
- **(a) "Shared with eps_f" in `registry_delta.toml`.** Do not add it.
  - The delta carries bounds and defaults only, and `eps_f` has no default.
  - The sharing is already stated in the `[[reads]]` comment and the coupling's `via`.
  - Correct the PR body instead (NIT-5).
  - Confidence: 8/10.
- **(b) The CLAUDE.md line and the reference's fallback at promotion.**
  - The fallback is not dead text. It becomes the live path, the same convention as the births reference.
  - CLAUDE.md's "(specs/systems/next while it is a draft)" and births' "next-version spec" will both go stale.
  - Add "update both CLAUDE.md lines" to step 4 of `next/README.md`.
  - Confidence: 8/10.
- **PR #61.** Verified against the current head `08a8db0` of `origin/arch/SPR-020-envelope`, two commits past `f7a0225`.
  - `git merge-tree --write-tree` is clean (tree `f5b22f6`) and keeps all four `roles` lines.
  - On the merged tree:
    - `lint_odd --next` gives the same self-check line;
    - plain `lint_odd` passes;
    - both references pass.

## Sound (checked)

**Fold fidelity**
- MR0 to MR13, `version` and the 8 `check_values` are byte-identical to the base (compared with `tomllib`).
- The parameter and invariant texts are the same, apart from added `[algorithm.mortality_f1]` pointers.
- The read, coupling and forcing are the same, with these exceptions:
  - `output_bounds` is now `[0.0, 1.0]`, because the lint requires a list;
  - `beyond_range` lost the exact bits, which MR11 keeps;
  - the "no MR4" typo is fixed.
- The bounds moved to `registry_delta.toml`, and the narrative moved verbatim to §8.

**The mortality reference**
- `--check` passes.
- Each of the three controls FAILs exactly one check (1, 5 and 7), and the stale-golden control FAILs.
- The golden body and the stale fixture's body are bit-identical to the base; only line 1 differs.

**The lint and the amendments**
- `lint_odd` passes in both modes.
- The self-check line is exact: "29 … (3 … lock), 18 amendments recorded and locked".
- `--write-amendments-lock` regenerates the lock byte for byte, and I recomputed all 18 sha256 digests independently.
- Each `frozen` text matches the frozen file.
- The four re-stated amendments keep what births needs: `tfr` and `elite_fertility_mult` compiled, ε unread, α refused.

**RNG**
- `death_time` is unchanged at position 0.
- MR10 changes only the threshold of the single k = 0 pulse draw. That keeps CRN, and adds no draw at the birth.
- MR12 re-draws through `ResampleCtx`, exactly as S11 does. The key holds `resample_entity_id`, which is h64 of the kind and the raw id; t_in; and the epoch after the input.
- No two draws share a full key:
  - pulse draws use the raw id at t_m, and t_m differs from month to month;
  - re-samples use the hashed id at t_in, with an epoch that grows with each input;
  - no death timer is pending at a pulse instant (INV-M2-03).

**The seams with births**
- MIN-7 (newborn exposure) is closed by MR10 and MR12.
- `apply_death` is the single death path: MR13 and the Day/Vital schedule step.
- `medical_level` is deferred to P1b in `not_yet` and `era_state`.

**Retiring `mortality_f1.toml`**
- No live reference remains. Only history mentions it, in `next/README.md` and §8.
- The `check_all.sh` wiring is unchanged.

**The other gates**
- These pass:
  - `gen_manifest --check`;
  - ruff and mypy;
  - pytest (619);
  - the smoke-world lint;
  - the births and rung-0 references.
- The next controls (algorithm, rng, bound, lock, `m2_f1_stale`) all FAIL as they must.

## Not run
- **`bin/check_all.sh`, cargo and the nightly sweeps.** `check_all` would need a cold cargo build in the copy, so I ran the relevant gates one by one instead.
- **REV-SPR-015.** It is not a file in this tree. I used the companion doc's §6 and §6b as the record of its rulings.
