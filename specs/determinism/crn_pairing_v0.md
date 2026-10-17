# CRN pairing v0: the V-DET-17 protocol (SPR-030, draft 0.3.0)

*Status: draft for SPR-030 (PRJ-002 §2 row SPR-030; §6 "dynastic fixture and 10k dynastic manifests"); 0.2.0 applied REV-SPR-030 round 1 (R3, R4, R5, R9, R10, R11); 0.3.0 applies round 2 (R14, R15, R16, R22). Companion of `specs/determinism/selftest_crn_pairing.py` and `specs/determinism/golden/crn_pairing_v0.txt`. Normative texts: ADR-0025 (scheduled outcomes; item 3, the per-entity epoch), ADR-0026 (CRN survival), `05.1` §6 item 2 (the keyed RNG), the next M2 spec's `[algorithm.mortality_f1]` MR10–MR12 and S8–S11 (`specs/systems/next/M2.demography.odd.toml` 0.3.0 draft), M7 0.3.4, `docs/09` V-DET-17.*

> **BLUF:** V-DET-17 pairs two rollouts of `twin_vales` that differ by one journaled material input, a scope-only input on the primogeniture sovereign of the Vale of Ash placed inside the month of his death in the run without it, as `solo`'s input on Ada is placed. The pair is VALID only if the target is alive at the input in both runs, dies at different instants, and at least one instant falls inside the 10-year horizon; a pair that changes nothing proves nothing. The protocol's assertion is **exact CRN**: in P1a every person the input cannot reach dies at the identical instant in both runs, because under the keyed RNG with a per-entity epoch every one of their draws is the same bytes. The correlation `docs/09` names (≥ 0.9) is reported beside it and becomes the binding gate only when later systems couple lineages. A VALID pair also needs power: at least five sim-years after the input and at least 100 informative pairs, and the target is chosen per seed to give it (the primogeniture sovereign, else the tanistry one, else the seed is skipped and pinned as such). The reference computes the protocol end to end on an M2-only model at seeds 1 to 3: seeds 1 and 3 give VALID pairs with the re-sample path exercised (1,382 and 1,316 unaffected persons, 229 and 179 paired deaths, every pair identical), seed 2 is skipped because neither sovereign dies early enough; five mutant engines (a no-op input, swap-remove visiting order, a one-day shift, a 2% re-key, a world-wide epoch bump) each make the protocol fail on the same run A, each for the reason it must.

## 1. What the gate asks

`docs/09` V-DET-17: "CRN survival (ADR-0026). Paired intervention rollouts over 10 years. Correlation of per-person death times across the pair ≥ 0.9 for unaffected lineages." The brief (the Architect, 2026-10-17) asks the protocol to say: what the intervention is (a journaled material input P1a can issue), when and on whom; what pairing means; who is "unaffected", with the justification; the statistic with its censoring; the threshold; and an exact reference or worked computation. REV-SPR-030 round 1 added: the pair must be VALID (R3), the assertion is exact identity with the correlation reported (R4), the affected set's argument is the mechanics, not a kinship bound (R5), and the controls run on the one run A (R10). Round 2 added: validity needs power, and the target is chosen to give it (R14); the partnering clause is bounded by N4's candidate lists (R15); S is completed after the runs (R16); the fallback when the target's run-A death is not M2's (R22).

## 2. The mechanism the gate verifies

Every authoritative draw is `H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)` (`05.1` §6 item 2; `rh-ids`). ADR-0025 item 3 makes the epoch a **per-entity** counter: a material input that re-samples an entity bumps that entity's epoch, and only that entity's later draws change keys. The cascade is `K1 = PRF(seed_key, salt, system | epoch << 32)`, `K2 = PRF(K1, purpose, entity)`, draw = `stream(K2, t, k)` (`bin/gen_rng_golden.py`): two runs with the same seed, salt and system agree on `K1` at epoch 0 and on `K2` for every entity whose epoch is 0 in both, so every one of their draws is the same bytes at the same (t, k). An M2 death is a function of the person's own draws, its own age and the month (MR10, S8–S10; the density modifier is 1 below `density_capacity`, which `twin_vales` leaves at the next Registry's default, §5 D-03), so a person whose epoch the input did not bump and whose state no consequence of the input reaches dies at the same instant in both runs. That is what "CRN survival" means: a perturbation stays local to the entities it touches and to those it reaches through the simulation's own couplings, never through the RNG.

