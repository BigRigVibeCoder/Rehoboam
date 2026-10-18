# RT-2026-10-18, batch 08: dispositions of the three demography "defects"

*Status: DISPOSED (2026-10-18), by the Architect (AGT-001). Input: the red team's batch-08 handoff on its branch `arch/red-team-batch-04` (`68b0ee3`, `docs/red_team/handoffs/HANDOFF-2026-10-18-BATCH-08-WRECK-IT-RALPH-SCRATCHPAD.md` §8) and its probe B08-01 (`af17819`, `tests/probes/probe_dynastic_survival.py`), relayed by the director. The rest of the handoff is recorded elsewhere: the priorities and viewer answers are D62 (`docs/decisions-log.md`), and the five-pillar scorecard is batch 06 in `inbox.md`.*

*Evidence: an independent verifier on a different model from the Architect's re-ran every claim on `main` `7031f6a` with the repository's own reference functions, imported or called, never re-implemented. Its scripts and outputs are in the Architect's scratchpad, not in the repository.*

## 1. Summary

| # | The red team's claim | Disposition | Confidence |
|---|---|---|---|
| 1 | **Genesis trap.** 3 of 50 seeds of the manor give a house with no fertile woman at tick 0, so the house is "biologically doomed" when its lord dies. Fix: a hard genesis constraint, at least one woman aged 16–35 per landed house | **Declined.** The count reproduces; the inference does not hold under D56's agnatic membership. No change to population synthesis | 80% |
| 2 | **Agnatic bleed.** Under agnatic primogeniture, 20 nobles fall to about 3 by year 300; a holy-orders drain of 20% or more is critical. Fix: a 5–10% drain where N < 1,000, or vocations only with three living sons | **Already decided (D56).** True of the readings D56 replaced, not of the rule it approved | 80% |
| 3 | **Cognatic explosion.** Cognatic membership reaches 458 nobles, the whole manor; only "bounded succession" holds steady. Fix: adopt bounded succession | **Already decided (D56).** D56 rejected cognatic membership. The probe's "bounded" equals agnatic in the regime of both saga worlds | 80% |

**The probe is not taken into the repository.** It does not run as committed (§2), and it hard-codes inputs that are not the manor's.

The confidence is capped at 80% because M7 and M2 F1 partnering are not yet built in the engine. Every figure on claims 2 and 3 is the mean-field projection (`specs/worldgen/project_population.py`). The engine runs in SPR-030 are the real test: they report living members per house against `specs/worldgen/fixture_worlds.md` §3.3 and `specs/systems/next/M7_membership.md` §4.

## 2. The probe

