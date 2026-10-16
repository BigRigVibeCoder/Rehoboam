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
version: 1.0.0
---

> **BLUF:** The development backlog, rolling wave. P0 is planned in full as SPR-001 to SPR-009. P1a is sketched as items. Later phases are listed by their `docs/11` work packages. Priorities follow GOV-007 §4. The Architect re-plans at every gate.

# BCK-001 — Development backlog

## 1. Current phase: P0 Foundations (planned in full)

| Sprint | WP (`docs/11`) | Title | Depends on | Class | Priority |
|---|---|---|---|---|---|
| SPR-001 | P0.1 | Repository bootstrap and CI | — | C2 | P1 |
| SPR-002 | P0.2 | `rh-ids`: IDs, keyed hash, counter RNG, salts | 001 | C3 (machinery) | P1 |
| SPR-003 | P0.3 | Samplers with golden vectors | 002 | C3 (machinery) | P1 |
| SPR-004 | P0.8 | Registry v0 loader and ownership lint | 001 | C2 | P1 |
| SPR-005 | P0.4a | `rh-core` time, pulses, phases, `SystemModel`, `AccessSpec`, scheduler | 002, 004 | C3 (machinery) | P1 |
| SPR-006 | P0.4b | Agenda, timers, lifecycle queues, event bus, invariants | 005 | C3 (machinery) | P1 |
| SPR-007 | P0.5 | Plane hashes, keyframes, journal | 005 | C3 (machinery) | P1 |
| SPR-008 | P0.6 | Golden-run CI, the `smoke` world, bisect tool | 006, 007 | C3 (machinery) | P1 |
| SPR-009 | P0.7 | Benchmark harness and regression gate | 005 | C1 | P2 |
| SPR-010 | P0.4 (DET-3) | `rh-core::fixed`: fixed-point and money types (gap found by the SPR-001 audit; awaiting approval) | 001 | C3 (machinery) | P1 |

**Approvals:** SPR-001 approved 2026-10-16; SPR-002 to SPR-009 approved as a batch on 2026-10-16. The director still merges the C3 sprints that change the determinism machinery (GOV-005 §6).

**Gate G0** is assembled by the Architect as `VER-G0` once SPR-001 to SPR-010 are closed.

## 2. Next phase: P1a (sketched; written up as sprints at G0)

| Item | WP | Notes |
|---|---|---|
| Walking skeleton on `solo` | P1.0 | Manifest → worldgen stub → ledger → timers → events → World Stream v0 → `rh-cli tail` → replay |
| World Stream v1 | P1.17 | Channels, segments, cursors, materializer; V-STR-01–05 |
| `S5.daily_life` F1 | P1.18 | Virtual telemetry; `rh-cli where/who`; spec exists (`specs/systems/S5.daily_life.odd.toml`) |
| M2 demography F1 (rung-0 slice, then births for rung 1) | P1.x | Spec exists (rung-0 slice) |
| M3/M4 household food for `solo` | P1.x | Needs specs: `M3.agriculture`, `M4.economy` |
| Rung 1 `household`, rung 2 `hamlet` manifests | P1.11 | `uz_job` is the rung-2 world |

## 3. Specs to write before their sprints (warnings from `lint_odd.py`)

| System | Needed by | Status |
|---|---|---|
| `S3.migration` (residence) | P1a (S5 reads it) | no spec |
| `M4.economy` (occupation, holdings, food) | P1a (`solo`'s food) | no spec |
| `M6.polity` (regional conditions) | P1b (S5 reads it; stub for rungs 0–2) | no spec |
| `M3.agriculture` | P1a (`solo`'s harvest) | no spec |

## 4. Later phases
Work packages as listed in `docs/11` (P1b, P2, P3a/b, P4, P5–P7, era packs). They are planned into sprints one phase ahead (GOV-007 §3).
