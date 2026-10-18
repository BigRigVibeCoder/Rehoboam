# M7 house membership and dormancy precedence (BCK-002 A-56, A-57)

*Status: the spec drafter's analysis for the Architect, 2026-10-17; rounds 1 and 2 of PR #79's independent review applied (R1–R14, N1–N15; §12 and §13 map each to its change). It carries the `M7.dynasty` 0.5.0 draft (`specs/systems/next/M7.dynasty.odd.toml`), the projection (`specs/worldgen/project_population.py`, golden `specs/worldgen/golden/population_projection_v0.txt`), the deny-by-default next lint (`specs/systems/lint_odd.py`, R3 and N4), the consequential text in `specs/worldgen/population_synthesis.toml`, `specs/annals/annals_v0.md` and `specs/worldgen/fixture_worlds.md`, and the director item D56 (`docs/decisions_pending.md`). Evidence: REV-SPR-030 F1, R1, R17, R18, R21, R26; REV-SPR-029 R18; PR #79's review, rounds 1 and 2. **2026-10-18: D56 is approved** (the director, 2026-10-17: agnatic membership, (d) and (e)). (e) is the amendment of the M2 0.3.0 draft that BCK-002 A-62 carries (`[algorithm.fertility_f1]` F3–F6: the elite table is read on the woman's `house_member` row or her husband's); it moves the back-fill's B6 (every back-filled wife draws `c_i^e`), so the genesis the projection starts from moved too (`kingdom_10k` seed 1: 185 living members and W_n 0.92 per house, against 117 and 0.67; `twin_vales` seed 1: 53 and 0.80, against 56 and 1.00), and every projected number below is re-read from the regenerated golden, whose baseline reading is now D56's.*

