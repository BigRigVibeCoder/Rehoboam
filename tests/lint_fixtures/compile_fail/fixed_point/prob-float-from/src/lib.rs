//! A plane-A crate builds a Prob32 from an f64. It must not compile (SPR-011 T-01; `05.1` §6 item
//! 4: no float conversion under any feature).

use rh_core::fixed::Prob32;

/// Builds a probability from a float.
pub fn from_float() -> Prob32 {
    Prob32::from(0.5_f64)
}
