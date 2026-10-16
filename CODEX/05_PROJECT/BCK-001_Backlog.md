---
id: BCK-001
title: "Development backlog"
type: reference
status: ACTIVE
owner: architect
agents: [all]
tags: [backlog, project-management]
related: [PRJ-001, BCK-002, GOV-007]
created: 2026-10-16
updated: 2026-10-16
version: 1.4.0
---

> **BLUF:** The development backlog, rolling wave. P0 is planned in full as SPR-001 to SPR-010 and is unchanged by ADR-0035 (the saga engine). P1a "Bloodlines" is planned in `PRJ-002`, approved by the director at G0 (D30). Later phases are listed by their `docs/11` work packages. Priorities follow GOV-007 §4. The Architect re-plans at every gate.

# BCK-001 — Development backlog

## 1. Current phase: P0 Foundations (planned in full)

| Sprint | WP (`docs/11`) | Title | Depends on | Class | Priority |
|---|---|---|---|---|---|
| SPR-001 | P0.1 | Repository bootstrap and CI (**closed** 2026-10-16, `VER-SPR-001`) | — | C2 | P1 |
| SPR-002 | P0.2 | `rh-ids`: IDs, keyed hash, counter RNG, salts (**closed** 2026-10-16, `VER-SPR-002`) | 001 | C3 (machinery) | P1 |
| SPR-003 | P0.3 | Samplers with golden vectors (**closed** 2026-10-16, `VER-SPR-003`) | 002 | C3 (machinery) | P1 |
| SPR-004 | P0.8 | Registry v0 loader and ownership lint (**closed** 2026-10-16, `VER-SPR-004`) | 001 | C2 | P1 |
| SPR-005 | P0.4a | `rh-core` time, pulses, phases, `SystemModel`, `AccessSpec`, scheduler (**closed** 2026-10-16, `VER-SPR-005`) | 002, 004 | C3 (machinery) | P1 |
| SPR-006 | P0.4b | Agenda, timers, lifecycle queues, event bus, invariants (**closed** 2026-10-16, `VER-SPR-006`) | 005 | C3 (machinery) | P1 |
| SPR-007 | P0.5 | Plane hashes, keyframes, journal (**closed** 2026-10-16, `VER-SPR-007`) | 005; T-02 006 | C3 (machinery) | P1 |
| SPR-008 | P0.6 | Golden-run CI, the `smoke` world, bisect tool (in progress) | 006, 007 | C3 (machinery) | P1 |
| SPR-009 | P0.7 | Benchmark harness and regression gate (in progress; bench 4 moved to the first P1a field sprint, director 2026-10-16) | 007, 008 | C1 | P2 |
| SPR-010 | P0.4 (DET-3) | `rh-core::fixed`: fixed-point and money types (gap found by the SPR-001 audit; approved 2026-10-16; **closed** 2026-10-16, `VER-SPR-010`) | 001 | C3 (machinery) | P1 |

