//! `rh-cli tail` (SPR-013 T-10; V-STR-05 for sealed segments).
//!
//! - Every channel-A record of a `solo` run prints exactly once, in `seq` order, from any
//!   `--from-seq` (and up to any `--to-seq`).
//! - The canonical output of one segment is a canonical file whose digest is the segment's
//!   `canonical_digest`, and the canonical output of the whole run digests to the run's records.
//! - Tail never writes to the run directory: every file, its bytes and its modification time are
//!   unchanged after every mode of tail, with and without channel C's annotations.
//! - Snapshots of the display form (`tests/snapshots/*.txt`): `RH_UPDATE_SNAPSHOTS=1` rewrites
//!   them (reviewed in the diff); `RH_TAIL_SNAPSHOTS=DIR` compares with another directory (the
//!   stale-snapshot control must FAIL).
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "test helpers fail loudly; the snapshot switches are environment overrides of a test; \
              the read-only check compares file modification times"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rh_golden::ladder::RungKind;
use rh_golden::run::{RunArgs, run};
use rh_golden::tail::{TailArgs, no_annotations, tail};
use rh_stream::canonical::digest_file;
use rh_stream::writer::read_channel_a;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One run directory per world, made once per test process (nextest runs each test in its own
/// process, so the directory is the process's: two processes never rebuild one directory).
fn run_dir(world: &str) -> PathBuf {
    static DIRS: OnceLock<BTreeMap<String, PathBuf>> = OnceLock::new();
    DIRS.get_or_init(|| {
        let mut out = BTreeMap::new();
        for (w, years) in [("solo", 200), ("hamlet", 120)] {
            let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
                .join("tail")
                .join(format!("{w}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let mut a = RunArgs::new(&repo(), years, 1, RungKind::Yearly, &dir.join("run.ladder"));
            w.clone_into(&mut a.world);
            a.journal = dir.join("journal.sqlite");
            a.run_dir = Some(dir.clone());
            run(&a).unwrap();
            out.insert(w.to_owned(), dir);
        }
        out
    })[world]
        .clone()
}

fn tail_text(args: &TailArgs) -> String {
    let mut out = Vec::new();
    tail(args, &no_annotations, &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn args(world: &str) -> TailArgs {
    TailArgs {
        run_dir: run_dir(world),
        repo: repo(),
        ..TailArgs::default()
    }
}

fn seqs(text: &str) -> Vec<u64> {
    text.lines()
        .filter(|l| !l.starts_with("STATUS") && !l.trim_start().starts_with('^'))
        .map(|l| l.split_whitespace().next().unwrap().parse().unwrap())
        .collect()
}

#[test]
fn every_record_prints_exactly_once_in_seq_order_from_any_cursor() {
    let total: u64 = read_channel_a(&run_dir("solo").join("stream"), 0, 518_400)
        .unwrap()
        .iter()
        .map(|s| s.row.record_count)
        .sum();
    for from in [0, 1, 649, 650, 701, total - 1, total] {
        let text = tail_text(&TailArgs {
            from_seq: from,
            ..args("solo")
        });
        let got = seqs(&text);
        assert_eq!(got, (from..total).collect::<Vec<_>>(), "from {from}");
        let status = text.lines().last().unwrap();
        assert!(
            status.contains(&format!("cursor {total} | lag 0")),
            "{status}"
        );
    }
    let text = tail_text(&TailArgs {
        from_seq: 100,
        to_seq: Some(1_000),
        ..args("solo")
    });
    assert_eq!(seqs(&text), (100..=1_000).collect::<Vec<_>>());
    assert!(
        text.lines()
            .last()
            .unwrap()
            .contains(&format!("cursor 1001 | lag {}", total - 1_001))
    );
    println!("T-10: every one of solo's {total} records prints once, in seq order, from 7 cursors");
}

#[test]
fn canonical_output_digests_to_the_segments_canonical_digest() {
    let dir = run_dir("solo");
    let segments = read_channel_a(&dir.join("stream"), 0, 518_400).unwrap();
    for s in [&segments[0], &segments[57], segments.last().unwrap()] {
        let text = tail_text(&TailArgs {
            from_seq: s.row.first_seq,
            to_seq: Some(s.row.last_seq),
            canonical: true,
            ..args("solo")
        });
        let file = digest_file(text.as_bytes()).unwrap();
        assert_eq!(file.digest, s.row.canonical_digest, "y{}", s.row.sim_year);
        assert_eq!(file.records, s.records);
    }
    let all = tail_text(&TailArgs {
        canonical: true,
        ..args("solo")
    });
    let file = digest_file(all.as_bytes()).unwrap();
    assert_eq!(
        file.records.len(),
        segments.iter().map(|s| s.records.len()).sum::<usize>()
    );
}

/// Every file of a directory: bytes and modification time.
fn tree(dir: &Path) -> BTreeMap<PathBuf, (Vec<u8>, std::time::SystemTime)> {
    fn walk(at: &Path, out: &mut BTreeMap<PathBuf, (Vec<u8>, std::time::SystemTime)>) {
        for e in std::fs::read_dir(at).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                let m = std::fs::metadata(&p).unwrap().modified().unwrap();
                out.insert(p.clone(), (std::fs::read(&p).unwrap(), m));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, &mut out);
    out
}

#[test]
fn tail_never_writes_to_the_run_directory() {
    let dir = run_dir("hamlet");
    // Sifted first, so the tails below read channel C's annotations too (REV-SPR-013 MIN-10).
    let (_, sifted) = rh_golden::sift::sift(&repo(), &dir).unwrap();
    assert!(sifted.matches > 0);
    let before = tree(&dir);
    let modes = [
        args("hamlet"),
        TailArgs {
            canonical: true,
            ..args("hamlet")
        },
        TailArgs {
            follow: Some("Job".to_owned()),
            kinds: vec!["DEATH".to_owned()],
            ..args("hamlet")
        },
        TailArgs {
            wait: true,
            ..args("hamlet")
        },
    ];
    let repo = repo();
    let notes = |d: &Path| rh_golden::sift::annotations(&repo, d);
    for m in &modes {
        let _ = tail_text(m);
        let mut out = Vec::new();
        tail(m, &notes, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        if m.kinds.is_empty() || m.kinds.iter().any(|k| k == "DEATH") {
            assert!(
                text.contains("match solitary_life/life_ends"),
                "no annotation: {m:?}"
            );
        }
    }
    assert_eq!(tree(&dir), before, "tail changed the run directory");
}

/// Compares `text` with the committed snapshot `name` byte for byte, or rewrites it under
/// `RH_UPDATE_SNAPSHOTS=1` (never in CI; a rewrite is reviewed in the diff). A missing snapshot is
/// a failure, never a silent create (the Architect's file-snapshot ruling, SPR-013 T-10).
fn snapshot(name: &str, text: &str) {
    let dir = std::env::var_os("RH_TAIL_SNAPSHOTS").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots"),
        PathBuf::from,
    );
    let path = dir.join(format!("{name}.txt"));
    if std::env::var_os("RH_UPDATE_SNAPSHOTS").is_some() {
        assert!(
            std::env::var_os("CI").is_none(),
            "RH_UPDATE_SNAPSHOTS is refused in CI: snapshots are rewritten locally and reviewed"
        );
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, text).unwrap();
        return;
    }
    let Ok(want) = std::fs::read_to_string(&path) else {
        panic!(
            "SNAPSHOT {name} is missing ({}): rerun with RH_UPDATE_SNAPSHOTS=1 and commit it",
            path.display()
        );
    };
    if want != text {
        let line = want
            .lines()
            .zip(text.lines())
            .position(|(a, b)| a != b)
            .unwrap_or(want.lines().count().min(text.lines().count()));
        panic!(
            "SNAPSHOT {name} differs at line {n}:\n--- committed {}\n+++ tail\n@@ -{n} +{n} @@\n-{}\n+{}\n\
             rerun with RH_UPDATE_SNAPSHOTS=1 and review the diff",
            path.display(),
            want.lines().nth(line).unwrap_or("(end of file)"),
            text.lines().nth(line).unwrap_or("(end of file)"),
            n = line + 1,
        );
    }
}

#[test]
fn the_display_form_matches_its_snapshots() {
    let not_digests = [
        "INPUT",
        "EVENT",
        "LIFE_ENTRY",
        "LIFE_EXIT",
        "KEYFRAME",
        "DEATH",
        "EXTINCTION",
    ]
    .map(str::to_owned)
    .to_vec();
    snapshot(
        "solo_story",
        &tail_text(&TailArgs {
            kinds: not_digests,
            ..args("solo")
        }),
    );
    snapshot(
        "solo_digests_year_57",
        &tail_text(&TailArgs {
            from_seq: 690,
            to_seq: Some(710),
            ..args("solo")
        }),
    );
    snapshot(
        "hamlet_follow_job",
        &tail_text(&TailArgs {
            follow: Some("Job".to_owned()),
            ..args("hamlet")
        }),
    );
    snapshot(
        "hamlet_deaths",
        &tail_text(&TailArgs {
            kinds: vec!["DEATH".to_owned(), "EXTINCTION".to_owned()],
            ..args("hamlet")
        }),
    );
    println!("T-10: the display form matches its 4 snapshots");
}
