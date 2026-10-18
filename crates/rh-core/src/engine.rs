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
//! **The write set and the gates** (SPR-038 T-03; `05.1` §9). Taking a column with
//! `SystemView::write` sets its written flag, and the put-back marks a written column; the core's
//! own writes (compaction, the row insert) and every `WorldTables::insert` mark too. The
//! compaction, the ledger invariants, the systems' `invariants()` and each lagged column's
//! start-of-instant copy run only when a column they cover carries a mark newer than their last
//! run: exact, since each is a pure function of the tables. The agenda's, the event log's and the
//! queues' invariants run at every `Bookkeeping`. [`Gating::Off`], in a `reference-gating-off`
//! build only, runs them all at every chance: the reference of the differential test.
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

use rh_events::{Event, EventDraft, EventLog, EventRef, Origin};
use rh_ids::{BranchSalt, EntityRef, SystemId, SystemKey, TimelineId};

use crate::access::Lag;
use crate::agenda::{Agenda, Timer, TimerEntry};
use crate::command::KeyRule;
use crate::command::{CommandQueues, QueuedCommand};
use crate::crash::{CrashArtifact, DIAGNOSTIC_RECORDS, Provenance, Record};
use crate::ledger::{Guarded, Ledger, guarded_column};
use crate::observe::{Boundary, JobRef, JobWork, Notice, Observer, Pass};
use crate::phase::Phase;
use crate::register::{Entry, Registered};
use crate::schedule::Node;
use crate::state::{EngineState, RestoreError, check_epochs, check_inputs, check_timers};
use crate::system::{
    DropKind, DropReason, InvariantResult, JobInfo, Outbox, PulseBinding, ResampleCtx, SimError,
    StepCtx, Taken,
};
use crate::time::{Calendar, Pulse, SimTime};
use crate::world::{Column, ColumnId, OwnedColumn, SystemView, WorldTables};

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
    /// Each lagged column present in the world: its slot in the tables and in `snapshot`.
    lagged: Vec<(ColumnId, ColumnId)>,
    /// The start-of-instant values of the lagged columns, copied at each instant's start into
    /// their kept allocations (SPR-038 T-02). Never state.
    snapshot: WorldTables,
    /// Each system's written columns with their slots, in write order, resolved once in
    /// [`Engine::new`] (SPR-038 T-02).
    writes: Vec<Vec<WriteSlot>>,
    /// Each system's ledger guard buffer: what its last job kept of its ledger columns (SPR-038
    /// T-03). Never state.
    guarded: Vec<Vec<Guarded>>,
    /// The gating of the checks, the compaction and the lagged copies (SPR-038 T-03): `Exact`
    /// except in a `reference-gating-off` build that sets `Off`.
    gating: Gating,
    /// The write mark each gated pass last ran at (T-03). Never state.
    seen: Seen,
    /// What the gated passes ran (an observation, like [`DispatchStats`]; never state).
    checks: CheckStats,
    /// Each system's own `person.*` writes, in name order: what a row it inserts must name.
    own_person: Vec<Vec<&'static str>>,
    /// Each system's owned-column buffer, lent to its jobs and returned, so moving columns
    /// allocates nothing in steady state (SPR-038 T-02; V-PERF-11).
    owned: Vec<Vec<OwnedColumn>>,
    /// Buffers kept across instants (SPR-038 T-02): never state.
    scratch: Scratch,
    agenda: Agenda,
    events: EventLog,
    commands: CommandQueues,
    by_system_id: BTreeMap<SystemId, usize>,
    epochs: BTreeMap<EntityRef, u32>,
    inputs: BTreeMap<SimTime, Vec<MaterialInput>>,
    /// The number of material inputs accepted ([`Engine::submit_input`]), applied or still queued:
    /// the journal position (VER-SPR-007-TEST GAP-10). The host journals an input and then submits
    /// it, so this is the length of the journal's `seq`-order prefix the engine holds.
    accepted_inputs: u64,
    timeline: TimelineId,
    provenance: Provenance,
    diagnostics: VecDeque<Record>,
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
    /// Snapshots `emit` discarded because their source's owner is not registered, counted per
    /// `dropped{system, snapshot, absent_owner}` (envelope v0 §4 rule 4; SPR-020 T-03). Host
    /// telemetry, never state.
    dropped_snapshots: BTreeMap<crate::envelope::DroppedKey, u64>,
    /// The chronicle's admission rule (`05.2` §4.0; D38): every draft is admitted (P1.0) unless
    /// the host sets a predicate. The flag is carried on the event and never hashed, so plane A
    /// never branches on it. Until DATA-5's rule lands (P1.11) only tests set one (SPR-020 T-06).
    admission: Option<fn(&EventDraft) -> bool>,
    /// Every drop so far, `dropped{system, kind, reason}` (`05.1` §2 rule 2; REV-EVO-001 R11): a
    /// diagnostic, printed with the run's vitals, outside every plane hash and every keyframe
    /// (GOV-006 §3: observability never changes history). A restored engine counts from zero.
    dropped: BTreeMap<(&'static str, DropKind, DropReason), u64>,
    /// The inboxes of the level being dispatched: a buffer kept across levels so that taking them
    /// allocates nothing in steady state (V-PERF-11). Never state.
    inboxes: Vec<Vec<QueuedCommand>>,
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

/// One column a system writes, as the engine resolved it in [`Engine::new`] (SPR-038 T-02, T-03).
#[derive(Clone, Copy, Debug)]
struct WriteSlot {
    name: &'static str,
    id: ColumnId,
    /// A ledger column (`person.*` or `life_course.*` of a world with a ledger): the guard keeps
    /// its row count.
    ledger: bool,
    /// One whose pre-image the guard keeps when the step writes it.
    guarded: bool,
}

/// The write mark at which each gated pass last ran (SPR-038 T-03): a pass runs again only when
/// a column it covers carries a newer mark. Construction marks every column, so each runs at the
/// first chance.
#[derive(Clone, Debug, Default)]
struct Seen {
    compaction: u64,
    ledger: u64,
    invariants: u64,
    /// One per lagged column, in `Engine::lagged` order.
    lagged: Vec<u64>,
}

/// How the engine gates the core's checks, the ledger compaction and the lagged copies (SPR-038
/// T-03, S12).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gating {
    /// Each runs only after a column it covers was written, inserted or compacted since it last
    /// ran: exact, since each is a pure function of the tables (`05.1` §9).
    #[default]
    Exact,
    /// Each runs at every chance: the reference the differential test compares `Exact` with. Set
    /// only through [`Engine::set_gating`], in a build with the `reference-gating-off` feature,
    /// which `rh-cli run` refuses (BCK-002 A-52).
    Off,
}

