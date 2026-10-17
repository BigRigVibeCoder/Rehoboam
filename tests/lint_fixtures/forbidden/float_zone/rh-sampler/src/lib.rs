//! Fixture: a plane-A crate named like the float zone but not it. Its float must be reported.

/// A float outside the float zone (DET-3 forbids it).
#[must_use]
pub fn half(x: f64) -> f64 {
    x / 2.0
}
