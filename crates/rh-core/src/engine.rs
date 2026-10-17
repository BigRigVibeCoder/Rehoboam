//! The instant loop: pulses, phases and the derived schedule, on one or several threads
//! (SPR-005 T-05; V-DET-01, V-DET-02), with the agenda, the lifecycle queues, the event bus and
//! the invariants (SPR-006).
//!
//! **Instants.** The engine runs every instant where a bound pulse fires, a timer is due
//! (`05.1` §4.1: timers fire at their own tick in every mode) or a material input is queued.
//!
//! **One instant.** The engine:
//! 1. records the start-of-instant value of every column some system reads with a lag;
//! 2. runs the phases in `05.1` §4.2 order. In `Inputs` it first applies the material inputs of
//!    the instant (re-sampling the timers in their scopes, below). In every phase it pops the
//!    timers due at `(instant, phase)`, in key order;
//! 3. within a phase, runs two passes over the derived schedule (`05.1` §2 rule 4: dependency levels
//!    over every binding, ties coarser pulse first, then id), each level by level: first the
//!    **timer pass**, the systems with timers due, each handling its timers in key order
//!    (`SystemModel::fire`) at the level of its first binding; then the firing bindings, a level
//!    split wherever one system's second binding appears (SPR-005 DEF-7). With one thread a level
//!    runs in order; with several its jobs are spread over the threads, each system's owned columns
//!    moved to its thread and back. Every job of a level runs before an error is reported;
//! 4. at the end of each level of each pass, **merges** what the level's steps buffered, in the
//!    canonical order (the level's systems in id order; SPR-006 ruling 1). For each system, in
//!    order: its cancellations, its new timers (which get their `seq` here), its events (which
//!    get their `EventId` here; draft-local causes become those ids) and its lifecycle commands
//!    (queued for their owners, with the id of their cause). Only
//!    the merge touches the agenda, the queues and the log, so the thread count never reaches an
//!    id;
//! 5. in `Bookkeeping`, first compacts the person ledger (the dead of the instant leave every
//!    `person.*` column; SPR-011 ruling 5, `crate::ledger`), then runs the phase's systems, then the
//!    invariants: the core's own (the ledger's among them), then each system's in id order. The
//!    first violation is fatal. After every step that owns a ledger column, the core checks it
//!    kept the ledger's row count and the life-course log's append-only rule.
//!
//! **Material inputs** (ADR-0025 item 3). [`Engine::submit_input`] queues an input with its
//! dependency scope (a set of entities) for an instant at or after the next one; an earlier one is
//! refused ([`InputError::PastInput`]: acting in the past forks, which is the Timeline Manager's
//! job). In `Inputs`, for each input in submission order, the engine increments the epoch of every
//! entity in the scope, cancels every pending timer about an entity in the scope (key order) and
//! asks its owner to re-draw it ([`crate::system::SystemModel::resample_timer`]) with draws keyed
//! by the entity's new epoch and the input's instant. P0 has no input journal (SPR-007), so an
//! input carries only its scope.
//!
//! **Crash artifacts** (GOV-004 §4; ruling 6). When an instant fails (a step error, a violated
//! invariant, or a step that panics, which dispatch catches as [`SimError::Panic`]), the engine keeps a
//! [`CrashArtifact`] ([`Engine::crash_artifact`]); the host writes it. [`Engine::replay`] runs a
//! fresh engine built from the artifact's tuple to the failing instant and checks that the same
//! failure recurs.
//!
//! The thread count changes scheduling only, never the result: `tests/determinism.rs` runs the
//! stub world at 1, 2, 4 and 8 threads and compares monthly state hashes (V-DET-01), and repeats a
//! run three times (V-DET-02); `tests/agenda_events.rs` does the same for timers, commands and
//! `EventId`s. The same systems run at an erroring instant at every thread count, and the first
//! error in canonical order is the one reported.
//!
//! **Observer** (SPR-009 ruling 4). A host may attach an [`Observer`] ([`Engine::set_observer`]),
//! which receives a [`Notice`] at every instant, phase, level and job boundary: sim time, phase and
//! binding, never a clock and never state. It is how the host times systems without plane A ever
//! reading a clock, and attaching it changes no result ([`crate::observe`]).
//!
//! No wall clock, no unkeyed RNG, no hash-map iteration (GOV-003 §2). Threads come from
//! `std::thread::scope`; no third-party dependency.
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §2 rules 2 and 4, §4.1–§4.3, §6 items 2
//! and 5, §9; ADR-0025; GOV-004 §4; `docs/09` V-DET-01, V-DET-02, V-DET-12;
//! `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-05;
//! `CODEX/05_PROJECT/SPR-006_AgendaEventsInvariants.md` T-01–T-05, rulings 1–6.

use core::fmt;
use std::any::Any;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use rh_events::{Event, EventLog};
use rh_ids::{BranchSalt, EntityRef, SystemId, SystemKey, TimelineId};

use crate::access::Lag;
use crate::agenda::{Agenda, Timer, TimerEntry};
use crate::command::{CommandQueues, QueuedCommand};
use crate::crash::{CrashArtifact, DIAGNOSTIC_RECORDS, Provenance};
use crate::ledger::{Ledger, LifeCourseGuard};
use crate::observe::{Boundary, JobRef, JobWork, Notice, Observer, Pass};
use crate::phase::Phase;
use crate::register::{Entry, Registered};
use crate::schedule::Node;
use crate::state::{EngineState, RestoreError, check_epochs, check_inputs, check_timers};
use crate::system::{
    InvariantResult, JobInfo, Outbox, PulseBinding, ResampleCtx, SimError, StepCtx, Taken,
};
use crate::time::{Calendar, Pulse, SimTime};
use crate::world::{Column, SystemView, WorldTables};

/// The name the core's own invariants and errors carry.
pub const CORE: &str = "rh-core";

/// The name a crash artifact gives the host's observer when it panicked (SPR-009 DEF-1). A replay
/// runs without the host's observer, so such an artifact replays as no failure.
pub const OBSERVER: &str = "observer";

/// A material input as P0 knows it: its dependency scope (ADR-0025 item 3). The host expands a
/// household, settlement or region scope into the entities it covers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialInput {
    /// The entities whose pending timers are cancelled and re-sampled.
    pub scope: BTreeSet<EntityRef>,
}

