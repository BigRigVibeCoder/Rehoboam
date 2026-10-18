//! The smoke world as V-PERF-11 measures it (SPR-009 T-05): its engine, and the positive control's
//! allocating system.
//!
//! The engine is `rh-sys-smoke`'s seven systems on their specs (`worlds/smoke`, through
//! `rh-golden`'s spec table), seed 1, with no material inputs: a steady state, so the per-tick
//! allocations are the engine's and the systems' own, not an input's.
//!
//! [`Allocating`] wraps one system and, after each of its steps, zero-fills a fresh buffer
//! (`vec![0; 64]`, the `alloc_zeroed` path). With it the allocation gate must fail (V-PERF-11's
//! positive control); it changes no column, so the run is otherwise the clean one.
//!
//! REF: `docs/09` V-PERF-11; `CODEX/05_PROJECT/SPR-009_Benchmarks.md` T-05, ruling 5.

use std::hint::black_box;
use std::path::Path;

use rh_core::access::AccessSpec;
use rh_core::agenda::TimerEntry;
use rh_core::engine::Engine;
use rh_core::register::register;
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, ResampleCtx, SimError, StepCtx,
    SystemDescriptor, SystemModel,
};
use rh_core::time::SimTime;
use rh_core::world::{SystemView, WorldTables};
use rh_ids::BranchSalt;

/// The world seed of the measured run (the golden runs' seed).
pub const SEED: u64 = 1;

/// The system the positive control wraps.
pub const CONTROL_SYSTEM: &str = "X1.smoke_weather";

/// Bytes the positive control zero-fills after each step.
pub const CONTROL_BYTES: usize = 64;

/// A system that also allocates after every step: V-PERF-11's positive control.
pub struct Allocating(pub Box<dyn SystemModel<Index = ()>>);

impl SystemModel for Allocating {
    type Index = ();
    
    fn descriptor(&self) -> SystemDescriptor {
        self.0.descriptor()
    }
    fn access(&self) -> AccessSpec {
        self.0.access()
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        self.0.schedule()
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        self.0.rng_purposes()
    }
    fn params_schema(&self) -> ParamSchema {
        self.0.params_schema()
    }
    fn step(
        &self,
        binding: PulseBinding,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        self.0.step(binding, view, ctx)?;
        drop(black_box(vec![0_u8; black_box(CONTROL_BYTES)]));
        Ok(())
    }
    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult> {
        self.0.invariants(world)
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        self.0.declared_aggregates()
    }
    fn fire(
        &self,
        timer: &TimerEntry,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        self.0.fire(timer, view, ctx)
    }
    fn resample_timer(
        &self,
        timer: &TimerEntry,
        ctx: &ResampleCtx,
    ) -> Result<Option<SimTime>, SimError> {
        self.0.resample_timer(timer, ctx)
    }
}

/// The smoke engine at instant 0 with `threads` workers; with `control`, [`CONTROL_SYSTEM`] is
/// wrapped in [`Allocating`].
///
/// # Errors
/// The specs cannot be read, or the code and the specs disagree.
pub fn engine(repo: &Path, threads: usize, control: bool) -> Result<Engine, String> {
    let specs = rh_golden::run::spec_table(repo).map_err(|e| e.0)?;
    let systems = rh_sys_smoke::systems()
        .into_iter()
        .map(|s| {
            if control && s.descriptor().id == CONTROL_SYSTEM {
                Box::new(Allocating(s)) as Box<dyn SystemModel<Index = ()>>
            } else {
                s as Box<dyn SystemModel<Index = ()>>
            }
        })
        .collect();
    let registered = register(systems, &specs).map_err(|e| e.to_string())?;
    let mut e = Engine::new(
        rh_sys_smoke::CALENDAR,
        SEED,
        BranchSalt::from_raw(0),
        registered,
        rh_sys_smoke::initial_tables(),
        SimTime::EPOCH,
    );
    e.set_threads(threads);
    Ok(e)
}
