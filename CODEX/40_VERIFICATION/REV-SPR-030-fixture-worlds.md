---
id: REV-SPR-030
title: "Independent review of the SPR-030 spec deliverables: the dynastic fixture world, the 10k manifest and the V-DET-17 protocol (PR #73)"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-030, SPR-019, SPR-021, SPR-022, SPR-015, REV-SPR-019, REV-SPR-029, BCK-002]
created: 2026-10-17
updated: 2026-10-18
version: 1.0.1
---

> **Supersession note (2026-10-18; BCK-002 A-62, D56 (e); the rulings below are unchanged).** D56 (e) moved the back-fill: every back-filled wife now draws `c_i^e`, so `fixture_worlds_v0.txt` and `crn_pairing_v0.txt` were regenerated. **R2's pinned empty-line route is retired**: no sovereign's line is empty at any pinned genesis (the one empty line, `kingdom_10k` seed 1's House Moss title, is a major title and goes to a regrant). **R8's routes are now the short lines** (`twin_vales` seed 1's Lord of Ash has a line of one; the tanistry lines are 1 to 9 long) **and a T1 pressed claim**. `specs/worldgen/fixture_worlds.md` §2.3 keeps its own statement of the routes. **R10's CRN seeds are now a search** (the A-62 review's E4, since (e) left one VALID seed of 1 to 3): the PR tier takes the first VALID seed of 1 to 4 (seed 1), and the nightly tier tries seeds in order until two are VALID, up to seed 12 (seeds 1 and 6; `specs/determinism/crn_pairing_v0.md` 0.4.0, D-08).
>
> **BLUF: CLOSED on `3b42714` (round 4, verified by the Architect, 2026-10-17). The SPR-030 spec deliverables are ready to merge** (§0d).
> - R23–R27 hold.
> - Every SPR-030 gate passes or fails as required in the PR tier, and both nightly commands pass.
> - All eleven CRN mutants fail at seeds 1 and 3; the correct engine passes.
> - The folded population-synthesis gate takes 27 s, where the gate and its separate control took about 50 s.
>
> Four rounds; rulings R1–R28. BCK-002 A-56 (M7 house membership) and A-57 (M7 dormancy precedence) go to SPR-021's plan.
>
> **Round 3** on `35b0717`. Ready after short fixes: no blocker, no major, 3 minors and 2 nits (§3). Rulings R23–R27 are in §0c.
>
> **Round 2** on `c442adb`. Changes are needed, but small: 1 major (the nightly CRN job fails every night), 7 minors and 1 nit (§2). Rulings R14–R22 are in §0b.
>
> **Round 1** on `c9efce8`. Changes are needed: 1 blocker, 3 majors, 7 minors and 2 nits.
>
> **What holds:**
> - Six new gates and their controls pass or fail as they should.
> - Rust-loader parity holds on 63 of 63 manifests.
> - Every M7 and T1 value is within its Registry bounds.
> - The rate arithmetic of `fixture_worlds.md` §2.3 recomputes exactly.
>
> **What fails:**
> - The population projection ignores the house-member parameters the manifests set (the blocker).
> - The named-at-start counts omit M7's heir clause.
> - V-DET-17 can pass vacuously when the input changes nothing.
> - Its re-sample path is never exercised.
>
> The review also exposes an M7 design question: cognatic house membership saturates the population. It is filed for SPR-021 as BCK-002 A-56 (§0).
>
> **Rulings:** R1–R13 in §0.
>
> **Reviewer:** a session on a different model from the drafter's.

# REV-SPR-030: independent review of the SPR-030 spec deliverables

## 0. Architect rulings, round 1 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R1 | F1 (BLK); fixture D-04 | **Accepted, in three parts.**<br>**(a) The G1a criterion is not reinterpreted.** G1a's performance run stays at 10,000 persons. `kingdom_10k` sets `density_capacity: 10000` as SPR-030's first C1 change, once the M2 F1 spec that carries the parameter is frozen (SPR-015/SPR-016, wave 5, long before SPR-030 runs). Delete "the G1a budget is read against the mean".<br>**(b) The projection includes the house-member mechanics.** M7's `on_birth` membership is cognatic. `elite_fertility_mult` applies to every member woman (M2 F3/F4). `vocation_holy_orders` removes members from marriage and conception (M2 N3/N4/F5). Replace §3.3 and D-04's figures with a derivation that includes them, for both worlds: the member share and the population over each world's horizon, with the uncapped and the capped (`kingdom_10k`) cases. Use the reviewer's `elite_growth.py` method or your own, and state its limits (settlement-bounded marriage, age structure).<br>**(c) The saturation is an M7 design question, not a fixture question.** It is filed as **BCK-002 A-56**, to be decided before SPR-021's plan with D40's pre-registered revisit and REV-SPR-029 R18's back-fill mismatch. The question is whether membership stays cognatic, becomes agnatic, or is bounded by degree from the line. The fixtures keep their values (1.2 and 0.2), so that G1a exercises the elite read | 75% |
| R2 | F2 (MAJ) | **Accepted.** The named set is M7's rule as written. It is the members within `dormancy_degrees` of a holder, plus the first and second of every title's line (on_birth), plus a ruling house's heir (dormancy), plus claim holders. Pin heir and second per title at genesis: `(k, distance, line length)`. Recount the named at start under that rule; it is the gate's reading, and the narrow reading is reported beside it. Correct §2.2's Ashcombe sentence and the BLUF's "from the first Year pulse" ("Yearly, and at world start"). Pin the High Fells sovereign's **empty line at genesis** in `kingdom_10k` seed 2: it is the computable route to an interregnum | 80% |
| R3 | F3, F4 (MAJ) | **Accepted.** A pair is **INVALID**, and the gate fails, unless three things hold: the target is alive at T_IN in both runs; its death instants differ; and at least one of them falls before T_END. T_IN is chosen per seed inside the month of the target's run-A death, before the death day, as `solo`'s `inputs.toml` does. That exercises S11's cancel-and-re-sample. Re-pin the golden with the new T_IN. Add the reviewer's no-op-input and swap-remove-after-T_IN mutants as controls that must FAIL, by name | 85% |
| R4 | F5 (MIN) | **The protocol's assertion is exact CRN:** in P1a, every person of U has an identical death instant in both runs (identical share = 1). ρ ≥ 0.9 stays docs/09's floor; it is reported, and it becomes the binding gate only when later systems couple lineages (contagion, war). Add the reviewer's +1-day-shift and 2% re-key mutants as must-FAIL controls | 75% |
| R5 | F6 (MIN) | **Accepted.** State the real argument for S's coverage: M2 deaths are age-only, non-M2 deaths fall only on sovereigns and named actors, and the house covers the genesis lines. KIN_DEGREES_MAX bounds a single read, not a chain of consequences, so it is not the argument. Add every candidate in the title's line to S. Also pair partnering, because N4's `husband_pick` cites V-DET-17: marriages and births where both spouses are in U are identical across the pair, and post-T_IN children are paired by PersonId | 70% |
| R6 | F7 (MIN) | **Accepted.** The fixture golden keeps `render_house`'s title and tenure lines, or their digests (tier, liege, polity, `held_since`, `predecessor_exit`), and each house's member k list instead of a range. The reviewer's four plants must each fail one of the two goldens | 90% |
| R7 | F8 (MIN) | **Accepted.** Restate the ≤500 argument from M7's full rule (R2). Kin classes are by parent-or-child steps: uncles, aunts, nieces and nephews are 3 steps. Count members only. Drop the office reasoning: no offices exist at genesis in P1a. SPR-030's G1a run asserts the yearly maximum of `person.named` ≤ 500; add that to §6's list for the Developer | 80% |
| R8 | F9 (MIN) | **Accepted.** "Contested by construction" becomes the computable routes: the empty line (R2), and a deposed ruler's pressed claim at a later succession, which T1 decides and is not counted. The V-INV-03 agenda-action path falls back to T1's isolation court fixture | 70% |
| R9 | F10 (MIN) | **Accepted.** `twin_vales` keeps the next Registry's default capacity (10,000,000), which never binds, and the CRN protocol's D-03 then holds. Only `kingdom_10k` sets a capacity (R1a) | 80% |
| R10 | F11 (MIN); CI time | **Accepted, and more.** In the PR tier:<ul><li>fixture-worlds renders once and compares with the real and the altered golden in one process;</li><li>one `kingdom_10k` seed and all three `twin_vales` seeds;</li><li>CRN at seed 1 only, and its control reuses run A.</li></ul>The other seeds go nightly. **Also, as owner of `selftest_population_synthesis.py`:** its PR-tier controls share one reference run, which saves about 2 min. Part core reached 13 min 45 s on SPR-013's head; two of those controls moved to part golden in SPR-013, and this restores the margin. Report each gate's seconds | 85% |
| R11 | F12, F13 (NIT) | **All accepted.** Fix the stale text (M7 0.3.4, seeds 1–3, the ρ² explanation, the 112 versus 39, "hazards compound", `MONTH_DAYS` mirrored against the YAML). The CRN control expects `PROTOCOL FAILED: seed 1`. Add a semantic fixture control, for example `DORMANCY_DEGREES = 1` | 90% |
| R12 | Drafter items 2 and 3 | No catalyst in either world (T1 reads none in P1a): accepted. `village` stays open in A-06 and moves with its P1b content: accepted | 85% |
| R13 | Director | No director item is needed for this PR. The Architect's merge summary tells the director about R1(a), which holds the G1a criterion as written, and about A-56, the M7 membership question that touches D40's revisit | — |

**Round 2:** the drafter applies R1–R13, regenerates the goldens with the tools, runs `check_all --part core` with the gate timings, and pushes once. An independent check follows, with the reviewer's `work/` scripts.

## 0d. Round 4, verified by the Architect: CLOSED (2026-10-17)

The Architect verified `3b42714` (a git archive in `fixtures_review_r4/src/`) with the round-3 reviewer's scripts. **No finding is open. The spec is ready to merge.**

**The round-3 rulings hold:**
- **R23.** The partnering clause covers a U woman only while her N4 candidate list and her own eligibility are identical in both runs at each of her draws. A differing marriage outside that set is reported with the person responsible: an S person, or an exposed U woman. The engine check replays N4 from the ledger.
- **R24.** §6's fallback for V-INV-03's agenda path is `kingdom_10k`'s 1,000-year G1a runs, with the gap reported.
- **R25.**
  - The four "pairs differ" controls require a VALID pair.
  - `choose_target` applies the window and the pairs floor from run A. The golden prints `runA_informative` per candidate.
  - Seed 2's SKIPPED block pins run A's deaths digest.
  - The companion states that the run fails when no tested seed gives a VALID pair.
- **R26.** The three text slips are fixed, and the full reading's resolution of A-57's overlap is stated (in favour of naming).
- **R27.**
  - The effect differential's reversed-line control runs inside the population-synthesis gate's run: "1148 of 1389 successions differ (as required)". The separate line and its inventory entry are gone.
  - Run A is snapshotted at the input's month instead of re-run.

**Evidence:**
- **PR-tier gates:** all ten SPR-030 lines pass or fail as required, each for its own reason.
  - The folded population-synthesis gate takes 27 s.
  - The fixture-worlds gate takes 10 s; its dormancy-one control, 2 s.
  - CRN at seed 1 with its five controls takes 28 s.
- **Nightly:**
  - `fixture_worlds --tier nightly`: PASS (17 s; 5 cases, 372 lines).
  - `crn_pairing --seeds 1,2,3 --controls`: PASS (63 s, from 71 s): "2 VALID and exact pair(s), 1 seed(s) skipped for power, controls 10 of 10 FAIL as required".
- **Mutants:** the reviewer's `crn_mutants_r2.py` at seeds 1 and 3.
  - All eleven fail, with the round-3 counts unchanged: review-swap 120/274 and 101/206, row-index 101/268 and 95/216, re-key 1% 3/231 and 4/179. No-op-input and skip-S11 give an INVALID pair.
  - The correct engine passes.
  - No person dead before T_IN gets a new death tick.

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R28 | `crn_pairing_v0.md` §4.1 still says run A is re-run to `T_IN`. Since R27, the common starting point is run A's snapshot at the input's month, run forward to `T_IN` | Fixed in this review's PR: a text change only, and the reference already does it | 95% |

**Merge order with SPR-013.** On `main` without SPR-013, PR #73 replaces the separate effect-control line in part core. SPR-013 (PR #64) moved that line to part golden. Whichever of the two merges second deletes the line from part golden and keeps the folded control (R27).

**Part core's budget after both merges.** SPR-030 adds about 41 s to part core, and its fold takes about 25 s out of part golden. The CRN controls (about 17 s) stay in part core unless a CI run puts part core above 12 minutes.

## 0c. Architect rulings, round 3 (2026-10-17)

The round-1 reviewer verified round 3 on `35b0717` (§3). **No blocker or major is left. The spec is ready to merge after the fixes below.**
- R14 and R16–R19, R21 and R22 hold; R15 and R20 hold partly.
- Both nightly commands pass.
- Every mutant fails at seeds 1 and 3, including row-index, skip-S11 and the 1% re-key.
- A seed that is skipped for power cannot hide a regression: if every seed skips, the gate fails, and a VALID seed that turns SKIPPED fails the golden.
- The degree-ignoring plant now fails `dormancy-one`.

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R23 | H1 | The partnering clause covers a U woman only while her N4 candidate list and her own eligibility are **identical in both runs at each of her draws**. That implies the same marriage, because `husband_pick` is keyed `(w, t, h)`. State how the lists are observed: the engine check replays N4 from the ledger, since the lists are not in channel A | 75% |
| R24 | H2 | §6 (line 161): V-INV-03's agenda-path fallback is `kingdom_10k`'s G1a runs, with the gap reported, as §2.3 and R20 say | 95% |
| R25 | H3 | All four:<ul><li>the four "pairs differ" controls require a VALID pair;</li><li>`choose_target` falls through to the next candidate on the pairs floor as well as the window;</li><li>a SKIPPED block pins run A's deaths digest;</li><li>the companion says the engine run fails when no tested seed gives a VALID pair</li></ul> | 90% |
| R26 | H4 | The text slips:<ul><li>a co-parent of a grandchild is 3 steps away;</li><li>state that the full reading resolves the far-adult-heir overlap in favour of naming, pending A-57;</li><li>CRN D-02 lists the realm-members component</li></ul> | 95% |
| R27 | H5 | Part core's budget:<ul><li>**Fold the effect-differential control into the population-synthesis gate's run.** Today it repeats a full reference run of about 25 s, of which the differential is about 7 s. Make it an in-process control that reuses the gate's genesis and runs the differential against a reversed `succession_line`, saving about 18–35 s.</li><li>Snapshot run A at T_IN instead of re-running it.</li><li>The CRN controls stay in part core unless CI shows part core above 12 minutes after these savings; then they move to part golden.</li><li>SPR-013 moves the separate effect-control line to part golden. When main has SPR-013, the folded control replaces that line</li></ul> | 80% |

**Round 4:** the drafter applies R23–R27 and merges main once SPR-013 has landed. The drafter then runs `check_all --part core` and the nightly commands, and pushes once. The Architect verifies with the reviewer's `_r2` scripts and closes the review.

## 0b. Architect rulings, round 2 (2026-10-17)

Round 2 was reviewed on `c442adb` by the round-1 reviewer, with its scripts (§2).
- R4, R6, R9 and R12 hold fully, and the rest hold partly.
- The projection reproduces the round-1 sketch to the person, and the named-at-start recount matches exactly.
- Every genesis plant and CRN mutant fails.
- The PR-tier gates fell from about 236 s to 96 s.
- The one real defect is the nightly CRN job, which fails every night.

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R14 | G1 (MAJ) | **Accepted.**<ul><li>The position-keyed mutant sets `order = sorted(alive)` at the input, so no one dead before T_IN is re-killed.</li><li>VALID also requires power: a post-input window of at least 5 sim-years and at least 100 informative pairs in U.</li><li>Choose each seed's target and T_IN to meet it: the primogeniture sovereign, else the tanistry sovereign, else the next seed. Pin the choice in the golden.</li><li>Each control asserts its own expected reason, not any failure.</li><li>The nightly command must pass. Run it before you push</li></ul> | 90% |
| R15 | G2 | **Accepted.** The partnering clause covers only U persons whose N4 candidate lists contain no S person, in either run, over the window. A differing marriage outside that set is reported with the S person responsible, not gated | 75% |
| R16 | G3 | **Accepted.** S is completed after the runs:<ul><li>everyone named at any instant in either run, from the Promote and Demote records;</li><li>every member of every house of the target's realm.</li></ul>Report \|U\| after the extension. It must stay at least 100 by R14 | 65% |
| R17 | G4 | **Accepted.**<ul><li>The capped case is "+4% / −9% over 1,000 years" (peak 10,415, year 1,000: 9,080).</li><li>State the knife edge: the saturated member NRR is 0.8 × 1.2449 = 0.9959, a 0.4% margin inside the model's own error.</li><li>Interpolate the sensitivity lines at the horizon, year 1,000, not year 1,028</li></ul> | 95% |
| R18 | G5 | **Accepted.**<ul><li>Report the dormancy counts excluding the full (named) set.</li><li>Correct the BLUF and D-01 claims about dormant members in every house (carrow has none at seeds 1–2), and `kingdom_10k.yaml`'s header (the full rule).</li><li>M7's precedence for a far adult heir, who meets both Demote and Promote, is filed as BCK-002 A-57 for SPR-021</li></ul> | 90% |
| R19 | G6 | **Accepted.** `--control dormancy-one` compares the counts without the label, or pins the expected narrow count at degree 1. The reviewer's `mut_degrees_ignored` plant must then fail | 95% |
| R20 | G7 | **Accepted, the round-1 suggestion withdrawn.** T1's isolation court cannot observe V-INV-03's consequence set. The agenda-action path falls back to the `kingdom_10k` 1,000-year G1a runs at seeds 1–3, with the gap reported if those show none. Add the line refilling when an agnate within three steps comes of age to the contest routes | 80% |
| R21 | G8 | **Accepted.** Fix §3.2:<ul><li>co-parents are 2 steps, so an in-married wife is kin by a shared child;</li><li>saturation adds the wife and mother as members (about 2 per holder);</li><li>one expected range for the named, not two</li></ul> | 85% |
| R22 | G9 | **Accepted.**<ul><li>T1 0.3.3.</li><li>The seeds in `twin_vales.yaml`.</li><li>Remove the dead `PR_SEEDS`, or make `check_all` use it.</li><li>`members_k <house>: …` with a separator.</li><li>The golden's line count.</li><li>State the T_IN fallback when the target's run-A death is not an M2 death.</li></ul>The optional savings (the effect control sharing the gate's run; snapshotting run A) are welcome if cheap | 90% |

**Round 3:** the drafter applies R14–R22 and merges main once SPR-013 has landed, reconciling the population-synthesis control placement. The drafter runs `check_all --part core` and the nightly CRN and fixture commands, then pushes once. The Architect then verifies with the reviewer's `_r2` scripts.

## 1. The review (round 1, independent reviewer)

Copy reviewed: a git archive of `c9efce8` in the Architect's scratchpad, `fixtures_review/src/`. The scripts are in `fixtures_review/work/`; run each from the base directory as `python3 -I -B work/X.py src`.

**Verdict: changes needed (1 BLK, 3 MAJ, 7 MIN, 2 NIT).**

**What passes:**
- Six new gates and their controls: both manifests VALID; the fixture-worlds golden passes and its control fails; CRN passes and `--control global-epoch` fails.
- Rust-loader parity: 63 of 63.
- The pytest mirror and inventory: 16 tests. ruff, ruff format and mypy are clean.
- No frozen spec, golden or baseline is changed.
- Every M7 and T1 value is within its Registry bounds, and the validator enforces them.
- Recomputed and correct: §2.3's rate arithmetic (h_d, h_m, 11.7 depositions, P(none) = 8e-6, P(no murder) = 2%), the yearly q values, and every per-house count in §2.2 and §3.2.

| # | Sev. | Finding | Reproduction | Proposed fix |
|---|---|---|---|---|
| F1 | BLK | The population projection omits `elite_fertility_mult` and `vocation_holy_orders`, which act on house members. M7's `on_birth` membership is cognatic, so membership spreads to the whole population. The F1 chain gives NRR 1.0341 at a multiplier of 1.0 and 1.2449 at 1.2. A two-type sketch puts `kingdom_10k`'s member share at about 98% by year ~300. Long-run growth then hangs on (1 − holy) × NRR(elite): 1.2/0.2 gives roughly stationary, 1.0/0.2 collapses, and 1.2/0.0 grows to millions. The drafter's own model gives a mean of 18,200, not 17,000. "The G1a budget is read against the mean" reinterprets a pre-registered criterion | `work/elite_growth.py` | Set 1.0/0.0, or derive and state the trajectory; take the saturation to M7's owners; take the G1a reading to the director |
| F2 | MAJ | The named-at-start reference omits M7's heir clauses (dormancy's "a ruling house's heir"; on_birth's "first or second in the line"). The counts are each at least 1 too low, 2–7 under the full reading. The Ashcombe story is wrong. An empty High Fells line at genesis in `kingdom_10k` seed 2 is missed | `work/heirs.py` | Pin heir and second per title, report both readings, fix the text |
| F3 | MAJ | V-DET-17 passes vacuously: an input on a dead target, or a target who survives both runs (P = 0.30 or 0.16), gives PASS. A swap-remove mutant passes | `work/crn_mutants.py src 1`, `work/target_survival.py` | INVALID unless the target is alive at T_IN in both runs and its death differs; T_IN inside the target's death month |
| F4 | MAJ | The S11 re-sample path is never exercised (`pending none` at both seeds) | golden | F3's T_IN fixes it |
| F5 | MIN | Pearson alone tolerates a uniform +1-day shift (ρ² = 1) and a 2% re-key (0.938) | `work/crn_mutants.py` | Gate identical share = 1; keep ρ ≥ 0.9 as the floor |
| F6 | MIN | k = 6's justification is wrong: KIN_DEGREES_MAX bounds a read, not a chain. The partnering CRN is not observed | — | State the real argument; add the line's candidates to S; pair marriages and births among U |
| F7 | MIN | The fixture golden keeps only digests: wrong `polity` and `predecessor_exit` plants pass both goldens | `work/plant_genesis.sh src work` | Keep the title and tenure lines; pin member k lists |
| F8 | MIN | The ≤500 argument uses wrong kin classes, counts non-members, and relies on offices that cannot exist at genesis | `work/holder_kin.py` | Restate from M7's full rule; gate the yearly maximum of `person.named` |
| F9 | MIN | "Contested by construction" is overstated: at the deposition's arbitration the deposed is excluded. The empty-line route is not pinned. Depositions are not deaths | — | Pin line lengths; fall back to the T1 isolation court |
| F10 | MIN | D-04's `density_capacity` 1,500 for `twin_vales` contradicts the CRN protocol's D-03; the next Registry's default is 10,000,000 | — | Keep the default for `twin_vales` |
| F11 | MIN | The new gates add about 77 s to part core; about 40 s can be saved | timing table | Render once; one 10k seed; CRN seed 1 in the PR tier; reuse run A |
| F12 | NIT | Stale text: M7 0.3.3; "seeds 1 and 2"; the ρ² explanation; 112 includes the dead; "hazards compound"; `MONTH_DAYS` not mirrored | — | Fix |
| F13 | NIT | The CRN control's string also matches a self-check failure; the fixture control tests only the comparator | — | `PROTOCOL FAILED: seed 1`; a semantic plant |

**Gate timings** (4-core box, single-threaded Python):

| Gate | Time | Notes |
|---|---|---|
| Manifests | 0.7 s and 0.4 s | |
| Fixture worlds | 16.9 s | 10.4 s with one 10k seed |
| Its control | 16.1 s | |
| CRN | 28.3 s | 14.9 s with one seed |
| CRN control | 14.9 s | |
| Pytest mirror | 1.2 s | |
| mypy | 1.7 s | |
| V-WC-04 parity | 18.0 s | the two manifests add about 0.6 s |
| **New in part core** | **about 77 s** | |

## 2. The review (round 2, the round-1 reviewer)

Copy reviewed: a git archive of `c442adb` (`fixtures_review_r2/src/`). The scripts are in `fixtures_review_r2/work/`.

**Verdict: changes needed, but small (1 MAJ, 7 MIN, 1 NIT).**

**Holds:**
- R4, R6, R9 and R12.
- The projection: the uncapped peak is 12,607 at year 294, and about 11,440 at year 1,000.
- The four genesis plants fail, in the PR tier too.
- The recount of the full reading matches: 33/35/26 and 65/71.
- The exact-identity gate catches row-index, swap-remove, a skipped S11 and a 1% re-key.
- The population-synthesis controls fail for their own reasons.
- The PR-tier gates: about 236 s before, 96 s after.

| # | Sev. | Finding |
|---|---|---|
| G1 | MAJ | The nightly `--seeds 1,2 --controls` fails ("CONTROL PASSED: seed 2 [swap-remove-after-input]"), for two reasons: the position vector keeps persons who died before T_IN and re-kills them, and seed 2's 1.3-year window (39 pairs) gives no power |
| G2 | MIN | The partnering clause does not follow from N4: a changed S eligibility cascades to later U women's matches |
| G3 | MIN | S is fixed at genesis, but exact identity needs it complete: later-named members of the realm's other houses |
| G4 | MIN | "Capped within 4%" is false: +4.2%, then −9.2% at year 1,000. The 0.4% knife edge is unstated, and the sensitivity lines are at year 1,028 |
| G5 | MIN | The full and dormant counts overlap (far adult heirs): 1–7 persons. M7's Demote/Promote precedence is ambiguous. The BLUF, D-01 and the `kingdom_10k.yaml` header are false |
| G6 | MIN | `--control dormancy-one` is vacuous: the compared line prints the degree itself |
| G7 | MIN | The T1 isolation fallback (the round-1 suggestion) cannot observe V-INV-03's consequences |
| G8 | MIN | §3.2: the kinship and membership text is inconsistent |
| G9 | NIT | Stale text; a dead `PR_SEEDS`; an ambiguous `members_k`; the golden's line count; the T_IN rule for a non-M2 death |

## 3. The verification (round 3, the round-1 reviewer)

Copy verified: a git archive of `35b0717` (`fixtures_review_r3/src/`). The scripts are in `fixtures_review_r3/work/`.

**Verdict: ready after two short text fixes. 0 BLK, 0 MAJ, 3 MIN, 2 NIT.**
- R14 and R16–R19, R21 and R22 hold; R15 and R20 hold partly.
- **Nightly:** `fixture_worlds --tier nightly` PASS (17 s); `crn_pairing --seeds 1,2,3 --controls` PASS (71 s): "2 VALID and exact pair(s), 1 seed(s) skipped for power, controls 10 of 10 FAIL as required".
- **Mutants**, pairs differing at seeds 1 / 3:

  | Mutant | Seed 1 | Seed 3 |
  |---|---|---|
  | Reference swap-remove | 38/247 | 24/176 |
  | Review-swap | 120/274 | 101/206 |
  | Row-index | 101/268 | 95/216 |
  | Re-key 1% | 3/231 | 4/179 |

  No-op-input and skip-S11 are INVALID at both seeds.
- **Skips:** `--seeds 2` alone gives `PROTOCOL FAILED: no seed produced a pair`. A window floor planted at 7 years gives STALE or no pair.
- **Plants:** the degree-ignoring plant fails the gate through `dormancy-one`; the four genesis plants fail the fixture golden.

| # | Sev. | Finding |
|---|---|---|
| H1 | MIN | The bounded partnering clause can still fail on a correct engine: N4's per-step matched set couples a woman with no S person in her list to an exposed earlier woman |
| H2 | MIN | §6 (line 161) still names T1's isolation court as the fallback R20 withdrew |
| H3 | NIT | "Pairs differ" controls accept an INVALID pair; `choose_target` checks only the window; a SKIPPED block pins only two death years; the companion does not state the all-skipped failure |
| H4 | NIT | A co-parent of a grandchild is 3 steps away; the full reading's resolution of A-57's overlap is not stated; CRN D-02 omits the realm-members component |
| H5 | MIN | The PR-tier gates take about 101 s on the reviewer's machine. Fold the effect control into the gate's run (about −18 s); snapshot run A (about −3 s); the CRN controls (about 17 s) to part golden only if needed |
