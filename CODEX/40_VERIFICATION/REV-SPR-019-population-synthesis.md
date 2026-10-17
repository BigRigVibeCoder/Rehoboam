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
version: 0.2.0
---

> **BLUF: round 2 on `5466fc6`. Changes are needed before the merge: no blocker, 1 major, 5 minors and 3 nits.**
> - **What holds.** R1, R2, R4, R6 and R7 are applied. The law lines agree with PR #41's `effect_ref.py` in all 4,307 successions the reviewer checked (up to G = 6, `majority_age` 12, 16 and 21, both calendars). The lock regenerates byte for byte, and every gate passes.
> - **The major.** When a law's line is empty, B9's stand-in seats a child: in every tanistry fallback observed, and the PR did not say so. The Architect accepts minor stand-ins, stated, measured and pinned (R9).
> - **The minors.** The sibling gap is below M2's minimum; three next-version texts are wrong; the seven-law pins discriminate little; `next/M7` breaks when PR #61 lands; a genesis mother's `last_birth_tick` has three rules.
> - **Rulings:** R1–R8 (round 1) and R9–R15 (round 2) in §0. Round 3 is verified by the Architect.
> - **Reviewer:** a session on a different model from the drafter's, in both rounds.

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

## 0b. Architect rulings, round 2 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R9 | MAJ-1; NIT-2 | **Accepted: minor stand-ins, stated, measured and pinned** (the reviewer's option (a)). The stand-in replaces M7's arbitration, which needs claims and a court that genesis does not write. Option (b), the liege's reversion, is not total for a polity with one dynasty, and it changes the title structure the back-fill exists to give history to. Required:<ul><li>B9 and D-04 (b) state that the stand-in is a minor whenever the house has no living adult male. Under tanistry, whose line is empty only when no adult agnate within reach is alive, that is nearly every fallback. Measured: 6 of 6 in seeds 1–20, 42 of 42 in seeds 21–60, and 26 of 29 at G = 6 on the 365-day calendar.</li><li>The other laws fall back too: partition when the holder has no living child, and primogeniture and ultimogeniture at G = 6. The texts stop saying "tanistry is the case that occurs" and "never for the other four laws".</li><li>The reference pins the fallback count by law, the count of minor stand-ins, and one case where the adult branch fires, so the mutant "oldest living male regardless of age" fails a gate.</li><li>No regency is written before tick 0. A minor stand-in still seated at tick 0 is D-09's case (SPR-021).</li></ul> | 7/10 |
| R10 | MIN-1 | **Accepted, as one constant: 609 days** (876,960 minutes). That is at least M2's least birth-to-birth gap on either calendar: 602 days on the 360-day calendar and 609 on the 365-day one, `t_(m+30) + 1 day − (t_(m+10) − 1 day)`. Correct the sentence: a synthesised pair is never closer than a born pair because the gap is *at least* M2's least gap. The reference computes M2's least gap for both calendars from the next M2 text, asserts that the constant is at least that, and asserts that every synthesised and back-filled sibling pair in the golden cases is at least that far apart. Regenerate the golden file with its tool | 8/10 |
| R11 | MIN-2 | **Accepted as proposed:**<ul><li>the `initialization` text names the stand-in;</li><li>"and their holders" goes;</li><li>"regent = 0; no step names one until SPR-021";</li><li>`holders_of(title, 16)` in the `tenure.title` text.</li></ul>The lock is regenerated by the tool | 8/10 |
| R12 | MIN-3 | **Accepted.** The golden file pins one digest per law over its 20 tenure logs, including each line's head. Add a G = 6 case whose logs reach 13 rows, so that a window of 8 changes a golden line. The reviewer's mutants must fail a gate: window 8, the stand-in picking the youngest, and the stand-in ignoring age. The PR body names the gate each one fails | 8/10 |
| R13 | MIN-4 | **Accepted.**<ul><li>PR #61 merges first, now. Merge `main` after it lands; that also resolves the `check_all.sh` conflict.</li><li>Re-derive `next/M7` 0.4.0 from frozen M7 0.3.3, which carries the `roles` and `claim.cause_event` as `event_ref`, and re-apply the three amendments. Regenerate the lock with the tool.</li><li>`next/README.md` gains the rule that, of a frozen spec's bump and its next version, whichever lands second re-derives the next version from the new frozen file.</li><li>PR #41 (D37) takes M7 0.3.4 and, if this PR is on `main` first, re-derives `next/M7`.</li></ul> | 8/10 |
| R14 | MIN-5 | **One rule for every genesis mother**, H2 included: `last_birth_tick` is the birth tick of her latest genesis child, and `i64::MIN` when she has none. The meaning text already says so; correct `[initialization]` and P10 to match. A mother with a newborn at genesis does not conceive at once. No engine baseline moves, because the column is new in the next M2 and SPR-015 builds it. The spec's golden file regenerates with its tool if it moves | 7/10 |
| R15 | NIT-1, NIT-3 | NIT-1: keep P3 unsigned, using ceil(x − φ) = ceil(x + 1 − φ) − 1 in `u128`, and state a bound the callers meet. NIT-3: the companion's open items say that SPR-021 gives M7's 17 parameters Registry defaults, or the compile refuses a manifest without them, and that `val_khor.yaml` (dynasties, no `majority_age`) is the case to check. No manifest change here | 8/10 |

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

