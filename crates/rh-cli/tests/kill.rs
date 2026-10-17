//! `kill -9` and resume (SPR-013 T-09; GOV-002 tier 15; PR #60 / `specs/stream/wire_v1.md` §11).
//!
//! For `solo` and the 60-person `hamlet`, a 60-year streaming run (past their committed inputs) is killed (SIGKILL) at two
//! points, then resumed from its year-10 keyframe into the same run directory:
//! - **during sealing**: after the 4th file of sim-year 15's segment. The kill leaves no partial
//!   segment (only the hidden `.y000015.partial`, which no reader lists), and the resumed run
//!   reseals year 15 to the same bytes;
//! - **mid-year**: inside sim-year 15 (probed: the stall names its tick and the year's records in
//!   memory), year 14 sealed. The resumed run re-emits no seq below year 15: years 10 to 14 verify
//!   against their sealed segments (5 verified, never rewritten), and year 15 on is sealed afresh;
//! - **before the first keyframe** (sim-year 5): `--resume genesis` restarts from genesis through
//!   the journal and verifies years 0 to 4.
//!
//! Every resume takes the defaults (its ladder is `run.ladder`). The run directory ends with the
//! unbroken run's stream, `STEP_DIGEST`s included, its whole `run.ladder` and its file names (no
//! `-wal` or `-shm`, VER-SPR-013-TEST DEF-4), and `rh-cli replay` of it reports `REPLAY EQUAL`
//! (REV-SPR-013 MAJ-3).
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
const YEARS: &str = "60";

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

/// The run directory's stream.
fn stream_of(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    files(&dir.join("stream"))
}

