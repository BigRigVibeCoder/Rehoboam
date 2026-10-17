//! Snapshot test of the phase table (SPR-005 T-02).
//!
//! The phase order is part of the determinism contract (`05.1` §4.2), so it is pinned three ways:
//! - against the committed snapshot `tests/snapshots/phase_table.txt` (a change shows in review);
//! - against the table in `docs/05_architecture/05.1_simulation_core.md` §4.2 itself;
//! - against the `PHASES` list of `specs/systems/lint_odd.py`, which validates the specs.
//!
//! The snapshot is never re-recorded to make this pass: a change to the phase order is a C3
//! change with director approval (`docs/13` §4.2).
//!
//! Refs: SPR-005 T-02; `05.1` §4.2; ADR-0005 v1.1 item 5; ADR-0033.
#![allow(
    clippy::expect_used,
    reason = "test helpers: a missing contract file must fail the test loudly"
)]

use std::path::{Path, PathBuf};

use rh_core::phase::Phase;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo().join(rel)).expect(rel)
}

#[test]
fn the_phase_table_matches_the_snapshot() {
    assert_eq!(
        Phase::table(),
        read("crates/rh-core/tests/snapshots/phase_table.txt")
    );
}

#[test]
fn the_phase_table_matches_05_1_section_4_2() {
    let doc = read("docs/05_architecture/05.1_simulation_core.md");
    let section = doc
        .split("### 4.2 ")
        .nth(1)
        .and_then(|s| s.split("\n### ").next())
        .expect("05.1 has a §4.2");
    // Rows look like: | 5a | `Activity` | ... |
    let rows: Vec<(String, String)> = section
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let name = cells.get(2)?.strip_prefix('`')?.strip_suffix('`')?;
            Some(((*cells.get(1)?).to_owned(), name.to_owned()))
        })
        .collect();
    let ours: Vec<(String, String)> = Phase::ALL
        .iter()
        .map(|p| (p.label().to_owned(), p.name().to_owned()))
        .collect();
    assert_eq!(rows, ours);
}

#[test]
fn the_phase_names_match_lint_odd() {
    let lint = read("specs/systems/lint_odd.py");
    let phases_src = lint
        .split("PHASES = [")
        .nth(1)
        .and_then(|s| s.split(']').next())
        .expect("lint_odd.py defines PHASES");
    let names: Vec<&str> = phases_src
        .split(',')
        .map(|s| s.trim().trim_matches('"'))
        .filter(|s| !s.is_empty())
        .collect();
    let ours: Vec<&str> = Phase::ALL.iter().map(|p| p.name()).collect();
    assert_eq!(names, ours);
}

#[test]
fn order_index_and_names_agree() {
    assert_eq!(Phase::ALL.len(), 12);
    for (i, p) in Phase::ALL.into_iter().enumerate() {
        assert_eq!(usize::from(p.index()), i);
        assert_eq!(Phase::from_name(p.name()), Some(p));
    }
    assert!(Phase::ALL.windows(2).all(|w| w[0] < w[1]));
    assert!(Phase::Vital < Phase::Activity && Phase::Activity < Phase::Contagion);
    assert_eq!(Phase::from_name("Realtime"), None);
}
