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
//! SPR-006 adds, without changing any SPR-005 signature:
//! - two provided methods, [`SystemModel::fire`] (a due timer, `05.1` §4.1) and
//!   [`SystemModel::resample_timer`] (a timer in a material input's scope, ADR-0025 item 3); both
//!   default to a loud error, so a system that schedules timers must implement them;
//! - the step context's output buffers ([`StepCtx::schedule`], [`StepCtx::cancel`],
//!   [`StepCtx::submit`], [`StepCtx::emit`]) and its command inbox ([`StepCtx::take_commands`]).
//!
//! SPR-008 adds [`StepCtx::stream`], the keyed stream an `rh-sample` sampler call takes (`05.1`
//! §6 item 3), with the same key as [`StepCtx::rng`] (Architect ruling, 2026-10-16).
//!   Nothing a step buffers is visible until the engine merges the buffers of a whole scheduler
//!   level in canonical order (SPR-006 ruling 1).
//!
//! SPR-011 adds [`StepCtx::chance`] (one keyed draw against a `Prob32`, T-01), keys every
//! `rng` and `stream` draw by the drawn entity's epoch (T-02, BCK-002 A-29), and keeps the stream
//! guard per (system, instant), across bindings and the timer pass (BCK-002 A-37).
//!
//! SPR-015 adds the derived per-system index (the Architect's W4 ruling): a system declares an index
//! built from its own columns at construction and at restore, kept by a serial post-merge `&mut`
//! hook, never hashed or keyframed; F9's issued-id table and the kinship, spouse and
//! children-by-parent lookups live in it, and M7 reuses it.
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §2, §4.1, §4.3; ADR-0025; ADR-0033 §3;
//! `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-03, T-06, rulings 1 and 2;
//! `CODEX/05_PROJECT/SPR-006_AgendaEventsInvariants.md` T-02–T-05, rulings 1–5.

// LINT-ALLOW(hidden-state): single-thread per-job step context
use core::cell::RefCell;
use core::fmt;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use rh_events::{DraftRef, EventDraft, EventRef};
use rh_ids::{
    BranchSalt, EntityKey, EntityKind, EntityRef, PurposeId, RngStream, StreamKey, SystemId,
    SystemKey,
};

