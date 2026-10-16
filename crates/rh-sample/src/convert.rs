//! Exact conversions across the edge of the float zone, with no `as` cast.
//!
//! Every `f64` is exactly `±m · 2^e` for integers `m < 2^53` and `e`; [`decompose`] reads `m` and
//! `e` from the bits. Conversions out of the zone start from that exact value:
//!
//! - [`q16_16_from_f64`]: the `Q16_16` nearest the exact value, ties to even (the one rounding rule
//!   of `rh_core::fixed`), then saturated to the type's range.
//! - [`floor_to_i64`]: the exact integer below a value.
//!
//! Conversions into the zone are exact by construction: a `Q16_16` is `bits / 2^16` with `bits` an
//! `i32`, which an `f64` holds exactly, and so is every `u32` count.
//!
//! DECISION: decode the bits rather than use `as`. `x as i64` truncates toward zero and saturates
//! silently, and `as` into a fixed-point type would need a multiply that may round twice; the
//! ruling (item 4) forbids both. ALTERNATIVES CONSIDERED: `x.round_ties_even() as i32` after scaling
//! by `2^16` (exact for in-range values, but the cast still saturates without saying so, and NaN
//! becomes 0 silently).
//!
//! REF: SPR-003 Architect ruling item 4; `rh_core::fixed` (`round.rs`: round half to even).

use rh_core::fixed::Q16_16;

/// Bits of an `f64`'s stored significand.
const MANTISSA_BITS: u32 = 52;
/// Mask of the stored significand.
const MANTISSA_MASK: u64 = (1 << MANTISSA_BITS) - 1;
/// Mask of the biased exponent, after shifting the significand out.
const EXPONENT_MASK: u64 = 0x7ff;
/// `e` such that a normal number with biased exponent `E` is `(2^52 + f) · 2^(E − EXPONENT_BIAS)`.
const EXPONENT_BIAS: i32 = 1075;
/// The exponent of every subnormal: `f · 2^-1074`.
const SUBNORMAL_EXPONENT: i32 = -1074;

/// `x = ±m · 2^e` exactly, as `(negative, m, e)`; `None` for an infinity or NaN.
pub(crate) fn decompose(x: f64) -> Option<(bool, u64, i32)> {
    let bits = x.to_bits();
    let negative = bits >> 63 == 1;
    let biased = (bits >> MANTISSA_BITS) & EXPONENT_MASK;
    let fraction = bits & MANTISSA_MASK;
    if biased == EXPONENT_MASK {
        return None;
    }
    if biased == 0 {
        return Some((negative, fraction, SUBNORMAL_EXPONENT));
    }
    // INVARIANT: biased < 2^11, so it fits an i32.
    let exponent = i32::try_from(biased).unwrap_or(0) - EXPONENT_BIAS;
    Some((negative, fraction | (1 << MANTISSA_BITS), exponent))
}

/// `value / 2^shift`, rounded half to even; `shift` at least 1.
fn shr_round_half_even(value: u128, shift: u32) -> u128 {
    if shift >= 128 {
        return 0; // value < 2^128 <= 2^(shift - 1): below one half
    }
    let floor = value >> shift;
    let remainder = value - (floor << shift);
    let half = 1_u128 << (shift - 1);
    if remainder > half || (remainder == half && floor & 1 == 1) {
        floor + 1
    } else {
        floor
    }
}

/// The `Q16_16` nearest `x`, ties to even, saturated to `[Q16_16::MIN, Q16_16::MAX]`.
///
/// Infinities saturate. NaN cannot reach here (every sampler returns a finite value); it maps to
/// zero so that the function stays total.
pub(crate) fn q16_16_from_f64(x: f64) -> Q16_16 {
    let Some((negative, m, e)) = decompose(x) else {
        return if x.is_nan() {
            Q16_16::ZERO
        } else if x > 0.0 {
            Q16_16::MAX
        } else {
            Q16_16::MIN
        };
    };
    // bits = x · 2^16 = ±m · 2^(e + 16), exactly.
    let scaled = e + i32::try_from(Q16_16::FRAC_BITS).unwrap_or(16);
    let magnitude: u128 = if m == 0 {
        0
    } else if scaled >= 0 {
        // m < 2^53, so m · 2^scaled is beyond every i32 once scaled > 64: saturate.
        let shift = u32::try_from(scaled).unwrap_or(u32::MAX);
        if shift > 64 {
            u128::MAX
        } else {
            u128::from(m) << shift
        }
    } else {
        shr_round_half_even(u128::from(m), scaled.unsigned_abs())
    };
    let limit = if negative {
        u128::from(i32::MIN.unsigned_abs())
    } else {
        u128::from(i32::MAX.unsigned_abs())
    };
    let clamped = magnitude.min(limit);
    // INVARIANT: clamped <= 2^31, so it fits an i64, and ±clamped fits an i32.
    let signed = i64::try_from(clamped).unwrap_or(i64::MAX);
    let bits = if negative { -signed } else { signed };
    Q16_16::from_bits(i32::try_from(bits).unwrap_or(if negative { i32::MIN } else { i32::MAX }))
}

/// The exact value of a `Q16_16`: `bits / 2^16`. A multiply by a power of two never rounds.
pub(crate) fn f64_from_q16_16(q: Q16_16) -> f64 {
    f64::from(q.to_bits()) * TWO_POW_MINUS_16
}

