//! A plane-A crate converts a Q32_32 into a Prob32 through `Into`. It must not compile (SPR-016
//! T-01: the conversion is the named `Prob32::from_q32`, which refuses a value outside [0, 1]).

use rh_core::fixed::{Prob32, Q32_32};

/// Converts implicitly.
pub fn converted() -> Prob32 {
    Q32_32::ONE.into()
}