/// VER-SPR-013-TEST DEF-4: a resumed run directory holds the unbroken run's file names (no `-wal`
/// or `-shm` beside the journal, no partial segment).
fn assert_same_names(world: &str, dir: &Path, clean: &Path) {
    let names = |d: &Path| files(d).into_keys().collect::<Vec<_>>();
    assert_eq!(
        names(dir),
        names(clean),
        "{world}: the resumed directory's file names differ"
    );
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
        let summary = run(world, &dir, &["--resume", kf.to_str().unwrap()]);
        assert!(
            summary.contains("stream: 45 segments sealed, 5 verified"),
            "{world}: {summary}"
        );
        assert!(!channel.join(".y000015.partial").exists());
        assert_same_names(world, &dir, &clean);
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

const MPY: i64 = 518_400;
const MONTH: i64 = 43_200;

/// `rh-cli replay <dir> --into <dir>.replayed`: its first line.
fn replay(dir: &Path) -> String {
    let into = dir.with_extension("replayed");
    let _ = std::fs::remove_dir_all(&into); // a stale earlier output would be refused (DEF-2)
    let out = Command::new(BIN)
        .arg("replay")
        .arg(dir)
        .arg("--into")
        .arg(&into)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

/// `rh-cli tail <dir> --wait`, which must end by itself within 60 s; its output.
fn tail_wait(dir: &Path) -> String {
    let mut child = Command::new(BIN)
        .arg("tail")
        .arg(dir)
        .arg("--wait")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    for _ in 0..600 {
        if child.try_wait().unwrap().is_some() {
            let out = child.wait_with_output().unwrap();
            return String::from_utf8_lossy(&out.stdout).into_owned();
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    child.kill().unwrap();
    panic!("tail --wait did not end on a killed run");
}

/// The `STALL mid tick T sim-year Y buffered N` line's tick, year and buffered count.
fn mid(line: &str) -> (i64, i64, usize) {
    let w: Vec<&str> = line.split(' ').collect();
    assert_eq!(&w[..2], ["STALL", "mid"], "{line}");
    (
        w[3].parse().unwrap(),
        w[5].parse().unwrap(),
        w[7].parse().unwrap(),
    )
}

/// REV-SPR-013 MAJ-3 and MIN-10: the kill is probed to be mid-year (inside sim-year 15, past its
/// first instant, with its records in memory and none sealed); the resume takes the defaults (no
/// `--out`), so the run directory's `run.ladder` is the whole run's; and the resumed directory
/// replays from its journal alone, `REPLAY EQUAL`.
#[test]
fn a_mid_year_kill_and_resume_re_emit_no_seq_below_the_first_unsealed_year() {
    for world in ["solo", "hamlet"] {
        let clean = scratch(&format!("{world}-clean-mid"));
        run(world, &clean, &[]);
        let dir = scratch(&format!("{world}-killed-mid"));
        let at = 15 * MPY + 6 * MONTH;
        let line = run_and_kill(world, &dir, &format!("mid:{at}"));
        let (tick, year, buffered) = mid(&line);
        assert!(
            year == 15 && tick > 15 * MPY && tick < 16 * MPY - 1 && buffered > 0,
            "{world}: not mid-year: {line}"
        );
        let channel = dir.join("stream/t0000000000/A");
        assert!(channel.join("y000014/segment.parquet").is_file());
        assert!(!channel.join("y000015").exists() && !channel.join(".y000015.partial").exists());
        let before: BTreeMap<String, Vec<u8>> = stream_of(&dir);
        // REV-SPR-013 MIN-9: a tail --wait on the killed run ends by itself, saying the writer is
        // gone, after printing what was sealed.
        let tail = tail_wait(&dir);
        assert!(
            tail.lines()
                .last()
                .is_some_and(|l| l.starts_with("STATUS writer gone")),
            "{world}: {tail}"
        );
        assert!(tail.contains("sealed through y000014"), "{world}: {tail}");
        let kf = dir.join("keyframes/year-0010.rhkf");
        let summary = run(world, &dir, &["--resume", kf.to_str().unwrap()]);
        assert!(
            summary.contains("stream: 45 segments sealed, 5 verified"),
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
        assert!(
            std::fs::read(dir.join("run.ladder")).unwrap()
                == std::fs::read(clean.join("run.ladder")).unwrap(),
            "{world}: run.ladder is not the whole run's"
        );
        assert_same_names(world, &dir, &clean);
        let replayed = replay(&dir);
        assert!(replayed.starts_with("REPLAY EQUAL"), "{world}: {replayed}");
        println!(
            "T-09 mid-year kill -9: {world} killed at tick {tick} (sim-year {year}, {buffered} records in memory); verified years 10-14, resealed 15-59 to the unbroken run's bytes; run.ladder whole; REPLAY EQUAL"
        );
    }
}

/// REV-SPR-013 MIN-4: a kill before the first keyframe (sim-year 5) is recoverable: `--resume
/// genesis` restarts from genesis through the run's journal, verifies the sealed years 0-4 and
/// seals the rest to the unbroken run's bytes; the directory replays `REPLAY EQUAL`.
#[test]
fn a_kill_before_the_first_keyframe_resumes_from_genesis() {
    for world in ["solo", "hamlet"] {
        let clean = scratch(&format!("{world}-clean-genesis"));
        run(world, &clean, &[]);
        let dir = scratch(&format!("{world}-killed-genesis"));
        let line = run_and_kill(world, &dir, &format!("mid:{}", 5 * MPY + 3 * MONTH));
        let (_, year, _) = mid(&line);
        assert_eq!(year, 5, "{line}");
        assert!(!dir.join("keyframes/year-0010.rhkf").exists());
        let summary = run(world, &dir, &["--resume", "genesis"]);
        assert!(
            summary.contains("stream: 55 segments sealed, 5 verified"),
            "{world}: {summary}"
        );
        assert_eq!(
            stream_of(&dir),
            stream_of(&clean),
            "{world}: stream differs"
        );
        assert!(
            std::fs::read(dir.join("run.ladder")).unwrap()
                == std::fs::read(clean.join("run.ladder")).unwrap(),
            "{world}: run.ladder differs"
        );
        assert_same_names(world, &dir, &clean);
        let replayed = replay(&dir);
        assert!(replayed.starts_with("REPLAY EQUAL"), "{world}: {replayed}");
        println!(
            "T-09 kill -9 before the first keyframe: {world} resumed from genesis, verified years 0-4, the unbroken run's bytes; REPLAY EQUAL"
        );
    }
}
