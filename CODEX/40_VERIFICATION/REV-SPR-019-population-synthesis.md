---
id: REV-SPR-019
title: "Independent review of the population synthesis and dynastic back-fill spec (PR #62)"
type: record
status: OPEN
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-019, SPR-015, SPR-016, SPR-018, SPR-021, SPR-030, REV-SPR-017, BCK-002]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF: round 1 on `87707ca`. Changes are needed before the fold: no blocker, 4 majors, 10 minors and 2 nits.**
> - **What holds.** The algorithm text is exact: the reviewer's independent re-implementation, written from the TOML alone, reproduces every golden line for P2–P7 and for the back-fill's houses and tenures. D-02's id scheme is sound.
> - **The majors:**
>   - the tick-0 channel-A records for dead ancestors cannot carry their dates, and would count as month-0 deaths;
>   - P5's "exact rationals" need about 13,000 bits;
>   - the back-fill's tenures disagree with PR #41's evaluator in many successions;
>   - frozen M7 is amended outside the lint-checked `next/` path.
> - **Rulings:** R1–R8 in §0.
> - **Reviewer:** a session on a different model from the drafter's.

# REV-SPR-019: independent review of population synthesis and the dynastic back-fill

## 0. Architect rulings, round 1 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R1 | MAJ-1; open question 1 | **Accepted: keyframe and life-course only.** Back-filled dead ancestors get no channel-A records; the chronicle reads them from the genesis keyframe and the life-course log.<ul><li>The R3 amendment is withdrawn.</li><li>State the discriminator: `exit_tick < entry_tick` marks a back-filled row.</li><li>If SPR-029's annals need ancestors in the stream, a record kind carrying `birth_tick` and `exit_tick`, outside `STEP_DIGEST`, comes then, in a format version</li></ul> | 8/10 |
| R2 | MAJ-2 | **Accepted.** State E(a) in fixed point: the S chain in Q0.64 with an `rhe` per month, or N1's Prob32 per band. Choose one and say why. Restate the widths, and regenerate the goldens with the tool | 8/10 |
| R3 | MAJ-3; open question 2 | **The back-fill's tenures must be what the succession law selects.** For v0, before D37:<ul><li>**Genealogical laws** (primogeniture, ultimogeniture, seniority, partition, tanistry): fix the reductions to PR #41's semantics. Primogeniture and ultimogeniture are male-preference cognatic, partition considers `children(holder)`, and tanistry requires `majority_age`, so no child holder.</li><li>**Elective and sortition,** whose candidates come from `court(title)`, which genesis does not back-fill: write only the genesis holder's tenure row and invent no past holders. Title blood then sees one holder. `aethelgard.yaml` uses `elective`, so this is a fallback, not a refusal.</li><li>D-04's text states the cost honestly.</li><li>When PR #41 merges, a differential gate runs the reference against `effect_ref.py` at every pre-epoch succession of the genealogical laws, and must agree on every one. Until then, the spec says so.</li></ul> | 7/10 |
| R4 | MAJ-4; NIT-2 | **Accepted.** Frozen text changes only through `next/` versions with lint-checked `[[amends]]` records and the lock:<ul><li>the M2 amendments go into `specs/systems/next/M2.demography.odd.toml`;</li><li>M7's go into a new `specs/systems/next/M7.dynasty.odd.toml` draft (0.4.0), which carries the dropped clauses (the journaled world-creation event; the first exit draw at the first Month pulse), the `tenure.title` and `house_member.house` meanings, and H2;</li><li>every `to` is a whole replacement text.</li></ul>`specs/worldgen/population_synthesis.toml` keeps the algorithm only. Coordinate with PR #59 (the mortality fold, merging) and PR #61 (the envelope's `roles` lines), which touch the same next M2: merge `main` after each lands | 7/10 |
| R5 | MIN-1 to MIN-7, MIN-9; NIT-1 | All accepted:<ul><li>the reference loads the TOML and asserts its constants and `check_values`, with a stale-value control;</li><li>the rounding controls plant P3 and P7 separately, matching each invariant's message;</li><li>INV-BF-03's window is stated at the conception pulse, and the law sweep runs about 20 seeds;</li><li>P10's sibling gap is in ticks, at least M2's minimum birth-to-birth gap;</li><li>D-11 also applies in P10, and the person digest pins `parity`, `last_birth_tick` and the `house_member` rows;</li><li>the order of houses, of g and of the B8 queue is stated, the ultimogeniture wording fixed, and all seven laws' tenure lines pinned;</li><li>`majority_age` is read from the compiled parameters, and the manor gets its M7 parameters;</li><li>A × 2 cells, and `synth_offset` keyed by people index and slot;</li><li>P7's row wording</li></ul> | 8/10 |
| R6 | MIN-8 | **One director item, D46:** P1.2's "IPF from manifest marginals" is deferred in v0 to the population marginal alone, with age and sex from the schedule's stationary population; class, culture and faith marginals wait for their owning systems (P1b). Draft it in the PR body, recommending approval (75%). The golden-corpus announcement (`hamlet`'s goldens move) belongs to SPR-019's sprint PR, not this spec | 8/10 |
| R7 | MIN-10 | **A finding for the director and for SPR-021.** At e₀ = 30 and tfr 4.9, a three-generation house has a median of 7 living members and 2 adult males, and 23 of 80 houses have at most one living adult male at genesis. The Architect adds this to D40 (the tfr default), which the director has not yet ruled on. SPR-021 tests collateral succession on a dedicated fixture, not on the manor | 8/10 |
| R8 | Open question 3 | `manor.yaml` stays in `specs/manifest/examples/` as a non-rung fixture, with its M7 parameters (R5) | 8/10 |

## 1. The review (round 1, independent reviewer)

Reviewer: independent and adversarial, on a different model from the drafter's. Copy reviewed: `$R/src`, a git archive of `87707ca`. `$R` = the Architect's scratchpad `popsynth_review/`; its `work/` scripts reproduce each finding.

Reviewer: an independent model, adversarial. The copy is `src/` beside this file. The scripts are in `work/`.

## BLUF

**Changes requested. Nothing blocks, and four findings are major.**

The algorithm text is exact enough to re-implement. My re-implementation from the TOML text alone (`work/indep.py`) reproduces the golden lines:
- P2–P7 on all four cases: the age–sex digest, the married marginals and the couple table;
- B0–B9 on both manor cases: the house, title and tenure lines.

All gates are green, and every new gate is in the core part. D-02's identity scheme is sound.

The four major findings:
1. **D-03's channel-A records cannot be written as the DDL stands.** `LIFE_EXIT.tick` is the row's exit tick, and no record may come before tick 0. The month-0 `STEP_DIGEST` would also count 17 to 19 phantom deaths.
2. **P5 is not implementable in the stated `u128` widths.** Its exact rationals reach 13,000 bits.
3. **D-04 understates its divergence from PR #41's laws.** I ran PR #41's evaluator on the back-filled genealogies to measure it.
4. **The amendment of frozen M7 0.3.2 bypasses the lint-checked `next/` path.** As written, it is also lossy.

## Findings

| ID | Where | Problem | Evidence | Fix (confidence) |
|---|---|---|---|---|
| MAJ-1 | `population_synthesis.toml:191-192`, `:208` (D-03); `channel_a_v0.sql:29,34,170,186` | D-03 writes a LIFE_EXIT record at tick 0 for each dead ancestor, but the DDL has no field that can carry its pre-epoch death. `life_course_exit.tick` is "= the life_course row's exit_tick", the table has no other exit-tick column, and R3 and R5 refuse any record before tick 0. The record is therefore either back-dated (it breaks R3 and R5, and ADR-0025 in spirit) or misdated to tick 0. `step_digest.deaths` counts "LIFE_EXIT records with exit_kind death since the previous digest", so the tick-0 digest reports the ancestors as deaths of month 0. That breaks 05.1 §9 population conservation and V-DATA-04's monthly reconciliation. D-02(c) avoided exactly this pollution for births. | Manor seed 1 has 33 back-filled rows, of which 14 are alive: 19 phantom deaths at tick 0. Seed 2 has 17. The R3 amendment changes one sentence and leaves the `tick` column comment and the digest definition unchanged. | Write no channel-A records for the back-filled dead. The chronicle reads them from the genesis keyframe or the life-course log. If they must be in the stream, add a record kind in a format bump that carries `birth_tick` and `exit_tick` and sits outside `STEP_DIGEST`. (8/10) |
| MAJ-2 | `population_synthesis.toml:72` (u128 claim), `:82` (P5) | E(a) is "an exact rational of N1's Prob32 bits", built on the F3 chain of up to 419 monthly products. Its denominator is 2^(32n). The header promises a `(u128, u128)` pair, and companion §7.2 says "rational arithmetic in u128". The Rust port cannot follow this text without bignums, and an approximation can move M_f, water-filling (WF) and the P3 cell boundaries, which φ resolves at 2^-64. | `work/probe4.py`: E(30) has a 5,923-bit denominator, E(49) 12,829 bits, ΣW_a 13,018 bits, and the WF targets 13,025 bits. | State E(a) in a fixed point: the S chain in Q0.64 with an `rhe` per month, or N1's Prob32 per band. Restate the widths and regenerate the goldens. (8/10) |
| MAJ-3 | `:111` (B9), `:213` (D-04) | D-04 says the only cost is that a past holder is "never a woman". The divergence is larger. PR #41's elective and sortition laws draw candidates from `court(title)`, not from the house. Its tanistry requires `majority_age`. Its partition considers only `children(holder)`. Its primogeniture and ultimogeniture are male-preference cognatic. D-04(a) also says the programs have "no Python twin", but `specs/registry/effect_ref.py` is that twin. | `work/compare_d04.py` evaluates PR #41's programs at each pre-epoch holder death, on the back-filled log and tenure rows. Over seeds 3–40: primogeniture 3 of 96 transitions differ, ultimogeniture 3 of 96, partition 6 of 96, tanistry 21 of 100 (every one a child aged 0–15 seated where PR #41's line is empty), and seniority 0 of 100. At seeds 1–2, elective and sortition differ in 11 of 11: the sovereign seat goes to House Thorne's holder, and the major title's line is empty. | Restate D-04's cost honestly. Fix tanistry's child holders. Prefer open point 2: evaluate the programs once D37 lands, with the reference calling `effect_ref.py`. Otherwise, refuse non-lineal laws for dynastic manifests in v0. (6/10) |
| MAJ-4 | `:185-186`, `:25`, `:198` (D-01) | Frozen M7 0.3.2 is amended through untyped `of` and `to` records that nothing lints. The established path is `specs/systems/next/<id>.odd.toml` with `[[amends]]` records, the frozen-text check and the lock, and M7 has no draft there yet. The M7 `to` text silently drops "as the journaled world-creation event" and "the first exit draws at the first Month pulse", and it embeds review prose. The amendments also miss frozen meanings that B4, B9 and B11 contradict: `tenure.title` ("rh-worldgen writes each title's initial holder's row") and `house_member.house` ("the manifest's for founders"). | A diff of the frozen `[initialization]` text against the amendment shows the drops. `lint_odd.py` does not read `specs/worldgen`. | Draft `next/M7.dynasty.odd.toml` (0.3.3) with `[[amends]]` records and the lock now. Keep the fold file for M2 only, the mortality precedent. (7/10) |
| MIN-1 | `selftest_population_synthesis.py:131-155` | The reference never reads `population_synthesis.toml`, so the TOML's constants and `check_values` can drift with the gate green. The mortality precedent reads its own `check_values`. | Setting `youngest_years = 25` and `manor_1_married = 79` in the TOML still gives `POPULATION SYNTHESIS: PASS`. | Load the TOML with `tomllib` and assert its constants and `check_values`. Add a stale-value control. (8/10) |
| MIN-2 | `bin/check_all.sh:968`; reference `:1101` | The rounding control fails at an IPF construction assertion ("the two marginals differ"), not at INV-PS-01, -02 or -03, and the gate matches only the generic FAIL line. | With `Checker.marginals()` disabled (`work/mut/`), the control still "fails as required", and the normal run passes. | Plant P3 and P7 separately, and match the invariant's own message. (8/10) |
| MIN-3 | `:156` (INV-BF-03) | B6 allows conception up to age 49y11m, so a birth can come at about age 50.7 (M2's own invariant allows this). INV-BF-03 requires age 15 to 49 at the birth, and the reference fails its own invariant beyond the golden seeds. | `work/robust.py` and `probe5.py`: at manor seed 25 the mother is 50.47 at the birth and 49.71 at conception. A 365-day calendar case with G = 6 fails at seed 15. | State the window at the conception pulse, as M2 does. Run the law sweep over about 20 seeds. (8/10) |
| MIN-4 | `:87` (P10) | "At least two years of whole-year age" allows a sibling gap of one year plus a minute. That is less than M2's closed period (G + P = 20 pulses), and P10 cites 05.1 §9 for a minimum spacing that §9 does not contain. | Manor seed 1 (golden) has a synthesised sibling gap of 19.6 months (`work/probe2.py`). | Require the gap in ticks to be at least M2's minimum birth-to-birth gap. (7/10) |
| MIN-5 | `:113`, `:248` (D-11); P4 and P10 | D-11 sets `last_birth_tick` for back-filled mothers only. Synthesised mothers with infants keep `i64::MIN` and so can conceive at month 1, which is the very case D-11 exists to prevent. The golden pins neither `parity` nor `last_birth_tick`, nor the `house_member` rows. | Manor seeds 1 and 2: 11 synthesised mothers each have a child under 1. No back-filled mother does. | Apply D-11 in P10 and add these columns to the person digest. (7/10) |
| MIN-6 | `:106-111` (B0, B4, B6, B9) | Two points of the order are left unstated. The k order across spine couples is not given (B4's anchor loop runs g = G−1 down to 0), nor the order across houses. In B9, "every law when no adult is alive → ascending pre-order" contradicts the reference's descending pre-order for ultimogeniture. Only primogeniture tenures are in the golden. | `work/ulti.py`: the two readings give different tenure logs at 14 of 60 seeds. | State the order of houses, of g = 0..G−1 and of the B8 queue. Fix the ultimogeniture wording. Pin all seven laws' tenure lines in the golden. (8/10) |
| MIN-7 | `:111` (B9); `manor.yaml:44` | B9 hardcodes `adult_years = 16` instead of M7's `majority_age`, which a manifest may set anywhere in [12, 21]. `M7.dynasty: {}` with no Registry default becomes a load error once M7 is built (Registry ruling 8). | `registry_v0.toml:100` gives M7 bounds and no defaults. | Read `majority_age` from the compiled parameters, and give the manor its M7 parameters, or depend on SPR-021's defaults. (7/10) |
| MIN-8 | `:253` (director items) | "No director item" is not accurate. SPR-019 sits on PRJ-002's golden-corpus announcement list, where the director can veto, and hamlet's goldens move. D-05 also defers P1.2's "IPF from manifest marginals": `class_shares` (manor.yaml declares 5% elite) is ignored. | `PRJ-002:208`; `docs/11:179`. | Add a director note covering the hamlet golden announcement and the deferral of the marginals, with no decision asked. (7/10) |
| MIN-9 | `:82` P5 and P9 vs `:51` and `:89`; companion `:64` | Open point 8 says the mortality fold needs "no text change", but P4 hardcodes "200 cells, a = 0..99" and `age_max = 100`. Separately, `synth_offset` is keyed by slot only, so the "later human people" of P1 would reuse the same offsets. | Text. | Write A × 2 cells, and key `synth_offset` by people index and slot. (7/10) |
| MIN-10 | Effect on SPR-021 | Houses barely grow: the NRR is 1.03 at e0 = 30 and TFR 4.9. Over 40 seeds a G = 3 house has a median of 7 living members and 2 adult males, and 23 of 80 houses have at most one living adult male at genesis. That fits M2, but not 05.1 §7's "about 2.5× per generation" or the effect language's D12 sizing. The elite multiplier never reaches the male line (open point 4). | `work/probe3.py`. | Raise this with the M2 and M7 owners. SPR-021 should not use the manor for collateral-succession tests. (6/10) |
| NIT-1 | `:84` (P7) | "From the first row with d_i > 0" differs from the reference, which skips a row with no path. The two readings differ only in pass 2, which none of 216 sweeps reached. | `work/flow_sweep.py`. | Say "each row with a deficit, ascending". (9/10) |
| NIT-2 | `:174` and `:192` (`to` texts) | The `to` texts are fragments, not whole replacement texts. H2's "every other founder gets a one-person household" clause, and its hamlet example, are not amended. | Text. | Write whole texts and add H2. (8/10) |

## Sound

- **Reference and goldens.** The reference passes. The spacing control fails on the right text, the golden control reports `STALE`, and `--write` is never needed.
- **Independent re-implementation.** It matches every P2–P7 line for hamlet seeds 42 and 7 and manor seeds 1 and 2, and every B0–B9 house and tenure line for manor seeds 1 and 2.
- **Keys.** Every draw is a fixed-structure W0 key. The keys are distinct across purposes and entities, with the one exception in MIN-9.
- **No HashMap dependence and no float in the reference.** Integer widths for P2, P3 and P6 hold.
- **D-02's identity scheme.** The `founder` and `child` domains differ, and `PersonIdTable` holds every id issued, the dead included, with the rehash. Nothing that reads `origin` breaks: V-DATA-04, INV-M2-06 and -07, the kinship walks (`mother`, `father`, `birth_tick`, `sex`), and M7's `house_member` rule.
- **D-03's life-course part.** It fits ruling 5 as clarified and DEF-3 ("appended closed is free"), and `entry_tick = 0` is the column's meaning. The span `[0, exit)` is empty, which matches the Architect's V-DATA-04 and S3 R7 exemption. State the discriminator explicitly: exit_tick < entry_tick.
- **Blood-rule precondition.** The root is the first holder, so every house member descends from a holder. There are 1 to 5 tenure rows, in chronological order.
- **The manor fixture** validates, and V-WC-04 parity picks it up through the glob. It AGREEs with an `rh-cli` built from this commit.
- **Other gates.** `lint_odd` passes in both modes, as do `gen_manifest --check`, ruff, mypy and pytest (621 tests).
- **Gate placement.** All new gates are in `part core` (`check_all.sh` lines 895 and 965–968, before `part golden` at line 981).

## The drafter's open questions

1. **Channel A for the dead:** keyframe and life-course only (MAJ-1). 8/10.
2. **D-04:** use the evaluator once D37 lands. Until then, use lineal laws only, or fix tanistry, elective and sortition (MAJ-3). 6/10.
3. **The manor's place:** keep it in `examples/` as a non-rung fixture. The parity coverage costs nothing. Add its M7 parameters. 7/10.

## Not run

- The full `bin/check_all.sh`: cargo nextest, clippy, the golden part and valgrind. The PR has no Rust changes.
- Rust `rh-worldgen`: it is not yet written for SPR-019.
- PR #41's evaluator on manifests other than the manor.
- The `selftest_channel_a` case that D-03 promises: it does not exist yet.
