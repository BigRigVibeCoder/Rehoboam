# Fixture worlds for SPR-030: `twin_vales` and `kingdom_10k` (draft 0.1.0)

*Status: draft for SPR-030 (PRJ-002 §2 row SPR-030, §6 "dynastic fixture and 10k dynastic manifests"; BCK-002 A-06, the dynastic part; `docs/09` §5 G1a; `docs/11` P1.14 and G1a). Companion of `specs/manifest/examples/twin_vales.yaml`, `specs/manifest/examples/kingdom_10k.yaml`, the exact reference `specs/worldgen/selftest_fixture_worlds.py` with `golden/fixture_worlds_v0.txt`, and the mirror test `python/tests/test_fixture_worlds_cases.py`. The V-DET-17 protocol these worlds serve is `specs/determinism/crn_pairing_v0.md`. Normative texts: M7 0.3.3 (frozen; `specs/systems/M7.dynasty.odd.toml`), T1 0.3.3 (frozen), the next M2 spec (`specs/systems/next/M2.demography.odd.toml` 0.3.0 draft), SPR-019's `population_synthesis.toml` and `.md`, the Registry 0.2.0.*

> **BLUF:** Two manifests, no new mechanics, no schema change. `twin_vales` is the dynastic fixture world: 1,500 persons in two valleys, two polities under two laws (primogeniture, tanistry), five great houses of 3 to 6 back-filled generations, 300 years; small enough for PR-tier goldens (its genesis builds in 3 s in the Python reference), rich enough that every P1a-variant gate has something to bite on: a deposition is near-certain within 300 years (expected 11.7 across the two sovereigns), a deposed ruler's pressed claim is contested by construction (lead 0.4 < `contest_margin` 0.5), and 17 of the 56 living house members at seed 1 are dormant-eligible from the first Year pulse. `kingdom_10k` is the G1a performance world: 10,000 persons, three polities under three laws, twelve houses, 1,000 years; the named-actor bound is checkable because the naming rule is M7's one criterion and the exact reference counts the named at genesis (58 at seed 1, 67 at seed 2), with `dormancy_degrees` as the lever if the Developer's yearly count approaches 500. Three findings the Architect should rule on are in §5: the manifests cannot set M2's `density_capacity` yet (the frozen Registry lacks it) so both populations drift upward at +0.11% a year; a `dynastic_tension` catalyst would be inert because T1 does not read catalysts; and rung 3's `village` stays open (its content is P1b's).

## 1. What SPR-030 needs from its worlds

| Gate (`docs/09`) | P1a variant | What the world must supply | `twin_vales` | `kingdom_10k` |
|---|---|---|---|---|
| V-INV-12 (SIM-9) | 300 years on the dynastic fixture world | great houses whose members are named from world start, go dormant and are re-promoted by simulation criteria only; genealogy that reconstructs from the life-course log | §2.2 | — |
| V-INV-03 (EVO-1) | deaths through M2's hazard, M7's ruler exit and the agenda action | sovereigns with non-trivial exit hazards; named actors with agenda decisions; everyone under M2 | §2.3 | (also runs) |
| a contested succession or deposition (the brief) | M7 can produce it | a deposed ruler who becomes a claimant; arbitration where the lead is under `contest_margin` | §2.3 | — |
| V-DET-17 (DET-2) | paired rollouts over 10 years | a world with a target whose consequences are bounded, and a thousand persons outside them | §2.4, `crn_pairing_v0.md` | — |
| V-DATA-04 (DATA-8) | identity, parents or founder provenance, death and residence over 1,000 years | back-filled ancestors with `exit_tick < entry_tick`, founders, births | §2.5 | §3 |
| V-DET-01/02/12 | determinism on a dynastic world | the same | §2.5 | §3 |
| V-SAGA-15 (SAGA-10) | stored bytes over 100 years of the 10k dynastic world | an event stream with successions, claims, agenda events | — | §3.1 |
| V-PERF-09 (PERF-5) | headless, then `rh-cli tail`, the TUI stub and a stalled consumer | the 10k world; D42's live-edge window | — | §3.1 |
| G1a default (`docs/11`) | 1,000 years of a 10k-person dynastic world with ≤ 500 named actors in ≤ 60 s on 8 cores | the named-actor bound checkable | — | §3.2, §3.3 |

