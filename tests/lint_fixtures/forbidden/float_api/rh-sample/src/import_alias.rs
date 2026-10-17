//! A float type renamed by an import, then exposed under the new name (VER-SPR-003-TEST GAP-11b).

use core::primitive::f64 as Fl;

/// The probability, as a float, under an imported name.
#[must_use]
pub fn leak() -> Fl {
    0.25
}
