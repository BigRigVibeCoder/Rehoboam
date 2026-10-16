//! Bench 7 of `docs/08` §9, `bench/plane_hashes`: `state_hash`, `observation_hash` and
//! `narrative_hash`, full and dirty-chunk, in instructions (iai-callgrind; SPR-009 T-01, T-02a).
//!
//! Each case's setup builds its input outside the measured function, and each measured function
//! returns its input, so neither construction nor drop is counted: only the hash.
//!
//! - `state_full`: `state_hash` of an engine's whole plane-A state (every chunk of every table).
//! - `state_dirty_column`: a primed plane-A [`MerkleCache`] after one world column of four was
//!   replaced: `update_world` re-hashes that column only.
//! - `state_dirty_chunk`, `observation_dirty_chunk`, `narrative_dirty_chunk`: a primed cache after
//!   one row of one table changed and was marked: one chunk is re-hashed, then the roots.
//! - `observation_full`, `narrative_full`: a fresh plane-B or plane-C cache over its tables.
//!   Planes B and C have no tables yet; theirs are synthetic rows ([`fixture::byte_tables`]).
//!
//! REF: `docs/08_performance_and_capacity.md` §9 bench 7; `docs/09` V-PERF-04;
//! `CODEX/05_PROJECT/SPR-009_Benchmarks.md` T-01, ruling 2.
#![allow(
    missing_docs,
    reason = "the iai-callgrind macros generate undocumented items"
)]

use std::fmt::Display;
use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use rh_bench::fixture::{self, ByteTables, COLUMNS, PR_ROWS};
use rh_core::state::EngineState;
use rh_core::world::WorldTables;
use rh_snapshot::hash::{MerkleCache, Plane};
use rh_snapshot::state_hash;

/// The row each dirty-chunk case changes: inside the table's middle chunk.
const DIRTY_ROW: usize = PR_ROWS / 2;

#[allow(
    clippy::panic,
    reason = "a bench whose setup fails has nothing to measure; it must stop loudly"
)]
fn ok<T, E: Display>(r: Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("bench setup failed: {e}"))
}

/// The full state, and (with the `slowdown-control` feature) the extra tables of V-PERF-04's
/// positive control.
struct Full {
    state: EngineState,
    extra: Option<WorldTables>,
}

fn setup_state(rows: usize) -> Full {
    let state = ok(fixture::engine(rows)).snapshot();
    // V-PERF-04 positive control (SPR-009 T-02): about 15% more instructions, from hashing a world
    // 13% the size of the measured one as well (13% of the world tables is about 15% of the whole
    // state hash, measured). Off in every build but the control's.
    let extra = cfg!(feature = "slowdown-control").then(|| fixture::tables(rows * 13 / 100));
    Full { state, extra }
}

#[library_benchmark]
#[bench::rows_10k(args = (PR_ROWS), setup = setup_state)]
fn state_full(input: Full) -> (u64, Full) {
    let mut h = state_hash(black_box(&input.state));
    if let Some(extra) = &input.extra {
        h ^= rh_snapshot::tables_hash(black_box(extra));
    }
    (h, input)
}

fn setup_dirty_column(rows: usize) -> (MerkleCache, WorldTables) {
    let mut tables = fixture::tables(rows);
    let mut cache = MerkleCache::new(Plane::A);
    cache.update_world(&tables);
    let replaced: Vec<i64> = (0..rows).map(|r| fixture::cell(7, r)).collect();
    tables.insert(COLUMNS[1], replaced);
    (cache, tables)
}

#[library_benchmark]
#[bench::rows_10k(args = (PR_ROWS), setup = setup_dirty_column)]
fn state_dirty_column(input: (MerkleCache, WorldTables)) -> (u64, (MerkleCache, WorldTables)) {
    let (mut cache, tables) = input;
    let h = cache.update_world(black_box(&tables));
    (h, (cache, tables))
}

