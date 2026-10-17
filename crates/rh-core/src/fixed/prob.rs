//! `Prob32`: an unsigned probability with 32 fraction bits, for draws that need finer steps than
//! `Q16_16` (SPR-011 ruling 4; `05.1` §6 item 4).
//!
//! `Q16_16` resolves a probability to 2⁻¹⁶: a monthly death probability of about 5 × 10⁻⁵ is under
//! four steps, a rounding error of up to 30%. A `Prob32` has the value `bits / 2^32`, with `bits`
//! an integer in `[0, 2^32]` held in a `u64`, so 0 and 1 are both exact. A value above `2^32` is
//! not a `Prob32`: construction from bits refuses it; nothing clamps or wraps.
//!
//! **The mapping** (`05.1` §6 item 4, SPR-010's `Q16_16` mapping at 32 fraction bits). A uniform
//! `u64` draw `d` stands for `u = d / 2^64`, and `p` hits iff `u < p`. In integers: `bits = 2^32`
//! always hits; otherwise `p` hits iff `d < bits · 2^32`, which fits a `u64` for `bits < 2^32`. So
//! `P(hit) = bits / 2^32` exactly: the 2^64 draws split into 2^32 whole blocks of 2^32 draws, one
//! per step, and `p` hits in exactly the first `bits` blocks. `bits = 0` never hits.
//!
//! **No implicit mixing.** There is no `From`, `Into`, arithmetic or comparison between `Prob32` and
//! `Q16_16`, `Q8_8` or `Money`. The conversions are named, with their rounding stated:
//! [`Prob32::from_q16_16`] is exact (`bits · 2^16`) on `[0, 1]` and refuses anything else;
//! [`Prob32::to_q16_16`] rounds half to even. There is no conversion from or to a float, under any
//! feature. Arithmetic on `Prob32` (a complement, a product) arrives with the sprint that needs it.
//!
//! REF: `CODEX/05_PROJECT/SPR-011_WalkingSkeleton.md` T-01, ruling 4; `05.1` §6 item 4; DET-3.

use super::q::Q16_16;
use super::round::div_round_half_even;

/// An unsigned probability with 32 fraction bits: value `bits / 2^32`, `bits` in `[0, 2^32]`.
///
/// The ordering of two values is the ordering of their bits (derived `Ord`). There is deliberately
/// no conversion from or to a float and no operator with another fixed-point type (module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Prob32(u64);

impl Prob32 {
    /// The number of fractional bits: a value is `bits / 2^FRAC_BITS`.
    pub const FRAC_BITS: u32 = 32;
    /// Probability 0: never hits.
    pub const ZERO: Self = Self(0);
    /// Probability 1 (`bits = 2^32`): always hits.
    pub const ONE: Self = Self(1 << Self::FRAC_BITS);
    /// The smallest positive probability, `2^-32`: one step of the type.
    pub const EPSILON: Self = Self(1);

    /// Builds a probability from its raw bits (`bits / 2^32`); `None` if `bits > 2^32`.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Option<Self> {
        if bits <= Self::ONE.0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the raw bits (`self · 2^32`), in `[0, 2^32]`; for storage and hashing.
    #[must_use]
    pub const fn to_bits(self) -> u64 {
        self.0
    }

    /// Builds the probability nearest `numerator / denominator`, rounding `2^32 · numerator /
    /// denominator` half to even (the numerator is computed exactly in `u128`).
    ///
    /// Returns `None` if `denominator` is zero or the rounded value is above 1: a probability that
    /// does not fit is a data error, never silently clamped.
    #[must_use]
    pub const fn from_ratio(numerator: u64, denominator: u64) -> Option<Self> {
        // (2^64 - 1) · 2^32 < 2^96: the scaled numerator and the denominator fit i128 exactly.
        let scaled = (numerator as i128) << Self::FRAC_BITS;
        match div_round_half_even(scaled, denominator as i128) {
            Some(bits) if bits <= Self::ONE.0 as i128 => Some(Self(narrow(bits))),
            _ => None,
        }
    }

    /// Converts a `Q16_16` probability exactly (`bits · 2^16`); `None` outside `[0, 1]`.
    #[must_use]
    pub const fn from_q16_16(p: Q16_16) -> Option<Self> {
        let bits = p.to_bits();
        if bits < 0 || bits > 1 << Q16_16::FRAC_BITS {
            None
        } else {
            // 0 <= bits <= 2^16: the cast is exact and the shift stays below 2^32 + 1.
            Some(Self(
                (bits.cast_unsigned() as u64) << (Self::FRAC_BITS - Q16_16::FRAC_BITS),
            ))
        }
    }

