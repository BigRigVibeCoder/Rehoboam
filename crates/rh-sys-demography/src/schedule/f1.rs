//! The F1 mortality schedule at construction: `[algorithm.mortality_f1]` MR1–MR9 (SPR-016 T-02).
//!
//! Brass at any (α, β) on the General Standard, the infant sub-schedule below age 1 and the
//! logistic old-age tail from 90 to 120, compiled once into 438 knots of `l(x)` (MR9). Every
//! quantity is an integer; every division is rounded once, half to even (MR0 = S0); the only
//! non-rational functions are `rh_core::fixed`'s `exp` and `ln_ratio` (`05.1` §6 item 4), evaluated
//! here and nowhere on the per-person path. At β = 1 the odds grid is the exact rational one, so
//! MR3–MR6 are rung 0's S3–S6 (the same functions) and `l(x)` on `[1, 90]` years is rung 0's S7 at
//! every minute (the reference's check 1; `tests/schedule_f1_golden.rs`).
//!
//! Nothing in the engine calls this yet: the per-person path still draws on rung 0's schedule
//! until T-03 (MR10–MR12) lands, so T-02 is hash-neutral.
//!
//! The exact reference is `specs/demography/selftest_m2_f1_mortality.py`; its golden file
//! `specs/demography/golden/m2_f1_mortality.txt` is read directly by the test (D-6).
//!
//! REF: the M2 spec's `[algorithm.mortality_f1]` (`specs/systems/next/M2.demography.odd.toml` while
//! it is a draft); `CODEX/05_PROJECT/SPR-016_MortalityF1.md` T-02; REV-SPR-016 R2, R6, R13; W22.

use rh_core::fixed::{FixedError, Q16_16, Q32_32, exp, ln_ratio};
use rh_core::time::Calendar;

use super::{
    BISECTION_STEPS, E0_BITS_MAX, E0_BITS_MIN, GRID_POINTS, K_HI, ONE, STANDARD_HALF_YEARS,
    ScheduleError, rhe, rhe_signed, survivor, trapezoid,
};
use crate::demography::DemographyError;

/// MR1: `lambda_i = ln_ratio(10^4 − s_i, s_i)` for the interior grid ages `i = 1..69` (entry
/// `i − 1`), `Q32_32` bits; pinned by the golden file's `lambda` lines.
pub const LAMBDA: [i64; GRID_POINTS - 2] = [
    -7_446_687_127,
    -6_144_524_006,
    -5_628_636_492,
    -5_341_467_959,
    -5_167_860_488,
    -5_050_218_169,
    -4_953_075_831,
    -4_866_381_775,
    -4_792_120_518,
    -4_723_086_372,
    -4_665_979_337,
    -4_609_246_009,
    -4_548_383_898,
    -4_483_475_941,
    -4_407_970_379,
    -4_333_086_727,
    -4_243_586_194,
    -4_144_175_389,
    -4_030_899_800,
    -3_908_416_823,
    -3_781_183_403,
    -3_649_347_590,
    -3_523_161_307,
    -3_404_404_755,
    -3_288_855_008,
    -3_166_578_163,
    -3_047_460_644,
    -2_931_383_687,
    -2_818_238_086,
    -2_706_029_002,
    -2_594_707_019,
    -2_480_493_698,
    -2_368_979_441,
    -2_256_399_292,
    -2_144_571_926,
    -2_029_820_489,
    -1_915_775_998,
    -1_798_802_918,
    -1_680_699_071,
    -1_559_668_629,
    -1_439_247_684,
    -1_314_108_629,
    -1_186_018_562,
    -1_056_706_351,
    -922_654_976,
    -782_120_240,
    -638_546_742,
    -490_157_573,
    -338_618_715,
    -182_133_903,
    245_739_146,
    714_608_576,
    1_226_316_547,
    1_804_186_910,
    2_467_442_368,
    3_217_703_618,
    4_054_301_515,
    4_997_949_856,
    6_103_281_842,
    7_450_054_804,
    9_023_711_451,
    10_728_736_957,
    12_737_557_728,
    15_079_761_767,
    17_829_671_426,
    21_280_792_069,
    24_937_174_893,
    29_664_285_896,
    39_557_681_164,
];

