//! The engine against its contract, by mutation guards and a serial oracle (SPR-005 DEF-6).
//!
//! The Tester's mutation matrix found breakages that the differential tests (1 vs N threads, run
//! vs run) cannot see, because they change every run alike. Each test here pins one rule against
//! an independent expectation:
//!
//! - (a) a lagged read sees the start-of-instant value (`05.1` §2 rule 4, P0 approximation);
//! - (b) `StepCtx::rng` keys on the current instant: a draw equals `rh_ids::h` computed directly;
//! - (c) when several systems of one level fail, the canonical-first error is reported at 1, 2, 4
//!   and 8 threads;
//! - (d) within a level, ties go coarser pulse first; and (DEF-7) a same-instant dependency between
//!   bindings of different pulses always holds, across up to three pulses;
//! - the serial oracle: a plain serial reference computes order, lags and draws for a tiny world,
//!   and the engine must match it month by month at 1 and 4 threads;
//! - (SPR-008) `StepCtx::stream` has `StepCtx::rng`'s key: its draw `k` equals `rh_ids::h` with
//!   draw index `k`; a step gets one stream per `(purpose, entity)`, and never a stream and `rng`
//!   draws for the same pair, in either order (`05.1` §6 item 3).
//!
//! Refs: SPR-005 T-05, DEF-6; `05.1` §2 rules 4 and 6, §6 items 2 and 3; SPR-008 (Architect ruling
//! on `StepCtx::stream`).
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken fixture must fail the test loudly"
)]

mod support;

use rh_core::access::Lag;
use rh_core::engine::Engine;
use rh_core::fixed::{Prob32, Q16_16, draw_hits};
use rh_core::register::{register, system_id};
use rh_core::system::{PulseBinding, SimError, SpecPurpose, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_ids::{BranchSalt, PurposeId, RegionId, RngKey, h};
use support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Two);
const SEED: u64 = 0x00C0_FFEE;
const SALT: u64 = 7;
const ROWS: u16 = 4;

enum Draw {
    Noise,
}

impl SpecPurpose for Draw {
    const ALL: &'static [Self] = &[Self::Noise];
    fn name(&self) -> &'static str {
        "noise"
    }
}

/// `H(seed, salt, system_id(system), 0, "noise", row, t, 0)`, computed without `StepCtx`.
fn independent_draw(system: &str, row: u16, t: SimTime) -> u64 {
    independent_draw_k(system, row, t, 0)
}

/// `H(seed, salt, system_id(system), 0, "noise", row, t, k)`, computed without `StepCtx`.
fn independent_draw_k(system: &str, row: u16, t: SimTime, k: u64) -> u64 {
    h(
        &RngKey {
            world_seed: SEED,
            branch_salt: BranchSalt::from_raw(SALT),
            system: system_id(system),
            purpose: PurposeId::of("noise"),
            entity: u64::from(row),
            epoch: 0,
            t: t.minutes(),
        },
        k,
    )
}

fn engine(stubs: Vec<Stub>, start: SimTime, threads: usize) -> Engine {
    let t = agreeing_specs(&stubs);
    let r = register(stubs.into_iter().map(Stub::boxed).collect(), &t)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut e = Engine::new(
        CALENDAR,
        SEED,
        BranchSalt::from_raw(SALT),
        r,
        WorldTables::new(),
        start,
    );
    e.set_threads(threads);
    e
}

fn draw_mod_100(c: &StepCtx<'_>, row: u16) -> Result<i64, SimError> {
    Ok(i64::try_from(c.rng(&Draw::Noise, RegionId::from_raw(row), 0)? % 100).unwrap())
}

// The tiny oracle world, all at Month:
//   X1.a (Production): a[i] = H(.., i, t, 0) % 100
//   X2.b (Exchange):   b[i] += a[i]                 (reads a at same_instant)
//   X3.c (Exchange):   c[i] = 2 * b_start[i] + a[i] (reads b lagged, a at same_instant)
fn step_a(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let rows = v.write("a.v")?;
    rows.resize(usize::from(ROWS), 0);
    for row in 0..ROWS {
        rows[usize::from(row)] = draw_mod_100(c, row)?;
    }
    Ok(())
}
fn step_b(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    _: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let a = v.read("a.v")?.to_vec();
    let b = v.write("b.v")?;
    b.resize(a.len(), 0);
    for (x, y) in b.iter_mut().zip(&a) {
        *x += y;
    }
    Ok(())
}
fn step_c(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    _: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let a = v.read("a.v")?.to_vec();
    let b = v.read("b.v")?.to_vec();
    let c = v.write("c.v")?;
    c.resize(a.len(), 0);
    for (i, x) in c.iter_mut().enumerate() {
        *x = 2 * b.get(i).copied().unwrap_or(0) + a[i];
    }
    Ok(())
}