What could break it in the engine, and what the gate catches (§4.4 runs each as a mutant of the model): a world-wide epoch counter (`global-epoch`); a draw keyed on the visiting position instead of the entity, so that a removal from the living vector shifts everyone after it (`swap-remove-after-input`); a re-key of entities the input did not touch (`rekey-2pct`); a systematic shift of instants that a correlation would not see (`shift-one-day`); and the protocol's own blind spot, an input that changes nothing (`no-op-input`). Branch salts differ between the runs only if one forks; both are on the root timeline, salt 0.

## 3. The protocol

### 3.1 World and pairing

| Item | Value |
|---|---|
| World | `specs/manifest/examples/twin_vales.yaml` (the dynastic fixture world, SPR-030); seed 1 in the PR tier, seeds 1 to 3 nightly (R10); a seed none of whose sovereigns meets the power floor of §3.2 is pinned as SKIPPED (seed 2 is) |
| Horizon | 10 sim-years from tick 0 (the 360-day calendar: `T_END` = 5,184,000 minutes); the fixture's own horizon of 300 years is not run |
| Run A | the manifest, the seed, an empty input journal |
| Run B | the same manifest hash, seed, Registry version, engine build and thread count; the input journal holds exactly one input (§3.2) |
| Equal in both | `engine_build`, `registry_version`, `manifest_hash`, `seed` (the history tuple of ADR-0025); the branch salt is 0 in both (no fork) |

Pairing is by `PersonId`: founder ids are `H64(founder/v1, [seed, k])` (V-DATA-05), the same in both runs; a child born before the input gets the same id in both (its parents' and its own draws are unchanged); a child born after the input in both runs is paired by `PersonId` too, which M2's child-id rule (`H64(child/v1, [mother, parity])`, F9) makes equal when the mother and the child's birth order are the same in both runs, which is what §3.4's partnering clause asserts for mothers in U.

### 3.2 The intervention and the pair's validity

One **scope-only journaled material input** on one person, the form `worlds/solo/inputs.toml` already uses (`[[input]] tick = T, founder = k`): it names no new state, so the engine's only consequence is ADR-0025 item 3 on that person: its epoch rises by one and a pending death timer of its own, if any, is cancelled and re-sampled by M2 S11 (the `ResampleCtx` draws, entity `H64(resample-entity/v1, [Person, id])`, keyed by the epoch after the input). Nothing else is written. P1a can issue it: it is the input `rh-cli run --inputs` reads today (SPR-012 T-08, solo's Ada).

