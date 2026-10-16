//! Test support for rh-snapshot's engine-level tests (SPR-007): a copy of rh-core's
//! `tests/support/mod.rs` (SPR-005), a configurable stub system and a helper that builds a spec
//! table in memory. rh-core's tests cannot use rh-snapshot (it depends on rh-core; Architect
//! ruling), so the tests that need the plane hash and a stub engine live here.
//!
//! A stub is a real `SystemModel`: descriptor, access, bindings and purposes are data, and the
//! step is a plain function, so each test states exactly what a system declares and does.
#![allow(
    dead_code,
    reason = "each test file uses a different subset of the helpers"
)]

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, Lag, Plane};
use rh_core::ownership::{Declaration, Source, SpecEntry, SpecTable, single_writer};
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, SimError, SpecPurpose, StepCtx,
    SystemDescriptor, SystemModel,
};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};

/// A step function.
pub type StepFn =
    fn(&Stub, PulseBinding, &mut SystemView<'_>, &mut StepCtx<'_>) -> Result<(), SimError>;

/// A stub system.
#[derive(Clone)]
pub struct Stub {
    pub id: &'static str,
    pub access: AccessSpec,
    pub bindings: Vec<PulseBinding>,
    pub purposes: Vec<&'static str>,
    pub step: StepFn,
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

    /// Boxes it for registration.
    pub fn boxed(self) -> Box<dyn SystemModel> {
        Box::new(self)
    }

    /// The spec entry this stub's declarations imply (writes = owns, reads = reads, purposes =
    /// rng): a spec the stub agrees with.
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
    fn invariants(&self, _: &WorldTables) -> Vec<InvariantResult> {
        Vec::new()
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
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
    }
}