fn oracle_world() -> Vec<Stub> {
    let s = |id| Stub::new(id).purposes_of::<Draw>();
    vec![
        s("X1.a")
            .writes("a.v")
            .at(Pulse::Month, Phase::Production)
            .stepping(step_a),
        s("X2.b")
            .writes("b.v")
            .reads("a.v", "X1.a", Lag::SameInstant)
            .at(Pulse::Month, Phase::Exchange)
            .stepping(step_b),
        s("X3.c")
            .writes("c.v")
            .reads("a.v", "X1.a", Lag::SameInstant)
            .reads("b.v", "X2.b", Lag::Lagged)
            .at(Pulse::Month, Phase::Exchange)
            .stepping(step_c),
    ]
}

/// The serial oracle (the Tester's model): the expected tables after each month, computed by a
/// plain loop with the draws from `rh_ids::h` directly. It covers (a) and (b) as well: a lagged read
/// that saw the current value, or a draw keyed on the wrong instant, diverges at the first month.
#[test]
fn the_engine_matches_a_serial_oracle() {
    let start = CALENDAR
        .from_date(rh_core::time::Date {
            year: -1,
            month: 11,
            day: 1,
            hour: 0,
            minute: 0,
        })
        .unwrap();
    for threads in [1, 4] {
        let mut e = engine(oracle_world(), start, threads);
        let (mut b, mut c) = (
            vec![0_i64; usize::from(ROWS)],
            vec![0_i64; usize::from(ROWS)],
        );
        let mut months = 0;
        let end = SimTime::from_minutes(2 * CALENDAR.minutes_per_year() - 1);
        e.advance_until(end, |t, tables| {
            let a: Vec<i64> = (0..ROWS)
                .map(|row| i64::try_from(independent_draw("X1.a", row, t) % 100).unwrap())
                .collect();
            let b_start = b.clone();
            for (x, y) in b.iter_mut().zip(&a) {
                *x += y;
            }
            for i in 0..c.len() {
                c[i] = 2 * b_start[i] + a[i];
            }
            assert_eq!(
                tables.get("a.v").unwrap(),
                &a,
                "{threads} threads, {t}: a (draws)"
            );
            assert_eq!(
                tables.get("b.v").unwrap(),
                &b,
                "{threads} threads, {t}: b (same_instant)"
            );
            assert_eq!(
                tables.get("c.v").unwrap(),
                &c,
                "{threads} threads, {t}: c (lagged)"
            );
            months += 1;
        })
        .unwrap();
        assert_eq!(months, 26, "Nov and Dec of year -1, then two years");
    }
}

/// (a) A lagged read sees the start-of-instant value even when the writer ran earlier in the same
/// instant (an earlier phase), where a `same_instant` read would see the new value.
#[test]
fn a_lagged_read_ignores_this_instants_write() {
    fn mark(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        *v.write("w.x")? = vec![c.now().minutes()];
        Ok(())
    }
    fn copy(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let seen = v.read("w.x")?.to_vec();
        *v.write("r.x")? = seen;
        Ok(())
    }
    let w = Stub::new("X1.w")
        .writes("w.x")
        .at(Pulse::Day, Phase::Production)
        .stepping(mark);
    let r = Stub::new("X2.r")
        .writes("r.x")
        .reads("w.x", "X1.w", Lag::Lagged)
        .at(Pulse::Day, Phase::Polity)
        .stepping(copy);
    let mut e = engine(vec![w, r], SimTime::EPOCH, 1);
    let day = SimTime::from_minutes(1_440);
    e.advance_until(day, |_, _| {}).unwrap();
    assert_eq!(
        e.tables().get("w.x").unwrap(),
        &vec![1_440],
        "the writer marked day 1"
    );
    assert_eq!(
        e.tables().get("r.x").unwrap(),
        &vec![0],
        "the reader saw day 0's mark"
    );
}