| Item | Value |
|---|---|
| Target | chosen per seed from run A (R14): the primogeniture sovereign (the Lord of Ash, `title.k = 0`) if his run-A death leaves at least `MIN_WINDOW_YEARS` = 5 sim-years before the horizon's end, else the tanistry sovereign (the Chieftain, `title.k = 3`) on the same condition, else the seed is **SKIPPED** and pinned as such. Seed 1: the Lord of Ash, founder `k = 55` (aged 61 at tick 0), dies in run A in year 2.10, window 7.9 years: chosen. Seed 2: the Lord dies in year 8.67 (window 1.3) and the Chieftain in year 6.17 (window 3.8): skipped. Seed 3: the Lord does not die inside the horizon, the Chieftain, founder `k = 160` (aged 68), dies in year 3.77, window 6.2: chosen. A candidate whose run-A death is not M2's (in the engine, a T1 `DEATH_DUE` from an agenda murder or M7's `MURDER_DUE`) is rejected the same way: the input re-samples M2's own pending timer only, so such a death gives no S11 path and no guarantee the instants differ |
| Tick `T_IN` | **the day before the target's death in run A, at 06:07** (`τ_A − 1440 + 367`): inside the month of that death, after its Month pulse (a death day is at least the pulse's day plus one, S9), not a day start, so the input's instant is its own and the death is pending at it, exactly as solo places Ada's input (R3). Seed 1: `T_IN` = 1,089,007 (year 3); seed 3: 1,953,007 (year 4) |
| What S11 does | cancels the pending death, draws whether the target dies in the rest of the month at the conditional probability `(l(x₀) − l(x₁)) / l(x₀)` (MR12), and on a hit re-draws the day; a miss means the target survives the month and its later months are drawn at epoch 1. At both seeds the re-sample **missed**: the target's death moved from tick 1,090,080 to 4,151,520 at seed 1 (the third year to the ninth) and from 1,954,080 to 4,711,680 at seed 3 (the fourth year to the tenth) |
| Validity (R3, R14) | the pair is **VALID** iff (a) the target is alive at `T_IN` in both runs, (b) its death instants differ (a censored instant differs from an uncensored one), (c) at least one of them is inside the horizon, (d) the window after the input is at least 5 sim-years (`T_END − T_IN ≥ MIN_WINDOW`), and (e) at least `MIN_INFORMATIVE` = 100 pairs of U are informative (not censored in both runs). (d) and (e) are the power floor: a late input leaves too few deaths to say anything (seed 2's Lord would leave 39); the target choice applies both floors from run A (the window, and run A's unaffected deaths after the input as a lower bound of the informative pairs). An INVALID pair fails the gate: it would pass with any engine, including one that ignores the input. **The run fails when no tested seed gives a VALID pair** (a skipped seed is pinned with its candidates and run A's death digest, so a VALID seed cannot silently become a skipped one) |
| Why the sovereign | its death is the one M7 reacts to most (immediate succession, the heir named, the dormancy pass redrawn), so the affected set is as large as this world makes it; the gate is hardest here |
| Why one input | two inputs would need the pairing to say which consequences are whose; one keeps the affected set a single closure |

### 3.3 Affected and unaffected

The **affected set S** is everyone a consequence of the input can reach through the simulation's own couplings in P1a. The argument is the mechanics, not a kinship bound (R5; `KIN_DEGREES_MAX` bounds one read of the effect language, not a chain of consequences):

- **M2 deaths are age-only.** A person's monthly death draw depends on its own age, the month and its own keyed stream (MR10, S8); nothing of another person enters it below `density_capacity`. So no M2 death outside the target's own can move unless some other system moves that person's state.
- **Non-M2 deaths fall on sovereigns and named actors only.** M7's exit hazards are drawn for sovereigns; M7's `MURDER_DUE` and T1's `MURDER_ATTEMPT` target the court; T1's agenda acts for named actors. The target's death (moved by the input) triggers M7's succession, which changes who holds the title, who is heir, who is named (the full reading of §2.2 of `fixture_worlds.md`) and whom T1 polls; every person those steps can touch is a holder, a candidate in the title's line, a member of the house, or a named actor of the realm.
- **Households.** M2's household rules re-head or dissolve a household at its head's death; a spouse's widowhood changes her partnering draws (N3/N4).

Hence S = the target ∪ **the title's line at genesis** (every candidate the law orders, `Law.line_for`: 15 persons at seed 1, 4 at seed 3) ∪ **the target's house** (every member, the dead included: 78 and 84) ∪ **the realm's named at genesis under the full reading** (21 and 13) ∪ **every member of every house of the target's realm** (R16: a later succession in any of the realm's houses can name someone the input's consequences reach; 133 and 103) ∪ **their spouses and household members** (9 and 14). S has 142 members at seed 1 of whom 44 are alive at `T_IN`, and 117 at seed 3 of whom 50 are alive at `T_IN` (R11: the dead in S are harmless, the living are the exclusion). Everyone else alive at `T_IN` in both runs is **unaffected (U)**: 1,382 at seed 1, 1,316 at seed 3. **In the engine S is completed after the runs** (R16): everyone named at any instant in either run, read from the Promote and Demote records of channel A, joins S, and |U| is reported after the extension; it stays at least 100 by the power floor, or the pair is INVALID. Persons born after `T_IN` are in neither set for the death statistic; §3.4 pairs them through the partnering clause.

A consequence that reaches a person outside S would be a coupling this protocol does not know, and the exact-identity gate (§3.4) would show it as a differing pair, by `PersonId`.

### 3.4 The statistic

For each person p in U, `x_p` = p's death tick in run A and `y_p` in run B, each **right-censored at `T_END`** (a person alive at the horizon's end has `T_END`). Pairs censored in both runs are dropped (they carry no information: the two runs agree that p outlived the horizon).

