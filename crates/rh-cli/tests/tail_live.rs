//! `rh-cli tail --wait` against a running `rh-cli run` (SPR-013 T-10; V-STR-05 for sealed
//! segments): a tail started before the run's first seal, which follows the segments as they seal,
//! prints exactly the record lines of a tail started after the run, in display and canonical
//! form, for `solo` and the 60-person `hamlet`.
#![allow(
    clippy::disallowed_types,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a tool-crate test that runs rh-cli as child processes; helpers fail loudly"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_rh-cli");

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("tail_live")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
    dir
}

/// The record lines of a tail's output (no status, no comment).
fn records(out: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(out)
        .lines()
        .filter(|l| !l.starts_with("STATUS") && !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_tail_started_mid_run_prints_what_a_tail_after_the_run_prints() {
    for (world, years) in [("solo", "60"), ("hamlet", "40")] {
        for canonical in [false, true] {
            let dir = scratch(&format!("{world}-{canonical}"));
            let mode: &[&str] = if canonical { &["--canonical"] } else { &[] };
            let live = Command::new(BIN)
                .arg("tail")
                .arg(&dir)
                .arg("--wait")
                .args(mode)
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let run = Command::new(BIN)
                .args(["run", world, "--years", years, "--run-dir"])
                .arg(&dir)
                .output()
                .unwrap();
            assert!(
                run.status.success(),
                "{}",
                String::from_utf8_lossy(&run.stderr)
            );
            let live = live.wait_with_output().unwrap();
            assert!(live.status.success());
            let after = Command::new(BIN)
                .arg("tail")
                .arg(&dir)
                .args(mode)
                .output()
                .unwrap();
            assert!(after.status.success());
            let (l, a) = (records(&live.stdout), records(&after.stdout));
            assert!(!a.is_empty());
            assert_eq!(l, a, "{world} canonical={canonical}");
            let statuses = String::from_utf8_lossy(&live.stdout)
                .lines()
                .filter(|x| x.contains("STATUS"))
                .count();
            assert_eq!(
                statuses,
                years.parse::<usize>().unwrap(),
                "one status per sealed year"
            );
        }
    }
    println!(
        "T-10: a tail started mid-run prints the after-run tail's lines (solo, hamlet; display, canonical)"
    );
}
