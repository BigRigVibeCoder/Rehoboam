# CRN pairing v0: the V-DET-17 protocol (SPR-030, draft 0.1.0)

*Status: draft for SPR-030 (PRJ-002 §2 row SPR-030; §6 "dynastic fixture and 10k dynastic manifests"). Companion of `specs/determinism/selftest_crn_pairing.py` and `specs/determinism/golden/crn_pairing_v0.txt`. Normative texts: ADR-0025 (scheduled outcomes; item 3, the per-entity epoch), ADR-0026 (CRN survival), `05.1` §6 item 2 (the keyed RNG), the next M2 spec's `[algorithm.mortality_f1]` MR10–MR12 and S8–S11, `docs/09` V-DET-17.*

> **BLUF:** V-DET-17 pairs two rollouts of `twin_vales` that differ by one journaled material input, a scope-only input on the primogeniture sovereign of the Vale of Ash in year 1, and correlates the death instants of everyone the input cannot reach over the 10-year horizon. Under the keyed RNG with a per-entity epoch the correlation is exactly 1: every draw of every other entity is the same bytes in both runs. The gate's threshold 0.9 is the room left for the engine couplings this reference does not model (M7's reactions, T1's agenda, births). The reference computes the protocol end to end on an M2-only model at seeds 1 and 2: 1,415 and 1,421 unaffected persons, 258 and 260 paired deaths, every pair identical, ρ² = 1; the one control, a world-wide epoch bump, gives ρ² = 0.117 and fails.

## 1. What the gate asks

`docs/09` V-DET-17: "CRN survival (ADR-0026). Paired intervention rollouts over 10 years. Correlation of per-person death times across the pair ≥ 0.9 for unaffected lineages." The brief (the Architect, 2026-10-17) asks the protocol to say: what the intervention is (a journaled material input P1a can issue), when and on whom; what pairing means; who is "unaffected", with the kinship degree k justified; the statistic with its censoring; the threshold; and an exact reference or worked computation.

## 2. The mechanism the gate verifies

Every authoritative draw is `H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)` (`05.1` §6 item 2; `rh-ids`). ADR-0025 item 3 makes the epoch a **per-entity** counter: a material input that re-samples an entity bumps that entity's epoch, and only that entity's later draws change keys. The cascade is `K1 = PRF(seed_key, salt, system | epoch << 32)`, `K2 = PRF(K1, purpose, entity)`, draw = `stream(K2, t, k)` (`bin/gen_rng_golden.py`): two runs with the same seed, salt and system agree on `K1` at epoch 0 and on `K2` for every entity whose epoch is 0 in both, so every one of their draws is the same bytes at the same (t, k). An M2 death is a function of the person's own draws, its own age and the month (MR10, S8–S10; the density modifier is 1 below `density_capacity`, which no P1a world sets: §5 D-03), so a person whose epoch the input did not bump and whose state no consequence of the input reaches dies at the same instant in both runs. That is what "CRN survival" means: a perturbation stays local to the entities it touches and to those it reaches through the simulation's own couplings, never through the RNG.

What could break it in the engine, and what the gate would catch: a world-wide epoch counter (the control of §4.4); a draw keyed on anything but the entity's own identity (a draw index shared across persons, a visiting-order dependence); a re-sample that consumes another entity's stream; branch salts that differ between the runs (they are both on the root timeline, salt 0).

## 3. The protocol

### 3.1 World and pairing

| Item | Value |
|---|---|
| World | `specs/manifest/examples/twin_vales.yaml` (the dynastic fixture world, SPR-030), seed 1; seed 2 as a second case |
| Horizon | 10 sim-years from tick 0 (the 360-day calendar: `T_END` = 5,184,000 minutes); the fixture's own horizon of 300 years is not run |
| Run A | the manifest, the seed, an empty input journal |
| Run B | the same manifest hash, seed, Registry version, engine build and thread count; the input journal holds exactly one input (§3.2) |
| Equal in both | `engine_build`, `registry_version`, `manifest_hash`, `seed` (the history tuple of ADR-0025); the branch salt is 0 in both (no fork) |

Pairing is by `PersonId`: founder ids are `H64(founder/v1, [seed, k])` (V-DATA-05), the same in both runs, and children born before the input get the same ids (their birth draws are their parents' and their own, unchanged). The reference models the founders only (§4.1).

### 3.2 The intervention

One **scope-only journaled material input** on one person, the form `worlds/solo/inputs.toml` already uses (`[[input]] tick = T, founder = k`): it names no new state, so the engine's only consequence is ADR-0025 item 3 on that person: its epoch rises by one and a pending death timer of its own, if any, is re-sampled by M2 S11 (the `ResampleCtx` draws, entity `H64(resample-entity/v1, [Person, id])`, keyed by the epoch after the input). Nothing else is written. P1a can issue it: it is the input `rh-cli run --inputs` reads today (SPR-012 T-08, solo's Ada).