    /// Converts to `Q16_16`, rounding `bits / 2^16` half to even; the result is in `[0, 1]`.
    #[must_use]
    pub const fn to_q16_16(self) -> Q16_16 {
        let shift = Self::FRAC_BITS - Q16_16::FRAC_BITS;
        let floor = self.0 >> shift;
        let remainder = self.0 - (floor << shift);
        let half = 1 << (shift - 1);
        let rounded = if remainder > half || (remainder == half && floor & 1 == 1) {
            floor + 1
        } else {
            floor
        };
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "rounded <= 2^16: bits <= 2^32"
        )]
        Q16_16::from_bits(rounded as i32)
    }

    /// Returns whether the draw `draw` hits this probability: `draw / 2^64 < p` (module docs).
    #[must_use]
    pub const fn hits(self, draw: u64) -> bool {
        if self.0 >= Self::ONE.0 {
            true
        } else {
            // bits < 2^32, so bits · 2^32 < 2^64 and the shift cannot overflow.
            draw < self.0 << Self::FRAC_BITS
        }
    }
}

/// `bits` as a `u64`, for `0 <= bits <= 2^32` (both operands of `from_ratio` are non-negative).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "callers pass 0 <= bits <= 2^32"
)]
const fn narrow(bits: i128) -> u64 {
    bits as u64
}

#[cfg(test)]
mod tests {
    use super::{Prob32, Q16_16};

    #[test]
    fn the_domain_is_zero_to_one_inclusive() {
        assert_eq!(Prob32::from_bits(0), Some(Prob32::ZERO));
        assert_eq!(Prob32::from_bits(1 << 32), Some(Prob32::ONE));
        assert_eq!(Prob32::from_bits((1 << 32) + 1), None);
        assert_eq!(Prob32::from_bits(u64::MAX), None);
        assert_eq!(Prob32::default(), Prob32::ZERO);
        assert!(Prob32::ZERO < Prob32::EPSILON && Prob32::EPSILON < Prob32::ONE);
    }

    #[test]
    fn the_smallest_step_hits_exactly_the_draws_below_two_to_the_32() {
        assert!(Prob32::EPSILON.hits(0));
        assert!(Prob32::EPSILON.hits((1 << 32) - 1));
        assert!(!Prob32::EPSILON.hits(1 << 32));
        let almost_one = Prob32::from_bits((1 << 32) - 1).unwrap_or_default();
        assert!(almost_one.hits(u64::MAX - (1 << 32)));
        assert!(!almost_one.hits(u64::MAX - (1 << 32) + 1));
        for draw in [0, 1, 1 << 32, u64::MAX - 1, u64::MAX] {
            assert!(!Prob32::ZERO.hits(draw));
            assert!(Prob32::ONE.hits(draw));
        }
    }

    #[test]
    fn q16_16_converts_exactly_in_and_rounds_half_to_even_out() {
        assert_eq!(Prob32::from_q16_16(Q16_16::ONE), Some(Prob32::ONE));
        assert_eq!(Prob32::from_q16_16(Q16_16::ZERO), Some(Prob32::ZERO));
        assert_eq!(
            Prob32::from_q16_16(Q16_16::EPSILON).map(Prob32::to_bits),
            Some(1 << 16)
        );
        assert_eq!(Prob32::from_q16_16(-Q16_16::EPSILON), None);
        assert_eq!(Prob32::from_q16_16(Q16_16::ONE + Q16_16::EPSILON), None);
        let p = |bits| Prob32::from_bits(bits).unwrap_or_default();
        assert_eq!(p(1 << 15).to_q16_16(), Q16_16::ZERO); // a half step: to even (0)
        assert_eq!(p(3 << 15).to_q16_16(), Q16_16::from_bits(2)); // 1.5 steps: to even (2)
        assert_eq!(p((1 << 15) + 1).to_q16_16(), Q16_16::EPSILON);
        assert_eq!(Prob32::ONE.to_q16_16(), Q16_16::ONE);
        assert_eq!(p((1 << 32) - 1).to_q16_16(), Q16_16::ONE);
    }

    #[test]
    fn from_ratio_rounds_half_to_even_and_refuses_what_does_not_fit() {
        assert_eq!(
            Prob32::from_ratio(1, 3).map(Prob32::to_bits),
            Some(1_431_655_765)
        );
        assert_eq!(Prob32::from_ratio(1, 1 << 33), Some(Prob32::ZERO)); // half a step: to even
        assert_eq!(Prob32::from_ratio(3, 1 << 33).map(Prob32::to_bits), Some(2));
        assert_eq!(Prob32::from_ratio(7, 7), Some(Prob32::ONE));
        assert_eq!(Prob32::from_ratio(0, 7), Some(Prob32::ZERO));
        assert_eq!(Prob32::from_ratio(1, 0), None);
        assert_eq!(Prob32::from_ratio(8, 7), None);
        assert_eq!(Prob32::from_ratio(u64::MAX, u64::MAX), Some(Prob32::ONE));
        // Just above 1 but rounding down to 2^32 is 1, not a refusal.
        assert_eq!(
            Prob32::from_ratio(u64::MAX, u64::MAX - 1),
            Some(Prob32::ONE)
        );
    }
}