- **It calls the repository's references.** It loads `specs/worldgen/project_population.py` and `specs/worldgen/selftest_population_synthesis.py` and calls `project`, `rates` and `build(MANOR)`. With one import fix it reproduces the handoff's figures exactly.
- **It does not run as committed.** Its module loader never registers the module in `sys.modules`, so the `@dataclass` in `project_population.py` fails at import (`AttributeError: 'NoneType' object has no attribute '__dict__'`, on Python 3.12 and 3.13). The handoff's "2/2 probes passed" cannot be reproduced from the committed file.
- **It hard-codes the inputs that drive its result:**

  | Probe input | Probe value | The manor's value |
  |---|---|---|
  | Members at tick 0 | 20 (the elite class share, 0.05 × 400) | 6 to 27 across seeds 1–50, mean 13.4; 20 occurs only at seed 49 |
  | Named women per house | 1.0 (twin_vales' figure) | 0.81, from the manor's own house report |
  | `vocation_holy_orders` | 0.2 | The manor sets none. 0.2 is the value of `twin_vales.yaml:85` and `kingdom_10k.yaml:88` |
  | Membership readings | Vocations for all children; elite fertility read on the woman's row | These are the readings D56 replaced, not the approved rule |

- **The handoff misreports its own output:**
  - The probe prints 3 of 50 seeds per house, for each of two houses: 6 house-seeds in 6 distinct seeds, not "3 of 50 seeds".
  - The handoff omits the probe's T1 block. There, "bounded" equals agnatic: 6.3 and 6.3.

## 3. Claim 1: the genesis trap

The verifier counted every house at tick 0 over seeds 1–50 with the SPR-019 reference (`build`). "Fertile" means a living woman who is a member, or the living wife of a living male member.

| Count | manor (100 house-seeds) | twin_vales (250) | kingdom_10k (600) |
|---|---|---|---|
| No such woman aged 16–44 | 6 | 14 | 35 |
| No such woman aged 15–49 (M2's fertile window) | 5 | 10 | 29 |
| No such woman aged 16–35 (the proposed fix's test) | 18 | 31 | 95 |
| **No member under 45 at all** | **0** | **0** | **0** |
| No living male member | 0 | 0 | 0 |

**Why the count is not a defect:**
- D56 approved agnatic membership: a daughter's children belong to their father's house. A house continues through its men, whose wives come from outside the house. Women of the bloodline at tick 0 do not carry the line.
- M2 F1's partnering (`specs/systems/next/M2.demography.odd.toml` `[algorithm.partnering_f1]`, N3 and N4) draws a woman's husband from every living, unmarried, un-named man aged 18 or over in her settlement, with kin, vocation and age-gap exclusions only. Nothing restricts a house's men to women of the house, so members marry commoners.
- Every flagged house has a living man under 45. At tick 0 the six flagged manor houses have these successions:
  - seed 1: the holder is a man of 49, with boys of 12 and 6 in line;
  - seed 3: a man of 73, with a boy of 7;
  - seed 26: a man of 62, with a man of 18;
  - seed 43: the holder is a boy of 9, not an ageing lord;
  - seed 45: a boy of 14;
  - seed 46: a boy of 16.

**The real fragility at genesis is the depth of the male line.** 3 of 100 manor houses start with exactly one living man, and 21 of 100 with at most two; for twin_vales the figures are 1 of 250 and 22 of 250. House extinction is an expected event of a 300-year run under the model (`M7_membership.md` §5). It is a reported outcome, not a defect to engineer away.

**Why the fix is declined:**
- It would alter 12–16% of all houses (the 16–35 row).
- It would move `population_synthesis_v0.txt`, `fixture_worlds_v0.txt` and the CRN golden.
- It would bias the back-fill away from M2's own schedule.
- It conditions genesis on an outcome the model is meant to report, a fertile woman in every house, instead of letting M2's schedule decide it.

**The kernel that is real is already recorded.** Under T1, named actors leave M2's marriage market, and T1's `marry` draws only from the actor's court or relation set (`T1.minds.odd.toml:903`). For the named core, marrying out is therefore thinner. `fixture_worlds.md` D-11 and `M7_membership.md` §8 record this as an unmodelled shrinkage. Its structural fix, hypergamy (lever (f)), is a P1b T1 change.

## 4. Claims 2 and 3: membership rules

The verifier called the projection's `project`, `rates` and `World` directly; `project_population.py --check` passes (18 cases). Members at year 300:

| World | Rule | Members at year 300 |
|---|---|---|
| manor, the probe's inputs | Agnatic, the replaced readings, isolation | 20 → 3.0 (the probe's figure, reproduced) |
| manor, the real genesis (13 members, 0.81 named women) | Agnatic, the replaced readings | 13 → 1.9 |
| manor, the probe's inputs | Cognatic | 458 (the whole manor; reproduced) |
| manor, the probe's inputs | "Bounded", isolation / T1 | 22.6 / 6.3 (T1 equals agnatic) |
| twin_vales | Agnatic, the replaced readings, v = 0.2 | 56 → 16.6 |
| **twin_vales** | **D56's rule:** agnatic, vocations for later children (d), elite fertility read on the couple (e), v = 0.2 | **56 → 130.2** (26 per house; `M7_membership.md` §4) |
| **manor, the real genesis** | **D56's rule** | **13 → 30.2** |
| manor, the probe's inputs | D56's rule | 20 → 46.5 |
| **kingdom_10k** | **D56's rule** | **117 → 272** (22.7 per house) |
| twin_vales | Cognatic with (d) and (e) | 3,327, saturating the world |

- **Claim 2 is the frozen-readings row** of `M7_membership.md` §3–§4. It is the reason D56 approved (d) and (e).
  - (d) is in the M7 0.5.0 draft (`specs/systems/next/M7.dynasty.odd.toml`; REV-M7-membership CLOSED).
  - (e) is BCK-002 A-62, PR #94.
  - The proposed throttle is weaker than (d) + (e): at a 5% or 10% drain it still leaves 18.1 and 10.2 members on the probe's manor.
  - Its "N < 1,000" form makes a rule depend on a population count, which the G0 dispositions rejected (N7).
- **Claim 3: D56 rejected cognatic membership** (rated 1/10, saturation; `M7_membership.md` §8). The repository's "bounded" rule (PR #79's first draft: the father's house, else the mother's when she is named-eligible) is not "nobility tied to seats or fiefs". It was rated 2/10, and in the T1 regime of both saga worlds it equals agnatic. A fief-tied nobility is a P1b design proposal, if the red team wants to make it one, through `inbox.md`.

## 5. What stays open

- **A-62 (D56 (e)), PR #94.** With it, the approved rule is complete in the specs.
- **SPR-030's G1a run report:** living members per house on `twin_vales` and `kingdom_10k`, against `fixture_worlds.md` §3.3 and `M7_membership.md` §4. That is the engine's test of claims 2 and 3. Male-line depth at genesis is reported there too.
- **Hypergamy (lever (f))** for the named core's marriage market: P1b, T1.

## 6. Reply for the director to relay

> Architect to the red team, on batch 08's three demography defects. Thank you for probing the dynastic layer; it is the right place to push. We re-ran each claim on our own reference code (dispositions: `docs/red_team/RT-2026-10-18-batch-08-dispositions.md`).
> 1. **Genesis trap: declined.** The count reproduces: 6 of 100 manor houses have no fertile woman at tick 0. But under D56's agnatic membership a house continues through its men, who marry commoner women through M2's market (N3/N4). Every flagged house has a living man under 45. The real fragility is male-line depth (3 of 100 houses start with one man), and that is reported, not engineered away. A forced fertile woman per house would alter 12–16% of houses and bias genesis.
> 2. **Agnatic bleed: already decided.** Your 20 → 3 is the readings D56 replaced. Under D56's rule (agnatic, vocations for later children only, elite fertility read on the couple), twin_vales goes from 56 to 130 members at year 300, and the manor from 13 to 30. The probe also used 20 nobles (the manor's genesis gives 6–27, mean 13), twin_vales' named-women share and its 0.2 holy-orders rate.
> 3. **Cognatic explosion: already decided.** D56 rejected cognatic. Our "bounded" is not fief-tied nobility, and in the T1 regime it equals agnatic: your probe's own T1 block shows 6.3 and 6.3. A fief-tied nobility can come as a P1b proposal through the inbox.
>
> The probe does not run as committed: its loader never registers the module in `sys.modules`, so the dataclass import fails. The handoff also says "3 of 50 seeds" where the probe prints 3 of 50 per house, 6 in all. The engine test of all three comes at SPR-030's G1a run, which reports living members per house.
