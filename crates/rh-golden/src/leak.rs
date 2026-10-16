//! The thread-identity leak, a planted test hook (VER-SPR-008 DEF-2), never used for a baseline.
//!
//! [`Leaky`] wraps `X7.smoke_market` and, after each of its Environment steps, adds to
//! `market.ticks` a per-thread step counter (`thread_local!`). On one thread the counter climbs
//! through the run; on N threads every parallel level runs on freshly spawned workers whose
//! counter starts again, so the state depends on the thread count. The 1-versus-N ladder
//! comparison must fail on it; a ladder run with it says so in its header (`leak thread`), so it
//! never passes as a clean one.
//!
//! REF: VER-SPR-008 DEF-2; `05.1` §6 (thread-count invariance).

use std::cell::Cell;

use rh_core::Phase;
use rh_core::access::AccessSpec;
use rh_core::agenda::TimerEntry;
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, ResampleCtx, SimError, StepCtx,
    SystemDescriptor, SystemModel,
};
use rh_core::time::SimTime;
use rh_core::world::{SystemView, WorldTables};

thread_local! {
    static STEPS: Cell<i64> = const { Cell::new(0) };
}

/// A system with the thread-identity leak planted after its Environment steps.
pub struct Leaky(pub Box<dyn SystemModel>);

/// The column the leak writes: `X7.smoke_market`'s own.
pub const COLUMN: &str = "market.ticks";

impl SystemModel for Leaky {
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
        if binding.phase == Phase::Environment {
            let steps = STEPS.with(|s| {
                s.set(s.get() + 1);
                s.get()
            });
            view.write(COLUMN)?[0] += steps;
        }
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
