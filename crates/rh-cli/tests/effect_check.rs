//! `rh-cli effect check` refuses a `[commands]` signature that names no known entity kind, and one
//! that declares a parameter twice, by name (the Tester's GAP-5 on SPR-017 T-06; the Architect's
//! ruling). Each is a planted copy of the real Registry.

#![allow(
    clippy::disallowed_types,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a tool-crate test that runs rh-cli as a child process; helpers fail loudly"
)]

use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_rh-cli");
const REGISTRY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../specs/manifest/registry_v0.toml"
);

/// `rh-cli effect check` on the real Registry with `old` replaced by `new`: (exit code, output).
fn check_planted(name: &str, old: &str, new: &str) -> (i32, String) {
    let text = std::fs::read_to_string(REGISTRY).unwrap();
    assert_eq!(text.matches(old).count(), 1, "plant anchor {old:?}");
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&path, text.replacen(old, new, 1)).unwrap();
    let out = Command::new(BIN)
        .args(["effect", "check", "--registry"])
        .arg(&path)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), text)
}

#[test]
fn an_unknown_entity_kind_is_refused_by_name() {
    let (code, out) = check_planted(
        "registry_unknown_kind.toml",
        r#"Promote         = { owner = "T1.minds", person = "person", outcome = false, params = [] }"#,
        r#"Promote         = { owner = "T1.minds", person = "person", outcome = false, params = [{ name = "whom", type = "nonesuch" }] }"#,
    );
    assert_ne!(code, 0, "{out}");
    assert!(
        out.contains("commands.Promote.whom: E_UNKNOWN_NAME: 'nonesuch'"),
        "{out}"
    );
}

#[test]
fn a_parameter_declared_twice_is_refused_by_name() {
    let (code, out) = check_planted(
        "registry_duplicate_param.toml",
        r#"ClearGoal       = { owner = "T1.minds", person = "holder", outcome = false, params = [{ name = "vow", type = "vow" }] }"#,
        r#"ClearGoal       = { owner = "T1.minds", person = "holder", outcome = false, params = [{ name = "vow", type = "vow" }, { name = "vow", type = "vow" }] }"#,
    );
    assert_ne!(code, 0, "{out}");
    assert!(
        out.contains("commands.ClearGoal: parameter 'vow' is declared twice, at positions 1 and 2"),
        "{out}"
    );
}

#[test]
fn the_real_registry_passes() {
    let out = Command::new(BIN)
        .args(["effect", "check", "--registry", REGISTRY])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .contains("EFFECT PROGRAMS: PASS (7 laws, 21 programs)")
    );
}
