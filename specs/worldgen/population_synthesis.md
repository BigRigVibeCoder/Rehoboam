# Population synthesis v0 and the dynastic back-fill: companion to `population_synthesis.toml` (SPR-019)

*Status: draft companion (ADR-0033 §4: prose for reasoning only; where it differs from the TOML, the TOML is the spec). Normative text: `specs/worldgen/population_synthesis.toml` `[algorithm.population_synthesis]` P0–P12 and `[algorithm.dynastic_backfill]` B0–B11, with its `[[amendments]]` for the Architect to fold into M2, M7, S3 and the channel-A DDL at SPR-019's freeze. Executable contract: `specs/worldgen/selftest_population_synthesis.py` with `specs/worldgen/golden/population_synthesis_v0.txt`.*

> **BLUF:** At genesis `rh-worldgen` does two things beyond the manifest's cast and households. For the anonymous rest of a people it synthesises persons from the compiled M2 schedule: a stationary age-sex marginal (integerised by systematic allocation with a keyed offset), married couples by fixed-point iterative proportional fitting of a wife-band by husband-band table to the married counts that the F1 nuptiality chain implies (rounded to both marginals exactly by a bipartite flow), dependent children assigned to eligible mothers, households, and residence by S3's rule. For a dynastic world it first writes each dynasty's pre-epoch history: a spine of `generations` ancestors anchored on a youngest generation born in the twenty years before the epoch, their wives, their children by M2's own monthly conception draws, every death by the schedule's inverse draw, every younger son's marriage and children, and the title's tenure log from the root's majority to the genesis holder, each dead row appended closed. Every draw is keyed under `rh-worldgen`'s reserved id, every quantity an exact integer or rational, and the exact Python reference pins hamlet and a new manor fixture at two seeds each. Eleven decisions with confidences are in the TOML; the Architect rules on them. No director item.

## 1. Why the algorithm is a TOML fold file and not this document

ADR-0033 §4 makes `.odd.toml` the spec and prose its companion, and SPR-018's `[algorithm.worldgen_lite]` set the precedent for genesis. But the three systems whose columns genesis writes are frozen (`M2.demography` 0.2.0 on `main`, with its 0.3.0 draft in PR #50; `M7.dynasty` 0.3.2; `S3.migration` 0.1.0), so an `[algorithm.*]` table cannot land in any of them before SPR-019's own freeze without conflicting with the births PR. SPR-016 solved the same problem with `specs/demography/mortality_f1.toml`: a fold file that holds the tables, the amendments and the decisions, and binds nothing until the Architect folds it. This spec does the same (`[fold]` names the targets). `lint_odd.py` does not read `specs/worldgen`, so until the fold the exact reference and its three gates are the contract. **Recommendation: TOML fold file with this companion** (decision D-01, 8/10).

## 2. The pipeline, and what each step owes to the sources

| Step | What | Precedent (docs/14; research) |
|---|---|---|
| P2 | The stationary age-sex marginal from the compiled schedule: cell `(a, s)` proportional to `l(aY) + l((a+1)Y)` times the sex ratio at birth | Lotka's stationary population (`c(a) ∝ l(a)` at `r = 0`); the schedule is Brass's General Standard at the manifest's `e0` (R12 §9, `[algorithm.mortality_rung0]`); V-SF-01's band for `r` straddles 0 |
| P3 | Systematic allocation with a keyed offset: each cell its floor or ceiling, totals exact | humanleague's *integer* microsynthesis (docs/14 §C, DEPEND/PORT candidate) uses quasirandom sampling for the same reason: largest remainder on many small cells piles the units into the largest ones (decision D-10 records the failure on hamlet) |
| P5 | Married counts by age from the F1 nuptiality chain, with a widowhood factor from the schedule | `[algorithm.partnering_f1]` N1 (the early, near-universal pattern, Hajnal 1965 as R09 §2 B summarises it); the husband three years older is the F1 reference's own chain |
| P6 | Iterative proportional fitting of the couple table to the two married marginals, seed = N2's preferred age gap | humanleague and ipfn (docs/14 §C: PORT of the idea, MIT); the arithmetic is ours: fixed point at 32 fraction bits, a fixed sweep count |
| P7 | Exact two-marginal rounding by a unit-capacity bipartite flow | the integrality of bipartite flows (a textbook property); PopulationSim's and humanleague's integerisation keep marginals the same way (docs/14 §C) |
| P9–P10 | Children allocated over mother bands by the model's own expectation (`n(a)` of Coale–Trussell 1974 times the child's survivorship), assigned to eligible mothers with M2's spacing | `[algorithm.fertility_f1]` F1, F5 (the closed period `G + P` = 20 months) |
| B3 | Deaths by inverse transform on the schedule, conditional on survival to a known tick | `[algorithm.mortality_rung0]` S7 (the same `l`), so the ancestors died as M2 would have killed them |
| B4–B8 | The spine anchored at the epoch, M2's conception draws forward, younger sons' marriages | `[algorithm.fertility_f1]` F4–F6 (the Prob32 conception table), N2 (ages); the effect language's deep-world fixture (House Vey) is the shape the blood rule needs (PR #41 §5.3) |
| B9 | Tenures by the back-fill's agnatic reduction of each law: pre-order of the agnatic tree for the lineal laws, the oldest adult for seniority, tanistry and elective, a keyed pick for sortition | `M7.dynasty` 0.3.2 `arbitrate` and the effect language's §8 (decision D-04 says why the programs themselves are not evaluated) |