/// (b) `StepCtx::rng` keys on the current instant: each day's draw equals `rh_ids::h` at that day.
#[test]
fn draws_are_keyed_on_the_current_instant() {
    fn record(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let d = c.rng(&Draw::Noise, RegionId::from_raw(3), 0)?;
        v.write("d.x")?.push(d.cast_signed());
        Ok(())
    }
    let s = Stub::new("X1.d")
        .purposes_of::<Draw>()
        .writes("d.x")
        .at(Pulse::Day, Phase::Vital)
        .stepping(record);
    let start = SimTime::from_minutes(-2 * 1_440);
    let mut e = engine(vec![s], start, 1);
    e.advance_until(SimTime::from_minutes(1_440), |_, _| {})
        .unwrap();
    let expected: Vec<i64> = (-2..=1)
        .map(|d| independent_draw("X1.d", 3, SimTime::from_minutes(d * 1_440)).cast_signed())
        .collect();
    assert_eq!(e.tables().get("d.x").unwrap(), &expected);
    assert_ne!(
        expected[0], expected[1],
        "different instants, different draws"
    );
}

/// (c) Three systems in one level, the second and third failing: the canonical-first failure (the
/// second) is reported at every thread count. With two threads the workers hold jobs [0, 2] and
/// [1], so an engine that did not re-sort the outcomes would report the third.
#[test]
fn the_canonical_first_error_wins_at_every_thread_count() {
    fn ok(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        v.write("a.x")?.push(1);
        Ok(())
    }
    fn fail_b(
        _: &Stub,
        _: PulseBinding,
        _: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        Err(SimError::System {
            system: "X2.b",
            message: "b failed".to_owned(),
        })
    }
    fn fail_c(
        _: &Stub,
        _: PulseBinding,
        _: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        Err(SimError::System {
            system: "X3.c",
            message: "c failed".to_owned(),
        })
    }
    let at = |s: Stub| s.at(Pulse::Day, Phase::Vital);
    for threads in [1, 2, 4, 8] {
        let stubs = vec![
            at(Stub::new("X1.a").writes("a.x").stepping(ok)),
            at(Stub::new("X2.b").writes("b.x").stepping(fail_b)),
            at(Stub::new("X3.c").writes("c.x").stepping(fail_c)),
        ];
        let mut e = engine(stubs, SimTime::EPOCH, threads);
        let err = e.advance_until(SimTime::EPOCH, |_, _| {}).unwrap_err();
        assert_eq!(err.to_string(), "X2.b: b failed", "{threads} threads");
        assert_eq!(
            e.tables().get("a.x").unwrap(),
            &vec![1],
            "{threads} threads: the whole level ran"
        );
    }
}

/// (d) Ties within a level go coarser pulse first (`05.1` §2 rule 4). A (Month and Day) and B
/// (Month) are independent, so at a month start all three bindings share level 0, in the order
/// A/Month, B/Month, A/Day. A/Day and B/Month both fail: B's error is reported, and A/Day never ran.
#[test]
fn on_a_tie_the_coarser_pulse_runs_first() {
    fn a_step(
        _: &Stub,
        b: PulseBinding,
        v: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        v.write("a.log")?
            .push(if b.pulse == Pulse::Month { 30 } else { 1 });
        if b.pulse == Pulse::Day {
            return Err(SimError::System {
                system: "X1.a",
                message: "day step".to_owned(),
            });
        }
        Ok(())
    }
    fn b_step(
        _: &Stub,
        _: PulseBinding,
        _: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        Err(SimError::System {
            system: "X2.b",
            message: "month step".to_owned(),
        })
    }
    for threads in [1, 4] {
        let a = Stub::new("X1.a")
            .writes("a.log")
            .at(Pulse::Month, Phase::Vital)
            .at(Pulse::Day, Phase::Vital)
            .stepping(a_step);
        let b = Stub::new("X2.b")
            .writes("b.x")
            .at(Pulse::Month, Phase::Vital)
            .stepping(b_step);
        let mut e = engine(vec![a, b], SimTime::EPOCH, threads);
        assert_eq!(
            e.registered().bindings(Phase::Vital),
            [
                ("X1.a", Pulse::Month),
                ("X2.b", Pulse::Month),
                ("X1.a", Pulse::Day)
            ]
        );
        let err = e.advance_until(SimTime::EPOCH, |_, _| {}).unwrap_err();
        assert_eq!(err.to_string(), "X2.b: month step", "{threads} threads");
        assert_eq!(
            e.tables().get("a.log").unwrap(),
            &vec![30],
            "{threads} threads: A/Day did not run"
        );
    }
}