## 2. The review (round 2, independent reviewer)

Copy reviewed: a git archive of `5466fc6` in the Architect's scratchpad `popsynth_review_r2/`; its `work/` scripts reproduce each finding.

Reviewer: an independent model, adversarial. Head reviewed: **`5466fc6`** (`origin/arch/SPR-019-popsynth`). The copy is `src/` beside this file, and the scripts are in `work/`. All paths below are relative to `src/` unless they start with `work/`.

### BLUF

**Not ready to merge. Nothing blocks, but one major and five minor findings remain.**

R1, R2, R4, R6 and R7 are applied faithfully. R3, R5 and R8 are applied only in part:
- **R3.** The restated law programs agree with PR #41's `effect_ref.py` on every line. I checked 4,307 successions across four configurations, up to G = 6, with `majority_age` of 12, 16 and 21 and both calendars. However, the stand-in rule puts a child on the seat under tanistry in every fallback observed (6 of 6 in the drafter's own sweep). R3 said "no child holder", and the PR does not disclose this.
- **R5.** The sibling gap is below M2's minimum birth-to-birth gap, so that ruling is not met, and the golden file contains such a pair.

Every gate passes. The lock regenerates byte for byte. Once PR #61 lands, `next/M7` 0.4.0 fails the lint with 11 errors. I checked that the port is mechanical: the lint passes and the lock is unchanged.

### R1–R8 status

| # | Status | Evidence |
|---|---|---|
| R1 | **Done** | B11 and D-03: no channel-A record; `exit_tick < entry_tick` is stated, and INV-BF-02 checks it. The withdrawn R3 amendment is gone. |
| R2 | **Done** | The P5 chain is Q0.64, and E(a) is Q0.32. My own computation from the TOML text alone reproduces E(19) = 2684395077 and E(49) = 4282935761. Check 4 matches E(a) against the births reference's exact fractions at every age. My restatement of P3 reproduces the age–sex, women and men allocations on all four golden cases (`work/indep_p3.py`). The widest integer is 99 bits at a population of 20,000 (`work/probe4.py`). Water-filling and `Fraction` are gone. Hamlet's married count is now 1. NIT-1 covers the width argument. |
| R3 | **Partial** | The line matches `effect_ref.py` in 263 of 263 cases (the drafter's gate) and in 4,307 of 4,307 (`work/compare_d04.py`, round 1's independent world builder). The court laws write the genesis row only. Women hold under the cognatic laws. **But the tanistry stand-in seats children (MAJ-1).** |
| R4 | **Done** | `next/M7` 0.4.0 has three legal `[[amends]]`. The world-creation event, the dormancy pass and the first exit draw are all restored. M2's `initialization.state` is re-stated. The lock goes from 19 to 22 and regenerates byte for byte. The self-checks pass: M2 rejects 29 planted changes and M7 rejects 23. The four `odd_next` controls still fail on their own diagnostics. **Direct edits of meanings:** legal for `parity`, `last_birth_tick`, `household.*`, `marriage.*` and H2, because none is in frozen 0.2.1, whose columns are `person.*` 1–9 and `life_course.*`. The same edit is illegal for a frozen column: the lint requires an amendment. MIN-2 and MIN-5 cover what is wrong in the texts themselves. |
| R5 | **Partial** | **Done:** TOML load with a stale-spec control; each rounding control fails on its own message; INV-BF-03 is evaluated at conception (`work/robust.py` passes 110 configurations, and the age-50.47 mother at seed 25 now passes); D-11 applies in P10; the digests pin `parity`, `last_birth_tick` and `house_member`; the orders are stated; `majority_age` comes from the compiled parameters; there are 2·A cells; `synth_offset` uses 16·i + slot; P7's wording is fixed. **Not done:** the sibling gap is below M2's minimum (MIN-1). The seven-law pins are weak (MIN-3). |
| R6 | **Done** | D46 is in the PR body and in companion §7 at 75%. |
| R7 | **Done** | D40 on `main` carries the house-size finding. |
| R8 | **Partial** | `manor.yaml` sets `majority_age` only, 1 of M7's 17 parameters. None of the 17 has a Registry default (NIT-3). |

