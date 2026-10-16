//! Plane-A fixture that tries to read a plane-B type. It must not compile.

/// Reads a pin: forbidden by ADR-0024 rule 1, and impossible without a dependency edge.
pub fn pinned_person(pin: &fixture_observation::Pin) -> u64 {
    pin.0
}
