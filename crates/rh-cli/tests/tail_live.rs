//! `rh-cli tail --wait` against a running `rh-cli run` (SPR-013 T-10; V-STR-05 for sealed
//! segments): a tail started before the run's first seal, which follows the segments as they seal,
//! prints exactly the record lines of a tail started after the run, in display and canonical
//! form, for `solo` and the 60-person `hamlet`. The overlap is asserted, not assumed: the live
//! tail printed a status line (a sealed segment) before the run process exited (REV-SPR-013).
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a tool-crate test that runs rh-cli as child processes and orders its outputs by the \
              wall clock (the overlap check); helpers fail loudly"
)]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

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
    for (world, years) in [("solo", "120"), ("hamlet", "80")] {
        for canonical in [false, true] {
            let dir = scratch(&format!("{world}-{canonical}"));
            let mode: &[&str] = if canonical { &["--canonical"] } else { &[] };
            let mut live = Command::new(BIN)
                .arg("tail")
                .arg(&dir)
                .arg("--wait")
                .args(mode)
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let stdout = live.stdout.take().unwrap();
            // Every line the live tail prints, with the instant it arrived.
            let reader = std::thread::spawn(move || {
                BufReader::new(stdout)
                    .lines()
                    .map(|l| (Instant::now(), l.unwrap()))
                    .collect::<Vec<_>>()
            });
            let run = Command::new(BIN)
                .args(["run", world, "--years", years, "--run-dir"])
                .arg(&dir)
                .output()
                .unwrap();
            let run_exited = Instant::now();
            assert!(
                run.status.success(),
                "{}",
                String::from_utf8_lossy(&run.stderr)
            );
            assert!(live.wait().unwrap().success());
            let lines = reader.join().unwrap();
            let first_status = lines
                .iter()
                .find(|(_, l)| l.contains("STATUS"))
                .map(|(t, _)| *t)
                .expect("the live tail printed a status line");
            assert!(
                first_status < run_exited,
                "{world}: the live tail printed nothing before the run ended: no overlap"
            );
            let text: String = lines.iter().fold(String::new(), |mut t, (_, l)| {
                t.push_str(l);
                t.push('\n');
                t
            });
            let after = Command::new(BIN)
                .arg("tail")
                .arg(&dir)
                .args(mode)
                .output()
                .unwrap();
            assert!(after.status.success());
            let (l, a) = (records(text.as_bytes()), records(&after.stdout));
            assert!(!a.is_empty());
            assert_eq!(l, a, "{world} canonical={canonical}");
            let statuses = text.lines().filter(|x| x.contains("STATUS")).count();
            assert_eq!(
                statuses,
                years.parse::<usize>().unwrap(),
                "one status per sealed year"
            );
        }
    }
    println!(
        "T-10: a tail started mid-run, which printed a sealed segment before the run ended, prints the after-run tail's lines (solo, hamlet; display, canonical)"
    );
}