fn mark_now(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let col = if v.write("w.c").is_ok() { "w.c" } else { "h.c" };
    *v.write(col)? = vec![c.now().minutes()];
    Ok(())
}

/// DEF-7, the Tester's case: W (Day) writes `w.c`, R (Month) reads it at `same_instant`, both in
/// Vital. The dependency is the law, so at a month start R runs after W and sees this instant's
/// write, not the previous day's.
#[test]
fn a_coarser_reader_sees_a_finer_writer_of_the_same_instant() {
    fn copy(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let seen = v.read("w.c")?[0];
        v.write("r.seen")?.push(seen);
        Ok(())
    }
    for threads in [1, 4] {
        let w = Stub::new("X1.w")
            .writes("w.c")
            .at(Pulse::Day, Phase::Vital)
            .stepping(mark_now);
        let r = Stub::new("X2.r")
            .writes("r.seen")
            .reads("w.c", "X1.w", Lag::SameInstant)
            .at(Pulse::Month, Phase::Vital)
            .stepping(copy);
        let mut e = engine(vec![w, r], SimTime::EPOCH, threads);
        assert_eq!(
            e.registered().bindings(Phase::Vital),
            [("X1.w", Pulse::Day), ("X2.r", Pulse::Month)]
        );
        e.advance_until(SimTime::from_minutes(43_200), |_, _| {})
            .unwrap();
        assert_eq!(
            e.tables().get("r.seen").unwrap(),
            &vec![0, 43_200],
            "{threads} threads"
        );
    }
}

/// DEF-7: edges across three pulses in one phase, Hour -> Day -> Month. At a month start the three
/// bindings run in dependency order, finest first, and the month value is this instant's.
#[test]
fn edges_across_three_pulses_run_in_dependency_order() {
    fn relay(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        _: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let (from, to) = if v.write("d.c").is_ok() {
            ("h.c", "d.c")
        } else {
            ("d.c", "m.c")
        };
        let seen = v.read(from)?[0];
        *v.write(to)? = vec![seen];
        Ok(())
    }
    let hour = Stub::new("X1.hour")
        .writes("h.c")
        .at(Pulse::Hour, Phase::Vital)
        .stepping(mark_now);
    let day = Stub::new("X2.day")
        .writes("d.c")
        .reads("h.c", "X1.hour", Lag::SameInstant)
        .at(Pulse::Day, Phase::Vital)
        .stepping(relay);
    let month = Stub::new("X3.month")
        .writes("m.c")
        .reads("d.c", "X2.day", Lag::SameInstant)
        .at(Pulse::Month, Phase::Vital)
        .stepping(relay);
    let mut e = engine(vec![month, day, hour], SimTime::EPOCH, 2);
    assert_eq!(
        e.registered().bindings(Phase::Vital),
        [
            ("X1.hour", Pulse::Hour),
            ("X2.day", Pulse::Day),
            ("X3.month", Pulse::Month)
        ]
    );
    e.advance_until(SimTime::from_minutes(43_200), |_, _| {})
        .unwrap();
    for col in ["h.c", "d.c", "m.c"] {
        assert_eq!(e.tables().get(col).unwrap(), &vec![43_200], "{col}");
    }
}

/// SPR-008: a stream's draw `k` is `rng`'s draw `k` (the same key), and it starts at draw 0.
#[test]
fn a_stream_has_the_key_of_rng() {
    fn record(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let mut s = c.stream(&Draw::Noise, RegionId::from_raw(3))?;
        let mut out: Vec<i64> = (0..3).map(|_| s.next_u64().cast_signed()).collect();
        out.push(s.draw(9).cast_signed());
        v.write("d.x")?.extend(out);
        Ok(())
    }
    let s = Stub::new("X1.d")
        .purposes_of::<Draw>()
        .writes("d.x")
        .at(Pulse::Day, Phase::Vital)
        .stepping(record);
    let t = SimTime::from_minutes(1_440);
    let mut e = engine(vec![s], t, 1);
    e.advance_until(t, |_, _| {}).unwrap();
    let expected: Vec<i64> = [0, 1, 2, 9]
        .map(|k| independent_draw_k("X1.d", 3, t, k).cast_signed())
        .to_vec();
    assert_eq!(e.tables().get("d.x").unwrap(), &expected);
}