**Not reused, and why.** humanleague and ipfn are float libraries: a genesis table must be bit-identical across machines (V-INV-10), so the IPF is restated in fixed point and the integerisation in exact rationals. synthpops (households, contact layers) is unmaintained and built for epidemic contact structure, not a dynastic substrate; its household-composition idea survives as P9's seed.

## 3. Determinism rules the algorithm obeys

- **Keyed draws only** (`05.1` §6 item 2): every draw is `H(world_seed, canonical salt, system_id("rh-worldgen"), epoch 0, purpose, entity, t = 0, k)` under the reserved id of SPR-011 ruling 10. Seven new purposes (`[[rng_purposes]]`): `synth_offset`, `backfill_spine`, `backfill_marriage`, `backfill_conception`, `backfill_sex`, `backfill_death`, `backfill_sortition`; `founder_birthday` and `founder_residence` are reused unchanged. Each purpose names its entity and its `k`; no stream, no rejection loop.
- **Fixed draw structure:** one offset draw per allocation; four spine draws per ancestor generation plus one; four per younger son; one death draw per person; per married pulse one conception draw and, on a hit, one day draw; one sex draw per child; one sortition draw per sortition tenure. A count depends only on the stated loops (the months a couple is married and fertile before the epoch), never on a retry.
- **No HashMap iteration, no float:** cells are visited in index order, persons in ascending `PersonId`, the agnatic tree in pre-order by birth tick; the IPF is integer arithmetic at 32 fraction bits with one `rhe` per cell per step; the stationary targets, the married shares and the allocations are exact rationals (a Rust implementation carries them as u128 pairs: the widths are stated in P2, P6). The reference uses `fractions.Fraction` and Python integers only.
- **Every tie broken:** the systematic offset decides every rounding; the flow's BFS order is fixed; pre-order ties go to the lower id; assignment ties to the lower id.
- **Nothing after the epoch:** every back-filled birth and death tick is a day start before tick 0; a pregnancy that would deliver at or after tick 0 is not carried (D-08); no timer and no event exists at genesis.
- **Genesis order:** the cast, the manifest households' children and servants (ruling 10), then the back-filled rows in creation order, then the synthesised cells; `founder(seed, k)` follows that order, so hamlet's existing founders keep their ids.

## 4. The two questions the brief asks, answered

**The id scheme for dead ancestors (ADR-0026).** `founder(seed, k)` for every genesis row, living or dead, with `origin = founder` and parent links where authored (decision D-02, 8/10). The founder and child domains differ (`person/founder/v1` against `person/child/v1`), so a back-filled id can collide with a runtime birth only by a 64-bit coincidence, which the deterministic rehash resolves against the table of every id ever issued (V-DATA-05). INV-M2-07 holds as written: a mother's parity counts her genesis children, and her runtime births continue `H64(mother, parity)` from it. The alternative, child ids with `origin = born` for every pre-epoch row, would count a living dynast as a birth of month 1 in `STEP_DIGEST` and needs a new clause in INV-M2-07.

**How a back-filled ancestor appears** (decision D-03, 7/10). To M2's ledger: not at all (the dead leave the ledger, ADR-0026 item 2). To the life-course log: a closed row with `entry_tick = 0` (the instant it was appended), `exit_kind = death`, `exit_tick` the pre-epoch death tick, `death_cause = natural`; its V-DATA-04 span `[entry_tick, exit_tick)` is therefore empty, which is the Architect's recorded exemption of the back-filled rows from the residence tiling, and no residence row exists for it (S3 R7). To kinship walks: exactly as a runtime dead person, through `life_course.person_id`, `mother`, `father`, `birth_tick` and `sex`, which is what `holders_of(title, 16)` and `descends` need (REV-SPR-017 N5). To the stream: a `LIFE_ENTRY` and a `LIFE_EXIT` record at tick 0, in `PersonId` order among the founders' entries, so the chronicle's `life_course` join and the family annals (SPR-029) see the ancestors; this amends SPR-013's tick-0 order rule R3 (the Architect's, C2).