use crate::access::AccessSpec;
use crate::agenda::{Agenda, TimerEntry, TimerId};
use crate::command::{Arg, Command, CommandDecl, QueuedCommand};
use crate::fixed::{Probability, draw_hits};
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
    /// A second keyed stream for one `(purpose, entity)` by one system at one instant, or a stream
    /// and an [`StepCtx::rng`] draw for the same pair (`05.1` §6 item 3: each sampler call takes
    /// its own stream; SPR-008), across the system's bindings and timer pass (BCK-002 A-37). A
    /// system that needs two samples per entity declares two purposes.
    StreamReuse {
        /// The system.
        system: &'static str,
        /// The purpose name.
        purpose: &'static str,
        /// The entity key (`EntityKey::entity_key`).
        entity: u64,
        /// What the system had already taken for the pair at this instant: `"stream"` or `"rng"`.
        taken: &'static str,
    },
    /// A timer the agenda refuses: in the past, in a phase the system is not bound in, or a
    /// cancellation of a timer that is not the system's pending timer (SPR-006 T-02).
    Timer {
        /// The system.
        system: &'static str,
        /// What was refused.
        message: String,
    },
    /// A command the system does not declare in `AccessSpec::sends`, or whose owner is not
    /// registered (SPR-006 T-03).
    UndeclaredCommand {
        /// The system.
        system: &'static str,
        /// The command.
        command: &'static str,
    },
    /// An event-typed payload slot given an integer, or an integer field given an event slot
    /// (`05.1` §2 rule 2, "Event-typed slots"; SPR-014 T-02).
    EventSlot {
        /// The submitting system.
        system: &'static str,
        /// The command.
        command: &'static str,
        /// The field (its name in the owner's `[[applies]] payload`).
        field: String,
    },
    /// A command whose payload does not fit the owner's `[[applies]] payload`: a wrong field
    /// count, an integer outside its field's type, or a key person that differs from the field
    /// `key_field` names (SPR-014 T-02).
    Payload {
        /// The submitting system.
        system: &'static str,
        /// The command.
        command: &'static str,
        /// What does not fit.
        message: String,
    },
    /// A draw whose entity is not of the kind its purpose's `[[rng]] key` declares (BCK-002
    /// A-50; SPR-014 T-07).
    EntityKind {
        /// The system.
        system: &'static str,
        /// The purpose.
        purpose: &'static str,
        /// The kind the purpose's key declares.
        declared: &'static str,
        /// The kind drawn with.
        drawn: &'static str,
    },
    /// An event kind the system does not declare in `AccessSpec::emits` (SPR-006 T-04).
    UndeclaredEvent {
        /// The system.
        system: &'static str,
        /// The kind.
        kind: &'static str,
    },
    /// A role row whose role the kind's `[[emits]] roles` does not list (envelope v0 §1).
    UndeclaredRole {
        /// The system.
        system: &'static str,
        /// The kind.
        kind: &'static str,
        /// The role code.
        role: u8,
    },
    /// A second row of a role other than `witness`, or a repeated `(role, entity)` (§1).
    DuplicateRole {
        /// The system.
        system: &'static str,
        /// The kind.
        kind: &'static str,
        /// The role code.
        role: u8,
        /// The entity of the second row.
        entity: EntityRef,
    },
    /// A cast of more than 256 rows: the ordinal is a `u8` (§1; R12 NIT-5).
    CastTooLarge {
        /// The system.
        system: &'static str,
        /// The kind.
        kind: &'static str,
        /// The number of rows.
        rows: usize,
    },
    /// A snapshot of an attribute the kind's `[[emits]] snapshots` does not declare (§4 rule 1).
    UndeclaredSnapshot {
        /// The system.
        system: &'static str,
        /// The kind.
        kind: &'static str,
        /// The attribute.
        attr: &'static str,
    },
    /// A snapshot whose entity (or partner) does not play the role its declaration names (§4
    /// rule 3).
    SnapshotNotOfTheCast {
        /// The system.
        system: &'static str,
        /// The kind.
        kind: &'static str,
        /// The attribute.
        attr: &'static str,
        /// The snapshot's entity.
        entity: EntityRef,
    },
    /// A snapshot value outside its attribute's domain (§4 rule 5).
    SnapshotOutOfDomain {
        /// The system.
        system: &'static str,
        /// The kind.
        kind: &'static str,
        /// The attribute.
        attr: &'static str,
        /// The value.
        value: i32,
    },
    /// Two snapshots with one key (§4 rule 6).
    DuplicateSnapshot {
        /// The system.
        system: &'static str,
        /// The kind.
        kind: &'static str,
        /// The attribute.
        attr: &'static str,
    },
    /// A declared snapshot whose roles are filled and whose source is present, absent from the
    /// draft (§4 rule 1; R10).
    MissingSnapshot {
        /// The system.
        system: &'static str,
        /// The kind.
        kind: &'static str,
        /// The attribute.
        attr: String,
    },
    /// A motive the kind's `[[emits]] motives` does not list (§2).
    UndeclaredMotive {
        /// The system.
        system: &'static str,
        /// The kind.
        kind: &'static str,
        /// The motive.
        motive: &'static str,
    },
    /// An event the log refuses at the merge: back-dated, or a cause that is not earlier
    /// (SPR-006 T-04, ruling 4).
    Event {
        /// The emitting system.
        system: &'static str,
        /// The refusal.
        message: String,
    },
    /// A violated invariant, found in `Bookkeeping` (SPR-006 T-05). Always fatal.
    Invariant {
        /// The system whose invariant it is (`rh-core` for the core's own).
        system: &'static str,
        /// The invariant's name.
        name: &'static str,
        /// What was found.
        detail: String,
    },
    /// A declared read whose lag the engine cannot yet honour exactly: `previous_pulse`, or
    /// `start_of_day` by a system bound to an Hour pulse (P0 reads both at the start of the
    /// instant; exact semantics are BCK-002 A-28). Registration accepts the system; the engine
    /// refuses to run it, so no system runs on the approximation unawares.
    Unsupported {
        /// The system.
        system: &'static str,
        /// The column read.
        column: &'static str,
        /// The lag's spec name.
        lag: &'static str,
    },
    /// A step panicked: caught at dispatch and turned into this fatal error (GOV-004 §2; SPR-006
    /// DEF-1), so the run stops with a crash artifact instead of unwinding.
    Panic {
        /// The system whose step panicked.
        system: &'static str,
        /// The panic message.
        message: String,
    },
    /// The host's observer panicked while receiving a notice (SPR-009 DEF-1). Caught like a step's
    /// panic: fatal, it poisons the engine and leaves a crash artifact naming the observer and the
    /// notice; it never unwinds to the host with columns out of the tables.
    Observer {
        /// The notice being delivered.
        notice: String,
        /// The panic message.
        message: String,
    },
    /// A ledger rule broken (SPR-011 ruling 5, `crate::ledger`): a step changed the row count of a
    /// `person.*` column (only the core's compaction removes rows), or wrote the life-course log
    /// other than by appending and closing an open entry once; or the world has ledger columns
    /// without the ones the core needs. Fatal.
    Ledger {
        /// The system whose step broke it (`rh-core` for a world the core cannot keep).
        system: &'static str,
        /// What was refused.
        message: String,
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
    #[allow(
        clippy::too_many_lines,
        reason = "one arm per error; the envelope's nine arms (SPR-020) passed the limit"
    )]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UndeclaredRole { system, kind, role } => write!(
                f,
                "{system} emits {kind} with role {role}, which its [[emits]] roles do not list (UndeclaredRole; envelope_v0.md §1)"
            ),
            Self::DuplicateRole {
                system,
                kind,
                role,
                entity,
            } => write!(
                f,
                "{system} emits {kind} with a second row of role {role} ({entity:?}); only witness repeats, and no (role, entity) twice (DuplicateRole)"
            ),
            Self::CastTooLarge { system, kind, rows } => write!(
                f,
                "{system} emits {kind} with a cast of {rows} rows; at most 256 (CastTooLarge)"
            ),
            Self::UndeclaredSnapshot { system, kind, attr } => write!(
                f,
                "{system} emits {kind} with a snapshot of {attr}, which its [[emits]] snapshots do not declare (UndeclaredSnapshot; envelope_v0.md §4)"
            ),
            Self::SnapshotNotOfTheCast {
                system,
                kind,
                attr,
                entity,
            } => write!(
                f,
                "{system} emits {kind} with a snapshot of {attr} of {entity:?}, which does not play the declared role (SnapshotNotOfTheCast)"
            ),
            Self::SnapshotOutOfDomain {
                system,
                kind,
                attr,
                value,
            } => write!(
                f,
                "{system} emits {kind} with {attr} = {value}, outside the attribute's domain (SnapshotOutOfDomain)"
            ),
            Self::DuplicateSnapshot { system, kind, attr } => write!(
                f,
                "{system} emits {kind} with two snapshots of {attr} with one key (DuplicateSnapshot)"
            ),
            Self::MissingSnapshot { system, kind, attr } => write!(
                f,
                "{system} emits {kind} without its declared snapshot {attr}, whose role is filled and whose source is present (MissingSnapshot; R10)"
            ),
            Self::UndeclaredMotive {
                system,
                kind,
                motive,
            } => write!(
                f,
                "{system} emits {kind} with the motive {motive}, which its [[emits]] motives do not list (UndeclaredMotive)"
            ),
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
            Self::StreamReuse {
                system,
                purpose,
                entity,
                taken,
            } => write!(
                f,
                "{system} drew again under purpose {purpose} for entity {entity}, which it already took as a {taken} at this instant (05.1 §6 item 3: one stream per sampler call, across bindings and the timer pass; declare another purpose)"
            ),
            Self::Timer { system, message } => write!(f, "{system}: timer refused: {message}"),
            Self::UndeclaredCommand { system, command } => write!(
                f,
                "{system} submitted command {command}, which its AccessSpec does not declare or no registered system applies"
            ),
            Self::EventSlot {
                system,
                command,
                field,
            } => write!(
                f,
                "{system} submitted command {command} with an integer in its event-typed field {field}, or an event slot in an integer field (05.1 §2 rule 2: an EventRef slot takes a DraftRef or an EventId, never an integer)"
            ),
            Self::Payload {
                system,
                command,
                message,
            } => write!(f, "{system} submitted command {command}: {message}"),
            Self::EntityKind {
                system,
                purpose,
                declared,
                drawn,
            } => write!(
                f,
                "{system} drew under purpose {purpose}, whose [[rng]] key is a {declared}, with a {drawn} (BCK-002 A-50)"
            ),
            Self::UndeclaredEvent { system, kind } => write!(
                f,
                "{system} emitted event {kind}, which its AccessSpec does not declare"
            ),
            Self::Invariant {
                system,
                name,
                detail,
            } => write!(f, "invariant {name} of {system} violated: {detail}"),
            Self::Panic { system, message } => write!(f, "{system} panicked: {message}"),
            Self::Observer { notice, message } => {
                write!(f, "the host's observer panicked at {notice}: {message}")
            }
            Self::Unsupported {
                system,
                column,
                lag,
            } => write!(
                f,
                "{system} reads {column} at lag {lag}, which the engine cannot yet run exactly (BCK-002 A-28)"
            ),
            Self::Ledger { system, message } => write!(f, "{system}: ledger: {message}"),
            Self::Event { system, message } | Self::System { system, message } => {
                write!(f, "{system}: {message}")
            }
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

