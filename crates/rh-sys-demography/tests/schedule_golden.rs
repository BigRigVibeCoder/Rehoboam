//! Golden vectors and acceptance checks of M2's rung-0 schedule (SPR-012 T-05).
//!
//! Replays every vector of `tests/golden/m2_schedule_v1.txt`, which `bin/gen_m2_reference.py`
//! generates from the exact reference `specs/demography/selftest_m2_rung0.py`; `bin/check_all.sh`
//! regenerates it with `--check`, so the vectors here are always the reference's. Then checks
//! T-05's acceptance on the engine's own integers:
//! - bit-equal at e₀ ∈ {20, 23.441086, 30, 36.569855, 43.45915, 60, 85} (and every other input);
//! - K = 1 at the standard's own e₀, 43.45915 (within 2^10 units of 2^-32: the `Q16_16` rounding of
//!   e₀ moves K by 569 units; one `Q16_16` step of e₀ moves it by about 1.3 · 10^5);
//! - Brass Table 19 within 1 per 10,000 at the e₀ each α row implies (the file carries those
//!   inputs, Architect ruling 2026-10-17), and the published e₀ within 0.06 years of the recomputed;
//! - the recomputed e₀ within 0.06 years of every input;
//! - q(15) ∈ [0.35, 0.55] exactly when e₀ ∈ [23.441086, 36.569855] (V-SF-02 at the schedule).
#![allow(
    clippy::panic,
    clippy::expect_used,
    reason = "test helpers: a malformed vector file must fail the test loudly"
)]

use std::collections::BTreeMap;
use std::sync::LazyLock;

use rh_core::fixed::{Prob32, Q16_16};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear};
use rh_sys_demography::schedule::{
    GRID_POINTS, ONE, STANDARD_HALF_YEARS, STANDARD_SURVIVORS, Schedule, death_day, resample_day,
    rhe, solve_k, survivors, trapezoid,
};

/// The committed vectors, or the file `RH_M2_SCHEDULE_GOLDEN` names (the positive control:
/// `tests/lint_fixtures/golden/altered_m2_schedule_v1.txt`, one altered value, must fail).
static VECTORS: LazyLock<String> = LazyLock::new(load);

// LINT-ALLOW(forbidden-api): a test reads an environment override for its golden file; tests never link into the engine (SPR-011 A-4)
#[allow(
    clippy::disallowed_methods,
    reason = "a test reads an environment override for its golden file"
)]
fn load() -> String {
    match std::env::var_os("RH_M2_SCHEDULE_GOLDEN") {
        Some(path) => std::fs::read_to_string(path).expect("the override file is readable"),
        None => include_str!("golden/m2_schedule_v1.txt").to_owned(),
    }
}
/// The T-05 inputs, as `Q16_16` bits of the decimal literals (half to even).
const T05_E0: [(&str, i32); 7] = [
    ("20", 1_310_720),
    ("23.441086", 1_536_235),
    ("30", 1_966_080),
    ("36.569855", 2_396_642),
    ("43.45915", 2_848_139),
    ("60", 3_932_160),
    ("85", 5_570_560),
];
/// V-SF-02's band in `Q16_16` bits (`selftest_m2_rung0.py` `E_LO`, `E_HI`; the M2 spec).
const E_LO: i32 = 1_536_235;
const E_HI: i32 = 2_396_642;
const STANDARD_E0_BITS: i32 = 2_848_139;
const K_NEAR_ONE: u64 = 1 << 10;
const AGE_15_HALF_YEARS: u8 = 30;

fn int<T: std::str::FromStr>(token: &str) -> T {
    token
        .parse()
        .unwrap_or_else(|_| panic!("not an integer: {token}"))
}

fn calendar(year: i64) -> Calendar {
    match year {
        518_400 => Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four),
        525_600 => Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four),
        _ => panic!("no calendar has {year} minutes per year"),
    }
}

