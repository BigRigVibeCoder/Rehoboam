//! `rh-events`: the event schema, causal edges and emission-order `EventId`s.
//!
//! Plane A, machinery. `EventId` order is emission order; outcomes are timers and are never
//! back-dated (ADR-0025).
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; ADR-0007; ADR-0025.
//!
//! Stub created by SPR-001 (repository bootstrap). SPR-006 fills it.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    /// Placeholder so the PR tier has a test to run in every crate; replaced by real tests
    /// when the crate gains code.
    #[test]
    fn stub_crate_builds() {}
}
