//! `rh-registry`: the Mechanics Registry loader and its validation.
//!
//! Plane A, machinery. Mechanics come only from the Registry; manifests carry no free-string
//! semantics (ADR-0010).
//!
//! - [`registry`]: the Registry v0, typed and bounds-checked (SPR-004 T-01).
//! - [`manifest`]: typed manifest loading with the same verdicts and error counts as the Python
//!   reference `specs/manifest/manifest_v0.py` (T-02, V-WC-04), and the JSON Schema emitted from
//!   the Rust types (T-04).
//! - [`ownership`]: single-writer column ownership from `specs/systems/*.odd.toml`, checked
//!   against each system's access declaration (T-03; the declaration is a stub until SPR-005).
//!
//! Loading happens before a world starts; nothing here runs inside a tick.
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.5_world_compiler_and_dsl.md` §2.1.
#![forbid(unsafe_code)]

pub mod manifest;
pub mod pyfmt;
pub mod registry;
pub mod yaml;

pub use registry::{Bounds, Number, Registry, RegistryError};
