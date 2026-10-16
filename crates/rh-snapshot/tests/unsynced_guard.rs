//! V-DET-10 on the stub world (SPR-007 T-04): fuzz the engine's read-only APIs under the unsynced
//! guard at every instant of a run; none changes plane A, and the run's hashes equal an unguarded
//! run's. The guard's positive control (an injected write) is `tests/guard.rs`. The stub support in
//! `support/` is a copy of rh-core's `tests/support/` (rh-core's tests stay hash-agnostic).
//!
//! The guard exists only in debug builds (CI's tests), so the file is compiled only there.
//!
//! REF: `docs/09` V-DET-10; `05.1` §6 item 7; SPR-007 T-04, ruling 5.
#![cfg(debug_assertions)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken stub world must fail the test loudly"
)]

mod support;

use rh_core::access::Lag;
use rh_core::engine::Engine;
use rh_core::register::{Registered, register};
use rh_core::system::{PulseBinding, SimError, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_ids::BranchSalt;
use rh_snapshot::guard::read_only;
use rh_snapshot::hash::{MerkleCache, Plane, PlaneDigest, state_hash};
use support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four);
const ROWS: usize = 1500;

#[allow(
    clippy::unnecessary_wraps,
    reason = "it must have the StepFn signature"
)]
fn count(
    _: &Stub,
    _: PulseBinding,
    view: &mut SystemView<'_>,
    ctx: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let day = ctx.now().minutes() / 1440;
    let rows = view.write("count.n")?;
    let i = usize::try_from(day).unwrap() % rows.len();
    rows[i] += day;
    Ok(())
}

fn sum(
    _: &Stub,
    _: PulseBinding,
    view: &mut SystemView<'_>,
    _: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let total: i64 = view.read("count.n")?.iter().sum();
    view.write("sum.s")?[0] = total;
    Ok(())
}

fn registered() -> Registered {
    let stubs = vec![
        Stub::new("count")
            .writes("count.n")
            .at(Pulse::Day, Phase::Exchange)
            .stepping(count),
        Stub::new("sum")
            .writes("sum.s")
            .reads("count.n", "count", Lag::Lagged)
            .at(Pulse::Month, Phase::Culture)
            .stepping(sum),
    ];
    let specs = agreeing_specs(&stubs);
    register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap_or_else(|e| panic!("{e}"))
}

fn engine() -> Engine {
    let mut w = WorldTables::new();
    w.insert("count.n", vec![0; ROWS]);
    w.insert("sum.s", vec![0]);
    Engine::new(
        CALENDAR,
        7,
        BranchSalt::from_raw(0),
        registered(),
        w,
        SimTime::EPOCH,
    )
}

/// `SplitMix64` (Steele, Lea and Flood 2014): picks the read-only calls.
fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One randomly chosen read-only call on the tables, under the guard.
fn fuzz_tables(tables: &WorldTables, cache: &mut MerkleCache, pick: u64) {
    let ok = match pick % 6 {
        0 => read_only("state_hash", tables, |t| {
            let _ = state_hash(t);
        }),
        1 => read_only("digest", tables, |t| {
            let _ = PlaneDigest::of_world(t);
        }),
        2 => read_only("merkle_cache", tables, |t| {
            let _ = cache.update_world(t);
        }),
        3 => read_only("column", tables, |t| {
            let _ = t.get("count.n").map(|r| r.iter().sum::<i64>());
        }),
        4 => read_only("columns", tables, |t| {
            let _: usize = t
                .columns()
                .map(|(n, v, r)| n.len() + r.len() + usize::from(v > 0))
                .sum();
        }),
        _ => read_only("version", tables, |t| {
            let _: u64 = t.names().filter_map(|n| t.version(n)).sum();
        }),
    };
    ok.unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn v_det_10_read_only_apis_never_change_plane_a() {
    let end = SimTime::from_minutes(3 * CALENDAR.minutes_per_year() - 1);
    let mut guarded = engine();
    let mut cache = MerkleCache::new(Plane::A);
    let mut rng = 0x5EED_0010_u64;
    let mut hashes = Vec::new();
    guarded
        .advance_until(end, |_, tables| {
            for _ in 0..3 {
                fuzz_tables(tables, &mut cache, next(&mut rng));
            }
            hashes.push(state_hash(tables));
        })
        .unwrap();
    // The engine's own read-only APIs, under the guard with the engine as the state.
    let e = &guarded;
    read_only("invariants", e, |e| e.invariants().len()).unwrap();
    read_only("next_instant", e, Engine::next_instant).unwrap();
    read_only("registered", e, |e| e.registered().ids()).unwrap();
    read_only("accesses", e, |e| e.registered().accesses().len()).unwrap();
    read_only("tables", e, |e| e.tables().names().count()).unwrap();
    // The cache, updated at random instants of the run, catches up by column versions alone.
    assert_eq!(cache.update_world(e.tables()), state_hash(e.tables()));
    assert!(cache.verify(e.tables()));
    // The guarded run equals an unguarded one, instant for instant.
    let mut plain = engine();
    let mut plain_hashes = Vec::new();
    plain
        .advance_until(end, |_, tables| plain_hashes.push(state_hash(tables)))
        .unwrap();
    assert_eq!(hashes, plain_hashes);
    assert_eq!(hashes.len(), 3 * 365);
    assert_ne!(hashes.first(), hashes.last());
}