| Item | Value |
|---|---|
| Target | the holder at genesis of the first polity's sovereign title (`title.k = 0`, tier 0, held by house 0, the Vale of Ash under primogeniture): founder `k = 55` at seed 1 (aged 61 at tick 0), `k = 6` at seed 2 (aged 67) |
| Tick `T_IN` | `538,927` = 1 year + 14 days + 6 h 7 min: inside year 1 (the target is alive at it at both seeds, the golden says so), inside a month (between the Month pulse at 518,400 and the next at 561,600), not a day start (so it lies between the pulse and any timer that month) |
| Why the sovereign | its death is the one M7 reacts to most (immediate succession, the heir named, the dormancy pass redrawn), so the affected set is as large as this world makes it; the gate is hardest here |
| Why one input | two inputs would need the pairing to say which consequences are whose; one keeps the affected set a single closure |

### 3.3 Affected and unaffected

The **affected set S** is everyone a consequence of the input can reach through the simulation's own couplings in P1a, plus a margin:

1. the target, and `kin(target, k)` over the life-course log with **k = 6**, the dead included (the effect language's `kin(p, n)` reaches at most 6 steps, `KIN_DEGREES_MAX`; M7's `dormancy_degrees` is bounded by it; no P1a mechanism reads kinship beyond six parent-or-child steps, so nothing the input changes can reach a seventh-degree relative through kinship);
2. every member of the target's house (M7's succession, claims, regrants and dormancy act on the house);
3. every named actor of the target's realm at genesis (the holders of the realm's titles and their kin within `dormancy_degrees`: T1's agenda reads the court, opinions and the holders of the realm's titles, so a succession there reaches them);
4. the spouses and household members of 1 to 3 (M2's households: a death dissolves or re-heads a household).

Everyone else alive at `T_IN` **in both runs** is **unaffected (U)**. Persons born after `T_IN` are in neither set (they need not exist in both runs). The reference's S at seed 1: 82 by kinship, 78 house members, 19 named in the realm, 6 more by household, 112 in all (the components overlap); U = 1,415 of the 1,454 alive at the input. At seed 2: S = 67, U = 1,421 of 1,455.

A consequence that reaches a person outside S would be a coupling this protocol does not know. The gate's margin (0.9, not 1.0) is for exactly that: a few such persons move the correlation a little; a world-wide re-keying moves it to nothing (§4.4).

### 3.4 The statistic

For each person p in U, `x_p` = p's death tick in run A and `y_p` in run B, each **right-censored at `T_END`** (a person alive at the horizon's end has `T_END`). Pairs censored in both runs are dropped (they carry no information: the two runs agree that p outlived the horizon). Over the remaining n pairs, Pearson's correlation

    ρ = (n Σxy − Σx Σy) / sqrt((n Σx² − (Σx)²) (n Σy² − (Σy)²))

computed **exactly in rationals** as ρ² with the sign of the covariance (no floating point in a gate: `05.1` §6; the reference uses `fractions.Fraction`). **Pass iff** cov > 0 and ρ² ≥ 0.81 (that is ρ ≥ 0.9, `docs/09` V-DET-17), and |U| ≥ 100 (so the correlation is over a population, not a handful; the fixture gives 1,400). The reference also reports the share of pairs that are identical (the expectation under §2 is all of them) and the two runs' death-instant digests.

Censoring at one horizon, in both runs, keeps the pairs comparable: an uncensored death paired with a censored one is a real disagreement (the person died in one run and not in the other) and lowers ρ, as it should.

### 3.5 Expectation

Under §2 with a per-entity epoch, every person in U has identical draws in both runs, so `x_p = y_p` for all p, the identical share is 1 and ρ = 1 exactly. The reference confirms it at both seeds (§4.3). The engine will differ from the reference in the couplings the reference omits (M7, T1, births): they can move persons into S's margin but not out of U's identical draws, so the engine's ρ on U should also be 1 unless a coupling the protocol does not list exists; 0.9 is the band for the Developer to report, not to tune.

## 4. The reference

### 4.1 The model

`specs/determinism/selftest_crn_pairing.py` runs the protocol on an **M2-only model** of the engine, every piece of which is a reference already on `main`: genesis by SPR-019's exact reference (the same persons, ids, births and houses the engine's `rh-worldgen` writes: `specs/worldgen/selftest_fixture_worlds.py` pins them); M2 F1 mortality MR9–MR12 and S8–S11 by `specs/demography/selftest_m2_f1_mortality.py` (the schedule of `e0 = 30`, `brass_beta = 1`, the density modifier 1); the keyed draws by `bin/gen_rng_golden.py` with the entity and epoch conventions of `bin/gen_m2_draws_reference.py` (StepCtx: entity = the PersonId; ResampleCtx: entity = `H64(resample-entity/v1, [Person, id])`, keyed by the epoch after the input). The model's `ell` uses a bisection where the mortality reference scans; it checks itself against the reference's `ell`, `month_q` and `resample_q` on a sample of ages before any run.

