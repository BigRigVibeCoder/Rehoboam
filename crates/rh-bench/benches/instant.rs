//! The per-instant engine cost, in instructions (iai-callgrind; SPR-038 T-01, BCK-002 A-60).
//!
//! Each case runs instants of the stub world of `rh_bench::instant`: one system that owns M2's 19
//! columns, with a drain-only `Day`/`Vital` binding whose inbox is always empty, a `Month`/`Vital`
//! step and a `Day`/`Vital` timer. `n` is the living persons, `l` the life-course rows.
//!
//! - `quiet/*`: one quiet instant, a day start where only the empty drain fires.
//! - `death/*`: one instant where a death timer closes one person's row and entry.
//! - `insert/*`: one instant where a birth timer inserts one ledger row and appends one entry.
//! - `year/*`: the genesis held for a year: 360 day starts, 12 of them month starts whose step
//!   rewrites one `person.spouse` cell.
//!
//! Every case asserts its path in its teardown, outside the measurement.
//!
//! REF: `CODEX/05_PROJECT/SPR-038_PerInstantCost.md` T-01; `docs/09` V-PERF-04.
#![allow(
    missing_docs,
    reason = "the iai-callgrind macros generate undocumented items"
)]
#![allow(
    clippy::disallowed_methods,
    reason = "iai-callgrind's main! macro reads the command line (std::env::args; SPR-011 A-4)"
)]
#![allow(
    clippy::needless_pass_by_value,
    reason = "an iai-callgrind teardown takes the bench's return value by value"
)]

use std::fmt::Display;
use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};

/// The deterministic arena: counts that do not depend on the checkout's path (`alloc_count::Arena`).
#[global_allocator]
static ARENA: rh_bench::alloc_count::Arena = rh_bench::alloc_count::Arena;
use rh_bench::instant::{self, MEASURED_DAY, Mode, YEAR_DAYS, end_of_day};
use rh_core::engine::{DispatchStats, Engine};
use rh_core::time::SimTime;

#[allow(
    clippy::panic,
    reason = "a bench whose setup fails has nothing to measure; it must stop loudly"
)]
fn ok<T, E: Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("bench setup failed: {e}"))
}

fn path(holds: bool, what: &str) {
    assert!(holds, "bench path not taken: {what}");
}

/// What a case measured, with what its teardown checks it against.
struct Ran {
    engine: Engine,
    before: DispatchStats,
    n: usize,
    l: usize,
}

fn rows(e: &Engine, column: &str) -> usize {
    e.tables().get(column).map_or(0, Vec::len)
}

fn measured((mode, n, l): (Mode, usize, usize)) -> (Engine, usize, usize) {
    (ok(instant::before_measured(mode, n, l)), n, l)
}

fn genesis((n, l): (usize, usize)) -> (Engine, usize, usize) {
    (ok(instant::engine(Mode::Quiet, n, l)), n, l)
}

fn one_instant((mut engine, n, l): (Engine, usize, usize)) -> Ran {
    let before = engine.dispatch_stats();
    ok(black_box(&mut engine).advance_until(end_of_day(MEASURED_DAY), |_, _| {}));
    Ran {
        engine,
        before,
        n,
        l,
    }
}

/// Common to every one-instant case: exactly one instant ran, with the levels `levels`.
fn one_instant_ran(r: &Ran, levels: u64) {
    path(
        r.engine.next_instant().map(SimTime::minutes) == Some((MEASURED_DAY + 1) * instant::DAY),
        "one instant, the measured day start",
    );
    path(
        r.engine.dispatch_stats().levels - r.before.levels == levels,
        "the measured instant's levels",
    );
}

fn quiet_path(r: Ran) {
    // Only the empty drain: one level, no row added or removed.
    one_instant_ran(&r, 1);
    path(rows(&r.engine, "person.id") == r.n, "no ledger row changed");
    path(
        rows(&r.engine, "life_course.person_id") == r.l,
        "no entry appended",
    );
    path(r.engine.agenda().is_empty(), "no timer pending");
}

fn death_path(r: Ran) {
    // The timer pass and the drain: two levels; one row compacted away, one entry closed.
    one_instant_ran(&r, 2);
    path(rows(&r.engine, "person.id") == r.n - 1, "one row compacted");
    path(
        rows(&r.engine, "life_course.person_id") == r.l,
        "no entry appended",
    );
    let t = MEASURED_DAY * instant::DAY;
    let closed_now = r
        .engine
        .tables()
        .get("life_course.exit_tick")
        .map_or(0, |c| c.iter().filter(|&&x| x == t).count());
    path(closed_now == 1, "one entry closed at the measured instant");
}

fn insert_path(r: Ran) {
    one_instant_ran(&r, 2);
    path(rows(&r.engine, "person.id") == r.n + 1, "one row inserted");
    path(
        rows(&r.engine, "life_course.person_id") == r.l + 1,
        "one entry appended",
    );
}

fn year_path(r: Ran) {
    // 360 day starts, each with the drain; 12 month starts, each with one more level.
    path(
        r.engine.next_instant().map(SimTime::minutes) == Some(YEAR_DAYS * instant::DAY),
        "the year's 360 instants",
    );
    let levels = u64::try_from(YEAR_DAYS + 12).unwrap_or(u64::MAX);
    path(
        r.engine.dispatch_stats().levels - r.before.levels == levels,
        "the year's levels",
    );
    path(rows(&r.engine, "person.id") == r.n, "no ledger row changed");
}

#[library_benchmark]
#[bench::n1_l1(args = ((Mode::Quiet, 1, 1)), setup = measured, teardown = quiet_path)]
#[bench::n10k_l100k(args = ((Mode::Quiet, 10_000, 100_000)), setup = measured, teardown = quiet_path)]
fn quiet(input: (Engine, usize, usize)) -> Ran {
    one_instant(input)
}

#[library_benchmark]
#[bench::n10k_l10k(args = ((Mode::Death, 10_000, 10_000)), setup = measured, teardown = death_path)]
#[bench::n10k_l100k(args = ((Mode::Death, 10_000, 100_000)), setup = measured, teardown = death_path)]
fn death(input: (Engine, usize, usize)) -> Ran {
    one_instant(input)
}

#[library_benchmark]
#[bench::n10k_l100k(args = ((Mode::Insert, 10_000, 100_000)), setup = measured, teardown = insert_path)]
fn insert(input: (Engine, usize, usize)) -> Ran {
    one_instant(input)
}

#[library_benchmark]
#[bench::n10k_l10k(args = ((10_000, 10_000)), setup = genesis, teardown = year_path)]
#[bench::n10k_l100k(args = ((10_000, 100_000)), setup = genesis, teardown = year_path)]
fn year((mut engine, n, l): (Engine, usize, usize)) -> Ran {
    let before = engine.dispatch_stats();
    ok(black_box(&mut engine).advance_until(end_of_day(YEAR_DAYS - 1), |_, _| {}));
    Ran {
        engine,
        before,
        n,
        l,
    }
}

library_benchmark_group!(name = instant_group; benchmarks = quiet, death, insert, year);
main!(library_benchmark_groups = instant_group);
