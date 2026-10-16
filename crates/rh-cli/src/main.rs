//! `rh-cli`: headless command-line tools around the engine.
//!
//! A tool, not plane A: it hosts the engine and reads its stream, and acts on a world only
//! through journaled inputs (ADR-0030).
//!
//! Commands:
//! - `rh-cli manifest validate <manifest.yaml> [--registry <registry.toml>]`: the Rust twin of
//!   `python specs/manifest/manifest_v0.py validate` (SPR-004 T-02), with the same report and
//!   exit codes (0 valid, 1 invalid, 2 structurally invalid).
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.10_world_stream.md`.
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use rh_registry::Registry;
use rh_registry::manifest;

const USAGE: &str = "usage: rh-cli manifest validate <manifest.yaml> [--registry <registry.toml>]";
const DEFAULT_REGISTRY: &str = "specs/manifest/registry_v0.toml";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["--version"] => {
            println!("rh-cli {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["manifest", "validate", rest @ ..] => manifest_validate(rest),
        _ => usage(),
    }
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(64)
}

fn manifest_validate(args: &[&str]) -> ExitCode {
    let (path, registry) = match args {
        [path] => (PathBuf::from(path), PathBuf::from(DEFAULT_REGISTRY)),
        [path, "--registry", reg] | ["--registry", reg, path] => {
            (PathBuf::from(path), PathBuf::from(reg))
        }
        _ => return usage(),
    };
    let reg = match Registry::load(&registry) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(3);
        }
    };
    let outcome = manifest::validate_file(&path, &reg);
    print!("{}", outcome.render(&path.display().to_string()));
    ExitCode::from(outcome.exit_code())
}
