# M7 house membership and dormancy precedence (BCK-002 A-56, A-57)

*Status: the spec drafter's analysis for the Architect, 2026-10-17; round 1 of PR #79's independent review applied (rulings R1 to R14; §12 maps each to its change). It carries the `M7.dynasty` 0.5.0 draft (`specs/systems/next/M7.dynasty.odd.toml`), the projection (`specs/worldgen/project_population.py`, golden `specs/worldgen/golden/population_projection_v0.txt`), the deny-by-default next lint (`specs/systems/lint_odd.py`, R3), the consequential text in `specs/worldgen/population_synthesis.toml`, `specs/annals/annals_v0.md` and `specs/worldgen/fixture_worlds.md`, and the director item D56 (`docs/decisions_pending.md`). Evidence: REV-SPR-030 F1, R1, R17, R18, R21, R26; REV-SPR-029 R18; PR #79's review, round 1.*

> **BLUF.** The frozen M7 0.3.4 gives a newborn "the father's house, else the mother's". In every regime that cognatic fallback makes every person of `kingdom_10k` a house member by year 300 and ties the world's growth to the elite parameters (§3). **The regime decides the rest.** In M7's isolation run (T1 stubbed, everyone on M2's market) a fallback bounded to the daughters near the seat holds a house at its genesis size; but both fixture worlds register T1, and there the named marry the named (T1's `marry` takes unmarried named adults; M2's N3/N4 exclude them), so the bounded fallback is idle and **agnatic and bounded membership are the same rule** (3.3 living members per house on `twin_vales` at year 300, 1.3 on `kingdom_10k` uncapped at year 1,000, 0.7 capped). Under T1 a house is a named core, bounded by `dormancy_degrees`, and a dormant periphery that reproduces at (1 − v) × NRR of a member man's wife; at the manifests' values with the frozen vocation reading that is 0.83 per generation and the periphery dies. **D56 is therefore "how do great houses persist in a T1 world"**, with membership one lever among four (§4): (d) vocations for later children only, the frozen texts' primary reading (0.90 × NRR); (e) M2's elite fertility read on the couple, so a member man's commoner wife bears at the elite level (1.2449); (f) hypergamy, T1's marry widened to the polity's unnamed, which lowers house growth on its own; and the rule. Only (d) with (e) makes houses persist: 1.117 per generation, about 97 living members per house on the capped 10k world at year 1,000, 11% of the world and still growing, which is the elite overproduction `elite_fertility_mult`'s own source describes (Turchin and Nefedov, R12 §12). **Recommendation (6/10): agnatic membership** (the father's house, else none; the back-fill's rule; no record, no plane-C change), **with (d) in the M7 draft and (e) proposed as an amendment of the M2 0.3.0 draft**, D56 to the director. The 0.5.0 draft carries agnatic membership, (d), A-57's precedence (one list; naming wins; a named minor is never demoted; criterion (d) widened to any title's heir, R6), the birth instant's edge cases (R5) and the posthumous child (M14). **D40 sets no trigger**: its revisit runs in SPR-030 regardless. **The fixture golden does not move**; the annals' walk is the father's, through any number of in-run rows (R7's probe).

## 1. The question and the evidence

| Item | Evidence | What it says |
|---|---|---|
| A-56 | REV-SPR-030 F1, R1 | Cognatic `on_birth` membership saturates `kingdom_10k` by year ~300; afterwards the population reproduces at (1 − 0.2) × 1.2449 = 0.9959 per generation |
| A-56 | REV-SPR-030 R17 | The capped case runs +4% / −9% over 1,000 years on that knife edge |
| A-56 | REV-SPR-029 R18 | The annals' `house_of` is exactly M7's `on_birth`; the agnatic back-fill and the cognatic fallback disagree on a daughter's child |
| A-57 | REV-SPR-030 R18, R21, R26 | A far adult heir (beyond `dormancy_degrees`, no seat) meets both Demote and Promote |
| D40 | `docs/decisions-log.md` | `tfr` 4.9 approved; a revisit pre-registered on the dynastic fixture world in SPR-030 |
| Round 1 | PR #79's review M1 | Both fixture worlds register T1; T1 marries the named among the named; a bounded fallback's mother is named, so the fallback is idle; the probe (Tilda, named from world start, and Xan, never named) cannot happen |

Three facts bound the design space:

1. **No succession law reads house membership.** Every program of `specs/registry/effect_language_v0.md` §8 takes its candidates from `lineage(holder, 3)`, `children(holder)` or `court(title)` over the life-course log and the tenure log. Membership touches succession only through the `vocation` step (members draw holy orders and leave every line) and through whose house the successor is in.
2. **Membership is what three other mechanics key on:** M2 F3/F4's elite fertility (the woman's row), M7's vocations and dormancy (members only), T1's situation sets (`house_member.*`), the annals' house groups and `HOUSE_EXTINCT`.
3. **A house's own reproduction depends on two readings, not on the membership rule.** A member woman not in holy orders has NRR 1.2449; a member man's commoner wife has no row and bears at 1.0341. Holy orders removes v of each cohort: 0.2 under the frozen submodel's reading ("not first in any title's line": every other member draws), 0.103 under the later-children reading (the first-born exempt; the frozen `[[params]]` meanings, the schedule step and RT-2026-10-16 item 12; R4 makes it primary). So the periphery of a house reproduces at (1 − v) × NRR_wife: 0.83 (all, woman), 0.93 (later, woman), 0.996 (all, couple) or **1.117 (later, couple)**. Everything below follows from which side of 1 that product falls.

## 2. The three rules and the two regimes

| Rule | `on_birth` writes the newborn's `house_member.house` as | Back-fill agrees? | Plane C derives it from |
|---|---|---|---|
| **Cognatic** (M7 0.3.4, frozen) | the father's house; else the mother's; else none | On every back-filled row (no daughter of a house bears there, SPR-019 D-07); the rules differ on an in-run child of a daughter and a houseless father | the father-then-mother walk (annals §4 as R18 had it) |
| **Agnatic** (M7 0.5.0 draft; the back-fill's rule, `population_synthesis.toml` INV "every house member other than the root has a father of the house") | the father's house; else none | By construction | the father walk through any number of in-run rows |
| **Bounded** (PR #79's first draft) | the father's house; else the mother's when she is named-eligible for it at the birth (a seat, office or claim; within `dormancy_degrees` of a holder of her house; first or second in a line), evaluated on M7's own columns; else none | On the back-fill's rows the mother branch never fires | a `HOUSE_JOINED` record (actor the mother, target the newborn), which would need its own act class with a `tellable_up_to` ceiling of 4294967295 so that it is always admitted under DATA-5/D38 (R2), two templates and a label, and the annals' walk reading it |

**The regimes** (R1). *Isolation*: M7's `[isolation]` run, T1 stubbed, nobody named, everyone on M2's market (N3/N4: random within the settlement). *T1 registered*: the saga world, both fixtures included. The named core (M7's criteria, a set bounded by `dormancy_degrees`) marries within itself through T1's `marry` decision (T1 0.3.3 `decide`: "unmarried named adults of the other sex … in the actor's court or relation set"), M2's N3/N4 exclude the named, and the un-named members marry on M2's market. A named-eligible woman is a named woman, so under T1 the bounded rule's mother branch fires only for a woman who became eligible after an M2 marriage (which the model puts at zero) or under hypergamy (f). Under T1, bounded = agnatic.

## 3. The projections (`project_population.py` v2, golden `population_projection_v0.txt`)

The model is REV-SPR-030 R1's two-type generational recursion, in persons, extended with the regime, the rule and the levers (the script's docstring states the recursion and its limits; the cognatic lines in isolation reproduce the old golden to the person). The named core is 2 H W_n persons, W_n the named member women of childbearing age per house measured at genesis by the fixture reference (`--derive-caps`: 1.00 on `twin_vales`, 0.67 on `kingdom_10k`), with a doubled sensitivity. Under the frozen readings (every member draws holy orders, the elite read on the woman, no hypergamy):

**T1 registered, the saga world** (population; living members; per house):

| World, case | Cognatic | Agnatic = bounded |
|---|---|---|
| `twin_vales`, year 300, uncapped | 1,771; 1,771 (100%) | 2,098; 17; **3.3** |
| `kingdom_10k`, year 300, uncapped | 12,709; 12,406 (98%) | 14,059; 30; 2.5 |
| `kingdom_10k`, year 1,000, uncapped | 11,546; 11,546 (100%) | 31,260; 16; **1.3** |
| `kingdom_10k`, year 300, capped at 10,000 | 10,038; 9,795 (98%) | 10,432; 24; 2.0 |
| `kingdom_10k`, year 1,000, capped | 9,090; 9,090 (100%) | 10,433; 8; **0.7** |

**M7 isolation, T1 stubbed** (the regime of M7's `[isolation]` run and of V-SF-22 only):

| World, case | Cognatic | Agnatic | Bounded |
|---|---|---|---|
| `twin_vales`, year 300, uncapped | 1,751; 100% | 2,100; 8; 1.7 | 2,087; 57; 11.4 |
| `kingdom_10k`, year 1,000, uncapped | 11,440; 100% | 31,288; 0 | 31,121; 92; 7.7 |
| `kingdom_10k`, year 1,000, capped | 9,080; 100% | 10,434; 0 | 10,430; 77; 6.4 |

**Reading.** Cognatic saturates in both regimes and the world then reproduces at the member rate (0.996: the knife edge; capped, −9% by year 1,000). Agnatic and bounded leave the world at the commoner rate (+0.114% a year: 10,433 capped, where the commoner NRR times the density term is 1; no knife edge) and the houses shrink: in isolation the bounded rule's inflow of the daughters' children holds a house at about 12 W_n persons, but under T1 that inflow does not exist, and a house is its named core plus a periphery that reproduces at 0.83 per generation. The mean field has 3.3 living members per house on `twin_vales` at year 300 and 0.7 on the capped 10k world at year 1,000: the houses are their holders' families and nothing else, `HOUSE_EXTINCT` is the common event, and dormancy has nothing to demote.

## 4. The levers under T1 (R1): what makes a house persist

One line each in the golden (`lever …`), for the agnatic and the bounded rule; the horizon's living members per house:

| Lever | What it is, and its cost | `twin_vales` year 300 | `kingdom_10k` year 1,000, uncapped | capped |
|---|---|---|---|---|
| none (the frozen readings) | — | 3.3 | 1.3 | 0.7 |
| (d) later children | the vocation submodel exempts the first-born of each father (R4): an M7 amendment in this draft; v = 0.103 | 8.2 | 4.0 | 2.5 |
| (e) the elite read on the couple | M2 F3/F4 give a member man's wife the elite table (an M2 0.3.0 amendment, SPR-015's slice; `c_i^e` when she or her husband has a row) | 10.7 | 8.5 | 2.8 |
| (f) hypergamy, half the core marrying out | T1's `marry` candidates widened to the polity's unnamed (a T1 change, P1b); agnatic loses the named women's children and the named men's wives bear at the commoner level, so it lowers house growth by itself (0.6 / 0.0 / 0.0); bounded keeps the named women's children (2.9 / 0.6 / 0.2) | — | — | — |
| (d) + (e) | the two readings together: the periphery reproduces at 0.897 × 1.2449 = 1.117 per generation | **34.5** | **412.7** (11% of 45,700) | **97.4** (11% of 10,750) |
| (d) + (f) | | 3.3 (agnatic), 7.5 (bounded) | 0.0, 3.2 | 0.0, 2.0 |
| (d) + (e) + (f) | | 24.5 (agnatic), 34.5 (bounded) | 280.5, 412.7 | 61.8, 97.4 |
| cognatic + (d) | the frozen rule with the later-children reading | 646.5 (the world) | 21,752 (the world: 261,000 persons) | 952 (the world: 11,421) |

**Reading.** Agnatic and bounded differ only under hypergamy, which is a P1b T1 change and which, on its own, shrinks houses further; so under T1 the simpler rule wins (R1). No lever gives a stationary house: the periphery's product (1 − v) × NRR_wife is 0.83, 0.93 or 0.996 (slow decay to the core) or 1.117 (growth at 0.4% a year, 11% of the world by year 1,000 and saturation after about 2,000 years). (d) + (e) is the combination that keeps a house a house over the G1a horizon with a periphery for dormancy to act on; its long run is the elite overproduction `elite_fertility_mult`'s source describes, and the manifest's multiplier is the lever that sets the rate (at 1.0 the product is 0.93 and the houses decay). Under (d) the cognatic knife edge does not survive either (M4): the saturated world grows to 261,000 persons uncapped.

**W_n and `dormancy_degrees`** (Q7, M8 (d)). The named core is the one input the fixtures measure: at `dormancy_degrees` 1 the reference's W_n falls from 1.00 to about 0.5 per house on `twin_vales` and from 0.67 to about 0.33 on `kingdom_10k`, so the core, and with it the isolation fixed point and the T1 periphery's seed, halve. The parameter bounds both naming and membership's reach; no second parameter is proposed.

## 5. Named-actor counts, eligibility and the annals, per regime

**Named counts.** At genesis the three rules give the same named set (the back-fill writes the same rows): 33 / 35 / 26 on `twin_vales` at seeds 1–3 and 65 / 71 on `kingdom_10k` at seeds 1–2 (`golden/fixture_worlds_v0.txt`, unchanged). Over a run the named set is the holders' member neighbourhoods plus heirs, seconds and claimants (`fixture_worlds.md` §3.2). Under T1 with agnatic membership a holder's neighbourhood is his father, paternal grandfather, siblings, children and sons' children (his wife and mother are members only of their own houses, if any): 5 to 8 members, 80 to 150 named on `kingdom_10k`, falling toward the 12 holders plus heirs and claimants as the periphery empties under the frozen readings, and rising to §3.2's 100 to 200 under (d) + (e). Cognatic adds the wife and the mother (about 2 per holder). The bound ≤ 500 holds in every case.

**Eligibility by law.** No law reads membership (§1). Indirectly: (i) holy orders removes members from lines: under cognatic every candidate is a member once the world has saturated; under agnatic only members draw, so a non-member on a cognatic line never does. (ii) The successor's house: under `primogeniture`, `ultimogeniture` and `partition` a daughter's line inherits before the holder's brother, about one succession in five (about 1.2 surviving sons per holder, Poisson). In isolation that son is of no house under agnatic membership; under T1 his mother is named and married a named man, so he belongs to another great house, and the seat passes between houses by marriage, which the dormancy rule treats as the first house losing its holder (every member dormant-eligible) and the second gaining a seat. Cognatic keeps the seat in the mother's house in both regimes. `seniority` and `tanistry` are agnatic laws: a member holder's agnates are members under every rule and the seat never leaves the house; their derbfine empties as the house shrinks, which under the frozen readings is within 300 years. `elective` and `sortition` take the court, of any house or none.

**The annals.** Cognatic: by year 300 of `kingdom_10k` every birth, marriage and death falls in one of the twelve house groups, each house annal tends to the year annal, and `HOUSE_EXTINCT` is never told. Agnatic: a house annal is the patriline; a daughter's marriage is in her house's annal (she is in its cast), her children's births are there only as her acts, and the children belong to their father's house; under the frozen readings `HOUSE_EXTINCT` is the common event of a 300-year run, under (d) + (e) it is rare. The walk reads the father through any number of in-run rows (the `ashcombe` fixture's Col: Col → Cedric → Osric → the keyframe's House Ashcombe), never the mother, and no record is needed.

## 6. A-57: the precedence rule, the named set and the dormancy counts

The 0.5.0 draft states, in `dormancy` and in the amended INV-M7-06:

1. **One list of naming criteria** for a great-house member (a holder of the house is a member of it who holds a title, office or claim): (a) it holds a title, office or claim; (b) an active vow (S7, P1b); (c) within `dormancy_degrees` of a holder of its house; (d) first or second in the line, by the title's law as `arbitrate` computes it (after M7's exclusions: the deposed and persons in holy orders; a court law's line is the court's order; a sortition title has no first or second outside its arbitration instant), **of any title, whatever house holds it** (R6: an heir apparent is named whatever its house; the frozen "a ruling house's heir" read as the house's own titles only, which left a sovereign's heir from another house unnamed until he succeeded).
2. **Naming wins.** A member meeting any criterion is named; a far adult heir or second is named and not dormant-eligible.
3. **Age enters Demote only.** A named adult meeting no criterion is dormant-eligible (Demote); a named minor is never demoted. Promote goes to a member not named that meets a criterion, at a pass or at `on_birth`, on any criterion (M13: `on_birth` and `dormancy` now say the same).
4. **The named set after a pass** is the members meeting a criterion plus the named minors. At world start the pass names exactly the members meeting a criterion, so a minor beyond the degrees who is not in a line's head is not named at start (the question `fixture_worlds.md` §6 left to SPR-021). The dormancy counts are the Demotes and Promotes sent, reported and never a target.

The fixture reference computed the full reading as "near or in the line's head" and the dormant-eligible adults as the far adults outside it (`house_report`), which is rule 2, so `golden/fixture_worlds_v0.txt` is unchanged, `--tier pr` passes and `--control dormancy-one` still fails for its own reason. R6's widening changes no genesis line: all 39 genesis lines have `outside_house 0`.

## 7. The birth instant's edge cases (R5) and the posthumous child (M14)

In `on_birth`, as the draft states them: (i) a parent dead in this slot or earlier counts by its state at the start of the instant (the father's `house_member` row outlives him; kinship is the log); (ii) a holder who dies this instant still holds at `on_birth`, which runs before `on_death` in the Day/Polity binding; (iii) newborns are processed in ascending PersonId; (iv) "the line" is the line after M7's exclusions; (v) a sortition title has no first or second outside its arbitration instant, so criterion (d) does not apply to it; (vi) `house_member` rows are written for great houses only (every manifest dynasty is a great house at F1). In `on_death` (M14): a house whose last living member died is extinct unless a pregnancy whose father is a member of the house is pending (a new `same_instant` identity read of M2's `person.unborn_father`); the house then waits, and the yearly pass emits `HOUSE_EXTINCT` if the pregnancy ends without a birth, so a posthumous child of a house's last male joins a house that is not yet extinct and `[isolation]`'s "no house is resurrected" holds.

## 8. Recommendation, confidence, D40

**Agnatic membership with (d) and (e), 6/10.** Agnatic is the simplest rule, the back-fill's own, needs no record and no plane-C change, and under T1 it is what the bounded rule reduces to. By itself, at the manifests' values and the frozen readings, it leaves no house standing, but so does every rule except cognatic, and cognatic keeps houses by making everyone noble. The two readings are the levers: (d) is the frozen texts' own primary reading and costs one amendment in this draft; (e) is an amendment of the M2 0.3.0 draft that SPR-015 is building ("she conceives at m_e's table if she or her husband has a house_member row", F6; the back-fill's B6 already notes the case), so its timing is the Architect's call, and until it lands the houses of a T1 world decay at 0.93 per generation under (d) alone (8.2 / 4.0 / 2.5 per house). What lowers the confidence: the model's W_n is a genesis cross-section with old holders; T1's marriage market is thin (a named woman marries only a named man of her court or relation set, so some never marry, which the model ignores and which shrinks every rule's houses further; hypergamy (f) is the structural fix and is P1b); and SPR-030's engine run is the test (`fixture_worlds.md` §6 reports the member share and the living members per house against §3 and §4). Against **bounded** (2/10 as a rule in itself): it adds a record, an always-admitted act class, two templates and a title-state dependence for an inflow that T1 forbids. Against **cognatic** (1/10): saturation.

**D40 sets no trigger** (R8). D40 approved `tfr` 4.9 and pre-registered its revisit on the dynastic fixture world in SPR-030; that revisit runs there regardless of D56. What this analysis adds is that under agnatic membership the commoner NRR, 1.0341 at `tfr` 4.9, is the world's rate (+0.114% a year: `twin_vales` +40% over 300 years, `kingdom_10k` ×3.1 uncapped and 10,433 capped), where under the frozen rule it was the elite parameters'.

## 9. The amendment (what changed where)

| File | Change |
|---|---|
| `specs/systems/next/M7.dynasty.odd.toml` | 0.5.0: ten `[[amends]]` records (`[initialization]`, `tenure.title`, `house_member.house` re-stated to the agnatic rule, INV-M7-06 with A-57 and the posthumous child, and under R3: `submodels.on_birth.summary`, `on_death`, `vocation`, `dormancy`, `commands.Promote.why`, `commands.Demote.why`); a new `[[reads]]` of M2's `person.unborn_father` (M14); no new verb |
| `specs/systems/next/registry_delta.toml` | no M7 entry (the first draft's `HOUSE_JOINED` verb is withdrawn) |
| `specs/systems/lint_odd.py` | deny by default (R3): generic amendment paths for every other field of the frozen slice; ten self-check probes per next version |
| `specs/systems/next/M2.demography.odd.toml` | 26 `[[amends]]` records the deny-by-default lint required, generated from the frozen and next texts by the lint's own functions (`submodels.mortality.summary`, `submodels.apply_death.summary` and `.earns_its_place_by`, `params.tfr.source`, `params.elite_fertility_mult.source`, `fidelity_ladder.F1.representation`, `couplings.T1.minds/out.via`, `couplings.M7.dynasty/out.via`, `purpose.statement`, `purpose.not_yet`, `isolation.expect`, nine `design.*`, four `scales.*`, `verification.prototype`, `system.requirements`); the Architect reads the AMEND lines |
| `specs/systems/next/amendments.lock.toml` | rewritten by `lint_odd.py --next … --write-amendments-lock` (M2 45 paths, M7 10); never typed |
| `bin/check_all.sh`, `python/tests/test_check_all_inventory.py` | thirteen `odd_next_control` cases (one per newly covered path class and the reviewer's `q5_summary`, `q5_why`, `q5_frozen_onbirth`), each failing for its own diagnostic; the M2 self-check pin (43 planted changes, 45 amendments); the annals pins |
| `specs/worldgen/project_population.py`, `golden/population_projection_v0.txt` | v2: the regimes, the levers, the members column, W_n, `--derive-caps`; golden regenerated with `--write` |
| `specs/worldgen/population_synthesis.toml` | B4's and B6's parentheticals name the agnatic rule; no algorithm text, constant or check value changes |
| `specs/annals/annals_v0.md`, `templates_v0.toml`, `selftest_annals_v0.py`, `fixtures/gen_fixtures.py`, `fixtures/`, `golden/` | the walk is the father's (§4, §16); the first draft's `HOUSE_JOINED` templates and check are withdrawn; the `ashcombe` fixture's Bram is of no house and Col is the in-run-father probe (R7); fixtures and goldens regenerated with the tools |
| `specs/worldgen/fixture_worlds.md` | 0.4.0: §2.2 (A-57 ruled), §3.3 (the regimes and the levers), §5, §6 (the `dormancy_degrees` coupling; the member share and the living members per house as the run's test) |
| `docs/decisions_pending.md`, `CODEX/05_PROJECT/BCK-002_ArchitectBacklog.md` | D56 after D55; A-59 (M11) |
| `CLAUDE.md`, `specs/systems/next/README.md` | the projection's and the lint's descriptions; the deny-by-default rule |

Change class **C1**: no frozen spec, Registry file, baseline or run golden changes. The promotion cost of a verb, had one been added, is the shift of every verb id after it in the name-sorted dictionary (M12), re-recorded in the stream goldens at promotion while the plane-A ladders stay; the draft now adds none.

## 10. The specs table (open questions, the drafter's proposed rulings, confidence)

| # | Question | Proposed ruling | Confidence |
|---|---|---|---|
| Q1 | A-56: the membership rule | Agnatic, with (d) in this draft and (e) as an M2 0.3.0 amendment; the director's D56 | 6/10 |
| Q2 | A-57 | One list; naming wins; a named adult meeting none is demoted; a named minor is never demoted; a minor beyond the degrees is not named at start (§6) | 7/10 |
| Q3 | R4, the vocation reading | Later children (the first-born of each father exempt) is the primary reading and the submodel is amended to it; the frozen submodel's "not first in any title's line" is a sensitivity line of the golden. The two readings straddle replacement for a house's periphery (0.83 against 0.93 with commoner wives; 0.996 against 1.117 with the couple read), so the reading is a design decision, not a detail | 7/10 |
| Q4 | (e), the elite read on the couple | An amendment of the M2 0.3.0 draft: F6's table choice reads the woman's row or her husband's; B6 of the back-fill already names the case. It is the lever that makes houses persist; its timing against SPR-015's build is the Architect's | 6/10 |
| Q5 | **Contract conflict, now closed (R3):** `lint_odd --next` did not pin `[[submodels]]` summaries, `[[commands]]` `why` and the rest | Deny by default in this PR; both drafts' records re-derived with the tool; thirteen rendered controls | 8/10 |
| Q6 | The six edge cases (R5) | As §7 states them, the ruling's defaults | 8/10 |
| Q7 | A separate `membership_degrees` | No: `dormancy_degrees` bounds naming and, through the named core, the house's reach; §4 states the coupling (halving the degree halves W_n) | 7/10 |
| Q8 | Change class | C1 (§9) | 8/10 |
| Q9 | The draft period | No run with M7 exists before SPR-021 builds it, so no chronicle disagrees with the annals' agnatic walk; `annals_v0.md` §16 states it | 9/10 |
| Q10 | **Contract tension, pre-existing:** SIM-9's text ("more than N degrees from any title, claim or office") against M7's dormancy ("of its house") | Keep M7's house-scoped (c) with R6's (d). The amendment text for `docs/03` on the director's D56: "Great-house members more than N degrees from every holder of a title, claim or office of their house, and not first or second in the line of any title, become **dormant** ordinary persons and are re-promoted only by simulation criteria" | 7/10 |
| Q11 | M11, same-instant learning of another system's EventId | BCK-002 A-59: `SUCCEED`'s `triggered` edge to M2's `DEATH` of the same instant needs M7 to learn M2's EventId within the instant, and no engine path exists; SPR-021's plan decides the mechanism (a draft-local cause as SPR-011 ruling 17, or the death command carrying its EventId) | 8/10 |
| Q12 | M10, a typed payload | Without a new verb nothing folds into A-55; A-55 already lists the house on `HOUSE_EXTINCT` | 9/10 |
| Q13 | Whether the fixture worlds' golden should move | No (§6) | 9/10 |

## 11. D56, the director item (the row in `docs/decisions_pending.md`)

**Question.** How do great houses persist in a T1 world, and which house does a newborn belong to (BCK-002 A-56)? The membership rule is one of four levers.

**Recommendation: agnatic membership with vocations for later children and the elite fertility read on the couple (60%).** Numbers, costs and alternatives in the row.

**Default if unanswered.** The 0.5.0 draft as it stands (agnatic membership, the later-children reading); the M2 amendment waits for the word.

## 12. Round 1 of PR #79's review: ruling to change

| Ruling | Change |
|---|---|
| R1 | §2 (the regimes), §3 (both regimes), §4 (the levers), §5 per regime, §8 re-derived from the T1 regime; the projection v2 |
| R2 | Moot: the draft adds no verb; the bounded rule's requirement (an always-admitted act class) is recorded in §2 for the director's alternative |
| R3 | `lint_odd.py` deny by default; both drafts' records; thirteen controls; §9, Q5 |
| R4 | The vocation submodel amended to later children; the sensitivity lines; §1 fact 3, Q3 |
| R5 | §7; the `on_birth` summary |
| R6 | Criterion (d) widened; the Promote `why`; Q10's SIM-9 text |
| R7 | Moot for the record; the annals' walk crosses in-run rows and the `ashcombe` fixture carries the in-run-father probe (Col) |
| R8 | D56 reworded with the golden's numbers, costs and alternatives, after D55; D40 sets no trigger |
| M9 | The header's stale line; no envelope row (no verb) |
| M10 | Q12 |
| M11 | A-59; Q11 |
| M12 | §9's promotion note |
| M13 | `on_birth` promotes on any criterion |
| M14 | §7; the `person.unborn_father` read; INV-M7-06 |
