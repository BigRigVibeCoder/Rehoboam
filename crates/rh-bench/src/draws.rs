//! The world of the draw-cost bench (`benches/draws.rs`; SPR-011 T-02, BCK-002 A-37).
//!
//! One system, bound to the Day pulse in `Vital`, makes [`DRAWS`] keyed draws per step, one per
//! person, through `StepCtx::rng` or `StepCtx::stream`, or none. Every draw pays the stream guard
//! (a `RefCell` borrow and one `BTreeMap` entry lookup, in the system's fixed slot for the instant
//! since A-37 and DEF-4)
//! and the epoch lookup (a `BTreeMap` lookup, since A-29); a person a material input touched draws
//! under its own `K1` as well. The bench compares one instant of each mode with the empty one and
//! with [`raw_draws`], the same draws straight from `rh-ids`, so the per-draw cost of the step
//! context is read off the counts.
//!
//! REF: `CODEX/05_PROJECT/SPR-011_WalkingSkeleton.md` T-02 (A-37); `crates/rh-core/src/system.rs`.

use std::collections::BTreeSet;

use rh_core::access::{AccessSpec, ColumnWrite, Plane};
use rh_core::engine::{Engine, InputError, MaterialInput};
use rh_core::ownership::{Declaration, Source, SpecEntry, SpecTable, single_writer};
use rh_core::register::{RegistrationError, register, system_id};
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, SimError, SpecPurpose, StepCtx,
    SystemDescriptor, SystemModel,
};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_ids::{BranchSalt, EntityRef, PersonId, PurposeId, StreamKey, SystemKey};

/// Draws per step: one per person.
pub const DRAWS: u64 = 1_000;
/// The bench system's id.
pub const SYSTEM: &str = "X1.bench_drawer";
/// One day, in minutes.
pub const DAY: i64 = 1_440;
const SEED: u64 = 9;
const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const COLUMN: &str = "bench.draws";

/// What the bench system does each step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// No draw: the cost of the instant and the step.
    Empty,
    /// [`DRAWS`] draws by `StepCtx::rng`, one per person.
    Rng,
    /// [`DRAWS`] streams by `StepCtx::stream`, one draw from each.
    Stream,
}

enum Draw {
    Bench,
}

impl SpecPurpose for Draw {
    const ALL: &'static [Self] = &[Self::Bench];
    fn name(&self) -> &'static str {
        "bench"
    }
}

struct Drawer(Mode);

fn access() -> AccessSpec {
    AccessSpec {
        writes: vec![ColumnWrite {
            column: COLUMN,
            plane: Plane::A,
        }],
        ..AccessSpec::default()
    }
}

impl SystemModel for Drawer {
    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: SYSTEM,
            fidelity: "F1",
            spec_version: "0.0.0",
        }
    }
    fn access(&self) -> AccessSpec {
        access()
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        vec![PulseBinding {
            pulse: Pulse::Day,
            phase: Phase::Vital,
        }]
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        Draw::names()
    }
    fn params_schema(&self) -> ParamSchema {
        ParamSchema::default()
    }
    fn step(
        &self,
        _: PulseBinding,
        world: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let mut acc = 0_u64;
        for p in 0..DRAWS {
            let person = PersonId::from_raw(p);
            acc ^= match self.0 {
                Mode::Empty => p,
                Mode::Rng => ctx.rng(&Draw::Bench, person, 0)?,
                Mode::Stream => ctx.stream(&Draw::Bench, person)?.next_u64(),
            };
        }
        world.write(COLUMN)?[0] = acc.cast_signed();
        Ok(())
    }
    fn invariants(&self, _: &WorldTables) -> Vec<InvariantResult> {
        Vec::new()
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

fn specs() -> SpecTable {
    let declaration: Declaration = access().declaration(SYSTEM);
    let entry = SpecEntry {
        owns: declaration.writes.clone(),
        reads: Vec::new(),
        rng: Draw::names().into_iter().map(str::to_owned).collect(),
        schedule: vec![("Day".to_owned(), "Vital".to_owned())],
        lags: Vec::new(),
        commands: Vec::new(),
        emits: Vec::new(),
    };
    let (owners, _) = single_writer(std::slice::from_ref(&declaration), Source::Spec);
    SpecTable {
        owners,
        specs: [(SYSTEM.to_owned(), entry)].into_iter().collect(),
    }
}

/// An engine whose next instant is day 1, every person at epoch `epoch` (0 or 1: one material
/// input at day 0 has re-sampled them all when 1), ready to run one step in `mode`.
///
/// # Errors
/// [`RegistrationError`] if the bench system does not register (never in practice), or the
/// input or day 0 fails, as text.
pub fn engine(mode: Mode, epoch: u32) -> Result<Engine, String> {
    let registered = register(vec![Box::new(Drawer(mode))], &specs())
        .map_err(|e: RegistrationError| e.to_string())?;
    let mut tables = WorldTables::new();
    tables.insert(COLUMN, vec![0]);
    let mut e = Engine::new(
        CALENDAR,
        SEED,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    if epoch == 1 {
        let scope: BTreeSet<EntityRef> = (0..DRAWS)
            .map(|p| EntityRef::person(PersonId::from_raw(p)))
            .collect();
        e.submit_input(SimTime::EPOCH, MaterialInput { scope })
            .map_err(|err: InputError| format!("{err:?}"))?;
    }
    // Day 0 runs outside the measurement: the next instant, day 1, is the measured one.
    e.advance_until(SimTime::EPOCH, |_, _| {})
        .map_err(|err| err.to_string())?;
    Ok(e)
}

/// The keys of the bench system's draws straight from `rh-ids`: `K1` once, then `K2` per
/// person, as `StepCtx::rng` derives them at epoch 0.
#[must_use]
pub fn raw_keys() -> (SystemKey, PurposeId) {
    let key = SystemKey::new(SEED, BranchSalt::from_raw(0), system_id(SYSTEM), 0);
    (key, PurposeId::of("bench"))
}

/// [`DRAWS`] draws at day 1 straight from `rh-ids`, with no step context: the baseline the
/// `StepCtx` cases are compared with.
#[must_use]
pub fn raw_draws(key: &SystemKey, purpose: PurposeId) -> u64 {
    let mut acc = 0_u64;
    for p in 0..DRAWS {
        let stream: StreamKey = key.stream_key(purpose, p);
        acc ^= stream.draw(DAY, 0);
    }
    acc
}
