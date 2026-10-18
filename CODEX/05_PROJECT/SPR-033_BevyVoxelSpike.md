---
id: SPR-033
title: "Bevy 0.20 native voxel viewer spike"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [PRJ-003, ADR-0030, ADR-0031, ADR-0035, ADR-0036, SPR-026, SPR-027]
created: 2026-10-17
updated: 2026-10-17
version: 1.2.0
---

> **BLUF:**
> - **What.** Create `viewer/rh-view`, a native Rust 3D voxel spectator and diorama viewer on Bevy 0.20. It has binary greedy meshing, a procedural demo diorama and an orbital camera, and lives in its own Cargo workspace.
> - **Why.** It is the first sprint of the Red Team's primary visual testing and observability harness (PRJ-003, ADR-0036).
> - **Boundaries.** Tool plane, read-only. No engine crate may depend on it, and Bevy never enters the engine's lock.
> - **Who.** Developer: AGT-002-TOOLS. Tester: AGT-003-IF and AGT-003-DET, one session on a different model from the Developer's. The Red Team audits.
> - **Size and class.** About 3 agent-days (GOV-005 §3; the Red Team's 2 agent-weeks is PRJ-003's total for SPR-033 to SPR-035). C1.
> - **When.** It starts after SPR-027's materializer lands (the director, D51).

# SPR-033 — Bevy 0.20 Native Voxel Viewer Spike

- **Roadmap item:** `PRJ-003` §4 (SPR-033); `docs/11` viewer lane V.1 · **Phase:** viewer lane (tool plane)
- **Developer:** `AGT-002-TOOLS`
- **Tester:** `AGT-003-IF` and `AGT-003-DET` (one session, on a different model from the Developer's). The Red Team audits.
- **Depends on:** D51 / ADR-0036 (ratified 2026-10-17). Starts after SPR-027 lands, per the director.
- **Binding contracts:**
  - ADR-0036 (the viewer: boundaries, determinism, findings become tests, procedural assets);
  - ADR-0030 (the headless engine, harnesses);
  - ADR-0031 (fidelity by zoom);
  - ADR-0035 directive 5, as amended by ADR-0036;
  - GOV-003 (architecture lints);
  - `docs/09` V-R3D-14.
- **Requirements covered:** R3D-11 (part: the isolation and the procedural diorama). V-VIEW-03 and V-VIEW-04 (PRJ-003 §5), the latter gated as V-R3D-14.

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C1. A tool crate in its own workspace; no effect on plane-A state, hashes or goldens |
| Planes touched (ADR-0024) | Tool plane; plane-B hydration of a demo diorama. No engine code touched |
| Owned columns written | None (a read-only spectator) |
| Reads (via `rh-contracts`) | None in this sprint: the demo diorama is seeded locally. Stream reads begin in SPR-034 |
| RNG purposes added | None in plane A. The demo diorama's keyed draws are plane-B purposes local to `rh-view` |
| Golden-hash effect | None: every golden ladder (`smoke`, `solo`, `household`, `hamlet`) is unchanged |
| Determinism lints (GOV-003 §2) | Pass. `#![deny(unsafe_code)]` in `rh-view`'s own code |
| Assets | Procedural only, or CC0. No art extracted from third-party games (PRJ-003 Invariant 5) |
| Tests that gate this sprint | V-R3D-14 (isolation); the mesher's unit tests and benchmark; `bin/check_all.sh`. V-VIEW-03 (60 FPS) is measured and reported on the director's machine |
| Docs updated in the same sprint | `PRJ-003`; `05.0` §6 (`viewer/rh-view` built); `docs/09` V-R3D-14's method line; `CLAUDE.md`'s check list (the viewer job) |

## Tasks

### T-01: The `viewer/` workspace and `rh-view`
- **Deliverable:**
  - `viewer/Cargo.toml` (its own workspace), `viewer/Cargo.lock`, `viewer/rh-view/Cargo.toml`, `src/lib.rs` and `src/main.rs`.
  - Dependencies: `bevy` at an exact 0.20 pin, at least two weeks old, with the features the demo needs, and `bytemuck`. No direct `wgpu` dependency unless Bevy's re-export is insufficient.
  - **Not** a member of the root workspace.
  - The crate declares `plane = "tool"`, `kind = "tool"`.
- **Acceptance:**
  - `cargo check` and `cargo test` in `viewer/` compile cleanly, with no warnings.
  - The root workspace's `Cargo.lock` is byte-identical to `main`'s.
  - `cargo deny` in `viewer/` keeps crates.io-only sources and no wildcards. Licences go in a ledger, as in the root (ADR-0021, personal-use mode).
- **Status:** [ ] not started

### T-02: Binary greedy chunk mesher (`rh-view::mesher`)
- **Deliverable:** `viewer/rh-view/src/mesher.rs`. Pure, integer binary greedy meshing for 64×64×32 voxel chunks:
  - culls hidden interior faces;
  - merges coplanar adjacent quad faces along X, Y and Z;
  - emits compact 32-bit packed vertex attributes (position, normal, voxel material ID).
- **Acceptance:**
  - Unit tests check face culling on solid cubes and on hollow structures.
  - A property test with fixed-seed SplitMix64 random chunks confirms that the merged quads cover exactly the visible faces.
  - A benchmark shows a 64×64×32 chunk meshes in ≤ 3.0 ms on one core.
- **Status:** [ ] not started

### T-03: Render pipeline & procedural demo diorama
- **Deliverable:** `viewer/rh-view/src/render.rs`, `camera.rs` and `demo.rs`:
  - a Bevy 3D scene with a directional sun, atmospheric scattering and ambient occlusion;
  - a procedural demo diorama generator: a 64×64 medieval keep, a timber house and a river gorge, from a seed, with keyed integer draws;
  - an orbital camera: mouse-drag orbit, mouse-wheel zoom, keyboard pan.
- **Acceptance:**
  - `cargo run -p rh-view -- --demo` in `viewer/` renders the diorama interactively. The FPS is reported on the director's machine (V-VIEW-03: ≥ 60 FPS).
  - A headless test checks that the demo diorama's voxel content digest is equal across two runs and two thread counts. This is V-R3D-13's method, on the demo world.
- **Status:** [ ] not started

### T-04: Isolation gate & the viewer's CI job
- **Deliverable:**
  - `bin/lint_arch.py`: no crate under `crates/` has a path or registry dependency on anything under `viewer/`, and the root `Cargo.lock` holds no `bevy*` package. A planted control must FAIL.
  - A CI job, path-filtered to `viewer/**` and to the engine crates `rh-view` reads, that builds and tests `viewer/` headless. Report its minutes.
- **Acceptance:**
  - V-R3D-14's lint and lock checks pass, and their control fails by name.
  - The CI job is green.
  - Measured with the viewer's demo running alongside it on the same machine, `rh-cli run solo` keeps at least 95% of its headless throughput (V-PERF-09's method). Report the delta; the Red Team's target is ≤ 0.5%.
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-IF | Interface boundaries: no reverse dependency from the engine to `viewer/`; the root lock unchanged; the lint's control (ADR-0036 §2, V-R3D-14) | `VER-SPR-033-TEST` |
| AGT-003-DET | Plane-A non-interference: every golden ladder is identical with the demo running; the demo's voxel digest is deterministic (V-R3D-13's method) | `VER-SPR-033-TEST` |
| Red Team (audit) | Visual fidelity of the demo diorama. Any finding becomes an automated check (ADR-0036 §4) | Red Team notes |