/// SPR-011 T-01: `chance` is one keyed draw against a probability: `draw_hits(rng(p, e, k), q)`,
/// for `Prob32` and `Q16_16` alike, at the edges 0 and 1 and in between.
#[test]
fn chance_is_one_keyed_draw_against_the_probability() {
    const PROBS: [u64; 6] = [0, 1, 1 << 31, (1 << 32) - 1, 1 << 32, 2_863_311_531];
    fn record(
        _: &Stub,
        _: PulseBinding,
        v: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let mut out = Vec::new();
        for (k, bits) in (0_u64..).zip(PROBS) {
            let p = Prob32::from_bits(bits).expect("a probability");
            out.push(i64::from(c.chance(
                &Draw::Noise,
                RegionId::from_raw(3),
                k,
                p,
            )?));
        }
        let q = Q16_16::from_bits(40_000);
        out.push(i64::from(c.chance(
            &Draw::Noise,
            RegionId::from_raw(3),
            6,
            q,
        )?));
        v.write("d.x")?.extend(out);
        Ok(())
    }
    let s = Stub::new("X1.d")
        .purposes_of::<Draw>()
        .writes("d.x")
        .at(Pulse::Day, Phase::Vital)
        .stepping(record);
    let t = SimTime::from_minutes(1_440);
    let mut e = engine(vec![s], t, 1);
    e.advance_until(t, |_, _| {}).unwrap();
    let mut expected: Vec<i64> = (0_u64..)
        .zip(PROBS)
        .map(|(k, bits)| {
            let p = Prob32::from_bits(bits).expect("a probability");
            i64::from(draw_hits(independent_draw_k("X1.d", 3, t, k), p))
        })
        .collect();
    let q = Q16_16::from_bits(40_000);
    expected.push(i64::from(draw_hits(independent_draw_k("X1.d", 3, t, 6), q)));
    assert_eq!(e.tables().get("d.x").unwrap(), &expected);
    assert_eq!(expected[0], 0, "p = 0 never hits");
    assert_eq!(expected[4], 1, "p = 1 always hits");
}

