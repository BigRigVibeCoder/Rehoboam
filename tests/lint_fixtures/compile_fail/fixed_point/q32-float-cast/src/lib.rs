//! A plane-A crate casts a Q32_32 to an f64. It must not compile (SPR-016 T-01; `05.1` §6 item 4).

use rh_core::fixed::Q32_32;

/// Casts a wide fixed-point value to a float.
pub fn to_float() -> f64 {
    Q32_32::ONE as f64
}
