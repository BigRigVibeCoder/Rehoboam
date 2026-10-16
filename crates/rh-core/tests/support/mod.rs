//! Shared test support for rh-core's integration tests (SPR-005): a configurable stub system and
//! helpers that build spec tables in memory.
//!
//! A stub is a real `SystemModel`: descriptor, access, bindings and purposes are data, and the
//! step is a plain function, so each test states exactly what a system declares and does.
#![allow(
    dead_code,
    reason = "each test file uses a different subset of the helpers"
)]
#![allow(
    clippy::expect_used,
    reason = "test helpers: a bad fixture must fail the test loudly"
)]

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, Lag, Plane};
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, SimError, SpecPurpose, StepCtx,
    SystemDescriptor, SystemModel,
};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_registry::ownership::{Ownership, SystemSpec, load_specs, registry_system_ids};

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

    /// The spec this stub's declarations imply (writes = owns, reads = reads, purposes = rng):
    /// a spec the stub agrees with.
    pub fn spec(&self) -> SystemSpec {
        let mut toml = format!("[system]\nid = \"{}\"\n", self.id);
        for w in &self.access.writes {
            let _ = write!(
                toml,
                "[[owns]]\ncolumn = \"{}\"\nplane = \"{}\"\ntype = \"i64\"\n",
                w.column,
                w.plane.name()
            );
        }
        for r in &self.access.reads {
            let _ = write!(
                toml,
                "[[reads]]\ncolumn = \"{}\"\nowner = \"{}\"\n",
                r.column, r.owner
            );
        }
        for p in &self.purposes {
            let _ = write!(toml, "[[rng]]\npurpose = \"{p}\"\n");
        }
        SystemSpec::from_toml(Path::new(&format!("{}.odd.toml", self.id)), &toml)
            .expect("stub spec")
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

/// The repository root.
pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The ownership table of the real specs (`specs/systems`), as `rh-cli ownership` builds it.
pub fn real_ownership() -> Ownership {
    let ids =
        registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).expect("registry");
    let specs = load_specs(&repo().join("specs/systems")).expect("specs");
    Ownership::build(&specs, &ids)
}

/// An ownership table from in-memory specs. The Registry check is the lint's business; tests
/// that build synthetic systems pass their ids as the Registry.
pub fn ownership(specs: &[SystemSpec]) -> Ownership {
    let ids: BTreeSet<String> = specs.iter().map(|s| s.id.clone()).collect();
    Ownership::build(specs, &ids)
}

/// An ownership table in which every stub's spec says exactly what the stub declares.
pub fn agreeing_ownership(stubs: &[Stub]) -> Ownership {
    ownership(&stubs.iter().map(Stub::spec).collect::<Vec<_>>())
}
