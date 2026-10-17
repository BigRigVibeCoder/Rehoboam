//! Fixture: the sampler float zone, computing in floats as the real rh-sample does.

/// A float in the float zone: exempt from the float rule (SPR-003 ruling item 2).
#[must_use]
pub(crate) fn half(x: f64) -> f64 {
    x * 0.5
}