The manifests use only what the frozen Registry 0.2.0 carries (ADR-0010): the `feudal_realm` template, `feudal_monarchy` and `tribal_confederation`, the succession laws, the `authority`, `successor` and `subordinate` roles, the `mountain_valleys` and `continental_plains` heightmaps, the `temperate_continental` zone, the `succession` and `solitary_life` pattern packs, M2's three bounded parameters and all 17 of M7's and all 16 of T1's (neither has Registry defaults yet: REV-SPR-019 R15, companion open point 7; SPR-021 gives them or the compile refuses a manifest without them). The template's systems P1a does not build (M3, M4, M6, M8) are listed as not built in the run header (SPR-011 ruling 3); M6 absent means `exec_constraint` reads its neutral 0 and `m_ec = 1` in M7's hazards; offices are not back-filled, so the arbitration's military and treasury proxies are 0 until M7 grants one.

## 2. `twin_vales`: the dynastic fixture world

### 2.1 The world

| Field | Value | Why |
|---|---|---|
| Population | 1,500 humans, classes 0.88 / 0.07 / 0.05 | about four manors: PR-tier goldens (the reference builds it in 3 s; the engine's year of 1,500 persons is 18,000 person-months of M2 draws); large enough that U in V-DET-17 is over 1,400 |
| Calendar, horizon | 360 days, 300 years (`historical`) | V-INV-12's 300 years; the fixture's goldens run shorter (the Developer's choice: solo runs 200) |
| Geography | `mountain_valleys`, raster 128, 6 regions | two valleys, two polities; M1 F0's regions give S3 F0 residence something to tile |
| Polity 1: the Vale of Ash | `feudal_monarchy`, **primogeniture**; titles Lord_of_Ash / Heir_of_Ash / Reeve; houses **ashcombe** (5 generations), **thorne** (4), **redmane** (3) | the law with the lowest exit hazard (`L = primogeniture_hr = 0.3`) and a legal heir most of the time: the slow, orderly realm |
| Polity 2: the High Fells | `tribal_confederation`, **tanistry**; titles Chieftain / Tanist; houses **fell** (6), **carrow** (3) | the law with `L = 1` and a line that empties (the stand-in fallbacks SPR-019 measured are tanistry's): the realm where depositions, minors and contested arbitration happen |
| M2 | `e0 30`, `tfr 4.9`, `elite_fertility_mult 1.2` | the Registry's defaults for the first two (the band's middle; r = +0.11% a year); 1.2 so the elite read is live (its default 1.0 makes it inert) |
| M7 | the 17 parameters, values in the YAML with the reason for each | §2.3 gives the arithmetic they produce |
| T1 | the 16 parameters | design values from T1's `[[params]]` sources; `marriage_min_age 16` = `majority_age` |
| Cast, households, catalysts | none | every person is synthesised or back-filled (D49's kinship fallback names them); a catalyst would be inert (§5 D-02) |

### 2.2 V-INV-12: great houses, naming, dormancy and re-promotion

**The naming rule (M7 SIM-9; T1 `[initialization]`).** A person is named at world start iff a cast member or a great-house member who meets M7's criteria: "a seat, office, claim or active vow, or within `dormancy_degrees` of a holder" (M7's Promote command). At genesis no offices, claims or vows exist (SPR-019 back-fills titles and tenures only), so the named at tick 0 are the five holders and every living house member within `dormancy_degrees = 2` parent-or-child steps of the holder of its house's title, over the life-course log with the dead included (the effect language's `kin(p, n)`). The exact reference computes exactly that set (`house_report` in `selftest_fixture_worlds.py`) and pins it per house; beside it the living adults beyond two degrees (M7's first Year-pulse dormancy pass sends Demote for each, if T1 named them at start) and the living minors beyond two degrees (whether a minor is named at start is SPR-021's question: the frozen criterion does not mention age, the vocation step does).

| Seed | Living house members | Named at start | Dormant-eligible adults | Minors beyond | Per house (living / named / dormant adults) |
|---|---|---|---|---|---|
| 1 | 56 | 31 | 17 | 8 | ashcombe 16 / 1 / 10; thorne 13 / 12 / 1; redmane 10 / 6 / 1; fell 12 / 7 / 5; carrow 5 / 5 / 0 |
| 2 | 84 | 33 | 31 | 20 | ashcombe 12 / 6 / 5; thorne 23 / 9 / 7; redmane 18 / 6 / 7; fell 22 / 3 / 12; carrow 9 / 9 / 0 |
| 3 | 65 | 22 | 24 | 19 | ashcombe 8 / 7 / 1; thorne 5 / 2 / 1; redmane 11 / 3 / 5; fell 32 / 7 / 14; carrow 9 / 3 / 3 |

(`golden/fixture_worlds_v0.txt`; the golden's `dynasts` line counts every living back-filled person, the in-married wives included: 70, 106 and 87.) Seed 1's House Ashcombe is the fixture's best case: a 61-year-old lord whose children are dead and whose 15 other living members are all beyond two degrees, so the house is one death from having no named member at all until the seat moves to a collateral line and re-promotes that line's neighbourhood.

**Dormancy and re-promotion over 300 years.** M7's Year pulse recomputes the criterion ("dormancy: recompute claim.rank and send Demote and Promote by SIM-9's criteria"); T1 applies Promote and Demote (deferred while a break is pending). Three mechanisms move the named set, all simulation criteria (no narrative input):

1. **The seat moves.** Each succession re-centres the two-degree neighbourhood on the new holder: the old holder's distant cousins fall out, the new holder's children and siblings come in. With tenures of 20 to 30 years (SPR-019's back-filled logs: 2 to 6 tenures per house over 3 to 6 generations), each house re-centres 10 to 15 times in 300 years.
2. **A house loses its seat.** When a title's line is empty under its law, M7 arbitrates among claimants and the court; a regrant can seat another house, and the former house, with no holder, office or claim, has no named member: a **dormant house**. It is re-promoted when a member gains a claim (`claim.rank` recomputed at the Year pulse: a descendant of a past holder within `holders_of(title, 16)` is in the line) or a seat (a regrant of a vacant office or title to eligible kin). The tanistry houses are where this happens: SPR-019's sweep measured the stand-in fallback (no living adult male in the house) in 6 of 55 tanistry successions on the manor, 25 of 264 at six generations, and 1 of 200 under primogeniture.
3. **A death or a birth.** `on_death` and `on_birth` send Promote for a newborn within reach; a death of a named member removes one and may move the heir.

The gate's assertion ("ruling houses and active great-house members are T1 from world start; dormancy and re-promotion follow simulation criteria only; any person's genealogy reconstructs from the life-course log") is checkable on this world because the reference pins the named set at tick 0 and the Developer logs Promote and Demote with their reasons; the companion does not predict the number of dormancy transitions over 300 years (they depend on M7's succession draws and T1's claims), only that every house has dormant-eligible members from the first pass at seeds 1 to 3 and that two of the five houses stand under the law whose line empties.

### 2.3 V-INV-03 and the contested succession: the arithmetic

**M2's hazard, on everyone.** At e0 30 the crude death rate of the stationary population is about 2.1% a year: the CRN reference's M2-only runs give 311 deaths in 10 years of 1,500 persons at seeds 1 and 2 (`specs/determinism/golden/crn_pairing_v0.txt`). Over 300 years, about 10,000 M2 deaths, among them most holders' (on the F1 schedule a holder's yearly q is 1.7% at 40, 2.7% at 50, 4.6% at 60 and 8.2% at 70).

**M7's ruler exits, on the two sovereigns** (M7 `[algorithm.exit_hazards]`, the frozen 0.3.3): per sovereign per month, `h_m = murder_hazard_base × L × m_ec × (predecessor_killed_or if the predecessor was murdered)` and `h_d = deposition_hazard_ref × L × m_ec × (predecessor_deposed_hr if the predecessor was deposed or murdered)`, `L = primogeniture_hr` under primogeniture and 1 under tanistry, `m_ec = 1 + (exec_constraint_hr − 1) × exec_constraint = 1` with M6 absent, the monthly probability `h × D_m / D` as Prob32. At the manifest's values, per ruler-year at the reference state:

| Sovereign | L | Deposition `h_d` | Murder `h_m` | Expected in 300 ruler-years | After a deposed predecessor (× 1.7) | After a murdered predecessor (deposition × 1.7, murder × 2.8) |
|---|---|---|---|---|---|---|
| Lord of Ash (primogeniture) | 0.3 | 0.009 | 0.003 | 2.7 depositions, 0.9 murders | 0.0153 / 0.003 | 0.0153 / 0.0084 |
| Chieftain (tanistry) | 1 | 0.03 | 0.01 | 9 depositions, 3 murders | 0.051 / 0.01 | 0.051 / 0.028 |

So the expected count of depositions across both realms in 300 years is 11.7 at the reference state and higher after the first (the hazard ratios compound within a realm's run of exits); the probability that **no** deposition happens anywhere in 300 years is e^−11.7 ≈ 8 × 10⁻⁶, and that no murder happens e^−3.9 ≈ 2%. The subordinate titles (Heir, Reeve, Tanist) draw no exit hazards (sovereigns only). The Developer measures the realised counts at seeds 1 to 3; the companion's expectation is the Poisson mean above, with the 300-year variance of the same size.

**The contested succession.** A deposed ruler survives with a claim on the title (M7's deposition outcome); M7's arbitration scores each candidate `arb_legal_weight × [legal heir] + arb_claim_weight × [pressed claim] + arb_military_weight × [holds the marshal office] + arb_treasury_weight × [holds the steward office]`, and the sovereign is seated only when the leader's score exceeds the runner-up's by at least `contest_margin`, else `INTERREGNUM_CRISIS` (the declared interregnum, `interregnum_max_days 180`, then resolution). At the manifest's weights, with no offices granted, the legal heir scores 1.0 and a deposed ruler with a pressed claim 0.6: the lead 0.4 is under `contest_margin 0.5`, so **every succession in which a deposed predecessor presses his claim is contested**, and every arbitration with two pressed claimants and no heir (0.6 against 0.6) likewise. The one step that is not M7's is the pressing: `PressClaim` is T1's agenda decision (`press_claim`, polled every 12 months by its considerations), so the companion cannot compute how many of the 11.7 expected depositions produce a pressed claim; the Developer counts `INTERREGNUM_CRISIS` events at seeds 1 to 3 and reports the share of depositions that led to one. Under tanistry the line also empties on its own (the stand-in rule, SPR-019 B9: 6 of 55 successions on the manor), an arbitration with no heir and whatever claims exist: a second route to the crisis that needs no deposition.

**The agenda action.** T1's `MURDER_ATTEMPT` (the `feud` decision, polled every 6 months) succeeds with `attempt_success 0.3` and submits `ScheduleDeath {cause = murder}` through M2's single death path (M7's `exit_murder` likewise). Its count is not computable from the specs: it depends on opinions, kin harmed and rival claims; the Developer reports it with the two above. V-INV-03's assertion is per death path, not per count: each produces the same consequence set (ledger removal, life-course exit, succession trigger, inheritance of titles and claims, the death event), so one murder in 300 years is enough to run the check, and 0 would be the finding.

### 2.4 V-DET-17

`specs/determinism/crn_pairing_v0.md`: the paired rollouts over 10 years, the scope-only input on the Lord of Ash at tick 538,927, the affected set (112 persons at seed 1), the unaffected (1,415), Pearson's correlation of censored death instants exact in rationals, the threshold 0.9, the expectation 1 and the reference's result (every pair identical), with the global-epoch control that fails. The fixture is the protocol's world because its sovereign's consequences are the largest this world makes and still leave 1,400 persons outside them.

### 2.5 V-DATA-04, V-DET-01/02/12

The back-fill gives this world 153 pre-epoch ancestors at seed 1 (`persons 1653 − living 1500`), each a closed life-course row with `exit_tick < entry_tick = 0` (SPR-019 D-03), the discriminator V-DATA-04's audit uses to exempt them from the residence tiling; the 1,500 living have founder provenance (`founder = k`, V-DATA-05) and S3 F0 residence rows. Births over the run have parents in the log. V-DET-01/02 (bit-equal across runs and thread counts) and V-DET-12 (keyframe resume = continuous) run on it as on solo and smoke; nothing in the manifest is thread- or order-dependent by construction (every system is the same one solo and the manor use).

## 3. `kingdom_10k`: the G1a performance world

### 3.1 One manifest for three gates

| Field | Value | Why |
|---|---|---|
| Population | 10,000 humans, classes 0.88 / 0.08 / 0.04, three cultures | the G1a default's N; `docs/11` P1.14 |
| Calendar, horizon | 360 days, 1,000 years | the G1a run; V-SAGA-15's variant runs its first 100 years; V-PERF-09's variant runs the same world headless and attached |
| Geography | `continental_plains`, raster 256, 40 regions | a kingdom and its marches; 40 regions keep M1 F0's graph and S3 F0's placement at the scale `docs/08` §9 bench 4 measured |
| Polities | the Crownlands (`feudal_monarchy`, **primogeniture**; King / Crown_Prince / Castellan; aldric 6, beaumont 5, corvay 4, dunmere 4, estwick 3); the Marches (`feudal_monarchy`, **seniority**; Margrave / Tanist / Warden; ferrand 5, gault 4, hollin 3, ingram 3); the High Fells (`tribal_confederation`, **tanistry**; Chieftain / Tanist; fell 6, carrow 4, moss 3) | three laws with three hazard multipliers (0.3, `seniority_hr 2.0`, 1): the Marches' sovereign expects 18 depositions and 6 murders per 300 years, the kingdom's 2.7 and 0.9; twelve houses so the named set is a population, not a court |
| Systems | the same M2, M7 and T1 values as `twin_vales` | one calibration for both worlds; the fixture's arithmetic (§2.3) carries over per sovereign |

V-SAGA-15's P1a variant measures stored bytes per event over 100 years: this world emits M2's BIRTH, MARRIAGE and DEATH on 10,000 persons (about 220 births and 210 deaths a year at the defaults: a crude death rate of 2.1% and r = +0.11%), M7's successions, claims, regrants and exits on twelve titles and their subordinates, T1's agenda events on the named; the 40-byte mean budget is measured over that mix. V-PERF-09's P1a variant needs nothing of the manifest beyond its size: the 10k run headless, then with `rh-cli tail`, the TUI stub and a stalled consumer, throughput within 5% and RSS within D42's live-edge window.

### 3.2 The named-actor rule and the bound

The rule that promotes a named actor is M7's one criterion (§2.2): a seat, office, claim or active vow, or within `dormancy_degrees` of a holder; plus the cast (empty here). The bound ≤ 500 is therefore the sum over the realm's title and office holders of their two-degree neighbourhoods, plus claimants and vowers, minus the overlaps.

**At genesis** the exact reference counts it: **58 named at seed 1, 67 at seed 2** (holders 12; the rest their kin within two degrees), with 34 and 57 dormant-eligible adults and 25 and 42 minors beyond two degrees among 117 and 166 living house members (161 and 217 living back-filled persons with the in-married wives) (`golden/fixture_worlds_v0.txt`, per house there).

**Over 1,000 years** the named set is not a count the specs let one compute, but it is bounded:

- A holder's living kin within two degrees at e0 30 and TFR 4.9: about 2.5 living children, 1 grandchild, 2 siblings, 3 nieces and nephews, half a parent, a spouse's side not counted (kinship is parent-or-child steps only): **about 10 per holder** for a holder in his forties, 15 in a large house. At genesis the twelve holders are older (36 to 74, most past 50, their children partly dead) and the reference measures 2 to 8 living kin within two degrees each, mean 5, and 0 to 5 within one degree, mean 2.3 (`kin_within` in `selftest_fixture_worlds.py`; the 58 named at seed 1 are the twelve holders and 46 kin).
- Holders: 12 sovereign and major titles at genesis; M7 grants the subordinate offices (Castellan, Warden) and regrants vacant seats, each new holder adding a neighbourhood: with the Registry's "5 to 200 titles" and this manifest's 3 title roles per realm, the realised holders are **12 to about 40**.
- Claimants (deposed rulers and their pressed claims) and vowers (P1b): a handful at a time.

Expected range: **100 to 400 named** at any instant, the lower end while offices are few and holders old, the upper if M7 fills every office with a holder from a different house and the holders are young fathers. The worst case, 40 holders × 15 kin, is 600, above the bound; the lever is the manifest's `dormancy_degrees`: at 1 a neighbourhood is parents, children and siblings only (about 5), and the same 40 holders give 200. The Developer measures the yearly named count at seeds 1 to 3 over the 1,000 years; if the peak exceeds 500 the manifest's `dormancy_degrees` drops to 1 (a C1 manifest change, no spec change, this companion's §5 D-01 records the choice). The reference pins the genesis count under both values only for 2 (the manifest's); the `kin_within` function takes the degree, so a Developer re-running it at 1 needs no new code.

### 3.3 The 60-second budget

`docs/08` §9's benches give the per-month costs; this manifest's 1,000 years are 12,000 Month pulses over a population that **drifts upward**: at `tfr 4.9` and e0 30 the next M2 spec's own calibration is r = +0.11% a year (NRR 1.034 with widowhood), a factor e^1.1 ≈ 3.0 over 1,000 years, so the mean population over the run is about 17,000 and the final about 30,000 unless a capacity binds. The one P1a mechanism that would hold it near 10,000 is M2 F1's density modifier (`density_capacity`, slope 1.0: mortality rises 1% per 1% above the capacity, and the +3.4% NRR excess is absorbed at about 1.03 × the capacity), and **the frozen Registry 0.2.0 does not carry that parameter** (it is in `specs/systems/next/registry_delta.toml` with the next M2 spec; the validator refuses `density_capacity` today: "unknown parameter"). §5 D-04 records the recommendation: when the next M2 spec and its Registry delta land, both manifests set `density_capacity` (10,000 and 1,500) in a one-line C1 change; until then the Developer's G1a run reports the realised population and the budget is read against the mean, not the genesis count. The named-actor bound of §3.2 scales with holders, not population, so it is unaffected.

## 4. The exact reference and its golden

`specs/worldgen/selftest_fixture_worlds.py` builds both worlds with SPR-019's reference (the same algorithm `rh-worldgen` implements: `population_synthesis.toml`), the fixture at seeds 1 to 3 and the 10k world at seeds 1 and 2, and pins in `golden/fixture_worlds_v0.txt`: the counts (persons, living, dynasts, synthesised, married, households, orphans), the living by sex and by age band (0–14, 15–49, 50+), the named at start with the dormant-eligible adults and the minors beyond, and per house its founder indices (`founder = k`: the annals' labels and the names file's keys, SPR-019 B4/B6/B8 and `specs/annals/annals_v0.md`), generations, rows, members, living, named, title, law, holder's k, tenure count, fallbacks and minors seated; and the SHA-256 of the person, marriage, household and house_member rows the SPR-019 reference renders, so genesis is bit-pinned. The SPR-019 invariants (`Checker`, `check_widths`) run on every case before a line is written. Run time: 30 s (the 10k world's synthesis is 12 s per seed). `python/tests/test_fixture_worlds_cases.py` keeps the hardcoded manifests equal to the YAML, the seeds to the brief, and both manifests' M7 and T1 parameter sets equal to the Registry's.

| World, seed | Persons | Living back-filled persons | Houses (k ranges) | Named at start | Households |
|---|---|---|---|---|---|
| twin_vales 1 | 1,653 | 70 | 0..95, 96..138, 139..163, 164..203, 205..222 | 31 | 744 |
| twin_vales 2 | 1,628 | 106 | 0..47, 48..96, 97..139, 140..214, 215..233 | 33 | 727 |
| twin_vales 3 | 1,634 | 87 | 0..22, 23..48, 49..81, 82..194, 195..220 | 22 | 743 |
| kingdom_10k 1 | 10,352 | 161 | 0..512 over twelve houses | 58 | 4,914 |
| kingdom_10k 2 | 10,355 | 217 | 0..571 over twelve houses | 67 | 4,888 |

(The golden file is the authority for every number here; this table is a reading aid.)

## 5. Decisions (the drafter's; the Architect rules; the director's items are marked)

| # | Decision | Choice and why | Confidence |
|---|---|---|---|
| D-01 | `dormancy_degrees = 2` in both manifests | 2 names a holder's parents, children, siblings, grandparents, grandchildren, uncles, aunts, nieces and nephews: the court a chronicle follows. 1 is the lever for the 10k bound (§3.2); 3 would name 30 per holder and put the 10k world over 500 at 40 holders. The fixture at 2 has dormant-eligible members in every house at seeds 1 to 3 | 8/10 |
| D-02 | No catalyst in either manifest | T1's `from_manifest = ["cast", "dynasties", "peoples"]`: a `dynastic_tension` catalyst would be read by nobody in P1a and the manifest would promise what the run cannot deliver. If SPR-023 to 025 give T1 a catalyst read, the fixture adds one then. **Architect:** whether T1 should read catalysts in P1a is a spec question for those sprints | 8/10 |
| D-03 | No cast | A cast entry fixes names and traits; the fixture's point is that every named actor arises from M7's criterion and the back-fill, which is what V-INV-12 asserts. The annals name everyone through D49's kinship fallback (`Founder #k` where no names file entry exists) | 9/10 |
| D-04 | `density_capacity` deferred | The frozen Registry has no such parameter (§3.3), so neither manifest can set it without failing validation. Recommendation: when the next M2 spec lands (registry 0.3.0), set 10,000 in `kingdom_10k` and 1,500 in `twin_vales` in a one-line C1 change and re-pin nothing (genesis does not read it). Until then the populations drift at +0.11% a year and the G1a budget is read against the mean population. The alternative, lowering `tfr` to 4.74 for NRR = 1, would move the manifests off M2's default and still drift by sampling | 8/10 |
| D-05 | Two laws in the fixture, three in the 10k world | Primogeniture and tanistry are the extremes of M7's hazard multiplier (0.3 and 1) and of SPR-019's measured line-emptying (1 of 200 and 6 of 55); seniority (2.0) joins in the 10k world so every multiplier the Registry carries is exercised somewhere. Ultimogeniture, partition, elective and sortition are the manor sweep's business (SPR-019 R12), not a fixture's | 8/10 |
| D-06 | Generations 3 to 6 | The back-fill's bound is 6 (`DESCENT_MAX`); a 6-generation house gives the tanistry line its depth and the annals a six-generation tree; a 3-generation house is one that can empty | 8/10 |
| D-07 | No separate V-SAGA-15 or V-PERF-09 manifest | Both variants say "the 10k dynastic world"; a separate manifest would split the baseline and double the goldens. The 100-year V-SAGA-15 run is the first 100 years of the same world | 9/10 |
| D-08 | Rung 3 `village` not drafted | BCK-002 A-06's `village` is "about 1k: a local lord, household and lord stores, tribute, the first relic and vow" (`docs/11`): stores, tribute, relics and vows are M3, M4 and S7 (P1b). A P1a `village` would be a hamlet with a lord, which `manor.yaml` already is. Recommendation: A-06's `village` stays open and moves to P1b with its content; `twin_vales` is the dynastic rung between `manor` and `kingdom_10k` | 8/10 |
| D-09 | The seeds pinned | The fixture at seeds 1 to 3 (the brief's "counts at seeds 1–3"); the 10k world at seeds 1 and 2 (12 s each); the CRN protocol at seeds 1 and 2 | 9/10 |

No director decision is needed: no schema change, no new mechanic, no licence, no baseline moved. D-02's and D-04's follow-ups are the Architect's.

## 6. What SPR-030's Developer still decides

- The golden run lengths for `twin_vales` (300 years at a yearly ladder, as solo's 200) and its baseline's thread count; both are C3 and go in a `[C3]` PR with `bin/regen_baseline.sh`.
- The realised counts at seeds 1 to 3: depositions, murders (M7 and agenda), `INTERREGNUM_CRISIS` events, Promote and Demote transitions, the share of successions with a stand-in, the yearly named count of `kingdom_10k` and its peak; whether to drop `dormancy_degrees` to 1 (§3.2).
- The G1a budget's reading against the drifting population (§3.3) until `density_capacity` can be set.
- The V-DET-17 engine run (`crn_pairing_v0.md` §6).
- Whether minors beyond `dormancy_degrees` are named at start (SPR-021's text; the reference counts them separately so either answer is checkable).
