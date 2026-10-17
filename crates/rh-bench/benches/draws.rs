//! The per-draw cost of `StepCtx`'s keyed draws, in instructions (iai-callgrind; SPR-011 T-02,
//! BCK-002 A-37).
//!
//! A-37 moved the stream guard's taken set from one step to one (system, instant), and A-29 keys
//! every draw by the drawn entity's epoch. Each case runs one instant of an engine whose one system
//! makes 1,000 draws (`rh_bench::draws`), so `(case - empty) / 1,000` is the cost of one draw
//! through the step context, and `raw_1000` is the same draws straight from `rh-ids`: their
//! difference is what the guard and the epoch lookup add per draw.
//!
//! - `empty`: the instant with a step that draws nothing.
//! - `rng_epoch0_1000`, `stream_epoch0_1000`: 1,000 draws at epoch 0, by `rng` and by `stream`.
//! - `rng_epoch1_1000`: the same 1,000 `rng` draws with every person at epoch 1 (one `K1` more).
//! - `raw_1000`: the 1,000 draws from `rh-ids`'s keys alone.
//!
//! REF: `CODEX/05_PROJECT/SPR-011_WalkingSkeleton.md` T-02; `docs/09` V-PERF-04.
#![allow(
    missing_docs,
    reason = "the iai-callgrind macros generate undocumented items"
)]

use std::fmt::Display;
use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};

/// The deterministic arena: counts that do not depend on the checkout's path (`alloc_count::Arena`).
#[global_allocator]
static ARENA: rh_bench::alloc_count::Arena = rh_bench::alloc_count::Arena;
use rh_bench::draws::{self, DAY, Mode};
use rh_core::engine::Engine;
use rh_core::time::SimTime;
use rh_ids::{PurposeId, SystemKey};

#[allow(
    clippy::panic,
    reason = "a bench whose setup fails has nothing to measure; it must stop loudly"
)]
fn ok<T, E: Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("bench setup failed: {e}"))
}

fn setup((mode, epoch): (Mode, u32)) -> Engine {
    ok(draws::engine(mode, epoch))
}

#[library_benchmark]
#[bench::empty(args = ((Mode::Empty, 0)), setup = setup)]
#[bench::rng_epoch0_1000(args = ((Mode::Rng, 0)), setup = setup)]
#[bench::stream_epoch0_1000(args = ((Mode::Stream, 0)), setup = setup)]
#[bench::rng_epoch1_1000(args = ((Mode::Rng, 1)), setup = setup)]
fn step_draws(mut engine: Engine) -> Engine {
    ok(black_box(&mut engine).advance_until(SimTime::from_minutes(DAY), |_, _| {}));
    engine
}

#[library_benchmark]
#[bench::raw_1000(setup = draws::raw_keys)]
fn raw_draws((key, purpose): (SystemKey, PurposeId)) -> u64 {
    draws::raw_draws(black_box(&key), black_box(purpose))
}

library_benchmark_group!(name = draws_group; benchmarks = step_draws, raw_draws);
main!(library_benchmark_groups = draws_group);
