//! The rung-0 mortality schedule in fixed point (SPR-012 T-05; SPR-011 ruling 7).
//!
//! Implements `[algorithm.mortality_rung0]` S0 to S9 and S11 of `specs/systems/M2.demography.odd.toml`
//! exactly: every quantity is an integer, every width is the one the spec states, and every division
//! is rounded once, half to even ([`rhe`], S0). No float and no transcendental: at β = 1 Brass is
//! l(x) = 1 / (1 + K·O(x)) with O(x) = (1 − lₛ(x)) / lₛ(x), so K = e^{2α} is solved from e₀ by a
//! fixed bisection (S5) and never needs `exp` or `ln`.
//!
//! The exact reference is `specs/demography/selftest_m2_rung0.py` (Python `fractions`);
//! `bin/gen_m2_reference.py` ports it and writes `tests/golden/m2_schedule_v1.txt`, which
//! `tests/schedule_golden.rs` replays bit for bit.
//!
//! REF: `specs/systems/M2.demography.odd.toml` `[algorithm.mortality_rung0]`; R12 §9;
//! `05.1` §6 item 4 (`Prob32`); `CODEX/05_PROJECT/SPR-012_WalkingSkeletonB.md` T-05.

use core::fmt;

use rh_core::fixed::{Prob32, Q16_16};
use rh_core::time::{Calendar, Pulse, SimTime};

/// The number of grid ages of the General Standard (S1).
pub const GRID_POINTS: usize = 71;

/// S1: the grid ages in half-years, `h_i = 2 a_i` (Brass 1971 Table 4; exact).
pub const STANDARD_HALF_YEARS: [u8; GRID_POINTS] = [
    0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32, 34, 36, 38, 40, 42, 44, 46, 48,
    50, 52, 54, 56, 58, 60, 62, 64, 66, 68, 70, 72, 74, 76, 78, 80, 82, 84, 86, 88, 90, 92, 94, 96,
    98, 100, 105, 110, 115, 120, 125, 130, 135, 140, 145, 150, 155, 160, 165, 170, 175, 180, 185,
    190, 195, 200,
];

/// S1: the four-digit survivors of the General Standard, `s_i = 10^4 lₛ(a_i)` (exact).
pub const STANDARD_SURVIVORS: [u16; GRID_POINTS] = [
    10000, 8499, 8070, 7876, 7762, 7691, 7642, 7601, 7564, 7532, 7502, 7477, 7452, 7425, 7396,
    7362, 7328, 7287, 7241, 7188, 7130, 7069, 7005, 6943, 6884, 6826, 6764, 6703, 6643, 6584, 6525,
    6466, 6405, 6345, 6284, 6223, 6160, 6097, 6032, 5966, 5898, 5830, 5759, 5686, 5612, 5535, 5454,
    5371, 5285, 5197, 5106, 4857, 4585, 4291, 3965, 3602, 3210, 2801, 2380, 1945, 1500, 1090, 760,
    490, 290, 155, 70, 30, 10, 1, 0,
];

/// The standard's scale: survivors per 10,000.
const SCALE: u64 = 10_000;
/// 1 with 32 fraction bits: l and q.
pub const ONE: u64 = 1 << 32;
/// S5: the bisection's upper end, K = 256.
pub const K_HI: u64 = 1 << 40;
/// S5: the bisection's iteration count.
pub const BISECTION_STEPS: u32 = 40;
/// The Registry bounds of e₀, [20, 85] years, as `Q16_16` bits (S2).
pub const E0_BITS_MIN: i32 = 20 << 16;
/// See [`E0_BITS_MIN`].
pub const E0_BITS_MAX: i32 = 85 << 16;
/// Minutes per day: deaths fire at day starts (amended ruling 7).
pub const DAY: i64 = 1_440;

/// A refusal of the schedule. Every variant names what was wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduleError {
    /// e₀ outside the Registry bounds [20, 85] years (S2).
    E0OutOfRange {
        /// The `Q16_16` bits given.
        bits: i32,
    },
    /// A negative age (S7: an error, never clamped).
    NegativeAge {
        /// The age in minutes.
        minutes: i64,
    },
    /// An instant the calendar cannot place (overflow at the end of time).
    Calendar {
        /// The instant.
        at: i64,
    },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::E0OutOfRange { bits } => write!(
                f,
                "e0 = {bits} / 2^16 years is outside the Registry bounds [20, 85] \
                 ([algorithm.mortality_rung0] S2)"
            ),
            Self::NegativeAge { minutes } => write!(
                f,
                "an age of {minutes} minutes is negative ([algorithm.mortality_rung0] S7)"
            ),
            Self::Calendar { at } => write!(f, "the calendar cannot place tick {at}"),
        }
    }
}

impl std::error::Error for ScheduleError {}

