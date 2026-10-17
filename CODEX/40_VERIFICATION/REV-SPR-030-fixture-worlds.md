---
id: REV-SPR-030
title: "Independent review of the SPR-030 spec deliverables: the dynastic fixture world, the 10k manifest and the V-DET-17 protocol (PR #73)"
type: record
status: OPEN
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-030, SPR-019, SPR-021, SPR-022, SPR-015, REV-SPR-019, REV-SPR-029, BCK-002]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF: round 1 on `c9efce8`. Changes are needed: 1 blocker, 3 majors, 7 minors and 2 nits.**
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