/// Where a timer fires: the instant, the phase and the entity it is about. The owning system,
/// the purpose, the closure id and the argument complete it ([`StepCtx::schedule`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerSpec {
    /// The instant it fires at: strictly after the current `(instant, phase)`.
    pub time: SimTime,
    /// The phase it fires in: one the system is bound in.
    pub phase: Phase,
    /// The entity it is about.
    pub entity: EntityRef,
    /// The system's closure id.
    pub kind: u32,
    /// The system's argument.
    pub payload: i64,
    /// The event that caused it: a draft of this step (resolved at the merge), an id read from
    /// state, or none (`05.1` §2 rule 2; EVO-001 R9). It is an `EventRef`, so an integer cannot
    /// stand in for it:
    ///
    /// ```compile_fail
    /// # use rh_core::system::TimerSpec;
    /// # use rh_core::{Phase, time::SimTime};
    /// # use rh_ids::{EntityRef, PersonId};
    /// let _ = TimerSpec {
    ///     time: SimTime::from_minutes(1),
    ///     phase: Phase::Vital,
    ///     entity: EntityRef::person(PersonId::from_raw(1)),
    ///     kind: 0,
    ///     payload: 0,
    ///     cause: 5, // an integer in an event-typed slot does not compile
    /// };
    /// ```
    pub cause: EventRef,
}

/// Why a command or a timer was dropped (`05.1` §2 rule 2: one diagnostic,
/// `dropped{system, kind, reason}`; REV-EVO-001 R11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DropReason {
    /// `rh-core` at take: a `key = "living"` command whose key person is dead or absent.
    DeadKey,
    /// The owner's drain or a fire: a person it names is dead.
    Dead,
    /// The owner: an ended claim or vow.
    Ended,
    /// A send whose owner the world does not register (`StepCtx::submit` discards it).
    AbsentOwner,
}

impl DropReason {
    /// The reason's name in the diagnostic.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::DeadKey => "dead_key",
            Self::Dead => "dead",
            Self::Ended => "ended",
            Self::AbsentOwner => "absent_owner",
        }
    }
}

/// What was dropped: a command kind, or a timer's closure id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DropKind {
    /// A command, by its Registry name.
    Command(&'static str),
    /// A timer, by its owner's closure id.
    Timer(u32),
}

impl fmt::Display for DropKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Command(kind) => f.write_str(kind),
            Self::Timer(kind) => write!(f, "timer.{kind}"),
        }
    }
}