**The gate (R4): exact CRN.** Every remaining pair is identical: `x_p = y_p` for all p in U. One differing pair fails, and the reference names it by `PersonId`.

**Reported beside it:** Pearson's correlation over the pairs,

    ρ = (n Σxy − Σx Σy) / sqrt((n Σx² − (Σx)²) (n Σy² − (Σy)²))

computed **exactly in rationals** as ρ² with the sign of the covariance (no floating point in a gate: `05.1` §6; the reference uses `fractions.Fraction`), with `docs/09`'s floor ρ ≥ 0.9 (cov > 0 and ρ² ≥ 0.81). In P1a the floor cannot bind while the gate holds (identical pairs give ρ = 1); it becomes the binding criterion when a later system couples lineages by design (M5's contagion, M8's war), and `docs/09`'s row is then re-read. Also required: |U| ≥ 100, so the statement is about a population.

**Partnering (R5, R15, R23).** M2 N4's `husband_pick` cites V-DET-17: the same identity holds for partnering and births, but N4's matching cascades: a woman's husband is drawn from a candidate list, and a candidate removed or added by a consequence of the input changes her match and, through the men she no longer or now takes, later women's matches, including women whose lists contain no S person at all (an exposed earlier woman takes a U man in one run only, and he leaves a later woman's list there). The clause is therefore bounded to what `husband_pick`'s keying guarantees: for every woman of U **whose N4 candidate list and whose own eligibility are identical in both runs at each of her draws over the window**, the same marriage (same spouses, same tick) occurs in both runs, because the draw is keyed `(w, t, h)` on her own stream; and the same births (same `PersonId`, same tick, same sex) follow for such a couple while both spouses stay in U. A differing marriage outside that set is **reported with the person responsible** (the first S person, or the first exposed U woman, whose different state entered the candidate list of the earliest differing draw), not gated. The candidate lists are not in channel A: the engine check **replays N4 from the ledger** at each pulse of both runs (the same deterministic rule on the same ledger rows) to obtain them. The reference's M2-only model runs no partnering and cannot (§4.1).

Censoring at one horizon, in both runs, keeps the pairs comparable: an uncensored death paired with a censored one is a real disagreement (the person died in one run and not in the other) and is a differing pair.

### 3.5 Expectation

Under §2 with a per-entity epoch, every person in U has identical draws in both runs, so `x_p = y_p` for all p, and ρ = 1 exactly. The reference confirms it at both seeds (§4.3). The engine will differ from the reference in the couplings the reference omits (M7, T1, births): they can move persons into S's components but not out of U's identical draws, so the engine's pairs on U should be identical unless a coupling the protocol does not list exists; a differing pair then names the person and the Developer traces which system touched them (§6).

## 4. The reference

### 4.1 The model

`specs/determinism/selftest_crn_pairing.py` runs the protocol on an **M2-only model** of the engine, every piece of which is a reference already on `main`: genesis by SPR-019's exact reference (the same persons, ids, births, houses and lines the engine's `rh-worldgen` writes: `specs/worldgen/selftest_fixture_worlds.py` pins them); M2 F1 mortality MR9–MR12 and S8–S11 by `specs/demography/selftest_m2_f1_mortality.py` (the schedule of `e0 = 30`, `brass_beta = 1`, the density modifier 1); the keyed draws by `bin/gen_rng_golden.py` with the entity and epoch conventions of `bin/gen_m2_draws_reference.py` (StepCtx: entity = the PersonId; ResampleCtx: entity = `H64(resample-entity/v1, [Person, id])`, keyed by the epoch after the input). The model's `ell` uses a bisection where the mortality reference scans; it checks itself against the reference's `ell`, `month_q` and `resample_q` on a sample of ages before any run.

