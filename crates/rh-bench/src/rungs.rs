//! The ladder's manifest worlds as V-PERF-11 measures them (SPR-013 T-13): rung 0 (`solo`), rung 1
//! (`household`) and rung 2 (`hamlet`), each built exactly as `rh-cli run <world>` builds it
//! (`rh-golden`'s world setup: the manifest compiled against the Registry, genesis, the built
//! systems registered against `specs/systems`), with no material inputs, so the per-tick
//! allocations of the window are the engine's and M2's own.
//!
//! REF: `docs/09` V-PERF-11; `CODEX/05_PROJECT/SPR-013_WalkingSkeletonC.md` T-13.

use std::path::Path;

use rh_core::engine::Engine;
use rh_core::register::register;
use rh_core::time::{Calendar, SimTime};
use rh_ids::BranchSalt;

/// The rungs measured, in ladder order.
pub const RUNGS: [&str; 3] = ["solo", "household", "hamlet"];

/// The engine of manifest world `world` at genesis, its calendar, and its seed.
///
/// # Errors
/// The world's setup or registration error, as text.
pub fn engine(repo: &Path, world: &str, threads: usize) -> Result<(Engine, Calendar, u64), String> {
    let w = rh_golden::world::setup(repo, world, None, false).map_err(|e| e.0)?;
    let registered = register((w.systems)(false), &w.specs).map_err(|e| e.to_string())?;
    let mut e = Engine::new(
        w.calendar,
        w.seed,
        BranchSalt::from_raw(0),
        registered,
        w.tables.clone(),
        SimTime::EPOCH,
    );
    e.set_provenance(w.provenance.clone());
    e.set_threads(threads);
    Ok((e, w.calendar, w.seed))
}