/// What one step buffered, merged by the engine at the end of the level (ruling 1).
#[derive(Debug, Default)]
pub(crate) struct Outbox {
    pub(crate) cancels: Vec<TimerId>,
    pub(crate) timers: Vec<(TimerSpec, PurposeId)>,
    pub(crate) commands: Vec<Command>,
    pub(crate) events: Vec<EventDraft>,
    /// Snapshots `emit` discarded for an absent source owner, as `(KIND.attr, owner)` (§4 rule 4).
    pub(crate) dropped: Vec<(String, String)>,
    /// The drops the step counted: its own (`drop_command`, `drop_timer`) and its discarded sends.
    pub(crate) drops: Vec<(DropKind, DropReason)>,
    /// Ledger rows the step hands `rh-core` to insert at the merge (`05.1` §2 rule 2).
    pub(crate) inserts: Vec<Vec<(&'static str, i64)>>,
}

/// What the engine lends one job: the system's identity and declarations and the agenda as it
/// stood at the start of the level.
pub(crate) struct JobInfo<'a> {
    pub(crate) system: &'static str,
    pub(crate) system_id: SystemId,
    /// `K1` at epoch 0, the common case (`05.1` §6 item 2).
    pub(crate) key: SystemKey,
    /// The history tuple's seed and salt, to derive `K1` at another epoch.
    pub(crate) world_seed: u64,
    pub(crate) branch_salt: BranchSalt,
    /// The entities' epochs as the instant's `Inputs` left them (ADR-0025; BCK-002 A-29).
    pub(crate) epochs: &'a BTreeMap<EntityRef, u32>,
    pub(crate) purposes: &'a [&'static str],
    pub(crate) access: &'a AccessSpec,
    pub(crate) bindings: &'a [PulseBinding],
    pub(crate) agenda: &'a Agenda,
    /// The system's compiled `[[emits]]` declarations (SPR-020 T-03).
    pub(crate) envelope: &'a crate::envelope::Envelope,
    /// Every registered command, with its owner and typing (`05.1` §2 rule 2).
    pub(crate) commands: &'a BTreeMap<&'static str, CommandDecl>,
    /// The entity kind each of the system's purposes keys its draws by, where its `[[rng]] key`
    /// names one (BCK-002 A-50).
    pub(crate) purpose_kinds: &'a BTreeMap<&'static str, EntityKind>,
}

/// The step context of one system at one instant and phase (`05.1` §2 rule 6): the keyed RNG,
/// the system's command inbox, and the buffers for timers, cancellations, commands and events.
pub struct StepCtx<'a> {
    info: JobInfo<'a>,
    t: SimTime,
    phase: Phase,
    inbox: Vec<QueuedCommand>,
    pub(crate) out: Outbox,
    /// The `(purpose, entity)` pairs the system has drawn from at this instant, and how (05.1 §6
    /// item 3). The engine keeps one set per (system, instant) and lends it to each of the
    /// system's jobs in turn, so the guard spans every binding and the timer pass (BCK-002 A-37).
    /// The `RefCell` makes `StepCtx` `!Sync`: a job runs on one thread and the context is never
    /// shared, so the set cannot be reached from two threads.
    // LINT-ALLOW(hidden-state): single-thread per-job step context
    taken: RefCell<Taken>,
    // LINT-ALLOW(hidden-state): single-thread per-job step context
    epoch_keys: RefCell<BTreeMap<u32, SystemKey>>,
}

/// The pairs a system has drawn from at one instant, `(purpose id, entity key)`, each with how it
/// was taken: repeated `rng` draws share one entry, and any other second take (a stream after an
/// `rng`, or anything after a stream) is refused; see [`StepCtx`]. The engine owns one per system
/// and clears them at each instant.
pub(crate) type Taken = BTreeMap<(u64, u64), TakenAs>;

/// How a system used a `(purpose, entity)` pair: through [`StepCtx::rng`] or [`StepCtx::stream`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TakenAs {
    Rng,
    Stream,
}

impl TakenAs {
    const fn name(self) -> &'static str {
        match self {
            Self::Rng => "rng",
            Self::Stream => "stream",
        }
    }
}

impl<'a> StepCtx<'a> {
    /// A context for one job: one system at instant `t` in `phase`, with its queued commands and
    /// the pairs it has already drawn from at this instant.
    pub(crate) fn new(
        info: JobInfo<'a>,
        t: SimTime,
        phase: Phase,
        inbox: Vec<QueuedCommand>,
        taken: Taken,
    ) -> Self {
        Self {
            info,
            t,
            phase,
            inbox,
            out: Outbox::default(),
            // LINT-ALLOW(hidden-state): single-thread per-job step context
            taken: RefCell::new(taken),
            // LINT-ALLOW(hidden-state): single-thread per-job step context
            epoch_keys: RefCell::new(BTreeMap::new()),
        }
    }

    /// The commands the system did not take, the buffers, and the pairs drawn from so far.
    pub(crate) fn finish(self) -> (Vec<QueuedCommand>, Outbox, Taken) {
        (self.inbox, self.out, self.taken.into_inner())
    }

    /// The epoch keying a draw for `entity`: its count of material inputs (ADR-0025), 0 for an
    /// entity no input has touched or an identifier no scope can name.
    fn epoch_of(&self, entity: impl EntityKey) -> u32 {
        entity
            .entity()
            .and_then(|e| self.info.epochs.get(&e).copied())
            .unwrap_or(0)
    }

    /// `K2` of `(purpose, entity)` at the entity's current epoch. At epoch 0 it is the P0 key
    /// exactly (BCK-002 A-29: an untouched entity draws as before).
    fn stream_key(&self, purpose: PurposeId, entity: impl EntityKey) -> StreamKey {
        let epoch = self.epoch_of(entity);
        let raw = entity.entity_key();
        if epoch == 0 {
            return self.info.key.stream_key(purpose, raw);
        }
        let mut keys = self.epoch_keys.borrow_mut();
        let key = keys.entry(epoch).or_insert_with(|| {
            SystemKey::new(
                self.info.world_seed,
                self.info.branch_salt,
                self.info.system_id,
                epoch,
            )
        });
        key.stream_key(purpose, raw)
    }

    /// The current instant.
    #[must_use]
    pub fn now(&self) -> SimTime {
        self.t
    }

