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
version: 1.1.0
---

> **BLUF:** The development backlog, rolling wave. P0 is planned in full as SPR-001 to SPR-010 and is unchanged by ADR-0035 (the saga engine). P1a "Bloodlines" is sketched as items. Later phases are listed by their `docs/11` work packages. Priorities follow GOV-007 §4. The Architect re-plans at every gate.

# BCK-001 — Development backlog

## 1. Current phase: P0 Foundations (planned in full)

| Sprint | WP (`docs/11`) | Title | Depends on | Class | Priority |
|---|---|---|---|---|---|
| SPR-001 | P0.1 | Repository bootstrap and CI (**closed** 2026-10-16, `VER-SPR-001`) | — | C2 | P1 |
| SPR-002 | P0.2 | `rh-ids`: IDs, keyed hash, counter RNG, salts (**closed** 2026-10-16, `VER-SPR-002`) | 001 | C3 (machinery) | P1 |
| SPR-003 | P0.3 | Samplers with golden vectors | 002 | C3 (machinery) | P1 |
| SPR-004 | P0.8 | Registry v0 loader and ownership lint | 001 | C2 | P1 |
| SPR-005 | P0.4a | `rh-core` time, pulses, phases, `SystemModel`, `AccessSpec`, scheduler | 002, 004 | C3 (machinery) | P1 |
| SPR-006 | P0.4b | Agenda, timers, lifecycle queues, event bus, invariants | 005 | C3 (machinery) | P1 |
| SPR-007 | P0.5 | Plane hashes, keyframes, journal | 005 | C3 (machinery) | P1 |
| SPR-008 | P0.6 | Golden-run CI, the `smoke` world, bisect tool | 006, 007 | C3 (machinery) | P1 |
| SPR-009 | P0.7 | Benchmark harness and regression gate | 005 | C1 | P2 |
| SPR-010 | P0.4 (DET-3) | `rh-core::fixed`: fixed-point and money types (gap found by the SPR-001 audit; approved 2026-10-16) | 001 | C3 (machinery) | P1 |

