//! The transcendentals of plane A: `exp`, `ln_ratio` and `prob_from_hazard` on `Q32_32` (SPR-016;
//! `05.1` §6 item 4, v1.10; format `rehoboam/fixed/exp-ln/v0`).
//!
//! One integer procedure, shift-and-add (De Lugish's pseudo-division, CORDIC's hyperbolic mode),
//! with 32 constants `C_k = round(ln(1 + 2^-k) · 2^64)`, `LN2_64 = round(ln 2 · 2^64)` and
//! `INV_LN2_62 = round(2^62 / ln 2)`, at 64 fraction bits in `i128`/`u128`. Steps E1–E6 and
//! L1–L6 are `05.1`'s, in order; every result is the half-to-even rounding of the 64-fraction-bit
//! internal value, faithful (within ¾ ulp for `exp`, ½ ulp + 2^-25 ulp for `ln`). No float, no
//! `std` transcendental, no `mul_add`, no platform dependence.
//!
//! The exact reference is `specs/numerics/exp_ln_v0.py`; `tests/exp_ln_golden.rs` reads its golden
//! file `specs/numerics/golden/exp_ln_v0.txt` directly (D-6) and checks every constant below and
//! every vector bit for bit. Changing a constant, the step count, the working precision, a domain or
//! a rounding is a C3 change with a new format version.
//!
//! There is no `logistic` and no `pow` (REV-SPR-016 R12): a power is `exp(β · ln(·))` at the call
//! site, its product rounded once from 48 to 32 fraction bits.
//!
//! REF: `CODEX/05_PROJECT/SPR-016_MortalityF1.md` T-01; `05.1` §6 item 4; DET-3; V-DET-19.

use core::fmt;

use super::prob::Prob32;
use super::q32::Q32_32;
use super::round::shr_round_half_even;

/// The format of this procedure; the golden file's first line names it.
pub const EXP_LN_VERSION: &str = "rehoboam/fixed/exp-ln/v0";

/// The pseudo-division steps, `k = 1..32`.
const STEPS: usize = 32;
/// The working precision: 64 fraction bits.
const WORK: u32 = 64;
/// `Q32_32`'s fraction bits.
const FRAC: u32 = 32;
/// `exp`'s domain: `y <= 16` (`e^16 · 2^32 < 2^56`, which keeps the error under one ulp).
const EXP_MAX: i64 = 16 << FRAC;
/// For `y < -23`, `e^y · 2^32 < 1/2`: the correctly rounded result is 0.
const EXP_ZERO_BELOW: i64 = -23 << FRAC;
/// `ln_ratio`'s operands are positive and below `2^63`.
const LN_OPERAND_LIMIT: u64 = 1 << 63;

/// `round(ln 2 · 2^64)`, half to even (the golden file's `const LN2_64`).
pub const LN2_64: u64 = 12_786_308_645_202_655_660;
/// `round(2^62 / ln 2)`, half to even (the golden file's `const INV_LN2_62`).
pub const INV_LN2_62: u64 = 6_653_256_548_922_161_246;
/// `C_k = round(ln(1 + 2^-k) · 2^64)` for `k = 1..32`, half to even (`const C k`); `C_32 = 2^32`.
pub const C: [u64; STEPS] = [
    7_479_511_080_090_283_979,
    4_116_271_982_791_902_040,
    2_172_713_514_977_912_298,
    1_118_326_883_213_202_225,
    567_636_912_149_151_763,
    286_001_761_100_004_474,
    143_555_153_080_252_560,
    71_917_221_983_051_266,
    35_993_658_392_855_450,
    18_005_608_138_891_807,
    9_005_000_947_051_281,
    4_503_049_961_018_713,
    2_251_662_385_915_563,
    1_125_865_548_502_293,
    562_941_363_661_479,
    281_472_829_248_853,
    140_736_951_487_147,
    70_368_609_960_277,
    35_184_338_534_443,
    17_592_177_655_813,
    8_796_090_925_057,
    4_398_045_986_816,
    2_199_023_124_480,
    1_099_511_595_008,
    549_755_805_696,
    274_877_904_896,
    137_438_952_960,
    68_719_476_608,
    34_359_738_336,
    17_179_869_176,
    8_589_934_590,
    4_294_967_296,
];

