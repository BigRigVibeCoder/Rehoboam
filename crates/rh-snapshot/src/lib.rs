//! `rh-snapshot`: keyframes, the plane hashes and input-journal I/O.
//!
//! Plane A, machinery. History is `f(engine_build, registry_version, manifest_hash, seed,
//! input_journal)`; this crate stores and checks that function's inputs and outputs. Plane B and
//! plane C crates may call its hashing; it never depends on them.
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.2_world_model_and_data.md`.
//!
//! Stub created by SPR-001 (repository bootstrap). SPR-007 fills it.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    /// Placeholder so the PR tier has a test to run in every crate; replaced by real tests
    /// when the crate gains code.
    #[test]
    fn stub_crate_builds() {}
}
