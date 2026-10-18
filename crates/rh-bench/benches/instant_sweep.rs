//! The per-instant cost's figures that are measured, not gated (SPR-038 T-01, T-04; BCK-002 A-60).
//!
//! Not in `bin/perf_gate.py`'s `BENCH_TARGETS`: these are the evidence of the plan's tables, run by
//! hand on the merge base and on the head (`cargo bench -p rh-bench --features arena-alloc --bench
//! instant_sweep`), with the figures recorded in `VER-SPR-038a` and the PR.
//!
//! - `quiet/*`, `death/*`: one instant of `rh_bench::instant`'s world over the sweep N in
//!   {1, 10^2, 10^3, 10^4} x L/N in {1, 10}.
//! - `m2_invariants/*`: one call of M2's `invariants()` at N = 10^4 and L = 10^4 or 10^5.
//! - `digest/*`: one month-end stop's digest as `rh-golden` takes it today
//!   (`state_digest(&engine.snapshot())`), at the `year` cases' sizes.
//! - `solo/years_20`: `solo` as `rh-cli run solo` builds it, 20 sim-years from genesis.
//!
//! REF: `CODEX/05_PROJECT/SPR-038_PerInstantCost.md` T-01.
#![allow(
    missing_docs,
    reason = "the iai-callgrind macros generate undocumented items"
)]
#![allow(
    clippy::disallowed_methods,
    reason = "iai-callgrind's main! macro reads the command line (std::env::args; SPR-011 A-4)"
)]

use std::fmt::Display;
use std::hint::black_box;
use std::path::Path;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};

/// The deterministic arena: counts that do not depend on the checkout's path (`alloc_count::Arena`).
#[global_allocator]
static ARENA: rh_bench::alloc_count::Arena = rh_bench::alloc_count::Arena;
use rh_bench::instant::{self, MEASURED_DAY, Mode, end_of_day};
use rh_bench::rungs;
use rh_core::engine::Engine;
use rh_core::system::InvariantResult;
use rh_core::time::SimTime;
use rh_core::world::WorldTables;

#[allow(
    clippy::panic,
    reason = "a bench whose setup fails has nothing to measure; it must stop loudly"
)]
fn ok<T, E: Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("bench setup failed: {e}"))
}

fn measured((mode, n, l): (Mode, usize, usize)) -> Engine {
    ok(instant::before_measured(mode, n, l))
}

fn one_instant(mut engine: Engine) -> Engine {
    ok(black_box(&mut engine).advance_until(end_of_day(MEASURED_DAY), |_, _| {}));
    engine
}

#[library_benchmark]
#[bench::n1_l1(args = ((Mode::Quiet, 1, 1)), setup = measured)]
#[bench::n1_l10(args = ((Mode::Quiet, 1, 10)), setup = measured)]
#[bench::n100_l100(args = ((Mode::Quiet, 100, 100)), setup = measured)]
#[bench::n100_l1k(args = ((Mode::Quiet, 100, 1_000)), setup = measured)]
#[bench::n1k_l1k(args = ((Mode::Quiet, 1_000, 1_000)), setup = measured)]
#[bench::n1k_l10k(args = ((Mode::Quiet, 1_000, 10_000)), setup = measured)]
#[bench::n10k_l10k(args = ((Mode::Quiet, 10_000, 10_000)), setup = measured)]
#[bench::n10k_l100k(args = ((Mode::Quiet, 10_000, 100_000)), setup = measured)]
fn quiet(engine: Engine) -> Engine {
    one_instant(engine)
}

#[library_benchmark]
#[bench::n1_l1(args = ((Mode::Death, 1, 1)), setup = measured)]
#[bench::n1_l10(args = ((Mode::Death, 1, 10)), setup = measured)]
#[bench::n100_l100(args = ((Mode::Death, 100, 100)), setup = measured)]
#[bench::n100_l1k(args = ((Mode::Death, 100, 1_000)), setup = measured)]
#[bench::n1k_l1k(args = ((Mode::Death, 1_000, 1_000)), setup = measured)]
#[bench::n1k_l10k(args = ((Mode::Death, 1_000, 10_000)), setup = measured)]
#[bench::n10k_l10k(args = ((Mode::Death, 10_000, 10_000)), setup = measured)]
#[bench::n10k_l100k(args = ((Mode::Death, 10_000, 100_000)), setup = measured)]
fn death(engine: Engine) -> Engine {
    one_instant(engine)
}

#[library_benchmark]
#[bench::n10k_l10k(args = (10_000, 10_000), setup = instant::tables)]
#[bench::n10k_l100k(args = (10_000, 100_000), setup = instant::tables)]
fn m2_invariants(tables: WorldTables) -> Vec<InvariantResult> {
    rh_sys_demography::demography::invariants(black_box(&tables))
}

fn genesis((n, l): (usize, usize)) -> Engine {
    ok(instant::engine(Mode::Quiet, n, l))
}

#[library_benchmark]
#[bench::n10k_l10k(args = ((10_000, 10_000)), setup = genesis)]
#[bench::n10k_l100k(args = ((10_000, 100_000)), setup = genesis)]
fn digest(engine: Engine) -> u64 {
    rh_snapshot::state::state_digest(&black_box(&engine).snapshot()).hash
}

fn solo_genesis(_: ()) -> Engine {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    ok(rungs::engine(&repo, "solo", 1)).0
}

#[library_benchmark]
#[bench::years_20(args = (()), setup = solo_genesis)]
fn solo(mut engine: Engine) -> Engine {
    let end = SimTime::from_minutes(20 * 360 * instant::DAY - 1);
    ok(black_box(&mut engine).advance_until(end, |_, _| {}));
    engine
}

library_benchmark_group!(
    name = sweep_group;
    benchmarks = quiet, death, m2_invariants, digest, solo
);
main!(library_benchmark_groups = sweep_group);