### New findings

| ID | Where | Problem and evidence | Fix (confidence) |
|---|---|---|---|
| MAJ-1 | `population_synthesis.toml:117` (B9, the stand-in); companion `:44` (D-04 b); reference `:1069-1078` | **The stand-in rule seats children under tanistry, contrary to R3's "tanistry requires majority_age, so no child holder".** It is the same outcome round 1's MAJ-3 found, now reached through the fallback. In the drafter's own 20-seed sweep, all 6 tanistry fallbacks seat a minor, aged 0.6 to 14.8. Over seeds 21–60 it is 42 of 42 (tanistry falls back in 48 of 161 successions over 60 seeds). In the G = 6, 365-day configuration it is 26 of 29. Partition also falls back 3 times in seeds 21–60, twice with a child (`work/standin.py`, `work/compare_d04.py`). D-04 (b) never says the stand-in is a child, and its claim "never for the other four laws" holds only for the 20 seeds swept. The adult branch of the rule is never exercised: the mutant "oldest living male regardless of age" passes every gate (`work/mut_standin.py standin-adult-any`). | Have the Architect rule explicitly, either (a) or (b). (a) Accept minor stand-ins: state the frequency in B9 and D-04, and pin a case where the adult branch fires (partition, a seed from 21–60). (b) For an empty line with no adult male, apply M7's own no-heir rule: a vassal title reverts in demesne to its liege's holder; a sovereign goes to the realm's highest-tier vassal. INV-BF-05's "house member" would then need to relax, and (b) is still not total for a polity with one dynasty. I recommend (a). (7/10) |
| MIN-1 | `population_synthesis.toml:91` (P10) and `:79` (`sibling_gap_minutes = 806400`); reference `:156` | **R5 required a sibling gap of at least M2's minimum birth-to-birth gap, and this one is below it.** The gap is 560 days, while M2's minimum is 602 days on a 360-day calendar and 609 on a 365-day one. The text's own reasoning, "below the least gap M2 itself can produce … so a synthesised pair of siblings is never closer than a born pair", is backwards. Across 80 cases, 22 of 1,983 synthesised sibling pairs fall below 602 days, the closest at 563.3. The golden manor seed 1 has a pair 588.9 days apart. The back-filled minimum is 604 (`work/gaps.py`). | Set the gap to M2's minimum, which is the t_(m+30) + 1 day − (t_(m+10) − 1 day) minimum: 602 days on a 360-day calendar, 609 on a 365-day one, or a single 609-day constant. Fix the sentence and regenerate the golden file. (8/10) |
| MIN-2 | `next/M7.dynasty.odd.toml:621` and `:846` (`initialization` to-text); `:137` and `:851` (`tenure.title` to-text) | Four problems in the amended texts. (1) The `initialization` text says "every past holder selected by the title's succession law"; that is false whenever the stand-in decides. (2) It keeps "offices and their holders from the manifest's dynasties" and also adds "offices vacant". (3) It says a minor genesis holder's regent is named by "this system's first Day/Polity binding", but no step does that: Day/Polity has on_birth, on_death, arbitrate, exit and regency-end only. (4) The `tenure.title` to-text keeps `holders_of(title, 8)`, while B9, the TOML header and PR #41 use 16. | Name the stand-in. Drop "and their holders". Write "regent = 0; no step names one until SPR-021". Write 16. Then regenerate the lock. (8/10) |
| MIN-3 | golden `:142-212`; TOML `:121` | **The golden pins of the seven laws discriminate little.** At seed 1, primogeniture, seniority, partition and tanistry produce identical tenure logs. The sweep pins counts only. The mutant "stand-in picks the youngest" was caught only because a count happened to move (55 → 54). A window of 8 changes no golden line and no line of the differential gate (0 differences), because none of the manor's logs exceeds 8 rows. Only the constants check catches the changed constant. | Pin a digest of each law's 20 tenure logs, including each line's head. Add a G = 6 case, where logs reach 13 rows. (8/10) |
| MIN-4 | `next/M7` 0.4.0 against PR #61 and PR #41 | **`next/M7` 0.4.0 breaks when either PR lands.** A trial `git merge-tree` with #61 (`2283e26`) gives 11 lint errors: `claim.cause_event` becomes `event_ref` (2 errors) and 9 `[[emits]]` lose their `roles`. Re-deriving `next/M7` from the merged 0.3.3 and re-applying the three amendments passes, with the lock unchanged (`merged61b/`). PR #41's window change (8 → 16, simulated in `sim41/`) breaks the `tenure.title` `frozen` text. The PR also conflicts with `main` right now, in `bin/check_all.sh` (GitHub reports `dirty`). | Whichever PR lands second re-derives `next/M7` from the frozen file. Say so in `next/README.md`. Merge `main` now. (8/10) |
| MIN-5 | `next/M2.demography.odd.toml:169` (`last_birth_tick` meaning) and `:737` (`initialization`); TOML P10 | **Three texts give three rules for a genesis mother's `last_birth_tick`.** The meaning says "the latest birth tick of her genesis children". Read literally, that includes H2 mothers, whose children range in age over [0, 18). `[initialization]` says `i64::MIN` for founders. P10 covers only synthesised and back-filled mothers. | State that H2 mothers keep `i64::MIN`, which leaves `household`'s golden unchanged. Alternatively, extend D-11 to H2 and announce that the golden moves. (7/10) |
| NIT-1 | TOML `:84` (P3) | The width argument is wrong as a general statement, though every caller is safe. "phi′·S < 2^96" does not follow from S·total < 2^96: at N = 10^7, P9 has S near 2^67, so phi′·S is near 2^99. "A magnitude below 2^128" is also not an i128 bound. All callers are at most 2^124 (`work/probe4.py` measures 99 bits at a population of 20,000). | Require S·total < 2^95, or use ceil(x − φ) = ceil(x + 1 − φ) − 1 so the arithmetic stays unsigned in u128. (8/10) |
| NIT-2 | B9; D-04 (b) | The claims "tanistry … is the case that occurs" and "never for the other four laws" hold only for the 20-seed sweep. Partition, primogeniture and ultimogeniture also fall back, in seeds 21–60 and in the G = 6 configurations. | Reword. (9/10) |
| NIT-3 | `manor.yaml:44`; `val_khor.yaml:49` | The manor sets 1 of 17 M7 parameters, and none has a Registry default. `val_khor` has dynasties but no `majority_age`, so the planned compile refusal would reject it. | Set the manor's parameters, or say that SPR-021's defaults cover them. Note `val_khor`. (6/10) |

