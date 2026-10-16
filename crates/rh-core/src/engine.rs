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
//! 5. in `Bookkeeping`, after the phase's systems, runs the invariants: the core's own, then each
//!    system's in id order. The first violation is fatal.
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

use rh_events::{Event, EventLog};
use rh_ids::{BranchSalt, EntityRef, SystemId, SystemKey, TimelineId};

use crate::agenda::{Agenda, Timer, TimerEntry};
use crate::command::{CommandQueues, QueuedCommand};
use crate::crash::{CrashArtifact, DIAGNOSTIC_RECORDS, Provenance};
use crate::phase::Phase;
use crate::register::{Entry, Registered};
use crate::system::{
    InvariantResult, JobInfo, Outbox, PulseBinding, ResampleCtx, SimError, StepCtx,
};
use crate::time::{Calendar, Pulse, SimTime};
use crate::world::{Column, SystemView, WorldTables};

/// The name the core's own invariants and errors carry.
pub const CORE: &str = "rh-core";

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
    /// The run applied material inputs before it failed (the artifact's journal position is
    /// non-zero), and replay needs that journal, which P0 cannot supply (SPR-006 DEF-2; SPR-007
    /// wires the journal in).
    JournalRequired {
        /// The artifact's journal position: the number of material inputs applied.
        position: u64,
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
            Self::JournalRequired { position } => write!(
                f,
                "the run applied {position} material inputs before it failed; replaying it needs the input journal (SPR-007)"
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
    /// The number of material inputs applied: the journal position a crash artifact records.
    applied_inputs: u64,
    timeline: TimelineId,
    provenance: Provenance,
    diagnostics: VecDeque<String>,
    at: (SimTime, Phase),
    crash: Option<CrashArtifact>,
    /// The error that stopped the run, if any. A poisoned engine never runs again (SPR-005 G-d).
    poisoned: Option<SimError>,
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

/// The result of one job.
struct JobOutcome {
    owned: Owned,
    result: Result<(), SimError>,
    untaken: Vec<QueuedCommand>,
    out: Outbox,
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
            applied_inputs: 0,
            timeline: TimelineId::from_raw(0),
            provenance: Provenance::default(),
            diagnostics: VecDeque::new(),
            at: (start, Phase::Inputs),
            crash: None,
            poisoned: None,
        }
    }

    /// Sets the number of worker threads (at least 1). Changes speed only, never the result.
    pub fn set_threads(&mut self, threads: usize) {
        self.threads = threads.max(1);
    }

    /// Sets the build, Registry version and manifest hash a crash artifact records.
    pub fn set_provenance(&mut self, provenance: Provenance) {
        self.provenance = provenance;
    }

    /// Sets the timeline id (0, the canonical timeline, by default).
    pub fn set_timeline(&mut self, timeline: TimelineId) {
        self.timeline = timeline;
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
    /// [`ReplayError::JournalRequired`] if the run had applied material inputs (a non-zero journal
    /// position), which cannot be replayed without the input journal (SPR-007).
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
        if artifact.journal_position != 0 {
            return Err(ReplayError::JournalRequired {
                position: artifact.journal_position,
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

    /// Every system's invariants over the current tables, in id order.
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
            | SimError::Timer { system, .. }
            | SimError::UndeclaredCommand { system, .. }
            | SimError::UndeclaredEvent { system, .. }
            | SimError::Event { system, .. }
            | SimError::Panic { system, .. }
            | SimError::System { system, .. } => ((*system).to_owned(), None),
        };
        CrashArtifact {
            timeline: self.timeline.to_raw(),
            build: self.provenance.build.clone(),
            registry_version: self.provenance.registry_version.clone(),
            manifest_hash: self.provenance.manifest_hash.clone(),
            seed: self.world_seed,
            branch_salt: self.branch_salt.to_raw(),
            journal_position: self.applied_inputs,
            last_keyframe: None,
            tick: self.at.0.minutes(),
            phase: self.at.1.name().to_owned(),
            system,
            invariant,
            error: error.to_string(),
            diagnostics: self.diagnostics(),
        }
    }

    fn run_instant(&mut self, t: SimTime) -> Result<(), SimError> {
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
            if phase == Phase::Inputs {
                tally.inputs = self.apply_inputs(t)?;
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
            let mut passes: Vec<Vec<Vec<Job>>> = Vec::new();
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
            passes.push(timer_pass);
            // Binding pass: a level is split wherever a system's second binding appears, so one
            // system's bindings never run at once (they share its columns); each split is a level
            // for the merge. The split depends on the schedule only, never on the thread count.
            let mut binding_pass: Vec<Vec<Job>> = Vec::new();
            for level in &plan.levels {
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
                binding_pass.extend(groups.into_iter().filter(|g| !g.is_empty()));
            }
            passes.push(binding_pass);
            for jobs in passes.into_iter().flatten() {
                self.run_level(t, phase, &snapshot, &jobs, &mut tally)?;
            }
            if phase == Phase::Bookkeeping {
                self.check_invariants(t)?;
            }
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
        Ok(())
    }

    /// Applies the material inputs queued for `t` (ADR-0025 item 3). Returns how many.
    fn apply_inputs(&mut self, t: SimTime) -> Result<usize, SimError> {
        let inputs = self.inputs.remove(&t).unwrap_or_default();
        for input in &inputs {
            // Counted before it takes effect: an input that fails is part of the history.
            self.applied_inputs += 1;
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
        if let Some(time) = e.model.resample_timer(&entry, &ctx)? {
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
        for e in &self.registered.entries {
            if let Some(r) = e
                .model
                .invariants(&self.tables)
                .into_iter()
                .find(|r| !r.holds)
            {
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
        t: SimTime,
        phase: Phase,
        snapshot: &WorldTables,
        jobs: &[Job],
        tally: &mut Tally,
    ) -> Result<(), SimError> {
        // Every owned column exists (materialised in `new`); check them all before moving any, so a
        // missing one fails the step without disturbing the tables (SPR-005 G-c).
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
        let inputs: Vec<(Owned, Vec<QueuedCommand>)> = jobs
            .iter()
            .map(|job| {
                let e = &self.registered.entries[job.index];
                let owned = e
                    .access
                    .writes
                    .iter()
                    .filter_map(|w| self.tables.take(w.column).map(|rows| (w.column, rows)))
                    .collect();
                (owned, self.commands.take(e.id))
            })
            .collect();
        let outcomes: Vec<JobOutcome> = {
            let tables = &self.tables;
            let agenda = &self.agenda;
            let entries = &self.registered.entries;
            let key = |e: &Entry| SystemKey::new(self.world_seed, self.branch_salt, e.system_id, 0);
            let step = |job: &Job, (cols, inbox): (Owned, Vec<QueuedCommand>)| -> JobOutcome {
                let e = &entries[job.index];
                let mut view = SystemView::new(e.id, &e.access, cols, tables, snapshot);
                let info = JobInfo {
                    system: e.id,
                    system_id: e.system_id,
                    key: key(e),
                    purposes: &e.purposes,
                    access: &e.access,
                    bindings: &e.bindings,
                    agenda,
                };
                let mut ctx = StepCtx::new(info, t, phase, inbox);
                let result = match &job.work {
                    Work::Timers(timers) => timers
                        .iter()
                        .try_for_each(|timer| e.model.fire(timer, &mut view, &mut ctx)),
                    Work::Step(binding) => e.model.step(*binding, &mut view, &mut ctx),
                };
                let (untaken, out) = ctx.finish();
                JobOutcome {
                    owned: view.into_owned(),
                    result,
                    untaken,
                    out,
                }
            };
            // A panic in a step becomes a poisoning `SimError::Panic` with a crash artifact
            // (GOV-004 §2; SPR-006 DEF-1), caught on the thread that ran the step. The panicking
            // job's columns and inbox are lost; the engine never runs again.
            let run = |job: &Job, input: (Owned, Vec<QueuedCommand>)| -> JobOutcome {
                std::panic::catch_unwind(AssertUnwindSafe(|| step(job, input))).unwrap_or_else(
                    |payload| JobOutcome {
                        owned: Owned::new(),
                        result: Err(SimError::Panic {
                            system: entries[job.index].id,
                            message: panic_message(payload.as_ref()),
                        }),
                        untaken: Vec::new(),
                        out: Outbox::default(),
                    },
                )
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
            if first_error.is_none() {
                first_error = o.result.err();
            }
            outboxes.push(o.out);
        }
        if let Some(e) = first_error {
            return Err(e);
        }
        for (job, out) in jobs.iter().zip(outboxes) {
            self.merge(t, phase, job.index, out, tally)?;
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

/// Runs `run` on every job with its input: in order on one thread, or spread over `threads`
/// workers (job k on worker k % threads, each keeping its jobs in order). Returns the outcomes in
/// job order whatever the thread count.
fn dispatch<I: Send, O: Send>(
    threads: usize,
    jobs: &[Job],
    inputs: Vec<I>,
    run: impl Fn(&Job, I) -> O + Sync,
) -> Vec<O> {
    let threads = threads.min(jobs.len());
    if threads <= 1 {
        return jobs.iter().zip(inputs).map(|(j, i)| run(j, i)).collect();
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
                        .map(|(k, j, input)| (k, run(j, input)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| match h.join() {
                Ok(v) => v,
                // INVARIANT: `run` catches every step's panic, so a worker never unwinds; if one
                // somehow does, re-raise it rather than lose it.
                Err(payload) => std::panic::resume_unwind(payload),
            })
            .collect()
    });
    done.sort_by_key(|(k, _)| *k);
    done.into_iter().map(|(_, o)| o).collect()
}

/// The text of a panic payload (`&str` or `String`; anything else is named as such).
fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic with a non-text payload".to_owned())
}
