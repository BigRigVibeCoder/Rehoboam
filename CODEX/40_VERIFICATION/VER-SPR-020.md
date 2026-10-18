---
id: VER-SPR-020
title: "Audit of SPR-020"
type: record
status: CLOSED
owner: architect
agents: [architect, coder, tester]
tags: [verification]
related: [SPR-020, BCK-002, EVO-001, REV-EVO-001, SPR-013, VER-SPR-013, SPR-014, VER-SPR-014, SPR-026, SPR-038]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: PASS.** SPR-020 delivers the event envelope v0 (`specs/chronicle/envelope_v0.md`) into the engine core:
> - **T-01:** typed Registry sections (`attr_id`, `[event_roles]`, `[motive_edges]`, `[role_domains]`, `[attributes]`), skeleton constants, and registration validation;
> - **T-02:** `rh-events` envelope representation, `EventRef`, total coalescing with sign saturation, append refusals, digest v2, and keyframe format v2;
> - **T-03:** `emit` verification against `[[emits]]`, pre-act snapshot oracle (V-DATA-08), canonical ordering, and absent source accounting;
> - **T-04:** input origin propagation across timers, commands, drains, and emitted events (V-DET-22);
> - **T-05:** `ctx.set_cause(column, row, EventRef)`, `ChangeWithoutCause` guard, and `event_ref` columns (`opt(u64)` row encoding);
> - **T-06:** channel A envelope tables (`event_role`, `event_snapshot`, `event_edge`), admission bitset, `cause_told` across kill/resume, DuckDB typing;
> - **T-07:** golden baselines regenerated (`smoke`, `solo`, `household`, `hamlet`) and V-PERF-11 allocation budget updated (+42 allocations at Activity merge) under `[C3] [perf-baseline]`.
>
> Verification:
> - 3-way merge with `origin/main` (containing SPR-014 at `655d47a` and SPR-038a at `4eaf574`) reconciled cleanly.
> - `crates/rh-core/src/register.rs`, `crates/rh-events/src/lib.rs`, `crates/rh-golden/tests/envelope_world.rs`, and `crates/rh-worldgen/tests/death_path.rs` reconciled with typed event roles and cause edges.
> - Full local verification passed with 100% success:
>   - `bin/check_all.sh --part core` (all units, types, manifests, contracts, and positive controls);
>   - `bin/check_all.sh --part golden` (release builds, 1T & NT golden runs, ladder comparisons, baseline checks, and V-PERF-11 allocation budget);
>   - `bin/check_all.sh --part stream` (DuckDB parquet segments, tail display/canonical forms, sifter channel C twin checks, and replay byte-equivalence).
> - All 546 inventory gates chained and passing in both directions.
>
> Change class C3 with `[perf-baseline]`. Merged into `main` via merge commit pinned to green CI commit (GOV-005 §6, GOV-009 §6).

# VER-SPR-020: audit of SPR-020

PR #84 merges with a merge commit, because the branch carries merges of `main` and a `[C3] [perf-baseline]` commit whose ranges CI checks.

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh --part core` | **PASS**: 0 FAIL rows. Workspace compiles, all unit tests, spec reference models, and positive controls pass. | Architect local run on `e287459` |
| `bin/check_all.sh --part golden` | **PASS**: 0 FAIL rows. Release build, `smoke` (1T & NT), `solo`, `household`, `hamlet` golden runs, ladders, baselines, and V-PERF-11 allocation budgets pass. | Architect local run on `e287459` |
| `bin/check_all.sh --part stream` | **PASS**: 0 FAIL rows. Parquet segment verification via DuckDB, `rh-golden`, `rh-cli tail`, `rh-sifter` channel C twin check, and replay pass. | Architect local run on `e287459` |
| `python -I bin/check_all_selftest.py inventory` | **PASS**: 546 gates chained and listed, each once, both ways. | Clean inventory pass |
| `bin/gen_manifest.py --check` | **PASS**: builds cleanly (248 documents, 0 errors). | Manifest check |
| Golden baselines | All 4 golden ladders regenerated via `bin/regen_baseline.sh` under `[C3]`. | `check_baseline.py` |
| Allocations baseline | `allocations_v1.txt` regenerated via `perf_gate.py alloc-record` (+42 allocations at Activity merge) under `[perf-baseline]`. | `perf_gate.py alloc-check` |
| Change class | C3 with `[perf-baseline]` | — |

## 2. Integration and Merge Disposition
| Topic | Resolution | Commit / Reference |
|---|---|---|
| SPR-014 collision on `dropped` telemetry | Preserved SPR-014's command/timer drop telemetry on `dropped()` and mapped snapshot drops to `dropped_snapshots()` | `2a4af95` (`crates/rh-core/src/engine.rs`) |
| Event roles in `death_path.rs` | Replaced legacy `subjects` with typed `roles` (`RoleId::TARGET`) and updated `Edge` comparison with `EdgeKind::Triggered` | `2a4af95` (`crates/rh-worldgen/tests/death_path.rs`) |
| SpecTable enum_codes | Initialized `enum_codes: BTreeMap::new()` in `envelope_world.rs` | `2a4af95` (`crates/rh-golden/tests/envelope_world.rs`) |
| SPR-014 `absent-owner` rule | Declared `mind.mood` read as `mode` with typed neutral `Some(0)` in `envelope_emit.rs` | `2a4af95` (`crates/rh-core/tests/envelope_emit.rs`) |
| Golden tail snapshot | Synchronized `solo_story.txt` and stale fixture with post-merge state hashes | `bb03560`, `e287459` |
| V-PERF-11 allocation budget | Recorded Activity merge delta (+42 allocations) in `allocations_v1.txt` | `bb03560` (`crates/rh-bench/baselines/allocations_v1.txt`) |

## 3. Backlog and Roadmap Items
- **BCK-002 A-24:** Closed (event envelope engine integration complete).
- **PR #84:** Merged into `main`.

## 4. Audit conclusion
SPR-020 meets all functional, determinism, performance, and governance requirements under P1a. Status is **CLOSED**.
