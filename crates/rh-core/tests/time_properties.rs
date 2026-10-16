//! Property tests for `rh_core::time`: calendar arithmetic (SPR-005 T-01; GOV-002 §2 tier 3).
//!
//! Inputs come from a `SplitMix64` with fixed seeds plus the edges of the range, so every run checks
//! the same cases. `proptest` is not used: it depends on `rand`, which `bin/lint_arch.py` forbids
//! in plane-A crates even as a dev-dependency (rng-crate rule).
//!
//! The properties check the calendar against its definition (`05.1` §4.1), never by re-running
//! the implementation's algorithm: day-by-day counting from the epoch, nesting of pulse
//! boundaries, and chronological order of dates.
//!
//! Refs: SIM-2; `05.1` §4.1; ADR-0005 v1.1 items 1–2.

use rh_core::time::{
    Calendar, Date, DaysPerYear, MINUTES_PER_DAY, MINUTES_PER_HOUR, Pulse, SeasonsPerYear, SimTime,
};

const CASES: usize = 50_000;

const CALENDARS: [Calendar; 4] = [
    Calendar::new(DaysPerYear::D360, SeasonsPerYear::Two),
    Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four),
    Calendar::new(DaysPerYear::D365, SeasonsPerYear::Two),
    Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four),
];

/// `SplitMix64` (Steele, Lea and Flood 2014).
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// An instant: alternately anywhere in a +-10^6-year window around the epoch (where worlds
    /// live) and anywhere in an `i64` range kept 2 years clear of its ends.
    fn time(&mut self) -> SimTime {
        let raw = self.next_u64().cast_signed();
        let m = if raw & 1 == 0 {
            raw % (1_000_000 * 365 * MINUTES_PER_DAY)
        } else {
            raw / 2
        };
        SimTime::from_minutes(m)
    }
}

fn samples(seed: u64) -> Vec<SimTime> {
    let mut g = SplitMix64(seed);
    let mut out: Vec<SimTime> = (0..CASES).map(|_| g.time()).collect();
    out.extend([-1, 0, 1, -MINUTES_PER_DAY, MINUTES_PER_DAY - 1].map(SimTime::from_minutes));
    out
}

#[test]
fn to_date_and_from_date_round_trip() {
    for c in CALENDARS {
        for t in samples(1) {
            let d = c.to_date(t);
            assert!((1..=12).contains(&d.month), "{t}: {d}");
            assert!(
                d.day >= 1 && Some(d.day) <= c.days_in_month(d.month),
                "{t}: {d}"
            );
            assert!(d.hour < 24 && d.minute < 60, "{t}: {d}");
            assert_eq!(c.from_date(d), Some(t), "{c:?} {t}: {d}");
        }
    }
}

#[test]
fn dates_are_chronological() {
    for c in CALENDARS {
        let mut ts = samples(2);
        ts.sort_unstable();
        let dates: Vec<Date> = ts.iter().map(|&t| c.to_date(t)).collect();
        assert!(dates.windows(2).all(|w| w[0] <= w[1]), "{c:?}");
    }
}

#[test]
fn a_year_counted_day_by_day_matches_the_calendar() {
    // Walk every day of years -2..=2 and check the date advances by exactly one calendar day.
    for c in CALENDARS {
        let start = c
            .from_date(Date {
                year: -2,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
            })
            .unwrap();
        let mut prev = c.to_date(start);
        for i in 1..(5 * c.days_in_year()) {
            let d = c.to_date(SimTime::from_minutes(start.minutes() + i * MINUTES_PER_DAY));
            let expected = if prev.day < c.days_in_month(prev.month).unwrap() {
                Date {
                    day: prev.day + 1,
                    ..prev
                }
            } else if prev.month < 12 {
                Date {
                    month: prev.month + 1,
                    day: 1,
                    ..prev
                }
            } else {
                Date {
                    year: prev.year + 1,
                    month: 1,
                    day: 1,
                    ..prev
                }
            };
            assert_eq!(d, expected, "{c:?} day {i}");
            prev = d;
        }
        assert_eq!(prev.year, 2);
    }
}

#[test]
fn floor_and_next_boundary_bracket_every_instant() {
    for c in CALENDARS {
        for t in samples(3) {
            for p in Pulse::ALL {
                let lo = c.floor(p, t).unwrap();
                let hi = c.next_boundary(p, t).unwrap();
                assert!(lo <= t && t < hi, "{c:?} {p} {t}: [{lo}, {hi})");
                assert!(
                    c.is_boundary(p, lo) && c.is_boundary(p, hi),
                    "{c:?} {p} {t}"
                );
                assert_eq!(c.floor(p, lo), Some(lo), "floor is idempotent");
                // No boundary strictly between lo and hi: the next boundary after lo is hi.
                assert_eq!(c.next_boundary(p, lo), Some(hi), "{c:?} {p} {t}");
            }
        }
    }
}

#[test]
fn pulse_periods_have_their_lengths() {
    for c in CALENDARS {
        for t in samples(4) {
            let len = |p| {
                let lo = c.floor(p, t).unwrap();
                c.next_boundary(p, t).unwrap().minutes() - lo.minutes()
            };
            assert_eq!(len(Pulse::Hour), MINUTES_PER_HOUR);
            assert_eq!(len(Pulse::Day), MINUTES_PER_DAY);
            assert_eq!(len(Pulse::Year), c.minutes_per_year());
            let d = c.to_date(t);
            assert_eq!(
                len(Pulse::Month),
                i64::from(c.days_in_month(d.month).unwrap()) * MINUTES_PER_DAY
            );
            let per = c.seasons.months_per_season();
            let first = c.season_of_month(d.month) * per + 1;
            let days: i64 = (first..first + per)
                .map(|m| i64::from(c.days_in_month(m).unwrap()))
                .sum();
            assert_eq!(len(Pulse::Season), days * MINUTES_PER_DAY);
        }
    }
}

#[test]
fn coarse_boundaries_are_fine_boundaries() {
    for c in CALENDARS {
        for t in samples(5) {
            for (i, &coarse) in Pulse::ALL.iter().enumerate() {
                let b = c.floor(coarse, t).unwrap();
                for &fine in &Pulse::ALL[i..] {
                    assert!(
                        c.is_boundary(fine, b),
                        "{c:?} {coarse} boundary {b} is not a {fine} boundary"
                    );
                }
            }
            let fired = c.pulses_at(t);
            assert_eq!(
                fired.as_slice(),
                &Pulse::ALL[Pulse::ALL.len() - fired.len()..],
                "a suffix"
            );
        }
    }
}

#[test]
fn next_instant_is_the_earliest_next_boundary() {
    for c in CALENDARS {
        for t in samples(6).into_iter().take(5_000) {
            assert_eq!(c.next_instant(t, &[]), None);
            let both = c.next_instant(t, &[Pulse::Month, Pulse::Day]).unwrap();
            assert_eq!(Some(both), c.next_boundary(Pulse::Day, t));
            let only = c.next_instant(t, &[Pulse::Year]).unwrap();
            assert_eq!(Some(only), c.next_boundary(Pulse::Year, t));
        }
    }
}