/// MR7: the infant knots, ages `j Y / 12` for `j = 0..12`.
pub const INFANT_KNOTS: usize = 13;
/// MR7: `cumulative_share_q16` of `specs/demography/infant_sub_schedule.toml`
/// (`bourgeois-pichat-eps0.15-v1`): `c_0 = 0`, `c_12 = 2^16`, non-decreasing.
pub const INFANT_SHARES_Q16: [u64; INFANT_KNOTS] = [
    0, 20_928, 28_835, 34_919, 39_964, 44_323, 48_186, 51_672, 54_860, 57_805, 60_546, 63_115,
    65_536,
];
/// MR8: the tail's monthly knots above 90, `j = 1..360` (the last is the cap at 120).
pub const TAIL_MONTHS: usize = 360;
/// MR9: 13 infant knots, the grid ages 2 to 90 (65), the tail knots (360).
pub const KNOTS: usize = INFANT_KNOTS + (GRID_90 - 1) + TAIL_MONTHS;
/// The grid index of age 87.5 (MR8's anchor `L_65`).
pub const GRID_87: usize = 65;
/// The grid index of age 90 (`L_66`).
pub const GRID_90: usize = 66;
/// The grid index of age 92.5 (`L_67`).
pub const GRID_92: usize = 67;
/// The Registry bounds of β, [0.6, 1.65], as `Q16_16` bits rounded half to even (MR2).
pub const BETA_BITS_MIN: i32 = 39_322;
/// See [`BETA_BITS_MIN`].
pub const BETA_BITS_MAX: i32 = 108_134;
/// β = 1 exactly: the rational form (MR3).
const BETA_ONE: i32 = 1 << 16;
/// Twelfths of a year per year.
const MONTHS: i64 = 12;

/// MR9: the knot ages in twelfths of a year: 0..12, `h_i · 6` for `i = 2..66`, `1080 + j` for
/// `j = 1..360`. Strictly increasing from 0 to 1440 (120 years).
pub const KNOT_TWELFTHS: [u16; KNOTS] = knot_twelfths();

const fn knot_twelfths() -> [u16; KNOTS] {
    let mut out = [0_u16; KNOTS];
    let mut n = 0;
    while n < INFANT_KNOTS {
        #[allow(clippy::cast_possible_truncation, reason = "n < 13")]
        let twelfth = n as u16;
        out[n] = twelfth;
        n += 1;
    }
    let mut i = 2;
    while i <= GRID_90 {
        out[n] = STANDARD_HALF_YEARS[i] as u16 * 6;
        n += 1;
        i += 1;
    }
    let mut j = 1;
    while j <= TAIL_MONTHS {
        #[allow(clippy::cast_possible_truncation, reason = "j <= 360")]
        let twelfth = 1080 + j as u16;
        out[n] = twelfth;
        n += 1;
        j += 1;
    }
    out
}

/// `n / d` to the nearest integer, ties to even, for a signed 128-bit numerator; `d > 0` (MR0).
const fn rhe_wide(n: i128, d: i128) -> i128 {
    let q = n.div_euclid(d);
    let r = n.rem_euclid(d);
    if 2 * r > d || (2 * r == d && q.rem_euclid(2) == 1) {
        q + 1
    } else {
        q
    }
}

/// A construction refusal naming its step (MR0, W22).
fn refused(step: &'static str, reason: String) -> DemographyError {
    DemographyError::Construction { step, reason }
}

/// A refusal of `exp` or `ln_ratio` during construction, naming the step (MR0).
fn transcendental(step: &'static str, what: &str) -> impl Fn(FixedError) -> DemographyError {
    move |e| refused(step, format!("{what}: {e}"))
}

