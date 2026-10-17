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
//! - `rh-cli run <world> …` (SPR-012 T-08: `smoke`, or a manifest world such as `solo`, with
//!   `--strict` refusing a manifest that names a system this build lacks), `rh-cli ladder compare <a> <b>`, `rh-cli ladder cut-yearly <monthly> <out>` and `rh-cli bisect …`: the golden
//!   runs, their hash ladders and the divergence bisect (SPR-008; `rh_golden::cli` documents the
//!   flags and exit codes).
//! - `rh-cli tail <run-dir> …` (SPR-013 T-10): the run's sealed World Stream as display or
//!   canonical lines, following new segments with `--wait` (`rh_golden::tail`).
//! - `rh-cli sift <run-dir>` (SPR-013 T-11): channel C (sift matches) from the run's sealed
//!   channel A, downstream of the engine (`rh_golden::sift`).
//! - `rh-cli replay <run-dir> --into D …` (SPR-013 T-12): the run rebuilt from its `run.toml` and
//!   journal alone and compared with it (`rh_golden::replay`). `--into` must be absent, empty or a
//!   previous replay's output, which holds the marker file `.rh-replay`
//!   (`rh_golden::replay::REPLAY_MARKER`), and must not be, be inside or contain the run directory.
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.10_world_stream.md`.
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use rh_manifest::Registry;
use rh_manifest::manifest;
use rh_registry::ownership::{AccessSpec, Ownership};

mod serve;

const USAGE: &str = "usage: rh-cli manifest validate <manifest.yaml> [--registry <registry.toml>]\n       rh-cli manifest schema [--diff <reference.schema.json>]\n       rh-cli ownership [--specs <dir>] [--registry <registry.toml>] [<access.toml> ...]\n       rh-cli run smoke [--repo R] [--seed S] --years N [--until Y] [--threads T] [--ladder yearly|monthly] --out F [--journal J] [--keyframes D] [--resume K [--extend]] [--dump M:F] [--inject M:TABLE:ROW] [--leak-thread] [--require-parallel PERCENT] [--vitals]\n       rh-cli run <world> [--strict] [--seed S] --years N [--threads T] [--ladder yearly|monthly] --out F [--journal J] [--keyframes D] [--resume K [--extend]] [--vitals] [--run-dir D] [--inputs SCRIPT] [--expect-serial]\n       rh-cli tail <run-dir> [--from-seq N] [--to-seq M] [--follow PERSON] [--kind K ...] [--canonical] [--wait]\n       rh-cli sift <run-dir>\n       rh-cli replay <run-dir> --into D [--from-keyframe TICK] [--threads T]   (D: absent, empty, or a previous replay's, holding .rh-replay)\n       rh-cli canonical parse < LINES | rh-cli canonical files F...\n       rh-cli ladder compare <a.ladder> <b.ladder>\n       rh-cli ladder cut-yearly <monthly.ladder> <out.ladder>\n       rh-cli bisect --baseline BIN --candidate BIN [--candidate-arg A ...] [--seed S] [--years N] [--threads T] --work DIR [--out REPORT]";
/// The repository the golden runs read `worlds/` from: this binary's own source tree, so a build
/// of another revision runs that revision's world (SPR-008 T-03).
const DEFAULT_REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
/// The defaults are found from the source tree, not the working directory, as the reference finds
/// its Registry next to itself (SPR-004 DEF-7).
const DEFAULT_SPECS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/systems");
const DEFAULT_REGISTRY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../specs/manifest/registry_v0.toml"
);

#[allow(
    clippy::disallowed_methods,
    reason = "tool plane: command-line arguments and environment overrides (SPR-011 A-4)"
)]
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
        // BCK-002 A-52: bin/regen_baseline.sh refuses a build that does not print `mutants: none`.
        ["build-info"] => {
            let mutants = rh_golden::run::mutant_features();
            if mutants.is_empty() {
                println!("mutants: none");
                ExitCode::SUCCESS
            } else {
                println!("mutants: {}", mutants.join(","));
                ExitCode::from(1)
            }
        }
        ["manifest", "validate", rest @ ..] => manifest_validate(rest),
        ["manifest", "schema"] => {
            println!("{:#}", manifest::schema::manifest_schema());
            ExitCode::SUCCESS
        }
        ["manifest", "schema", "--diff", reference] => schema_diff(reference),
        ["ownership", rest @ ..] => ownership(rest),
        ["run", "smoke", rest @ ..] => golden(rh_golden::cli::run_smoke, rest),
        // SPR-026 T-03: the run serving channel A's live edge on its socket.
        ["run", world, rest @ ..] if !world.starts_with('-') && rest.contains(&"--socket") => {
            ExitCode::from(serve::run(world, rest, std::path::Path::new(DEFAULT_REPO)))
        }
        // SPR-012 T-08: a manifest world under worlds/ (its world.toml).
        ["run", world, rest @ ..] if !world.starts_with('-') => {
            let words: Vec<String> = rest.iter().map(|s| (*s).to_owned()).collect();
            ExitCode::from(rh_golden::cli::run_world(
                world,
                &words,
                std::path::Path::new(DEFAULT_REPO),
            ))
        }
        ["ladder", "compare", a, b] => {
            ExitCode::from(rh_golden::cli::ladder_compare(a.as_ref(), b.as_ref()))
        }
        ["ladder", "cut-yearly", monthly, out] => ExitCode::from(
            rh_golden::cli::ladder_cut_yearly(monthly.as_ref(), out.as_ref()),
        ),
        ["bisect", rest @ ..] => golden(rh_golden::cli::bisect_cmd, rest),
        // SPR-026 T-05: the live edge, through the socket.
        ["tail", rest @ ..] if rest.contains(&"--live") => {
            ExitCode::from(serve::tail_live(rest, std::path::Path::new(DEFAULT_REPO)))
        }
        // SPR-013 T-10: the sealed World Stream of a run directory.
        ["tail", rest @ ..] => golden(rh_golden::cli::tail_cmd, rest),
        // SPR-026 T-04: channel C sifted as channel A seals, served on C.sock with --socket.
        ["sift", rest @ ..] if rest.contains(&"--follow") => {
            ExitCode::from(serve::sift(rest, std::path::Path::new(DEFAULT_REPO)))
        }
        // SPR-013 T-11: channel C from the run's sealed channel A (downstream of the engine).
        ["sift", rest @ ..] => golden(rh_golden::cli::sift_cmd, rest),
        // SPR-013 T-12: a run rebuilt from its history tuple and journal alone, compared.
        ["replay", rest @ ..] => golden(rh_golden::cli::replay_cmd, rest),
        // REV-SPR-013 MAJ-1: rh-stream's canonical parser, for the differential gate.
        ["canonical", rest @ ..] => golden(rh_golden::cli::canonical_cmd, rest),
        _ => usage(),
    }
}

fn golden(command: fn(&[String], &std::path::Path) -> u8, rest: &[&str]) -> ExitCode {
    let words: Vec<String> = rest.iter().map(|s| (*s).to_owned()).collect();
    ExitCode::from(command(&words, std::path::Path::new(DEFAULT_REPO)))
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
        .and_then(|reg| {
            Ok((
                reg,
                rh_manifest::specs::load_specs(&PathBuf::from(specs_dir))?,
            ))
        });
    let (reg, specs) = match loaded {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(3);
        }
    };
    let system_ids = reg.systems.keys().cloned().collect();
    let table = Ownership::build(&specs, &system_ids);
    let mut accesses: Vec<AccessSpec> = Vec::new();
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
    let from_files = accesses.len();
    // Until SPR-005 no system declares its access in code: without files, each spec's own
    // declaration is checked (writes = owns), which exercises the two-owner and Registry rules.
    let derived = if files.is_empty() {
        table.specs.len()
    } else {
        0
    };
    if files.is_empty() {
        accesses.extend(table.specs.values().map(AccessSpec::from_spec));
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
        "OWNERSHIP: {} columns owned by {} systems; 0 access declarations from code (derived from \
         specs until SPR-005: {derived}; from files: {from_files}); {} violations",
        table.owners.len(),
        table.specs.len(),
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
