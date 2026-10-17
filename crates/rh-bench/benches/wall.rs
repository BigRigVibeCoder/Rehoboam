//! Wall-clock trends of benches 7 and 8 at the sizes of `docs/08` §9 (SPR-009 T-02b), with
//! criterion. The cases mirror the instruction-count benches (`plane_hashes.rs`,
//! `keyframe_write_read.rs`); only the sizes and the clock differ.
//!
//! - Sizes: `RH_WALL_ROWS`, a comma-separated list of row counts, default `100000,1000000`.
//! - Ids: `<bench>/<case>/<rows>`, for example `plane_hashes/state_full/1000000`.
//! - `bin/wall_trend.py` reads criterion's own sample data (`target/criterion/**/new/sample.json`),
//!   takes the median and the spread of the samples (at least 5; RUN-002 §5), records the run
//!   conditions and compares medians with a committed trend file (a 10% alert).
//!
//! Wall time is noisy on a laptop or a hosted runner (RUN-001 §6), so these never gate a PR: they
//! gate only on `adjutant`, once T-04 has measured its run-to-run spread.
//!
//! REF: `docs/08_performance_and_capacity.md` §9 benches 7 and 8; RUN-002 §5;
//! `CODEX/05_PROJECT/SPR-009_Benchmarks.md` T-02b, T-04; Architect ruling on criterion, 2026-10-16.
#![allow(
    missing_docs,
    reason = "criterion's macros generate undocumented items"
)]

use std::fmt::Display;
use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use rh_bench::fixture::{self, CALENDAR, COLUMNS};
use rh_core::world::WorldTables;
use rh_snapshot::hash::{MerkleCache, Plane};
use rh_snapshot::keyframe::{load_engine, read_keyframe, write_keyframe};
use rh_snapshot::state_hash;

/// `docs/08` §9: benches 7 and 8 at 100k and 1M.
const DEFAULT_ROWS: [usize; 2] = [100_000, 1_000_000];

/// Samples per case: RUN-002 §5 asks for at least 5 repeated runs; criterion's minimum is 10.
const SAMPLES: usize = 10;

#[allow(
    clippy::panic,
    reason = "a bench whose setup fails has nothing to measure; it must stop loudly"
)]
fn ok<T, E: Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("bench setup failed: {e}"))
}

#[allow(
    clippy::panic,
    reason = "a malformed RH_WALL_ROWS must stop the run, not measure the wrong size"
)]
#[allow(
    clippy::disallowed_methods,
    reason = "tool plane: command-line arguments and environment overrides (SPR-011 A-4)"
)]
fn sizes() -> Vec<usize> {
    match std::env::var("RH_WALL_ROWS") {
        Ok(list) => list
            .split(',')
            .map(|s| {
                s.trim()
                    .parse()
                    .unwrap_or_else(|e| panic!("RH_WALL_ROWS: {s:?}: {e}"))
            })
            .collect(),
        Err(_) => DEFAULT_ROWS.to_vec(),
    }
}

fn plane_hashes(c: &mut Criterion) {
    let mut g = c.benchmark_group("plane_hashes");
    g.sample_size(SAMPLES);
    for rows in sizes() {
        let state = ok(fixture::engine(rows)).snapshot();
        g.bench_with_input(BenchmarkId::new("state_full", rows), &state, |b, s| {
            b.iter(|| state_hash(black_box(s)));
        });
        g.bench_function(BenchmarkId::new("state_dirty_column", rows), |b| {
            b.iter_batched(
                || dirty_column(rows),
                |(mut cache, tables)| (cache.update_world(black_box(&tables)), cache, tables),
                BatchSize::PerIteration,
            );
        });
        g.bench_function(BenchmarkId::new("state_dirty_chunk", rows), |b| {
            b.iter_batched(
                || dirty_chunk(rows),
                |(mut cache, name, rows)| {
                    cache.update(&name, black_box(rows.as_slice()));
                    (cache.hash(), cache, rows)
                },
                BatchSize::PerIteration,
            );
        });
        for (case, plane, prefix) in [
            ("observation_full", Plane::B, "observation"),
            ("narrative_full", Plane::C, "narrative"),
        ] {
            let tables = fixture::byte_tables(prefix, rows);
            g.bench_with_input(BenchmarkId::new(case, rows), &tables, |b, t| {
                b.iter(|| {
                    let mut cache = MerkleCache::new(plane);
                    for (name, rows) in t {
                        cache.update(name, black_box(rows.as_slice()));
                    }
                    cache.hash()
                });
            });
        }
    }
    g.finish();
}

/// A primed plane-A cache and the world after one of its four columns was replaced.
fn dirty_column(rows: usize) -> (MerkleCache, WorldTables) {
    let mut tables = fixture::tables(rows);
    let mut cache = MerkleCache::new(Plane::A);
    cache.update_world(&tables);
    tables.insert(COLUMNS[1], (0..rows).map(|r| fixture::cell(7, r)).collect());
    (cache, tables)
}

/// A primed plane-A cache over one column, with one row changed and marked.
fn dirty_chunk(rows: usize) -> (MerkleCache, String, Vec<i64>) {
    let name = COLUMNS[0].to_owned();
    let mut column: Vec<i64> = (0..rows).map(|r| fixture::cell(0, r)).collect();
    let mut cache = MerkleCache::new(Plane::A);
    cache.update(&name, column.as_slice());
    column[rows / 2] ^= 1;
    cache.mark_rows(&name, rows / 2..rows / 2 + 1);
    (cache, name, column)
}

fn keyframe_write_read(c: &mut Criterion) {
    let mut g = c.benchmark_group("keyframe_write_read");
    g.sample_size(SAMPLES);
    for rows in sizes() {
        let engine = ok(fixture::engine(rows));
        g.bench_with_input(BenchmarkId::new("write", rows), &engine, |b, e| {
            b.iter(|| ok(write_keyframe(black_box(e), fixture::anchor())));
        });
        let (bytes, _) = ok(write_keyframe(&engine, fixture::anchor()));
        drop(engine);
        let registered = ok(fixture::registered());
        g.bench_with_input(BenchmarkId::new("read", rows), &bytes, |b, k| {
            b.iter(|| ok(read_keyframe(black_box(k), &registered)));
        });
        g.bench_with_input(BenchmarkId::new("load", rows), &bytes, |b, k| {
            b.iter_batched(
                || ok(fixture::registered()),
                |r| ok(load_engine(black_box(k), CALENDAR, r, &fixture::history())),
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

criterion_group!(benches, plane_hashes, keyframe_write_read);
criterion_main!(benches);
