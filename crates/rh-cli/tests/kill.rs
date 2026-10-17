//! `kill -9` and resume (SPR-013 T-09; GOV-002 tier 15; PR #60 / `specs/stream/wire_v1.md` §11).
//!
//! For `solo` and the 60-person `hamlet`, a 30-year streaming run is killed (SIGKILL) at two
//! points, then resumed from its year-10 keyframe into the same run directory:
//! - **during sealing**: after the 4th file of sim-year 15's segment. The kill leaves no partial
//!   segment (only the hidden `.y000015.partial`, which no reader lists), and the resumed run
//!   reseals year 15 to the same bytes;
//! - **mid-year**: after sim-year 14's segment is sealed, with year 15's records in memory. The
//!   resumed run re-emits no seq below year 15: years 10 to 14 verify against their sealed
//!   segments (5 verified, never rewritten), and year 15 on is sealed afresh.
//!
//! Either way the run directory's stream ends byte-identical to an unbroken run's,
//! `STEP_DIGEST`s included.
#![allow(
    clippy::disallowed_types,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a tool-crate test that runs rh-cli as a child process to kill it; helpers fail loudly"
)]

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_rh-cli");
const YEARS: &str = "30";

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("kill")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
    dir
}

fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(at)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                let rel = p.strip_prefix(root).unwrap().display().to_string();
                out.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn run(world: &str, dir: &Path, extra: &[&str]) -> String {
    let out = Command::new(BIN)
        .args(["run", world, "--years", YEARS, "--run-dir"])
        .arg(dir)
        .args(extra)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{world} {extra:?}: {text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    text
}

/// Starts a run that stalls at `stall`, waits for its `STALL` line and kills it with SIGKILL.
fn run_and_kill(world: &str, dir: &Path, stall: &str) -> String {
    let mut child = Command::new(BIN)
        .args(["run", world, "--years", YEARS, "--run-dir"])
        .arg(dir)
        .args(["--test-stall", stall])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let line = BufReader::new(stdout)
        .lines()
        .map(Result::unwrap)
        .find(|l| l.starts_with("STALL"))
        .expect("the run reaches its stall");
    child.kill().unwrap(); // SIGKILL on Unix
    child.wait().unwrap();
    line
}

/// The run directory's stream (the ladders differ: a resumed run's starts at its keyframe).
fn stream_of(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    files(&dir.join("stream"))
}

#[test]
fn a_kill_during_sealing_leaves_no_partial_segment_and_the_resume_reseals_the_same_bytes() {
    for world in ["solo", "hamlet"] {
        let clean = scratch(&format!("{world}-clean-seal"));
        run(world, &clean, &[]);
        let dir = scratch(&format!("{world}-killed-seal"));
        let line = run_and_kill(world, &dir, "seal:15:3");
        assert!(line.contains("seal y000015 file 3"), "{line}");
        let channel = dir.join("stream/t0000000000/A");
        assert!(
            !channel.join("y000015").exists(),
            "{world}: a partial y000015 is visible"
        );
        assert!(channel.join(".y000015.partial").is_dir());
        assert!(channel.join("y000014/segment.parquet").is_file());
        let kf = dir.join("keyframes/year-0010.rhkf");
        let ladder = dir.join("resumed.ladder");
        let summary = run(
            world,
            &dir,
            &[
                "--resume",
                kf.to_str().unwrap(),
                "--out",
                ladder.to_str().unwrap(),
            ],
        );
        assert!(
            summary.contains("stream: 15 segments sealed, 5 verified"),
            "{world}: {summary}"
        );
        assert!(!channel.join(".y000015.partial").exists());
        assert_eq!(
            stream_of(&dir),
            stream_of(&clean),
            "{world}: resealed bytes differ"
        );
        println!(
            "T-09 kill -9 during sealing: {world} resealed sim-year 15 to the unbroken run's bytes"
        );
    }
}

#[test]
fn a_mid_year_kill_and_resume_re_emit_no_seq_below_the_first_unsealed_year() {
    for world in ["solo", "hamlet"] {
        let clean = scratch(&format!("{world}-clean-mid"));
        run(world, &clean, &[]);
        let dir = scratch(&format!("{world}-killed-mid"));
        let line = run_and_kill(world, &dir, "sealed:14");
        assert!(line.contains("sealed y000014"), "{line}");
        let channel = dir.join("stream/t0000000000/A");
        assert!(!channel.join("y000015").exists() && !channel.join(".y000015.partial").exists());
        let before: BTreeMap<String, Vec<u8>> = stream_of(&dir);
        let kf = dir.join("keyframes/year-0010.rhkf");
        let ladder = dir.join("resumed.ladder");
        let summary = run(
            world,
            &dir,
            &[
                "--resume",
                kf.to_str().unwrap(),
                "--out",
                ladder.to_str().unwrap(),
            ],
        );
        assert!(
            summary.contains("stream: 15 segments sealed, 5 verified"),
            "{world}: {summary}"
        );
        let after = stream_of(&dir);
        for (name, bytes) in &before {
            assert_eq!(
                after.get(name),
                Some(bytes),
                "{world}: {name} was rewritten"
            );
        }
        assert_eq!(
            after,
            stream_of(&clean),
            "{world}: the resumed stream differs"
        );
        println!(
            "T-09 mid-year kill -9: {world} verified years 10-14 and resealed 15-29 to the unbroken run's bytes"
        );
    }
}
