//! `rh-contracts`: column types and read accessors, one module per owning system.
//!
//! Plane A, contract crate. It holds no implementation: systems read columns they do not own
//! through these accessors and request changes through lifecycle commands (single writer).
//! Changing it is a C2 change (docs/13 §4.2).
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.1_simulation_core.md` §2.
//!
//! Stub created by SPR-001 (repository bootstrap). The first system that owns columns fills it.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    /// Placeholder so the PR tier has a test to run in every crate; replaced by real tests
    /// when the crate gains code.
    #[test]
    fn stub_crate_builds() {}
}
