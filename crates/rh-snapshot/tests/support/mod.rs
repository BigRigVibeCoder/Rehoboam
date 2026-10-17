//! Test support for rh-snapshot's engine-level tests (SPR-007): a copy of rh-core's
//! `tests/support/mod.rs` (SPR-005, SPR-006), a configurable stub system and a helper that builds a
//! spec table in memory. rh-core's tests cannot use rh-snapshot (it depends on rh-core; Architect
//! ruling), so the tests that need the plane hash and a stub engine live here.
//!
//! A stub is a real `SystemModel`: descriptor, access, bindings and purposes are data, and the
//! step is a plain function, so each test states exactly what a system declares and does.
#![allow(
    dead_code,
    reason = "each test file uses a different subset of the helpers"
)]

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, CommandSend, Lag, Plane};
use rh_core::agenda::TimerEntry;
use rh_core::ownership::{Declaration, Source, SpecEntry, SpecTable, single_writer};
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, ResampleCtx, SimError, SpecPurpose,
    StepCtx, SystemDescriptor, SystemModel,
};
use rh_core::time::SimTime;
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};

/// A step function.
pub type StepFn =
    fn(&Stub, PulseBinding, &mut SystemView<'_>, &mut StepCtx<'_>) -> Result<(), SimError>;

/// A timer handler (SPR-006).
pub type FireFn =
    fn(&Stub, &TimerEntry, &mut SystemView<'_>, &mut StepCtx<'_>) -> Result<(), SimError>;

/// A re-sample handler (SPR-006).
pub type ResampleFn = fn(&Stub, &TimerEntry, &ResampleCtx) -> Result<Option<SimTime>, SimError>;

/// An invariant check (SPR-006).
pub type InvariantFn = fn(&Stub, &WorldTables) -> Vec<InvariantResult>;

/// A stub system.
#[derive(Clone)]
pub struct Stub {
    pub id: &'static str,
    pub access: AccessSpec,
    pub bindings: Vec<PulseBinding>,
    pub purposes: Vec<&'static str>,
    pub step: StepFn,
    /// `None`: the trait's default (a loud error).
    pub fire: Option<FireFn>,
    /// `None`: the trait's default (a loud error).
    pub resample: Option<ResampleFn>,
    /// `None`: no invariants.
    pub check: Option<InvariantFn>,
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "it must have the StepFn signature"
)]
fn no_op(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    _: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    Ok(())
}

impl Stub {
    /// A stub that does nothing, bound to nothing, declaring nothing.
    pub fn new(id: &'static str) -> Self {
        Self {
            id,
            access: AccessSpec::default(),
            bindings: Vec::new(),
            purposes: Vec::new(),
            step: no_op,
            fire: None,
            resample: None,
            check: None,
        }
    }

    /// Adds a plane-A write.
    pub fn writes(mut self, column: &'static str) -> Self {
        self.access.writes.push(ColumnWrite {
            column,
            plane: Plane::A,
        });
        self
    }

    /// Adds a write in another plane.
    pub fn writes_in(mut self, column: &'static str, plane: Plane) -> Self {
        self.access.writes.push(ColumnWrite { column, plane });
        self
    }

    /// Adds a read.
    pub fn reads(mut self, column: &'static str, owner: &'static str, lag: Lag) -> Self {
        self.access.reads.push(ColumnRead { column, owner, lag });
        self
    }

    /// Adds a binding.
    pub fn at(mut self, pulse: Pulse, phase: Phase) -> Self {
        self.bindings.push(PulseBinding { pulse, phase });
        self
    }

    /// Declares the purposes of a generated-style enum.
    pub fn purposes_of<P: SpecPurpose>(mut self) -> Self {
        self.purposes = P::names();
        self
    }

    /// Sets the step function.
    pub fn stepping(mut self, step: StepFn) -> Self {
        self.step = step;
        self
    }

    /// Sets the timer handler.
    pub fn firing(mut self, fire: FireFn) -> Self {
        self.fire = Some(fire);
        self
    }

