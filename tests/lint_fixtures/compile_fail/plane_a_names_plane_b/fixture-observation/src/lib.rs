//! Plane-B fixture: an observation-plane type (a pin), which plane A must never read.

/// A pin on a person: observation state (ADR-0024), never read by plane A.
pub struct Pin(pub u64);