/// A primed cache over `tables`, then one row of the first table changed and marked.
struct Dirty<R> {
    cache: MerkleCache,
    tables: Vec<(String, Vec<R>)>,
}

fn prime<R>(plane: Plane, mut tables: Vec<(String, Vec<R>)>, change: impl Fn(&mut R)) -> Dirty<R>
where
    [R]: rh_snapshot::hash::HashRows,
{
    let mut cache = MerkleCache::new(plane);
    for (name, rows) in &tables {
        cache.update(name, rows.as_slice());
    }
    let (name, rows) = &mut tables[0];
    change(&mut rows[DIRTY_ROW]);
    cache.mark_rows(name, DIRTY_ROW..DIRTY_ROW + 1);
    Dirty { cache, tables }
}

fn rehash<R>(mut d: Dirty<R>) -> (u64, Dirty<R>)
where
    [R]: rh_snapshot::hash::HashRows,
{
    for (name, rows) in &d.tables {
        d.cache.update(name, black_box(rows.as_slice()));
    }
    (d.cache.hash(), d)
}

fn int_tables(rows: usize) -> Vec<(String, Vec<i64>)> {
    COLUMNS
        .iter()
        .enumerate()
        .map(|(c, name)| {
            (
                (*name).to_owned(),
                (0..rows).map(|r| fixture::cell(c, r)).collect(),
            )
        })
        .collect()
}

fn setup_state_chunk(rows: usize) -> Dirty<i64> {
    prime(Plane::A, int_tables(rows), |v| *v ^= 1)
}

#[library_benchmark]
#[bench::rows_10k(args = (PR_ROWS), setup = setup_state_chunk)]
fn state_dirty_chunk(input: Dirty<i64>) -> (u64, Dirty<i64>) {
    rehash(input)
}

fn fresh(plane: Plane, tables: ByteTables) -> (u64, ByteTables) {
    let mut cache = MerkleCache::new(plane);
    for (name, rows) in &tables {
        cache.update(name, black_box(rows.as_slice()));
    }
    (cache.hash(), tables)
}

fn setup_observation(rows: usize) -> ByteTables {
    fixture::byte_tables("observation", rows)
}

fn setup_narrative(rows: usize) -> ByteTables {
    fixture::byte_tables("narrative", rows)
}

#[library_benchmark]
#[bench::rows_10k(args = (PR_ROWS), setup = setup_observation)]
fn observation_full(tables: ByteTables) -> (u64, ByteTables) {
    fresh(Plane::B, tables)
}

#[library_benchmark]
#[bench::rows_10k(args = (PR_ROWS), setup = setup_narrative)]
fn narrative_full(tables: ByteTables) -> (u64, ByteTables) {
    fresh(Plane::C, tables)
}

fn setup_observation_chunk(rows: usize) -> Dirty<Vec<u8>> {
    prime(Plane::B, setup_observation(rows), |row| row[0] ^= 1)
}

fn setup_narrative_chunk(rows: usize) -> Dirty<Vec<u8>> {
    prime(Plane::C, setup_narrative(rows), |row| row[0] ^= 1)
}

#[library_benchmark]
#[bench::rows_10k(args = (PR_ROWS), setup = setup_observation_chunk)]
fn observation_dirty_chunk(input: Dirty<Vec<u8>>) -> (u64, Dirty<Vec<u8>>) {
    rehash(input)
}

#[library_benchmark]
#[bench::rows_10k(args = (PR_ROWS), setup = setup_narrative_chunk)]
fn narrative_dirty_chunk(input: Dirty<Vec<u8>>) -> (u64, Dirty<Vec<u8>>) {
    rehash(input)
}

library_benchmark_group!(
    name = plane_hashes;
    benchmarks =
        state_full,
        state_dirty_column,
        state_dirty_chunk,
        observation_full,
        observation_dirty_chunk,
        narrative_full,
        narrative_dirty_chunk
);

main!(library_benchmark_groups = plane_hashes);
