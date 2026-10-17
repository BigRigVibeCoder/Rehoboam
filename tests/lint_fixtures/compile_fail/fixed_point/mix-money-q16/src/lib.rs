//! A plane-A crate passes a Q16_16 where Money is expected. It must not compile.

use rh_core::fixed::{Money, Q8_8, Q16_16};

/// Treats a rate as an amount of money.
pub fn mixed() -> Money {
    let _ = Q8_8::ONE;
    let amount: Money = Q16_16::ONE;
    amount
}
