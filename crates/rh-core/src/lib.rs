//! `rh-core`: the world, its tables, the scheduler, time, the RNG context and invariants.
//!
//! Plane A, the core. `rh-core` has no I/O; every host (CLI, server, Python wheel) wraps it.
//! Plane-A code never reads planes B or C (ADR-0024).
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.1_simulation_core.md`.
//!
//! Created by SPR-001 (repository bootstrap); SPR-005 fills the world and the scheduler.
//! SPR-010 adds `fixed`: the fixed-point and money types (DET-3).
//! SPR-005 adds `time` (sim-time, calendar, pulses).
#![forbid(unsafe_code)]

pub mod fixed;
pub mod time;
