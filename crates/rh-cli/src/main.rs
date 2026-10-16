//! `rh-cli`: headless command-line tools around the engine.
//!
//! A tool, not plane A: it hosts the engine and reads its stream, and acts on a world only
//! through journaled inputs (ADR-0030).
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.10_world_stream.md`.
//!
//! Stub created by SPR-001 (repository bootstrap); commands arrive with SPR-009 and later.
#![forbid(unsafe_code)]

use std::process::ExitCode;

/// Prints the tool's name and version; there are no commands yet.
fn main() -> ExitCode {
    println!("rh-cli {}", env!("CARGO_PKG_VERSION"));
    ExitCode::SUCCESS
}