    /// The phase running.
    #[must_use]
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Draw `k` of `purpose` for `entity` at the current instant:
    /// `H(world_seed, branch_salt, system_id, epoch, purpose, entity, t, k)`.
    ///
    /// `epoch` is the drawn entity's: the number of material inputs whose scope has held it, as
    /// of this instant's `Inputs` (ADR-0025; `05.1` §6 item 2; BCK-002 A-29). So after an input,
    /// only the entities in its scope draw differently, from the input's instant on; an entity at
    /// epoch 0 draws exactly as in P0.
    ///
    /// # Errors
    /// [`SimError::UndeclaredPurpose`] if `purpose` is not one the system registered.
    ///
    /// Any number of draws `k` may be taken for one pair, but not from a pair this step has
    /// handed out as a [`StepCtx::stream`].
    ///
    /// # Errors
    /// [`SimError::StreamReuse`] if this step took the pair as a stream.
    pub fn rng<P: SpecPurpose>(
        &self,
        purpose: &P,
        entity: impl EntityKey,
        k: u64,
    ) -> Result<u64, SimError> {
        let id = self.purpose_id(purpose)?;
        self.check_kind(purpose.name(), entity)?;
        self.take(purpose.name(), id, entity.entity_key(), TakenAs::Rng)?;
        Ok(self.stream_key(id, entity).draw(self.t.minutes(), k))
    }

    /// The keyed stream of `purpose` for `entity` at the current instant, positioned at draw 0:
    /// what an `rh-sample` sampler call takes (`05.1` §6 item 3). Its key is [`StepCtx::rng`]'s,
    /// so `stream(p, e)?.draw(k) == rng(p, e, k)?`.
    ///
    /// One sampler call, one stream: within one instant a system hands a pair out as a stream at
    /// most once, and never alongside [`StepCtx::rng`] draws for the same pair, across all its
    /// bindings and its timer pass (BCK-002 A-37), because how many draws a sampler consumes
    /// depends on its parameters. A system that needs two samples per entity declares two
    /// purposes. Like `rng`, the stream is keyed by the entity's epoch.
    ///
    /// # Errors
    /// [`SimError::UndeclaredPurpose`] if `purpose` is not one the system registered;
    /// [`SimError::StreamReuse`] if this step already took the pair, as a stream or by `rng`.
    pub fn stream<P: SpecPurpose>(
        &self,
        purpose: &P,
        entity: impl EntityKey,
    ) -> Result<RngStream, SimError> {
        let id = self.purpose_id(purpose)?;
        self.check_kind(purpose.name(), entity)?;
        self.take(purpose.name(), id, entity.entity_key(), TakenAs::Stream)?;
        let key = self.stream_key(id, entity);
        Ok(RngStream::from_stream_key(&key, self.t.minutes()))
    }

    /// One keyed Bernoulli trial: draw `k` of `purpose` for `entity` ([`StepCtx::rng`]) against
    /// `p`, which hits iff `draw / 2^64 < p` (`rh_core::fixed::draw_hits`; a [`crate::fixed::Prob32`]
    /// or a `Q16_16`). Exactly one draw is taken whatever `p` is, 0 and 1 included, so the stream is
    /// read the same way on every branch (common random numbers, `05.1` §6 item 9; SPR-011 T-01).
    ///
    /// # Errors
    /// As [`StepCtx::rng`].
    pub fn chance<P: SpecPurpose>(
        &self,
        purpose: &P,
        entity: impl EntityKey,
        k: u64,
        p: impl Probability,
    ) -> Result<bool, SimError> {
        Ok(draw_hits(self.rng(purpose, entity, k)?, p))
    }

    /// Records that this step uses `(id, entity)` as `how`, refusing a second use that the
    /// call-site rule forbids: a stream after anything, or a draw after a stream.
    fn take(
        &self,
        purpose: &'static str,
        id: PurposeId,
        entity: u64,
        how: TakenAs,
    ) -> Result<(), SimError> {
        // One B-tree lookup per draw (SPR-011 T-02 measured the guard as most of a draw's cost
        // in the step context; crates/rh-bench/benches/draws.rs).
        let mut taken = self.taken.borrow_mut();
        match taken.entry((id.to_raw(), entity)) {
            Entry::Vacant(slot) => {
                slot.insert(how);
                Ok(())
            }
            Entry::Occupied(slot) => match (*slot.get(), how) {
                (TakenAs::Rng, TakenAs::Rng) => Ok(()),
                (earlier, _) => Err(SimError::StreamReuse {
                    system: self.info.system,
                    purpose,
                    entity,
                    taken: earlier.name(),
                }),
            },
        }
    }

    /// BCK-002 A-50: the drawn entity is of the kind the purpose's `[[rng]] key` declares, when
    /// the key names a kind (registration reports the purposes whose key has none yet).
    fn check_kind(&self, purpose: &'static str, entity: impl EntityKey) -> Result<(), SimError> {
        let Some(&declared) = self.info.purpose_kinds.get(purpose) else {
            return Ok(());
        };
        let drawn = entity.kind();
        if drawn == Some(declared) {
            return Ok(());
        }
        Err(SimError::EntityKind {
            system: self.info.system,
            purpose,
            declared: declared.name(),
            drawn: drawn.map_or("event", EntityKind::name),
        })
    }

    fn purpose_id<P: SpecPurpose>(&self, purpose: &P) -> Result<PurposeId, SimError> {
        let name = purpose.name();
        if self.info.purposes.contains(&name) {
            Ok(PurposeId::of(name))
        } else {
            Err(SimError::UndeclaredPurpose {
                system: self.info.system,
                purpose: name,
            })
        }
    }

    fn timer_error(&self, message: String) -> SimError {
        SimError::Timer {
            system: self.info.system,
            message,
        }
    }