fn lines(op: &str) -> impl Iterator<Item = Vec<&'static str>> + '_ {
    let text: &'static str = VECTORS.as_str();
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split(' ').collect::<Vec<_>>())
        .filter(move |t| t[0] == op)
}

fn schedule(e: i32, year: i64) -> Schedule {
    Schedule::new(Q16_16::from_bits(e), calendar(year)).expect("an e0 inside the bounds")
}

/// |a − b| ≤ 0.06 years, both in units of 2^-34 years.
fn within_006(a: u128, b: u128) -> bool {
    100 * a.abs_diff(b) <= 6 << 34
}

#[test]
fn the_standard_is_the_compiled_in_constants() {
    let mut n = 0;
    for t in lines("standard") {
        let i: usize = int(t[1]);
        assert_eq!(STANDARD_HALF_YEARS[i], int::<u8>(t[2]), "h_{i}");
        assert_eq!(STANDARD_SURVIVORS[i], int::<u16>(t[3]), "s_{i}");
        n += 1;
    }
    assert_eq!(n, GRID_POINTS);
}

#[test]
fn every_schedule_is_bit_equal_to_the_reference() {
    let mut seen = BTreeMap::new();
    for t in lines("schedule") {
        let e: i32 = int(t[1]);
        let k = solve_k(e).expect("an e0 inside the bounds");
        assert_eq!(k, int::<u64>(t[2]), "K at e0 bits {e}");
        let s = schedule(e, 518_400);
        assert_eq!(s.k(), k);
        assert_eq!(s.e0_units(), int::<u64>(t[3]), "T(K) at e0 bits {e}");
        assert!(
            within_006(
                u128::from(s.e0_units()),
                u128::from(e.cast_unsigned()) << 18
            ),
            "the recomputed e0 is more than 0.06 years from the input {e}"
        );
        seen.insert(e, s);
    }
    for t in lines("grid") {
        let e: i32 = int(t[1]);
        let want: Vec<u64> = t[2..].iter().map(|v| int(v)).collect();
        assert_eq!(seen[&e].grid().as_slice(), want.as_slice(), "grid at {e}");
    }
    for (literal, bits) in T05_E0 {
        assert!(seen.contains_key(&bits), "T-05 input {literal} is missing");
    }
    println!(
        "T-05: {} schedules bit-equal to the exact reference, the seven T-05 e0 among them",
        seen.len()
    );
}

#[test]
fn k_is_one_at_the_standards_own_e0() {
    let k = solve_k(STANDARD_E0_BITS).expect("inside the bounds");
    assert!(k.abs_diff(ONE) < K_NEAR_ONE, "K = {k}");
    // The standard itself: K = 1 gives rhe(s 2^32 / 10^4) at every grid age (S3).
    for (i, &l) in survivors(ONE).iter().enumerate() {
        let s = u128::from(STANDARD_SURVIVORS[i]);
        assert_eq!(u128::from(l), rhe(s << 32, 10_000));
    }
}

#[test]
fn brass_table_19_within_one_per_ten_thousand() {
    let ages: Vec<i64> = lines("table19.ages")
        .next()
        .expect("the ages line")
        .iter()
        .skip(1)
        .map(|v| int(v))
        .collect();
    let mut rows = 0;
    for t in lines("table19") {
        let e: i32 = int(t[3]);
        let published: u128 = int(t[4]);
        let s = schedule(e, 518_400);
        // The published two-decimal e0 bounds the recomputed one (0.06 years).
        assert!(
            (u128::from(s.e0_units()) * 100).abs_diff(published << 34) <= 6 << 34,
            "{}: e0 {} vs published {published}/100",
            t[1],
            s.e0_units()
        );
        for (age, want) in ages.iter().zip(&t[5..]) {
            let l = s.survivors_at(age * 518_400).expect("a non-negative age");
            let got = rhe(u128::from(l) * 10_000, u128::from(ONE));
            let want: u128 = int(want);
            assert!(
                got.abs_diff(want) <= 1,
                "{} age {age}: {got} vs {want}",
                t[1]
            );
        }
        rows += 1;
    }
    assert_eq!(rows, 6);
}

