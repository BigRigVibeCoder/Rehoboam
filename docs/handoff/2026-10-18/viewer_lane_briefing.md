# Briefing: P2 Viewer Lane Execution (SPR-033a..SPR-035)

- **Author:** Architect (`AGT-001`)
- **Recipient:** Adjutant Agent 2 (`AGT-002-TOOLS`), Director, Red Team
- **Date:** 2026-10-18
- **Priority:** Priority 2 (Dedicated Observability Lane, off critical path)
- **Governing Plan:** [`CODEX/05_PROJECT/PRJ-003_BevyVoxelViewer.md`](file:///home/bdavidriggins/Documents/Rehoboam/CODEX/05_PROJECT/PRJ-003_BevyVoxelViewer.md) (Status: **APPROVED** under D51/D62/D63)

---

## 1. Current State: SPR-033a (Bevy-Free Core) Complete

- **PR:** [#111](https://github.com/BigRigVibeCoder/Rehoboam/pull/111) (`sprint/SPR-033a-viewer-core`, head `17f6ca7`)
- **Verification:** Passed all 22 gates in `bin/check_viewer.sh`.
- **Isolation Compliance:**
  - Workspace isolation enforced by `bin/lint_arch.py` (`viewer-isolation` rules a–e).
  - 4 positive/negative control fixtures verified.
  - Zero Bevy or rendering crates entered root engine lock (`Cargo.lock`).
  - Scoping rules in `bin/ci_scope.py` skip heavy simulation golden runs when modifying viewer-only paths.
- **Architect Action:** Review PR #111 and prepare merge commit once CI runs complete.

---

## 2. Next Up: SPR-033b (Bevy 0.20 Rendering Spike)

### Scope & Deliverables
1. **Bevy 0.20 Application (`viewer/rh-view`):**
   - Initialize the Bevy native application in `viewer/rh-view`.
   - Implement the `binary-greedy-meshing` pipeline consuming `rh-view-core` chunks.
   - Target voxel size: **0.35 m default** (Director D62), with LOD fallback to 0.70 m and 1.40 m when frame budget requires.
2. **Spectator Camera System:**
   - Free orbital camera, fly camera, and ground-level first-person walking camera.
3. **Live Panel over `A.sock`:**
   - Ingest live tick stream from `rh-cli run --socket` (local or SSH tunnel from `adjutant`).
4. **Performance Target:**
   - Stable $\ge 60$ FPS on integrated GPUs for the 62³ chunk observation bubble.
5. **Governance Constraint (The 14-Day Lock Rule):**
   - Direct dependencies added to `viewer/Cargo.toml` must satisfy the 14-day age rule (`viewer/tools/lock_age.py`). Bevy 0.20.0 enters the lock at or after 2026-10-22 22:31Z.

---

## 3. Horizon: SPR-034 (Stream Ingestion & Plane-B Procedural Hydration)

- **Deliverables:**
  - Fast-forward stream reader over `rh-protocol` and `rh-stream` with catch-up logic.
  - Plane-B procedural building grammar (cruck and box-framed dwellings, church, timber palisades).
  - Hydration determinism invariant (`V-R3D-13`): byte-identical voxel output across runs, threads, and rewinds for identical seed and state.
- **Integration Points:**
  - Connects to Adjutant 1's work on SPR-027 (`crates/rh-golden/src/governor.rs` and `control.sock`).