    /// Schedules an outcome as a timer (ADR-0025 item 1), drawn under `purpose`. The timer gets
    /// its id and `seq` at the end of the level.
    ///
    /// # Errors
    /// [`SimError::UndeclaredPurpose`]; [`SimError::Timer`] if `(spec.time, spec.phase)` is not
    /// strictly after the current `(instant, phase)` (no back-dating) or the system is not bound
    /// in `spec.phase` (it would never fire).
    pub fn schedule<P: SpecPurpose>(
        &mut self,
        purpose: &P,
        spec: TimerSpec,
    ) -> Result<(), SimError> {
        let id = self.purpose_id(purpose)?;
        if (spec.time, spec.phase) <= (self.t, self.phase) {
            return Err(self.timer_error(format!(
                "a timer at ({}, {}) is not after the current ({}, {}): no back-dating (ADR-0025)",
                spec.time, spec.phase, self.t, self.phase
            )));
        }
        if !self.info.bindings.iter().any(|b| b.phase == spec.phase) {
            return Err(self.timer_error(format!(
                "a timer in phase {}, where the system is not bound, would never fire",
                spec.phase
            )));
        }
        if spec
            .cause
            .draft()
            .is_some_and(|d| d.index() >= self.out.events.len())
        {
            return Err(self.event_error(
                "a timer names a draft as its cause which this step has not emitted".to_owned(),
            ));
        }
        self.out.timers.push((spec, id));
        Ok(())
    }

    /// Cancels one of the system's pending timers. It is gone at the end of the level and never
    /// fires.
    ///
    /// # Errors
    /// [`SimError::Timer`] if `id` is not pending, is another system's timer, or is already
    /// cancelled in this step.
    pub fn cancel(&mut self, id: TimerId) -> Result<(), SimError> {
        match self.info.agenda.get(id) {
            Some((_, timer)) if timer.owner == self.info.system_id => {}
            Some(_) => {
                return Err(self.timer_error(format!("{id} belongs to another system")));
            }
            None => return Err(self.timer_error(format!("{id} is not pending"))),
        }
        if self.out.cancels.contains(&id) {
            return Err(self.timer_error(format!("{id} is cancelled twice")));
        }
        self.out.cancels.push(id);
        Ok(())
    }

    /// The system's own pending timers about `entity`, in key order, as the agenda stood at the
    /// start of the level (timers due now have already been handed to `fire`).
    #[must_use]
    pub fn own_timers(&self, entity: EntityRef) -> Vec<TimerEntry> {
        self.info
            .agenda
            .pending_for(entity)
            .into_iter()
            .filter(|(_, t)| t.owner == self.info.system_id)
            .map(|(key, timer)| TimerEntry { key, timer })
            .collect()
    }

    /// Takes the lifecycle commands queued for this binding, in `(tick, phase, PersonId, seq)`
    /// order (`05.1` §2 rule 2): the commands of every kind the system's spec says this binding
    /// drains (`drained_by`), and nothing in any other job (another binding, the timer pass). The
    /// binding settles each one in this step: applies it, or drops it with
    /// [`StepCtx::drop_command`]. A `key = "living"` command whose key person was dead when the
    /// level started never reaches it: `rh-core` dropped it at the take (BCK-002 A-30). Commands a
    /// binding does not take stay queued, in order.
    pub fn take_commands(&mut self) -> Vec<QueuedCommand> {
        std::mem::take(&mut self.inbox)
    }

    /// Drops a command this binding took, counted under `dropped{system, kind, reason}`: a key
    /// person it killed earlier in the same drain, a dead party in the payload, an ended claim
    /// (`05.1` §2 rule 2). A drop is never an error, and it changes no state.
    pub fn drop_command(&mut self, command: &QueuedCommand, reason: DropReason) {
        self.out
            .drops
            .push((DropKind::Command(command.command.kind), reason));
    }

    /// Drops a due timer instead of acting on it (a death timer for a person already dead),
    /// counted under `dropped{system, kind, reason}` (`05.1` §2 rule 2; REV-EVO-001 NIT-15).
    pub fn drop_timer(&mut self, timer: &TimerEntry, reason: DropReason) {
        self.out
            .drops
            .push((DropKind::Timer(timer.timer.kind), reason));
    }