/// What the gated passes ran since the engine was built or restored (SPR-038 T-03): an
/// observation of the engine, like [`DispatchStats`], never part of the state, a keyframe or a
/// hash. Each test case asserts its path through it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CheckStats {
    /// Compaction passes run (each scans `person.alive`).
    pub compactions: u64,
    /// Ledger rows the compactions removed.
    pub compacted_rows: u64,
    /// Runs of the core's ledger invariants.
    pub ledger_checks: u64,
    /// Runs of the systems' `invariants()` (each runs every system's).
    pub invariant_runs: u64,
    /// Start-of-instant copies of a lagged column.
    pub snapshot_copies: u64,
}

/// The columns one system owns, moved out of the tables for its step, in write order.
type Owned = Vec<OwnedColumn>;

/// The buffers of one instant, kept across instants so a quiet one allocates nothing (SPR-038
/// T-02; V-PERF-11). The multi-thread dispatch keeps its own (BCK-002 A-32).
#[derive(Default)]
struct Scratch {
    /// The phase's levels: the timer pass's, then the binding pass's. Only the first `used` are
    /// this phase's; the rest keep their allocations.
    levels: Vec<Vec<Job>>,
    used: usize,
    inputs: Vec<JobInput>,
    outcomes: Vec<JobOutcome>,
    outboxes: Vec<Outbox>,
    /// The dead rows' positions at compaction.
    dead: Vec<usize>,
}

impl Scratch {
    /// A cleared level at the end of the used ones.
    fn next_level(&mut self) -> &mut Vec<Job> {
        if self.used == self.levels.len() {
            self.levels.push(Vec::new());
        }
        let level = &mut self.levels[self.used];
        level.clear();
        self.used += 1;
        level
    }

