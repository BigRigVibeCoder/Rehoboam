//! V-WC-10: the Rust evaluator reproduces every golden case of the effect language, byte for byte
//! in rendered text with every pinned diagnostic (SPR-017 T-05; `effect_language_v0.md` §13).
//!
//! `RH_EFFECT_GOLDEN` points the test at another golden-format file (`check_all`'s positive
//! control: one altered expected value must fail with `FAIL lit_true`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers fail loudly"
)]

use std::path::PathBuf;

use rh_manifest::effect_cases::{Golden, check_case};
use rh_registry::effect::{EvalOptions, LANGUAGE_VERSION};

#[allow(
    clippy::disallowed_methods,
    reason = "a test-only override of the golden file's path for check_all's positive control, outside plane A"
)]
fn golden_path() -> PathBuf {
    std::env::var_os("RH_EFFECT_GOLDEN").map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../specs/registry/effect_language_golden_v0.toml")
        },
        PathBuf::from,
    )
}

#[test]
fn golden_cases_reproduce() {
    let path = golden_path();
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let golden = Golden::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut failures = Vec::new();
    for case in &golden.cases {
        if let Err(line) = check_case(&golden, case, EvalOptions::default()) {
            failures.push(line);
        }
    }
    for line in &failures {
        println!("{line}");
    }
    let total = golden.cases.len();
    if failures.is_empty() {
        println!("EFFECT LANGUAGE v{LANGUAGE_VERSION} (rust): PASS ({total} cases)");
    } else {
        panic!(
            "EFFECT LANGUAGE v{LANGUAGE_VERSION} (rust): FAIL ({} of {total} cases)",
            failures.len()
        );
    }
}