    /// Hands `rh-core` one complete row of this system's own `person.*` columns, inserted at the
    /// merge of this step's level at its sorted position in every `person.*` column (`05.1` §2
    /// rule 2, "Ledger rows enter by insert"; SPR-014 T-05). The row names each of the system's
    /// `person.*` columns once, `person.id` among them; another owner's column gets its declared
    /// default, else 0. A duplicate id fails the merge with [`SimError::Ledger`].
    pub fn insert_person(&mut self, row: Vec<(&'static str, i64)>) {
        self.out.inserts.push(row);
    }

    /// Submits a lifecycle command to its owner's queue.
    ///
    /// A command whose owner the world does not register is accepted and discarded, never
    /// queued, and counted under `dropped{sender, command, absent_owner}`: the sender never asks
    /// whether the owner exists (`05.1` §2 rule 2, absent owners).
    ///
    /// # Errors
    /// [`SimError::UndeclaredCommand`] if the system does not declare it in `AccessSpec::sends`;
    /// [`SimError::Event`] if its cause or an event-typed field names a draft this step has not
    /// emitted; [`SimError::EventSlot`] for an integer in an `EventRef` field (or an event slot in
    /// an integer field); [`SimError::Payload`] for a wrong field count, a value outside its
    /// field's type, or a key person that differs from the owner's `key_field`.
    pub fn submit(&mut self, command: Command) -> Result<(), SimError> {
        if self.info.access.send_of(command.kind).is_none() {
            return Err(SimError::UndeclaredCommand {
                system: self.info.system,
                command: command.kind,
            });
        }
        let emitted = self.out.events.len();
        let unknown_draft = |r: EventRef| r.draft().is_some_and(|d| d.index() >= emitted);
        if unknown_draft(command.cause) {
            return Err(self.event_error(format!(
                "command {} names a draft as its cause which this step has not emitted",
                command.kind
            )));
        }
        let Some(decl) = self.info.commands.get(command.kind) else {
            self.out
                .drops
                .push((DropKind::Command(command.kind), DropReason::AbsentOwner));
            return Ok(());
        };
        let payload_error = |message: String| SimError::Payload {
            system: self.info.system,
            command: command.kind,
            message,
        };
        if let Some(fields) = &decl.payload {
            if fields.len() != command.args.len() {
                return Err(payload_error(format!(
                    "{} fields, but {}'s [[applies]] payload has {}",
                    command.args.len(),
                    decl.owner,
                    fields.len()
                )));
            }
            for ((name, ty), arg) in fields.iter().zip(&command.args) {
                match (*ty == crate::command::ArgType::Event, arg) {
                    (true, Arg::Event(r)) if unknown_draft(*r) => {
                        return Err(self.event_error(format!(
                            "command {} names a draft in field {name} which this step has not emitted",
                            command.kind
                        )));
                    }
                    (true, Arg::Event(_)) => {}
                    (false, Arg::Int(v)) if ty.admits(*v) => {}
                    (false, Arg::Int(v)) => {
                        return Err(payload_error(format!(
                            "field {name} = {v} is outside its type {ty:?}"
                        )));
                    }
                    (true, Arg::Int(_)) | (false, Arg::Event(_)) => {
                        return Err(SimError::EventSlot {
                            system: self.info.system,
                            command: command.kind,
                            field: name.clone(),
                        });
                    }
                }
            }
            if let Some(i) = decl.key_field {
                let key = command.person.to_raw().cast_signed();
                if command.int(i) != Some(key) {
                    return Err(payload_error(format!(
                        "its key person {} differs from its key field {} (05.1 §2 rule 2)",
                        command.person.to_raw(),
                        fields[i].0
                    )));
                }
            }
        } else if let Some(i) = command.args.iter().position(|a| matches!(a, Arg::Event(_))) {
            // An untyped command's row stores integers only: an event slot would hash as an id
            // and not decode back from a keyframe (the review's K1).
            return Err(payload_error(format!(
                "field {i} is an event slot, but {}'s [[applies]] declares no payload: an untyped \
                 command carries integers only",
                decl.owner
            )));
        }
        self.out.commands.push(command);
        Ok(())
    }

    fn event_error(&self, message: String) -> SimError {
        SimError::Event {
            system: self.info.system,
            message,
        }
    }

    /// Emits an event at the current instant and phase, and returns its reference within this
    /// step. It gets its `EventId` at the end of the level.
    ///
    /// # Errors
    /// [`SimError::UndeclaredEvent`] if the system does not declare the kind in
    /// `AccessSpec::emits`; [`SimError::Event`] if a draft-local cause is not an earlier draft.
    pub fn emit(&mut self, draft: EventDraft) -> Result<DraftRef, SimError> {
        if !self.info.access.emits_kind(draft.kind) {
            return Err(SimError::UndeclaredEvent {
                system: self.info.system,
                kind: draft.kind,
            });
        }
        let mut draft = draft;
        let none = crate::envelope::KindEnvelope::default();
        let kind = self.info.envelope.get(draft.kind).unwrap_or(&none);
        let dropped = crate::envelope::check(self.info.system, kind, &mut draft)?;
        let index = self.out.events.len();
        for edge in &draft.causes {
            match edge.cause {
                EventRef::Draft(r) if r.index() >= index => {
                    return Err(self.event_error(format!(
                        "event {} names draft {} as its cause, which is not an earlier draft of this step",
                        draft.kind, r.index()
                    )));
                }
                EventRef::None => {
                    return Err(self.event_error(format!(
                        "event {} has a {} edge to no event; EventRef::None is not an edge",
                        draft.kind, edge.kind
                    )));
                }
                EventRef::Draft(_) | EventRef::Known(_) => {}
            }
        }
        self.out.events.push(draft);
        self.out.dropped.extend(dropped);
        Ok(DraftRef::from_index(index))
    }
}

/// Domain tag of [`resample_entity_id`].
pub const TAG_RESAMPLE_ENTITY: u64 = rh_ids::rng::tag("rehoboam/rh-core/resample-entity/v1");

/// The `entity_id` of a re-sample draw: `h64_words(tag("rehoboam/rh-core/resample-entity/v1"),
/// [kind tag, raw id])`, a function of the whole typed key (SPR-006 G3), so `Person:5` and
/// `Household:5` never share a stream. Part of the determinism contract: changing it is C3.
#[must_use]
pub fn resample_entity_id(entity: EntityRef) -> u64 {
    rh_ids::rng::h64_words(
        TAG_RESAMPLE_ENTITY,
        &[u64::from(entity.kind().tag()), entity.raw()],
    )
}

/// The context of one re-sample (ADR-0025 item 3): draws keyed by
/// `H(world_seed, branch_salt, system_id, epoch, purpose, resample_entity_id(entity), t_input, k)`,
/// where `epoch` counts the material inputs that have touched the entity, `purpose` is the timer's
/// own and `t_input` the input's instant.
pub struct ResampleCtx {
    pub(crate) key: SystemKey,
    pub(crate) purpose: PurposeId,
    pub(crate) entity: EntityRef,
    pub(crate) t_input: SimTime,
    pub(crate) epoch: u32,
}

impl ResampleCtx {
    /// The input's instant: a re-drawn time must be after it.
    #[must_use]
    pub fn now(&self) -> SimTime {
        self.t_input
    }