/// Narrows an `i128` the caller bounds to `i64`, saturating; `exp` refuses the high side and gives
/// 0 on the low side, as it would for the exact value.
fn saturate(v: i128) -> i64 {
    i64::try_from(v).unwrap_or(if v < 0 { i64::MIN } else { i64::MAX })
}

/// MR3: the odds grid.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(
    clippy::large_enum_variant,
    reason = "one value per construction, never stored in bulk; boxing would allocate for nothing"
)]
pub enum OddsGrid {
    /// β = 1 exactly: `W_i` is the exact rational `O_i`, and MR4 uses rung 0's S3.
    Rational,
    /// `w_i = exp(rhe(B lambda_i / 2^16))` for `i = 1..69` (`Q32_32` bits; entries 0 and 70 unused).
    Power([u64; GRID_POINTS]),
}

impl OddsGrid {
    /// MR3 for the `Q16_16` β bits `beta_bits`. The bounds are [`F1Schedule::new`]'s to check: this
    /// is the test hook that reaches MR3's `exp` refusal outside the Registry box.
    ///
    /// # Errors
    /// [`DemographyError::Construction`] naming MR3 when `exp` refuses an argument above 16.
    pub fn new(beta_bits: i32) -> Result<Self, DemographyError> {
        if beta_bits == BETA_ONE && !cfg!(feature = "mutant-beta-one-power") {
            return Ok(Self::Rational);
        }
        let mut w = [0_u64; GRID_POINTS];
        for (i, &lambda) in LAMBDA.iter().enumerate() {
            let y = saturate(rhe_wide(
                i128::from(beta_bits) * i128::from(lambda),
                1 << 16,
            ));
            let wi = exp(Q32_32::from_bits(y))
                .map_err(transcendental("MR3", &format!("w_{}", i + 1)))?;
            w[i + 1] = wi.to_bits().cast_unsigned();
        }
        Ok(Self::Power(w))
    }

    /// MR4: `L_i(k)`, `2^32 / (1 + K W_i)` rounded once, in the rational or the power form.
    #[must_use]
    pub fn survivor(&self, i: usize, k: u64) -> u64 {
        match self {
            Self::Rational => survivor(i, k),
            Self::Power(_) if i == 0 => ONE,
            Self::Power(_) if i == GRID_POINTS - 1 => 0,
            Self::Power(w) => {
                let den = (1_u128 << 64) + u128::from(k) * u128::from(w[i]);
                // 2^96 / den <= 2^32: the narrowing is exact.
                u64::try_from(rhe(1 << 96, den)).unwrap_or(ONE)
            }
        }
    }

    /// MR4 at every grid age.
    #[must_use]
    pub fn survivors(&self, k: u64) -> [u64; GRID_POINTS] {
        core::array::from_fn(|i| self.survivor(i, k))
    }

