//! A plane-A crate multiplies a Q16_16 by a Prob32. It must not compile (SPR-011 T-01: no
//! arithmetic between Prob32 and another fixed-point type).

use rh_core::fixed::{Prob32, Q16_16};

/// Scales a rate by a probability without a named conversion.
pub fn mixed() -> Q16_16 {
    Q16_16::ONE * Prob32::ONE
}