/// A refusal of a transcendental: the input is outside the function's domain. Named, never clamped,
/// never wrapped, never `NaN`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedError {
    /// `exp(y)` with `y > 16`.
    ExpDomain {
        /// The `Q32_32` bits of `y`.
        y: i64,
    },
    /// `ln_ratio(n, d)` with `n` or `d` outside `(0, 2^63)`.
    LnDomain {
        /// The numerator.
        n: u64,
        /// The denominator.
        d: u64,
    },
    /// `prob_from_hazard(h)` with `h < 0`.
    HazardDomain {
        /// The `Q32_32` bits of `h`.
        h: i64,
    },
}

impl fmt::Display for FixedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpDomain { y } => write!(f, "ExpDomain: exp of y = {y} / 2^32 > 16"),
            Self::LnDomain { n, d } => write!(
                f,
                "LnDomain: ln_ratio({n}, {d}) needs both operands positive and below 2^63"
            ),
            Self::HazardDomain { h } => {
                write!(f, "HazardDomain: prob_from_hazard of h = {h} / 2^32 < 0")
            }
        }
    }
}

impl std::error::Error for FixedError {}

/// `e^y` (`05.1` §6 item 4, steps E1–E6), faithful, within ¾ ulp.
///
/// # Errors
/// [`FixedError::ExpDomain`] for `y > 16`. For `y < -23` the result is 0 (the correctly rounded
/// value), without computing.
#[allow(
    clippy::many_single_char_names,
    reason = "the names are 05.1 §6 item 4's (y, n, r, p, k; n, d, e, m, p, s, t, k), so each line reads against its step"
)]
pub const fn exp(y: Q32_32) -> Result<Q32_32, FixedError> {
    let y = y.to_bits();
    // E1: the domain.
    if y > EXP_MAX {
        return Err(FixedError::ExpDomain { y });
    }
    if y < EXP_ZERO_BELOW {
        return Ok(Q32_32::ZERO);
    }
    // E2: n = floor(y · INV_LN2_62 / 2^94), an i128 product and an arithmetic shift; n in [-34, 23].
    let n = ((y as i128) * (INV_LN2_62 as i128)) >> (62 + FRAC);
    // E3: the reduced argument at 64 fraction bits, in [-2^-59, ln 2 + 2^-59].
    let mut r = ((y as i128) << FRAC) - n * (LN2_64 as i128);
    // E4: p = prod (1 + 2^-k)^(a_k), truncating.
    let mut p: i128 = 1 << WORK;
    let mut k = 1;
    while k <= STEPS {
        let c = C[k - 1] as i128;
        if r >= c {
            r -= c;
            p += p >> k;
        }
        k += 1;
    }
    // E5: the first-order term, floor (an arithmetic shift; r may be slightly negative).
    p += (p * r) >> WORK;
    // E6: p · 2^n / 2^32, rounded half to even; the shift 32 - n is in [9, 66].
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "INVARIANT: n is in [-34, 23] (E2), so 32 - n is in [9, 66]"
    )]
    let shift = (FRAC as i128 - n) as u32;
    let bits = shr_round_half_even(p, shift);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "INVARIANT: e^16 · 2^32 < 2^56, so the result fits an i64"
    )]
    Ok(Q32_32::from_bits(bits as i64))
}

/// The bit length of a non-zero `u64`.
const fn bit_length(v: u64) -> i32 {
    #[allow(clippy::cast_possible_wrap, reason = "a bit count is at most 64")]
    let bits = (u64::BITS - v.leading_zeros()) as i32;
    bits
}