    /// MR5: K for the `Q16_16` e₀ `e_bits`: S5's 40 bisection steps on S4's trapezoid of MR4.
    ///
    /// # Errors
    /// [`ScheduleError::E0OutOfRange`] outside the Registry bounds.
    pub fn solve_k(&self, e_bits: i32) -> Result<u64, ScheduleError> {
        if !(E0_BITS_MIN..=E0_BITS_MAX).contains(&e_bits) {
            return Err(ScheduleError::E0OutOfRange { bits: e_bits });
        }
        let target = u64::from(e_bits.cast_unsigned()) << 18;
        let (mut lo, mut hi) = (0_u64, K_HI);
        for _ in 0..BISECTION_STEPS {
            let mid = (lo + hi) >> 1;
            if trapezoid(&self.survivors(mid)) >= target {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Ok(lo)
    }
}

/// MR8's logistic (Beard's Perks logistic, R12 §9 p. 59), `Q32_32` bits per year.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tail {
    /// Brass's per-year hazard over [87.5, 90].
    pub m1: i64,
    /// Brass's per-year hazard over [90, 92.5].
    pub m2: i64,
    /// The hazard at 90: `(m1 + m2) / 2`.
    pub mu: i64,
    /// `mu / (1 − mu)`.
    pub a: i64,
    /// `g / (1 − mu)`, `g = ln(m2 / m1) / 2.5`.
    pub b: i64,
}

impl Tail {
    /// MR8's parameters from the Brass survivors at 87.5, 90 and 92.5 (`L_65`, `L_66`, `L_67`): the
    /// test hook that reaches each precondition outside the Registry box (W22).
    ///
    /// # Errors
    /// [`DemographyError::Construction`] naming MR8: survivors not strictly decreasing and positive,
    /// a hazard that does not rise through 90, `mu` of one per year or more, `a` outside `Q32_32`,
    /// `30 b` above 16 (`exp`'s domain), or a refusal of `ln_ratio`.
    pub fn from_anchor(l87: u64, l90: u64, l92: u64) -> Result<Self, DemographyError> {
        if !(l87 > l90 && l90 > l92 && l92 > 0) {
            return Err(refused(
                "MR8",
                format!(
                    "the Brass survivors around 90 must be positive and strictly decreasing: \
                     L_65 = {l87}, L_66 = {l90}, L_67 = {l92}"
                ),
            ));
        }
        let hazard = |hi: u64, lo: u64, what: &str| {
            ln_ratio(hi, lo)
                .map(|v| rhe_wide(2 * i128::from(v.to_bits()), 5))
                .map_err(transcendental("MR8", what))
        };
        let m1 = saturate(hazard(l87, l90, "m1")?);
        let m2 = saturate(hazard(l90, l92, "m2")?);
        if !(m2 > m1 && m1 > 0) {
            return Err(refused(
                "MR8",
                format!("the hazard must rise through 90: m1 = {m1}, m2 = {m2} (Q32_32 bits)"),
            ));
        }
        let one = i128::from(ONE);
        let mu = rhe_wide(i128::from(m1) + i128::from(m2), 2);
        if mu >= one {
            return Err(refused(
                "MR8",
                format!("the hazard at 90 must be below one per year: mu = {mu} (Q32_32 bits)"),
            ));
        }
        let a = i64::try_from(rhe_wide(mu * one, one - mu)).map_err(|_| {
            refused(
                "MR8",
                format!("a = mu / (1 - mu) is outside Q32_32 at mu = {mu}"),
            )
        })?;
        let g = hazard(m2.cast_unsigned(), m1.cast_unsigned(), "g")?;
        let b = saturate(rhe_wide(g * one, one - mu));
        if !(b > 0 && i128::from(b) * 30 <= 16 * one) {
            return Err(refused(
                "MR8",
                format!("the tail needs 0 < 30 b <= 16 (exp's domain): b = {b} (Q32_32 bits)"),
            ));
        }
        Ok(Self {
            m1,
            m2,
            mu: saturate(mu),
            a,
            b,
        })
    }

