# M7 house membership and dormancy precedence (BCK-002 A-56, A-57)

*Status: the spec drafter's analysis for the Architect, 2026-10-17, before SPR-021's plan. It carries the `M7.dynasty` 0.5.0 draft (`specs/systems/next/M7.dynasty.odd.toml`), the extended projection (`specs/worldgen/project_population.py`, golden `specs/worldgen/golden/population_projection_v0.txt`), the consequential text in `specs/worldgen/population_synthesis.toml`, `specs/annals/annals_v0.md` and `specs/worldgen/fixture_worlds.md`, and the director item D56 (`docs/decisions_pending.md`). Evidence: REV-SPR-030 F1, R1, R17, R18, R21, R26; REV-SPR-029 R18. The Architect runs an independent review on it and rules.*

> **BLUF.** The frozen M7 0.3.4 gives a newborn "the father's house, else the mother's". That cognatic fallback makes every person of `kingdom_10k` a house member by year 300 and ties the world's growth to `elite_fertility_mult` × (1 − `vocation_holy_orders`), a knife edge at 0.996 per generation (§3). The two alternatives the backlog names behave as follows at the fixtures' values: **agnatic** (the father's house, else none; the back-fill's rule) empties every house in expectation by year 300 to 500, because a member man's commoner wife bears at the commoner rate and holy orders takes a fifth of each generation (0.83 per generation), and it passes a primogeniture seat out of its house about one succession in five (a daughter's son inherits before an uncle); **bounded** (the father's house; else the mother's only when she is named-eligible for it: within `dormancy_degrees` of a holder of her house, first or second in one of its lines, or holding a seat, office or claim) holds every house near its genesis size (6 to 11 living members per house at year 1,000), keeps the common daughter's-son heir in the house, and returns the world to the commoner rate (+0.114% a year, about 10,430 persons capped). **Recommendation: bounded, confidence 7/10**, drafted as M7 0.5.0 with one new chronicle record, `HOUSE_JOINED`, so plane C's house walk equals plane A's row without reading title state. **A-57:** the naming criteria are one list and naming wins; a named adult meeting none is demoted; a named minor is never demoted; a minor beyond the degrees is not named at start. The fixture golden does not move (the reference already read it so). **D40's revisit is not triggered**: the rule change restores `tfr` as the world's driver, which is what D40 pre-registered SPR-030 to check.

## 1. The question and the evidence

| Item | Evidence | What it says |
|---|---|---|
| A-56 | REV-SPR-030 F1, R1 | The projection must include the house-member mechanics; cognatic `on_birth` membership saturates `kingdom_10k` by year ~300; afterwards the population reproduces at (1 − 0.2) × 1.2449 = 0.9959 per generation |
| A-56 | REV-SPR-030 R17 | The capped case runs +4% / −9% over 1,000 years on that knife edge, inside the model's own error |
| A-56 | REV-SPR-029 R18 | The annals' `house_of` is exactly M7's `on_birth`; the agnatic back-fill and the cognatic fallback disagree on a daughter's child |
| A-57 | REV-SPR-030 R18, R21, R26 | A far adult heir (beyond `dormancy_degrees`, no seat) meets both Demote and Promote; the fixture resolves it in favour of naming, pending M7's own precedence |
| D40 | `docs/decisions-log.md` | `tfr` 4.9 approved with a pre-registered revisit on the dynastic fixture world in SPR-030 |

Three facts bound the design space, and the review did not state them:

