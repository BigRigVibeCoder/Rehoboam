//! The instant loop: pulses, phases and the derived schedule, on one or several threads
//! (SPR-005 T-05; V-DET-01, V-DET-02).
//!
//! At each instant where a bound pulse fires, the engine:
//! 1. records the start-of-instant value of every column some system reads with a lag;
//! 2. runs the phases in `05.1` §4.2 order;
//! 3. within a phase, runs the firing bindings level by level in the derived schedule (`05.1` §2
//!    rule 4: dependency levels over every binding, ties coarser pulse first, then id): with one
//!    thread a level's bindings run in order; with several they are spread over the threads, each
//!    system's owned columns moved to its thread and back. Two bindings of one system in a level
//!    run in turn. Every binding of a level runs before an error is reported.
//!
//! The thread count changes scheduling only, never the result: `tests/determinism.rs` runs the
//! stub world at 1, 2, 4 and 8 threads and compares monthly state hashes (V-DET-01), and repeats a
//! run three times (V-DET-02). The same systems run at an erroring instant at every thread count,
//! and the first error in canonical order is the one reported.
//!
//! No wall clock, no unkeyed RNG, no hash-map iteration (GOV-003 §2). Threads come from
//! `std::thread::scope`; no third-party dependency.
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §4.1, §4.2, §6 items 2 and 5;
//! `docs/09` V-DET-01, V-DET-02; `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-05.

use std::collections::{BTreeMap, BTreeSet};

use rh_ids::{BranchSalt, SystemKey};

use crate::phase::Phase;
use crate::register::{Entry, Registered};
use crate::system::{InvariantResult, PulseBinding, SimError, StepCtx};
use crate::time::{Calendar, Pulse, SimTime};
use crate::world::{Column, SystemView, WorldTables};

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
    /// The error that stopped the run, if any. A poisoned engine never runs again (G-d).
    poisoned: Option<SimError>,
}

/// One system's work at one instant: the entry index and the binding that fires.
type Job = (usize, PulseBinding);

/// The columns one system owns, moved out of the tables for its step.
type Owned = BTreeMap<&'static str, Column>;

/// The result of one job: its owned columns, and the step error if any.
type JobOutcome = (Owned, Result<(), SimError>);

