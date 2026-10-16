//! The derived scheduler: binding order within each phase (SPR-005 T-05; `05.1` §2 rule 4).
//!
//! "Within each phase, system order is a topological sort of the declared read/write sets, with
//! ties broken by system ID. A dependency that runs against phase order must be declared as a
//! lag." Within a phase the unit of scheduling is the **binding**, a system at one pulse
//! ([`Node`]; DEF-7, Architect ruling). The scheduler derives everything from the registered
//! `AccessSpec`s and bindings:
//!
//! - **Edge.** System R reads column c at `same_instant` and system W ≠ R writes c: every binding
//!   of W in phase p precedes every binding of R in p, whatever their pulses. Two bindings of one
//!   system have no edge between them.
//! - **Phase order.** R reads c at `same_instant` in phase p, and W writes c in a later phase:
//!   a registration error, because R would see the previous instant's value without saying so.
//!   Pulses do not matter: every pulse fires at a year start, so any two bindings can meet.
//! - **Lags.** A lagged read (`start_of_day`, `previous_pulse`, `lagged`) adds no edge.
//! - **Cycle.** Same-instant edges that form a cycle within a phase are a registration error
//!   naming the systems on it; one read must declare a lag.
//!
//! The graph is the union over every pulse, so the bindings that fire at any one instant form an
//! acyclic subgraph, and its order is the full graph's order restricted to them. Two outputs per
//! phase, both pure functions of the registered declarations:
//! - [`PhaseSchedule::levels`]: antichains by longest-path depth over the full graph (a binding's
//!   level is one more than the deepest binding it depends on), each sorted coarser pulse first
//!   (Year, Season, Month, Day, Hour), then by Registry id. Bindings of one level write no column
//!   another member reads at `same_instant`, so different systems in a level can run on any number
//!   of threads; two bindings of one system in a level run in turn (`crate::engine`).
//! - [`PhaseSchedule::order`]: the canonical order, the levels in sequence. It is a topological
//!   sort with ties broken by pulse coarseness, then system id, and it is the order the engine runs
//!   and reports errors in at every thread count.
//!
//! This is the order `05.1` §2 rule 4 defines (v1.5, Architect rulings on SPR-005): dependency
//! levels, then pulse, then Registry id. It is not the lexicographically smallest topological order
//! (Kahn's algorithm taking the smallest ready id): both give the same state, but only the level
//! form is also the parallel execution order, so the first error, and later the emission order of
//! events, is the same at 1 and N threads. `tests/registration.rs` pins it with a case where the two
//! orders differ, and `tests/engine_contract.rs` with edges across pulses.
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
use crate::time::Pulse;

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

/// One binding within a phase: a system at one pulse. The derived order (pulse first, coarse to
/// fine, then system) is the tie order within a level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node {
    /// The pulse of the binding.
    pub pulse: Pulse,
    /// The system: an index into the scheduler's input, which the registration sorts by id.
    pub system: usize,
}

/// The schedule of one phase, over its bindings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PhaseSchedule {
    /// The canonical order: the levels in sequence.
    pub order: Vec<Node>,
    /// Antichains in dependency order, each sorted coarser pulse first, then by system.
    pub levels: Vec<Vec<Node>>,
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
    let nodes_of = |i: usize, phase: Phase| -> Vec<Node> {
        let pulses: BTreeSet<Pulse> = systems[i]
            .bindings
            .iter()
            .filter(|b| b.phase == phase)
            .map(|b| b.pulse)
            .collect();
        pulses
            .into_iter()
            .map(|pulse| Node { pulse, system: i })
            .collect()
    };
    // Edges per phase, between bindings: (writer binding, reader binding).
    let mut edges: BTreeMap<Phase, BTreeSet<(Node, Node)>> = BTreeMap::new();
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
                            let phase_edges = edges.entry(pr).or_default();
                            for wn in nodes_of(w, pr) {
                                for rn in nodes_of(r, pr) {
                                    phase_edges.insert((wn, rn));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let mut phases = Vec::with_capacity(Phase::ALL.len());
    for phase in Phase::ALL {
        let mut nodes: Vec<Node> = (0..systems.len())
            .flat_map(|i| nodes_of(i, phase))
            .collect();
        nodes.sort_unstable();
        let phase_edges = edges.remove(&phase).unwrap_or_default();
        match order_phase(&nodes, &phase_edges) {
            Ok(p) => phases.push(p),
            Err(stuck) => {
                // Name only the systems on a cycle: each strongly connected component of more than
                // one binding. Bindings merely downstream of a cycle are stuck too, but they are not
                // part of it (Tester DEF-1).
                for component in cycles(&stuck, &phase_edges) {
                    let ids: BTreeSet<&str> =
                        component.iter().map(|n| systems[n.system].id).collect();
                    let ids: Vec<&str> = ids.into_iter().collect();
                    problems.push(violation(
                        ids[0],
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

/// Kahn's algorithm over one phase's bindings (`nodes`, sorted): levels by longest-path depth,
/// each in node order (coarser pulse first, then system), and the canonical order (the levels in
/// sequence). Returns the bindings left on or behind a cycle if there is one.
fn order_phase(nodes: &[Node], edges: &BTreeSet<(Node, Node)>) -> Result<PhaseSchedule, Vec<Node>> {
    let mut indegree: BTreeMap<Node, usize> = nodes.iter().map(|&n| (n, 0)).collect();
    let mut succ: BTreeMap<Node, Vec<Node>> = BTreeMap::new();
    for &(w, r) in edges {
        succ.entry(w).or_default().push(r);
        *indegree.entry(r).or_default() += 1;
    }
    let mut depth: BTreeMap<Node, usize> = BTreeMap::new();
    let mut ready: BTreeSet<Node> = indegree
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
        let done: BTreeSet<Node> = visited.iter().copied().collect();
        return Err(nodes
            .iter()
            .copied()
            .filter(|n| !done.contains(n))
            .collect());
    }
    let mut levels: Vec<Vec<Node>> = Vec::new();
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

/// The cycles among `stuck` (the bindings Kahn's algorithm could not order): the strongly
/// connected components with more than one binding, each sorted, in order of their smallest
/// binding. Two bindings are in one component when each reaches the other; the graphs are a
/// phase's bindings, so the quadratic reachability is cheap.
fn cycles(stuck: &[Node], edges: &BTreeSet<(Node, Node)>) -> Vec<Vec<Node>> {
    let nodes: BTreeSet<Node> = stuck.iter().copied().collect();
    let mut succ: BTreeMap<Node, Vec<Node>> = BTreeMap::new();
    for &(w, r) in edges {
        if nodes.contains(&w) && nodes.contains(&r) {
            succ.entry(w).or_default().push(r);
        }
    }
    let reach = |from: Node| -> BTreeSet<Node> {
        let mut seen = BTreeSet::new();
        let mut todo = vec![from];
        while let Some(n) = todo.pop() {
            for &r in succ.get(&n).into_iter().flatten() {
                if seen.insert(r) {
                    todo.push(r);
                }
            }
        }
        seen
    };
    let reaches: BTreeMap<Node, BTreeSet<Node>> = nodes.iter().map(|&n| (n, reach(n))).collect();
    let mut placed = BTreeSet::new();
    let mut out = Vec::new();
    for &n in &nodes {
        if placed.contains(&n) || !reaches[&n].contains(&n) {
            continue;
        }
        let component: Vec<Node> = nodes
            .iter()
            .copied()
            .filter(|m| reaches[&n].contains(m) && reaches[m].contains(&n))
            .collect();
        placed.extend(component.iter().copied());
        out.push(component);
    }
    out
}
