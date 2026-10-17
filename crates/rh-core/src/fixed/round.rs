//! The one rounding rule of `rh-core::fixed`, and the clamps that make arithmetic saturating.
//!
//! Every operation in this module family computes its exact result in `i128` (which holds every
//! intermediate product and scaled numerator without overflow), rounds it once with the rule below,
//! and then clamps it to the storage type. Rounding happens before saturation, so a result is always
//! the representable value nearest the exact one.
//!
//! DECISION: round half to even ("banker's rounding") for every multiply and divide. ALTERNATIVES
//! CONSIDERED: truncation toward zero or toward negative infinity (biased: a repeated product drifts
//! one way, which is visible over 1,000 simulated years of compounding rates); round half away from
//! zero (biased on exact ties, which are common with power-of-two scales). Half to even is unbiased,
//! is IEEE 754's default, and is what Python's `round(Fraction)` does, so the golden-vector generator
//! needs no rounding code of its own. REF: SPR-010 T-02; `05.5` §2.1; ADR-0006 item 4.
//!
//! REF: `CODEX/05_PROJECT/SPR-010_FixedPoint.md` T-02.

/// Divides `value` by `2^shift`, rounding half to even.
///
/// `shift` must be at least 1 and below 127; every caller passes a type's fractional bit count.
pub(crate) const fn shr_round_half_even(value: i128, shift: u32) -> i128 {
    let floor = value >> shift; // an arithmetic shift is floor division by 2^shift
    let remainder = value - (floor << shift); // 0 <= remainder < 2^shift
    let half = 1_i128 << (shift - 1);
    if remainder > half || (remainder == half && floor & 1 == 1) {
        floor + 1
    } else {
        floor
    }
}

/// Divides `numerator` by `denominator`, rounding half to even; `None` if the denominator is zero.
///
/// Callers keep `|numerator|` below `2^126`, so negating it cannot overflow.
pub(crate) const fn div_round_half_even(numerator: i128, denominator: i128) -> Option<i128> {
    if denominator == 0 {
        return None;
    }
    let (n, d) = if denominator < 0 {
        (-numerator, -denominator)
    } else {
        (numerator, denominator)
    };
    let floor = n.div_euclid(d);
    let twice_remainder = 2 * n.rem_euclid(d); // 0 <= remainder < d
    if twice_remainder > d || (twice_remainder == d && floor & 1 == 1) {
        Some(floor + 1)
    } else {
        Some(floor)
    }
}

/// Clamps an exact value to `i16`: the saturation step of `Q8_8`.
pub(crate) const fn saturate_i16(value: i128) -> i16 {
    if value > i16::MAX as i128 {
        i16::MAX
    } else if value < i16::MIN as i128 {
        i16::MIN
    } else {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "INVARIANT: value is within i16 here"
        )]
        let narrowed = value as i16;
        narrowed
    }
}

/// Clamps an exact value to `i32`: the saturation step of `Q16_16`.
pub(crate) const fn saturate_i32(value: i128) -> i32 {
    if value > i32::MAX as i128 {
        i32::MAX
    } else if value < i32::MIN as i128 {
        i32::MIN
    } else {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "INVARIANT: value is within i32 here"
        )]
        let narrowed = value as i32;
        narrowed
    }
}

/// Clamps an exact value to `i64`: the saturation step of `Money`.
pub(crate) const fn saturate_i64(value: i128) -> i64 {
    if value > i64::MAX as i128 {
        i64::MAX
    } else if value < i64::MIN as i128 {
        i64::MIN
    } else {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "INVARIANT: value is within i64 here"
        )]
        let narrowed = value as i64;
        narrowed
    }
}

#[cfg(test)]
mod tests {
    use super::{div_round_half_even, saturate_i16, shr_round_half_even};

    #[test]
    fn shift_rounds_ties_to_even_in_both_signs() {
        // 2^4 = 16: 8/16 = 0.5 -> 0; 24/16 = 1.5 -> 2; 40/16 = 2.5 -> 2; -8/16 -> 0; -24/16 -> -2.
        let cases = [
            (8, 0),
            (24, 2),
            (40, 2),
            (-8, 0),
            (-24, -2),
            (-40, -2),
            (9, 1),
            (-9, -1),
        ];
        for (value, expected) in cases {
            assert_eq!(shr_round_half_even(value, 4), expected, "{value} / 16");
        }
    }

    #[test]
    fn division_rounds_ties_to_even_for_every_sign_combination() {
        let cases = [
            (5, 2, 2),
            (7, 2, 4),
            (-5, 2, -2),
            (5, -2, -2),
            (-5, -2, 2),
            (1, 3, 0),
            (2, 3, 1),
        ];
        for (n, d, expected) in cases {
            assert_eq!(div_round_half_even(n, d), Some(expected), "{n} / {d}");
        }
        assert_eq!(div_round_half_even(1, 0), None);
    }

    #[test]
    fn saturation_clamps_at_both_bounds() {
        assert_eq!(saturate_i16(40_000), i16::MAX);
        assert_eq!(saturate_i16(-40_000), i16::MIN);
        assert_eq!(saturate_i16(-7), -7);
    }
}
