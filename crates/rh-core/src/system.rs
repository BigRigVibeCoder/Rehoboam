//! The `SystemModel` contract every simulation system implements (SPR-005 T-03; `05.1` §2).
//!
//! The trait maps one to one onto an ODD-R spec (`05.1` §2): `descriptor` → the header;
//! `access` → §2 reads and owned columns; `schedule` → §3; `rng_purposes` → the `[[rng]]` table;
//! `params_schema` → §7.0; `step` → §3 and §7; `invariants` → §9; `declared_aggregates` → §4.11.
//!
//! The skeleton `lint_odd.py --rust <id>` generates (ADR-0033 §3) compiles against this module:
//! it names [`Pulse`], [`Phase`] and [`Fixed`] unqualified, so a system module brings them in with
//! `use rh_core::{Fixed, Phase, Pulse};` and `include!`s its generated file. The skeleton's
//! `SCHEDULE` feeds [`PulseBinding::from_schedule`], and its `RngPurpose` enum (with its generated
//! `name()`) implements [`SpecPurpose`], the only way a system names an RNG purpose (ruling 2).
//!
//! Signatures that differ from the illustrative one in `05.1` §2 (the final ones are frozen here,
//! P0):
//! - `step` receives the [`PulseBinding`] (pulse and phase), not only the pulse: S6 and S7 bind the
//!   Day pulse in two phases and must know which one is running;
//! - `step` sees a [`SystemView`] (the world through the system's `AccessSpec`), and `invariants`
//!   the whole [`WorldTables`];
//! - `rng_purposes` is added (T-06, ruling 2).
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §2; ADR-0033 §3;
//! `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-03, T-06, rulings 1 and 2.

use core::fmt;

use rh_ids::{EntityKey, PurposeId, SystemKey};

use crate::access::AccessSpec;
use crate::phase::Phase;
use crate::time::{Pulse, SimTime};
use crate::world::{SystemView, WorldTables};

/// The fixed-point type of every generated `Params` field (ruling 1): `Q16_16`.
pub type Fixed = crate::fixed::Q16_16;

/// A system's identity: the spec header (`05.1` §2: `"M2.demography"`, fidelity `"F1"`, semver).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemDescriptor {
    /// The Registry system id, e.g. `M2.demography` (the skeleton's `SYSTEM_ID`).
    pub id: &'static str,
    /// The fidelity level, e.g. `F1`.
    pub fidelity: &'static str,
    /// The spec version the code implements (the skeleton's `SPEC_VERSION`).
    pub spec_version: &'static str,
}

/// One binding of a system to a pulse and a phase (`[[schedule]]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PulseBinding {
    /// The pulse at whose boundaries the system runs.
    pub pulse: Pulse,
    /// The phase of the instant it runs in.
    pub phase: Phase,
}

impl PulseBinding {
    /// The bindings of a generated `SCHEDULE` table.
    #[must_use]
    pub fn from_schedule(schedule: &[(Pulse, Phase)]) -> Vec<Self> {
        schedule
            .iter()
            .map(|&(pulse, phase)| Self { pulse, phase })
            .collect()
    }
}

/// The parameters a system takes (`05.1` §2 `params_schema`; ODD-R §7.0). Every parameter is a
/// [`Fixed`] (ruling 1); bounds come from the Registry at load.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParamSchema {
    /// Parameter names, in spec order (the fields of the skeleton's `Params`).
    pub names: Vec<&'static str>,
}

/// The outcome of one invariant check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvariantResult {
    /// The invariant's name in the spec.
    pub name: &'static str,
    /// Whether it holds.
    pub holds: bool,
    /// What was found, when it does not hold.
    pub detail: String,
}

/// A declared aggregate: a stylized-fact target or a conserved quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AggregateSpec {
    /// The aggregate's name.
    pub name: &'static str,
    /// Whether it is conserved (`true`) or a calibration target (`false`).
    pub conserved: bool,
}

