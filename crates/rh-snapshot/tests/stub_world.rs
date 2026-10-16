//! The stub-world hash ladder (SPR-005 DEF-6 (e), moved here by SPR-007): the plane-A state hash
//! after each of the first 120 monthly instants of the stub world, against a committed golden.
//!
//! Thread-count and repeat comparisons (V-DET-01/02, in rh-core's `tests/determinism.rs`) cannot
//! see a change that alters every run alike (phase order, scheduler order, pulse order, draw keys);
//! this golden can. It lives in rh-snapshot because it needs `state_hash`, and rh-snapshot depends
//! on rh-core, never the reverse: rh-core's tests stay hash-agnostic (Architect ruling, SPR-007).
//! The stub world below is a copy of rh-core's `tests/determinism.rs` world; `support/` is a copy of
//! rh-core's `tests/support/`.
//!
//! Refs: SPR-005 DEF-6; SPR-007 T-01; `05.1` §6 item 6; `docs/13` §4.2.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken stub world must fail the test loudly"
)]

mod support;

use rh_core::access::Lag;
use rh_core::engine::Engine;
use rh_core::register::{Registered, register};
use rh_core::system::{PulseBinding, SimError, SpecPurpose, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_ids::{BranchSalt, RegionId};
use support::{Stub, agreeing_specs};

const ROWS: u16 = 64;
const YEARS: i64 = 10;
const CALENDAR: Calendar = Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four);

/// The stub world's one RNG purpose, as a generated enum would name it.
enum Draw {
    Noise,
}

impl SpecPurpose for Draw {
    const ALL: &'static [Self] = &[Self::Noise];
    fn name(&self) -> &'static str {
        "noise"
    }
}

fn noise(ctx: &StepCtx<'_>, row: u16, k: u64, modulus: u64) -> Result<i64, SimError> {
    let d = ctx.rng(&Draw::Noise, RegionId::from_raw(row), k)?;
    Ok(i64::try_from(d % modulus).unwrap())
}

/// `out[i] = f(i, inputs...)` over every row; `out` is an owned column.
fn update(
    view: &mut SystemView<'_>,
    ctx: &StepCtx<'_>,
    inputs: &[&str],
    out: &str,
    f: impl Fn(i64, &[i64], i64) -> i64,
) -> Result<(), SimError> {
    let cols: Vec<Vec<i64>> = inputs
        .iter()
        .map(|c| view.read(c).map(<[i64]>::to_vec))
        .collect::<Result<_, _>>()?;
    let mut draws = Vec::with_capacity(usize::from(ROWS));
    for row in 0..ROWS {
        draws.push(noise(ctx, row, 0, 1_000)?);
    }
    let target = view.write(out)?;
    for (i, slot) in target.iter_mut().enumerate() {
        let ins: Vec<i64> = cols.iter().map(|c| c[i]).collect();
        *slot = f(*slot, &ins, draws[i]).rem_euclid(1_000_003);
    }
    Ok(())
}

fn climate(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    update(v, c, &[], "climate.temp", |_, _, d| d)
}
fn farm(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    update(v, c, &["climate.temp"], "farm.yield", |old, x, d| {
        old + x[0] * 3 + d
    })
}
fn market(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    update(v, c, &["farm.yield"], "market.price", |old, x, d| {
        old / 2 + 7 * x[0] + d
    })
}
fn wages(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    update(v, c, &["market.price"], "wages.w", |old, x, d| {
        old + x[0] - d
    })
}
fn tax(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    update(v, c, &["farm.yield"], "tax.t", |old, x, d| {
        old + x[0] / 5 + d
    })
}
fn trade(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    update(v, c, &["wages.w"], "trade.v", |old, x, d| {
        old * 3 + x[0] + d
    })
}
fn daily(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    update(v, c, &["market.price"], "daily.count", |old, x, d| {
        old + d % 3 + x[0] % 2
    })
}
fn culture(
    _: &Stub,
    b: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let weight = if b.pulse == Pulse::Year { 11 } else { 5 };
    update(v, c, &["daily.count"], "culture.c", move |old, x, d| {
        old + weight * x[0] + d
    })
}

fn stubs() -> Vec<Stub> {
    use Lag::{Lagged, SameInstant, StartOfDay};
    let s = |id| Stub::new(id).purposes_of::<Draw>();
    vec![
        s("X1.climate")
            .writes("climate.temp")
            .at(Pulse::Month, Phase::Environment)
            .stepping(climate),
        s("X2.farm")
            .reads("climate.temp", "X1.climate", SameInstant)
            .writes("farm.yield")
            .at(Pulse::Month, Phase::Production)
            .stepping(farm),
        s("X3.market")
            .reads("farm.yield", "X2.farm", SameInstant)
            .writes("market.price")
            .at(Pulse::Month, Phase::Exchange)
            .stepping(market),
        s("X4.wages")
            .reads("market.price", "X3.market", SameInstant)
            .writes("wages.w")
            .at(Pulse::Month, Phase::Exchange)
            .stepping(wages),
        s("X5.tax")
            .reads("farm.yield", "X2.farm", SameInstant)
            .writes("tax.t")
            .at(Pulse::Month, Phase::Exchange)
            .stepping(tax),
        s("X6.trade")
            .reads("wages.w", "X4.wages", Lagged) // SPR-006: as in rh-core's determinism.rs
            .writes("trade.v")
            .at(Pulse::Month, Phase::Exchange)
            .stepping(trade),
        s("X7.daily")
            .reads("market.price", "X3.market", StartOfDay)
            .writes("daily.count")
            .at(Pulse::Day, Phase::Vital)
            .stepping(daily),
        s("X8.culture")
            .reads("daily.count", "X7.daily", SameInstant)
            .writes("culture.c")
            .at(Pulse::Season, Phase::Culture)
            .at(Pulse::Year, Phase::Culture)
            .stepping(culture),
    ]
    .into_iter()
    .map(|stub| {
        if stub.id == "X6.trade" {
            stub.reads("tax.t", "X5.tax", Lagged)
        } else {
            stub
        }
    })
    .collect()
}

