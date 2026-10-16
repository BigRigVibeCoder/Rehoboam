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
//! REF: `docs/05_architecture/05.1_simulation_core.md` §2, §4.1, §4.3; ADR-0025; ADR-0033 §3;
//! `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-03, T-06, rulings 1 and 2;
//! `CODEX/05_PROJECT/SPR-006_AgendaEventsInvariants.md` T-02–T-05, rulings 1–5.

use core::cell::RefCell;
use core::fmt;
use std::collections::BTreeSet;

use rh_events::{DraftRef, EventDraft};
use rh_ids::{EntityKey, EntityRef, PurposeId, RngStream, SystemId, SystemKey};

use crate::access::AccessSpec;
use crate::agenda::{Agenda, TimerEntry, TimerId};
use crate::command::{Command, QueuedCommand};
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
    /// A second keyed stream for one `(purpose, entity)` in one step, or a stream and an
    /// [`StepCtx::rng`] draw for the same pair (`05.1` §6 item 3: each sampler call takes its own
    /// stream; SPR-008). A system that needs two samples per entity declares two purposes.
    StreamReuse {
        /// The system.
        system: &'static str,
        /// The purpose name.
        purpose: &'static str,
        /// The entity key (`EntityKey::entity_key`).
        entity: u64,
        /// What the step had already taken for the pair: `"stream"` or `"rng"`.
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
    /// An event kind the system does not declare in `AccessSpec::emits` (SPR-006 T-04).
    UndeclaredEvent {
        /// The system.
        system: &'static str,
        /// The kind.
        kind: &'static str,
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
            Self::StreamReuse {
                system,
                purpose,
                entity,
                taken,
            } => write!(
                f,
                "{system} asked for a stream of purpose {purpose} for entity {entity}, which this step already used as a {taken} (05.1 §6 item 3: one stream per sampler call; declare another purpose)"
            ),
            Self::Timer { system, message } => write!(f, "{system}: timer refused: {message}"),
            Self::UndeclaredCommand { system, command } => write!(
                f,
                "{system} submitted command {command}, which its AccessSpec does not declare or no registered system applies"
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
            Self::Unsupported {
                system,
                column,
                lag,
            } => write!(
                f,
                "{system} reads {column} at lag {lag}, which the engine cannot yet run exactly (BCK-002 A-28)"
            ),
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
}

/// What one step buffered, merged by the engine at the end of the level (ruling 1).
#[derive(Debug, Default)]
pub(crate) struct Outbox {
    pub(crate) cancels: Vec<TimerId>,
    pub(crate) timers: Vec<(TimerSpec, PurposeId)>,
    pub(crate) commands: Vec<Command>,
    pub(crate) events: Vec<EventDraft>,
}

/// What the engine lends one job: the system's identity and declarations and the agenda as it
/// stood at the start of the level.
pub(crate) struct JobInfo<'a> {
    pub(crate) system: &'static str,
    pub(crate) system_id: SystemId,
    pub(crate) key: SystemKey,
    pub(crate) purposes: &'a [&'static str],
    pub(crate) access: &'a AccessSpec,
    pub(crate) bindings: &'a [PulseBinding],
    pub(crate) agenda: &'a Agenda,
}

/// The step context of one system at one instant and phase (`05.1` §2 rule 6): the keyed RNG,
/// the system's command inbox, and the buffers for timers, cancellations, commands and events.
pub struct StepCtx<'a> {
    info: JobInfo<'a>,
    t: SimTime,
    phase: Phase,
    inbox: Vec<QueuedCommand>,
    pub(crate) out: Outbox,
    /// The `(purpose, entity)` pairs this step has drawn from, and how (05.1 §6 item 3).
    taken: RefCell<BTreeSet<(u64, u64, Taken)>>,
}

/// How a step used a `(purpose, entity)` pair: through [`StepCtx::rng`] or [`StepCtx::stream`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Taken {
    Rng,
    Stream,
}

impl Taken {
    const fn name(self) -> &'static str {
        match self {
            Self::Rng => "rng",
            Self::Stream => "stream",
        }
    }
}

