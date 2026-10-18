//! `rh-manifest`: the load-time front end for the Mechanics Registry and world manifests.
//!
//! Tool plane (SPR-004 ruling A): it reproduces the Python reference
//! (`specs/manifest/manifest_v0.py`), which checks and reports floats, so it lives outside plane A.
//! Authored numbers reach plane A only as fixed point, compiled from the literal's exact decimal
//! value (`docs/05_architecture/05.5` §3.2); plane A never sees an `f64`.
//!
//! - [`compile`]: a validated manifest compiled into the plane-A `rh_registry::WorldSpec`
//!   (SPR-011 T-04): exact decimals to fixed point, Registry defaults, founders in genesis order.
//! - [`effect_cases`]: the golden-format case runner of the effect language (SPR-017 T-05).
//! - [`effect_registry`]: the Registry's effect-language sections, the real environment built
//!   from the specs, and the Registry's program refusals (SPR-017 T-06).
//! - [`exact`]: authored numbers as exact decimals, compiled into fixed point (`05.5` §3.2).
//! - [`registry`]: the Registry v0 as typed sections with bounds (SPR-004 T-01).
//! - [`manifest`]: typed manifest loading with the same verdicts, error counts and lines as the
//!   reference (T-02, V-WC-04), and the JSON Schema emitted from the Rust types (T-04).
//! - [`yaml`]: a YAML 1.1 reader that resolves scalars as `PyYAML`'s `safe_load` does.
//! - [`pyfmt`]: Python-compatible number and list formatting for diagnostics.
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.5_world_compiler_and_dsl.md` §3.
#![forbid(unsafe_code)]
#![allow(
    clippy::float_arithmetic,
    clippy::float_cmp,
    reason = "tool plane: load-time validation must reproduce the reference's float checks (manifest_v0.py); lint_arch pins this crate to the tool plane"
)]

pub mod compile;
pub mod effect_cases;
pub mod effect_registry;
pub mod exact;
pub mod manifest;
pub mod pyfmt;
pub mod registry;
pub mod specs;
pub mod yaml;

/// The `mutant-*` features compiled into this build (BCK-002 A-52; the feature forwards to
/// `rh-registry`'s).
pub const MUTANTS: &[&str] = &[
    #[cfg(feature = "mutant-effect")]
    "mutant-effect",
];

pub use compile::{CompileError, CompileErrors, compile, compile_file};
pub use registry::{Bounds, Number, Registry, RegistryError};