**Approvals:** SPR-001 approved 2026-10-16; SPR-002 to SPR-009 approved as a batch on 2026-10-16; SPR-010 approved 2026-10-16. The Architect merges every sprint after its Tester pass and audit (GOV-005 §6, director's ruling 2026-10-16); the director approved the plans. The director no longer merges the C3 sprints that change the determinism machinery (GOV-005 §6).

**Gate G0** is assembled by the Architect as `VER-G0` once SPR-001 to SPR-010 are closed.

## 2. Next phase: P1a "Bloodlines" (sketched; written up as sprints at G0)

Goal (ADR-0035): a dynasty lives, dies and inherits deterministically, and the templated chronicle reads as a family history (G1a).

| Item | WP | Notes |
|---|---|---|
| Walking skeleton on `solo` | P1.0 | Manifest → worldgen stub → ledger → timers → events → World Stream v0 → `rh-cli tail` → replay |
| Worldgen-lite | P1.1a | Regions, a few settlements, routes; the full Azgaar-style generation waits for P4 |
| M2 demography F1 (rung-0 slice, then births, partnering, households and kinship for rungs 1–2) | P1.3 | Spec exists (rung-0 slice); keep Brass; lean |
| M7 succession and dynasts F1 (moved up from P1b) | P1.7 | Needs a spec: `M7.dynasty` (§3). Hazards at the reference institutional state until M6-lite (P1b) |
| Minds v0 (moved up from P3a) | P3.2a | Needs a spec: `T1.minds` (§3). Traits, ambition, relationships, opinions; IAUS; same-instant agenda; stress and mental breaks (V-SAGA-16) |
| Event emission, causal edges, Chronicle Writer (moved up from P1b) | P1.9 | Typed events, anchored snapshots, Parquet; **the cinematic event envelope** (ADR-0035 §7a, `05.2` §4.1a; V-SAGA-15). **Note for P0:** SPR-006's event schema (`EventDraft`) must leave room for event roles (actor, target, witnesses, instrument), anchored snapshots and typed edges, including the Registry's motive edge types. SPR-006 is approved and is not changed; its implementer keeps the schema open for these |
| Family annals v0 | P2.8a | Templates in `python/` (plane C); the G1a family-history read |
| World Stream v1 (moved up from P1b) | P1.17 | Channels, segments, cursors, materializer; V-STR-01–05 |
| `S5.daily_life` F1 | P1.18 | Virtual telemetry; `rh-cli where/who`; spec exists (`specs/systems/S5.daily_life.odd.toml`) |
| M9 harness v0: isolation harness and the demography and dynasty bands | P1.10 | V-SF-22 for M2, M7, `T1.minds` |
| Effect language v0 | P1.15 | Succession-law eligibility and ordering |
| Household food for `solo` | P1.4, P1.5a | Needs specs: `M3.agriculture` (F0/F1), `M4.economy` (F0: stores, no markets) |
| Rung 1 `household`, rung 2 `hamlet` manifests | P1.11 | `uz_job` is the rung-2 world |
| Engine-purity lint rule | BCK-002 A-14 | V-INSP-05; EVO-4 |

## 3. Specs to write before their sprints (warnings from `lint_odd.py`)

| System | Needed by | Status |
|---|---|---|
| `T1.minds` (traits, opinions, agenda goals, stress, secrets v0 knowledge rows) | P1a (minds v0; S6 and S7 read it) | no spec; Registry id added by ADR-0035. **Must include:** the lifecycle commands it accepts (`ApplyInfluence` from S6; `AgendaGoal`, `ClearGoal`, `OpinionModifier`, `ApplyThought` from S7); the rule that a vow's weight never enters a survival decision (V-SAGA-09); **stress and mental breaks** (SAGA-11, V-SAGA-16): `mind.stress` as `q8_8` on a 0–100 scale, crossing the threshold as a deterministic transition that schedules the break timer, its rate read of `vow.conflict` (owned by S7) as a bounded, sourced in-coupling with a neutral value, break thresholds and the break table from the Registry, breaks as scheduled timers with keyed RNG; and the `emotion@e` snapshot attributes of the event envelope; **secrets v0 and the information horizon** (P1b; SAGA-4, V-SAGA-11, V-SAGA-18): knowledge records held by `{actor, accomplices, witnesses}`, spread only by scheduled outcomes (discovery timer with evidence and patrol hazards, alarm, eyewitness flight, news at travel speed, testimony, confession), and every reaction referencing its knowledge-acquisition event |
| `M7.dynasty` (titles, succession, claims) | P1a (moved up) | no spec. **Must include:** **immediate succession** (SAGA-13, V-SAGA-19): a titleholder's death schedules succession arbitration in the same instant, at a later phase (legal heir, claims, military proximity, treasury control), emitting a succession or an interregnum-crisis event; an `[isolation]` forcing of the institutional state it reads from M6 (constant at the reference state in P1a, then step and ramp); and `title.holder`, which S6 reads |
| `S3.migration` (residence, travel) | P1a (S5 and S6 read it); P1b (fugitives) | no spec. **Must include** for P1b: **fugitives** (SAGA-14, V-SAGA-20): the `fugitive` status (set by a lifecycle command from `T1.minds` after a crime or a break) and flight as scheduled moves along the M1 route graph toward a border, sanctuary or ally, with an interception hazard per segment; tile paths only in focus regions and micro worlds |
| `M4.economy` (occupation, holdings, food) | P1a (`solo`'s food); P1b (F0 stores for the kingdom) | no spec. **Must include** (RT-2026-10-16; ADR-0035): at F0, food and wealth are household and lord stores and holdings (barn, herds, goods), with tribute and rents in kind; **no markets, prices or wages until F1 (P4)**; **stocks of mundane props** (tools, weapons, poisons) drawn through simulated acquisition events, so every prop has provenance (SAGA-12, V-SAGA-17); `household.head`, which S6 reads; the entitlement hook into M2; `[isolation]` forcing of its rate reads (harvest, population). The F1 price rule's `data_range` and sourced `beyond_range` move with F1 to P4 |
| `M6.polity` (regional conditions, institutional state) | P1b (polity-lite; S5, S7 and M7 read it; stub for rungs 0–2) | no spec. At P1b: legitimacy, unrest and a revolt hazard only (no Ψ) |
| `M3.agriculture` | P1a (`solo`'s harvest); P1b (kingdom) | no spec. **Must include** (RT-2026-10-16): at the micro profile (F1), the harvest is the yield of the household's own fields, field by field, not a regional soil grid; at F0, a regional annual anomaly; `[isolation]` forcing of its rate reads (weather, labour) |
| `S6.artifacts` | P1b | **draft spec exists** (`specs/systems/S6.artifacts.odd.toml`), with the red team's provenance rules (SAGA-12, SAGA-15: origin event, unbroken bearer chain, physical state with stored causes). Before freezing: the coupling rules (its rate read of the bearer's Honesty-Humility has a bounded, sourced in-coupling with a neutral value; design values allowed while draft); the `[isolation]` forcing (a bearer line with H stepped from 0 to −80 to +80); and **micro behaviour** (V-SAGA-13: a relic's rumour raises the seeking rate of ambitious actors inside its reach, CRN-paired against outside it) |
| `S7.vows` | P1b | **draft spec exists** (`specs/systems/S7.vows.odd.toml`). Before freezing: the coupling rules (its rate read of the vower's conscientiousness has a bounded, sourced in-coupling); the `[isolation]` forcing (conscientiousness constant at −80, 0, +80; regional conditions stepped normal → famine → war → normal); and **micro behaviour** (V-SAGA-13: a witnessed kin murder by a known culprit leads to `AVENGE_KIN` at a rate inside its declared range; V-SAGA-09: survival responses unchanged by an active vow) |

Every new spec follows the coupling rules of `lint_odd.py` (rate reads need a bounded, sourced in-coupling with a neutral value; `[isolation]` forcing for rate and mode reads), and every system passes V-SF-22 alone before its in-couplings are switched on.

## 4. Later phases
Work packages as listed in `docs/11` (P1b, P2, P3, P4, P5, Later / optional, era packs). They are planned into sprints one phase ahead (GOV-007 §3). Work packages marked **Cut (ADR-0035)** are never planned.