/// `ln(n / d)` of the exact rational `n / d` (`05.1` §6 item 4, steps L1–L6), faithful, within
/// ½ ulp + 2^-25 ulp; the result lies in `(-43.7, 43.7)`.
///
/// # Errors
/// [`FixedError::LnDomain`] unless `0 < n < 2^63` and `0 < d < 2^63`.
#[allow(
    clippy::many_single_char_names,
    reason = "the names are 05.1 §6 item 4's (y, n, r, p, k; n, d, e, m, p, s, t, k), so each line reads against its step"
)]
pub const fn ln_ratio(n: u64, d: u64) -> Result<Q32_32, FixedError> {
    // L1: the domain.
    if n == 0 || d == 0 || n >= LN_OPERAND_LIMIT || d >= LN_OPERAND_LIMIT {
        return Err(FixedError::LnDomain { n, d });
    }
    // L2: the exact binary exponent e with 2^e <= n / d < 2^(e+1); e in [-62, 62].
    let mut e = bit_length(n) - bit_length(d);
    let shifted = if e >= 0 {
        (n as u128) >> e
    } else {
        (n as u128) << -e
    };
    if shifted < d as u128 {
        e -= 1;
    }
    // L3: the mantissa m = floor(n · 2^(64 - e) / d), in [2^64, 2^65); every intermediate < 2^128.
    #[allow(
        clippy::cast_sign_loss,
        reason = "each shift is taken on its non-negative branch"
    )]
    let m = if e >= 0 {
        ((n as u128) << WORK) / ((d as u128) << (e as u32))
    } else {
        ((n as u128) << (WORK + (-e) as u32)) / (d as u128)
    };
    // L4: p <= m, built from (1 + 2^-k) factors, and the sum s of the accepted C_k.
    let mut p: u128 = 1 << WORK;
    let mut s: u128 = 0;
    let mut k = 1;
    while k <= STEPS {
        let t = p + (p >> k);
        if t <= m {
            p = t;
            s += C[k - 1] as u128;
        }
        k += 1;
    }
    // L5: the residual's first-order term, floor.
    let residual = ((m - p) << WORK) / p;
    // L6: (e · LN2_64 + s + residual) / 2^32, rounded half to even.
    #[allow(
        clippy::cast_possible_wrap,
        reason = "INVARIANT: s < 2^70 and residual < 2^33 (L4, L5), far below 2^127"
    )]
    let internal = (e as i128) * (LN2_64 as i128) + (s as i128) + (residual as i128);
    let bits = shr_round_half_even(internal, FRAC);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "INVARIANT: |ln(n / d)| < 43.7, so the result fits an i64"
    )]
    Ok(Q32_32::from_bits(bits as i64))
}

/// The probability `1 - e^-h` of a cumulative hazard `h`: `2^32 - exp(-h).bits`, exactly 0 at
/// `h = 0`, exactly 1 above `h = 23`, non-decreasing; within ¾ ulp of the true value.
///
/// # Errors
/// [`FixedError::HazardDomain`] for `h < 0`.
pub const fn prob_from_hazard(h: Q32_32) -> Result<Prob32, FixedError> {
    let bits = h.to_bits();
    if bits < 0 {
        return Err(FixedError::HazardDomain { h: bits });
    }
    // -h >= -(2^63 - 1) and -h <= 0 < 16: exp's domain, with a result in [0, 2^32].
    let survival = match exp(Q32_32::from_bits(-bits)) {
        Ok(v) => v.to_bits().cast_unsigned(),
        Err(e) => return Err(e),
    };
    match Prob32::from_bits(Prob32::ONE.to_bits() - survival) {
        Some(p) => Ok(p),
        // INVARIANT: 0 <= survival <= 2^32, so 2^32 - survival is a Prob32.
        None => Ok(Prob32::ZERO),
    }
}

#[cfg(test)]
mod tests {
    use super::{C, FixedError, Prob32, Q32_32, exp, ln_ratio, prob_from_hazard};

    #[test]
    fn the_anchors_are_exact() {
        assert_eq!(exp(Q32_32::ZERO), Ok(Q32_32::ONE));
        assert_eq!(ln_ratio(1, 1), Ok(Q32_32::ZERO));
        assert_eq!(prob_from_hazard(Q32_32::ZERO).map(Prob32::to_bits), Ok(0));
        assert_eq!(C[C.len() - 1], 1 << 32);
    }

    #[test]
    fn each_domain_is_refused_by_name() {
        let above = Q32_32::from_bits((16 << 32) + 1);
        assert_eq!(exp(above), Err(FixedError::ExpDomain { y: (16 << 32) + 1 }));
        assert_eq!(exp(Q32_32::MIN), Ok(Q32_32::ZERO));
        assert_eq!(ln_ratio(0, 1), Err(FixedError::LnDomain { n: 0, d: 1 }));
        assert_eq!(
            ln_ratio(1, 1 << 63),
            Err(FixedError::LnDomain { n: 1, d: 1 << 63 })
        );
        let e = prob_from_hazard(-Q32_32::EPSILON).err();
        assert_eq!(e, Some(FixedError::HazardDomain { h: -1 }));
        assert!(e.is_some_and(|e| e.to_string().starts_with("HazardDomain")));
        assert!(exp(above).is_err_and(|e| e.to_string().starts_with("ExpDomain")));
        assert!(ln_ratio(0, 1).is_err_and(|e| e.to_string().starts_with("LnDomain")));
    }
}
