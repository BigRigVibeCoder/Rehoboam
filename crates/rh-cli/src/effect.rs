//! `rh-cli effect …`: the effect language's tool commands (SPR-017).
//!
//! - `rh-cli effect check [--registry <registry.toml>] [--specs <dir>]` (T-06): compiles every
//!   Registry program (the succession laws, the verbs' `effects`) against the environment built
//!   from the Registry and the system specs, and prints `EFFECT PROGRAMS: PASS (L laws, P
//!   programs)`, or each problem by name and `EFFECT PROGRAMS: FAIL (N problems)`. Exit 0 on a
//!   pass, 1 on a problem, 3 when a file cannot be read or the Registry does not load, 64 on a bad
//!   argument.
//! - `rh-cli effect keys [--registry <registry.toml>]` (T-06): the ids of every keyed top-level
//!   table of the Registry in the order the loader takes them (source order: an enum's ordinal is
//!   its index, §3.1), one `section: id id …` line each, which `bin/check_key_order.py` compares
//!   with `tomllib`'s order.
//! - `rh-cli effect eval <golden-format file> [--mutant <Name>]` (T-08): every case of the file
//!   through the Rust evaluator (`rh_manifest::effect_cases`), one line each, as
//!   `effect_ref.py --emit` prints them: `id result`, then `name=value` for the four diagnostics
//!   when the case evaluated, or `id crash:<message>`. Cases need not carry `expect`. `--mutant`
//!   plants a T-05 mutant, in a build with `--features rh-manifest/mutant-effect` only (the fuzz's
//!   power); `rh-cli run` refuses such a build (BCK-002 A-52, through `rh_manifest::MUTANTS`).
//!
//! REF: `specs/registry/effect_language_v0.md` §7.3, §7.4; `rh_manifest::effect_registry`.

use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;

use rh_manifest::Registry;
use rh_manifest::effect_cases::Golden;
use rh_manifest::effect_registry::{compile_programs, effect_env, key_order};
use rh_registry::effect::{EvalOptions, Mutant};

/// The diagnostics in `effect_ref.py`'s order.
const DIAGNOSTICS: [&str; 4] = [
    "missing_reads",
    "dropped_actions",
    "neutral_reads",
    "truncated",
];

/// `rh-cli effect <args>`; `None` for arguments it does not take (the caller prints the usage).
pub fn run(args: &[&str], default_registry: &str, default_specs: &str) -> Option<ExitCode> {
    match args {
        ["check", rest @ ..] => check(rest, default_registry, default_specs),
        ["keys"] => Some(keys(default_registry)),
        ["eval", file] => Some(eval(file, None)),
        ["eval", file, "--mutant", name] => Some(eval(file, Some(name))),
        ["keys", "--registry", registry] => Some(keys(registry)),
        _ => None,
    }
}

fn check(args: &[&str], default_registry: &str, default_specs: &str) -> Option<ExitCode> {
    let (mut registry, mut specs_dir) = (default_registry, default_specs);
    let mut it = args.iter();
    while let Some(&flag) = it.next() {
        let value = it.next().copied()?;
        match flag {
            "--registry" => registry = value,
            "--specs" => specs_dir = value,
            _ => return None,
        }
    }
    let loaded = std::fs::read_to_string(registry)
        .map_err(|e| format!("{registry}: {e}"))
        .and_then(|text| {
            let reg = Registry::from_toml(&text).map_err(|e| e.to_string())?;
            Ok((reg, text, spec_texts(Path::new(specs_dir))?))
        });
    let (reg, text, specs) = match loaded {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{e}");
            return Some(ExitCode::from(3));
        }
    };
    let compiled = effect_env(&reg, &text, &specs).and_then(|env| compile_programs(&reg, &env));
    Some(match compiled {
        Ok(programs) => {
            println!(
                "EFFECT PROGRAMS: PASS ({} laws, {} programs)",
                programs.laws.len(),
                programs.count()
            );
            ExitCode::SUCCESS
        }
        Err(problems) => {
            for p in &problems {
                println!("ERROR {p}");
            }
            println!("EFFECT PROGRAMS: FAIL ({} problems)", problems.len());
            ExitCode::from(1)
        }
    })
}

fn eval(file: &str, mutant: Option<&str>) -> ExitCode {
    let mutant = match mutant {
        None => Mutant::None,
        Some(name) => match Mutant::ALL.iter().find(|m| format!("{m:?}") == name) {
            Some(&m) if Mutant::enabled() => m,
            Some(_) => {
                eprintln!("--mutant {name} needs a build with the mutant-effect feature");
                return ExitCode::from(64);
            }
            None => {
                eprintln!("unknown mutant {name}");
                return ExitCode::from(64);
            }
        },
    };
    let golden = match std::fs::read_to_string(file)
        .map_err(|e| format!("{file}: {e}"))
        .and_then(|text| Golden::parse(&text))
    {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(3);
        }
    };
    let opts = EvalOptions { mutant };
    for case in &golden.cases {
        let id = case.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        match golden.run_case(case, opts) {
            Ok(outcome) => {
                let mut diags = String::new();
                if outcome.diags.is_some() {
                    for name in DIAGNOSTICS {
                        // Writing to a String cannot fail.
                        let _ = write!(diags, " {name}={}", outcome.diagnostic(name));
                    }
                }
                println!("{id} {}{diags}", outcome.text);
            }
            Err(e) => println!("{id} crash:{}", e.replace('\n', " ")),
        }
    }
    ExitCode::SUCCESS
}

fn keys(registry: &str) -> ExitCode {
    let order = std::fs::read_to_string(registry)
        .map_err(|e| format!("{registry}: {e}"))
        .and_then(|text| key_order(&text));
    match order {
        Ok(order) => {
            for (section, ids) in order {
                println!("{section}: {}", ids.join(" "));
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(3)
        }
    }
}

/// Every spec file of `dir` (`*.odd.toml`, not `_`-prefixed) as `(file name, text)`, in name order.
fn spec_texts(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };
        if rh_registry::ownership::is_spec_file(&name) {
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            out.push((name, text));
        }
    }
    out.sort();
    Ok(out)
}
