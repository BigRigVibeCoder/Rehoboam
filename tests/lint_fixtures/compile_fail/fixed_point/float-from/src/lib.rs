//! A plane-A crate builds a Q16_16 from an f64. It must not compile: no float conversion exists.

use rh_core::fixed::{Money, Q8_8, Q16_16};

/// Builds a fixed-point value from a float.
pub fn from_float() -> Q16_16 {
    let _ = (Q8_8::ONE, Money::ZERO);
    Q16_16::from(0.5_f64)
}