impl<'a> StepCtx<'a> {
    /// A context for one job: one system at instant `t` in `phase`, with its queued commands.
    pub(crate) fn new(
        info: JobInfo<'a>,
        t: SimTime,
        phase: Phase,
        inbox: Vec<QueuedCommand>,
    ) -> Self {
        Self {
            info,
            t,
            phase,
            inbox,
            out: Outbox::default(),
            taken: RefCell::new(BTreeSet::new()),
        }
    }

    /// The commands the system did not take, and the buffers.
    pub(crate) fn finish(self) -> (Vec<QueuedCommand>, Outbox) {
        (self.inbox, self.out)
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
    /// `epoch` is 0 here: a material input's re-sample draws its own epoch-keyed values
    /// ([`ResampleCtx`]).
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
        let entity = entity.entity_key();
        self.take(purpose.name(), id, entity, Taken::Rng)?;
        let stream = self.info.key.stream_key(id, entity);
        Ok(stream.draw(self.t.minutes(), k))
    }

    /// The keyed stream of `purpose` for `entity` at the current instant, positioned at draw 0:
    /// what an `rh-sample` sampler call takes (`05.1` §6 item 3). Its key is [`StepCtx::rng`]'s,
    /// so `stream(p, e)?.draw(k) == rng(p, e, k)?`.
    ///
    /// One sampler call, one stream: within one step a pair is handed out as a stream at most
    /// once, and never alongside [`StepCtx::rng`] draws for the same pair, because how many draws
    /// a sampler consumes depends on its parameters. A system that needs two samples per entity
    /// declares two purposes.
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
        let entity = entity.entity_key();
        self.take(purpose.name(), id, entity, Taken::Stream)?;
        let key = self.info.key.stream_key(id, entity);
        Ok(RngStream::from_stream_key(&key, self.t.minutes()))
    }

    /// Records that this step uses `(id, entity)` as `how`, refusing a second use that the
    /// call-site rule forbids: a stream after anything, or a draw after a stream.
    fn take(
        &self,
        purpose: &'static str,
        id: PurposeId,
        entity: u64,
        how: Taken,
    ) -> Result<(), SimError> {
        let mut taken = self.taken.borrow_mut();
        let pair = (id.to_raw(), entity);
        let earlier = [Taken::Stream, Taken::Rng]
            .into_iter()
            .find(|&t| taken.contains(&(pair.0, pair.1, t)));
        match (earlier, how) {
            (Some(Taken::Stream), _) | (Some(Taken::Rng), Taken::Stream) => {
                Err(SimError::StreamReuse {
                    system: self.info.system,
                    purpose,
                    entity,
                    taken: earlier.map_or("", Taken::name),
                })
            }
            _ => {
                taken.insert((pair.0, pair.1, how));
                Ok(())
            }
        }
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

    /// Takes the lifecycle commands queued for this system, in `(tick, phase, PersonId, seq)`
    /// order (`05.1` §2 rule 2). The system applies them; commands it does not take stay queued.
    pub fn take_commands(&mut self) -> Vec<QueuedCommand> {
        std::mem::take(&mut self.inbox)
    }

    /// Submits a lifecycle command to its owner's queue.
    ///
    /// # Errors
    /// [`SimError::UndeclaredCommand`] if the system does not declare it in `AccessSpec::sends`;
    /// [`SimError::Event`] if its cause is not an event this step already emitted.
    pub fn submit(&mut self, command: Command) -> Result<(), SimError> {
        if self.info.access.send_of(command.kind).is_none() {
            return Err(SimError::UndeclaredCommand {
                system: self.info.system,
                command: command.kind,
            });
        }
        if let Some(r) = command.cause
            && r.0 >= self.out.events.len()
        {
            return Err(self.event_error(format!(
                "command {} names draft {} as its cause, which this step has not emitted",
                command.kind, r.0
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
        let index = self.out.events.len();
        if let Some(r) = draft.local_causes.iter().find(|r| r.0 >= index) {
            return Err(self.event_error(format!(
                "event {} names draft {} as its cause, which is not an earlier draft of this step",
                draft.kind, r.0
            )));
        }
        self.out.events.push(draft);
        Ok(DraftRef(index))
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
