---
id: VER-SPR-033a
title: "Audit of SPR-033a (voxel viewer spike stage A: Bevy-free core)"
type: record
status: CLOSED
owner: architect
agents: [architect, coder]
tags: [verification, observability, stream]
related: [SPR-033, PRJ-003, ADR-0036, REV-VIEWER, BCK-002, SPR-026, SPR-034]
created: 2026-10-19
updated: 2026-10-19
version: 1.0.0
---

> **BLUF: PASS.** SPR-033a delivers the Bevy-free core of the voxel viewer lane (`PRJ-003`, priority 2, D62; `CODEX/05_PROJECT/SPR-033_BevyVoxelSpike.md`) into Rehoboam:
> - **T-01a:** `viewer/` workspace with `rh-view-core`, integer-only 62³ voxel coordinates, `#![deny(unsafe_code)]` (V-R3D-14, V-VIEW-04), `binary-greedy-meshing` =0.5.2, and `viewer/tools/lock_age.py` enforcing the 14-day rule on direct pins;
> - **T-02:** `Chunk` data structure, binary greedy mesher, surface-preserving level-of-detail reduction (top-most solid block), chunk/volume content digests (`sha2`), and determinism tests;
> - **T-03:** `specs/viewer/terrain_v0.md` written terrain specification, `specs/viewer/terrain_ref.py` Python reference, `viewer/tools/export_worldgen_lite.py` heightfield export, committed goldens (`viewer/golden/terrain_v0.txt`, `specs/viewer/golden/terrain_v0_columns.txt`), differential tests between Rust voxelizer and Python reference;
> - **T-06:** `viewer-isolation` lint rule in `bin/lint_arch.py` with 4 negative/positive fixtures, PR #89 scope framework in `bin/ci_scope.py`, and `.github/workflows/viewer.yml` standalone workflow running `bin/check_viewer.sh`.
>
> Verification:
> - All 4 CI checks on PR #111 (`17f6ca7e`) passed with 100% success:
>   - `PR tier (bin/check_all.sh --part core)`: SUCCESS
>   - `Viewer workspace checks (bin/check_viewer.sh)`: SUCCESS
>   - `PR tier (bin/check_all.sh --part golden)`: SUCCESS
>   - `PR tier (bin/check_all.sh --part stream)`: SUCCESS
> - The viewer workspace builds and passes all unit, integration, and differential tests with zero dependencies on Bevy or the render stack.
> - Change class C1. Merged into `main` via merge commit `7083e649` pinned to head `17f6ca7e` (GOV-005 §6, GOV-009 §6).

# VER-SPR-033a: audit of SPR-033a

PR #111 merges with a merge commit, because the branch introduces a new workspace (`viewer/`), new CI workflow, and new tooling contracts whose ranges CI checks.

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh --part core` | **PASS**: 0 FAIL rows. All workspace unit tests, lints, manifests, and controls pass. | CI run on `17f6ca7e` |
| `bin/check_all.sh --part golden` | **PASS**: 0 FAIL rows. Release builds, golden runs, baselines, and ladders pass. | CI run on `17f6ca7e` |
| `bin/check_all.sh --part stream` | **PASS**: 0 FAIL rows. Parquet, sifter, tail, and serve pass. | CI run on `17f6ca7e` |
| `bin/check_viewer.sh` | **PASS**: 0 FAIL rows. Formatting, clippy, lock-age check, unit tests, differential tests, terrain goldens, and isolation fixtures pass. | CI run on `17f6ca7e` |
| `viewer-isolation` lint | **PASS**: Root workspace does not depend on viewer; viewer does not expose render stack into core. | `lint_arch.py` |
| `lock_age.py --check` | **PASS**: All direct pins in `viewer/Cargo.lock` >= 14 days old at lock entry. | `viewer/tools/lock_age.py` |
| Change class | C1 (tooling, viewer workspace, zero-engine-dependency viewer core) | PR body |

## 2. Plan Fulfillment against SPR-033 Stage A Tasks
| Task | Requirement | Status | Deliverables |
|---|---|---|---|
| **T-01a** | Viewer workspace & `rh-view-core` | **DONE** | `viewer/Cargo.toml`, `viewer/rh-view-core/`, `#![deny(unsafe_code)]`, `binary-greedy-meshing` =0.5.2 pin, `viewer/tools/lock_age.py` |
| **T-02** | Chunks, mesher, LOD, digests | **DONE** | `viewer/rh-view-core/src/chunk.rs`, `mesh.rs`, `lod.rs`, `digest.rs`, `tests/lod.rs`, `tests/culling.rs`, `tests/determinism.rs` |
| **T-03** | Terrain spec, reference, goldens | **DONE** | `specs/viewer/terrain_v0.md`, `specs/viewer/terrain_ref.py`, `export.rs`, `viewer/golden/terrain_v0.txt`, `tests/terrain_differential.rs`, `tests/terrain_golden.rs` |
| **T-06** | Isolation lint, scope rule, CI workflow | **DONE** | `bin/lint_arch.py` (`viewer-isolation`), `tests/lint_fixtures/viewer_isolation/`, `bin/ci_scope.py`, `.github/workflows/viewer.yml`, `bin/check_viewer.sh` |

## 3. Findings and Rulings
- **Isolated Workspace Design:** The separate `viewer/` workspace guarantees that graphic dependencies (wgpu, bevy) never bleed into the authoritative engine root `Cargo.lock`.
- **Differential Oracle:** The Python reference `terrain_ref.py` and Rust voxelizer test on 400 sample columns (`specs/viewer/golden/terrain_v0_columns.txt`) confirm bit-exact voxel material mapping.
- **Stage B Readiness:** Stage A completes all prerequisites for Stage B (`rh-view`, cameras, render-work LOD controller, HUD, bench scene). Stage B will pin Bevy `=0.20.0` at or after 2026-10-22 22:31Z per ruling V3.
