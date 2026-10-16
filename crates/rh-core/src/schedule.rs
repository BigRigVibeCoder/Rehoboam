//! The derived scheduler: system order within each phase (SPR-005 T-05; `05.1` §2 rule 4).
//!
//! "Within each phase, system order is a topological sort of the declared read/write sets, with
//! ties broken by system ID. A dependency that runs against phase order must be declared as a
//! lag." The scheduler derives everything from the registered `AccessSpec`s and bindings:
//!
//! - **Edge.** System R reads column c at `same_instant` and system W ≠ R writes c, both bound in
//!   phase p: W runs before R.
//! - **Phase order.** R reads c at `same_instant` in phase p, and W writes c in a later phase:
//!   a registration error, because R would see the previous instant's value without saying so.
//!   Pulses do not matter: every pulse fires at a year start, so any two bindings can meet.
//! - **Lags.** A lagged read (`start_of_day`, `previous_pulse`, `lagged`) sees the start-of-instant
//!   value and adds no edge.
//! - **Cycle.** Same-instant reads that form a cycle within a phase are a registration error naming
//!   the systems; one read must declare a lag.
//!
//! Two outputs per phase, both pure functions of the registered declarations:
//! - [`PhaseSchedule::levels`]: antichains by longest-path depth (a system's level is one more than
//!   the deepest system it reads at `same_instant`), each sorted by id. Systems in one level write
//!   disjoint columns and read nothing another member writes at `same_instant`, so they can run on
//!   any number of threads.
//! - [`PhaseSchedule::order`]: the canonical order, the levels in sequence. It is a topological
//!   sort with ties broken by system id (`05.1` §2 rule 4), and it is the order the engine runs and
//!   reports errors in at every thread count.
//!
//! This is the order `05.1` §2 rule 4 defines (v1.4, Architect ruling on SPR-005): levels, then
//! Registry id, not the lexicographically smallest topological order (Kahn's algorithm taking the
//! smallest ready id). Both give the same state, because systems not joined by an edge never touch
//! the same column at the same instant. Only the level form is also the parallel execution order,
//! so the first error, and later the emission order of events, is the same at 1 and N threads.
//! `tests/registration.rs` pins it with a case where the two orders differ.
//!
//! No `HashMap` anywhere: `BTreeMap`/`BTreeSet` and sorted vectors only (GOV-003 §2).
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §2 rule 4, §4.2, §6 item 5;
//! `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-05.

use std::collections::{BTreeMap, BTreeSet};

use crate::ownership::Violation;

use crate::access::AccessSpec;
use crate::phase::Phase;
use crate::system::PulseBinding;

/// What the scheduler needs to know about one registered system.
#[derive(Clone, Debug)]
pub struct SchedulerInput<'a> {
    /// The system id (ties are broken by it).
    pub id: &'a str,
    /// Its declared access.
    pub access: &'a AccessSpec,
    /// Its bindings.
    pub bindings: &'a [PulseBinding],
}

/// The schedule of one phase. Systems are indices into the scheduler's input, which the
/// registration sorts by id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PhaseSchedule {
    /// The canonical order: the levels in sequence.
    pub order: Vec<usize>,
    /// Antichains for parallel execution, in dependency order.
    pub levels: Vec<Vec<usize>>,
}

/// The schedule of every phase, indexed by [`Phase::index`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Schedule {
    /// One entry per phase, in execution order.
    pub phases: Vec<PhaseSchedule>,
}

impl Schedule {
    /// The schedule of one phase.
    #[must_use]
    pub fn phase(&self, phase: Phase) -> &PhaseSchedule {
        &self.phases[usize::from(phase.index())]
    }
}

fn violation(system: &str, rule: &'static str, message: String) -> Violation {
    Violation {
        rule,
        system: system.to_owned(),
        message,
    }
}