/// A refused input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputError {
    /// The input's instant is before the next instant to run: acting in the past forks
    /// (ADR-0025 item 5; V-DET-13).
    PastInput {
        /// The input's instant.
        at: SimTime,
        /// The next instant to run, if any.
        next: Option<SimTime>,
    },
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PastInput { at, next } => match next {
                Some(n) => write!(
                    f,
                    "a material input at {at} is before the next instant {n}: acting in the past forks (ADR-0025)"
                ),
                None => write!(f, "a material input at {at} arrives after the run ended"),
            },
        }
    }
}

impl std::error::Error for InputError {}

/// Why a replay did not reproduce a crash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayError {
    /// The engine was not built from the artifact's tuple: the named field differs.
    Tuple(&'static str),
    /// The run reached the instant without failing.
    NoFailure,
    /// The run had accepted material inputs (the artifact's journal position), and the replaying
    /// engine has not been given the same number: the host submits the journal's first
    /// `position` inputs before replaying (SPR-006 DEF-2; VER-SPR-007-TEST GAP-10).
    JournalRequired {
        /// The artifact's journal position: the number of material inputs accepted.
        position: u64,
        /// The number the replaying engine has accepted.
        submitted: u64,
    },
    /// The run failed differently.
    Mismatch {
        /// The artifact replayed.
        expected: Box<CrashArtifact>,
        /// What the replay produced.
        got: Box<CrashArtifact>,
    },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tuple(field) => write!(f, "the engine differs from the artifact in {field}"),
            Self::NoFailure => f.write_str("the replay reached the instant without failing"),
            Self::JournalRequired {
                position,
                submitted,
            } => write!(
                f,
                "the run had accepted {position} material inputs before it failed, the replay {submitted}: submit the journal's first {position} inputs first"
            ),
            Self::Mismatch { expected, got } => write!(
                f,
                "the replay failed at {} in {} with \"{}\", not at {} in {} with \"{}\"",
                got.tick, got.phase, got.error, expected.tick, expected.phase, expected.error
            ),
        }
    }
}

impl std::error::Error for ReplayError {}

/// The engine: a registered system set running over the world tables.
pub struct Engine {
    calendar: Calendar,
    world_seed: u64,
    branch_salt: BranchSalt,
    registered: Registered,
    tables: WorldTables,
    cursor: Option<SimTime>,
    threads: usize,
    bound_pulses: Vec<Pulse>,
    lagged_columns: BTreeSet<&'static str>,
    agenda: Agenda,
    events: EventLog,
    commands: CommandQueues,
    command_owner: BTreeMap<&'static str, &'static str>,
    by_system_id: BTreeMap<SystemId, usize>,
    epochs: BTreeMap<EntityRef, u32>,
    inputs: BTreeMap<SimTime, Vec<MaterialInput>>,
    /// The number of material inputs accepted ([`Engine::submit_input`]), applied or still queued:
    /// the journal position (VER-SPR-007-TEST GAP-10). The host journals an input and then submits
    /// it, so this is the length of the journal's `seq`-order prefix the engine holds.
    accepted_inputs: u64,
    timeline: TimelineId,
    provenance: Provenance,
    diagnostics: VecDeque<String>,
    at: (SimTime, Phase),
    crash: Option<CrashArtifact>,
    /// The journal's chain head (hex) and the last keyframe, as the host last set them; only a
    /// crash artifact records them (SPR-007).
    journal_head: Option<String>,
    last_keyframe: Option<String>,
    /// The error that stopped the run, if any. A poisoned engine never runs again (SPR-005 G-d).
    poisoned: Option<SimError>,
    /// How many levels ran and how many of them held more than one job (SPR-008 DEF-2). An
    /// observation of the scheduler, never part of the state, a keyframe or a hash.
    dispatch: DispatchStats,
    /// The host's observer, if attached (SPR-009 ruling 4): notices only, never state, so it is
    /// not part of [`EngineState`] and never changes a result.
    observer: Option<Arc<dyn Observer>>,
    /// The person ledger and life-course log, when the world has them (SPR-011 ruling 5): which
    /// columns the core compacts at `Bookkeeping` and checks after each step that owns one.
    ledger: Option<Ledger>,
    /// The `(purpose, entity)` pairs each system (by entry index) has drawn from at the current
    /// instant, lent to each of its jobs in turn and cleared at every instant (BCK-002 A-37).
    /// Bookkeeping of the step guard, never state: an instant always starts it empty. One slot
    /// per system, built once, so an instant with no draws allocates nothing (V-PERF-11).
    taken: Vec<Taken>,
}

/// What the scheduler dispatched since the engine was built or restored (SPR-008 DEF-2): the
/// evidence that a thread-count comparison exercises parallel levels at all. It depends on the
/// schedule and the instants run, never on the thread count, and is not part of the state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DispatchStats {
    /// Levels run (timer-pass and binding levels, each split level counted once).
    pub levels: u64,
    /// Levels that held two or more jobs: the ones several threads can share.
    pub parallel_levels: u64,
    /// The most jobs any level held.
    pub max_jobs: usize,
}

impl DispatchStats {
    /// Counts one level of `jobs` jobs.
    fn record(&mut self, jobs: usize) {
        self.levels += 1;
        if jobs > 1 {
            self.parallel_levels += 1;
        }
        self.max_jobs = self.max_jobs.max(jobs);
    }
}

/// One system's work in one pass: its due timers, or one firing binding.
enum Work {
    Timers(Vec<TimerEntry>),
    Step(PulseBinding),
}

/// One system's work in one pass of one instant and phase.
struct Job {
    index: usize,
    work: Work,
}

/// The columns one system owns, moved out of the tables for its step.
type Owned = BTreeMap<&'static str, Column>;

/// What one job takes to its thread: its owned columns, its command inbox, and the pairs its
/// system has drawn from at this instant (BCK-002 A-37).
type JobInput = (Owned, Vec<QueuedCommand>, Taken);

/// The result of one job.
struct JobOutcome {
    owned: Owned,
    result: Result<(), SimError>,
    untaken: Vec<QueuedCommand>,
    out: Outbox,
    taken: Taken,
}

/// Counts for the per-instant diagnostic record.
#[derive(Default)]
struct Tally {
    timers: usize,
    events: usize,
    commands: usize,
    inputs: usize,
}