    /// The entity's epoch after this input.
    #[must_use]
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    /// Draw `k` of the re-sample stream. Its `entity_id` is [`resample_entity_id`] of the
    /// timer's entity, so entities of different kinds with the same raw id draw apart.
    #[must_use]
    pub fn draw(&self, k: u64) -> u64 {
        self.key
            .stream_key(self.purpose, resample_entity_id(self.entity))
            .draw(self.t_input.minutes(), k)
    }
}

/// The contract every simulation system implements (`05.1` §2).
pub trait SystemModel: Send + Sync {
    /// The system's per-column index, built from its own columns. A system that declares one
    /// implements [`SystemModel::build_index`] (W4), which the engine calls at construction and
    /// at restore to build it from the current tables, and [`SystemModel::post_merge`] (W4),
    /// which it calls after merging each step to update it from the step's writes only: the
    /// post-merge hook sees the index as it stood before the step, and exactly the columns the
    /// step wrote. The index is never hashed or keyframed, and does not outlive the engine
    /// (W4): it holds no state, only derived data the step computes from the tables as they
    /// stand, so restoring a snapshot restores the tables and builds a fresh index from them.
    ///
    /// The index type itself is opaque to `rh-core`: the trait fixes it as an associated type
    /// only so the compiler can track mutations through borrows. A system that uses one gives
    /// the type it wants to hold, and its `build_index` and `post_merge` signatures say how to
    /// construct and update exactly that type. An index that holds interior mutability (`RefCell`,
    /// `Cell`, `Mutex`) must be behind a `Box`: the trait bounds below (`Clone` especially)
    /// require a type that moves without disturbing its contents, and a `Box` moves as a pointer.
    ///
    /// SPR-015 adds this hook (the Architect's W4 ruling): F9's issued-id table and the kinship,
    /// spouse and children-by-parent lookups live in it, and M7 reuses it.
    type Index: Clone + fmt::Debug;

    /// Builds the system's index from the current tables (`WorldTables`), for construction or
    /// restore (W4). A system that declares one implements this method, and `post_merge` too.
    ///
    /// SPR-015 adds this hook (the Architect's W4 ruling).
    ///
    /// The default implementation panics. Systems that want to use indexing must override this.
    #[allow(
        clippy::panic,
        reason = "INVARIANT: SystemModel::build_index default stub panics if indexing is used without an implementation"
    )]
    fn build_index(&self, _tables: &WorldTables) -> Self::Index {
        panic!(
            "System {} does not implement build_index but tries to use indexing",
            std::any::type_name::<Self>()
        )
    }

    /// Updates the system's index from exactly the columns the step wrote (W4). The engine calls
    /// this after merging each step: the hook sees the index as it stood before the step, and
    /// exactly the columns the step wrote (the same ones `SystemView::into_owned` returns).
    /// A step that writes no column of the system calls this with an empty `writes` map.
    ///
    /// The hook's borrow of the index is `&mut`: nothing else can see the index while this
    /// method runs, so it can mutate freely without interior mutability (`RefCell` etc.). The
    /// index type itself may hold such types, but if it does it must be behind a `Box` (see
    /// `Index`).
    ///
    /// SPR-015 adds this hook (the Architect's W4 ruling).
    ///
    /// The default implementation panics. Systems that want to use indexing must override this.
    #[allow(
        clippy::panic,
        reason = "INVARIANT: SystemModel::post_merge default stub panics if indexing is used without an implementation"
    )]
    fn post_merge(&self, _index: &mut Self::Index, _writes: BTreeMap<&str, &[i64]>) {
        panic!(
            "System {} does not implement post_merge but tries to use indexing",
            std::any::type_name::<Self>()
        )
    }

    /// The spec header.
    fn descriptor(&self) -> SystemDescriptor;
    /// Reads (with owner and lag) and owned writes.
    fn access(&self) -> AccessSpec;
    /// The pulses and phases it runs at.
    fn schedule(&self) -> Vec<PulseBinding>;
    /// The RNG purposes it draws under: `<RngPurpose as SpecPurpose>::names()` (T-06).
    fn rng_purposes(&self) -> Vec<&'static str>;
    /// The role constants of its skeleton (`ROLES`), which registration compares with the
    /// Registry's codes (SPR-020 T-01). The default declares none.
    fn roles(&self) -> &'static [(&'static str, rh_events::RoleId)] {
        &[]
    }
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

    /// Handles one of its timers, due at the current instant and phase (`05.1` §4.1: a timer
    /// fires at its own tick in every mode). The engine hands a system its due timers in key
    /// order, before its pulse steps in that phase.
    ///
    /// The default refuses: a system that schedules timers implements this (SPR-006 T-02).
    ///
    /// # Errors
    /// Any [`SimError`]; the run stops.
    fn fire(
        &self,
        timer: &TimerEntry,
        world: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let _ = (world, ctx);
        Err(SimError::System {
            system: self.descriptor().id,
            message: format!(
                "{} fired, but the system does not implement SystemModel::fire",
                timer.id()
            ),
        })
    }

    /// Re-draws the time of one of its timers that a material input's scope cancelled (ADR-0025
    /// item 3), with the input's epoch-keyed draws. `Some(t)` re-schedules the timer at `t` (in the
    /// same phase, about the same entity); `None` drops it: the outcome no longer happens.
    ///
    /// The default refuses: a system that schedules timers implements this (SPR-006 T-02).
    ///
    /// # Errors
    /// Any [`SimError`]; the run stops.
    fn resample_timer(
        &self,
        timer: &TimerEntry,
        ctx: &ResampleCtx,
    ) -> Result<Option<SimTime>, SimError> {
        let _ = ctx;
        Err(SimError::System {
            system: self.descriptor().id,
            message: format!(
                "{} is in a material input's scope, but the system does not implement SystemModel::resample_timer",
                timer.id()
            ),
        })
    }
}