/// S0 for unsigned operands: `n / d` to the nearest integer, ties to even; `d > 0`.
#[must_use]
pub const fn rhe(n: u128, d: u128) -> u128 {
    let q = n / d;
    let r = n % d;
    if 2 * r > d || (2 * r == d && q % 2 == 1) {
        q + 1
    } else {
        q
    }
}

/// S0 for a signed numerator: `n / d` to the nearest integer, ties to even; `d > 0`. The quotient
/// is the floor (`div_euclid`), so `0 <= r < d` as S0 states.
#[must_use]
pub const fn rhe_signed(n: i64, d: i64) -> i64 {
    let q = n.div_euclid(d);
    let r = n.rem_euclid(d);
    if 2 * r > d || (2 * r == d && q.rem_euclid(2) == 1) {
        q + 1
    } else {
        q
    }
}

/// Narrows a value the algorithm bounds below 2^33 (L, q bits, a day index).
#[allow(
    clippy::cast_possible_truncation,
    reason = "INVARIANT: every caller's value is below 2^33 by the widths S3, S8 and S9 state"
)]
const fn narrow(v: u128) -> u64 {
    v as u64
}

/// S3: the survivors at grid age `i` for the level `k` (K = k / 2^32), with 32 fraction bits.
#[must_use]
pub const fn survivor(i: usize, k: u64) -> u64 {
    let s = STANDARD_SURVIVORS[i] as u64;
    if s == 0 {
        return 0;
    }
    let num = (s as u128) << 64;
    let den = (s << 32) + k * (SCALE - s);
    // L_i(k) <= 2^32, so the narrowing is exact.
    narrow(rhe(num, den as u128))
}

/// S3 at every grid age.
#[must_use]
pub fn survivors(k: u64) -> [u64; GRID_POINTS] {
    core::array::from_fn(|i| survivor(i, k))
}

/// S4: the trapezoid sum T, e₀ in units of 2^-34 years.
#[must_use]
pub fn trapezoid(l: &[u64; GRID_POINTS]) -> u64 {
    (0..GRID_POINTS - 1)
        .map(|i| {
            let width = u64::from(STANDARD_HALF_YEARS[i + 1] - STANDARD_HALF_YEARS[i]);
            width * (l[i] + l[i + 1])
        })
        .sum()
}

/// S5: K (32 fraction bits) for the `Q16_16` e₀ `e_bits`: the largest k with e₀(k) ≥ e₀, by 40
/// bisection steps on [0, 2^40].
///
/// # Errors
/// [`ScheduleError::E0OutOfRange`] outside the Registry bounds.
pub fn solve_k(e_bits: i32) -> Result<u64, ScheduleError> {
    if !(E0_BITS_MIN..=E0_BITS_MAX).contains(&e_bits) {
        return Err(ScheduleError::E0OutOfRange { bits: e_bits });
    }
    let target = u64::from(e_bits.cast_unsigned()) << 18;
    let (mut lo, mut hi) = (0_u64, K_HI);
    for _ in 0..BISECTION_STEPS {
        let mid = (lo + hi) >> 1;
        if trapezoid(&survivors(mid)) >= target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(lo)
}

/// The rung-0 schedule of one world: K, the 71 grid survivors (S6) and the grid ages in minutes
/// of the world's calendar (S7). Computed once, at construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schedule {
    k: u64,
    grid: [u64; GRID_POINTS],
    ages: [i64; GRID_POINTS],
    calendar: Calendar,
}

impl Schedule {
    /// The schedule for e₀ (S2, compiled from the manifest) on `calendar`.
    ///
    /// # Errors
    /// [`ScheduleError::E0OutOfRange`].
    pub fn new(e0: Q16_16, calendar: Calendar) -> Result<Self, ScheduleError> {
        let k = solve_k(e0.to_bits())?;
        let year = calendar.minutes_per_year();
        Ok(Self {
            k,
            grid: survivors(k),
            ages: core::array::from_fn(|i| i64::from(STANDARD_HALF_YEARS[i]) * year / 2),
            calendar,
        })
    }

    /// K in units of 2^-32.
    #[must_use]
    pub const fn k(&self) -> u64 {
        self.k
    }

    /// The grid survivors `L_i` (S6), 32 fraction bits.
    #[must_use]
    pub const fn grid(&self) -> &[u64; GRID_POINTS] {
        &self.grid
    }

    /// The trapezoid e₀ of this schedule, in units of 2^-34 years (S4 at K).
    #[must_use]
    pub fn e0_units(&self) -> u64 {
        trapezoid(&self.grid)
    }

    /// The calendar the schedule's ages are measured in.
    #[must_use]
    pub const fn calendar(&self) -> Calendar {
        self.calendar
    }