    /// MR8's 361 survivors at ages `90 + j / 12` years, `j = 0..360`: `T_0 = L_66`, the closed-form
    /// survival of the logistic, forced non-increasing, `T_360 = 0` (the cap).
    ///
    /// # Errors
    /// [`DemographyError::Construction`] naming MR8 when `exp` or `ln_ratio` refuses an argument.
    pub fn knots(&self, l90: u64) -> Result<[u64; TAIL_MONTHS + 1], DemographyError> {
        let one = i128::from(ONE);
        let base = u64::try_from(one + i128::from(self.a)).unwrap_or(u64::MAX);
        let mut knots = [0_u64; TAIL_MONTHS + 1];
        knots[0] = l90;
        for j in 1..TAIL_MONTHS {
            let y = saturate(rhe_wide(i128::from(self.b) * j as i128, i128::from(MONTHS)));
            let e_j = exp(Q32_32::from_bits(y)).map_err(transcendental("MR8", "E_j"))?;
            let a_j = one + rhe_wide(i128::from(self.a) * i128::from(e_j.to_bits()), one);
            let r_j = ln_ratio(base, u64::try_from(a_j).unwrap_or(u64::MAX))
                .map_err(transcendental("MR8", "R_j"))?;
            let y_j = saturate(rhe_wide(
                i128::from(r_j.to_bits()) * one,
                i128::from(self.b),
            ));
            let s_j = exp(Q32_32::from_bits(y_j)).map_err(transcendental("MR8", "S_j"))?;
            let scaled = rhe(
                u128::from(l90) * u128::from(s_j.to_bits().cast_unsigned()),
                u128::from(ONE),
            );
            knots[j] = knots[j - 1].min(u64::try_from(scaled).unwrap_or(u64::MAX));
        }
        Ok(knots)
    }
}

/// MR7: the 13 infant survivors `I_j = 2^32 − rhe(q0 c_j / 2^16)`, `q0 = 2^32 − L_1`.
#[must_use]
pub fn infant_knots(l1: u64) -> [u64; INFANT_KNOTS] {
    let q0 = u128::from(ONE - l1.min(ONE));
    core::array::from_fn(|j| {
        let share = u64::try_from(rhe(q0 * u128::from(INFANT_SHARES_Q16[j]), 1 << 16))
            .unwrap_or(ONE)
            .min(ONE);
        ONE - share
    })
}

/// One compiled F1 schedule: the level, the Brass grid, the tail and the 438 knots of `l` (MR9)
/// with their ages in minutes of the world's calendar. Computed once, at construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct F1Schedule {
    e_bits: i32,
    beta_bits: i32,
    k: u64,
    grid: [u64; GRID_POINTS],
    tail: Tail,
    knots: [u64; KNOTS],
    ages: [i64; KNOTS],
    calendar: Calendar,
}

impl F1Schedule {
    /// MR2–MR9 for e₀ and β (compiled from the manifest) on `calendar`.
    ///
    /// # Errors
    /// [`DemographyError::Schedule`] for e₀ outside the Registry bounds; [`DemographyError::Construction`]
    /// naming MR2 for β outside them, and MR3 or MR8 for a refusal of theirs.
    pub fn new(e0: Q16_16, beta: Q16_16, calendar: Calendar) -> Result<Self, DemographyError> {
        let beta_bits = beta.to_bits();
        if !(BETA_BITS_MIN..=BETA_BITS_MAX).contains(&beta_bits) {
            return Err(refused(
                "MR2",
                format!(
                    "brass_beta = {beta_bits} / 2^16 is outside the Registry bounds [0.6, 1.65]"
                ),
            ));
        }
        let odds = OddsGrid::new(beta_bits)?;
        let k = odds
            .solve_k(e0.to_bits())
            .map_err(DemographyError::Schedule)?;
        let grid = odds.survivors(k);
        let tail = Tail::from_anchor(grid[GRID_87], grid[GRID_90], grid[GRID_92])?;
        let above = tail.knots(grid[GRID_90])?;
        let infant = infant_knots(grid[1]);
        let mut knots = [0_u64; KNOTS];
        knots[..INFANT_KNOTS].copy_from_slice(&infant);
        knots[INFANT_KNOTS..INFANT_KNOTS + GRID_90 - 1].copy_from_slice(&grid[2..=GRID_90]);
        knots[INFANT_KNOTS + GRID_90 - 1..].copy_from_slice(&above[1..]);
        let year = calendar.minutes_per_year();
        Ok(Self {
            e_bits: e0.to_bits(),
            beta_bits,
            k,
            grid,
            tail,
            knots,
            ages: core::array::from_fn(|j| i64::from(KNOT_TWELFTHS[j]) * year / MONTHS),
            calendar,
        })
    }

    /// e₀ as compiled (`Q16_16` bits).
    #[must_use]
    pub const fn e_bits(&self) -> i32 {
        self.e_bits
    }

    /// β as compiled (`Q16_16` bits).
    #[must_use]
    pub const fn beta_bits(&self) -> i32 {
        self.beta_bits
    }

    /// K in units of 2^-32 (MR5).
    #[must_use]
    pub const fn k(&self) -> u64 {
        self.k
    }

