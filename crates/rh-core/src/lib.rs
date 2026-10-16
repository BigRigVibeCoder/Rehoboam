//! `rh-core`: the world, its tables, the scheduler, time, the RNG context and invariants.
//!
//! Plane A, the core. `rh-core` has no I/O; every host (CLI, server, Python wheel) wraps it.
//! Plane-A code never reads planes B or C (ADR-0024).
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.1_simulation_core.md`.
//!
//! Stub created by SPR-001 (repository bootstrap). SPR-005 fills it.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    /// Placeholder so the PR tier has a test to run in every crate; replaced by real tests
    /// when the crate gains code.
    #[test]
    fn stub_crate_builds() {}
}