impl Engine {
    /// An engine whose first instant is the first bound-pulse boundary at or after `start`.
    ///
    /// Every column a registered system declares as a write and `tables` lacks is materialised
    /// here, empty: the only place a column is created (G-c). At run time an access to a column
    /// the world does not have is a fatal [`SimError::MissingColumn`], never a creation.
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
        for e in &registered.entries {
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
            poisoned: None,
        }
    }

    /// Sets the number of worker threads (at least 1). Changes speed only, never the result.
    pub fn set_threads(&mut self, threads: usize) {
        self.threads = threads.max(1);
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

    /// The next instant to run, if any.
    #[must_use]
    pub fn next_instant(&self) -> Option<SimTime> {
        let c = self.cursor?;
        if self
            .bound_pulses
            .iter()
            .any(|&p| self.calendar.is_boundary(p, c))
        {
            Some(c)
        } else {
            self.calendar.next_instant(c, &self.bound_pulses)
        }
    }

    /// Runs every instant up to and including `end`, calling `after_instant` after each one with
    /// the instant and the tables (for hashes, invariants, logging).
    ///
    /// # Errors
    /// The first [`SimError`] in schedule order; the engine stops at that instant and is
    /// poisoned: every later call returns that same error and runs nothing, so a failed instant is
    /// never replayed on half-mutated tables (G-d; SPR-006 adds the crash artifact).
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
                self.poisoned = Some(e.clone());
                return Err(e);
            }
            after_instant(t, &self.tables);
            self.cursor = t.checked_add_minutes(1);
        }
        Ok(())
    }

    /// The error that poisoned the engine, if a step has failed.
    #[must_use]
    pub fn poisoned(&self) -> Option<&SimError> {
        self.poisoned.as_ref()
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
        // Within a phase, the firing bindings run level by level in the derived schedule (`05.1`
        // §2 rule 4): the levels come from the dependency graph over every binding, and each level
        // is sorted coarser pulse first, then by id. A level is split wherever a system's second
        // binding appears, so that one system's bindings never run at once (they share its
        // columns); the split depends on the schedule only, never on the thread count.
        for phase in Phase::ALL {
            let plan = self.registered.schedule().phase(phase).clone();
            for level in &plan.levels {
                let mut groups: Vec<Vec<Job>> = vec![Vec::new()];
                for n in level.iter().filter(|n| fired.contains(&n.pulse)) {
                    let repeats = groups
                        .last()
                        .is_some_and(|g| g.iter().any(|(i, _)| *i == n.system));
                    if repeats {
                        groups.push(Vec::new());
                    }
                    if let Some(g) = groups.last_mut() {
                        g.push((
                            n.system,
                            PulseBinding {
                                pulse: n.pulse,
                                phase,
                            },
                        ));
                    }
                }
                for jobs in groups.into_iter().filter(|g| !g.is_empty()) {
                    self.run_level(t, &snapshot, &jobs)?;
                }
            }
        }
        Ok(())
    }

    /// Runs one level of jobs (in order on one thread, spread over several otherwise), then returns
    /// every owned column to the tables and reports the first error in job order.
    fn run_level(
        &mut self,
        t: SimTime,
        snapshot: &WorldTables,
        jobs: &[Job],
    ) -> Result<(), SimError> {
        // Every owned column exists (materialised in `new`); check them all before moving any, so a
        // missing one fails the step without disturbing the tables (G-c).
        for (i, _) in jobs {
            let e = &self.registered.entries[*i];
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
        let mut owned: Vec<Owned> = jobs
            .iter()
            .map(|(i, _)| {
                self.registered.entries[*i]
                    .access
                    .writes
                    .iter()
                    .filter_map(|w| self.tables.take(w.column).map(|rows| (w.column, rows)))
                    .collect()
            })
            .collect();
        let outcomes: Vec<JobOutcome> = {
            let tables = &self.tables;
            let entries = &self.registered.entries;
            let key = |e: &Entry| SystemKey::new(self.world_seed, self.branch_salt, e.system_id, 0);
            let run = |&(i, binding): &Job, cols: Owned| -> JobOutcome {
                let e = &entries[i];
                let mut view = SystemView::new(e.id, &e.access, cols, tables, snapshot);
                let mut ctx = StepCtx::new(e.id, key(e), t, &e.purposes);
                let result = e.model.step(binding, &mut view, &mut ctx);
                (view.into_owned(), result)
            };
            let threads = self.threads.min(jobs.len());
            if threads <= 1 {
                jobs.iter()
                    .zip(owned.drain(..))
                    .map(|(j, cols)| run(j, cols))
                    .collect()
            } else {
                // Job k goes to worker k % threads; each worker keeps its jobs in order.
                let mut per_worker: Vec<Vec<(usize, &Job, Owned)>> =
                    (0..threads).map(|_| Vec::new()).collect();
                for (k, (j, cols)) in jobs.iter().zip(owned.drain(..)).enumerate() {
                    per_worker[k % threads].push((k, j, cols));
                }
                let mut done: Vec<(usize, JobOutcome)> = std::thread::scope(|scope| {
                    let handles: Vec<_> = per_worker
                        .into_iter()
                        .map(|work| {
                            scope.spawn(move || {
                                work.into_iter()
                                    .map(|(k, j, cols)| (k, run(j, cols)))
                                    .collect::<Vec<_>>()
                            })
                        })
                        .collect();
                    handles
                        .into_iter()
                        .flat_map(|h| match h.join() {
                            Ok(v) => v,
                            // A panic in a system step is a violated invariant: re-raise it.
                            Err(payload) => std::panic::resume_unwind(payload),
                        })
                        .collect()
                });
                done.sort_by_key(|(k, _)| *k);
                done.into_iter().map(|(_, o)| o).collect()
            }
        };
        let mut first_error = None;
        for (cols, result) in outcomes {
            for (name, rows) in cols {
                self.tables.insert(name, rows);
            }
            if first_error.is_none() {
                first_error = result.err();
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}