Run A is run once to the horizon; the target is chosen from the sovereigns' deaths in it (R14) and `T_IN` read from the chosen one; run A is re-run to `T_IN` (checked equal to run A's deaths before it) and that state is the common starting point of run B and of every mutant (R10). Each mutant must fail the protocol **for its own reason** (an INVALID pair for the no-op input, differing pairs on a VALID pair for the four others: a mutant that also invalidates the pair proves nothing about the pairs, R25); a control that fails for another reason fails the gate (R14). No births, no M7, no T1 (§3.3 says why they add nothing the model could test). Timers fire at day starts and are processed before the Month pulse of the same instant, the input at its tick before both (the draws model's order).

### 4.2 What it pins

Per seed: every sovereign candidate with its run-A death year and run-A informative count, and the choice; for a skipped seed, run A's death digest (R25); otherwise the target (founder k, id, age, title, law); the input's tick and what S11 did; the affected set by component, its size and how many of it are alive at the input; the living at the input in A and B and |U|; the pair's validity, the deaths in each run, the pairs, the identical pairs and ρ² rounded down to nine decimals, the floor's verdict; the target's death in each run; the SHA-256 of each run's death instants.

### 4.3 Results (`golden/crn_pairing_v0.txt`)

| Seed | Target | `T_IN` (window) | S (alive at `T_IN`) | U | Deaths A / B | Pairs | Identical | ρ² | Target's death A → B |
|---|---|---|---|---|---|---|---|---|---|
| 1 | the Lord of Ash, k 55, aged 61 | 1,089,007 (7.9 y) | 142 (44) | 1,382 | 311 / 311 | 229 | 229 | 1.000000000 | 1,090,080 → 4,151,520 |
| 2 | none qualifies: the Lord dies in year 8.67, the Chieftain in 6.17 | SKIPPED | — | — | — | — | — | — | — |
| 3 | the Chieftain, k 160, aged 68 | 1,953,007 (6.2 y) | 117 (50) | 1,316 | 323 / 323 | 179 | 179 | 1.000000000 | 1,954,080 → 4,711,680 |

Both pairs are VALID with the power floor met; at both seeds S11's re-sample ran and missed. Seed 1 is the PR tier's case; seeds 2 and 3 run nightly, seed 2 as the pinned skip. The 311 and 323 deaths in 10 years of 1,500 persons at e0 30 (2.1% a year) are M2's alone; the engine adds M7's ruler exits and T1's agenda deaths on the named.

### 4.4 The controls that must fail (`--controls`, on the same run A)

| Mutant | What it models | Required reason | Outcome at seed 1 | At seed 3 |
|---|---|---|---|---|
| `no-op-input` | an engine that ignores the input (run B = run A) | INVALID: the target's death instants are equal | INVALID (1,090,080 in both) | INVALID (1,954,080) |
| `swap-remove-after-input` | draws keyed by the person's position in the living vector, which is the living at the input (R14) and is swap-remove compacted at each death, from the input on | pairs differ | 38 of 247 pairs differ; ρ² 0.737 | 24 of 176 |
| `shift-one-day` | every unaffected death in run B one day later | pairs differ | 229 of 229 pairs differ (ρ² stays 1: the correlation alone would pass) | 179 of 179 |
| `rekey-2pct` | the input re-keys every 50th unaffected person as well | pairs differ | 7 of 232 pairs differ | 6 of 180 |
| `global-epoch` | a world-wide epoch counter: everyone re-keyed at the input (ADR-0025 item 3 forbids it) | pairs differ | 376 of 379 pairs differ; ρ² = 0.097000335 | 309 of 311; ρ² 0.096 |

The reference prints `PROTOCOL FAILED: seed <n> [mutant]: reason` for each and passes only when every mutant fails **with its required reason** (`controls 5 of 5 FAIL as required` in the PR tier, `10 of 10` nightly over seeds 1 and 3); a mutant that fails for another reason, or does not fail, fails the gate (R14). The `rekey-2pct` mutant re-runs only the re-keyed persons from run A's state at the input, which is exact because a person's draws depend on nothing but its own state in this model; `global-epoch` and the position mutant re-run everyone. Run time on the reference machine: 52 s for the PR tier (seed 1 with its controls), 112 s nightly (three seeds, two with controls).

## 5. Decisions (the drafter's; the Architect's round-1 rulings applied)

| # | Decision | Choice and why | Confidence |
|---|---|---|---|
| D-01 | The intervention | A scope-only input on one person, not a new input kind: it exists (solo's Ada), P1a can issue it, and its only consequence is the per-entity epoch and S11, so the affected set is a closure the protocol can state. A kill order or a marriage would need M7's and T1's consequences enumerated | 9/10 |
| D-02 | The affected set's argument (R5, R16) | S is what the mechanics can reach: the title's line, the house, the realm's named under the full reading, every member of every house of the realm, their households, and in the engine everyone named at any instant in either run. `KIN_DEGREES_MAX` is no longer cited: it bounds one read, not a chain of consequences | 8/10 |
| D-03 | The density modifier is 1 (R9) | `twin_vales` keeps the next Registry's default capacity (10,000,000), which never binds in a 1,500-person world, so N enters no draw and the model's identical draws hold for U. A world whose living count exceeds its capacity couples every person to every death and this protocol needs restating there (the correlation floor, not identity, is then the criterion); `kingdom_10k` will set a capacity (`fixture_worlds.md` D-04), and the protocol runs on `twin_vales` | 8/10 |
| D-04 | Exact identity is the gate, Pearson is reported (R4) | In P1a a differing pair is a bug, whatever ρ says; a correlation tolerates a uniform shift (§4.4) and a 2% re-key (ρ² 0.938 in the reviewer's run). ρ stays in the report because `docs/09` names it and later systems will need it | 8/10 |
| D-05 | The reference is M2-only | The engine's M7 and T1 do not exist as references; what they add falls inside S. The Developer runs the protocol on the engine with all three systems and reports the pairs on U; the reference's role is to fix the sets, the validity rule, the statistic and the expected value | 8/10 |
| D-06 | `T_IN` from run A's death (R3) | The day before the target's run-A death at 06:07, as solo does: it exercises S11 and makes validity checkable from run A alone. The alternative, a fixed tick, gave `pending none` at both seeds and let an input on a dead target pass | 9/10 |
| D-07 | The target choice and the power floor (R14) | The primogeniture sovereign first (the largest consequence set), the tanistry sovereign second, the seed skipped third, all read from run A, so the choice is deterministic and pinned; the floor of 5 years and 100 informative pairs is what makes a pass mean something (seed 2's 39 pairs did not). A skipped seed costs one run A (15 s) nightly and documents why | 8/10 |

## 6. What SPR-030's Developer still decides

- The engine's run: `rh-cli run twin_vales --years 10` once; read the sovereigns' death ticks and causes from channel A and choose the target by §3.2; run again with an `inputs.toml` holding `[[input]] tick = τ − 1073, founder = 55` (seed 1); the death instants from the life-course log; S from §3.3 computed on the engine's own line (the effect language's succession laws at genesis), house, the realm's houses, named set (the full reading, then everyone named at any instant in either run from the Promote and Demote records) and households; U reported after the extension; the validity check with the power floor; every pair on U identical; ρ reported; the partnering clause on MARRIAGE and BIRTH records with N4's candidate lists, differing marriages outside its scope reported with the S person responsible.
- Seeds 2 and 3 (the nightly tier): seed 3's target is the Chieftain (house 3, tanistry), seed 2 is a pinned skip; the same code path; the engine's run A decides the target the same way, and a sovereign whose run-A death is a `DEATH_DUE` (not M2's) is rejected like one that dies too late.
- The thread count: the pair runs at the same count; a second pair at another count is V-DET-02's job, not this gate's.
- If a pair on U differs: name the person and the system that touched them (the event log's motive edges give the chain), then widen S in a v0.3 of this document, or fix the engine if no mechanic explains it.
