//! World Manifest v0: typed loading and validation with the same verdicts and error counts as
//! the Python reference `specs/manifest/manifest_v0.py` (SPR-004 T-02; V-WC-04).
//!
//! The pipeline is the reference's, step for step:
//! 1. YAML text to a value, resolved as `PyYAML`'s `safe_load` does ([`crate::yaml`]);
//! 2. structural validation into [`Manifest`], counting errors as Pydantic does ([`coerce`]);
//!    any error stops here (the reference prints `SCHEMA INVALID` and exits 2);
//! 3. Registry-aware validation ([`check`]): errors make the manifest invalid (exit 1),
//!    warnings do not (exit 0).
//!
//! Syntactic validity is necessary, not sufficient: the liveness and certification gates
//! (05.5 §4.2) live in the engine.

pub mod check;
pub mod coerce;
pub mod types;

use std::fmt::Write as _;
use std::path::Path;

pub use check::{Issue, Level, validate_against_registry};
pub use coerce::SchemaError;
pub use types::*;

use crate::registry::Registry;
use crate::yaml::{self, YamlError};

/// What validating one manifest found.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The file could not be read or is not loadable YAML. The reference raises an uncaught
    /// exception here (exit 1, no verdict line).
    LoadError(String),
    /// The manifest does not have the manifest's structure; every structural error.
    SchemaInvalid(Vec<SchemaError>),
    /// The manifest is structurally valid and was checked against the Registry.
    Checked {
        /// The typed manifest.
        manifest: Box<Manifest>,
        /// Registry-aware findings, errors and warnings, in the reference's order.
        issues: Vec<Issue>,
    },
}

/// The verdict on one manifest, as the parity test compares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Loaded and no Registry error.
    Valid,
    /// Loaded, but at least one Registry error.
    Invalid,
    /// Structurally invalid.
    SchemaInvalid,
    /// Not loadable at all.
    LoadError,
}

impl Outcome {
    /// The verdict.
    #[must_use]
    pub fn verdict(&self) -> Verdict {
        match self {
            Outcome::LoadError(_) => Verdict::LoadError,
            Outcome::SchemaInvalid(_) => Verdict::SchemaInvalid,
            Outcome::Checked { .. } if self.error_count() > 0 => Verdict::Invalid,
            Outcome::Checked { .. } => Verdict::Valid,
        }
    }

    /// Number of errors: structural errors, or Registry errors (warnings excluded).
    #[must_use]
    pub fn error_count(&self) -> usize {
        match self {
            Outcome::LoadError(_) => 1,
            Outcome::SchemaInvalid(e) => e.len(),
            Outcome::Checked { issues, .. } => {
                issues.iter().filter(|i| i.level == Level::Error).count()
            }
        }
    }

    /// Number of Registry warnings.
    #[must_use]
    pub fn warning_count(&self) -> usize {
        match self {
            Outcome::Checked { issues, .. } => {
                issues.iter().filter(|i| i.level == Level::Warning).count()
            }
            _ => 0,
        }
    }

    /// The reference's exit code: 0 valid, 1 invalid (or not loadable), 2 structurally invalid.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self.verdict() {
            Verdict::Valid => 0,
            Verdict::Invalid | Verdict::LoadError => 1,
            Verdict::SchemaInvalid => 2,
        }
    }

    /// The report the reference prints for `validate <name>`.
    #[must_use]
    pub fn render(&self, name: &str) -> String {
        let mut s = String::new();
        match self {
            Outcome::LoadError(e) => {
                let _ = writeln!(s, "LOAD ERROR: {name}: {e}");
            }
            Outcome::SchemaInvalid(errors) => {
                let n = errors.len();
                let _ = writeln!(s, "SCHEMA INVALID: {name}");
                let _ = writeln!(
                    s,
                    "{n} validation error{} for Manifest",
                    if n == 1 { "" } else { "s" }
                );
                for e in errors {
                    let _ = writeln!(s, "{e}");
                }
            }
            Outcome::Checked { issues, .. } => {
                for i in issues {
                    let _ = writeln!(s, "{i}");
                }
                let verdict = if self.error_count() > 0 {
                    "INVALID"
                } else {
                    "VALID"
                };
                let _ = writeln!(
                    s,
                    "{verdict}: {name} ({} errors, {} warnings) — next gate: liveness burn-in (engine)",
                    self.error_count(),
                    self.warning_count()
                );
            }
        }
        s
    }
}

/// Validates manifest text against a Registry.
#[must_use]
pub fn validate_str(text: &str, reg: &Registry) -> Outcome {
    let value = match yaml::load(text) {
        Ok(v) => v,
        Err(YamlError { message }) => return Outcome::LoadError(message),
    };
    match coerce::manifest(&value) {
        Err(errors) => Outcome::SchemaInvalid(errors),
        Ok(m) => {
            let issues = validate_against_registry(&m, reg);
            Outcome::Checked {
                manifest: Box::new(m),
                issues,
            }
        }
    }
}

/// Reads and validates a manifest file against a Registry.
#[must_use]
pub fn validate_file(path: &Path, reg: &Registry) -> Outcome {
    match std::fs::read_to_string(path) {
        Ok(text) => validate_str(&text, reg),
        Err(e) => Outcome::LoadError(format!("cannot read {}: {e}", path.display())),
    }
}

#[cfg(test)]
mod tests;