/// SPR-008 (`05.1` §6 item 3): one stream per `(purpose, entity)` per step, never alongside
/// `rng` for the same pair; another entity is a different pair. Each refusal names the system,
/// the purpose and the entity.
#[test]
fn a_step_takes_one_stream_per_pair_and_never_mixes_it_with_rng() {
    type Use = fn(&StepCtx<'_>) -> Result<(), SimError>;
    let cases: [(&str, Use, Option<&str>); 5] = [
        (
            "stream twice",
            |c| {
                c.stream(&Draw::Noise, RegionId::from_raw(3))
                    .and_then(|_| c.stream(&Draw::Noise, RegionId::from_raw(3)))
                    .map(|_| ())
            },
            Some("stream"),
        ),
        (
            "rng then stream",
            |c| {
                c.rng(&Draw::Noise, RegionId::from_raw(3), 0)
                    .and_then(|_| c.stream(&Draw::Noise, RegionId::from_raw(3)))
                    .map(|_| ())
            },
            Some("rng"),
        ),
        (
            "stream then rng",
            |c| {
                c.stream(&Draw::Noise, RegionId::from_raw(3))
                    .and_then(|_| c.rng(&Draw::Noise, RegionId::from_raw(3), 0))
                    .map(|_| ())
            },
            Some("stream"),
        ),
        (
            "two entities",
            |c| {
                c.stream(&Draw::Noise, RegionId::from_raw(3))
                    .and_then(|_| c.stream(&Draw::Noise, RegionId::from_raw(4)))
                    .map(|_| ())
            },
            None,
        ),
        (
            "rng twice",
            |c| {
                c.rng(&Draw::Noise, RegionId::from_raw(3), 0)
                    .and_then(|_| c.rng(&Draw::Noise, RegionId::from_raw(3), 1))
                    .map(|_| ())
            },
            None,
        ),
    ];
    thread_local! {
        static USE: std::cell::Cell<Option<Use>> = const { std::cell::Cell::new(None) };
    }
    fn step(
        _: &Stub,
        _: PulseBinding,
        _: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        USE.with(|u| u.get().expect("a use is set"))(c)
    }
    for (name, f, refused) in cases {
        USE.with(|u| u.set(Some(f)));
        let s = Stub::new("X1.d")
            .purposes_of::<Draw>()
            .at(Pulse::Day, Phase::Vital)
            .stepping(step);
        let t = SimTime::from_minutes(1_440);
        let mut e = engine(vec![s], t, 1);
        let got = e.advance_until(t, |_, _| {});
        match refused {
            None => assert_eq!(got, Ok(()), "{name}"),
            Some(taken) => assert_eq!(
                got,
                Err(SimError::StreamReuse {
                    system: "X1.d",
                    purpose: "noise",
                    entity: 3,
                    taken,
                }),
                "{name}"
            ),
        }
    }
}

/// BCK-002 A-37: the pair set spans one system's bindings at one instant. Bound in Vital and in
/// Agenda on the same day, the second binding's stream of the same pair is refused (the two would
/// draw identical numbers); a new instant starts with no pair taken, so a system bound once draws
/// the same pair's stream every day.
#[test]
fn the_taken_pairs_span_every_binding_of_an_instant_and_reset_with_each_instant() {
    fn step(
        _: &Stub,
        _: PulseBinding,
        _: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        c.stream(&Draw::Noise, RegionId::from_raw(3)).map(|_| ())
    }
    let t = SimTime::from_minutes(1_440);
    let twice = Stub::new("X1.d")
        .purposes_of::<Draw>()
        .at(Pulse::Day, Phase::Vital)
        .at(Pulse::Day, Phase::Agenda)
        .stepping(step);
    let mut e = engine(vec![twice], t, 1);
    assert_eq!(
        e.advance_until(SimTime::from_minutes(3 * 1_440), |_, _| {}),
        Err(SimError::StreamReuse {
            system: "X1.d",
            purpose: "noise",
            entity: 3,
            taken: "stream",
        })
    );
    assert_eq!(e.crash_artifact().map(|a| a.tick), Some(1_440));
    let once = Stub::new("X1.d")
        .purposes_of::<Draw>()
        .at(Pulse::Day, Phase::Vital)
        .stepping(step);
    let mut e = engine(vec![once], t, 1);
    assert_eq!(
        e.advance_until(SimTime::from_minutes(3 * 1_440), |_, _| {}),
        Ok(())
    );
}

/// BCK-002 A-37: a timer pass and a step of one system at one instant share the pair set. The
/// timer fires in Vital and takes the stream; the Day step in Agenda asks for the same pair.
#[test]
fn a_timer_pass_and_a_step_at_one_instant_share_the_pair_set() {
    use rh_core::agenda::TimerEntry;
    use rh_core::system::TimerSpec;
    use rh_ids::EntityRef;
    fn step(
        _: &Stub,
        b: PulseBinding,
        _: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        if b.phase == Phase::Vital && c.now() == SimTime::EPOCH {
            let time = c.now().checked_add_minutes(1_440).unwrap();
            let entity = EntityRef::region(RegionId::from_raw(3));
            let spec = TimerSpec {
                time,
                phase: Phase::Vital,
                entity,
                kind: 0,
                payload: 0,
                cause: rh_events::EventRef::None,
            };
            c.schedule(&Draw::Noise, spec)?;
        }
        if b.phase == Phase::Agenda && c.now() > SimTime::EPOCH {
            c.stream(&Draw::Noise, RegionId::from_raw(3))?;
        }
        Ok(())
    }
    fn fire(
        _: &Stub,
        _: &TimerEntry,
        _: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        c.stream(&Draw::Noise, RegionId::from_raw(3)).map(|_| ())
    }
    let s = Stub::new("X1.d")
        .purposes_of::<Draw>()
        .at(Pulse::Day, Phase::Vital)
        .at(Pulse::Day, Phase::Agenda)
        .stepping(step)
        .firing(fire);
    let mut e = engine(vec![s], SimTime::EPOCH, 1);
    assert_eq!(
        e.advance_until(SimTime::from_minutes(2 * 1_440), |_, _| {}),
        Err(SimError::StreamReuse {
            system: "X1.d",
            purpose: "noise",
            entity: 3,
            taken: "stream",
        })
    );
    assert_eq!(e.crash_artifact().map(|a| a.tick), Some(1_440));
}

/// SPR-008 (Architect ruling on D3): the pair set spans every fire of one system in one timer
/// pass. Two timers due in the same slot that ask for the same `(purpose, entity)` stream would
/// draw identical numbers, so the second fire is refused (one slot fires together, SPR-006 G2).
#[test]
fn the_fires_of_one_timer_pass_share_the_pair_set() {
    use rh_core::agenda::TimerEntry;
    use rh_core::system::TimerSpec;
    use rh_ids::EntityRef;
    fn schedule_two(
        _: &Stub,
        _: PulseBinding,
        _: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let time = c.now().checked_add_minutes(1_440).unwrap();
        for (row, payload) in [(0_u16, 0_i64), (1, 1)] {
            c.schedule(
                &Draw::Noise,
                TimerSpec {
                    time,
                    phase: Phase::Vital,
                    entity: EntityRef::region(RegionId::from_raw(row)),
                    kind: 0,
                    payload,
                    cause: rh_events::EventRef::None,
                },
            )?;
        }
        Ok(())
    }
    fn fire(
        _: &Stub,
        _: &TimerEntry,
        _: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        c.stream(&Draw::Noise, RegionId::from_raw(3)).map(|_| ())
    }
    let s = Stub::new("X1.d")
        .purposes_of::<Draw>()
        .at(Pulse::Year, Phase::Vital)
        .stepping(schedule_two)
        .firing(fire);
    let mut e = engine(vec![s], SimTime::EPOCH, 1);
    assert_eq!(
        e.advance_until(SimTime::from_minutes(2 * 1_440), |_, _| {}),
        Err(SimError::StreamReuse {
            system: "X1.d",
            purpose: "noise",
            entity: 3,
            taken: "stream",
        })
    );
    assert_eq!(e.crash_artifact().map(|a| a.tick), Some(1_440));
}

// --- BCK-002 A-50 (SPR-014 T-07): the entity kind of a draw --------------------------------

/// The purposes of the kind-check stub: one keyed by a person, one by the world, one by a title
/// (a key name with no `EntityKind` yet).
enum Kp {
    Person,
    World,
    Title,
}

impl SpecPurpose for Kp {
    const ALL: &'static [Self] = &[Self::Person, Self::World, Self::Title];
    fn name(&self) -> &'static str {
        match self {
            Self::Person => "by_person",
            Self::World => "by_world",
            Self::Title => "by_title",
        }
    }
}

