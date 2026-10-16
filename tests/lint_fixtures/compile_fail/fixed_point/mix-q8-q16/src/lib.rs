//! A plane-A crate compares a Q8_8 with a Q16_16 without a named conversion. It must not compile.

use rh_core::fixed::{Money, Q8_8, Q16_16};

/// Mixes two fixed-point types implicitly.
pub fn mixed() -> bool {
    let _ = Money::ZERO;
    Q8_8::ONE == Q16_16::ONE
}
