//! A plane-A crate casts Money to an f64. It must not compile: a newtype has no numeric cast.

use rh_core::fixed::{Money, Q8_8, Q16_16};

/// Casts money to a float.
pub fn cast() -> f64 {
    let _ = (Q8_8::ONE, Q16_16::ONE);
    Money::ZERO as f64
}