fn registered(order: impl Fn(&mut Vec<Stub>)) -> Registered {
    let mut all = stubs();
    let t = agreeing_specs(&all);
    order(&mut all);
    register(all.into_iter().map(Stub::boxed).collect(), &t).unwrap_or_else(|e| panic!("{e}"))
}

fn world() -> WorldTables {
    let mut w = WorldTables::new();
    for c in [
        "climate.temp",
        "farm.yield",
        "market.price",
        "wages.w",
        "tax.t",
        "trade.v",
        "daily.count",
        "culture.c",
    ] {
        w.insert(c, vec![0; usize::from(ROWS)]);
    }
    w
}

/// Runs the stub world for `YEARS` and returns the plane-A state hash after every Month instant.
fn monthly_hashes(threads: usize, seed: u64) -> Vec<u64> {
    let mut engine = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered(|_| {}),
        world(),
        SimTime::EPOCH,
    );
    engine.set_threads(threads);
    let end = SimTime::from_minutes(YEARS * CALENDAR.minutes_per_year() - 1);
    month_end_hashes(&mut engine, end)
}

/// Runs `engine` to `end` one instant at a time and returns `state_hash` of its whole snapshot
/// (world and engine tables, SPR-007 T-02) after every Month instant.
fn month_end_hashes(engine: &mut Engine, end: SimTime) -> Vec<u64> {
    let mut hashes = Vec::new();
    while let Some(t) = engine.next_instant().filter(|&t| t <= end) {
        engine
            .advance_until(t, |_, _| {})
            .unwrap_or_else(|e| panic!("{e}"));
        if CALENDAR.is_boundary(Pulse::Month, t) {
            hashes.push(rh_snapshot::state_hash(&engine.snapshot()));
        }
    }
    hashes
}

/// The committed golden of the stub world's first 120 monthly hashes (`tests/golden/`).
const STUB_GOLDEN: &str = "tests/golden/stub_world_hashes.txt";

fn render_stub_golden(ladder: &[u64]) -> String {
    let mut out = String::from(
        "# rh-snapshot stub-world hash ladder: the plane-A state hash of the engine's snapshot\n\
         # (rh_snapshot::state_hash: world and engine tables, layout rehoboam/rh-snapshot/hash/v1;\n\
         # SPR-007 T-01, T-02) after each of the first 120 monthly instants of\n\
         # the stub world of crates/rh-snapshot/tests/stub_world.rs (seed 0x5EED, salt 0, 1 thread).\n\
         # GENERATED by `RH_SNAPSHOT_WRITE_STUB_GOLDEN=1 cargo test -p rh-snapshot --test stub_world`;\n\
         # never edited by hand. A change here is a change of phase order, scheduler order, pulse\n\
         # order, draw keys or the hash layout: a C3 change (docs/13 §4.2).\n\
         # Format: <month index from 0> <state hash, decimal u64>\n",
    );
    for (i, hash) in ladder.iter().enumerate() {
        out.push_str(&i.to_string());
        out.push(' ');
        out.push_str(&hash.to_string());
        out.push('\n');
    }
    out
}

/// DEF-6 (e): the stub world's monthly hashes against a committed golden. Thread-count and repeat
/// comparisons cannot see a change that alters every run alike (phase order, scheduler order, pulse
/// order, draw keys); this golden can.
#[test]
fn the_stub_world_hash_ladder_matches_its_golden() {
    let ladder: Vec<u64> = monthly_hashes(1, 0x5EED).into_iter().take(120).collect();
    let text = render_stub_golden(&ladder);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(STUB_GOLDEN);
    if std::env::var_os("RH_SNAPSHOT_WRITE_STUB_GOLDEN").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{STUB_GOLDEN}: {e}"));
    let first = committed
        .lines()
        .zip(text.lines())
        .position(|(a, b)| a != b);
    assert_eq!(
        first, None,
        "{STUB_GOLDEN} differs at line {first:?}: a C3 change of the run"
    );
    assert_eq!(committed.lines().count(), text.lines().count());
}

/// Two runs of the stub world build their tables as different instances (different identities,
/// VER-SPR-007 DEF-1) and still hash equal at every month: the identity never reaches a hash.
#[test]
fn two_runs_hash_equal_despite_different_instance_identities() {
    let run = || {
        let mut engine = Engine::new(
            CALENDAR,
            0x5EED,
            BranchSalt::from_raw(0),
            registered(|_| {}),
            world(),
            SimTime::EPOCH,
        );
        let end = SimTime::from_minutes(CALENDAR.minutes_per_year() - 1);
        let hashes = month_end_hashes(&mut engine, end);
        (engine.tables().identity(), hashes)
    };
    let (id_a, a) = run();
    let (id_b, b) = run();
    assert_ne!(id_a, id_b);
    assert_eq!(a.len(), 12);
    assert_eq!(a, b);
}