    /// S7: l(x) for an age of `x` minutes, 32 fraction bits, linear between grid ages; 0 from 100.
    ///
    /// # Errors
    /// [`ScheduleError::NegativeAge`].
    pub fn survivors_at(&self, x: i64) -> Result<u64, ScheduleError> {
        if x < 0 {
            return Err(ScheduleError::NegativeAge { minutes: x });
        }
        let last = GRID_POINTS - 1;
        if x >= self.ages[last] {
            return Ok(0);
        }
        // The unique i with A_i <= x < A_(i+1); A_0 = 0 <= x.
        let i = self.ages.partition_point(|&a| a <= x) - 1;
        let (l0, l1) = (self.grid[i].cast_signed(), self.grid[i + 1].cast_signed());
        let step = rhe_signed(
            (l1 - l0) * (x - self.ages[i]),
            self.ages[i + 1] - self.ages[i],
        );
        // In [0, 2^32]: between l1 and l0.
        Ok((l0 + step).cast_unsigned())
    }

    /// The probability of dying between ages `x0` and `x1` minutes (`x0 <= x1`): 1 if l(x0) = 0,
    /// else rhe(2^32 (l(x0) − l(x1)) / l(x0)). S8 with x1 = x + D; S11 with the input's instant
    /// and the month's end.
    ///
    /// # Errors
    /// [`ScheduleError::NegativeAge`].
    pub fn interval_q(&self, x0: i64, x1: i64) -> Result<Prob32, ScheduleError> {
        let now = self.survivors_at(x0)?;
        if now == 0 {
            return Ok(Prob32::ONE);
        }
        let later = self.survivors_at(x1)?;
        let bits = rhe(u128::from(ONE) * u128::from(now - later), u128::from(now));
        // now - later <= now, so bits <= 2^32 and from_bits succeeds.
        Ok(Prob32::from_bits(narrow(bits)).unwrap_or(Prob32::ONE))
    }

    /// S8: the month's probability of death for a person born at `birth` at the Month pulse
    /// instant `t_m`, with the month's length from the calendar. Also returns the month's end.
    ///
    /// # Errors
    /// [`ScheduleError::NegativeAge`] (born after `t_m`); [`ScheduleError::Calendar`].
    pub fn month_q(&self, t_m: SimTime, birth: i64) -> Result<(Prob32, SimTime), ScheduleError> {
        let end = self.month_end(t_m)?;
        let x = t_m.minutes() - birth;
        let d = end.minutes() - t_m.minutes();
        Ok((self.interval_q(x, x + d)?, end))
    }

    /// The first month start strictly after `t`.
    ///
    /// # Errors
    /// [`ScheduleError::Calendar`].
    pub fn month_end(&self, t: SimTime) -> Result<SimTime, ScheduleError> {
        self.calendar
            .next_boundary(Pulse::Month, t)
            .ok_or(ScheduleError::Calendar { at: t.minutes() })
    }
}

/// S9: the death instant for draw `d1` in the month starting at `t_m` that lasts `d` minutes:
/// `t_m + 1440 j`, `j = 1 + floor(d1 (n − 1) / 2^64)`, `n = d / 1440`. Never the pulse instant.
#[must_use]
pub fn death_day(t_m: i64, d: i64, d1: u64) -> i64 {
    let n = d / DAY;
    let spread = u128::from((n - 1).cast_unsigned());
    // floor(d1 (n - 1) / 2^64) < n - 1 <= 30.
    let j = 1 + narrow((u128::from(d1) * spread) >> 64).cast_signed();
    t_m + DAY * j
}

/// S11: the re-drawn death instant after an input at `t_in`, in the month that ends at `t_end`:
/// one of the `c` day starts strictly after `t_in` and before `t_end`, `s_(j+1)` with
/// `j = floor(d1 c / 2^64)`; `None` when `c = 0`.
#[must_use]
pub fn resample_day(t_in: i64, t_end: i64, d1: u64) -> Option<i64> {
    let first = (t_in.div_euclid(DAY) + 1) * DAY;
    let c = if first < t_end {
        (t_end - first) / DAY
    } else {
        0
    };
    if c == 0 {
        return None;
    }
    let j = narrow((u128::from(d1) * u128::from(c.cast_unsigned())) >> 64).cast_signed();
    Some(first + DAY * j)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_ties_go_to_even() {
        assert_eq!(rhe(5, 2), 2);
        assert_eq!(rhe(7, 2), 4);
        assert_eq!(rhe(7, 3), 2);
        assert_eq!(rhe_signed(-5, 2), -2);
        assert_eq!(rhe_signed(-7, 2), -4);
        assert_eq!(rhe_signed(-7, 3), -2);
        assert_eq!(rhe_signed(-8, 3), -3);
    }

    #[test]
    fn the_spec_check_values_hold() {
        for (literal_bits, k) in [
            (20 << 16, 18_743_037_470_u64),
            (30 << 16, 9_582_773_732),
            (60 << 16, 1_466_952_214),
            (85 << 16, 73_327_863),
        ] {
            assert_eq!(solve_k(literal_bits), Ok(k));
        }
        assert_eq!(
            solve_k(19 << 16),
            Err(ScheduleError::E0OutOfRange { bits: 19 << 16 })
        );
    }
}