    /// The Brass grid survivors `L_i` (MR6), 32 fraction bits.
    #[must_use]
    pub const fn grid(&self) -> &[u64; GRID_POINTS] {
        &self.grid
    }

    /// The tail's parameters (MR8).
    #[must_use]
    pub const fn tail(&self) -> &Tail {
        &self.tail
    }

    /// The 438 knot survivors (MR9), 32 fraction bits, at the ages [`KNOT_TWELFTHS`].
    #[must_use]
    pub const fn knots(&self) -> &[u64; KNOTS] {
        &self.knots
    }

    /// The calendar the knot ages are measured in.
    #[must_use]
    pub const fn calendar(&self) -> Calendar {
        self.calendar
    }

    /// The age in twelfths of a year of the first knot where `l = 0` (MR8's `j_0`; 1440 = the cap).
    #[must_use]
    pub fn zero_twelfths(&self) -> u16 {
        let j = self.knots.iter().position(|&l| l == 0).unwrap_or(KNOTS - 1);
        KNOT_TWELFTHS[j]
    }

    /// The trapezoid area over the knots, in units of `2^-32 / 24` years: the F1 schedule's own e₀
    /// (the reference's check 8; reported, not part of the algorithm).
    #[must_use]
    pub fn realized_e0_units(&self) -> u128 {
        (0..KNOTS - 1)
            .map(|j| {
                let width = u128::from(KNOT_TWELFTHS[j + 1] - KNOT_TWELFTHS[j]);
                width * u128::from(self.knots[j] + self.knots[j + 1])
            })
            .sum()
    }

    /// MR9: `l(x)` for an age of `x` minutes, 32 fraction bits, linear between knots (S7's rule);
    /// 0 from 120 years.
    ///
    /// # Errors
    /// [`ScheduleError::NegativeAge`].
    pub fn survivors_at(&self, x: i64) -> Result<u64, ScheduleError> {
        if x < 0 {
            return Err(ScheduleError::NegativeAge { minutes: x });
        }
        if x >= self.ages[KNOTS - 1] {
            return Ok(0);
        }
        // The unique knot interval [A_j, A_(j+1)) holding x; A_0 = 0 <= x.
        let j = self.ages.partition_point(|&a| a <= x) - 1;
        let (l0, l1) = (self.knots[j].cast_signed(), self.knots[j + 1].cast_signed());
        let step = rhe_signed(
            (l1 - l0) * (x - self.ages[j]),
            self.ages[j + 1] - self.ages[j],
        );
        // In [0, 2^32]: between l1 and l0.
        Ok((l0 + step).cast_unsigned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_knot_ages_are_strictly_increasing_from_0_to_120_years() {
        assert_eq!(KNOTS, 438);
        assert_eq!(KNOT_TWELFTHS[0], 0);
        assert_eq!(KNOT_TWELFTHS[INFANT_KNOTS - 1], 12);
        assert_eq!(KNOT_TWELFTHS[INFANT_KNOTS], 24);
        assert_eq!(KNOT_TWELFTHS[INFANT_KNOTS + GRID_90 - 2], 1080);
        assert_eq!(KNOT_TWELFTHS[KNOTS - 1], 1440);
        assert!(KNOT_TWELFTHS.windows(2).all(|p| p[0] < p[1]));
    }

    #[test]
    fn lambda_is_ln_of_the_standard_odds() {
        for (i, &lambda) in LAMBDA.iter().enumerate() {
            let s = u64::from(super::super::STANDARD_SURVIVORS[i + 1]);
            assert_eq!(
                ln_ratio(10_000 - s, s).map(Q32_32::to_bits),
                Ok(lambda),
                "lambda_{}",
                i + 1
            );
        }
    }

    #[test]
    fn the_infant_table_runs_from_one_to_l1() {
        let knots = infant_knots(3_000_000_000);
        assert_eq!(knots[0], ONE);
        assert_eq!(knots[INFANT_KNOTS - 1], 3_000_000_000);
        assert!(knots.windows(2).all(|p| p[0] >= p[1]));
    }
}
