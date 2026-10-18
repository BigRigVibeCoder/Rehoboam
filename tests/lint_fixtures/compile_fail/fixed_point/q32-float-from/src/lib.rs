//! A plane-A crate builds a Q32_32 from an f64. It must not compile: no float conversion exists
//! (SPR-016 T-01; `05.1` §6 item 4).

use rh_core::fixed::Q32_32;

/// Builds a wide fixed-point value from a float.
pub fn from_float() -> Q32_32 {
    Q32_32::from(0.5_f64)
}
