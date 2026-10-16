//! Plane-C fixture: a narrative-plane type (a sift match), which plane A must never read.

/// A completed sift match: narrative state (ADR-0024), never read by plane A.
pub struct SiftMatch(pub u64);
