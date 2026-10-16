//! `rh-ids`: typed identifiers, the keyed hash `H` and the counter-based RNG.
//!
//! Plane A, determinism machinery. Every random draw in the engine is keyed:
//! `H(world_seed, branch_salt(t), system_id, purpose, entity_id, t, k)`; there is no other RNG.
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; ADR-0006 (determinism contract); `CLAUDE.md`.
//!
//! Stub created by SPR-001 (repository bootstrap). SPR-002 fills it.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    /// Placeholder so the PR tier has a test to run in every crate; replaced by real tests
    /// when the crate gains code.
    #[test]
    fn stub_crate_builds() {}
}