/// `2^-16`, exactly.
const TWO_POW_MINUS_16: f64 = 1.0 / 65_536.0;

/// The integer `⌊x⌋`, exactly; `None` if `x` is not finite or `⌊x⌋` is outside `i64`.
pub(crate) fn floor_to_i64(x: f64) -> Option<i64> {
    let (negative, m, e) = decompose(libm::floor(x))?;
    let magnitude: u64 = if m == 0 {
        0
    } else if e >= 0 {
        // ⌊x⌋ = ±m · 2^e with m < 2^53: refuse anything at or beyond 2^63.
        let shift = e.unsigned_abs();
        if m.leading_zeros() <= shift {
            return None;
        }
        m << shift
    } else {
        // An integer-valued float with e < 0 has its low -e bits clear.
        let shift = e.unsigned_abs();
        if shift >= 64 { 0 } else { m >> shift }
    };
    let value = i64::try_from(magnitude).ok()?;
    Some(if negative { -value } else { value })
}

#[cfg(test)]
mod tests {
    use super::{decompose, f64_from_q16_16, floor_to_i64, q16_16_from_f64};
    use rh_core::fixed::Q16_16;

    #[test]
    fn decompose_reads_the_exact_value() {
        assert_eq!(decompose(1.0), Some((false, 1 << 52, -52)));
        assert_eq!(decompose(-0.75), Some((true, 3 << 51, -53)));
        assert_eq!(decompose(0.0), Some((false, 0, -1074)));
        assert_eq!(decompose(f64::from_bits(1)), Some((false, 1, -1074)));
        assert_eq!(decompose(f64::INFINITY), None);
        assert_eq!(decompose(f64::NAN), None);
    }

    #[test]
    fn q16_16_rounds_the_exact_value_half_to_even() {
        let step = 1.0 / 65_536.0;
        assert_eq!(q16_16_from_f64(1.0), Q16_16::ONE);
        assert_eq!(q16_16_from_f64(0.5 * step), Q16_16::ZERO); // tie -> even 0
        assert_eq!(q16_16_from_f64(1.5 * step), Q16_16::from_bits(2)); // tie -> even 2
        assert_eq!(q16_16_from_f64(2.5 * step), Q16_16::from_bits(2)); // tie -> even 2
        assert_eq!(q16_16_from_f64(-1.5 * step), Q16_16::from_bits(-2));
        // Just above a tie rounds up: the exact value decides, not a pre-rounded product.
        let above = f64::from_bits((0.5 * step).to_bits() + 1);
        assert_eq!(q16_16_from_f64(above), Q16_16::EPSILON);
        assert_eq!(q16_16_from_f64(f64::from_bits(1)), Q16_16::ZERO); // subnormal
        assert_eq!(q16_16_from_f64(-0.0), Q16_16::ZERO);
    }

    #[test]
    fn q16_16_saturates_and_is_total() {
        assert_eq!(q16_16_from_f64(32_768.0), Q16_16::MAX);
        assert_eq!(q16_16_from_f64(-32_768.0), Q16_16::MIN);
        assert_eq!(q16_16_from_f64(-1e300), Q16_16::MIN);
        assert_eq!(q16_16_from_f64(1e300), Q16_16::MAX);
        assert_eq!(q16_16_from_f64(f64::INFINITY), Q16_16::MAX);
        assert_eq!(q16_16_from_f64(f64::NEG_INFINITY), Q16_16::MIN);
        assert_eq!(q16_16_from_f64(f64::NAN), Q16_16::ZERO);
        // The largest value below the ceiling still rounds to MAX, not past it.
        assert_eq!(q16_16_from_f64(32_767.999_99), Q16_16::MAX);
    }

    #[test]
    fn round_trips_every_fixed_value_exactly() {
        for bits in [i32::MIN, -65_537, -1, 0, 1, 32_768, 65_536, i32::MAX] {
            let q = Q16_16::from_bits(bits);
            assert_eq!(q16_16_from_f64(f64_from_q16_16(q)), q, "{bits}");
        }
    }

    #[test]
    fn floor_is_exact_and_refuses_what_does_not_fit() {
        assert_eq!(floor_to_i64(2.999), Some(2));
        assert_eq!(floor_to_i64(-0.5), Some(-1));
        assert_eq!(floor_to_i64(-0.0), Some(0));
        assert_eq!(floor_to_i64(0.25), Some(0));
        assert_eq!(
            floor_to_i64(4_503_599_627_370_497.0),
            Some(4_503_599_627_370_497)
        );
        assert_eq!(floor_to_i64(9_223_372_036_854_775_808.0), None); // 2^63
        assert_eq!(floor_to_i64(-9_223_372_036_854_775_808.0), None); // -2^63: refused too
        assert_eq!(
            floor_to_i64(9_223_372_036_854_774_784.0), // the largest float below 2^63
            Some(9_223_372_036_854_774_784)
        );
        assert_eq!(floor_to_i64(4.611_686_018_427_388e18), Some(1 << 62));
        assert_eq!(floor_to_i64(f64::NAN), None);
        assert_eq!(floor_to_i64(f64::INFINITY), None);
    }
}
