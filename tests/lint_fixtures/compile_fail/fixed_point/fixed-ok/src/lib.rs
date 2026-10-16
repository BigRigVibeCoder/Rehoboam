//! Control: well-typed use of rh-core::fixed from a plane-A crate. It must compile, which shows the
//! failures of its sibling crates come from the type barrier, not from a broken fixture.

use rh_core::fixed::{Money, Q8_8, Q16_16};

/// Uses every type through its named conversions only.
pub fn well_typed() -> (Q16_16, Money) {
    let threshold = Q8_8::from_int(1).widen();
    let money = Money::from_minor(Q16_16::ONE.to_bits().into());
    (threshold, money)
}
