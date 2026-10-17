//! A plane-A crate casts a Prob32 to an f64. It must not compile (SPR-011 T-01; `05.1` §6 item 4).

use rh_core::fixed::Prob32;

/// Casts a probability to a float.
pub fn to_float() -> f64 {
    Prob32::ONE as f64
}