/// q(15) in [0.35, 0.55], exactly: 45 · 2^32 ≤ 100 L(15) ≤ 65 · 2^32.
fn q15_in_band(e: i32) -> bool {
    let i = STANDARD_HALF_YEARS
        .iter()
        .position(|&h| h == AGE_15_HALF_YEARS)
        .expect("age 15 on the grid");
    let l15 = u128::from(survivors(solve_k(e).expect("inside the bounds"))[i]);
    let one = u128::from(ONE);
    45 * one <= 100 * l15 && 100 * l15 <= 65 * one
}

#[test]
fn q15_band_holds_exactly_on_the_e0_interval() {
    for e in [E_LO - 1, E_LO, E_HI, E_HI + 1] {
        assert_eq!(q15_in_band(e), (E_LO..=E_HI).contains(&e), "e0 bits {e}");
    }
    // A sweep over the Registry bounds, every 2^12 bits (about 0.06 years).
    for e in ((20 << 16)..=(85 << 16)).step_by(1 << 12) {
        assert_eq!(q15_in_band(e), (E_LO..=E_HI).contains(&e), "e0 bits {e}");
    }
}

#[test]
fn ages_and_intervals_are_bit_equal_to_the_reference() {
    let mut cache: BTreeMap<(i32, i64), Schedule> = BTreeMap::new();
    let mut n = 0;
    for t in lines("ell") {
        let (e, year, x): (i32, i64, i64) = (int(t[1]), int(t[2]), int(t[3]));
        let s = cache.entry((e, year)).or_insert_with(|| schedule(e, year));
        assert_eq!(
            s.survivors_at(x),
            Ok(int::<u64>(t[4])),
            "l({x}) at {e}, {year}"
        );
        n += 1;
    }
    for t in lines("q") {
        let (e, year, x0, x1): (i32, i64, i64, i64) = (int(t[1]), int(t[2]), int(t[3]), int(t[4]));
        let s = cache.entry((e, year)).or_insert_with(|| schedule(e, year));
        let want = Prob32::from_bits(int(t[5])).expect("a Prob32");
        assert_eq!(
            s.interval_q(x0, x1),
            Ok(want),
            "q({x0}, {x1}) at {e}, {year}"
        );
        n += 1;
    }
    assert!(n > 1_000, "only {n} vectors");
}

#[test]
fn day_draws_are_bit_equal_to_the_reference() {
    let mut n = 0;
    for t in lines("day") {
        let (t_m, d, d1): (i64, i64, u64) = (int(t[1]), int(t[2]), int(t[3]));
        assert_eq!(
            death_day(t_m, d, d1),
            int::<i64>(t[4]),
            "day {t_m} {d} {d1}"
        );
        n += 1;
    }
    for t in lines("resample") {
        let (t_in, t_end, d1): (i64, i64, u64) = (int(t[1]), int(t[2]), int(t[3]));
        let want = (t[4] != "none").then(|| int::<i64>(t[4]));
        assert_eq!(
            resample_day(t_in, t_end, d1),
            want,
            "resample {t_in} {t_end} {d1}"
        );
        n += 1;
    }
    assert!(n > 300, "only {n} vectors");
}

#[test]
fn the_month_q_uses_the_calendars_month() {
    // At e0 = 30 on the 365-day calendar, February (28 days) gives a smaller q than January.
    let s = schedule(1_966_080, 525_600);
    let jan = rh_core::time::SimTime::from_minutes(40 * 525_600);
    let (q_jan, feb) = s.month_q(jan, 0).expect("a month");
    let (q_feb, _) = s.month_q(feb, 0).expect("a month");
    assert_eq!(feb.minutes() - jan.minutes(), 31 * 1_440);
    assert!(q_feb < q_jan);
    assert_eq!(trapezoid(s.grid()), s.e0_units());
}