    /// Sets the re-sample handler.
    pub fn resampling(mut self, resample: ResampleFn) -> Self {
        self.resample = Some(resample);
        self
    }

    /// Sets the invariant check.
    pub fn checking(mut self, check: InvariantFn) -> Self {
        self.check = Some(check);
        self
    }

    /// Declares a command submission to `target`.
    pub fn sends(mut self, command: &'static str, target: &'static str) -> Self {
        self.access.sends.push(CommandSend { command, target });
        self
    }

    /// Declares a command it owns and applies.
    pub fn applies(mut self, command: &'static str) -> Self {
        self.access.applies.push(command);
        self
    }

    /// Declares an event kind it emits.
    pub fn emits(mut self, kind: &'static str) -> Self {
        self.access.emits.push(kind);
        self
    }

    /// Boxes it for registration.
    pub fn boxed(self) -> Box<dyn SystemModel> {
        Box::new(self)
    }

    /// The spec entry this stub's declarations imply (writes = owns, reads = reads, purposes =
    /// rng, sends = commands, emits = emits): a spec the stub agrees with.
    pub fn spec(&self) -> (Declaration, SpecEntry) {
        let declaration = self.access.declaration(self.id);
        let entry = SpecEntry {
            owns: declaration.writes.clone(),
            reads: self.access.read_columns(),
            rng: self.purposes.iter().map(|p| (*p).to_owned()).collect(),
            schedule: self
                .bindings
                .iter()
                .map(|b| (b.pulse.name().to_owned(), b.phase.name().to_owned()))
                .collect(),
            lags: self
                .access
                .reads
                .iter()
                .map(|r| (r.column.to_owned(), r.lag.name().to_owned()))
                .collect(),
            commands: self
                .access
                .sends
                .iter()
                .map(|c| (c.target.to_owned(), c.command.to_owned()))
                .collect(),
            emits: self.access.emits.iter().map(|k| (*k).to_owned()).collect(),
            from_manifest: Vec::new(),
        };
        (declaration, entry)
    }
}

impl SystemModel for Stub {
    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: self.id,
            fidelity: "F1",
            spec_version: "0.0.0",
        }
    }
    fn access(&self) -> AccessSpec {
        self.access.clone()
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        self.bindings.clone()
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        self.purposes.clone()
    }
    fn params_schema(&self) -> ParamSchema {
        ParamSchema::default()
    }
    fn step(
        &self,
        binding: PulseBinding,
        world: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        (self.step)(self, binding, world, ctx)
    }
    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult> {
        self.check.map(|f| f(self, world)).unwrap_or_default()
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
    fn fire(
        &self,
        timer: &TimerEntry,
        world: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        match self.fire {
            Some(f) => f(self, timer, world, ctx),
            None => Err(SimError::System {
                system: self.id,
                message: format!("{} fired, but the stub has no fire handler", timer.id()),
            }),
        }
    }
    fn resample_timer(
        &self,
        timer: &TimerEntry,
        ctx: &ResampleCtx,
    ) -> Result<Option<SimTime>, SimError> {
        match self.resample {
            Some(f) => f(self, timer, ctx),
            None => Err(SimError::System {
                system: self.id,
                message: format!("{} re-sampled, but the stub has no handler", timer.id()),
            }),
        }
    }
}

/// A spec table in which every stub's spec says exactly what the stub declares, built as the lint
/// builds one (the owners from `single_writer` over the specs; the first owner keeps a contested
/// column).
pub fn agreeing_specs(stubs: &[Stub]) -> SpecTable {
    let specs: Vec<(Declaration, SpecEntry)> = stubs.iter().map(Stub::spec).collect();
    let declarations: Vec<Declaration> = specs.iter().map(|(d, _)| d.clone()).collect();
    let (owners, _) = single_writer(&declarations, Source::Spec);
    SpecTable {
        owners,
        specs: specs.into_iter().map(|(d, e)| (d.system, e)).collect(),
        registry: rh_core::ownership::RegistryTable::default(),
    }
}