## 5. What the goldens show

| Case | Synthesised | Married | Children with parents | Dynasts alive at 0 | Rows per house | Tenures per title |
|---|---|---|---|---|---|---|
| hamlet, seed 42 | 14 | 4 | 4 of 4 under 15 | — | — | — |
| hamlet, seed 7 | 14 | 2 | 4 of 4 | — | — | — |
| manor, seed 1 | 386 | 78 | 117 of 117 | 14 (house members 6 and 4; 4 wives married in) | 21, 12 | 3, 2 |
| manor, seed 2 | 385 | 80 | 117 of 117 | 15 (11 and 2; 2 wives) | 25, 7 | 3, 2 |

The houses are small at `e0 = 30`: a three-generation house of about twenty rows with six living members. That is what the schedule and `tfr = 4.9` give; `elite_fertility_mult` does not reach the back-fill because a wife married in has no `house_member` row (M7's rule, which M2 F4 reads), so the elite table only ever applies to daughters of a house. Open point 4 raises it. hamlet's 14 anonymous persons now span the ages 0 to 67 (before D-10 they were all under ten).

## 6. Open points, with a recommendation each

| # | Point | Recommendation |
|---|---|---|
| 1 | **Manifest marginals.** `population` is the only marginal the manifest supplies; `class_shares`, cultures and faiths have no owner in P1a, so nothing can be synthesised for them (ruling 10) | Add nothing in v0 (D-05). When M4 F0 and S1 own class and culture (P1b), add optional `peoples[].age_structure` and class and culture shares, with the Rust loader's parity fixtures |
| 2 | **Law programs in the back-fill.** B9 states agnatic reductions of the seven laws; the effect language's programs apply from tick 0 | Keep B9 for v0 (D-04, 6/10). Alternative for the Architect: SPR-019's Rust calls `rh-registry`'s evaluator at each pre-epoch holder death once PR #41 lands, and the reference then pins everything but the tenure rows |
| 3 | **Daughters of the house** never marry in the back-fill | Accept (D-07). A married-out daughter costs one new row per husband and her children are of no house; add if the annals want aunts |
| 4 | **Elite fertility never reaches a house's wives** (M7's `house_member` rule, M2 F4's read) | A question for the M7 and M2 owners, outside this spec: a `house_member` row for a wife married in, or a read of the husband's house. Not changed here |
| 5 | **A child holder at genesis** has no regent until M7's first binding | An SPR-021 item (D-09): M7 names the regent at its first Day/Polity binding and emits REGENCY_BEGUN |
| 6 | **The `manor` example manifest** sits beside the rung manifests, which are the Architect's (BCK-002 A-06) | Keep it as the SPR-019 fixture (not a rung); SPR-030's dynastic fixture world and the 10k manifest remain the Architect's |
| 7 | **M7's `majority_age` has no Registry default**; B9 uses its own `adult_years = 16` | Keep the constant in v0, inside the bounds [12, 21]; align it with the default SPR-021 sets |
| 8 | **The mortality fold** (SPR-016, branch `arch/M2-F1-mortality-fold`) changes the schedule's knots below age 1 and above 90 and its end | The spec reads the schedule as knots (P0), so the fold needs no text change here; it moves the goldens (the stationary marginal and every death draw read `l`): regenerate them in the fold's PR or in SPR-019's, whichever lands second. The reference reads the births constants from the merged next spec through `selftest_m2_f1.py`, so a changed constant moves the goldens too |

No director item: no requirement, ADR or roadmap text changes (`[director_items]`).

## 7. What SPR-019 builds, in the order the spec implies

1. `rh-ids`: the `house/v1` and `title/v1` layouts beside SPR-015's `household/v1`, pinned by `bin/gen_rng_golden.py`.
2. `rh-worldgen`: the seven purposes in `rng_purposes.rs`; B0–B11 before P1–P12 (the living dynasts count against the population); the rational arithmetic in u128 at the stated widths; the Rust port reproduces every line of `population_synthesis_v0.txt`.
3. The fold: the `[[amendments]]` into M2 (next draft), M7 (0.3.2 → 0.3.3, C2), the channel-A DDL R3 and `selftest_channel_a.py`; `lint_odd.py --next` green with the folded M2.
4. `rh_manifest`: the genesis refusals (a people's population below its founders plus dynasts; a dynasty beyond `max_rows`; a dynasty in a world without M7), with parity fixtures.
5. The golden regeneration of `hamlet` (its 14 anonymous founders change: C3) and the new `manor` ladder; `solo` and `household` have no anonymous rest and no dynasty, so their goldens should not move (an acceptance criterion).
