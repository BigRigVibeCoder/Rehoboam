//! A plane-A crate converts a Q16_16 into an f64. It must not compile: no float conversion exists.

use rh_core::fixed::{Money, Q8_8, Q16_16};

/// Turns a fixed-point value into a float.
pub fn to_float() -> f64 {
    let _ = (Q8_8::ONE, Money::ZERO);
    f64::from(Q16_16::ONE)
}