1. **No succession law reads house membership.** Every program of `specs/registry/effect_language_v0.md` §8 takes its candidates from `lineage(holder, 3)`, `children(holder)` or `court(title)` over the life-course log and the tenure log; `house_member` appears in none. Membership touches succession only through the `vocation` step (members draw holy orders at majority and leave every line) and through whose house the successor is in.
2. **Membership is what three other mechanics key on:** M2 F3/F4's elite fertility (the woman's row), M7's vocations and dormancy (members only), T1's situation sets (`house_member.*`, T1 0.3.3 `[[reads]]`), the annals' house groups and `HOUSE_EXTINCT`.
3. **Under any rule the member population's own reproduction is below replacement at the fixtures' values.** A member woman not in holy orders has NRR 1.2449 daughters; holy orders removes 20% at majority; 0.8 × 1.2449 = 0.996. A member man's commoner wife has no row and bears at 1.0341, so his line reproduces at 0.8 × 1.0341 = 0.83. Houses therefore grow only by recruiting: cognatic recruits every mixed marriage's children (saturation), agnatic recruits nobody (extinction), bounded recruits the children of the daughters near the seat (a fixed point).

## 2. The three rules, stated exactly

| Rule | `on_birth` writes the newborn's `house_member.house` as | Back-fill agrees? | Plane C can derive it from |
|---|---|---|---|
| **Cognatic** (M7 0.3.4, frozen) | the father's house; else the mother's; else none | On every back-filled row, yes (no daughter of a house bears in the back-fill, SPR-019 D-07); the rules differ on an in-run child of a daughter and a houseless father | the life-course rows: the father-then-mother walk (annals §4, R18) |
| **Agnatic** (the back-fill's rule, `population_synthesis.toml` INV "every house member other than the root has a father of the house") | the father's house; else none | Yes, by construction | the life-course rows: the father walk |
| **Bounded** (M7 0.5.0 draft) | the father's house; else the mother's **when she is named-eligible for it at the birth instant** (she holds a title, office or claim of the house; or is within `dormancy_degrees` of a holder of the house, a member holding one; or is first or second in the line of a title a member of the house holds); else none. Evaluated by M7 on its own columns and `kin(p, n)`, never on T1's `person.named` (lagged, false without a mind). Recorded by `HOUSE_JOINED` (actor = the mother, target = the newborn, a `triggered` edge to the `BIRTH`) | Yes: on the back-fill's rows the mother branch never fires | the life-course rows plus the `HOUSE_JOINED` records: the father walk, else the record, else none |

"Bounded by degree from a title's line", the backlog's wording, is the third rule: the mother's reach is the dormancy neighbourhood of the seat and the head of the line, with no new parameter (`dormancy_degrees` bounds both naming and membership; a separate `membership_degrees` would be a Registry change for no measured gain, §10 Q7). A rule bounded genealogically alone (one link through a daughter, then fathers only) was computed and rejected: if a daughter's son passes the house to his children it saturates at 1.36 per generation (about 400 years); if he does not, it decays like agnatic. The stability of the bounded rule comes from the inflow being capped by the named set, which is constant, while the loss is proportional to the house.

## 3. The projections (`project_population.py`, golden `population_projection_v0.txt`)

The model is REV-SPR-030 R1's two-type generational recursion (unchanged for the cognatic rule: every cognatic line of the old golden reproduces to the person), extended with the membership rule and, for the bounded rule, the named member women of childbearing age per house, W_n, measured at genesis by the fixture reference (`--derive-caps`: twin_vales 5, 5 and 4 named member women aged 15–49 over five houses at seeds 1–3, kingdom_10k 8 over twelve houses at seed 1), pinned as 1.00 and 0.67 per house with a doubled sensitivity (a mid-reign holder is younger than the genesis holders, who are 36 to 74). Limits as the script states them; the model says what the mechanics imply to within tens of per cent.

**`twin_vales`, 300 years, uncapped** (population; members; member share):

| Rule | Year 100 | Year 200 | Year 300 | Peak population | Least members |
|---|---|---|---|---|---|
| Cognatic | 1,664; 401; 24% | 1,764; 1,520; 86% | 1,751; 1,751; 100% | 1,767 (year 206) | 56 (year 0) |
| Agnatic | 1,675; 30; 1.8% | 1,875; 16; 0.8% | 2,100; 8; 0.4% | 2,100 (year 300) | 8 (year 300), 1.7 per house |
| Bounded | 1,674; 57; 3.4% | 1,869; 57; 3.1% | 2,087; 57; 2.7% | 2,087 (year 300) | 56 (year 0), 11 per house |

**`kingdom_10k`, 1,000 years, uncapped:**

| Rule | Year 300 | Year 500 | Year 1,000 | Peak population | Least members |
|---|---|---|---|---|---|
| Cognatic | 12,598; 12,440; 99% | 12,260; 100% | 11,440; 100% | 12,607 (year 294) | 117 (year 0) |
| Agnatic | 14,062; 17; 0.1% | 17,670; 5 | 31,288; 0 | 31,288 (year 1,000) | 0 (year 1,000) |
| Bounded | 14,042; 96; 0.7% | 17,620; 93 | 31,121; 92; 0.3% | 31,121 (year 1,000) | 92 (year 1,000), 7.7 per house |

**`kingdom_10k`, 1,000 years, capped at `density_capacity` 10,000:**

| Rule | Year 300 | Year 500 | Year 1,000 | Peak population | Least members |
|---|---|---|---|---|---|
| Cognatic | 10,017; 9,892; 99% | 9,731; 100% | 9,080; 100% | 10,415 (year 88) | 117 (year 0) |
| Agnatic | 10,433; 13 | 10,433; 3 | 10,434; 0 | 10,434 (year 1,000) | 0 (year 1,000) |
| Bounded | 10,430; 82 | 10,430; 78 | 10,430; 77; 0.7% | 10,430 (year 1,000) | 77 (year 1,000), 6.4 per house |

**Sensitivities** (in the golden): cognatic at 1.0 / 0.2 collapses to 108 persons, at 1.2 / 0.0 explodes to 6.4 million, at 1.0 / 0.0 grows to 31,356 (REV-SPR-030's lines, unchanged); bounded with W_n doubled holds 106 members on `twin_vales` (5.1%) and 184 on `kingdom_10k` (0.6%) at the horizon, so the fixed point scales with the named women and stays two orders of magnitude under saturation.

**Reading.** Cognatic: the houses absorb the world; the world then reproduces at the member rate, 0.996, and the capped 10k world drifts to −9%. Agnatic: the world grows at the commoner rate and the houses decay at 0.83 per generation; the mean field has 1.7 living members per house on `twin_vales` at year 300 and none on `kingdom_10k` by year 750. Real houses are stochastic branching processes, so some survive by luck and most die earlier; `HOUSE_EXTINCT` would be the common event of a 1,000-year run and the named set would fall toward the holders alone. Bounded: the houses sit at about their genesis size (the fixed point is about 12 W_n persons per house: the inflow of the named daughters' children, 2 W_n per generation, over the loss 0.17), the world grows at the commoner rate, and the capped 10k world sits at 10,430, where the commoner NRR times the density term is 1. Under the agnatic and bounded rules the G1a world is 10,000 persons as written, with no knife edge.

## 4. Named-actor counts

At genesis the three rules give the same named set, because the back-fill writes the same rows under all three (no daughter bears before the epoch): 33 / 35 / 26 on `twin_vales` at seeds 1–3 and 65 / 71 on `kingdom_10k` at seeds 1–2 under the full reading (`golden/fixture_worlds_v0.txt`, unchanged by this PR). Over a run, from `fixture_worlds.md` §3.2's arithmetic (the named set is the holders' two-step member neighbourhoods, plus each title's heir and second, plus claim holders; kinship in parent-or-child steps):

| Rule | A holder's two-step member neighbourhood (mid-reign, e0 30, TFR 4.9) | Expected named on `kingdom_10k` at any instant | Bound ≤ 500 |
|---|---|---|---|
| Cognatic | parents, wife (a co-parent), siblings, children, grandchildren: all members after saturation, about 7 to 11 | 100 to 200 (§3.2 as written; the wife and mother add about 2 per holder) | holds |
| Agnatic | father, paternal grandfather, siblings, children, sons' children: about 5 to 8; the wife, mother and daughters' children are not members | 80 to 150 while the houses last; falls toward the 12 holders plus heirs and claimants as the houses empty (§3) | holds, vacuously in the long run |
| Bounded | agnatic's plus the daughters' children (the daughters are one step from the holder, so their children are members): about 6 to 9; the wife and mother are not members unless of a house of their own | 90 to 170 | holds |

The dormancy counts (Demote and Promote per year) are reported, never targets; under the bounded rule a succession re-centres the neighbourhood as under the frozen rule, with fewer members outside it to demote.

## 5. The effect on succession eligibility, by Registry law

No law reads membership (§1), so the direct eligibility of a candidate is identical under the three rules. Three indirect effects differ:

| Law (sex rule) | Through holy orders (members draw at majority and leave every line) | Through the successor's house | Through the line's length |
|---|---|---|---|
| `primogeniture`, `ultimogeniture` (male-preference cognatic, by representation) | Cognatic: every candidate is a member once the world has saturated, so every line loses 20% of each generation. Agnatic and bounded: only member candidates draw; a non-member on a cognatic line (a daughter's son of no house) never does | A daughter's line inherits before the holder's brother (`up` = 0 before `up` = 1). With about 1.2 surviving sons per holder (Poisson), a holder leaves no son's line but a daughter's about one succession in five. Cognatic: the daughter's son is a member, the seat stays in the house. Agnatic: he is a member of his father's house or none; his sons are not members; the title's house has no holder from then on (every member dormant-eligible, no `HOUSE_EXTINCT`), with no cadet house to catch it (F2). Bounded: the holder's daughters, granddaughters and sisters are within two steps, so their children are members and the seat stays; it leaves only through a line beyond the degrees (a niece's son, when every nearer line is gone) | Unchanged: the line is the pruned lineage, 7 to 22 long at genesis |
| `partition` (children only) | As above | The daughter case at the same rate (daughters are eligible after sons) | Unchanged |
| `seniority`, `tanistry` (agnatic) | The candidates are the holder's agnates, members under every rule (the father's house), so holy orders shortens the line identically: tanistry's 1-to-5-long lines empty whenever the house's adult men are few (`fixture_worlds.md` §2.3), under agnatic faster, since the house itself shrinks | A member holder's agnates are members: the seat never leaves the house under any rule | Agnatic: the house's male line shrinks at 0.83 per generation, so the empty-line route to `INTERREGNUM_CRISIS` becomes the common case within 300 years |
| `elective`, `sortition` (the court) | Court members who are house members draw; the others do not | The court is the vassal holders and office holders of any house or none: membership is irrelevant | Unchanged |

So the rule matters most for the cognatic laws, where agnatic membership breaks the dynasty's continuity at the seat about once in five successions, and for tanistry, where agnatic membership empties the derbfine.

## 6. What the annals would say

The annals group events by house through the house walk (`annals_v0.md` §4, §5), and a house annal opens with the keyframe's prologue.

- **Cognatic.** Every in-run birth joins a house once its father or mother is a member; by year 300 of `kingdom_10k` every birth, marriage and death falls in one of the twelve house groups, and each house annal tends to the year annal. `HOUSE_EXTINCT` is never told. The walk reads only life-course rows.
- **Agnatic.** A house annal is the patriline: the root's male-line descendants and the in-married wives' events through their husbands. A daughter's marriage is in her house's annal (she is in its cast), her children's births are there only as her acts (the mother is `BIRTH`'s actor), and the children themselves belong to no house. `HOUSE_EXTINCT` is told for most houses within 400 years. The walk stops at the father.
- **Bounded.** The patriline plus the children of the daughters near the seat, each marked by one sentence, `house.joined.after`: "On day 2 of month 2, following the birth of p-3374, the child was received into the house of the second daughter of Hugh Thorne, the mother." (the `ashcombe` fixture's probe, Bram, now joins House Thorne through a `HOUSE_JOINED`). `HOUSE_EXTINCT` is rare. The walk reads the life-course rows and the record (`joined/<event>`); nothing in plane C evaluates the criterion.

## 7. A-57: the precedence rule, the named set and the dormancy counts

The frozen `dormancy` submodel lists Demote's conditions (an adult, no seat, office or claim, no vow, beyond `dormancy_degrees` of every holder of its house) and Promote's (within the degrees, a seat, office or claim, "or is a ruling house's heir"), and `on_birth` names a newborn first or second in a line. A far adult heir meets both. The 0.5.0 draft states, in `dormancy` and in the amended INV-M7-06:

1. **One list of naming criteria** for a great-house member (a holder of the house is a member of it who holds a title, office or claim): (a) it holds a title, office or claim; (b) an active vow (S7, P1b); (c) within `dormancy_degrees` of a holder of its house; (d) first or second in the line, by the title's law, of a title a member of its house holds ("a ruling house's heir and second"; a court law's line is the court's order).
2. **Naming wins.** A member meeting any criterion is named; a far adult heir or second is named and not dormant-eligible.
3. **Age enters Demote only.** A named adult meeting no criterion is dormant-eligible (Demote); a named minor is never demoted. Promote goes to a member not named that meets a criterion, at a pass or at `on_birth`.
4. **The named set after a pass** is the members meeting a criterion plus the named minors. **At world start** the pass names exactly the members meeting a criterion, so a minor beyond the degrees who is not in a line's head is not named at start (the question `fixture_worlds.md` §6 left to SPR-021; the reference counts those minors separately, so either answer was checkable: this one leaves the golden as it is). The dormancy counts are the Demotes and Promotes sent, reported and never a target.

The fixture reference computed the full reading as "near or in the line's head" and the dormant-eligible adults as the far adults outside it (`house_report`), which is rule 2; `golden/fixture_worlds_v0.txt` is therefore unchanged, `--tier pr` passes, and `--control dormancy-one` still fails for its own reason. A-56's rule does not touch genesis either (§4).

## 8. Recommendation, confidence, D40

**Bounded, 7/10.** The case: it is the only rule under which a 1,000-year dynastic world keeps houses that are houses (6 to 11 living members each, the patriline and the daughters of the seat), keeps the daughter's-son heir in the house under the cognatic laws, returns the world's growth to `tfr` and M2 rather than to two elite parameters, and keeps the named set bounded by the degrees as the fixture text already argues. The cost is one event kind, `HOUSE_JOINED` (a verb in the delta, an `[[emits]]` entry, two templates and a label, the fixture's probe, every annals golden regenerated), and a criterion `on_birth` evaluates on M7's own columns, which M7 computes for the newborn's naming already. What lowers the confidence: the member population is sub-replacement by itself (§1 fact 3), so the houses live on the daughters' inflow, and the fixed point rests on W_n, a genesis cross-section measured once; the engine's run (SPR-030's member share, `fixture_worlds.md` §6) is the test. Against **agnatic** (the runner-up, 2/10): it is the simplest rule and already the back-fill's, but at the manifests' values it kills the houses the saga is about within the V-INV-12 horizon and passes the seat out of the house, and making it viable would mean lowering `vocation_holy_orders` to the point of inertness or amending M2's elite read to the husband's row while SPR-015 builds it. Against **cognatic** (1/10): it keeps houses by making everyone noble, and D40's `tfr` stops mattering.

**D40's revisit is not triggered.** D40 approved `tfr` 4.9 and pre-registered a revisit on the dynastic fixture world in SPR-030. Under the frozen rule the world's long-run growth was the elite parameters', so SPR-030's run would have tested `elite_fertility_mult` and `vocation_holy_orders`, not `tfr`; under the bounded (or agnatic) rule the commoner NRR, 1.0341 from the F1 chain at `tfr` 4.9, is the world's rate again: `twin_vales` +39% over 300 years, `kingdom_10k` ×3.1 uncapped over 1,000 years and 10,430 capped. That is V-SF-01's middle (r = +0.114% a year), so the pre-registered check stays where it is, in SPR-030's engine run, and nothing here asks the director to reopen `tfr`.

## 9. The amendment (what changed where)

| File | Change |
|---|---|
| `specs/systems/next/M7.dynasty.odd.toml` | 0.4.0 → 0.5.0. `[[amends]]` `owns.house_member.house.meaning` re-stated (the bounded rule, the record, the back-fill's agreement); a new `[[amends]]` `invariants.INV-M7-06.statement` (A-57's precedence; in-run rows follow the rule). A new `[[emits]]` `HOUSE_JOINED` (roles actor, target). Free text (not lint-pinned, §10 Q5): the `on_birth` and `dormancy` submodel summaries, the Promote and Demote `why`, the header |
| `specs/systems/next/registry_delta.toml` | `[verbs] HOUSE_JOINED` (act class `bond`, tellability 104, emitted by M7) |
| `specs/systems/next/amendments.lock.toml` | rewritten by `lint_odd.py --next … --write-amendments-lock` (4 M7 paths); never typed |
| `specs/worldgen/project_population.py`, `golden/population_projection_v0.txt` | the three rules, the members column, W_n, `--derive-caps`; golden regenerated with `--write` (v1; the cognatic lines reproduce v0's numbers) |
| `specs/worldgen/population_synthesis.toml` | B4's and B6's parentheticals name the draft's rule and why the back-fill agrees; no algorithm text, constant or check value changes (the gate passes) |
| `specs/annals/annals_v0.md`, `templates_v0.toml` (0.3.0), `selftest_annals_v0.py`, `fixtures/gen_fixtures.py`, `fixtures/`, `golden/` | §4's walk reads `HOUSE_JOINED`; §3, §5, §8, §12 and a new §16; two templates and a label; the reference's `house_of` and a possible-history rule for the record; the probe's record; fixtures and goldens regenerated with the tools; `check_all`'s template count 29 → 31 |
| `specs/worldgen/fixture_worlds.md` | 0.4.0: §2.2 (A-57 ruled in the draft), §3.3 (the three rules), §5, §6; the fixture golden unchanged |
| `docs/decisions_pending.md` | D56 |
| `bin/check_all.sh` | the template-lint count 29 → 31; the `odd_next_control` cases `algorithm`, `rng` and `lock` copy the M7 draft beside the M2 one, so that the delta's verb does not fail the frozen M7 for a reason that is not the control's own (each still fails for its own diagnostic alone) |
| `CLAUDE.md`, `specs/systems/next/README.md` | the projection's and the draft's one-line descriptions |

Change class **C1**: no frozen spec, Registry file, baseline or run golden changes; the frozen M7 stays 0.3.4 and the verb lives in the delta until promotion. The annals goldens and the projection golden move, regenerated by their tools, and neither is a run baseline.

## 10. The specs table (open questions, the drafter's proposed rulings, confidence)

| # | Question | Proposed ruling | Confidence |
|---|---|---|---|
| Q1 | A-56: the membership rule | Bounded, as §2 and the draft state it; the director's D56 | 7/10 |
| Q2 | A-57: Demote against Promote for a far adult heir | One list of criteria; naming wins; a named adult meeting none is demoted; a named minor is never demoted; a minor beyond the degrees is not named at start (§7) | 7/10 |
| Q3 | Whose "named" the mother's criterion reads | M7's own criteria on its own columns and `kin(p, n)` at the birth instant, never T1's `person.named` (lagged; false in M7's isolation run; deferred by T1 while a break is pending), so membership is M7's alone and the isolation run is well-defined (agnatic in a world without titles) | 8/10 |
| Q4 | How plane C learns a membership through the mother | A `HOUSE_JOINED` record from `on_birth`, read by the annals' walk. The alternative, reconstructing titles and claims from `SUCCEED`, `DEPOSED`, `SEAT_GRANTED` and `CLAIM_*` in plane C and evaluating the lines with the law programs, would put a second M7 in the annals and breach §3 rule 1 | 7/10 |
| Q5 | **Contract conflict (a lint gap):** `lint_odd.py --next` pins `[[owns]]` meanings, algorithm steps, `[[rng]]` uses, invariants, schedule steps, `[[emits]]` whens, `[initialization]` and `[[params]]`, but not `[[submodels]]` summaries or `[[commands]]` `why`, where M7's mechanics (on_birth, dormancy, vocation, arbitrate) actually live; this draft changes two summaries and two `why` texts with no `[[amends]]` possible | Report, not fix here: add `submodels.<name>.summary` and `commands.<command>.why` to the amendable paths with a planted control (`tests/lint_fixtures/odd_next`), before SPR-021 promotes 0.5.0; then re-derive this draft's records with the tool. A C1 change to the lint | 8/10 |
| Q6 | The member population is sub-replacement by itself at the fixtures' values (0.83 per generation with commoner wives), so under any non-saturating rule the houses live on recruitment | Leave M2's elite read (the woman's row) and the fixtures' 1.2 / 0.2 as they are now; SPR-030's run reports the member share and the house sizes against §3; if the houses shrink faster than the projection, the levers are `vocation_holy_orders` (a manifest value) and, as an M2 amendment after SPR-015 lands, the elite level for a member's wife | 6/10 |
| Q7 | A separate `membership_degrees` parameter | No: `dormancy_degrees` bounds both naming and membership, one lever, no Registry change | 7/10 |
| Q8 | Change class | C1 (§9) | 8/10 |
| Q9 | The draft period: an engine built on the frozen M7 writes cognatic rows while the annals' walk now follows the father and the record | No run with M7 exists before SPR-021 builds it (M7 is `missing; drafting` in PRJ-002), so the gap is theoretical; `annals_v0.md` §16 states it and the promotion closes it | 9/10 |
| Q10 | The verb's act class and tellability | `bond`, 104 (between BIRTH's 96 and MARRIAGE's 112): design values, one Registry line either way | 6/10 |
| Q11 | **Contract tension, pre-existing:** SIM-9's text ("more than N degrees from any title, claim or office") against M7's dormancy ("of its house") | Keep M7's house-scoped reading (the draft states it): the requirement's "any" would name every member within two steps of any holder in the world; report to the Architect for docs/03's wording | 7/10 |
| Q12 | Whether the fixture worlds' golden should move for A-57 | No: the reference already applied naming-wins (`house_report`: full = near or in the line's head; dormant-eligible = far adults outside it), `--tier pr` passes unchanged and `dormancy-one` fails as before. The brief's "golden lines regenerated" is satisfied by showing nothing moves | 9/10 |

## 11. D56, the director item (the row in `docs/decisions_pending.md`)

**Question.** House membership in M7 (BCK-002 A-56): cognatic, agnatic or bounded? Whether a child of a house's daughter and a man of no house belongs to her house decides the shape of every dynastic history.

**Recommendation: bounded (70%).** The father's house; else the mother's only when she is named-eligible for it (within `dormancy_degrees` of a holder of her house, first or second in one of its lines, or holding a seat, office or claim); else none. Houses stay near their genesis size, a daughter's son who inherits stays in the house, the world grows at `tfr`'s rate, and one new chronicle record tells the annals which house such a child joined.

**Default if unanswered.** The bounded rule as drafted in M7 0.5.0; SPR-021 builds it.
