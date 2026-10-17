---
id: SPR-033
title: "Bevy 0.20 native voxel viewer spike"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [PRJ-003, ADR-0035, ADR-0036, ADR-0037, SPR-027]
created: 2026-10-17
updated: 2026-10-17
version: 1.1.0
---

> **BLUF:** Developer: AGT-002-TOOLS. Testers: AGT-003-DET, AGT-003-IF, and the Red Team. Create `crates/rh-view`, a native Rust 3D voxel spectator and diorama viewer built on Bevy 0.20, with high-performance binary greedy meshing and an orbital camera. Serves as the primary visual testing and observability harness for the Red Team. Operates in the Tool plane; no Plane A engine crate may depend on it. Scope-bounded; about 2 agent-weeks.

# SPR-033 — Bevy 0.20 Native Voxel Viewer Spike

- **Roadmap item:** `PRJ-003` §4 (SPR-033) · **Phase:** P1a-Tool (Tool Plane)
- **Developer:** `AGT-002-TOOLS` (Rust monorepo developer)
- **Auditor / Tester:** Red Team & `AGT-003-IF` (testing visual parity and zero-interference)
- **Depends on:** Ratification of Decision D51 / ADR-0036.
- **Binding contracts:** ADR-0030 (headless engine), ADR-0031 (fidelity by zoom), ADR-0036 (Plane B hydration), ADR-0037 (view protocol), GOV-003 (architecture lints).
- **Requirements covered:** V-VIEW-01, V-VIEW-02, V-VIEW-03, V-VIEW-04.

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C1 (Tool crate: `crates/rh-view` has no effect on Plane A state hash or simulation goldens) |
| Planes touched (ADR-0024) | Tool plane / Plane B hydration (no Plane A engine code touched) |
| Owned columns written | None (read-only spectator) |
| Reads (via `rh-contracts`) | Read-only stream records via `rh-protocol` |
| RNG purposes added | None (Plane A PRNG untouched; visual particle seeds isolated to Plane B) |
| Golden-hash effect | None (`smoke` and `solo` golden ladders unchanged) |
| Determinism lints (GOV-003 §2) | Must pass; `#![deny(unsafe_code)]` in `crates/rh-view` |
| Tests that gate this sprint | V-VIEW-03 (60 FPS on 64×64×32 chunk), V-VIEW-04 (zero reverse dependencies); `bin/check_all.sh` |
| Docs updated in the same sprint | `PRJ-003`, `docs/red_team/dashboard.html` |

## Tasks

### T-01: Scaffolding `crates/rh-view` in Cargo Workspace
- **Deliverable:** `crates/rh-view/Cargo.toml`, `crates/rh-view/src/lib.rs`, `crates/rh-view/src/main.rs`. Dependencies: `bevy` 0.20, `wgpu`, `rh-protocol`, `bytemuck`. Added to root workspace `Cargo.toml`.
- **Acceptance:** `cargo check -p rh-view` and `cargo test -p rh-view` compile cleanly without warnings. Architecture lint confirms zero reverse dependencies from `rh-core`.
- **Status:** [ ] not started

### T-02: Binary Greedy Chunk Mesher (`rh-view::mesher`)
- **Deliverable:** `crates/rh-view/src/mesher.rs`. Implements pure bitwise binary greedy meshing for 64×64×32 voxel chunk volumes:
  - Culls hidden interior faces.
  - Merges coplanar adjacent quad faces along X, Y, and Z planes.
  - Emits compact 32-bit packed vertex attributes (position, normal, voxel material ID).
- **Acceptance:** Unit tests verify face culling on solid cubes and hollow structures. Benchmark proves 64×64×32 chunk meshes in $\le 3.0\text{ ms}$ on a single core.
- **Status:** [ ] not started

### T-03: Render Pipeline & Procedural Test Diorama
- **Deliverable:** `crates/rh-view/src/render.rs`, `crates/rh-view/src/camera.rs`.
  - Sets up Bevy 3D scene with directional sun, atmospheric scattering, and ambient occlusion.
  - Procedural test diorama generator: spawns a 64×64 medieval keep, timber house, and river gorge from seed.
  - Smooth orbital camera controller: mouse drag orbit, mouse wheel zoom, keyboard pan.
- **Acceptance:** Running `cargo run -p rh-view -- --demo` renders the diorama at $\ge 60\text{ FPS}$ with interactive camera navigation.
- **Status:** [ ] not started

### T-04: Non-Interference Benchmark & Architecture Gate
- **Deliverable:** `tests/test_rh_view_isolation.rs` and `bin/check_all.sh` verification.
- **Acceptance:**
  - Automated test confirms running `rh-view` alongside `rh-cli run solo` causes $\le 0.5\%$ delta in sim tick throughput (V-VIEW-01).
  - Architecture linter asserts `rh-core` and `rh-snapshot` do not transitively depend on `bevy` or `rh-view`.
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-IF | Interface boundaries: zero reverse dependencies from core crates to `rh-view` | `VER-SPR-033` |
| AGT-003-DET | Simulation hash non-interference: Plane A goldens produce identical hashes with `rh-view` active | `VER-SPR-033` |
