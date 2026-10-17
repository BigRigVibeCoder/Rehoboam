//! A plane-A crate compares a Prob32 with a Q16_16 without a named conversion. It must not compile
//! (SPR-011 T-01; `05.1` §6 item 4: no implicit mixing).

use rh_core::fixed::{Prob32, Q16_16};

/// Mixes two probability types implicitly.
pub fn mixed() -> bool {
    Prob32::ONE == Q16_16::ONE
}