**Approvals:** SPR-001 approved 2026-10-16; SPR-002 to SPR-009 approved as a batch on 2026-10-16; SPR-010 approved 2026-10-16. The Architect merges every sprint after its Tester pass and audit (GOV-005 §6, director's ruling 2026-10-16); the director approved the plans. The director no longer merges the C3 sprints that change the determinism machinery (GOV-005 §6).

**Sequencing (Architect, 2026-10-16).** The plans listed SPR-009 after SPR-005 alone, but benches 7 and 8 measure SPR-007's hashes and keyframes, and its per-tick measures need SPR-008's `smoke` world. SPR-007's keyframes hold the pending timers of SPR-006. The order is therefore SPR-005, then SPR-006 and SPR-007 side by side (SPR-007 T-02 last), then SPR-008 and SPR-009 side by side. Two tasks need machines only the director can provide: SPR-008 T-02 (the laptop and `adjutant` in the cross-machine hash check) and SPR-009 T-04 (wall-clock baselines on both).

**Gate G0** is assembled by the Architect as `VER-G0` once SPR-001 to SPR-010 are closed.

## 2. Next phase: P1a "Bloodlines" (approved at G0, D30)

The plan is `PRJ-002`, approved by the director at G0 (D30, 2026-10-16; `docs/decisions_pending.md` D30 to D36). Its §2 is the sprint list and replaces this section. The walking skeleton is written up in full as `SPR-011`, issued as SPR-011 (engine prerequisites), SPR-012 (`solo` lives and dies) and SPR-013 (World Stream v0 and the chronicle) under its ruling 1.

Corrections to the earlier sketch here:
- **Household food** (P1.4, P1.5a) and **the rung manifests as P1.11** were listed in P1a. `docs/11` puts P1.4, P1.5a and P1.11 in P1b, and the rung-1 and rung-2 manifests are BCK-002 A-06 (Architect).
- **`S5.daily_life` F1 (P1.18)** reads M4 and M6 columns that have no owner in P1a, so it moves to P1b with M1 F1 (P1.8) (D33; `PRJ-002` rulings M-4, M-5).

## 3. Specs to write before their sprints (warnings from `lint_odd.py`)

| System | Needed by | Status |
|---|---|---|
| `T1.minds` (traits, opinions, agenda goals, stress, secrets v0 knowledge rows) | P1a (minds v0; S6 and S7 read it) | no spec; Registry id added by ADR-0035. **Must include:** the lifecycle commands it accepts (`ApplyInfluence` from S6; `AgendaGoal`, `ClearGoal`, `OpinionModifier`, `ApplyThought` from S7); the rule that a vow's weight never enters a survival decision (V-SAGA-09); **stress and mental breaks** (SAGA-11, V-SAGA-16): `mind.stress` as `q8_8` on a 0–100 scale, crossing the threshold as a deterministic transition that schedules the break timer, its rate read of `vow.conflict` (owned by S7) as a bounded, sourced in-coupling with a neutral value, break thresholds and the break table from the Registry, breaks as scheduled timers with keyed RNG; and the `emotion@e` snapshot attributes of the event envelope; **secrets v0 and the information horizon** (P1b; SAGA-4, V-SAGA-11, V-SAGA-18): knowledge records held by `{actor, accomplices, witnesses}`, spread only by scheduled outcomes (discovery timer with evidence and patrol hazards, alarm, eyewitness flight, news at travel speed, testimony, confession), and every reaction referencing its knowledge-acquisition event |
| `M7.dynasty` (titles, succession, claims) | P1a (moved up) | no spec. **Must include:** **immediate succession** (SAGA-13, V-SAGA-19): a titleholder's death schedules succession arbitration in the same instant, at a later phase (legal heir, claims, military proximity, treasury control), emitting a succession or an interregnum-crisis event; an `[isolation]` forcing of the institutional state it reads from M6 (constant at the reference state in P1a, then step and ramp); and `title.holder`, which S6 reads |
| `S3.migration` (residence, travel) | P1a (S5 and S6 read it); P1b (fugitives) | no spec. **Must include** for P1b: **fugitives** (SAGA-14, V-SAGA-20): the `fugitive` status (set by a lifecycle command from `T1.minds` after a crime or a break) and flight as scheduled moves along the M1 route graph toward a border, sanctuary or ally, with an interception hazard per segment; tile paths only in focus regions and micro worlds |
| `M4.economy` (occupation, holdings, food) | P1b (F0 stores for the kingdom; `solo`'s food, moved from P1a by D31; property inheritance, D35) | no spec. **Must include** (RT-2026-10-16; ADR-0035): at F0, food and wealth are household and lord stores and holdings (barn, herds, goods), with tribute and rents in kind; **no markets, prices or wages until F1 (P4)**; **stocks of mundane props** (tools, weapons, poisons) drawn through simulated acquisition events, so every prop has provenance (SAGA-12, V-SAGA-17); `household.head`, which S6 reads; the entitlement hook into M2; `[isolation]` forcing of its rate reads (harvest, population). The F1 price rule's `data_range` and sourced `beyond_range` move with F1 to P4 |
| `M6.polity` (regional conditions, institutional state) | P1b (polity-lite; S5, S7 and M7 read it; stub for rungs 0–2) | no spec. At P1b: legitimacy, unrest and a revolt hazard only (no Ψ) |
| `M3.agriculture` | P1b (kingdom; `solo`'s harvest, moved from P1a by D31) | no spec. **Must include** (RT-2026-10-16): at the micro profile (F1), the harvest is the yield of the household's own fields, field by field, not a regional soil grid; at F0, a regional annual anomaly; `[isolation]` forcing of its rate reads (weather, labour) |
| `S6.artifacts` | P1b | **draft spec exists** (`specs/systems/S6.artifacts.odd.toml`), with the red team's provenance rules (SAGA-12, SAGA-15: origin event, unbroken bearer chain, physical state with stored causes). Before freezing: the coupling rules (its rate read of the bearer's Honesty-Humility has a bounded, sourced in-coupling with a neutral value; design values allowed while draft); the `[isolation]` forcing (a bearer line with H stepped from 0 to −80 to +80); and **micro behaviour** (V-SAGA-13: a relic's rumour raises the seeking rate of ambitious actors inside its reach, CRN-paired against outside it) |
| `S7.vows` | P1b | **draft spec exists** (`specs/systems/S7.vows.odd.toml`). Before freezing: the coupling rules (its rate read of the vower's conscientiousness has a bounded, sourced in-coupling); the `[isolation]` forcing (conscientiousness constant at −80, 0, +80; regional conditions stepped normal → famine → war → normal); and **micro behaviour** (V-SAGA-13: a witnessed kin murder by a known culprit leads to `AVENGE_KIN` at a rate inside its declared range; V-SAGA-09: survival responses unchanged by an active vow) |

Every new spec follows the coupling rules of `lint_odd.py` (rate reads need a bounded, sourced in-coupling with a neutral value; `[isolation]` forcing for rate and mode reads), and every system passes V-SF-22 alone before its in-couplings are switched on.

## 4. Later phases
Work packages as listed in `docs/11` (P1b, P2, P3, P4, P5, Later / optional, era packs). They are planned into sprints one phase ahead (GOV-007 §3). Work packages marked **Cut (ADR-0035)** are never planned.