impl Engine {
    /// An engine whose first instant is the first bound-pulse boundary at or after `start`.
    ///
    /// Every column a registered system declares as a write and `tables` lacks is materialised
    /// here, empty: the only place a column is created (SPR-005 G-c). At run time an access to a
    /// column the world does not have is a fatal [`SimError::MissingColumn`], never a creation.
    ///
    /// A system that declares a `previous_pulse` read, or a `start_of_day` read while bound to an
    /// Hour pulse, leaves the engine poisoned from the start with [`SimError::Unsupported`] (the
    /// first such read in id order): P0 reads those lags at the start of the instant, which is not
    /// their meaning (BCK-002 A-28). [`Engine::advance_until`] returns that error and runs
    /// nothing.
    #[must_use]
    pub fn new(
        calendar: Calendar,
        world_seed: u64,
        branch_salt: BranchSalt,
        registered: Registered,
        tables: WorldTables,
        start: SimTime,
    ) -> Self {
        let mut tables = tables;
        let mut bound: BTreeSet<Pulse> = BTreeSet::new();
        let mut lagged = BTreeSet::new();
        let mut command_owner = BTreeMap::new();
        let mut by_system_id = BTreeMap::new();
        for (i, e) in registered.entries.iter().enumerate() {
            for w in &e.access.writes {
                if tables.get(w.column).is_none() {
                    tables.insert(w.column, Column::new());
                }
            }
            bound.extend(e.bindings.iter().map(|b| b.pulse));
            lagged.extend(
                e.access
                    .reads
                    .iter()
                    .filter(|r| r.lag.is_lagged())
                    .map(|r| r.column),
            );
            for &c in &e.access.applies {
                command_owner.entry(c).or_insert(e.id);
            }
            by_system_id.insert(e.system_id, i);
        }
        let (ledger, ledger_error) = match Ledger::of(&tables) {
            Ok(ledger) => (ledger, None),
            Err(e) => (None, Some(e)),
        };
        let poisoned = unsupported_lag(&registered.entries).or(ledger_error);
        Self {
            calendar,
            world_seed,
            branch_salt,
            registered,
            tables,
            cursor: Some(start),
            threads: 1,
            bound_pulses: bound.into_iter().collect(),
            lagged_columns: lagged,
            agenda: Agenda::new(),
            events: EventLog::new(),
            commands: CommandQueues::new(),
            command_owner,
            by_system_id,
            epochs: BTreeMap::new(),
            inputs: BTreeMap::new(),
            accepted_inputs: 0,
            timeline: TimelineId::from_raw(0),
            provenance: Provenance::default(),
            diagnostics: VecDeque::new(),
            at: (start, Phase::Inputs),
            crash: None,
            journal_head: None,
            last_keyframe: None,
            poisoned,
            dispatch: DispatchStats::default(),
            observer: None,
            ledger,
            taken: Vec::new(),
        }
    }

    /// The engine's plane-A state (SPR-007 T-02): what a keyframe holds and `state_hash` covers.
    /// A poisoned engine still has a state, but a keyframe of it is refused by its writer.
    #[must_use]
    pub fn snapshot(&self) -> EngineState {
        EngineState {
            cursor: self.cursor,
            timeline: self.timeline,
            accepted_inputs: self.accepted_inputs,
            tables: self.tables.clone(),
            agenda: self.agenda.state(),
            commands: self.commands.state(),
            epochs: self.epochs.iter().map(|(e, n)| (*e, *n)).collect(),
            inputs: self
                .inputs
                .iter()
                .map(|(t, list)| (*t, list.clone()))
                .collect(),
            events: self.events.state(),
        }
    }

    /// Rebuilds an engine from its state, for the registered systems and the history tuple given
    /// (SPR-007 T-02). Every part must be in the canonical form [`Engine::snapshot`] produces,
    /// and every name in it (timer owners, command owners, senders and kinds, event emitters and
    /// kinds) must belong to `registered`; `restore(x).snapshot() == x`.
    ///
    /// # Errors
    /// [`RestoreError`] naming the first part that is not canonical or names something unknown,
    /// or a declared written column missing from the tables.
    pub fn restore(
        calendar: Calendar,
        world_seed: u64,
        branch_salt: BranchSalt,
        registered: Registered,
        provenance: Provenance,
        state: EngineState,
    ) -> Result<Self, RestoreError> {
        for e in &registered.entries {
            if let Some(w) = e
                .access
                .writes
                .iter()
                .find(|w| state.tables.get(w.column).is_none())
            {
                return Err(RestoreError::Tables(format!(
                    "{} writes {}, which the state lacks",
                    e.id, w.column
                )));
            }
        }
        let agenda = Agenda::restore(&state.agenda).map_err(RestoreError::Agenda)?;
        let commands =
            CommandQueues::from_state(&state.commands).map_err(RestoreError::Commands)?;
        check_epochs(&state.epochs)?;
        check_inputs(&state.inputs, state.cursor, state.accepted_inputs)?;
        check_timers(&state.agenda.entries, state.cursor)?;
        let events = EventLog::from_state(&state.events).map_err(RestoreError::Events)?;
        let start = state.cursor.unwrap_or(SimTime::EPOCH);
        let mut engine = Self::new(
            calendar,
            world_seed,
            branch_salt,
            registered,
            state.tables,
            start,
        );
        engine.cursor = state.cursor;
        engine.at = (start, Phase::Inputs);
        for (key, timer) in &state.agenda.entries {
            if !engine.by_system_id.contains_key(&timer.owner) {
                return Err(RestoreError::TimerOwner(format!(
                    "timer seq {} names the unregistered system {}",
                    key.seq, timer.owner
                )));
            }
        }
        for (owner, queue) in &state.commands.queues {
            for c in queue {
                if engine.command_owner.get(c.command.kind) != Some(owner) {
                    return Err(RestoreError::Commands(format!(
                        "{} is not the owner of command {}",
                        owner, c.command.kind
                    )));
                }
                if engine.registered.entries.iter().all(|e| e.id != c.sender) {
                    return Err(RestoreError::Commands(format!(
                        "command seq {} names the unregistered sender {}",
                        c.seq, c.sender
                    )));
                }
            }
        }
        for ev in &state.events.events {
            let declared = ev.emitter == CORE
                || engine
                    .registered
                    .entries
                    .iter()
                    .any(|e| e.id == ev.emitter && e.access.emits.contains(&ev.kind));
            if !declared {
                return Err(RestoreError::Events(format!(
                    "event {} ({} by {}) is not declared by a registered system",
                    ev.id, ev.kind, ev.emitter
                )));
            }
        }
        engine.agenda = agenda;
        engine.commands = commands;
        engine.epochs = state.epochs.into_iter().collect();
        engine.inputs = state.inputs.into_iter().collect();
        engine.accepted_inputs = state.accepted_inputs;
        engine.timeline = state.timeline;
        engine.provenance = provenance;
        engine.events = events;
        Ok(engine)
    }

    /// Sets the number of worker threads (at least 1). Changes speed only, never the result.
    pub fn set_threads(&mut self, threads: usize) {
        self.threads = threads.max(1);
    }

