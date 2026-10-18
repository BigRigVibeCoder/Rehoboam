//! The smoke world against its specs, and what a run exercises (SPR-008 T-01, rulings 2 and 3).
//!
//! - Registration compares the dummy code with `worlds/smoke/systems` and its Registry overlay
//!   (rule `spec` and every other rule), as for real systems, with no reports left over.
//! - Every phase of `05.1` §4.2 and every pulse is bound; two bindings of one system share a phase.
//! - A 30-year run with the synthetic material inputs exercises timers (scheduled, fired,
//!   cancelled, re-sampled), lifecycle commands (sent with and without a cause, applied), events
//!   with causal edges (through a command and draft-local), and passes every invariant.
//! - Thread-count and repeat invariance on that run (V-DET-01/02 at small scale; the golden run
//!   is T-02's).
//!
//! Refs: SPR-008 T-01, rulings 2–3; `05.1` §2, §4; ADR-0025.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken smoke world must fail the test loudly"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rh_core::engine::Engine;
use rh_core::ownership::SpecTable;
use rh_core::state::EngineState;
use rh_core::time::SimTime;
use rh_core::{Phase, Pulse};
use rh_events::Event;
use rh_registry::ownership::Ownership;
use rh_sys_smoke::{CALENDAR, OVERLAY, SPECS_DIR, material, synthetic_inputs};
use spec_files::{load_specs, registry_system_ids, registry_table};

mod spec_files;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn specs() -> SpecTable {
    let ids = registry_system_ids(&repo().join(OVERLAY)).expect("overlay");
    let specs = load_specs(&repo().join(SPECS_DIR)).expect("specs");
    let table = registry_table(&repo().join("specs/manifest/registry_v0.toml")).expect("registry");
    Ownership::build(&specs, &ids).spec_table_with(table)
}

fn engine(seed: u64, threads: usize) -> Engine {
    rh_sys_smoke::engine(&specs(), seed, threads).unwrap_or_else(|e| panic!("{e}"))
}

/// Everything a run leaves.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    events: Vec<Event>,
    /// The whole engine state at the end of every year (hash-agnostic: a system crate's tests
    /// never depend on rh-snapshot, V-INSP-03; the hash ladder is rh-cli's).
    yearly: Vec<EngineState>,
    agenda: usize,
    waiting: usize,
    instants: usize,
}

fn run(seed: u64, threads: usize, years: i64) -> Run {
    let mut engine = engine(seed, threads);
    for (at, entity) in synthetic_inputs(years) {
        engine.submit_input(at, material(entity)).expect("input");
    }
    let mut events = Vec::new();
    let mut yearly = Vec::new();
    let mut instants = 0;
    for year in 1..=years {
        let end = SimTime::from_minutes(year * CALENDAR.minutes_per_year() - 1);
        engine
            .advance_until(end, |_, _| instants += 1)
            .unwrap_or_else(|e| panic!("{e}"));
        events.extend(engine.drain_events());
        yearly.push(engine.snapshot());
    }
    Run {
        events,
        yearly,
        agenda: engine.agenda().len(),
        waiting: engine.commands().len(),
        instants,
    }
}

fn count(events: &[Event], kind: &str) -> usize {
    events.iter().filter(|e| e.kind == kind).count()
}

#[test]
fn the_code_registers_against_its_specs_with_nothing_reported() {
    let registered =
        rh_core::register::register(rh_sys_smoke::systems(), &specs()).unwrap_or_else(|e| {
            panic!("{e}");
        });
    assert_eq!(registered.reports(), [] as [String; 0]);
    assert_eq!(registered.ids().len(), 7);
}

#[test]
fn every_phase_and_every_pulse_is_bound_and_one_system_binds_a_phase_twice() {
    let engine = engine(1, 1);
    let r = engine.registered();
    let mut pulses = BTreeSet::new();
    for phase in Phase::ALL {
        let bindings = r.bindings(phase);
        assert!(!bindings.is_empty(), "{phase} has no binding");
        pulses.extend(bindings.iter().map(|(_, p)| *p));
    }
    assert_eq!(pulses, Pulse::ALL.into_iter().collect());
    let polity = r.bindings(Phase::Polity);
    assert_eq!(
        polity
            .iter()
            .filter(|(id, _)| *id == "X5.smoke_polity")
            .count(),
        2,
        "{polity:?}"
    );
}

