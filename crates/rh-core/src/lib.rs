//! `rh-core`: the world, its tables, the scheduler, time, the RNG context and invariants.
//!
//! Plane A, the core. `rh-core` has no I/O; every host (CLI, server, Python wheel) wraps it.
//! Plane-A code never reads planes B or C (ADR-0024).
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.1_simulation_core.md`.
//!
//! Created by SPR-001 (repository bootstrap); SPR-005 fills the world and the scheduler.
//! SPR-010 adds `fixed`: the fixed-point and money types (DET-3).
//! SPR-005 adds `time` (sim-time, calendar, pulses), `phase` (the phase order), `system` (the
//! `SystemModel` contract), `access` (`AccessSpec`), `ownership` (the single-writer and plane rules), `world` (tables
//! and the step view),
//! `register` (the conflict checker and RNG purpose check), `schedule` (the derived scheduler) and
//! `engine` (the instant loop).
//! SPR-006 adds `agenda` (timers keyed by `(time, phase, entity, seq)`), `command` (the lifecycle
//! command queues) and `crash` (crash artifacts), and gives the engine the per-level merge, the
//! event bus (`rh-events`), material-input re-sampling and the invariants in `Bookkeeping`.
//! SPR-009 adds `observe`: boundary notices for a host's observer, with no clock (ruling 4).
//! SPR-011 adds `ledger`: the person ledger's compaction and the life-course log's rules
//! (ruling 5), and `fixed::Prob32` (ruling 4).
#![forbid(unsafe_code)]

pub mod access;
pub mod agenda;
pub mod command;
pub mod crash;
pub mod engine;
pub mod fixed;
pub mod ledger;
pub mod observe;
pub mod ownership;
pub mod phase;
pub mod register;
pub mod schedule;
pub mod state;
pub mod system;
pub mod time;
pub mod world;

pub use phase::Phase;
pub use rh_events::RoleId;
pub use system::Fixed;
pub use time::Pulse;
