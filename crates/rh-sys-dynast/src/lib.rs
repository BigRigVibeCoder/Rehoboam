//! `rh-sys-dynast`: the system `M7.dynasty` dynastic succession subsystem (`SPEC-002`).
//!
//! Governed by: GOV-003, GOV-004, SPEC-002, SPR-021.
#![forbid(unsafe_code)]

pub mod succession;

pub use succession::{DynasticSuccessionArbitrator, SuccessionF1, SuccessionOutcome};
