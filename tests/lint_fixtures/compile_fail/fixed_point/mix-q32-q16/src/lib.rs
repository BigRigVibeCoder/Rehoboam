//! A plane-A crate compares a Q32_32 with a Q16_16 without a named conversion. It must not compile
//! (SPR-016 T-01: the conversions are the named `Q16_16::widen` and `Q32_32::narrow_saturating`).

use rh_core::fixed::{Q16_16, Q32_32};

/// Mixes two fixed-point types implicitly.
pub fn mixed() -> bool {
    Q32_32::ONE == Q16_16::ONE
}