fn draws(
    _: &support::Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let person = rh_ids::PersonId::from_raw(7);
    let _ = c.rng(&Kp::Person, person, 0)?;
    let _ = c.rng(&Kp::World, rh_ids::WorldKey::WORLD, 0)?;
    // A title has no EntityKind yet: not kind-checked, so any key draws.
    let _ = c.rng(&Kp::Title, RegionId::from_raw(3), 0)?;
    if v.read("kind.case")?[0] == 1 {
        let _ = c.rng(&Kp::Person, RegionId::from_raw(3), 1)?;
    }
    if v.read("kind.case")?[0] == 2 {
        let _ = c.stream(&Kp::World, person)?;
    }
    Ok(())
}

fn kind_run(case: i64) -> (Result<(), SimError>, Vec<String>) {
    let stub = support::Stub::new("X1.kinds")
        .writes("kind.case")
        .purposes_of::<Kp>()
        .rng_key("by_person", "person")
        .rng_key("by_world", "world")
        .rng_key("by_title", "title")
        .at(Pulse::Day, Phase::Vital)
        .stepping(draws);
    let specs = support::agreeing_specs(std::slice::from_ref(&stub));
    let registered = register(vec![stub.boxed()], &specs).unwrap_or_else(|e| panic!("{e}"));
    let reports = registered.reports().to_vec();
    let mut tables = WorldTables::new();
    tables.insert("kind.case", vec![case]);
    let mut e = Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    (e.advance_until(SimTime::EPOCH, |_, _| {}), reports)
}

/// A draw whose entity is not the kind its purpose's `[[rng]] key` declares is a `SimError`
/// naming the system, the purpose and both kinds; a key name with no kind yet is reported.
#[test]
fn a_draw_of_the_wrong_entity_kind_is_refused_by_name() {
    let (ok, reports) = kind_run(0);
    ok.unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        reports,
        [
            "X1.kinds: RNG purpose by_title keys by title, which has no EntityKind yet: its draws are not kind-checked (BCK-002 A-50)"
        ]
    );
    assert_eq!(
        kind_run(1).0.unwrap_err().to_string(),
        "X1.kinds drew under purpose by_person, whose [[rng]] key is a person, with a region (BCK-002 A-50)"
    );
    assert_eq!(
        kind_run(2).0.unwrap_err().to_string(),
        "X1.kinds drew under purpose by_world, whose [[rng]] key is a world, with a person (BCK-002 A-50)"
    );
}
