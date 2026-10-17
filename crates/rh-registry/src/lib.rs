//! `rh-registry`: the plane-A side of the Mechanics Registry.
//!
//! Plane A, machinery, float-free. Mechanics come only from the Registry; manifests carry no
//! free-string semantics (ADR-0010).
//!
//! - [`ownership`]: single-writer column ownership from `specs/systems/*.odd.toml`, checked
//!   against each system's access declaration (SPR-004 T-03; the declaration is a stub until
//!   SPR-005). It takes the Registry's system ids as input.
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

pub mod ownership;
pub mod world;

pub use world::{DemographyParams, Founder, FounderRole, NamedSystem, Sex, WorldSpec};