No births (a child born before the input exists identically in both runs and would only add identical pairs; one born after it is excluded by §3.3); no M7 and no T1 (their deaths, ruler exits and agenda murders, fall on named actors, who are in S by construction: the model cannot be used to test them, and does not claim to). Timers fire at day starts and are processed before the Month pulse of the same instant, the input at its tick before both (the draws model's order).

### 4.2 What it pins

Per seed: the target (founder k, id, age, title, law); the input and what S11 did (`pending none`: at both seeds the target had no pending death at `T_IN`, so the input bumped its epoch and nothing else; its later draws changed keys, and its death moved from tick 1,090,080 to 4,151,520 at seed 1, the third year to the ninth, and from 4,494,240 to 2,746,080 at seed 2, the ninth year to the sixth); the affected set by component; the living at the input in A and B and |U|; the deaths in each run, the pairs, the identical pairs, ρ² rounded down to nine decimals; the target's death in each run; the SHA-256 of each run's death instants.

### 4.3 Results (`golden/crn_pairing_v0.txt`)

| Seed | Target | Alive at `T_IN` (A = B) | S | U | Deaths A / B | Pairs | Identical | ρ² | Target's death A → B |
|---|---|---|---|---|---|---|---|---|---|
| 1 | k 55, aged 61 | 1,454 | 112 | 1,415 | 311 / 311 | 258 | 258 | 1.000000000 | 1,090,080 → 4,151,520 |
| 2 | k 6, aged 67 | 1,455 | 67 | 1,421 | 311 / 311 | 260 | 260 | 1.000000000 | 4,494,240 → 2,746,080 |

The 311 deaths in 10 years of 1,500 persons at e0 30 (2.1% a year) are M2's alone; the engine adds M7's ruler exits and T1's agenda deaths on the named.

### 4.4 The control that must fail

`--control global-epoch`: the input bumps **every** living person's epoch at `T_IN` (the mistake a world-wide epoch counter would be; ADR-0025 item 3 forbids it). Every draw after the input changes keys for everyone, so the runs diverge: at seed 1, 314 deaths in B against 311 in A, 434 pairs of which 1 identical, ρ² = 0.116966014, `PROTOCOL FAILED` and `CRN PAIRING: FAIL`. That ρ² is not 0 only because the deaths before `T_IN` (year 1) are shared and the censoring bounds both instants.

## 5. Decisions (the drafter's; the Architect rules)

| # | Decision | Choice and why | Confidence |
|---|---|---|---|
| D-01 | The intervention | A scope-only input on one person, not a new input kind: it exists (solo's Ada), P1a can issue it, and its only consequence is the per-entity epoch and S11, so the affected set is a closure the protocol can state. A kill order or a marriage would need M7's and T1's consequences enumerated | 9/10 |
| D-02 | k = 6 | `KIN_DEGREES_MAX = 6` bounds every kinship read in P1a (the effect language; M7's `dormancy_degrees`), so six steps is the farthest a consequence travels by kinship; the house, the realm's named and the households cover the non-kinship couplings | 8/10 |
| D-03 | The density modifier is 1 | No P1a manifest can set `density_capacity` (the frozen Registry 0.2.0 does not carry it: `specs/worldgen/fixture_worlds.md` D-04), so N does not enter any draw and the model's "identical draws" holds for U. When a world sets a capacity below its living count, N couples every person to every death and this protocol needs restating (ρ drops below 1 by design, and the band 0.9 is what absorbs it): SPR-028's calibration decides, not this document | 8/10 |
| D-04 | Pearson on censored instants, exact | The gate text says "correlation"; Pearson on ticks censored at one horizon, in rationals, is the plainest reading and needs no ranks or survival model. A rank correlation would hide a one-day shift; this one does not | 7/10 |
| D-05 | The reference is M2-only | The engine's M7 and T1 do not exist as references; what they add falls inside S. The Developer runs the protocol on the engine with all three systems and reports ρ on U; the reference's role is to fix the statistic, the sets and the expected value | 8/10 |

## 6. What SPR-030's Developer still decides

- The engine's run: `rh-cli run twin_vales --years 10` twice, the second with an `inputs.toml` holding `[[input]] tick = 538927, founder = 55`; the death instants from the life-course log; U from §3.3 computed on the engine's own kinship (`kin(p, 6)` through the effect language), houses, named set and households at `T_IN`.
- Whether to also run the protocol at seed 2 (the reference does) and on the tanistry sovereign (house 3): optional, the same code path.
- The thread count: the pair runs at the same count; a second pair at another count is V-DET-02's job, not this gate's.
- If ρ on the engine's U is below 1 but above 0.9: name the persons whose instants differ and which coupling reached them; the protocol's S then grows in a v0.2 of this document.
