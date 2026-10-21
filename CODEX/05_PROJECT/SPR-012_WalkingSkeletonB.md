---
id: SPR-012
title: "Walking skeleton B: solo lives and dies (P1.0, increment B)"
type: how-to
status: ARCHIVED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, BCK-002, PRJ-002, SPR-011, SPR-013, SPR-008, ADR-0025, ADR-0026, ADR-0031]
created: 2026-10-16
updated: 2026-10-17
version: 1.1.0
---
> [!NOTE]
> **ARCHIVED BASELINE:** Traditional sprint-based tracking has been superseded by Specification-Driven Development (SDD v2.0). All future deliverables are tracked as Spec Waves in `.spec/ARCHITECT_BACKLOG.md`.

> **BLUF:** Increment B of the walking skeleton (WP P1.0): `solo` lives and dies. The M2 rung-0 mortality schedule in fixed point, the `rh-worldgen` stub that writes founders, `rh-sys-demography` (the first real system crate) and `rh-cli run <world>` with the `solo` golden ladder. Developer: AGT-002-CORE. Testers: AGT-003-DET and AGT-003-STAT (one Tester session may hold both roles, on a different model from the Developer's). About 3 agent-days. **APPROVED by the director at G0** (C3; D30, 2026-10-16). SPR-011's rulings 1–19 bind this sprint and are not repeated here.

# SPR-012 — Walking skeleton B: `solo` lives and dies (WP P1.0)

- **Roadmap item:** `docs/11` P1.0 (increment B) · **Phase / rung:** P1a, rung 0
- **Depends on:** SPR-011 merged · **Plan approved by the director 2026-10-16** (C3, D30; GOV-005 §2)
- **Binding contracts:** `05.1` §2, §4, §6, §9; `05.2` §0, §1, §4, §6; `05.5` §3.2; ADR-0006, ADR-0024, ADR-0025, ADR-0026, ADR-0031; `docs/01` §5 principle 8
- **System spec:** `specs/systems/M2.demography.odd.toml`, frozen 0.2.0 (PR #37; D36), with `[algorithm.mortality_rung0]` and its exact reference `specs/demography/selftest_m2_rung0.py`
- **Rulings (SPR-011, binding):** 3 (systems a manifest names that the build lacks), 5 (ledger rows and the life-course log), 6 (the death path sends no lifecycle command), 7 (the mortality algorithm; deaths at the start of a keyed day), 8 (Registry defaults), 9 (the authored-number boundary), 10 (the worldgen stub), 11 (`DEATH`, roles, verb ids), 15 (material inputs from a committed script), 16 (golden worlds and CI tiers), 17 (extinction)
- **Requirements covered:** DET-1, DET-4, DET-5, DET-7 and DET-13 on `solo`; DATA-1 and DATA-6 (subset); DATA-8 (founders and deaths); SIM-1 (N = 1); SIM-5 (death draws); SIM-6 (pre-epoch founders); SIM-15

## Specs this sprint needs

| Spec | Exists? | Gap | Writer | Needed by |
|---|---|---|---|---|
| `specs/systems/M2.demography.odd.toml` | Frozen 0.2.0 | — | — | T-05, T-07 |
| `specs/manifest/registry_v0.toml` (`DEATH`, `EXTINCTION`, `tellability`, `defaults`) | Exists (0.2.0) | — | — | T-07, T-08 |
| `specs/manifest/examples/solo.yaml` | Exists | None (ruling 3 covers its unbuilt systems) | — | T-08 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C3**: the first real system crate (`rh-sys-demography`) and the first real golden world (`solo`). Contains C2: two new crates (`rh-worldgen`, `rh-sys-demography`) |
| Planes touched (ADR-0024) | A (`rh-core`, `rh-registry`, `rh-worldgen`, `rh-sys-demography`); tool (`rh-manifest`, `rh-cli`, `bin/`). B and C: none |
| Owned columns written | `M2.demography`: `person.id`, `person.birth_tick`, `person.sex`, `person.alive`, `person.death_tick`, `life_course.*`. `rh-worldgen`: initial values only (ruling 10) |
| Reads (via `rh-contracts`) | None: the rung-0 slice reads no other system |
| RNG purposes added | M2 `death_time` (person, month; k = 0 the monthly Bernoulli, k = 1 the day), flagged `resample_on_fork` (a vital timer, `05.1` §6 item 9). `rh-worldgen`: `founder_sex`, `founder_age`, `founder_birthday` (founder, genesis) |
| Golden-hash effect | A new baseline for `solo`, announced to the director before merge (SPR-011, director approval 2). `smoke` unchanged |
| Fixed-point quantities | K, O(x) and l(x) at the widths the M2 spec states; monthly death probabilities as `Prob32`; e₀ and β compiled from the manifest (ruling 9) |
| Determinism lints (GOV-003 §2, §6) | Must pass. Pinned labels for `rh-worldgen` and `rh-sys-demography` |
| Tests that gate this sprint | V-DET-01, V-DET-02, V-DET-05, V-DET-12, V-DET-19, V-INV-01 (subset), V-INV-08, V-INV-11, V-DATA-04 (subset), V-DATA-05, V-PLN-01, V-SCALE-03, V-GOLD-01, V-PERF-04, plus `bin/check_all.sh` |
| Docs updated in the same sprint | `bin/check_all.sh` and `CLAUDE.md` checks; `05.0` §6 (new crates); the M2 brief |

## Tasks

#### T-05: The M2 rung-0 schedule in fixed point (ruling 7)
- **Deliverable:** In `rh-sys-demography`: the Brass General Standard as exact decimals, K from e₀ by bisection, l on the monthly grid, monthly death probabilities as `Prob32`. `bin/gen_m2_reference.py` (Python `fractions`, the same algorithm and rounding) with `--check`, and golden vectors in `crates/rh-sys-demography/tests/golden/m2_schedule_v1.txt`.
- **Acceptance:** Bit-equal to the reference at e₀ ∈ {20, 23.441086, 30, 36.569855, 43.45915, 60, 85}. At the standard's own e₀ (43.45915, the trapezoid on the standard grid; corrected from 43.44 by the skeleton-spec drafter), K = 1 within the spec's 2^10 units (M2 lines 296–297: e₀'s `Q16_16` rounding puts K at 2^32 − 569; Architect ruling at the audit, `VER-SPR-012` §3). At the e₀ values that Brass Table 19's α rows imply, l(x) matches Table 19 within 1 per 10,000 (the `selftest_brass.py` tolerance). The implied e₀ of a row is the trapezoid e₀ of its β = 1 schedule on M2's age grid, rounded to `Q16_16` half to even, as `selftest_m2_rung0.py` check 4 computes it; the tool computes these inputs and the golden file carries them. The published two-decimal `golden_e0` values are not inputs to this comparison: they bound the recomputed e₀ (within 0.06 years) only (Architect ruling, 2026-10-17). The e₀ recomputed from the fixed-point schedule is within 0.06 years of the input. q(15) ∈ [0.35, 0.55] exactly when e₀ ∈ [23.441086, 36.569855] (V-SF-02's identity at the schedule level).
- **Status:** [ ] not started

#### T-06: The `rh-worldgen` stub (ruling 10)
- **Deliverable:** Crate `rh-worldgen`, pinned (A, machinery), with `genesis(&WorldSpec) -> WorldTables`: founders into M2's ledger columns and life-course entries (origin founder, pre-epoch birth ticks), keyed draws, `rng_purposes.rs`. No raster, regions, settlements or households.
- **Acceptance:** V-DATA-05: every founder id equals `founder(seed, k)` or its deterministic rehash, in ruling-10 order; a forced collision resolves the same way on every run. The same `WorldSpec` gives bit-equal tables twice and on 1 and N threads. V-INV-11: negative birth ticks round-trip through a keyframe. `solo` gives Ada: female, aged 25 at tick 0. A planted write to a column whose owner does not initialise from the manifest is refused.
- **Status:** [ ] not started

#### T-07: `rh-sys-demography`, the rung-0 slice
- **Deliverable:** System `M2.demography`, with the skeleton from `lint_odd.py --rust` brought in by `include!`, bound to Month/Vital. For each living person: one Bernoulli on the month's probability (k = 0); on a hit, a death timer at the start of a keyed day inside the month (k = 1), never at the Month-pulse instant (ruling 7, amended). `fire` calls `apply_death` (ruling 6): alive flag, death tick, life-course exit with cause `natural`, `DEATH` with the deceased as `target`. `EXTINCTION` once when the ledger empties (ruling 17). `resample_timer` re-draws a pending death at the new epoch. Invariants `INV-M2-01` (conservation) and life-course consistency.
- **Acceptance:** V-DET-12: every event's tick ≥ its instant, ids strictly increase, a sampled death sits in the agenda as a timer until it fires. V-INV-01 subset after every pulse on random small worlds. V-INV-08: Bernoulli draws only (a grep gate: no Poisson death draw). Registration rule `spec` passes against the frozen spec. M2 refuses `brass_alpha` or β ≠ 1 at construction with a named error (ruling 7). A fixture `WorldSpec` of 10,000 founders aged 0 (born in the year before tick 0, ruling 10) gives, for each of 32 fixed seeds, deaths by age 15 inside the 99.9% two-sided band of the schedule's conditional expectation Σ(1 − l(15)/l(x_i)) with its exact Poisson-binomial variance, and over the 32 seeds pooled, the same band (Architect rulings, `VER-SPR-012` §3 and §4, MAJ-1); the band harness itself (V-SF-02, V-SCALE-02) is P1.10. V-DATA-04 subset on the same fixture over 1,000 years: every `PersonId` ever emitted has exactly one life-course entry with founder provenance, and a death record if and only if it is absent from the ledger (residence and occupation intervals wait for S3 and M4).
- **Status:** [ ] not started

#### T-08: `rh-cli run <world>` and the `solo` golden ladder (rulings 3, 16)
- **Deliverable:** SPR-008's runner generalised to real worlds: compile, genesis, register the built systems against `specs/systems/`, run with SPR-007's journal and keyframe store (10-year cadence), write the ladder. The run header carries the history tuple and the built and not-built systems. `worlds/solo/` with `world.toml`, the input script and baselines. PR and nightly jobs. SPR-008's baseline-integrity gate covers `solo`.
- **Acceptance:** V-DET-01/02 on `solo`: monthly at 1, 2, 4 and N threads × 3, nightly; yearly at 1 and N threads on PRs. V-GOLD-01: the PR ladder equals the committed baseline. SPR-008 ruling 6 controls: a hand-edited `solo` baseline FAILS; a regenerated one in a PR not titled `[C3]` FAILS. `--strict` refuses `solo` (ruling 3). V-SCALE-03: the 1,000-year wall time of `solo` is reported. V-PERF-04: the PR tier stays within 15 minutes.
- **Status:** [ ] not started

## Out of scope (explicit)
- Increment C (SPR-013): the stream and the chronicle, `tail`, the sifter, end-to-end replay, rungs 1 and 2.
- Births, partnering, households and kinship (P1.3); fixed-point `exp`/`ln` and Brass at any (α, β) (P1.3).
- Regions, settlements, routes and residence (P1.1a, `S3.migration`); place on events stays empty.
- The event envelope: anchored snapshots, typed and motive edges, the reconciled role codes (P1.9).
- The live socket, cursors as a protocol, `stream.subscribe`, the materializer, pace commands, the TUI stub (P1.17).
- The live in-process sifter and multi-clause patterns (P2.1).
- Same-instant command delivery, BCK-002 A-30 and A-31 (P1.13); exact lags, A-28 (P1.18).
- Food (P1b, D31); daily life (P1.18, in P1b since D33; Later for `minutes`, D31); worlds above 60 persons (P1.14).
- The infant sub-schedule (P1.3): rung 0's hazard stays linear through year 0 until then (PR #37 Q13, accepted).

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | T-06–T-08: V-DET-01/02/05/12 on `solo`, V-DATA-05 founder ids, V-INV-11, the `solo` baseline gate and its controls. Written from `05.1` §6, ADR-0025, ADR-0026 before reading the implementation | `VER-SPR-012-TEST` |
| AGT-003-STAT | T-05, T-07: the fixed-point schedule against the exact reference and Brass Table 19; the 10,000-founder q(15) check against the schedule. Written from `specs/demography/` and R12 §9 | As above |

## Blockers

| # | Blocker | Status |
|---|---|---|
| 1 | SPR-011 not merged | Open |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-012` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class; each golden-corpus change was announced to the director first.
- [ ] Rung checkpoint (GOV-007 §3.5): when `solo` first runs green, the director watches its stream through `rh-cli tail` for 15–30 minutes once SPR-013 lands `tail`. The Architect records the notes as `EVO-` or `DEF-` items.

## Closure (2026-10-17)
Closed by `VER-SPR-012` (PASS). The acceptance sentences of T-05 and T-07 above carry the Architect's rulings at the audit; the deviations, gaps and backlog items are in `VER-SPR-012` §3, §4 and §7.