/// Derives the schedule, or returns every phase-order and cycle violation. `systems` must be
/// sorted by id with no duplicates (registration guarantees both).
///
/// # Errors
/// `phase-order`: a `same_instant` read of a column written in a later phase.
/// `schedule-cycle`: `same_instant` reads forming a cycle within a phase.
pub fn derive(systems: &[SchedulerInput<'_>]) -> Result<Schedule, Vec<Violation>> {
    let mut problems = Vec::new();
    // column -> writers (index), in index order.
    let mut writers: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, s) in systems.iter().enumerate() {
        for w in &s.access.writes {
            writers.entry(w.column).or_default().push(i);
        }
    }
    let phases_of =
        |i: usize| -> BTreeSet<Phase> { systems[i].bindings.iter().map(|b| b.phase).collect() };
    // Edges per phase: (writer, reader).
    let mut edges: BTreeMap<Phase, BTreeSet<(usize, usize)>> = BTreeMap::new();
    for (r, reader) in systems.iter().enumerate() {
        for read in reader.access.reads.iter().filter(|x| !x.lag.is_lagged()) {
            for &w in writers.get(read.column).into_iter().flatten() {
                if w == r {
                    continue;
                }
                for pr in phases_of(r) {
                    for pw in phases_of(w) {
                        if pw > pr {
                            problems.push(violation(
                                reader.id,
                                "phase-order",
                                format!(
                                    "reads {} at same_instant in phase {pr}, but {} writes it in the later phase {pw}; declare a lag (05.1 §2 rule 4)",
                                    read.column, systems[w].id
                                ),
                            ));
                        } else if pw == pr {
                            edges.entry(pr).or_default().insert((w, r));
                        }
                    }
                }
            }
        }
    }
    let mut phases = Vec::with_capacity(Phase::ALL.len());
    for phase in Phase::ALL {
        let nodes: Vec<usize> = (0..systems.len())
            .filter(|&i| phases_of(i).contains(&phase))
            .collect();
        let phase_edges = edges.remove(&phase).unwrap_or_default();
        match order_phase(&nodes, &phase_edges) {
            Ok(p) => phases.push(p),
            Err(stuck) => {
                // Name only the systems on a cycle: each strongly connected component of more than
                // one system (an edge never joins a system to itself). Systems merely downstream of
                // a cycle are stuck too, but they are not part of it (Tester DEF-1).
                for component in cycles(&stuck, &phase_edges) {
                    let ids: Vec<&str> = component.iter().map(|&i| systems[i].id).collect();
                    problems.push(violation(
                        systems[component[0]].id,
                        "schedule-cycle",
                        format!(
                            "in phase {phase}, {} read each other's columns at same_instant in a cycle; declare a lag on one read (05.1 §2 rule 4)",
                            ids.join(", ")
                        ),
                    ));
                }
                phases.push(PhaseSchedule::default());
            }
        }
    }
    if problems.is_empty() {
        Ok(Schedule { phases })
    } else {
        Err(problems)
    }
}

/// Kahn's algorithm over one phase: levels by longest-path depth, and the canonical order (the
/// levels in sequence). Returns the nodes left on a cycle if there is one.
fn order_phase(
    nodes: &[usize],
    edges: &BTreeSet<(usize, usize)>,
) -> Result<PhaseSchedule, Vec<usize>> {
    let mut indegree: BTreeMap<usize, usize> = nodes.iter().map(|&n| (n, 0)).collect();
    let mut succ: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &(w, r) in edges {
        succ.entry(w).or_default().push(r);
        *indegree.entry(r).or_default() += 1;
    }
    let mut depth: BTreeMap<usize, usize> = BTreeMap::new();
    let mut ready: BTreeSet<usize> = indegree
        .iter()
        .filter(|&(_, &d)| d == 0)
        .map(|(&n, _)| n)
        .collect();
    let mut visited = Vec::with_capacity(nodes.len());
    while let Some(n) = ready.pop_first() {
        visited.push(n);
        let d = depth.get(&n).copied().unwrap_or(0);
        for &m in succ.get(&n).into_iter().flatten() {
            let md = depth.entry(m).or_insert(0);
            *md = (*md).max(d + 1);
            if let Some(deg) = indegree.get_mut(&m) {
                *deg -= 1;
                if *deg == 0 {
                    ready.insert(m);
                }
            }
        }
    }
    if visited.len() < nodes.len() {
        let done: BTreeSet<usize> = visited.iter().copied().collect();
        return Err(nodes
            .iter()
            .copied()
            .filter(|n| !done.contains(n))
            .collect());
    }
    let mut levels: Vec<Vec<usize>> = Vec::new();
    for &n in nodes {
        let d = depth.get(&n).copied().unwrap_or(0);
        if levels.len() <= d {
            levels.resize_with(d + 1, Vec::new);
        }
        levels[d].push(n);
    }
    let order = levels.concat();
    Ok(PhaseSchedule { order, levels })
}

/// The cycles among `stuck` (the nodes Kahn's algorithm could not order): the strongly connected
/// components with more than one node, each sorted, in order of their smallest node. Two nodes are
/// in one component when each reaches the other; the graphs are a phase's systems, so the
/// quadratic reachability is cheap.
fn cycles(stuck: &[usize], edges: &BTreeSet<(usize, usize)>) -> Vec<Vec<usize>> {
    let nodes: BTreeSet<usize> = stuck.iter().copied().collect();
    let reach = |from: usize| -> BTreeSet<usize> {
        let mut seen = BTreeSet::new();
        let mut todo = vec![from];
        while let Some(n) = todo.pop() {
            for &(w, r) in edges.range((n, 0)..=(n, usize::MAX)) {
                debug_assert_eq!(w, n);
                if nodes.contains(&r) && seen.insert(r) {
                    todo.push(r);
                }
            }
        }
        seen
    };
    let reaches: BTreeMap<usize, BTreeSet<usize>> = nodes.iter().map(|&n| (n, reach(n))).collect();
    let mut placed = BTreeSet::new();
    let mut out = Vec::new();
    for &n in &nodes {
        if placed.contains(&n) || !reaches[&n].contains(&n) {
            continue;
        }
        let component: Vec<usize> = nodes
            .iter()
            .copied()
            .filter(|m| reaches[&n].contains(m) && reaches[m].contains(&n))
            .collect();
        placed.extend(component.iter().copied());
        out.push(component);
    }
    out
}
