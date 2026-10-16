//! `rh-cli`: headless command-line tools around the engine.
//!
//! A tool, not plane A: it hosts the engine and reads its stream, and acts on a world only
//! through journaled inputs (ADR-0030).
//!
//! Commands:
//! - `rh-cli manifest validate <manifest.yaml> [--registry <registry.toml>]`: the Rust twin of
//!   `python specs/manifest/manifest_v0.py validate` (SPR-004 T-02), with the same report and
//!   exit codes (0 valid, 1 invalid, 2 structurally invalid).
//! - `rh-cli manifest schema [--diff <reference.schema.json>]`: the JSON Schema emitted from the
//!   Rust manifest types (SPR-004 T-04); with `--diff`, every unexplained difference from the
//!   reference schema, exit 1 if there is any.
//! - `rh-cli ownership [--specs <dir>] [--registry <registry.toml>] [<access.toml> ...]`: the
//!   single-writer ownership lint (SPR-004 T-03). With no access files it checks the access each
//!   spec implies; with files, the `[[access]]` declarations in them. Exit 1 on any violation.
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.10_world_stream.md`.
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use rh_registry::Registry;
use rh_registry::manifest;
use rh_registry::ownership::{self, AccessSpec, Ownership};

const USAGE: &str = "usage: rh-cli manifest validate <manifest.yaml> [--registry <registry.toml>]\n       rh-cli manifest schema [--diff <reference.schema.json>]\n       rh-cli ownership [--specs <dir>] [--registry <registry.toml>] [<access.toml> ...]";
const DEFAULT_SPECS: &str = "specs/systems";
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
        ["manifest", "schema"] => {
            println!("{:#}", manifest::schema::manifest_schema());
            ExitCode::SUCCESS
        }
        ["manifest", "schema", "--diff", reference] => schema_diff(reference),
        ["ownership", rest @ ..] => ownership(rest),
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

fn ownership(args: &[&str]) -> ExitCode {
    let (mut specs_dir, mut registry, mut files) = (DEFAULT_SPECS, DEFAULT_REGISTRY, Vec::new());
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        match (a, it.clone().next()) {
            ("--specs", Some(&d)) => {
                specs_dir = d;
                it.next();
            }
            ("--registry", Some(&r)) => {
                registry = r;
                it.next();
            }
            (f, _) if !f.starts_with("--") => files.push(f),
            _ => return usage(),
        }
    }
    let loaded = Registry::load(&PathBuf::from(registry))
        .map_err(|e| e.to_string())
        .and_then(|reg| Ok((reg, ownership::load_specs(&PathBuf::from(specs_dir))?)));
    let (reg, specs) = match loaded {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(3);
        }
    };
    let table = Ownership::build(&specs, &reg);
    let mut accesses: Vec<AccessSpec> = Vec::new();
    if files.is_empty() {
        accesses.extend(table.specs.values().map(AccessSpec::from_spec));
    }
    for f in &files {
        match std::fs::read_to_string(f)
            .map_err(|e| e.to_string())
            .and_then(|t| AccessSpec::from_toml(&t))
        {
            Ok(a) => accesses.extend(a),
            Err(e) => {
                eprintln!("{f}: {e}");
                return ExitCode::from(3);
            }
        }
    }
    let mut violations = table.problems.clone();
    for a in &accesses {
        violations.extend(table.check(a));
    }
    for note in table.unresolved_reads() {
        println!("NOTE {note}");
    }
    for v in &violations {
        println!("{v}");
    }
    println!(
        "OWNERSHIP: {} columns owned by {} systems; {} access declarations checked; {} violations",
        table.owners.len(),
        table.specs.len(),
        accesses.len(),
        violations.len()
    );
    if violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn schema_diff(reference: &str) -> ExitCode {
    let parsed = std::fs::read_to_string(reference)
        .map_err(|e| e.to_string())
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).map_err(|e| e.to_string()));
    let reference_schema = match parsed {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{reference}: {e}");
            return ExitCode::from(3);
        }
    };
    let generated = manifest::schema::manifest_schema();
    let differences = manifest::schema::differences(&reference_schema, &generated);
    for d in &differences {
        println!("SCHEMA DIFFERENCE {d}");
    }
    let defs = generated
        .get("$defs")
        .and_then(serde_json::Value::as_object)
        .map_or(0, serde_json::Map::len);
    if differences.is_empty() {
        println!(
            "SCHEMA: no unexplained difference from {reference} ({defs} definitions and the root compared)"
        );
        ExitCode::SUCCESS
    } else {
        println!(
            "SCHEMA: {} unexplained differences from {reference}",
            differences.len()
        );
        ExitCode::from(1)
    }
}
