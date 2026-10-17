//! The RNG `system_id` layout against the independent reference (SPR-005; Architect ruling on
//! decision 2).
//!
//! `system_id(id)` is the high 32 bits of `h64_bytes(tag("rehoboam/rh-core/system/v1"), id)`
//! (`05.1` §6 item 2). `bin/gen_rng_golden.py` computes it independently, for every system with a
//! spec, into the rh-ids golden file (`system` lines); this test reproduces each line. A change to
//! the layout, or renaming a system, changes every draw of that system: a C3 change.
//!
//! Refs: SPR-005 T-04; `05.1` §6 item 2; V-DET-09 style known-answer test.
#![allow(
    clippy::expect_used,
    reason = "test helpers: a malformed golden line must fail the test loudly"
)]

use std::path::Path;

use rh_core::register::system_id;

#[test]
fn system_ids_match_the_reference() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rh-ids/tests/golden/rh_ids_v2.txt");
    let text = std::fs::read_to_string(&path).expect("the rh-ids golden file");
    let lines: Vec<Vec<&str>> = text
        .lines()
        .filter(|l| l.starts_with("system "))
        .map(|l| l.split_whitespace().skip(1).collect())
        .collect();
    let names: Vec<&str> = lines.iter().map(|f| f[0]).collect();
    assert_eq!(
        names,
        ["M2.demography", "S5.daily_life", "S6.artifacts", "S7.vows"]
    );
    for f in &lines {
        let expected: u32 = f[1].parse().expect("a u32");
        assert_eq!(system_id(f[0]).to_raw(), expected, "system line {f:?}");
    }
}
