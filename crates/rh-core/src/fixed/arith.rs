//! Arithmetic of `Q8_8` and `Q16_16`: saturating add, subtract, negate and multiply, and the
//! guarded division of the effect language.
//!
//! Multiply and divide compute the exact result in `i128`, round it once half to even
//! (`round.rs`), then saturate. There is deliberately no `Div` operator: the only division is
//! `checked_div`, which returns `None` for a zero divisor (`05.5` §2.1, "no division by a
//! non-constant without a guard").
//!
//! DECISION: `checked_div` returns `None` only for a zero divisor; an overflowing quotient
//! saturates like every other result: `MIN / -ONE` is `MAX` (pinned by a golden row; ruled by the
//! Architect on the Tester's GAP-1, 2026-10-16). `Money::checked_div_int` follows the same rule
//! (`MIN / -1` is `MAX`). ALTERNATIVES CONSIDERED: `None` on overflow too, which would
//! make callers handle two different failures where the effect language defines only one (the
//! guard) and saturation everywhere else.
//!
//! REF: `CODEX/05_PROJECT/SPR-010_FixedPoint.md` T-02; `05.5` §2.1.

use core::ops::{Add, Mul, Neg, Sub};

use super::q::{Q8_8, Q16_16};
use super::round::{div_round_half_even, saturate_i16, saturate_i32, shr_round_half_even};

macro_rules! fixed_point_arithmetic {
    ($name:ident, frac_bits = $frac:expr, saturate = $saturate:ident) => {
        impl $name {
            /// Adds, saturating at `MIN` and `MAX`.
            #[must_use]
            pub const fn saturating_add(self, rhs: Self) -> Self {
                Self::from_bits(self.to_bits().saturating_add(rhs.to_bits()))
            }

            /// Subtracts, saturating at `MIN` and `MAX`.
            #[must_use]
            pub const fn saturating_sub(self, rhs: Self) -> Self {
                Self::from_bits(self.to_bits().saturating_sub(rhs.to_bits()))
            }

            /// Negates, saturating: `-MIN` is `MAX`.
            #[must_use]
            pub const fn saturating_neg(self) -> Self {
                Self::from_bits(self.to_bits().saturating_neg())
            }

            /// Multiplies, rounding half to even, then saturating at `MIN` and `MAX`.
            #[must_use]
            pub const fn saturating_mul(self, rhs: Self) -> Self {
                // The exact product, scaled by 2^(2 * FRAC_BITS); it always fits in i128.
                let exact = (self.to_bits() as i128) * (rhs.to_bits() as i128);
                Self::from_bits($saturate(shr_round_half_even(exact, $frac)))
            }

            /// Divides: the guarded division of the effect language (`05.5` §2.1).
            ///
            /// Returns `None` if and only if `rhs` is zero. Otherwise the quotient is rounded half
            /// to even and saturated at `MIN` and `MAX` (`MIN / -ONE` is `MAX`).
            #[must_use]
            pub const fn checked_div(self, rhs: Self) -> Option<Self> {
                let scaled = (self.to_bits() as i128) << $frac;
                match div_round_half_even(scaled, rhs.to_bits() as i128) {
                    Some(bits) => Some(Self::from_bits($saturate(bits))),
                    None => None,
                }
            }
        }

        /// Saturating addition (`saturating_add`): the effect language's `+`.
        impl Add for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                self.saturating_add(rhs)
            }
        }

        /// Saturating subtraction (`saturating_sub`): the effect language's `-`.
        impl Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                self.saturating_sub(rhs)
            }
        }

        /// Saturating multiplication, rounded half to even (`saturating_mul`).
        impl Mul for $name {
            type Output = Self;
            fn mul(self, rhs: Self) -> Self {
                self.saturating_mul(rhs)
            }
        }

        /// Saturating negation (`saturating_neg`).
        impl Neg for $name {
            type Output = Self;
            fn neg(self) -> Self {
                self.saturating_neg()
            }
        }
    };
}

// SPR-016: `Q32_32` (`q32.rs`) has the same arithmetic, from the same macro.
pub(super) use fixed_point_arithmetic;

fixed_point_arithmetic!(Q8_8, frac_bits = 8, saturate = saturate_i16);
fixed_point_arithmetic!(Q16_16, frac_bits = 16, saturate = saturate_i32);

#[cfg(test)]
mod tests {
    use super::{Q8_8, Q16_16};

    #[test]
    fn operators_are_the_saturating_methods() {
        assert_eq!(Q16_16::MAX + Q16_16::EPSILON, Q16_16::MAX);
        assert_eq!(Q16_16::MIN - Q16_16::EPSILON, Q16_16::MIN);
        assert_eq!(-Q8_8::MIN, Q8_8::MAX);
        assert_eq!(Q8_8::from_int(100) * Q8_8::from_int(2), Q8_8::MAX);
        assert_eq!(Q8_8::from_int(100) * Q8_8::from_int(-2), Q8_8::MIN);
    }

    #[test]
    fn multiplication_rounds_ties_to_even() {
        let half = Q16_16::from_bits(0x8000);
        assert_eq!(Q16_16::EPSILON * half, Q16_16::ZERO); // 0.5 step -> 0
        assert_eq!(Q16_16::from_bits(3) * half, Q16_16::from_bits(2)); // 1.5 steps -> 2
        assert_eq!(Q16_16::from_bits(-3) * half, Q16_16::from_bits(-2)); // -1.5 steps -> -2
    }

    #[test]
    fn checked_div_is_none_only_for_zero() {
        assert_eq!(Q16_16::ONE.checked_div(Q16_16::ZERO), None);
        assert_eq!(Q16_16::MIN.checked_div(-Q16_16::ONE), Some(Q16_16::MAX));
        assert_eq!(
            Q16_16::ZERO.checked_div(Q16_16::EPSILON),
            Some(Q16_16::ZERO)
        );
        assert_eq!(
            Q8_8::ONE.checked_div(Q8_8::from_int(3)),
            Some(Q8_8::from_bits(85))
        );
    }
}
