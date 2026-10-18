//! `rh-registry`: the plane-A side of the Mechanics Registry.
//!
//! Plane A, machinery, float-free. Mechanics come only from the Registry; manifests carry no
//! free-string semantics (ADR-0010).
//!
//! - [`ownership`]: single-writer column ownership from `specs/systems/*.odd.toml`, checked
//!   against each system's access declaration (SPR-004 T-03; the declaration is a stub until
//!   SPR-005). It takes the Registry's system ids as input.
//! - [`effect`]: the Registry effect language v0 (SPR-017): the front end, the evaluator, the
//!   kinship prelude, the succession protocol and verb effects, behind a host seam over the
//!   engine's view (`specs/registry/effect_language_v0.md`).
//! - [`world`]: `WorldSpec`, a manifest compiled for plane A (SPR-011 T-04): fixed point, typed
//!   ids, no string with meaning. `rh_manifest::compile` builds it; this crate never reads a
//!   manifest.
//!
//! The load-time front end (YAML, the float view of the Registry, manifest validation) is the tool
//! crate `rh-manifest` (SPR-004 ruling A). This crate later becomes the fixed-point Registry the
//! engine reads, plus the effect-language evaluator (05.5 §2.1); authored numbers enter it only as
//! fixed point (05.5 §3.2).
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.5_world_compiler_and_dsl.md` §2.1.
#![forbid(unsafe_code)]

pub mod effect;
pub mod ownership;
pub mod world;

/// The `mutant-*` features compiled into this build (BCK-002 A-52): empty in any build a world or a
/// baseline may come from; `rh-cli run` and `bin/regen_baseline.sh` refuse a build where it is not.
pub const MUTANTS: &[&str] = &[
    #[cfg(feature = "mutant-effect")]
    "mutant-effect",
];

pub use world::{DemographyParams, Founder, FounderRole, NamedSystem, Sex, WorldSpec};