    /// Sets the build, Registry version and manifest hash a crash artifact records.
    pub fn set_provenance(&mut self, provenance: Provenance) {
        self.provenance = provenance;
    }

    /// Sets the journal's chain head (hex) a crash artifact records (SPR-007): the host updates
    /// it as it journals inputs.
    pub fn set_journal_head(&mut self, head: Option<String>) {
        self.journal_head = head;
    }

    /// Sets the last keyframe a crash artifact records, as `<path>#<state hash>` (SPR-007).
    pub fn set_last_keyframe(&mut self, keyframe: Option<String>) {
        self.last_keyframe = keyframe;
    }

    /// Attaches the host's observer, or detaches it with `None` (SPR-009 ruling 4; [`crate::observe`]).
    /// It receives boundary notices with no clock and no state; attached or not, every result is
    /// the same.
    pub fn set_observer(&mut self, observer: Option<Arc<dyn Observer>>) {
        self.observer = observer;
    }

    /// Sends one notice from the engine's own thread, if an observer is attached. A panic in the
    /// observer is a fatal [`SimError::Observer`] (DEF-1), which poisons the engine like any other.
    fn notify(
        &self,
        at: SimTime,
        phase: Option<Phase>,
        boundary: Boundary,
    ) -> Result<(), SimError> {
        let notice = Notice {
            at,
            phase,
            worker: 0,
            boundary,
        };
        self.observer
            .as_deref()
            .map_or(Ok(()), |o| observe(o, &notice))
    }

    /// Sets the timeline id (0, the canonical timeline, by default).
    pub fn set_timeline(&mut self, timeline: TimelineId) {
        self.timeline = timeline;
    }

    /// The world seed this engine draws with (VER-SPR-007-TEST DEF-4: a keyframe takes its history
    /// tuple from the engine, never from the caller).
    #[must_use]
    pub const fn world_seed(&self) -> u64 {
        self.world_seed
    }

    /// The branch salt this engine draws with.
    #[must_use]
    pub const fn branch_salt(&self) -> BranchSalt {
        self.branch_salt
    }

    /// The journal position: the number of material inputs accepted, applied or still queued
    /// (VER-SPR-007-TEST GAP-10). A keyframe records it, a crash artifact records it, and a host
    /// resuming from either submits only the journal's inputs after it.
    #[must_use]
    pub const fn journal_position(&self) -> u64 {
        self.accepted_inputs
    }

    /// The number of material inputs applied so far: accepted, less those still queued.
    #[must_use]
    pub fn applied_inputs(&self) -> u64 {
        let queued: usize = self.inputs.values().map(Vec::len).sum();
        self.accepted_inputs
            .saturating_sub(u64::try_from(queued).unwrap_or(u64::MAX))
    }

    /// The build, Registry version and manifest hash this engine runs under.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// The tables.
    #[must_use]
    pub fn tables(&self) -> &WorldTables {
        &self.tables
    }

    /// The registered systems.
    #[must_use]
    pub fn registered(&self) -> &Registered {
        &self.registered
    }

    /// The agenda.
    #[must_use]
    pub fn agenda(&self) -> &Agenda {
        &self.agenda
    }

    /// The event log.
    #[must_use]
    pub fn events(&self) -> &EventLog {
        &self.events
    }

    /// Hands the logged events to a consumer; ids keep counting.
    pub fn drain_events(&mut self) -> Vec<Event> {
        self.events.drain()
    }

    /// The lifecycle command queues.
    #[must_use]
    pub fn commands(&self) -> &CommandQueues {
        &self.commands
    }

    /// The epoch of `entity`: the number of material inputs that have touched it.
    #[must_use]
    pub fn epoch(&self, entity: EntityRef) -> u32 {
        self.epochs.get(&entity).copied().unwrap_or(0)
    }

    /// The error that poisoned the engine, if an instant has failed.
    #[must_use]
    pub fn poisoned(&self) -> Option<&SimError> {
        self.poisoned.as_ref()
    }

    /// The crash artifact of the failed instant, once one has failed.
    #[must_use]
    pub fn crash_artifact(&self) -> Option<&CrashArtifact> {
        self.crash.as_ref()
    }

    /// What the scheduler has dispatched so far (SPR-008 DEF-2).
    #[must_use]
    pub const fn dispatch_stats(&self) -> DispatchStats {
        self.dispatch
    }

    /// The last diagnostic records, oldest first.
    #[must_use]
    pub fn diagnostics(&self) -> Vec<String> {
        self.diagnostics.iter().cloned().collect()
    }

    /// The next instant to run, if any: the earliest of the next bound-pulse boundary, the first
    /// due timer and the first queued input.
    #[must_use]
    pub fn next_instant(&self) -> Option<SimTime> {
        let c = self.cursor?;
        let pulse = if self
            .bound_pulses
            .iter()
            .any(|&p| self.calendar.is_boundary(p, c))
        {
            Some(c)
        } else {
            self.calendar.next_instant(c, &self.bound_pulses)
        };
        let timer = self.agenda.peek().map(|k| k.time);
        let input = self.inputs.keys().next().copied();
        [pulse, timer, input].into_iter().flatten().min()
    }

    /// Queues a material input for instant `at`, applied in that instant's `Inputs` phase.
    ///
    /// # Errors
    /// [`InputError::PastInput`] if `at` is before the next instant the engine can run.
    pub fn submit_input(&mut self, at: SimTime, input: MaterialInput) -> Result<(), InputError> {
        match self.cursor {
            Some(c) if at >= c => {
                self.inputs.entry(at).or_default().push(input);
                self.accepted_inputs += 1;
                Ok(())
            }
            cursor => Err(InputError::PastInput { at, next: cursor }),
        }
    }

    /// Runs every instant up to and including `end`, calling `after_instant` after each one with
    /// the instant and the tables (for hashes, logging).
    ///
    /// # Errors
    /// The first [`SimError`] in schedule order, or a violated invariant; the engine stops at that
    /// instant, keeps a crash artifact ([`Engine::crash_artifact`]) and is poisoned: every later
    /// call returns that same error and runs nothing, so a failed instant is never re-run on
    /// half-mutated tables (SPR-005 G-d).
    pub fn advance_until(
        &mut self,
        end: SimTime,
        mut after_instant: impl FnMut(SimTime, &WorldTables),
    ) -> Result<(), SimError> {
        if let Some(e) = &self.poisoned {
            return Err(e.clone());
        }
        while let Some(t) = self.next_instant() {
            if t > end {
                break;
            }
            if let Err(e) = self.run_instant(t) {
                self.record(format!("{t} FATAL in {}: {e}", self.at.1));
                self.crash = Some(self.artifact(&e));
                self.poisoned = Some(e.clone());
                return Err(e);
            }
            after_instant(t, &self.tables);
            self.cursor = t.checked_add_minutes(1);
        }
        Ok(())
    }