/// An error from a system step. Every variant fails the run loudly (GOV-004).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimError {
    /// A read of a column the system did not declare.
    UndeclaredRead {
        /// The system.
        system: &'static str,
        /// The column.
        column: String,
    },
    /// A write to a column the system does not own.
    UnownedWrite {
        /// The system.
        system: &'static str,
        /// The column.
        column: String,
    },
    /// A declared column that does not exist in the world: fatal, never created at run time (G-c).
    MissingColumn {
        /// The system.
        system: &'static str,
        /// The column.
        column: String,
    },
    /// A draw under a purpose the system did not register (ruling 2).
    UndeclaredPurpose {
        /// The system.
        system: &'static str,
        /// The purpose name.
        purpose: &'static str,
    },
    /// A system-specific failure.
    System {
        /// The system.
        system: &'static str,
        /// What failed.
        message: String,
    },
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UndeclaredRead { system, column } => {
                write!(
                    f,
                    "{system} read {column}, which its AccessSpec does not declare"
                )
            }
            Self::UnownedWrite { system, column } => {
                write!(f, "{system} wrote {column}, which it does not own")
            }
            Self::MissingColumn { system, column } => {
                write!(
                    f,
                    "{system} accesses {column}, which the world does not have (columns are created only from declared writes)"
                )
            }
            Self::UndeclaredPurpose { system, purpose } => {
                write!(
                    f,
                    "{system} drew under RNG purpose {purpose}, which it did not register"
                )
            }
            Self::System { system, message } => write!(f, "{system}: {message}"),
        }
    }
}

impl std::error::Error for SimError {}

/// An RNG purpose enum generated from a spec's `[[rng]]` table (`lint_odd.py --rust`, the
/// `RngPurpose` enum). Ruling 2: this is the only way a system names a purpose.
///
/// The system module implements it next to the `include!`: `ALL` lists the variants and `name`
/// returns the generated `RngPurpose::name()`. Registration checks every name against the spec
/// (T-06), so a stale or incomplete list fails registration (or is reported) rather than drawing
/// from the wrong stream.
pub trait SpecPurpose: Sized + 'static {
    /// Every variant, in spec order.
    const ALL: &'static [Self];

    /// The spec name of this purpose (`[[rng]] purpose`).
    fn name(&self) -> &'static str;

    /// Every name, in spec order: what `SystemModel::rng_purposes` returns.
    #[must_use]
    fn names() -> Vec<&'static str> {
        Self::ALL.iter().map(Self::name).collect()
    }
}

/// The step context: the keyed RNG of the running system at the current instant
/// (`05.1` §2 rule 6: all randomness comes from `ctx.rng(purpose, entity, k)`).
pub struct StepCtx<'a> {
    system: &'static str,
    key: SystemKey,
    t: SimTime,
    purposes: &'a [&'static str],
}

impl<'a> StepCtx<'a> {
    /// A context for one system step.
    pub(crate) fn new(
        system: &'static str,
        key: SystemKey,
        t: SimTime,
        purposes: &'a [&'static str],
    ) -> Self {
        Self {
            system,
            key,
            t,
            purposes,
        }
    }

    /// The current instant.
    #[must_use]
    pub fn now(&self) -> SimTime {
        self.t
    }

    /// Draw `k` of `purpose` for `entity` at the current instant:
    /// `H(world_seed, branch_salt, system_id, epoch, purpose, entity, t, k)`.
    ///
    /// `epoch` is 0 until material inputs exist to re-sample an entity (ADR-0025 item 3).
    ///
    /// # Errors
    /// [`SimError::UndeclaredPurpose`] if `purpose` is not one the system registered.
    pub fn rng<P: SpecPurpose>(
        &self,
        purpose: &P,
        entity: impl EntityKey,
        k: u64,
    ) -> Result<u64, SimError> {
        let name = purpose.name();
        if !self.purposes.contains(&name) {
            return Err(SimError::UndeclaredPurpose {
                system: self.system,
                purpose: name,
            });
        }
        let stream = self
            .key
            .stream_key(PurposeId::of(name), entity.entity_key());
        Ok(stream.draw(self.t.minutes(), k))
    }
}

/// The contract every simulation system implements (`05.1` §2).
pub trait SystemModel: Send + Sync {
    /// The spec header.
    fn descriptor(&self) -> SystemDescriptor;
    /// Reads (with owner and lag) and owned writes.
    fn access(&self) -> AccessSpec;
    /// The pulses and phases it runs at.
    fn schedule(&self) -> Vec<PulseBinding>;
    /// The RNG purposes it draws under: `<RngPurpose as SpecPurpose>::names()` (T-06).
    fn rng_purposes(&self) -> Vec<&'static str>;
    /// Its parameters.
    fn params_schema(&self) -> ParamSchema;
    /// One step at one binding of the current instant.
    ///
    /// # Errors
    /// Any [`SimError`]; the run stops.
    fn step(
        &self,
        binding: PulseBinding,
        world: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError>;
    /// Its invariants over the current tables.
    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult>;
    /// Its stylized-fact targets and conserved quantities.
    fn declared_aggregates(&self) -> Vec<AggregateSpec>;
}
