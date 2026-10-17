//! A plane-A crate converts a Prob32 into a Q16_16 through `Into`. It must not compile (SPR-011
//! T-01: the conversion is the named `to_q16_16`, which states its rounding).

use rh_core::fixed::{Prob32, Q16_16};

/// Converts implicitly.
pub fn converted() -> Q16_16 {
    Prob32::ONE.into()
}