    /// Replays `artifact` on this engine, which the host built from the artifact's tuple: runs to
    /// the failing instant and checks that the same failure recurs there (GOV-004 §4).
    ///
    /// # Errors
    /// [`ReplayError`] if the tuple differs, the run does not fail, or it fails differently;
    /// [`ReplayError::JournalRequired`] unless this engine has accepted exactly the artifact's
    /// journal position of inputs: the host submits the journal's first `journal_position` inputs
    /// (in `seq` order) before replaying, as the run did before it failed.
    pub fn replay(&mut self, artifact: &CrashArtifact) -> Result<(), ReplayError> {
        let checks: [(&'static str, bool); 6] = [
            ("timeline", artifact.timeline == self.timeline.to_raw()),
            ("build", artifact.build == self.provenance.build),
            (
                "registry_version",
                artifact.registry_version == self.provenance.registry_version,
            ),
            (
                "manifest_hash",
                artifact.manifest_hash == self.provenance.manifest_hash,
            ),
            ("seed", artifact.seed == self.world_seed),
            (
                "branch_salt",
                artifact.branch_salt == self.branch_salt.to_raw(),
            ),
        ];
        if let Some((field, _)) = checks.iter().find(|(_, ok)| !ok) {
            return Err(ReplayError::Tuple(field));
        }
        if artifact.journal_position != self.accepted_inputs {
            return Err(ReplayError::JournalRequired {
                position: artifact.journal_position,
                submitted: self.accepted_inputs,
            });
        }
        if self
            .advance_until(SimTime::from_minutes(artifact.tick), |_, _| {})
            .is_ok()
        {
            return Err(ReplayError::NoFailure);
        }
        let Some(got) = self.crash.clone() else {
            return Err(ReplayError::NoFailure);
        };
        let same = got.tick == artifact.tick
            && got.phase == artifact.phase
            && got.system == artifact.system
            && got.invariant == artifact.invariant
            && got.error == artifact.error;
        if same {
            Ok(())
        } else {
            Err(ReplayError::Mismatch {
                expected: Box::new(artifact.clone()),
                got: Box::new(got),
            })
        }
    }

    /// Every system's invariants over the current tables, in id order. A host helper outside a
    /// run: a panic here reaches the host. During a run, `Bookkeeping` checks them through `guard`.
    #[must_use]
    pub fn invariants(&self) -> Vec<(&'static str, InvariantResult)> {
        self.registered
            .entries
            .iter()
            .flat_map(|e| {
                e.model
                    .invariants(&self.tables)
                    .into_iter()
                    .map(move |r| (e.id, r))
            })
            .collect()
    }

    fn record(&mut self, line: String) {
        if self.diagnostics.len() == DIAGNOSTIC_RECORDS {
            self.diagnostics.pop_front();
        }
        self.diagnostics.push_back(line);
    }

    fn artifact(&self, error: &SimError) -> CrashArtifact {
        let (system, invariant) = match error {
            SimError::Invariant { system, name, .. } => {
                ((*system).to_owned(), Some((*name).to_owned()))
            }
            SimError::UndeclaredRead { system, .. }
            | SimError::UnownedWrite { system, .. }
            | SimError::MissingColumn { system, .. }
            | SimError::UndeclaredPurpose { system, .. }
            | SimError::StreamReuse { system, .. }
            | SimError::Timer { system, .. }
            | SimError::UndeclaredCommand { system, .. }
            | SimError::UndeclaredEvent { system, .. }
            | SimError::Event { system, .. }
            | SimError::Panic { system, .. }
            | SimError::Unsupported { system, .. }
            | SimError::Ledger { system, .. }
            | SimError::System { system, .. } => ((*system).to_owned(), None),
            SimError::Observer { .. } => (OBSERVER.to_owned(), None),
        };
        CrashArtifact {
            timeline: self.timeline.to_raw(),
            build: self.provenance.build.clone(),
            registry_version: self.provenance.registry_version.clone(),
            manifest_hash: self.provenance.manifest_hash.clone(),
            seed: self.world_seed,
            branch_salt: self.branch_salt.to_raw(),
            journal_position: self.accepted_inputs,
            journal_head: self.journal_head.clone(),
            last_keyframe: self.last_keyframe.clone(),
            tick: self.at.0.minutes(),
            phase: self.at.1.name().to_owned(),
            system,
            invariant,
            error: error.to_string(),
            diagnostics: self.diagnostics(),
        }
    }

    fn run_instant(&mut self, t: SimTime) -> Result<(), SimError> {
        // The instant is the current one before its first notice, so a failure there (an observer
        // panic at InstantStart) names this instant, not the last (VER-SPR-009-TEST OBS-1).
        self.at = (t, Phase::Inputs);
        self.taken
            .resize_with(self.registered.entries.len(), Taken::default);
        self.taken.iter_mut().for_each(Taken::clear);
        self.notify(t, None, Boundary::InstantStart)?;
        let fired: Vec<Pulse> = self
            .calendar
            .pulses_at(t)
            .into_iter()
            .filter(|p| self.bound_pulses.contains(p))
            .collect();
        let mut snapshot = WorldTables::new();
        for &c in &self.lagged_columns {
            if let Some(rows) = self.tables.get(c) {
                snapshot.insert(c, rows.clone());
            }
        }
        let mut tally = Tally::default();
        for phase in Phase::ALL {
            self.at = (t, phase);
            self.notify(t, Some(phase), Boundary::PhaseStart)?;
            if phase == Phase::Inputs {
                tally.inputs = self.apply_inputs(t)?;
            }
            // SPR-011 ruling 5: the dead of this instant leave every person.* column before any
            // Bookkeeping system, invariant or hash sees the tables.
            if phase == Phase::Bookkeeping
                && let Some(ledger) = &self.ledger
            {
                ledger.compact(&mut self.tables)?;
            }
            let mut due: BTreeMap<usize, Vec<TimerEntry>> = BTreeMap::new();
            for (key, timer) in self.agenda.pop_due(t, phase) {
                let Some(&i) = self.by_system_id.get(&timer.owner) else {
                    return Err(SimError::Timer {
                        system: CORE,
                        message: format!("{} is owned by no registered system", key.id()),
                    });
                };
                tally.timers += 1;
                due.entry(i).or_default().push(TimerEntry { key, timer });
            }
            // The timer pass, then the firing bindings (SPR-005 DEF-7). Both run the canonical
            // order: dependency levels over every binding, each level coarser pulse first, then id.
            let plan = self.registered.schedule().phase(phase).clone();
            let mut passes: Vec<(Pass, Vec<Vec<Job>>)> = Vec::new();
            // Timer pass: each owner once, at the level of its first binding in the phase, with
            // its timers in key order.
            let timer_pass: Vec<Vec<Job>> = plan
                .levels
                .iter()
                .map(|level| {
                    level
                        .iter()
                        .filter_map(|n| {
                            due.remove(&n.system).map(|timers| Job {
                                index: n.system,
                                work: Work::Timers(timers),
                            })
                        })
                        .collect::<Vec<Job>>()
                })
                .filter(|jobs| !jobs.is_empty())
                .collect();
            if let Some((&i, timers)) = due.iter().next() {
                return Err(SimError::Timer {
                    system: self.registered.entries[i].id,
                    message: format!(
                        "{} is due in phase {phase}, where the system is not bound",
                        timers[0].id()
                    ),
                });
            }
            passes.push((Pass::Timers, timer_pass));
            // Binding pass: a level is split wherever a system's second binding appears.
            let binding_pass = binding_pass(&plan.levels, &fired, phase);
            passes.push((Pass::Bindings, binding_pass));
            self.run_passes((t, phase), &snapshot, passes, &mut tally)?;
            if phase == Phase::Bookkeeping {
                self.check_invariants(t)?;
            }
            self.notify(t, Some(phase), Boundary::PhaseEnd)?;
        }
        let pulses: Vec<&str> = fired.iter().map(|p| p.name()).collect();
        self.record(format!(
            "{t} pulses=[{}] inputs={} timers_fired={} commands={} events={} agenda={}",
            pulses.join(","),
            tally.inputs,
            tally.timers,
            tally.commands,
            tally.events,
            self.agenda.len()
        ));
        self.notify(t, None, Boundary::InstantEnd)?;
        Ok(())
    }

    /// Runs the passes of one phase level by level, each level between its observer notices.
    fn run_passes(
        &mut self,
        (t, phase): (SimTime, Phase),
        snapshot: &WorldTables,
        passes: Vec<(Pass, Vec<Vec<Job>>)>,
        tally: &mut Tally,
    ) -> Result<(), SimError> {
        for (pass, levels) in passes {
            for (level, jobs) in levels.iter().enumerate() {
                let start = Boundary::LevelStart {
                    pass,
                    level,
                    jobs: jobs.len(),
                };
                self.notify(t, Some(phase), start)?;
                self.run_level((t, phase), snapshot, jobs, tally)?;
                self.notify(t, Some(phase), Boundary::LevelEnd)?;
            }
        }
        Ok(())
    }

    /// Applies the material inputs queued for `t` (ADR-0025 item 3). Returns how many.
    fn apply_inputs(&mut self, t: SimTime) -> Result<usize, SimError> {
        let inputs = self.inputs.remove(&t).unwrap_or_default();
        for input in &inputs {
            for &entity in &input.scope {
                let epoch = self.epochs.entry(entity).or_insert(0);
                *epoch = epoch.checked_add(1).ok_or_else(|| SimError::System {
                    system: CORE,
                    message: format!("the epoch of {entity} overflowed"),
                })?;
            }
            let mut touched: Vec<TimerEntry> = input
                .scope
                .iter()
                .flat_map(|&entity| self.agenda.pending_for(entity))
                .map(|(key, timer)| TimerEntry { key, timer })
                .collect();
            touched.sort_unstable();
            for entry in touched {
                self.resample(t, entry)?;
            }
        }
        Ok(inputs.len())
    }

    /// Cancels one timer and re-schedules it where its owner re-draws it.
    fn resample(&mut self, t: SimTime, entry: TimerEntry) -> Result<(), SimError> {
        let cancel = |e: crate::agenda::AgendaError| SimError::Timer {
            system: CORE,
            message: e.to_string(),
        };
        self.agenda.cancel(entry.id()).map_err(cancel)?;
        let Some(&i) = self.by_system_id.get(&entry.timer.owner) else {
            return Err(SimError::Timer {
                system: CORE,
                message: format!("{} is owned by no registered system", entry.id()),
            });
        };
        let e = &self.registered.entries[i];
        let epoch = self.epoch(entry.key.entity);
        let ctx = ResampleCtx {
            key: SystemKey::new(self.world_seed, self.branch_salt, e.system_id, epoch),
            purpose: entry.timer.purpose,
            entity: entry.key.entity,
            t_input: t,
            epoch,
        };
        if let Some(time) = guard(e.id, || e.model.resample_timer(&entry, &ctx))? {
            if (time, entry.key.phase) <= (t, Phase::Inputs) {
                return Err(SimError::Timer {
                    system: e.id,
                    message: format!(
                        "{} re-drawn at ({time}, {}), not after the input at ({t}, Inputs): no back-dating (ADR-0025)",
                        entry.id(),
                        entry.key.phase
                    ),
                });
            }
            self.agenda
                .insert(time, entry.key.phase, entry.key.entity, entry.timer);
        }
        Ok(())
    }

    /// The core's invariants, then each system's in id order; the first violation is an error.
    fn check_invariants(&self, t: SimTime) -> Result<(), SimError> {
        let violated = |name: &'static str, detail: String| SimError::Invariant {
            system: CORE,
            name,
            detail,
        };
        if let Some(k) = self.agenda.peek()
            && k.time <= t
        {
            return Err(violated(
                "agenda-no-stale-timer",
                format!(
                    "{} is due at {} but did not fire by the end of {t}",
                    k.id(),
                    k.time
                ),
            ));
        }
        if let Some((tick, _)) = self.events.last()
            && tick > t.minutes()
        {
            return Err(violated(
                "events-not-in-future",
                format!("an event carries tick {tick}, after the current instant {t}"),
            ));
        }
        if let Some(ledger) = &self.ledger {
            ledger.check(&self.tables)?;
        }
        for e in &self.registered.entries {
            let results = guard(e.id, || Ok(e.model.invariants(&self.tables)))?;
            if let Some(r) = results.into_iter().find(|r| !r.holds) {
                return Err(SimError::Invariant {
                    system: e.id,
                    name: r.name,
                    detail: r.detail,
                });
            }
        }
        Ok(())
    }