### The machinery questions

- **Each amendment is legal.** Every M7 `frozen` text is byte for byte. Each `to` is a whole text and a superset of the frozen meaning, apart from the changes MIN-2 lists.
- **Editing a meaning without `[[amends]]`:** this is illegal for a frozen column (the lint error is "frozen column … changed its meaning without an [[amends]] record", which I saw on the merged tree). It is legal for a column that is new in the next version: README `:25` binds only frozen columns. The drafter's four direct edits are all on new columns.
- **Stand-in totality:** the rule is total and deterministic. Ties go by (birth, PersonId). Under B3 the latest-born spine son is always alive at a pre-epoch death, because his death falls strictly after his heir's birth, or at or after 0 for S_G. S_G is alive at tick 0. No case was found in which no male is alive. The rule contradicts none of PR #41's programs, which define only the line (§7.2). It does replace frozen M7's no-heir arbitration (claims, regrant, demesne, interregnum), which D-04 (a) acknowledges. It also seats minors without a regent (MAJ-1).

### Round-1 scripts re-run

- `compare_d04.py`: rewritten for round 2. No line differs; MAJ-1 still reproduces through the stand-in.
- `probe4.py`: the 13,000-bit widths are gone; the widest integer is 99 bits.
- `robust.py` and `probe5.py`: INV-BF-03 holds in every configuration, and the seed-25 mother passes.
- `probe2.py` and `gaps.py`: MIN-4 of round 1 still reproduces (MIN-1).
- `ulti.py`: superseded. Its B9 helpers no longer exist; the differential covers 742 ultimogeniture successions.

### Gates (all pass at `5466fc6`)

- **Reference:** passes 4 cases and 212 lines.
- **Controls:** `spacing`, `rounding-cells`, `rounding-table`, `--spec` (stale) and `--golden` (altered) all fail as required, each on its own message.
- **Differential:** `--effect-ref` runs 263 successions with no difference.
- **`lint_odd`:** passes in both modes; the lock regeneration is identical.
- **Other checks:** `gen_manifest --check` builds 179 documents; `manifest_v0.py validate manor.yaml` reports VALID; pytest passes 627 tests; ruff and mypy are clean.

### Not run

- **The full `bin/check_all.sh`** (cargo, valgrind, the golden part). The PR has no Rust changes.
- **A complete independent re-implementation of B4–B8.** Round 1's `indep.py` is not ported. B4–B8 are unchanged apart from wording (I diffed the round-1 TOML), and B9 was checked against PR #41.
- **`flow_sweep.py`.** P7 changed only in wording.
- **`probe3.py`.** The house-size finding was ruled into D40.
- **The lint on a real merge with PR #41.** That PR conflicts in M7 on its own branch, so I simulated its window change with `sed`.
