//! T-05 and V-DET-01/02 on a stub world: the derived order and the run are identical across runs
//! and thread counts (SPR-005).
//!
//! The stub world has eight systems over four pulses (Year, Season, Month, Day) and five phases.
//! It is built so that every scheduler rule matters:
//! - a same-instant chain across phases (climate → farm → market) and within a phase
//!   (market → wages in `Exchange`);
//! - siblings in one level (market, tax and trade in `Exchange`) that run on different threads;
//! - a lagged read of a column written in the same phase and level (trade reads wages lagged while
//!   wages is written), which is correct only if the reader sees the start-of-instant value;
//! - keyed RNG draws in every system, a system bound twice in one phase (Season and Year in
//!   `Culture`), and a Day system between the Month ones.
//!
//! V-DET-01 (thread-count invariance): 50 sim-years at 1, 2, 4 and 8 threads; every monthly state
//! hash equal. V-DET-02 (repeat invariance): the same run three times; all equal. The derived
//! order is checked identical across registrations in different input orders.
//!
//! Refs: SPR-005 T-05; `docs/09` V-DET-01, V-DET-02; `05.1` §2 rules 4 and 6, §4.2, §6 item 5.
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
use support::{Stub, agreeing_ownership};

const ROWS: u16 = 64;
const YEARS: i64 = 50;
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
    use Lag::{Lagged, PreviousPulse, SameInstant, StartOfDay};
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
            .reads("wages.w", "X4.wages", PreviousPulse)
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
    let t = agreeing_ownership(&all);
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

/// Runs the stub world for `YEARS` and returns the state hash after every Month instant.
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
    let mut hashes = Vec::new();
    engine
        .advance_until(end, |t, tables| {
            if CALENDAR.is_boundary(Pulse::Month, t) {
                hashes.push(tables.state_hash());
            }
        })
        .unwrap_or_else(|e| panic!("{e}"));
    hashes
}

#[test]
fn the_derived_order_is_identical_across_registrations() {
    let base = registered(|_| {});
    for shuffle in [
        (|v: &mut Vec<Stub>| v.reverse()) as fn(&mut Vec<Stub>),
        |v| v.rotate_left(3),
        |v| v.swap(0, 5),
    ] {
        let other = registered(shuffle);
        assert_eq!(other.ids(), base.ids());
        assert_eq!(other.schedule(), base.schedule());
    }
    // The order the rules imply: in Exchange, market before wages; trade reads wages lagged, so it
    // is free. The canonical order is the levels in sequence, ties by id.
    assert_eq!(
        base.order(Phase::Exchange),
        ["X3.market", "X5.tax", "X6.trade", "X4.wages"]
    );
    let levels: Vec<Vec<&str>> = base
        .schedule()
        .phase(Phase::Exchange)
        .levels
        .iter()
        .map(|l| l.iter().map(|&i| base.ids()[i]).collect())
        .collect();
    assert_eq!(
        levels,
        [vec!["X3.market", "X5.tax", "X6.trade"], vec!["X4.wages"]]
    );
    assert_eq!(base.order(Phase::Culture), ["X8.culture"]);
}

/// V-DET-01: thread-count invariance, monthly hashes over 50 sim-years.
#[test]
fn v_det_01_thread_count_invariance() {
    let one = monthly_hashes(1, 0x5EED);
    assert_eq!(one.len(), usize::try_from(YEARS * 12).unwrap());
    for threads in [2, 4, 8] {
        let many = monthly_hashes(threads, 0x5EED);
        let first = one.iter().zip(&many).position(|(a, b)| a != b);
        assert_eq!(first, None, "{threads} threads diverge at month {first:?}");
        assert_eq!(many.len(), one.len());
    }
    println!(
        "V-DET-01: {} monthly hashes equal at 1, 2, 4 and 8 threads",
        one.len()
    );
}

/// V-DET-02: repeat invariance, three runs of the same binary.
#[test]
fn v_det_02_repeat_invariance() {
    let runs: Vec<Vec<u64>> = (0..3).map(|_| monthly_hashes(4, 0x5EED)).collect();
    assert_eq!(runs[0], runs[1]);
    assert_eq!(runs[1], runs[2]);
    // The hashes carry information: another seed gives another history, and the state changes.
    assert_ne!(monthly_hashes(4, 0x5EEE), runs[0]);
    assert_ne!(runs[0][0], runs[0][1]);
    println!("V-DET-02: 3 runs of {} monthly hashes equal", runs[0].len());
}

