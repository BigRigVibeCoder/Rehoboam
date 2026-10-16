//! Money: a signed amount in integer minor units (`05.1` §6 item 4: "money (i64 minor units)").
//!
//! The minor unit (a penny, a copper) is the world's smallest coin; the Registry names it. Money is
//! never a fraction of a minor unit, so totals, transfers and ledgers are exact integers.
//!
//! REF: `CODEX/05_PROJECT/SPR-010_FixedPoint.md` T-01.

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
}

#[cfg(test)]
mod tests {
    use super::Money;

    #[test]
    fn minor_units_round_trip_and_order() {
        for minor in [i64::MIN, -1, 0, 1, 12_345, i64::MAX] {
            assert_eq!(Money::from_minor(minor).to_minor(), minor);
        }
        assert!(Money::MIN < Money::from_minor(-1) && Money::from_minor(-1) < Money::ZERO);
        assert_eq!(Money::default(), Money::ZERO);
    }
}