    /// Runs one level of jobs (in order on one thread, spread over several otherwise), returns
    /// every owned column to the tables, reports the first error in job order, and otherwise
    /// merges the level's buffers in job order (ruling 1).
    fn run_level(
        &mut self,
        (t, phase): (SimTime, Phase),
        snapshot: &WorldTables,
        jobs: &[Job],
        tally: &mut Tally,
    ) -> Result<(), SimError> {
        self.dispatch.record(jobs.len());
        self.check_owned_columns(jobs)?;
        let guards = self.ledger_guards(jobs, phase);
        let inputs: Vec<JobInput> = jobs
            .iter()
            .map(|job| {
                let e = &self.registered.entries[job.index];
                let owned = e
                    .access
                    .writes
                    .iter()
                    .filter_map(|w| self.tables.take(w.column).map(|rows| (w.column, rows)))
                    .collect();
                let taken = std::mem::take(&mut self.taken[job.index]);
                (owned, self.commands.take(e.id), taken)
            })
            .collect();
        let workers = self.threads.min(jobs.len()).max(1);
        let outcomes: Vec<JobOutcome> = {
            let observer = self.observer.as_deref();
            let tables = &self.tables;
            let agenda = &self.agenda;
            let entries = &self.registered.entries;
            let epochs = &self.epochs;
            let (world_seed, branch_salt) = (self.world_seed, self.branch_salt);
            let key = |e: &Entry| SystemKey::new(world_seed, branch_salt, e.system_id, 0);
            let run = |k: usize, job: &Job, (cols, inbox, taken): JobInput| {
                let e = &entries[job.index];
                let notice = |boundary| Notice {
                    at: t,
                    phase: Some(phase),
                    worker: k % workers,
                    boundary,
                };
                let job_ref = job_ref(e.id, job, k);
                // DEF-1: an observer panic is this job's fatal error; the job's columns still go
                // back to the tables with the others.
                let started =
                    observer.map_or(Ok(()), |o| observe(o, &notice(Boundary::JobStart(job_ref))));
                let mut view = SystemView::new(e.id, &e.access, cols, tables, snapshot);
                let info = JobInfo {
                    system: e.id,
                    system_id: e.system_id,
                    key: key(e),
                    world_seed,
                    branch_salt,
                    epochs,
                    purposes: &e.purposes,
                    access: &e.access,
                    bindings: &e.bindings,
                    agenda,
                };
                let mut ctx = StepCtx::new(info, t, phase, inbox, taken);
                // A failed JobStart skips the step; the instant is lost either way.
                let result = started.and_then(|()| match &job.work {
                    Work::Timers(timers) => timers.iter().try_for_each(|timer| {
                        guard(e.id, || e.model.fire(timer, &mut view, &mut ctx))
                    }),
                    Work::Step(binding) => {
                        guard(e.id, || e.model.step(*binding, &mut view, &mut ctx))
                    }
                });
                let (untaken, out, taken) = ctx.finish();
                let ended =
                    observer.map_or(Ok(()), |o| observe(o, &notice(Boundary::JobEnd(job_ref))));
                let result = result.and(ended);
                JobOutcome {
                    owned: view.into_owned(),
                    result,
                    untaken,
                    out,
                    taken,
                }
            };
            dispatch(self.threads, jobs, inputs, run)
        };
        let mut first_error = None;
        let mut outboxes = Vec::with_capacity(outcomes.len());
        for (job, o) in jobs.iter().zip(outcomes) {
            for (name, rows) in o.owned {
                self.tables.insert(name, rows);
            }
            self.commands
                .restore(self.registered.entries[job.index].id, o.untaken);
            self.taken[job.index] = o.taken;
            if first_error.is_none() {
                first_error = o.result.err();
            }
            outboxes.push(o.out);
        }
        if let Some(e) = first_error {
            return Err(e);
        }
        // `guards` is empty without a ledger: the zip pairs nothing (DEF-4).
        for (job, g) in jobs.iter().zip(&guards) {
            g.as_ref().map_or(Ok(()), |g| {
                g.check(self.registered.entries[job.index].id, &self.tables)
            })?;
        }
        // After every owned column is back in the tables, so a failing observer loses none (DEF-1).
        self.notify(t, Some(phase), Boundary::LevelMerge)?;
        for (job, out) in jobs.iter().zip(outboxes) {
            self.merge(t, phase, job.index, out, tally)?;
        }
        Ok(())
    }

