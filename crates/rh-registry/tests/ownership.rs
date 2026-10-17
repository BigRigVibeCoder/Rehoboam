//! The ownership lint on the real specs (SPR-004 T-03; DEF-8). Moved out of `src/ownership.rs` by
//! BCK-002 A-51: `rh-registry` keeps pure parsers, and the spec files are read here by the test
//! helper.
#![allow(clippy::unwrap_used, reason = "test helpers fail loudly")]

use std::path::{Path, PathBuf};

use rh_registry::ownership::{AccessSpec, DeclaredRead, OwnedColumn, Ownership, SystemSpec};
use spec_files::{load_specs, registry_system_ids};

mod spec_files;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn table() -> Ownership {
    let reg = registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).unwrap();
    let specs = load_specs(&repo().join("specs/systems")).unwrap();
    Ownership::build(&specs, &reg)
}

fn access(system: &str, reads: &[&str], writes: &[&str]) -> AccessSpec {
    AccessSpec {
        system: system.to_owned(),
        reads: reads.iter().map(ToString::to_string).collect(),
        writes: writes.iter().map(ToString::to_string).collect(),
    }
}

#[test]
fn the_specs_agree_with_themselves() {
    let t = table();
    assert_eq!(t.problems, vec![]);
    assert_eq!(t.owners["person.alive"].system, "M2.demography");
    assert_eq!(t.owners["encounter_log.day_index"].system, "S5.daily_life");
    for spec in t.specs.values() {
        assert_eq!(t.check(&AccessSpec::from_spec(spec)), vec![], "{}", spec.id);
    }
}

/// T-03 acceptance: a system writing a column it does not own is rejected.
#[test]
fn writing_an_unowned_column_is_rejected() {
    let t = table();
    let found = t.check(&access(
        "S5.daily_life",
        &[],
        &["encounter_log.day_index", "person.alive"],
    ));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule, "single-writer");
    assert!(
        found[0]
            .message
            .starts_with("writes person.alive, which M2.demography owns")
    );
    let nobody = t.check(&access("M2.demography", &[], &["person.mood"]));
    assert_eq!(
        nobody[0].message,
        "writes person.mood, which no system spec owns"
    );
}

#[test]
fn undeclared_reads_and_unknown_systems_are_rejected() {
    let t = table();
    let found = t.check(&access("M2.demography", &["person.residence"], &[]));
    assert_eq!(found[0].rule, "undeclared-read");
    assert_eq!(t.check(&access("M9.ghost", &[], &[]))[0].rule, "registry");
}

#[test]
fn two_owners_and_plane_leaks_are_found() {
    let reg = registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).unwrap();
    let spec = |id: &str, col: &str, plane: &str, reads: &[&str]| SystemSpec {
        path: PathBuf::from(format!("{id}.odd.toml")),
        id: id.to_owned(),
        owns: vec![OwnedColumn {
            column: col.to_owned(),
            plane: plane.to_owned(),
            ty: "bool".to_owned(),
        }],
        reads: reads
            .iter()
            .map(|c| DeclaredRead {
                column: (*c).to_owned(),
                owner: "M1.regions".to_owned(),
                lag: "lagged".to_owned(),
            })
            .collect(),
        rng: vec![],
        schedule: vec![],
        commands: vec![],
        emits: vec![],
        from_manifest: vec![],
    };
    let t = Ownership::build(
        &[
            spec("M1.regions", "region.pins", "B", &[]),
            spec("M2.demography", "person.alive", "A", &["region.pins"]),
            spec("M3.agriculture", "person.alive", "A", &[]),
        ],
        &reg,
    );
    assert_eq!(t.problems.len(), 1);
    assert!(
        t.problems[0]
            .message
            .contains("which M2.demography already owns")
    );
    let leak = t.check(&access("M2.demography", &["region.pins"], &[]));
    assert_eq!(leak[0].rule, "plane");
}

/// DEF-8: the lint checks the spec fields it relies on, without `lint_odd.py`.
#[test]
fn spec_fields_the_lint_relies_on_are_validated() {
    let reg = registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).unwrap();
    let dir = repo().join("specs/systems");
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap();
    let m2 = read("M2.demography.odd.toml");
    let s7 = read("S7.vows.odd.toml");
    let cases = [
        (
            "M2.demography.odd.toml",
            m2.replacen("plane = \"A\"", "plane = \"D\"", 1),
            "plane",
            "in plane 'D'",
        ),
        (
            "M2.demography.odd.toml",
            m2.replacen("plane = \"A\"", "plane = \"b\"", 1),
            "plane",
            "in plane 'b'",
        ),
        (
            "M2.demography.odd.toml",
            m2.replacen("type = \"i64\"", "type = \"wibble\"", 1),
            "spec",
            "unknown type 'wibble'",
        ),
        (
            "S7.vows.odd.toml",
            s7.replacen("column = \"person.alive\"", "column = \"person.mood\"", 1),
            "single-writer",
            "reads person.mood, which M2.demography's spec does not declare",
        ),
        (
            "S7.vows.odd.toml",
            s7.replacen("owner = \"M2.demography\"", "owner = \"S6.artifacts\"", 1),
            "single-writer",
            "from S6.artifacts, but M2.demography owns it",
        ),
    ];
    for (file, text, rule, needle) in cases {
        let mut specs = load_specs(&dir).unwrap();
        let i = specs
            .iter()
            .position(|s| s.path.file_name().is_some_and(|n| n == file))
            .unwrap();
        specs[i] = SystemSpec::from_toml(&dir.join(file), &text).unwrap();
        let t = Ownership::build(&specs, &reg);
        assert!(
            t.problems
                .iter()
                .any(|v| v.rule == rule && v.message.contains(needle)),
            "{needle}: {:?}",
            t.problems
        );
    }
}

#[test]
fn access_files_parse() {
    let a = AccessSpec::from_toml(
        "[[access]]\nsystem = \"S5.daily_life\"\nwrites = [\"person.alive\"]\n",
    )
    .unwrap();
    assert_eq!(a[0].writes, vec!["person.alive".to_owned()]);
    assert!(AccessSpec::from_toml("[[access]]\nsystem = \"x\"\nowns = []\n").is_err());
}