    /// Drops the last used level if it holds no job.
    fn drop_empty_level(&mut self) {
        if self.used > 0 && self.levels[self.used - 1].is_empty() {
            self.used -= 1;
        }
    }
}

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
        let mut by_system_id = BTreeMap::new();
        for (i, e) in registered.entries.iter().enumerate() {
            for w in &e.access.writes {
                if tables.get(w.column).is_none() {
                    tables.insert(w.column, Column::new());
                }
            }
            bound.extend(e.bindings.iter().map(|b| b.pulse));
            by_system_id.insert(e.system_id, i);
        }
        let (ledger, ledger_error) = match Ledger::of(&tables) {
            Ok(ledger) => (ledger, None),
            Err(e) => (None, Some(e)),
        };
        let (lagged, snapshot) = lagged_slots(&registered, &tables);
        let (writes, own_person) = write_slots(&registered, &tables, ledger.as_ref());
        let poisoned = unsupported_lag(&registered.entries).or(ledger_error);
        let owned = registered.entries.iter().map(|_| Vec::new()).collect();
        let guarded = registered.entries.iter().map(|_| Vec::new()).collect();
        let seen = Seen {
            lagged: vec![0; lagged.len()],
            ..Seen::default()
        };
        Self {
            calendar,
            world_seed,
            branch_salt,
            registered,
            tables,
            cursor: Some(start),
            threads: 1,
            bound_pulses: bound.into_iter().collect(),
            lagged,
            snapshot,
            writes,
            guarded,
            gating: Gating::Exact,
            seen,
            checks: CheckStats::default(),
            own_person,
            owned,
            scratch: Scratch::default(),
            agenda: Agenda::new(),
            events: EventLog::new(),
            commands: CommandQueues::new(),
            by_system_id,
            epochs: BTreeMap::new(),
            inputs: BTreeMap::new(),
            accepted_inputs: 0,
            timeline: TimelineId::from_raw(0),
            provenance: Provenance::default(),
            diagnostics: VecDeque::new(),
            dropped_snapshots: BTreeMap::new(),
            admission: None,
            at: (start, Phase::Inputs),
            crash: None,
            journal_head: None,
            last_keyframe: None,
            poisoned,
            dispatch: DispatchStats::default(),
            observer: None,
            ledger,
            taken: Vec::new(),
            dropped: BTreeMap::new(),
            inboxes: Vec::new(),
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
                if engine
                    .registered
                    .commands
                    .get(c.command.kind)
                    .map(|d| d.owner)
                    != Some(owner)
                {
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

    /// The drops counted so far, `dropped{system, kind, reason}` (`05.1` §2 rule 2): the core's
    /// dead-person drop at take (under the owner, `dead_key`), the owners' own drops (`dead`,
    /// `ended`) and the discarded sends to absent owners (under the sender, `absent_owner`). A
    /// diagnostic: no hash or keyframe reads it.
    #[must_use]
    pub fn dropped(&self) -> &BTreeMap<(&'static str, DropKind, DropReason), u64> {
        &self.dropped
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

    /// The `dropped{system, snapshot, absent_owner}` counter: snapshots discarded because their
    /// source's owner is not registered (envelope v0 §4 rule 4).
    #[must_use]
    pub const fn dropped_snapshots(&self) -> &BTreeMap<crate::envelope::DroppedKey, u64> {
        &self.dropped_snapshots
    }

    /// Sets the chronicle's admission predicate (`None`: admit every draft, the P1.0 rule). The
    /// flag only decides which events the Chronicle Writer records; it is in no hash.
    pub fn set_admission(&mut self, admission: Option<fn(&EventDraft) -> bool>) {
        self.admission = admission;
    }

    /// What the gated passes ran so far (SPR-038 T-03).
    #[must_use]
    pub const fn check_stats(&self) -> CheckStats {
        self.checks
    }

    /// Sets the gating (SPR-038 S12): `Gating::Off` runs every check, compaction and lagged copy
    /// at every chance, the reference of the differential test. Only a `reference-gating-off`
    /// build has it, and `rh-cli run` refuses such a build (BCK-002 A-52).
    #[cfg(feature = "reference-gating-off")]
    pub fn set_gating(&mut self, gating: Gating) {
        self.gating = gating;
    }

    /// Whether a pass that last ran at mark `seen` must run again for a column marked `mark`.
    fn due(&self, mark: u64, seen: u64) -> bool {
        self.gating == Gating::Off || mark > seen
    }

    /// The last diagnostic records, oldest first.
    #[must_use]
    pub fn diagnostics(&self) -> Vec<String> {
        self.diagnostics.iter().map(ToString::to_string).collect()
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
                self.record(Record::Text(format!("{t} FATAL in {}: {e}", self.at.1)));
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

    fn record(&mut self, line: Record) {
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
            | SimError::UndeclaredRole { system, .. }
            | SimError::DuplicateRole { system, .. }
            | SimError::CastTooLarge { system, .. }
            | SimError::UndeclaredSnapshot { system, .. }
            | SimError::SnapshotNotOfTheCast { system, .. }
            | SimError::SnapshotOutOfDomain { system, .. }
            | SimError::DuplicateSnapshot { system, .. }
            | SimError::MissingSnapshot { system, .. }
            | SimError::UndeclaredMotive { system, .. }
            | SimError::EventSlot { system, .. }
            | SimError::Payload { system, .. }
            | SimError::EntityKind { system, .. }
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
        let mut fired = [false; 5];
        for (on, p) in fired.iter_mut().zip(Pulse::ALL) {
            *on = self.bound_pulses.contains(&p) && self.calendar.is_boundary(p, t);
        }
        // A lagged column is copied only when it was written since its last copy (T-03); the
        // planted engine of `mutant-stale-lagged-snapshot` never copies it again.
        for (k, &(from, to)) in self.lagged.iter().enumerate() {
            let mark = self.tables.mark_at(from);
            let stale = cfg!(feature = "mutant-stale-lagged-snapshot")
                && self.gating == Gating::Exact
                && self.seen.lagged[k] > 0;
            if !self.due(mark, self.seen.lagged[k]) || stale {
                continue;
            }
            if let Some(rows) = self.tables.get_at(from) {
                self.snapshot.copy_at(to, rows);
                self.checks.snapshot_copies += 1;
            }
            self.seen.lagged[k] = self.tables.stamp();
        }
        let mut tally = Tally::default();
        // The snapshot is lent to the levels while the engine runs them; it comes back whatever
        // the outcome.
        let snapshot = std::mem::take(&mut self.snapshot);
        let result = self.run_phases(t, fired, &snapshot, &mut tally);
        self.snapshot = snapshot;
        result?;
        self.record(Record::Instant {
            t,
            pulses: fired,
            inputs: tally.inputs,
            timers: tally.timers,
            commands: tally.commands,
            events: tally.events,
            agenda: self.agenda.len(),
        });
        self.notify(t, None, Boundary::InstantEnd)?;
        Ok(())
    }

    /// The phases of one instant, in `05.1` §4.2 order.
    fn run_phases(
        &mut self,
        t: SimTime,
        fired: [bool; 5],
        snapshot: &WorldTables,
        tally: &mut Tally,
    ) -> Result<(), SimError> {
        for phase in Phase::ALL {
            self.at = (t, phase);
            self.notify(t, Some(phase), Boundary::PhaseStart)?;
            if phase == Phase::Inputs {
                tally.inputs = self.apply_inputs(t)?;
            }
            // SPR-011 ruling 5: the dead of this instant leave every person.* column before any
            // Bookkeeping system, invariant or hash sees the tables.
            // Gated (T-03): only after person.alive was written or a row inserted since the last
            // compaction; without either the table has no dead row and compaction changes nothing.
            if phase == Phase::Bookkeeping
                && let Some(ledger) = &self.ledger
                && (self.gating == Gating::Off
                    || self.tables.mark_at(ledger.alive_slot()) > self.seen.compaction)
            {
                let removed = ledger.compact(&mut self.tables, &mut self.scratch.dead)?;
                self.checks.compactions += 1;
                self.checks.compacted_rows += u64::try_from(removed).unwrap_or(u64::MAX);
                self.seen.compaction = self.tables.stamp();
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
            // The plan is borrowed, and the levels go into kept buffers (SPR-038 T-02).
            let plan = self.registered.schedule().phase(phase);
            let scratch = &mut self.scratch;
            scratch.used = 0;
            // Timer pass: each owner once, at the level of its first binding in the phase, with
            // its timers in key order.
            for level in &plan.levels {
                let jobs = scratch.next_level();
                for n in level {
                    if let Some(timers) = due.remove(&n.system) {
                        jobs.push(Job {
                            index: n.system,
                            work: Work::Timers(timers),
                        });
                    }
                }
                scratch.drop_empty_level();
            }
            if let Some((&i, timers)) = due.iter().next() {
                return Err(SimError::Timer {
                    system: self.registered.entries[i].id,
                    message: format!(
                        "{} is due in phase {phase}, where the system is not bound",
                        timers[0].id()
                    ),
                });
            }
            let timer_levels = scratch.used;
            // Binding pass: a level is split wherever a system's second binding appears.
            binding_pass(scratch, &plan.levels, fired, phase);
            let mut levels = std::mem::take(&mut self.scratch.levels);
            let used = self.scratch.used;
            let passes = [
                (Pass::Timers, &levels[..timer_levels]),
                (Pass::Bindings, &levels[timer_levels..used]),
            ];
            let result = self.run_passes((t, phase), snapshot, passes, tally);
            // The jobs' timers are spent; the levels keep their allocations.
            levels.iter_mut().take(used).for_each(Vec::clear);
            self.scratch.levels = levels;
            result?;
            if phase == Phase::Bookkeeping {
                self.check_invariants(t)?;
            }
            self.notify(t, Some(phase), Boundary::PhaseEnd)?;
        }
        Ok(())
    }

    /// Runs the passes of one phase level by level, each level between its observer notices.
    fn run_passes(
        &mut self,
        (t, phase): (SimTime, Phase),
        snapshot: &WorldTables,
        passes: [(Pass, &[Vec<Job>]); 2],
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
            // Keys are unique (the seq), so the key order is the whole order.
            touched.sort_unstable_by_key(|e| e.key);
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
        // BCK-002 A-50: the timer's entity is of the kind its purpose's key declares.
        if let Some((&purpose, &declared)) = e
            .purpose_kinds
            .iter()
            .find(|(p, _)| rh_ids::PurposeId::of(p) == entry.timer.purpose)
            && entry.key.entity.kind() != declared
        {
            return Err(SimError::EntityKind {
                system: e.id,
                purpose,
                declared: declared.name(),
                drawn: entry.key.entity.kind().name(),
            });
        }
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
    /// The ledger's and the systems' run only after a column they cover was written, inserted or
    /// compacted since they last ran (SPR-038 T-03; `05.1` §9): each is a pure function of the
    /// tables, so a table unchanged since a passing run passes again. The agenda's, the event
    /// log's and the queues' run at every `Bookkeeping`.
    fn check_invariants(&mut self, t: SimTime) -> Result<(), SimError> {
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
        let stamp = self.tables.stamp();
        if let Some(ledger) = &self.ledger {
            let mark = ledger
                .slots()
                .iter()
                .map(|&id| self.tables.mark_at(id))
                .max()
                .unwrap_or(0);
            if self.due(mark, self.seen.ledger) {
                ledger.check(&self.tables)?;
                self.checks.ledger_checks += 1;
                self.seen.ledger = stamp;
            }
        }
        self.check_queue_age(t)?;
        if self.due(stamp, self.seen.invariants) {
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
            self.checks.invariant_runs += 1;
            self.seen.invariants = stamp;
        }
        Ok(())
    }

    /// `queue-no-stale-command` (`05.1` §2 rule 2, queue age; §9; BCK-002 A-31): no queued
    /// command is as old as the period of its draining binding's pulse, `t - tick <
    /// period(pulse(drained_by))`, the period being the calendar's length of that pulse's period
    /// containing `tick` (`Day`: 1,440 minutes).
    fn check_queue_age(&self, t: SimTime) -> Result<(), SimError> {
        for (owner, c) in self.commands.iter() {
            let Some(decl) = self.registered.commands.get(c.command.kind) else {
                continue;
            };
            let pulse = decl.drained_by.pulse;
            let period = self
                .calendar
                .floor(pulse, c.tick)
                .and_then(|start| Some((start, self.calendar.next_boundary(pulse, start)?)))
                .map_or(i64::MAX, |(start, end)| end.minutes() - start.minutes());
            let age = t.minutes() - c.tick.minutes();
            if age >= period {
                return Err(SimError::Invariant {
                    system: CORE,
                    name: "queue-no-stale-command",
                    detail: format!(
                        "{owner}'s {} command seq {} from {} has waited {age} minutes at {t}, not less than the {period}-minute period of its draining binding {}/{}",
                        c.command.kind,
                        c.seq,
                        c.sender,
                        pulse.name(),
                        decl.drained_by.phase.name()
                    ),
                });
            }
        }
        Ok(())
    }

    /// Each job's inbox, taken before any column moves: a binding gets the queued commands of
    /// every kind its system's spec says it drains, a timer pass none (`05.1` §2 rule 2). In a
    /// world with a ledger, a `key = "living"` command whose key person has no ledger row or has
    /// `person.alive = 0` in the tables as they stand at the level's start is dropped here, never
    /// delivered, and counted under `dropped{owner, kind, dead_key}` (BCK-002 A-30).
    ///
    /// The inboxes go into `out` (cleared first), a buffer the engine keeps across levels, so a
    /// level whose jobs have no commands allocates nothing (V-PERF-11).
    fn fill_inboxes(&mut self, jobs: &[Job], out: &mut Vec<Vec<QueuedCommand>>) {
        out.clear();
        for job in jobs {
            let Work::Step(binding) = &job.work else {
                out.push(Vec::new());
                continue;
            };
            let owner = self.registered.entries[job.index].id;
            let decls = &self.registered.commands;
            let mut inbox = self.commands.take_kinds(owner, |kind| {
                decls.get(kind).is_some_and(|d| {
                    d.owner == owner
                        && (d.drained_by == *binding || cfg!(feature = "mutant-deliver-every-job"))
                })
            });
            if !inbox.is_empty() && self.ledger.is_some() {
                let ids = self.tables.get(crate::ledger::PERSON_ID);
                let alive = self.tables.get(crate::ledger::PERSON_ALIVE);
                let living = |person: u64| -> bool {
                    ids.and_then(|ids| {
                        ids.binary_search_by_key(&person, |v| v.cast_unsigned())
                            .ok()
                    })
                    .and_then(|row| alive.and_then(|a| a.get(row)))
                    .is_some_and(|&a| a != 0)
                };
                let dropped = &mut self.dropped;
                inbox.retain(|c| {
                    let rule = decls.get(c.command.kind).map(|d| d.key);
                    let drop = rule == Some(KeyRule::Living) && !living(c.command.person.to_raw());
                    if drop {
                        *dropped
                            .entry((
                                owner,
                                DropKind::Command(c.command.kind),
                                DropReason::DeadKey,
                            ))
                            .or_default() += 1;
                    }
                    !drop
                });
            }
            out.push(inbox);
        }
    }

    /// What each job of a level takes to its thread: its owned columns (moved out of the tables
    /// by slot, into its system's kept buffer), its inbox and its system's drawn pairs, into
    /// `out` (cleared first). Every inbox is taken, and every dead-person drop decided, before any
    /// column moves.
    fn job_inputs(
        &mut self,
        jobs: &[Job],
        inboxes: &mut Vec<Vec<QueuedCommand>>,
        out: &mut Vec<JobInput>,
    ) {
        out.clear();
        for (job, inbox) in jobs.iter().zip(inboxes.drain(..)) {
            let mut owned = std::mem::take(&mut self.owned[job.index]);
            owned.clear();
            for w in &self.writes[job.index] {
                if let Some(rows) = self.tables.take_at(w.id) {
                    owned.push(OwnedColumn {
                        name: w.name,
                        id: w.id,
                        rows_before: rows.len(),
                        rows,
                        written: false,
                        guarded: w.guarded,
                        pre: None,
                    });
                }
            }
            let taken = std::mem::take(&mut self.taken[job.index]);
            out.push((owned, inbox, taken));
        }
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
        let mut inboxes = std::mem::take(&mut self.inboxes);
        self.fill_inboxes(jobs, &mut inboxes);
        let mut inputs = std::mem::take(&mut self.scratch.inputs);
        self.job_inputs(jobs, &mut inboxes, &mut inputs);
        self.inboxes = inboxes;
        let workers = self.threads.min(jobs.len()).max(1);
        let mut outcomes = std::mem::take(&mut self.scratch.outcomes);
        {
            let observer = self.observer.as_deref();
            let tables = &self.tables;
            let agenda = &self.agenda;
            let entries = &self.registered.entries;
            let commands = &self.registered.commands;
            let epochs = &self.epochs;
            let (world_seed, branch_salt) = (self.world_seed, self.branch_salt);
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
                let mut view = SystemView::new(e.id, &e.access, cols, tables, snapshot)
                    .with_dropped(&e.dropped_reads);
                let info = JobInfo {
                    system: e.id,
                    system_id: e.system_id,
                    key: SystemKey::new(world_seed, branch_salt, e.system_id, 0),
                    world_seed,
                    branch_salt,
                    epochs,
                    purposes: &e.purposes,
                    access: &e.access,
                    bindings: &e.bindings,
                    agenda,
                    envelope: &e.envelope,
                    commands,
                    purpose_kinds: &e.purpose_kinds,
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
                let mut owned = view.into_owned();
                // The planted engine of `mutant-late-pre-image` takes the pre-image after the step,
                // so a changed entry field goes unseen.
                if cfg!(feature = "mutant-late-pre-image") {
                    for o in owned.iter_mut().filter(|o| o.written && o.guarded) {
                        o.pre = Some(o.rows.clone());
                    }
                }
                JobOutcome {
                    owned,
                    result,
                    untaken,
                    out,
                    taken,
                }
            };
            dispatch(self.threads, jobs, &mut inputs, &mut outcomes, run);
        }
        self.scratch.inputs = inputs;
        let mut outboxes = std::mem::take(&mut self.scratch.outboxes);
        let first_error = self.put_back(jobs, &mut outcomes, &mut outboxes);
        self.scratch.outcomes = outcomes;
        let result = self.after_level((t, phase), jobs, &mut outboxes, tally, first_error);
        outboxes.clear();
        self.scratch.outboxes = outboxes;
        result
    }

    /// Returns every job's owned columns to their slots, its untaken commands to its queue and
    /// its drawn pairs to its system, and its outbox to `outboxes` (cleared first), in job order.
    /// Returns the first job error in job order.
    fn put_back(
        &mut self,
        jobs: &[Job],
        outcomes: &mut Vec<JobOutcome>,
        outboxes: &mut Vec<Outbox>,
    ) -> Option<SimError> {
        let mut first_error = None;
        outboxes.clear();
        for (job, o) in jobs.iter().zip(outcomes.drain(..)) {
            let mut owned = o.owned;
            let guarded = &mut self.guarded[job.index];
            guarded.clear();
            for c in owned.drain(..) {
                if self.writes[job.index]
                    .iter()
                    .any(|w| w.id == c.id && w.ledger)
                {
                    guarded.push(Guarded {
                        name: c.name,
                        rows_before: c.rows_before,
                        rows_after: c.rows.len(),
                        written: c.written,
                        pre: c.pre,
                    });
                }
                // The write set (T-03): a written column is marked at its put-back. The planted
                // engine of `mutant-unmarked-put-back` ignores person.alive's flag.
                let unmarked = cfg!(feature = "mutant-unmarked-put-back")
                    && c.name == crate::ledger::PERSON_ALIVE;
                self.tables.put_at(c.id, c.rows, c.written && !unmarked);
            }
            self.owned[job.index] = owned;
            self.commands
                .restore(self.registered.entries[job.index].id, o.untaken);
            self.taken[job.index] = o.taken;
            if first_error.is_none() {
                first_error = o.result.err();
            }
            outboxes.push(o.out);
        }
        first_error
    }

    /// A level's end, once every owned column is back: the first job error, else the ledger
    /// guards, the merge notice and the merges in job order.
    fn after_level(
        &mut self,
        (t, phase): (SimTime, Phase),
        jobs: &[Job],
        outboxes: &mut Vec<Outbox>,
        tally: &mut Tally,
        first_error: Option<SimError>,
    ) -> Result<(), SimError> {
        if let Some(e) = first_error {
            return Err(e);
        }
        // A job that owns no ledger column, or any job in a world without a ledger, kept nothing
        // and is not checked (DEF-4).
        for job in jobs {
            let guarded = &self.guarded[job.index];
            if !guarded.is_empty() {
                let system = self.registered.entries[job.index].id;
                Ledger::check_step(system, phase, guarded, &self.tables)?;
            }
        }
        // After every owned column is back in the tables, so a failing observer loses none (DEF-1).
        self.notify(t, Some(phase), Boundary::LevelMerge)?;
        for (job, out) in jobs.iter().zip(outboxes.drain(..)) {
            self.merge(t, phase, job.index, out, tally)?;
        }
        Ok(())
    }

    /// Every column the level's jobs own exists (materialised in `new`). Checked for all before any
    /// is moved, so a missing one fails the step without disturbing the tables (SPR-005 G-c).
    fn check_owned_columns(&self, jobs: &[Job]) -> Result<(), SimError> {
        for job in jobs {
            let e = &self.registered.entries[job.index];
            let present = |column: &str| {
                self.writes[job.index]
                    .iter()
                    .find(|w| w.name == column)
                    .is_some_and(|w| self.tables.get_at(w.id).is_some())
            };
            if let Some(w) = e.access.writes.iter().find(|w| !present(w.column)) {
                return Err(SimError::MissingColumn {
                    system: e.id,
                    column: w.column.to_owned(),
                });
            }
        }
        Ok(())
    }

    /// Commits one system's buffers, in the order of `05.1` §2 rule 4 (EVO-001): cancellations,
    /// events (which get their `EventId` here), timers (which get their `seq`, and their `cause`
    /// resolved to the id of the draft it names), lifecycle commands (queued for their owners,
    /// every event-typed slot resolved), then the ledger rows the step inserts (rule 2) and the
    /// drops it counted. The agenda's `seq` and the `EventId` are separate counters, so taking
    /// events before timers changes no id (SPR-006 ruling 1 merged timers first).
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
        let mut ids = Vec::with_capacity(out.events.len());
        for mut draft in out.events {
            let event_error = |err: rh_events::EventError| SimError::Event {
                system: e.id,
                message: err.to_string(),
            };
            // Resolve every draft-local cause to the id this merge gave it, in place, then
            // coalesce to one edge per cause (envelope §5). INVARIANT: `StepCtx::emit` admits
            // only references to earlier drafts, so every `Draft` resolves; one that does not
            // (or a `None`) stays and `append` refuses it.
            for edge in &mut draft.causes {
                if let EventRef::Draft(r) = edge.cause
                    && let Some(&id) = ids.get(r.index())
                {
                    edge.cause = EventRef::Known(id);
                }
            }
            let motives = &self.registered.motives;
            rh_events::coalesce_draft(&mut draft, |m| motives.iter().position(|x| x == m))
                .map_err(event_error)?;
            let admitted = self.admission.is_none_or(|admit| admit(&draft));
            let id = self
                .events
                .append_admitted(
                    t.minutes(),
                    phase.index(),
                    e.id,
                    Origin::Sim,
                    admitted,
                    draft,
                )
                .map_err(event_error)?;
            ids.push(id);
            tally.events += 1;
        }
        // INVARIANT: `StepCtx` admits only references to drafts this step already emitted.
        let resolve = |r: rh_events::EventRef| r.resolved(|d| ids.get(d.index()).copied());
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
                    cause: resolve(spec.cause),
                },
            );
        }
        for (snapshot, owner) in out.dropped {
            *self.dropped_snapshots.entry((e.id, snapshot, owner)).or_default() += 1;
        }
        for mut command in out.commands {
            let Some(decl) = self.registered.commands.get(command.kind) else {
                return Err(SimError::UndeclaredCommand {
                    system: e.id,
                    command: command.kind,
                });
            };
            let owner = decl.owner;
            // R9 (05.1 §2 rule 2; the review's K3): an outcome-carrying command is applied in the
            // instant it is submitted, so its draining binding must run later in this instant.
            let drain = decl.drained_by;
            if decl.outcome
                && !(self.calendar.is_boundary(drain.pulse, t)
                    && drain.phase.index() >= phase.index())
            {
                return Err(SimError::Timer {
                    system: e.id,
                    message: format!(
                        "submitted outcome-carrying command {} at ({t}, {phase}), but its drain \
                         {}/{} does not run later in this instant: an outcome is submitted only \
                         where its draining binding fires after it (05.1 §2 rule 2; R9)",
                        command.kind, drain.pulse, drain.phase
                    ),
                });
            }
            let draft = command.cause.draft();
            command.cause = resolve(command.cause);
            for arg in &mut command.args {
                if let crate::command::Arg::Event(r) = arg {
                    *r = resolve(*r);
                }
            }
            self.commands.push(owner, t, phase, e.id, draft, command);
            tally.commands += 1;
        }
        let system = e.id;
        if !out.inserts.is_empty() {
            self.insert_rows(system, out.inserts)?;
        }
        for (kind, reason) in out.drops {
            *self.dropped.entry((system, kind, reason)).or_default() += 1;
        }
        Ok(())
    }

    /// Inserts ledger rows handed over by `system`'s step (`05.1` §2 rule 2, "Ledger rows enter by
    /// insert"; SPR-014 T-05): each at its sorted position in every `person.*` column, the
    /// inserter's columns with its values, every other owner's with its declared default, else 0.
    fn insert_rows(
        &mut self,
        system: &'static str,
        rows: Vec<Vec<(&'static str, i64)>>,
    ) -> Result<(), SimError> {
        let refuse = |message: String| SimError::Ledger { system, message };
        let Some(ledger) = &self.ledger else {
            return Err(refuse(
                "inserted a ledger row in a world without a ledger (no person.id)".to_owned(),
            ));
        };
        let index = self
            .registered
            .entries
            .iter()
            .position(|e| e.id == system)
            .ok_or_else(|| refuse("is not registered".to_owned()))?;
        let own = &self.own_person[index];
        for row in rows {
            let given: BTreeSet<&str> = row.iter().map(|(c, _)| *c).collect();
            if given.len() != row.len() || !given.iter().eq(own.iter()) {
                return Err(refuse(format!(
                    "inserted a row of {:?}, not each of its own person.* columns {:?} once",
                    row.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
                    own
                )));
            }
            let id = row
                .iter()
                .find(|(c, _)| *c == crate::ledger::PERSON_ID)
                .map(|(_, v)| *v)
                .ok_or_else(|| refuse("inserted a row without person.id".to_owned()))?;
            let at = match self
                .tables
                .get(crate::ledger::PERSON_ID)
                .map(|ids| ids.binary_search_by_key(&id.cast_unsigned(), |v| v.cast_unsigned()))
            {
                Some(Ok(_)) => {
                    return Err(refuse(format!(
                        "inserted person {}, whose id the ledger already holds",
                        id.cast_unsigned()
                    )));
                }
                Some(Err(at)) => at,
                None => 0,
            };
            for (column, id) in ledger.person_slots() {
                let value = row
                    .iter()
                    .find(|(c, _)| *c == column)
                    .map(|(_, v)| *v)
                    .or_else(|| self.registered.defaults.get(column).copied())
                    .unwrap_or(0);
                if let Some(mut rows) = self.tables.take_at(id) {
                    rows.insert(at.min(rows.len()), value);
                    self.tables.put_at(id, rows, true);
                }
            }
        }
        Ok(())
    }
}

/// Each lagged column present in the world, with its slot in the tables and in a snapshot of
/// them that holds their current copies (SPR-038 T-02).
fn lagged_slots(
    registered: &Registered,
    tables: &WorldTables,
) -> (Vec<(ColumnId, ColumnId)>, WorldTables) {
    let mut snapshot = WorldTables::new();
    let present: Vec<&'static str> = registered
        .lagged_reads()
        .into_iter()
        .filter(|c| tables.get(c).is_some())
        .collect();
    for &c in &present {
        snapshot.insert(c, tables.get(c).cloned().unwrap_or_default());
    }
    let lagged = present
        .iter()
        .filter_map(|c| Some((tables.id(c)?, snapshot.id(c)?)))
        .collect();
    (lagged, snapshot)
}

/// Each system's written columns with their slots (SPR-038 T-02, T-03), and its own `person.*`
/// writes in name order.
fn write_slots(
    registered: &Registered,
    tables: &WorldTables,
    ledger: Option<&Ledger>,
) -> (Vec<Vec<WriteSlot>>, Vec<Vec<&'static str>>) {
    let writes = registered
        .entries
        .iter()
        .map(|e| {
            e.access
                .writes
                .iter()
                .filter_map(|w| {
                    let is_ledger =
                        ledger.is_some_and(|l| l.is_person(w.column) || l.is_life_course(w.column));
                    Some(WriteSlot {
                        name: w.column,
                        id: tables.id(w.column)?,
                        ledger: is_ledger,
                        guarded: is_ledger && guarded_column(w.column),
                    })
                })
                .collect()
        })
        .collect();
    let own_person = registered
        .entries
        .iter()
        .map(|e| {
            let own: BTreeSet<&'static str> = e
                .access
                .writes
                .iter()
                .map(|w| w.column)
                .filter(|c| ledger.is_some_and(|l| l.is_person(c)))
                .collect();
            own.into_iter().collect()
        })
        .collect();
    (writes, own_person)
}

/// The binding pass of one phase: the bindings that fire at this instant, level by level, a level
/// split wherever one system's second binding appears, so one system's bindings never run at once
/// (they share its columns); each split is a level for the merge. The split depends on the
/// schedule only, never on the thread count (SPR-005 DEF-7). The levels go into `scratch`'s kept
/// buffers, after the timer pass's (SPR-038 T-02).
fn binding_pass(scratch: &mut Scratch, levels: &[Vec<Node>], fired: [bool; 5], phase: Phase) {
    for level in levels {
        scratch.next_level();
        for n in level.iter().filter(|n| fired[n.pulse as usize]) {
            let repeats = scratch.levels[scratch.used - 1]
                .iter()
                .any(|j| j.index == n.system);
            if repeats {
                scratch.drop_empty_level();
                scratch.next_level();
            }
            scratch.levels[scratch.used - 1].push(Job {
                index: n.system,
                work: Work::Step(PulseBinding {
                    pulse: n.pulse,
                    phase,
                }),
            });
        }
        scratch.drop_empty_level();
    }
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

/// Runs `run` on every job with its input (drained from `inputs`), into `out` (cleared first):
/// in order on one thread, allocating nothing (SPR-038 T-02), or spread over `threads` workers (job
/// k on worker k % threads, each keeping its jobs in order; its buffers are BCK-002 A-32's). The
/// outcomes are in job order whatever the thread count.
fn dispatch<I: Send, O: Send>(
    threads: usize,
    jobs: &[Job],
    inputs: &mut Vec<I>,
    out: &mut Vec<O>,
    run: impl Fn(usize, &Job, I) -> O + Sync,
) {
    out.clear();
    let threads = threads.min(jobs.len());
    if threads <= 1 {
        out.extend(
            jobs.iter()
                .zip(inputs.drain(..))
                .enumerate()
                .map(|(k, (j, i))| run(k, j, i)),
        );
        return;
    }
    let mut per_worker: Vec<Vec<(usize, &Job, I)>> = (0..threads).map(|_| Vec::new()).collect();
    for (k, (j, input)) in jobs.iter().zip(inputs.drain(..)).enumerate() {
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
    out.extend(done.into_iter().map(|(_, o)| o));
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