    /// What each job must preserve of the ledger columns it owns (SPR-011 ruling 5): `None` for a
    /// job that owns none, or in a world without a ledger.
    /// A world without a ledger pays nothing: no guard and no allocation (DEF-4, V-PERF-11).
    fn ledger_guards(&self, jobs: &[Job], phase: Phase) -> Vec<Option<LifeCourseGuard>> {
        if self.ledger.is_none() {
            return Vec::new();
        }
        jobs.iter()
            .map(|job| {
                let e = &self.registered.entries[job.index];
                self.ledger.as_ref().and_then(|ledger| {
                    let writes = e.access.writes.iter().map(|w| w.column);
                    let g = ledger.guard(&self.tables, writes, phase);
                    (!g.is_empty()).then_some(g)
                })
            })
            .collect()
    }

    /// Every column the level's jobs own exists (materialised in `new`). Checked for all before any
    /// is moved, so a missing one fails the step without disturbing the tables (SPR-005 G-c).
    fn check_owned_columns(&self, jobs: &[Job]) -> Result<(), SimError> {
        for job in jobs {
            let e = &self.registered.entries[job.index];
            if let Some(w) = e
                .access
                .writes
                .iter()
                .find(|w| self.tables.get(w.column).is_none())
            {
                return Err(SimError::MissingColumn {
                    system: e.id,
                    column: w.column.to_owned(),
                });
            }
        }
        Ok(())
    }