> **BLUF.** The frozen M7 0.3.4 gives a newborn "the father's house, else the mother's". In every regime that cognatic fallback makes every person of `kingdom_10k` a house member by year 300 and ties the world's growth to the elite parameters (§3). **The regime decides the rest.** In M7's isolation run (T1 stubbed, everyone on M2's market) a fallback bounded to the daughters near the seat holds a house at its genesis size; but both fixture worlds register T1, and there the named marry the named (T1's `marry` takes unmarried named adults; M2's N3/N4 exclude them), so the bounded fallback is idle and **agnatic and bounded membership are the same rule** (at the frozen readings, 2.9 living members per house on `twin_vales` at year 300, 1.8 on `kingdom_10k` uncapped at year 1,000, 1.0 capped). Under T1 a house is a named core, bounded by `dormancy_degrees`, and a dormant periphery that reproduces at (1 − v) × NRR of a member man's wife; at the manifests' values with the frozen vocation reading that is 0.83 per generation and the periphery dies. **D56 is therefore "how do great houses persist in a T1 world"**, with membership one lever among four (§4): (d) vocations for later children only, an elder child of the same father alive at the draw, the frozen texts' primary reading (the M2 chain with a birth counter puts holy orders' share of a cohort at 0.116 of a commoner mother's children and 0.128 of a member mother's: 0.91 per generation); (e) M2's elite fertility read on the couple, so a member man's commoner wife bears at the elite level (1.2449); (f) hypergamy, T1's marry widened to the polity's unnamed, which lowers house growth on its own; and the rule. Only (d) with (e) makes houses persist: 1.086 per generation, about 65 living members per house on the capped 10k world at year 1,000, 7% of the world and growing at 0.3% a year (257 per house uncapped). **Recommendation (6/10), adopted by D56 (2026-10-17): agnatic membership** (the father's house, else none; the back-fill's rule; no record, no plane-C change), **with (d) in the M7 draft and (e) as an amendment of the M2 0.3.0 draft** (BCK-002 A-62, the M2 draft's F3–F6: the woman's row or her husband's). The 0.5.0 draft carries agnatic membership, (d), A-57's precedence (one list; naming wins; a named minor is never demoted; criterion (a) names any holder, member or not, N7; criterion (d) widened to any title's heir, R6), the birth instant's edge cases (R5) and no row for a child born into an extinct house (N6). **D40 sets no trigger**: its revisit runs in SPR-030 regardless. **A-57 moves no fixture golden**; (e) does, through the back-fill's B6 (A-62 regenerated `fixture_worlds_v0.txt`, `population_synthesis_v0.txt` and `crn_pairing_v0.txt` with their tools); the annals' walk is the father's, through any number of in-run rows (R7's probe).

## 1. The question and the evidence

| Item | Evidence | What it says |
|---|---|---|
| A-56 | REV-SPR-030 F1, R1 | Cognatic `on_birth` membership saturates `kingdom_10k` by year ~300; afterwards the population reproduces at (1 − 0.2) × 1.2449 = 0.9959 per generation |
| A-56 | REV-SPR-030 R17 | The capped case runs +4% / −9% over 1,000 years on that knife edge |
| A-56 | REV-SPR-029 R18 | The annals' `house_of` is exactly M7's `on_birth`; the agnatic back-fill and the cognatic fallback disagree on a daughter's child |
| A-57 | REV-SPR-030 R18, R21, R26 | A far adult heir (beyond `dormancy_degrees`, no seat) meets both Demote and Promote |
| D40 | `docs/decisions-log.md` | `tfr` 4.9 approved; a revisit pre-registered on the dynastic fixture world in SPR-030 |
| Round 1 | PR #79's review M1 | Both fixture worlds register T1; T1 marries the named among the named; a bounded fallback's mother is named, so the fallback is idle |
| Round 2 | PR #79's review N2 | The later-children share must come from the M2 chain, under the ruled reading (an elder child of the same father alive at the draw), not from a first-born share |

Three facts bound the design space:

1. **No succession law reads house membership.** Every program of `specs/registry/effect_language_v0.md` §8 takes its candidates from `lineage(holder, 3)`, `children(holder)` or `court(title)` over the life-course log and the tenure log. Membership touches succession only through the `vocation` step (members draw holy orders and leave every line) and through whose house the successor is in.
2. **Membership is what three other mechanics key on:** M2 F3/F4's elite fertility (the woman's row), M7's vocations and dormancy (members only), T1's situation sets (`house_member.*`), the annals' house groups and `HOUSE_EXTINCT`.
3. **A house's own reproduction depends on two readings, not on the membership rule.** A member woman not in holy orders has NRR 1.2449; a member man's commoner wife has no row and bears at 1.0341 under the frozen reading (the elite read on the woman's row), at 1.2449 under D56 (e) (the read on the couple). Holy orders removes v of each cohort at majority. Under the frozen submodel's reading ("not first in any title's line": every other member draws) v = 0.2. Under the ruled reading (R4, N2, N5: a later child is one with an elder child of the same father alive at the draw, so the father's eldest living child is exempt, as is a child first or second in a line) the share comes from the M2 F1 widowhood chain extended with a birth counter (`project_population.py`, `counter_chain`, `gap_distribution`, `exempt_share`): a commoner woman born bears 2.120 children with probability 0.505 of bearing at all, a member woman 2.552 and 0.511; the j-th child is exempt when every elder sibling is dead at its majority, with l(16) = 0.55 and the inter-birth gaps from the chain, which gives v = 0.116 for a commoner mother's children and 0.128 for a member mother's (the reviewer's own extension gave 0.121 and 0.132; the difference is the gap convolution's independence approximation). The literal reading, the eldest by birth tick dead or alive, gives 0.152 and 0.160 (the first-born share of births, 0.505 / 2.120 = 0.238). So the periphery of a house reproduces at (1 − v) × NRR_wife: 0.83 (frozen, woman), 0.91 (ruled, woman), 0.996 (frozen, couple) or **1.086 (ruled, couple: D56's reading)**. Everything below follows from which side of 1 that product falls.

## 2. The three rules and the two regimes

| Rule | `on_birth` writes the newborn's `house_member.house` as | Back-fill agrees? | Plane C derives it from |
|---|---|---|---|
| **Cognatic** (M7 0.3.4, frozen) | the father's house; else the mother's; else none | On every back-filled row (no daughter of a house bears there, SPR-019 D-07); the rules differ on an in-run child of a daughter and a houseless father | the father-then-mother walk (annals §4 as R18 had it) |
| **Agnatic** (M7 0.5.0 draft; the back-fill's rule, `population_synthesis.toml` INV "every house member other than the root has a father of the house") | the father's house when that house is not extinct; else none (a child born to the last male of an extinct house gets no row, so no house is resurrected: N6) | By construction | the father walk through any number of in-run rows |
| **Bounded** (PR #79's first draft) | the father's house; else the mother's when she is named-eligible for it at the birth (a seat, office or claim; within `dormancy_degrees` of a holder of her house; first or second in a line), evaluated on M7's own columns; else none | On the back-fill's rows the mother branch never fires | a `HOUSE_JOINED` record (actor the mother, target the newborn), which would need its own act class with a `tellable_up_to` ceiling of 4294967295 so that it is always admitted under DATA-5/D38 (R2), two templates and a label, and the annals' walk reading it |

**The regimes** (R1). *Isolation*: M7's `[isolation]` run, T1 stubbed, nobody named, everyone on M2's market (N3/N4: random within the settlement). *T1 registered*: the saga world, both fixtures included. The named core (M7's criteria, a set bounded by `dormancy_degrees`) marries within itself through T1's `marry` decision (T1 0.3.3 `decide`: "unmarried named adults of the other sex … in the actor's court or relation set"), M2's N3/N4 exclude the named, and the un-named members marry on M2's market. A named-eligible woman is a named woman, so under T1 the bounded rule's mother branch fires only for a woman who became eligible after an M2 marriage (which the model puts at zero) or under hypergamy (f). Under T1, bounded = agnatic, with one difference N7 closes: a periphery woman who marries a commoner while un-named and later becomes heir has a houseless heir, who under the bounded rule would have been a member; under the agnatic rule he succeeds houseless and is named by his seat (criterion (a) names any holder, member or not).

## 3. The projections (`project_population.py` v3, golden `population_projection_v0.txt`)

The model is REV-SPR-030 R1's two-type generational recursion, in persons, extended with the regime, the rule and the levers (the script's docstring states the recursion and its limits; the cognatic lines in isolation reproduced the pre-v2 golden to the person). The named core is 2 H W_n persons, W_n the named member women of childbearing age per house measured at genesis by the fixture reference (`--derive-caps`: 0.80 on `twin_vales`, 0.92 on `kingdom_10k`, on the genesis A-62 regenerated; 1.00 and 0.67 before it), with a doubled sensitivity. **The golden's baseline reading is D56's** (T1 registered, agnatic, (d), (e)): `twin_vales` reaches 2,311 persons and 123 living members (24.6 per house, 5.3%) at year 300; `kingdom_10k` 42,096 and 3,082 (256.8 per house, 7.3%) uncapped and 10,626 and 781 (65.1 per house, 7.4%) capped at year 1,000. The tables below are alternatives, under the frozen readings (every member draws holy orders, the elite read on the woman, no hypergamy):

**T1 registered, the saga world** (population; living members; per house):

| World, case | Cognatic | Agnatic = bounded |
|---|---|---|
| `twin_vales`, year 300, uncapped | 1,774; 1,774 (100%) | 2,099; 14; **2.9** |
| `kingdom_10k`, year 300, uncapped | 12,323; 12,301 (99.8%) | 14,041; 45; 3.8 |
| `kingdom_10k`, year 1,000, uncapped | 11,185; 11,185 (100%) | 31,210; 22; **1.8** |
| `kingdom_10k`, year 300, capped at 10,000 | 9,982; 9,964 (99.8%) | 10,432; 36; 3.0 |
| `kingdom_10k`, year 1,000, capped | 9,060; 9,060 (100%) | 10,433; 12; **1.0** |

**M7 isolation, T1 stubbed** (the regime of M7's `[isolation]` run and of V-SF-22 only):

| World, case | Cognatic | Agnatic | Bounded |
|---|---|---|---|
| `twin_vales`, year 300, uncapped | 1,758; 100% | 2,101; 8; 1.6 | 2,091; 47; 9.4 |
| `kingdom_10k`, year 1,000, uncapped | 11,098; 100% | 31,248; 0 | 31,019; 127; 10.6 |
| `kingdom_10k`, year 1,000, capped | 9,050; 100% | 10,434; 0 | 10,429; 106; 8.8 |

**Reading.** Cognatic saturates in both regimes and the world then reproduces at the member rate (0.996: the knife edge; capped, −9% by year 1,000). Agnatic and bounded leave the world at the commoner rate (+0.114% a year: 10,433 capped, where the commoner NRR times the density term is 1; no knife edge) and, at the frozen readings, the houses shrink: in isolation the bounded rule's inflow of the daughters' children holds a house at about 10 to 12 W_n persons, but under T1 that inflow does not exist, and a house is its named core plus a periphery that reproduces at 0.83 per generation. The mean field has 2.9 living members per house on `twin_vales` at year 300 and 1.0 on the capped 10k world at year 1,000: the houses are their holders' families and nothing else, `HOUSE_EXTINCT` is the common event, and dormancy has nothing to demote. D56's readings are what changes that (§4).

## 4. The levers under T1 (R1, N2): what makes a house persist

One line each in the golden (60 `lever` lines: the three under T1 with the agnatic rule and (d) + (e) are labelled `baseline`, D56's reading, the rest `alternative`), for the agnatic and the bounded rule; the horizon's living members per house:

| Lever | What it is, and its cost | `twin_vales` year 300 | `kingdom_10k` year 1,000, uncapped | capped |
|---|---|---|---|---|
| none (the frozen readings) | — | 2.9 | 1.8 | 1.0 |
| (d) later children, the ruled reading | the vocation submodel and the Year step exempt a child whose elder siblings by the same father are all dead at the draw, and the first or second in any line (R4, N5): an M7 amendment in this draft; v = 0.116 / 0.128 | 6.3 | 4.2 | 2.8 |
| (d) under the literal reading | the eldest by birth tick, dead included: v = 0.152 / 0.160 (a sensitivity line) | 4.4 | 2.7 | 2.0 |
| (e) the elite read on the couple | **Adopted by D56.** M2 F3–F6 give a member man's wife the elite table (BCK-002 A-62, the amendment of the M2 0.3.0 draft in SPR-015's slice: F6's table choice reads the woman's `house_member` row or her husband's, at the conception draw). It reached the back-fill (B6: every back-filled wife draws `c_i^e`, so every back-filled genealogy moved and `fixture_worlds_v0.txt`, `population_synthesis_v0.txt` and `crn_pairing_v0.txt` were regenerated by their tools) and reopens REV-SPR-015's F6 and the meaning of `elite_fertility_mult` (N8) | 10.2 | 13.4 | 4.3 |
| (f) hypergamy, half the core marrying out | T1's `marry` candidates widened to the polity's unnamed (a T1 change, P1b); agnatic loses the named women's children and the named men's wives bear at the commoner level, so it lowers house growth by itself (0.7 / 0.0 / 0.0); bounded keeps the named women's children (2.6 / 0.8 / 0.4) | — | — | — |
| (d) + (e): **D56's reading, the golden's baseline** | the two readings together: the periphery reproduces at (1 − 0.128) × 1.2449 = 1.086 per generation | **24.6** | **256.8** (7.3% of 42,096) | **65.1** (7.4% of 10,626) |
| (d) + (e), literal | | 16.8 | 70.9 | 21.1 |
| (d) + (f) | | 2.7 (agnatic), 5.8 (bounded) | 0.0, 3.4 | 0.0, 2.2 |
| (d) + (e) + (f) | | 18.0 (agnatic), 24.6 (bounded) | 166.3, 256.8 | 35.0, 65.1 |
| cognatic + (d) | the frozen rule with the ruled reading | 556 (the world) | 10,570 (the world: 127,000 persons) | 922 (the world: 11,067) |

**Reading.** In the model, agnatic and bounded differ only under hypergamy; in the engine they also differ on the post-marriage-eligibility path of N7 (a periphery woman married to a commoner while un-named who later becomes heir), small in members but not in kind, which criterion (a) now closes by naming any holder. Hypergamy is a P1b T1 change and, on its own, shrinks houses further; so under T1 the simpler rule wins (R1). No lever gives a stationary house: the periphery's product (1 − v) × NRR_wife is 0.83, 0.91 or 0.996 (slow decay to the core) or 1.086 (growth at 0.3% a year, 7% of the world by year 1,000). (d) + (e), D56's choice, is the combination that keeps a house a house over the G1a horizon with a periphery for dormancy to act on; its long run is the elite overproduction `elite_fertility_mult`'s source describes (Turchin and Nefedov, R12 §12), at the slow rate the ruled reading gives, and the manifest's multiplier sets it (at 1.0 the product is 0.90 and the houses decay). Under (d) the cognatic knife edge does not survive either (M4): the saturated world grows to 127,000 persons uncapped.

**W_n and `dormancy_degrees`** (Q7, M8 (d)). The named core is the one input the fixtures measure: at `dormancy_degrees` 1 the reference's W_n falls from 0.80 to 0.40 per house on `twin_vales` and from 0.92 to 0.50 on `kingdom_10k` (seed 1), so the core, and with it the isolation fixed point and the T1 periphery's seed, about halve. The parameter bounds both naming and membership's reach; no second parameter is proposed.

## 5. Named-actor counts, eligibility and the annals, per regime

**Named counts.** At genesis the three rules give the same named set (the back-fill writes the same rows): 24 / 29 / 25 on `twin_vales` at seeds 1–3 and 68 / 71 on `kingdom_10k` at seeds 1–2 (`golden/fixture_worlds_v0.txt` as A-62 regenerated it for (e); 33 / 35 / 26 and 65 / 71 before). Over a run the named set is the holders' member neighbourhoods plus heirs, seconds, claimants and, under N7, any holder of a seat, office or claim whatever its house (`fixture_worlds.md` §3.2). Under T1 with agnatic membership a holder's neighbourhood is his father, paternal grandfather, siblings, children and sons' children (his wife and mother are members only of their own houses, if any): 5 to 8 members, 80 to 150 named on `kingdom_10k`, falling toward the 12 holders plus heirs and claimants as the periphery empties under the frozen readings, and rising toward §3.2's 100 to 200 under (d) + (e), D56's reading. Cognatic adds the wife and the mother (about 2 per holder). The bound ≤ 500 holds in every case.

**Eligibility by law.** No law reads membership (§1). Indirectly: (i) holy orders removes members from lines: under cognatic every candidate is a member once the world has saturated; under agnatic only members draw, so a non-member on a cognatic line never does. (ii) The successor's house: under `primogeniture`, `ultimogeniture` and `partition` a daughter's line inherits before the holder's brother, about one succession in five (about 1.2 surviving sons per holder, Poisson). In isolation that son is of no house under agnatic membership and is named by his seat (N7); under T1 his mother is named and married a named man, so he belongs to another great house, and the seat passes between houses by marriage, which the dormancy rule treats as the first house losing its holder (every member dormant-eligible) and the second gaining a seat. Cognatic keeps the seat in the mother's house in both regimes. `seniority` and `tanistry` are agnatic laws: a member holder's agnates are members under every rule and the seat never leaves the house; their derbfine empties as the house shrinks, which under the frozen readings is within 300 years. `elective` and `sortition` take the court, of any house or none.

**The annals.** Cognatic: by year 300 of `kingdom_10k` every birth, marriage and death falls in one of the twelve house groups, each house annal tends to the year annal, and `HOUSE_EXTINCT` is never told. Agnatic: a house annal is the patriline; a daughter's marriage is in her house's annal (she is in its cast), her children's births are there only as her acts, and the children belong to their father's house; under the frozen readings `HOUSE_EXTINCT` is the common event of a 300-year run, under (d) + (e) it is rare. The walk reads the father through any number of in-run rows (the `ashcombe` fixture's Col: Col → Cedric → Osric → the keyframe's House Ashcombe; his mother Nell, a named daughter of House Thorne, passes nothing), never the mother, and no record is needed.

## 6. A-57: the precedence rule, the named set and the dormancy counts

The 0.5.0 draft states, in `dormancy` and in the amended INV-M7-06:

1. **One list of naming criteria.** (a) applies to any person: it holds a title, office or claim, member of a great house or not (N7). The rest to great-house members (a holder of the house is a member of it who holds a title, office or claim): (b) an active vow (S7, P1b); (c) within `dormancy_degrees` of a holder of its house; (d) first or second in the line, by the title's law as `arbitrate` computes it (after M7's exclusions: the deposed and persons in holy orders; a court law's line is the court's order; a sortition title has no first or second outside its arbitration instant), **of any title, whatever house holds it** (R6: an heir apparent is named whatever its house).
2. **Naming wins.** A member meeting any criterion is named; a far adult heir or second is named and not dormant-eligible.
3. **Age enters Demote only.** A named adult meeting no criterion is dormant-eligible (Demote); a named minor is never demoted. Promote goes to a person not named that meets a criterion, at a pass or at `on_birth`, on any criterion (M13).
4. **The named set after a pass** is the persons meeting a criterion plus the named minors. At world start the pass names exactly those meeting a criterion, so a minor beyond the degrees who is not in a line's head is not named at start. The dormancy counts are the Demotes and Promotes sent, reported and never a target.

The fixture reference computed the full reading as "near or in the line's head" and the dormant-eligible adults as the far adults outside it (`house_report`), which is rule 2, so A-57 moves no line of `golden/fixture_worlds_v0.txt` (A-62 regenerated it for (e)), `--tier pr` passes and `--control dormancy-one` still fails for its own reason. R6's widening and N7's (a) change no genesis line: all 39 genesis lines have `outside_house 0`, and every genesis holder is a member.

## 7. The birth instant's edge cases (R5), the extinct house (N6) and the vocation draw (N5)

In `on_birth`, as the draft states them: (i) a parent dead in this slot or earlier counts by its state at the start of the instant (the father's `house_member` row outlives him; kinship is the log); (ii) a holder who dies this instant still holds at `on_birth`, which runs before `on_death` in the Day/Polity binding; (iii) newborns are processed in ascending PersonId; (iv) "the line" is the line after M7's exclusions; (v) a sortition title has no first or second outside its arbitration instant, so criterion (d) does not apply to it; (vi) `house_member` rows are written for great houses only (every manifest dynasty is a great house at F1). A child born to the last male of a house that is extinct at the birth (`house.extinct_at` ≠ 0) gets no row, so no house is resurrected and `on_death`, `house.extinct_at`, the `[[emits]]` and the envelope row stay as frozen (N6; round 1's deferral of `HOUSE_EXTINCT` on M2's `person.unborn_father` is withdrawn, which keeps M7 0.5.0 independent of M2 0.3.0's freeze). The vocation draw (N5), in the submodel and the Year step: a member who reached majority this year, is a later child (an elder child of the same father is alive at the draw), is not first or second in any title's line, and holds no seat, office or claim; at e0 30, where l(16) is 0.55, the first-born is often dead and a younger child is the eldest living, which the chain's 0.116 and 0.128 reflect.

## 8. Recommendation, confidence, D40

**Agnatic membership with (d) and (e), 6/10; adopted by D56 (the director, 2026-10-17).** Agnatic is the simplest rule, the back-fill's own, needs no record and no plane-C change, and under T1 it is what the bounded rule reduces to. By itself, at the manifests' values and the frozen readings, it leaves no house standing, but so does every rule except cognatic, and cognatic keeps houses by making everyone noble. The two readings are the levers: (d) is the frozen texts' own primary reading and costs one amendment in this draft, with the share now derived from the M2 chain; (e) is an amendment of the M2 0.3.0 draft that SPR-015 is building, with the costs §4 lists (the back-fill's spine wives, every back-filled genealogy, the fixture, population-synthesis and CRN goldens, REV-SPR-015's F6 and the meaning of `elite_fertility_mult`); D56 approved it, and it is its own M2 amendment PR with its own review before SPR-015 freezes 0.3.0 (BCK-002 A-62: F3–F6 read the elite table on the woman's row or her husband's, at the conception draw; B6 follows; the three goldens regenerated by their tools). Without it the houses of a T1 world would decay at 0.91 per generation under (d) alone (6.3 / 4.2 / 2.8 per house). What lowers the confidence: the model's W_n is a genesis cross-section with old holders; T1's marriage market is thin (a named woman marries only a named man of her court or relation set, so some never marry, which the model ignores and which shrinks every rule's houses further; hypergamy (f) is the structural fix and is P1b); the exempt share rests on the gap convolution's independence; and SPR-030's engine run is the test (`fixture_worlds.md` §6 reports the member share and the living members per house against §3 and §4). Against **bounded** (2/10 as a rule in itself): it adds a record, an always-admitted act class, two templates and a title-state dependence for an inflow that T1 forbids. Against **cognatic** (1/10): saturation.

**D40 sets no trigger** (R8). D40 approved `tfr` 4.9 and pre-registered its revisit on the dynastic fixture world in SPR-030; that revisit runs there regardless of D56. What this analysis adds is that under agnatic membership the commoner NRR, 1.0341 at `tfr` 4.9, is the world's rate (+0.114% a year: `twin_vales` +40% over 300 years, `kingdom_10k` ×3.1 uncapped and 10,433 capped), where under the frozen rule it was the elite parameters'.

**D38** (N3). `05.2` §4.0 criterion 1 tells a commoner act only while `living_at_start(y)` is at or under 10,000. Every rule but cognatic puts the capped 10k world above the ceiling from its first years (10,433 at the agnatic equilibrium; 31,210 uncapped; the levers at 10,433 to 10,626 capped, D56's baseline the highest), so from then on commoner births, marriages and deaths are told only with a named participant, and D38's premise that the dynastic 10k world is fully told holds only under cognatic-capped (9,060 at year 1,000). D56 says so.

## 9. The amendment (what changed where)

| File | Change |
|---|---|
| `specs/systems/next/M7.dynasty.odd.toml` | 0.5.0: ten `[[amends]]` records (`[initialization]`, `tenure.title`, `house_member.house` re-stated to the agnatic rule with N6's extinct-house clause, INV-M7-06 with A-57, N6 and N7, and under R3: `submodels.on_birth.summary`, `vocation` (N5), `dormancy` (R6, N7), `schedule.Year/Polity.steps.vocation` (N5), `commands.Promote.why` (N7), `commands.Demote.why`); no new read, no new verb |
| `specs/systems/next/registry_delta.toml` | no M7 entry (the first draft's `HOUSE_JOINED` verb is withdrawn) |
| `specs/systems/lint_odd.py` | deny by default (R3, N4): every frozen leaf is compared on a generic path, with the exemption list of `specs/systems/next/README.md` (versions, status and fidelity; the lists that may only grow; an enum's values, append-only by the prefix rule; the algorithm tables' own rule; `[[amends]]`); a repeated key carries its ordinal, appended after; fourteen self-check probes per next version |
| `specs/systems/next/M2.demography.odd.toml` | 26 `[[amends]]` records the deny-by-default lint required (round 1, accepted in round 2); the fully generic walk asked for no more |
| `specs/systems/next/amendments.lock.toml` | rewritten by `lint_odd.py --next … --write-amendments-lock` (M2 45 paths, M7 10); never typed |
| `bin/check_all.sh`, `python/tests/test_check_all_inventory.py` | seventeen `odd_next_control` cases (one per path class, the reviewer's `q5_summary`, `q5_why`, `q5_frozen_onbirth`, and N4's enum code renamed, `drained_by` moved, `from_manifest` edited, a read's `lag` changed), each failing for its own diagnostic, their names and expectations free of backticks or single-quoted (N1); the M2 and T1 self-check pins |
| `specs/worldgen/project_population.py`, `golden/population_projection_v0.txt` | v2: the regimes, the levers, the members column, W_n, `--derive-caps`, the M2 chain with a birth counter and the inter-birth gaps for the ruled and the literal vocation readings (N2), one geometric interpolation for population and members (N12); golden regenerated with `--write` |
| `specs/worldgen/population_synthesis.toml` | B4's and B6's parentheticals name the agnatic rule; no algorithm text, constant or check value changes |
| `specs/annals/annals_v0.md`, `templates_v0.toml`, `selftest_annals_v0.py`, `fixtures/gen_fixtures.py`, `fixtures/`, `golden/` | the walk is the father's (§4, §16); the `ashcombe` fixture's probes are possible histories under T1 (N9: Bram is the son of two houseless, un-named founders, Xan and Sibby; Col is the son of Cedric of Ashcombe and Nell, a named daughter of Thorne, who dies before the house's extinction); fixtures and goldens regenerated with the tools |
| `specs/worldgen/fixture_worlds.md` | 0.4.0: §2.2 (A-57 ruled), §3.3 (the regimes, the levers, the chain's shares), §5, §6 (the `dormancy_degrees` coupling; the member share and the living members per house as the run's test) |
| `docs/decisions_pending.md`, `CODEX/05_PROJECT/BCK-002_ArchitectBacklog.md` | D56 after D55, reworded with the chain's numbers, (e)'s costs and the D38 correction; A-59 (M11) |
| `CLAUDE.md`, `specs/systems/next/README.md` | the projection's and the lint's descriptions; the deny-by-default rule, its exemption list and the repeated-key rule (N14) |
| After D56, in (e)'s own PR (BCK-002 A-62) | the M2 0.3.0 draft's F3–F6 (the read on the couple) with its two re-stated `[[amends]]` texts and the lock; `selftest_m2_f1.py` check 9 and its `woman-row` control; B6 and its reference, with the back-fill's check values; `population_synthesis_v0.txt`, `fixture_worlds_v0.txt` and `crn_pairing_v0.txt` regenerated by their tools; the projection v3 (D56's reading as the baseline, the genesis counts and W_n re-pinned); this note's numbers |

Change class **C1**: no frozen spec, Registry file, baseline or run golden changes. The promotion cost of a verb, had one been added, is the shift of every verb id after it in the name-sorted dictionary (M12), re-recorded in the stream goldens at promotion while the plane-A ladders stay; the draft adds none.

## 10. The specs table (open questions, the drafter's proposed rulings, confidence)

| # | Question | Proposed ruling | Confidence |
|---|---|---|---|
| Q1 | A-56: the membership rule | Agnatic, with (d) in this draft and (e) as an M2 0.3.0 amendment; the director's D56. **Ruled: D56 approved, 2026-10-17** | 6/10 |
| Q2 | A-57 | One list; naming wins; a named adult meeting none is demoted; a named minor is never demoted; a minor beyond the degrees is not named at start (§6) | 7/10 |
| Q3 | R4, N2, N5: the vocation reading | Later children, an elder child of the same father alive at the draw, not first or second in a line, is the primary reading, in the submodel and the Year step; the share from the M2 chain (0.116 / 0.128); the frozen reading (0.2) and the literal one (0.152 / 0.160) are sensitivity lines. The readings straddle replacement for a house's periphery (0.83 / 0.91 / 0.996 / 1.086), so the reading is a design decision, not a detail | 7/10 |
| Q4 | (e), the elite read on the couple | **Adopted by D56 (2026-10-17); BCK-002 A-62.** An amendment of the M2 0.3.0 draft: F6's table choice reads the woman's `house_member` row or her husband's (the man `person.spouse` names at the conception draw, whom F7 records as `unborn_father`), once, at the k = 0 draw, with one multiplier whether one spouse or both are members; nothing after the draw reads it, so a husband's death before the birth changes nothing, and a woman with no husband does not draw (F5). Its costs are §4's (N8): B6 and the three back-fill goldens regenerated, REV-SPR-015's F6 reopened. Its own M2 amendment PR with its own review before SPR-015 freezes 0.3.0 | 6/10 |
| Q5 | **Contract conflict, closed (R3, N4):** the next lint pinned neither `[[submodels]]` nor the rest | Deny by default over every frozen leaf, the exemption list in the README for the Architect's ruling; both drafts' records re-derived with the tool; seventeen rendered controls | 8/10 |
| Q6 | The six edge cases (R5) | As §7 states them, the ruling's defaults | 8/10 |
| Q7 | A separate `membership_degrees` | No: `dormancy_degrees` bounds naming and, through the named core, the house's reach; §4 states the coupling (halving the degree halves W_n) | 7/10 |
| Q8 | Change class | C1 (§9) | 8/10 |
| Q9 | The draft period | No run with M7 exists before SPR-021 builds it, so no chronicle disagrees with the annals' agnatic walk; `annals_v0.md` §16 states it | 9/10 |
| Q10 (round 1's Q11) | **Contract tension, pre-existing:** SIM-9's text ("more than N degrees from any title, claim or office") against M7's dormancy ("of its house") | Keep M7's house-scoped (c) with R6's (d) and N7's (a). The amendment text for `docs/03` on the director's D56: "Ruling houses, every holder of a title, claim or office, and the active members of great houses shall be tracked as named actors (T1) from world start. Great-house members more than N degrees from every holder of a title, claim or office of their house, and not first or second in the line of any title, become **dormant** ordinary persons and are re-promoted only by simulation criteria" | 7/10 |
| Q11 | M11, same-instant learning of another system's EventId | BCK-002 A-59: `SUCCEED`'s `triggered` edge to M2's `DEATH` of the same instant needs M7 to learn M2's EventId within the instant, and no engine path exists; SPR-021's plan decides the mechanism (a draft-local cause as SPR-011 ruling 17, or the death command carrying its EventId) | 8/10 |
| Q12 | M10, a typed payload | Without a new verb nothing folds into A-55; A-55 already lists the house on `HOUSE_EXTINCT` | 9/10 |
| Q13 | Whether the fixture worlds' golden should move | No for A-57 (§6); (e) moves it through B6, and A-62 regenerated it with the tool | 9/10 |
| Q14 | N6, the posthumous child of an extinct house | No row, no resurrection, no M2 read (the ruling's one-line alternative); the frozen `on_death` text stands | 8/10 |
| Q15 | N7, criterion (a) for any holder | Any holder of a title, office or claim is named, member or not; recorded in the dormancy summary, the Promote `why` and INV-M7-06; D56 names the path it closes | 7/10 |
| Q16 | N9, the fixture's marriages under T1 | Possible by construction (the named marry the named, the un-named marry on M2's market); check 5 has no marriage-eligibility rule, since T1's named set is not derivable in plane C, so the fixture's text states each marriage's standing | 7/10 |

## 11. D56, the director item (the row in `docs/decisions_pending.md`)

**Question.** How do great houses persist in a T1 world, and whose house is a newborn's (BCK-002 A-56)? The membership rule is one of four levers.

**Recommendation: agnatic membership with vocations for later children and the elite fertility read on the couple (60%).** Numbers, costs, alternatives and the D38 interaction in the row.

**Default if unanswered.** The 0.5.0 draft as it stands (agnatic membership, the later-children reading); the M2 amendment waits for the word.

**Ruled (the director, 2026-10-17): approved** as recommended: agnatic membership, (d) and (e) (`docs/decisions-log.md`). (e) is BCK-002 A-62, the M2 0.3.0 draft amendment of §4 and Q4.

## 12. Round 1 of PR #79's review: ruling to change

| Ruling | Change |
|---|---|
| R1 | §2 (the regimes), §3 (both regimes), §4 (the levers), §5 per regime, §8 re-derived from the T1 regime; the projection v2 |
| R2 | Moot: the draft adds no verb; the bounded rule's requirement (an always-admitted act class) is recorded in §2 for the director's alternative |
| R3 | `lint_odd.py` deny by default; both drafts' records; the rendered controls; §9, Q5 |
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
| M14 | Superseded by N6 |

## 13. Round 2 of PR #79's review: ruling to change

| Ruling | Change |
|---|---|
| N1 | The gate names carry no backticks; the q5 expectations are single-quoted; the inventory names match; every changed `run` line executed verbatim through check_all's own `run` function (111 lines, 0 failures) |
| N2 | The share from the M2 chain with a birth counter and the inter-birth gaps, under the ruled reading; the literal and the frozen readings as sensitivity lines; §1 fact 3, §4, §8, D56 and `fixture_worlds.md` §3.3 restated; "elite overproduction" kept at the 5% the numbers support |
| N3 | §8 and D56: D38's premise fails under every rule but cognatic-capped |
| N4 | The fully generic walk with the exemption list in the README; four more probes and controls; both drafts re-derived (the walk asked for no new record) and the lock by the tool |
| N5 | The vocation text in the submodel and the Year step; §7 |
| N6 | M14's deferral and the `person.unborn_father` read withdrawn; the one-line alternative in `on_birth`, `house_member.house` and INV-M7-06; §7, Q14 |
| N7 | Criterion (a) for any holder; the Promote `why`; INV-M7-06; D56's costs; §2, §5, Q15 |
| N8 | (e)'s costs in §4, §8, Q4 and D56; D56 asks for direction only; §8's B6 sentence corrected |
| N9 | Sibby and Nell; `gen_fixtures.py`, `annals_v0.md` §4 and §16; Q16 |
| N10 | The stale texts (`check_all.sh`'s comment, the projection's rule labels, 0.3.4) |
| N11 | 1.086 under the ruled reading; "within 1.1%" |
| N12 | One geometric interpolation for population and members |
| N13 | "if the holder is male" |
| N14 | The repeated-key rule, "append after", in the README |
| N15 | Q10 cross-referenced to round 1's Q11; "60 lever lines" |
