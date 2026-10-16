//! Plane-A fixture that tries to read a plane-C type. It must not compile.

/// Reads a sift match: forbidden by ADR-0024 rule 1, and impossible without a dependency edge.
pub fn matched_event(found: &fixture_narrative::SiftMatch) -> u64 {
    found.0
}