    /// Commits one system's buffers: cancellations, timers, events, commands (ruling 1).
    fn merge(
        &mut self,
        t: SimTime,
        phase: Phase,
        index: usize,
        out: Outbox,
        tally: &mut Tally,
    ) -> Result<(), SimError> {
        let e = &self.registered.entries[index];
        for id in out.cancels {
            self.agenda.cancel(id).map_err(|err| SimError::Timer {
                system: e.id,
                message: err.to_string(),
            })?;
        }
        for (spec, purpose) in out.timers {
            self.agenda.insert(
                spec.time,
                spec.phase,
                spec.entity,
                Timer {
                    owner: e.system_id,
                    purpose,
                    kind: spec.kind,
                    payload: spec.payload,
                },
            );
        }
        let mut ids = Vec::with_capacity(out.events.len());
        for mut draft in out.events {
            let local = std::mem::take(&mut draft.local_causes);
            // INVARIANT: `StepCtx::emit` admits only references to earlier drafts.
            draft
                .causes
                .extend(local.iter().filter_map(|r| ids.get(r.0).copied()));
            let id = self
                .events
                .append(t.minutes(), phase.index(), e.id, draft)
                .map_err(|err| SimError::Event {
                    system: e.id,
                    message: err.to_string(),
                })?;
            ids.push(id);
            tally.events += 1;
        }
        for command in out.commands {
            let Some(&owner) = self.command_owner.get(command.kind) else {
                return Err(SimError::UndeclaredCommand {
                    system: e.id,
                    command: command.kind,
                });
            };
            // INVARIANT: `StepCtx::submit` admits only references to drafts already emitted.
            let cause = command.cause.and_then(|r| ids.get(r.0).copied());
            self.commands.push(owner, t, phase, e.id, cause, command);
            tally.commands += 1;
        }
        Ok(())
    }
}

/// The binding pass of one phase: the bindings that fire at this instant, level by level, a level
/// split wherever one system's second binding appears, so one system's bindings never run at once
/// (they share its columns); each split is a level for the merge. The split depends on the
/// schedule only, never on the thread count (SPR-005 DEF-7).
fn binding_pass(levels: &[Vec<Node>], fired: &[Pulse], phase: Phase) -> Vec<Vec<Job>> {
    let mut pass: Vec<Vec<Job>> = Vec::new();
    for level in levels {
        let mut groups: Vec<Vec<Job>> = vec![Vec::new()];
        for n in level.iter().filter(|n| fired.contains(&n.pulse)) {
            let repeats = groups
                .last()
                .is_some_and(|g| g.iter().any(|j| j.index == n.system));
            if repeats {
                groups.push(Vec::new());
            }
            if let Some(g) = groups.last_mut() {
                g.push(Job {
                    index: n.system,
                    work: Work::Step(PulseBinding {
                        pulse: n.pulse,
                        phase,
                    }),
                });
            }
        }
        pass.extend(groups.into_iter().filter(|g| !g.is_empty()));
    }
    pass
}

/// What an observer is told about job `k` of a level (SPR-009 ruling 4): no state, only names.
fn job_ref(system: &'static str, job: &Job, k: usize) -> JobRef {
    JobRef {
        system,
        work: match &job.work {
            Work::Timers(timers) => JobWork::Timers(timers.len()),
            Work::Step(binding) => JobWork::Binding(*binding),
        },
        index: k,
    }
}

/// Runs `run` on every job with its input: in order on one thread, or spread over `threads`
/// workers (job k on worker k % threads, each keeping its jobs in order). Returns the outcomes in
/// job order whatever the thread count.
fn dispatch<I: Send, O: Send>(
    threads: usize,
    jobs: &[Job],
    inputs: Vec<I>,
    run: impl Fn(usize, &Job, I) -> O + Sync,
) -> Vec<O> {
    let threads = threads.min(jobs.len());
    if threads <= 1 {
        return jobs
            .iter()
            .zip(inputs)
            .enumerate()
            .map(|(k, (j, i))| run(k, j, i))
            .collect();
    }
    let mut per_worker: Vec<Vec<(usize, &Job, I)>> = (0..threads).map(|_| Vec::new()).collect();
    for (k, (j, input)) in jobs.iter().zip(inputs).enumerate() {
        per_worker[k % threads].push((k, j, input));
    }
    let run = &run;
    let mut done: Vec<(usize, O)> = std::thread::scope(|scope| {
        let handles: Vec<_> = per_worker
            .into_iter()
            .map(|work| {
                scope.spawn(move || {
                    work.into_iter()
                        .map(|(k, j, input)| (k, run(k, j, input)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| match h.join() {
                Ok(v) => v,
                // INVARIANT: `guard` catches every panic in system code, so a worker never unwinds;
                // if engine code itself panics, re-raise it rather than lose it.
                Err(payload) => std::panic::resume_unwind(payload),
            })
            .collect()
    });
    done.sort_by_key(|(k, _)| *k);
    done.into_iter().map(|(_, o)| o).collect()
}

/// Every call the engine makes into system code during a run goes through here (SPR-006 DEF-1,
/// DEF-4): `step`, `fire`, `resample_timer` and `invariants`. A panic becomes a fatal
/// [`SimError::Panic`] naming the system, so the run stops with a crash artifact instead of
/// unwinding out of [`Engine::advance_until`].
fn guard<T>(
    system: &'static str,
    call: impl FnOnce() -> Result<T, SimError>,
) -> Result<T, SimError> {
    std::panic::catch_unwind(AssertUnwindSafe(call)).unwrap_or_else(|payload| {
        Err(SimError::Panic {
            system,
            message: panic_message(payload.as_ref()),
        })
    })
}

/// Delivers one notice to the host's observer under the same discipline as system code: a panic
/// becomes a fatal [`SimError::Observer`] naming the notice (SPR-009 DEF-1), never an unwind out of
/// [`Engine::advance_until`]. The text never names the worker, so it is the same at every thread
/// count; a job notice's [`JobRef`] already names the job by its index in the level.
fn observe(observer: &dyn Observer, notice: &Notice) -> Result<(), SimError> {
    std::panic::catch_unwind(AssertUnwindSafe(|| observer.notify(notice))).map_err(|payload| {
        SimError::Observer {
            notice: format!(
                "{:?} at {} in {}",
                notice.boundary,
                notice.at,
                notice.phase.map_or("no phase", Phase::name)
            ),
            message: panic_message(payload.as_ref()),
        }
    })
}

/// The text of a panic payload (`&str` or `String`; anything else is named as such).
fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic with a non-text payload".to_owned())
}

/// The first declared read, in id order, whose lag P0 cannot run exactly (BCK-002 A-28):
/// `previous_pulse`, or `start_of_day` by a system bound to an Hour pulse.
fn unsupported_lag(entries: &[Entry]) -> Option<SimError> {
    entries.iter().find_map(|e| {
        let hourly = e.bindings.iter().any(|b| b.pulse == Pulse::Hour);
        e.access
            .reads
            .iter()
            .find(|r| match r.lag {
                Lag::PreviousPulse => true,
                Lag::StartOfDay => hourly,
                Lag::SameInstant | Lag::Lagged => false,
            })
            .map(|r| SimError::Unsupported {
                system: e.id,
                column: r.column,
                lag: r.lag.name(),
            })
    })
}