#[test]
fn a_run_exercises_timers_commands_events_and_invariants() {
    let r = run(3, 1, 30);
    for kind in [
        "X_DEATH",
        "CURED",
        "BIRTH",
        "INHERITANCE",
        "DECREE",
        "CENSUS",
        "CENSUS_NOTE",
        "FROST",
        "TRADE",
    ] {
        assert!(
            count(&r.events, kind) > 10,
            "{kind}: {}",
            count(&r.events, kind)
        );
    }
    let deaths = count(&r.events, "X_DEATH");
    let caused = r
        .events
        .iter()
        .filter(|e| e.kind == "INHERITANCE" && !e.causes.is_empty())
        .count();
    let uncaused = count(&r.events, "INHERITANCE") - caused;
    assert!(
        caused > 0 && uncaused > 0,
        "Settle with and without a cause"
    );
    assert!(caused <= deaths);
    for e in r.events.iter().filter(|e| e.kind == "CENSUS_NOTE") {
        let edges: Vec<_> = e.causes.iter().map(|c| (c.cause, c.kind)).collect();
        assert_eq!(
            edges,
            [(
                rh_ids::EventId::from_raw(e.id.to_raw() - 1),
                rh_events::EdgeKind::Triggered
            )]
        );
    }
    // Death ticks fall between pulse boundaries: timers fire at their own minute.
    let off_day = r
        .events
        .iter()
        .filter(|e| e.kind == "X_DEATH" && e.tick % 1_440 != 0)
        .count();
    assert!(off_day > 0);
    assert_eq!(r.waiting, 0, "every Settle is applied");
    assert!(r.agenda > 0, "death timers are pending at the end");
    assert_eq!(synthetic_inputs(30).len(), 4 * 8);
}

#[test]
fn material_inputs_re_sample_the_timers_in_their_scope() {
    let years = 8;
    let inputs = synthetic_inputs(years);
    let at = inputs[0].0;
    let scope: BTreeSet<_> = inputs
        .iter()
        .filter(|(t, _)| *t == at)
        .map(|(_, e)| *e)
        .collect();
    assert_eq!(scope.len(), 8);
    let mut with = engine(5, 1);
    let mut without = engine(5, 1);
    let before = at.checked_add_minutes(-1).unwrap();
    with.advance_until(before, |_, _| {}).unwrap();
    without.advance_until(before, |_, _| {}).unwrap();
    let pending = |e: &Engine| {
        e.agenda()
            .pending()
            .into_iter()
            .map(|(k, _)| (k.entity, k.time))
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(pending(&with), pending(&without));
    let scoped: Vec<_> = pending(&with)
        .into_iter()
        .filter(|(e, _)| scope.contains(e))
        .collect();
    assert!(!scoped.is_empty(), "the input's scope has pending timers");
    for &e in &scope {
        with.submit_input(at, material(e)).unwrap();
    }
    with.advance_until(at, |_, _| {}).unwrap();
    without.advance_until(at, |_, _| {}).unwrap();
    let (a, b) = (pending(&with), pending(&without));
    let moved: Vec<_> = b.difference(&a).collect();
    assert!(!moved.is_empty(), "the scope's timers are re-drawn");
    assert!(moved.iter().all(|(e, _)| scope.contains(e)), "{moved:?}");
    for e in &scope {
        assert_eq!(with.epoch(*e), 1);
    }
}

#[test]
fn v_det_01_02_small_scale_thread_and_repeat_invariance() {
    let base = run(9, 1, 6);
    assert!(base.instants > 6 * 8_760);
    for threads in [2, 4, 8] {
        assert_eq!(run(9, threads, 6), base, "{threads} threads");
    }
    assert_eq!(run(9, 1, 6), base, "repeat");
    println!(
        "SMOKE: {} yearly states and {} events equal at 1, 2, 4 and 8 threads and on repeat",
        base.yearly.len(),
        base.events.len()
    );
}

/// VER-SPR-008 DEF-2: the thread-count comparison is not vacuous. Every day, Environment holds X1
/// and X7 and Activity holds X4 and X7: those levels hold two jobs, so N threads dispatch them in
/// parallel. The Activity pair also exchanges commands (Settle, Audit).
#[test]
fn def_2_two_phases_hold_parallel_levels_every_day() {
    let engine = engine(1, 1);
    let r = engine.registered();
    for phase in [Phase::Environment, Phase::Activity] {
        let level0: Vec<_> = r.schedule().phase(phase).levels[0]
            .iter()
            .map(|n| r.ids()[n.system])
            .collect();
        assert!(level0.len() >= 2, "{phase}: {level0:?}");
    }
    let mut engine = engine;
    let end = SimTime::from_minutes(CALENDAR.minutes_per_year() - 1);
    engine.advance_until(end, |_, _| {}).unwrap();
    let stats = engine.dispatch_stats();
    assert_eq!(stats.max_jobs, 2);
    // Two parallel levels every day (Environment and Activity), and nothing else holds two jobs.
    assert_eq!(stats.parallel_levels, 2 * 365, "{stats:?}");
    assert!(stats.parallel_levels * 100 / stats.levels >= 5, "{stats:?}");
    println!("DEF-2: {stats:?}");
    let events = engine.drain_events();
    let audits = engine.tables().get("market.audits").unwrap()[0];
    assert!(audits > 0, "X4 sends Audit to X7, which applies it");
    assert!(events.iter().any(|e| e.kind == "TRADE"));
}
