//! Bench 8 of `docs/08` §9, `bench/keyframe_write_read`: writing a keyframe (the copy-on-write
//! snapshot, row encoding, zstd compression and the read-back check) and reading one
//! (decompression, decoding, the hash check), in instructions (iai-callgrind; SPR-009 T-01, T-02a).
//!
//! Each case's setup builds its input outside the measured function, and each measured function
//! returns its input, so neither construction nor drop is counted.
//!
//! - `write`: `write_keyframe` of the fixture engine. It includes the read-back that every write
//!   does (SPR-007 DEF-3 layer 3), as a real write does.
//! - `read`: `read_keyframe`: envelope, IPC guard, decompression, decoding and the state-hash check.
//! - `load`: `load_engine`: `read`, the history check, `Engine::restore` and the restored engine's
//!   hash.
//!
//! The 100k and 1M sizes of `docs/08` §9 are wall-clock trends (T-02b, T-04), not this gate.
//!
//! REF: `docs/08_performance_and_capacity.md` §9 bench 8; `docs/09` V-PERF-04, V-PERF-05;
//! `CODEX/05_PROJECT/SPR-009_Benchmarks.md` T-01, ruling 2.
#![allow(
    missing_docs,
    reason = "the iai-callgrind macros generate undocumented items"
)]

use std::fmt::Display;
use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use rh_bench::fixture::{self, CALENDAR, PR_ROWS};
use rh_core::engine::Engine;
use rh_core::register::Registered;
use rh_core::state::EngineState;
use rh_snapshot::keyframe::{load_engine, read_keyframe, write_keyframe};

#[allow(
    clippy::panic,
    reason = "a bench whose setup fails has nothing to measure; it must stop loudly"
)]
fn ok<T, E: Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("bench setup failed: {e}"))
}

fn setup_engine(rows: usize) -> Engine {
    ok(fixture::engine(rows))
}

/// A keyframe's bytes and the registered set to read it with.
fn setup_keyframe(rows: usize) -> (Vec<u8>, Registered) {
    let engine = setup_engine(rows);
    let (bytes, _) = ok(write_keyframe(&engine, fixture::anchor()));
    (bytes, ok(fixture::registered()))
}

#[library_benchmark]
#[bench::rows_10k(args = (PR_ROWS), setup = setup_engine)]
fn write(engine: Engine) -> (Vec<u8>, Engine) {
    let (bytes, _) = ok(write_keyframe(black_box(&engine), fixture::anchor()));
    (bytes, engine)
}

#[library_benchmark]
#[bench::rows_10k(args = (PR_ROWS), setup = setup_keyframe)]
fn read(input: (Vec<u8>, Registered)) -> (u64, EngineState, (Vec<u8>, Registered)) {
    let (state, header) = ok(read_keyframe(black_box(&input.0), &input.1));
    (header.state_hash, state, input)
}

#[library_benchmark]
#[bench::rows_10k(args = (PR_ROWS), setup = setup_keyframe)]
fn load(input: (Vec<u8>, Registered)) -> (u64, Engine, Vec<u8>) {
    let (bytes, registered) = input;
    let (engine, header) = ok(load_engine(
        black_box(&bytes),
        CALENDAR,
        registered,
        &fixture::history(),
    ));
    (header.state_hash, engine, bytes)
}

library_benchmark_group!(
    name = keyframe_write_read;
    benchmarks = write, read, load
);

main!(library_benchmark_groups = keyframe_write_read);