/// The lagged read really sees the start of the instant: X6 (trade) reads wages at
/// `previous_pulse`, and wages changes in the same phase. A run where trade instead saw the
/// current value would differ; we check the first month by hand.
#[test]
fn a_lagged_read_sees_the_start_of_the_instant() {
    let mut engine = Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        registered(|_| {}),
        world(),
        SimTime::EPOCH,
    );
    engine.set_threads(1);
    engine.advance_until(SimTime::EPOCH, |_, _| {}).unwrap();
    // At the first instant wages starts at 0, so trade = 0 * 3 + 0 + d = d < 1000 for every row,
    // even though wages is already non-zero by the end of the instant.
    let trade = engine.tables().get("trade.v").unwrap();
    assert!(trade.iter().all(|&v| (0..1_000).contains(&v)), "{trade:?}");
    assert!(
        engine
            .tables()
            .get("wages.w")
            .unwrap()
            .iter()
            .any(|&v| v != 0)
    );
}

#[test]
fn a_draw_under_an_unregistered_purpose_stops_the_run() {
    enum Other {
        Sneaky,
    }
    impl SpecPurpose for Other {
        const ALL: &'static [Self] = &[Self::Sneaky];
        fn name(&self) -> &'static str {
            "sneaky"
        }
    }
    fn sneaky(
        _: &Stub,
        _: PulseBinding,
        _: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        c.rng(&Other::Sneaky, RegionId::from_raw(0), 0).map(|_| ())
    }
    fn undeclared(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        v.read("farm.yield").map(|_| ())
    }
    fn unowned(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        v.write("farm.yield").map(|_| ())
    }
    for (step, expected) in [
        (
            sneaky as support::StepFn,
            "X1.s drew under RNG purpose sneaky, which it did not register",
        ),
        (
            undeclared,
            "X1.s read farm.yield, which its AccessSpec does not declare",
        ),
        (unowned, "X1.s wrote farm.yield, which it does not own"),
    ] {
        let s = Stub::new("X1.s")
            .purposes_of::<Draw>()
            .writes("s.x")
            .at(Pulse::Day, Phase::Vital)
            .stepping(step);
        let t = agreeing_ownership(std::slice::from_ref(&s));
        let r = register(vec![s.boxed()], &t).unwrap();
        let mut engine = Engine::new(
            CALENDAR,
            1,
            BranchSalt::from_raw(0),
            r,
            world(),
            SimTime::EPOCH,
        );
        let err = engine.advance_until(SimTime::EPOCH, |_, _| {}).unwrap_err();
        assert_eq!(err.to_string(), expected);
    }
}

#[test]
fn instants_follow_the_bound_pulses() {
    let s = Stub::new("X1.s")
        .writes("s.x")
        .at(Pulse::Month, Phase::Vital);
    let t = agreeing_ownership(std::slice::from_ref(&s));
    let r = register(vec![s.boxed()], &t).unwrap();
    let start = SimTime::from_minutes(-3 * CALENDAR.minutes_per_year());
    let mut engine = Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        r,
        WorldTables::new(),
        start,
    );
    let mut seen = Vec::new();
    engine
        .advance_until(SimTime::EPOCH, |t, _| seen.push(t))
        .unwrap();
    // Three pre-epoch years of months, then the epoch itself.
    assert_eq!(seen.len(), 37);
    assert!(seen.iter().all(|&t| CALENDAR.is_boundary(Pulse::Month, t)));
    assert_eq!(seen.last(), Some(&SimTime::EPOCH));
    assert_eq!(
        engine.next_instant(),
        CALENDAR.next_boundary(Pulse::Month, SimTime::EPOCH)
    );
}

/// Two systems of one level fail at the same instant: every thread count runs both and reports the
/// first in canonical order, with the same state.
#[test]
fn the_reported_error_is_the_same_at_every_thread_count() {
    fn fail_a(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        v.write("a.x")?.push(1);
        Err(SimError::System {
            system: "X1.a",
            message: "first".to_owned(),
        })
    }
    fn fail_b(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        v.write("b.x")?.push(2);
        Err(SimError::System {
            system: "X2.b",
            message: "second".to_owned(),
        })
    }
    let a = Stub::new("X1.a")
        .writes("a.x")
        .at(Pulse::Day, Phase::Vital)
        .stepping(fail_a);
    let b = Stub::new("X2.b")
        .writes("b.x")
        .at(Pulse::Day, Phase::Vital)
        .stepping(fail_b);
    let mut seen = Vec::new();
    for threads in [1, 2, 4] {
        let t = agreeing_ownership(&[a.clone(), b.clone()]);
        let r = register(vec![b.clone().boxed(), a.clone().boxed()], &t).unwrap();
        let mut engine = Engine::new(
            CALENDAR,
            1,
            BranchSalt::from_raw(0),
            r,
            WorldTables::new(),
            SimTime::EPOCH,
        );
        engine.set_threads(threads);
        let err = engine.advance_until(SimTime::EPOCH, |_, _| {}).unwrap_err();
        seen.push((err.to_string(), engine.tables().state_hash()));
    }
    assert_eq!(seen[0].0, "X1.a: first");
    assert!(seen.iter().all(|s| s == &seen[0]), "{seen:?}");
}
