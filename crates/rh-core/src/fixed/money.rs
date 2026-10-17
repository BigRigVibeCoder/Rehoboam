//! Money: a signed amount in integer minor units (`05.1` §6 item 4: "money (i64 minor units)").
//!
//! The minor unit (a penny, a copper) is the world's smallest coin; the Registry names it. Money is
//! never a fraction of a minor unit, so totals, transfers and ledgers are exact integers.
//!
//! Arithmetic follows the rules of the fixed-point types: saturating, one rounding rule (half to
//! even, `round.rs`) for a rate or a share, and division only through `checked_div_int`.
//!
//! REF: `CODEX/05_PROJECT/SPR-010_FixedPoint.md` T-01, T-02.

use core::ops::{Add, Neg, Sub};

use super::q::Q16_16;
use super::round::{div_round_half_even, saturate_i64, shr_round_half_even};

/// An amount of money in integer minor units (`i64`).
///
/// It does not mix with `Q8_8` or `Q16_16`: there is no conversion and no operator between them.
/// There is no conversion from or to a float.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Money(i64);

impl Money {
    /// No money.
    pub const ZERO: Self = Self(0);
    /// The most negative amount (the deepest debt); every result below it saturates to it.
    pub const MIN: Self = Self(i64::MIN);
    /// The largest amount; every result above it saturates to it.
    pub const MAX: Self = Self(i64::MAX);

    /// Builds an amount from a count of minor units.
    #[must_use]
    pub const fn from_minor(minor_units: i64) -> Self {
        Self(minor_units)
    }

    /// Returns the amount as a count of minor units.
    #[must_use]
    pub const fn to_minor(self) -> i64 {
        self.0
    }

    /// Adds, saturating at `MIN` and `MAX`.
    #[must_use]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Subtracts, saturating at `MIN` and `MAX`.
    #[must_use]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Negates, saturating: `-MIN` is `MAX`.
    #[must_use]
    pub const fn saturating_neg(self) -> Self {
        Self(self.0.saturating_neg())
    }

    /// Multiplies by a count (a price times a quantity), saturating. Exact; nothing to round.
    #[must_use]
    pub const fn saturating_mul_int(self, count: i64) -> Self {
        Self(self.0.saturating_mul(count))
    }

    /// Multiplies by a rate (a tax, a wage share, an interest rate), rounding half to even to a
    /// whole minor unit, then saturating.
    #[must_use]
    pub const fn mul_rate(self, rate: Q16_16) -> Self {
        let exact = (self.0 as i128) * (rate.to_bits() as i128); // scaled by 2^16
        Self(saturate_i64(shr_round_half_even(exact, Q16_16::FRAC_BITS)))
    }

    /// Divides by a count (an equal split), rounding half to even to a whole minor unit.
    ///
    /// Returns `None` if and only if `count` is zero (guarded division, `05.5` §2.1). The only
    /// overflowing case, `MIN / -1`, saturates to `MAX`.
    #[must_use]
    pub const fn checked_div_int(self, count: i64) -> Option<Self> {
        match div_round_half_even(self.0 as i128, count as i128) {
            Some(minor) => Some(Self(saturate_i64(minor))),
            None => None,
        }
    }
}

/// Saturating addition (`saturating_add`).
impl Add for Money {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
}

/// Saturating subtraction (`saturating_sub`).
impl Sub for Money {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

/// Saturating negation (`saturating_neg`).
impl Neg for Money {
    type Output = Self;
    fn neg(self) -> Self {
        self.saturating_neg()
    }
}

#[cfg(test)]
mod tests {
    use super::{Money, Q16_16};

    #[test]
    fn minor_units_round_trip_and_order() {
        for minor in [i64::MIN, -1, 0, 1, 12_345, i64::MAX] {
            assert_eq!(Money::from_minor(minor).to_minor(), minor);
        }
        assert!(Money::MIN < Money::from_minor(-1) && Money::from_minor(-1) < Money::ZERO);
        assert_eq!(Money::default(), Money::ZERO);
    }

    #[test]
    fn arithmetic_saturates_and_rounds_half_to_even() {
        assert_eq!(Money::MAX + Money::from_minor(1), Money::MAX);
        assert_eq!(Money::MIN - Money::from_minor(1), Money::MIN);
        assert_eq!(-Money::MIN, Money::MAX);
        assert_eq!(Money::MAX.saturating_mul_int(-2), Money::MIN);
        let half = Q16_16::from_bits(0x8000);
        assert_eq!(Money::from_minor(5).mul_rate(half), Money::from_minor(2)); // 2.5 -> 2
        assert_eq!(Money::from_minor(7).mul_rate(half), Money::from_minor(4)); // 3.5 -> 4
        assert_eq!(Money::MAX.mul_rate(Q16_16::from_int(2)), Money::MAX);
        assert_eq!(
            Money::from_minor(10).checked_div_int(4),
            Some(Money::from_minor(2))
        ); // 2.5
        assert_eq!(
            Money::from_minor(-10).checked_div_int(4),
            Some(Money::from_minor(-2))
        );
        assert_eq!(Money::from_minor(1).checked_div_int(0), None);
        assert_eq!(Money::MIN.checked_div_int(-1), Some(Money::MAX));
    }
}
