//! World Stream v0 from golden runs (SPR-013 T-09; V-STR-01).
//!
//! - On `solo` (1 person) and `hamlet` (60 persons), two runs and runs at 1, 2 and 4 threads seal
//!   byte-identical segment files, with equal canonical digests; `seq` is contiguous from 0
//!   across the segments; every death has its `LIFE_EXIT` linked to its `DEATH` event; the
//!   `STEP_DIGEST` of each Month pulse carries the ladder's state hash of the state after that
//!   instant, checked against an engine driven independently (REV-SPR-013 MIN-10).
//! - A streaming run's ladder equals the same run's ladder without a stream: watching changes
//!   nothing (05.10 §3).
//! - A run resumed from a 10-year keyframe into the same run directory verifies the years it
//!   reproduces against the sealed segments and changes no byte.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken run must fail the test loudly"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rh_golden::ladder::RungKind;
use rh_golden::run::{RunArgs, keyframe_path, run, run_reporting};
use rh_stream::record::Body;
use rh_stream::writer::read_channel_a;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("stream")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn args(world: &str, years: i64, threads: usize, dir: &Path) -> RunArgs {
    let mut a = RunArgs::new(
        &repo(),
        years,
        threads,
        RungKind::Yearly,
        &dir.join("run.ladder"),
    );
    world.clone_into(&mut a.world);
    a.journal = dir.join("journal.sqlite");
    a.run_dir = Some(dir.to_path_buf());
    a
}

/// Every file under `dir/stream`, by relative path.
fn stream_files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(at).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                let rel = p.strip_prefix(root).unwrap().display().to_string();
                out.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(&dir.join("stream"), &dir.join("stream"), &mut out);
    out
}

fn mpy() -> i64 {
    518_400
}

/// V-STR-01 on `solo` and the 60-person `hamlet`: byte-identical segments over repeats and thread
/// counts, contiguous seq, linked deaths.
#[test]
fn segments_are_byte_identical_across_runs_and_thread_counts() {
    for (world, years) in [("solo", 80), ("hamlet", 60)] {
        let base = scratch(&format!("{world}-t1"));
        let first = run_reporting(&args(world, years, 1, &base)).unwrap();
        let reference = stream_files(&base);
        assert!(!reference.is_empty());
        for (label, threads) in [("again", 1), ("t2", 2), ("t4", 4)] {
            let dir = scratch(&format!("{world}-{label}"));
            let other = run_reporting(&args(world, years, threads, &dir)).unwrap();
            assert_eq!(other.ladder, first.ladder, "{world} {label}: ladder");
            assert_eq!(
                stream_files(&dir),
                reference,
                "{world} {label}: stream bytes"
            );
        }
        let segments = read_channel_a(&base.join("stream"), 0, mpy()).unwrap();
        assert_eq!(segments.len(), usize::try_from(years).unwrap());
        let (mut seq, mut deaths, mut events, mut digests) = (0u64, 0, 0, 0);
        for s in &segments {
            assert_eq!(
                rh_stream::canonical::digest_records(&s.records).unwrap(),
                s.row.canonical_digest
            );
            for r in &s.records {
                assert_eq!(r.header.seq, seq);
                seq += 1;
                match &r.body {
                    Body::LifeExit(x) => {
                        deaths += 1;
                        assert!(x.event_id.is_some(), "{world}: an unlinked death");
                    }
                    Body::Event(_) => events += 1,
                    Body::StepDigest(_) => digests += 1,
                    _ => {}
                }
            }
        }
        assert_eq!(digests, years * 12, "{world}: one digest per Month pulse");
        assert!(
            deaths > 0 && events >= deaths,
            "{world}: {deaths} deaths, {events} events"
        );
        println!(
            "T-09 V-STR-01: {world}, {years} years: {} segments, {seq} records, {deaths} deaths; \
             byte-identical twice at 1 thread and at 2 and 4",
            segments.len()
        );
    }
}

/// Watching changes nothing: the ladder of a streaming run is the headless run's.
#[test]
fn a_streaming_run_has_the_headless_runs_ladder() {
    for world in ["solo", "hamlet"] {
        let dir = scratch(&format!("{world}-headless"));
        let mut headless = RunArgs::new(&repo(), 40, 1, RungKind::Monthly, &dir.join("h.ladder"));
        world.clone_into(&mut headless.world);
        let h = run(&headless).unwrap();
        let mut streaming = args(world, 40, 1, &dir.join("run"));
        streaming.rung = RungKind::Monthly;
        let s = run(&streaming).unwrap();
        assert_eq!(h, s, "{world}");
    }
}

/// A resumed run into the same directory verifies every year it reproduces and changes no byte,
/// its `run.ladder` included.
#[test]
fn a_resumed_run_verifies_the_sealed_years_and_changes_no_byte() {
    let dir = scratch("hamlet-resume");
    run(&args("hamlet", 40, 1, &dir)).unwrap();
    let before = stream_files(&dir);
    let ladder = std::fs::read(dir.join("run.ladder")).unwrap();
    for year in [10, 20] {
        let mut resumed = args("hamlet", 40, 2, &dir);
        resumed.resume = Some(keyframe_path(&dir.join("keyframes"), year));
        let o = run_reporting(&resumed).unwrap();
        let s = o.stream.unwrap();
        assert_eq!(s.sealed, 0);
        assert_eq!(s.verified, u64::try_from(40 - year).unwrap());
        assert_eq!(stream_files(&dir), before, "from year {year}");
        // The resumed run's run.ladder is the whole run's (REV-SPR-013 MAJ-3 (b)).
        assert!(
            std::fs::read(dir.join("run.ladder")).unwrap() == ladder,
            "from year {year}"
        );
    }
}

/// REV-SPR-013 MIN-10: the `STEP_DIGEST` of each Month pulse carries the plane-A state hash of the
/// state after that instant, the ladder's hash function: an engine driven independently (the
/// world's setup, no stream, draining its events at the run's stops) gives every digest's
/// `state_hash` at its pulse and every monthly rung's hash at its month end.
#[test]
fn step_digests_carry_the_ladders_state_hash_at_every_month_pulse() {
    use rh_core::engine::Engine;
    use rh_core::register::register;
    use rh_core::time::SimTime;
    use rh_ids::BranchSalt;
    use rh_snapshot::state::state_hash;
    let base = scratch("digest-hash");
    std::fs::create_dir_all(&base).unwrap();
    let none = base.join("no_inputs.toml");
    std::fs::write(&none, "# no input\n").unwrap();
    let dir = base.join("run");
    let mut a = args("hamlet", 12, 1, &dir);
    a.rung = RungKind::Monthly;
    a.inputs = Some(none.clone());
    let ladder = run(&a).unwrap();
    let digests: Vec<(i64, u64)> = read_channel_a(&dir.join("stream"), 0, 518_400)
        .unwrap()
        .into_iter()
        .flat_map(|s| s.records)
        .filter_map(|r| match r.body {
            Body::StepDigest(d) => Some((r.header.tick, d.state_hash)),
            _ => None,
        })
        .collect();
    assert_eq!(digests.len(), 12 * 12);
    let w = rh_golden::world::setup_with(&repo(), "hamlet", None, false, Some(&none)).unwrap();
    let mut engine = Engine::new(
        w.calendar,
        w.seed,
        BranchSalt::from_raw(0),
        register((w.systems)(false), &w.specs).unwrap(),
        w.tables.clone(),
        SimTime::EPOCH,
    );
    engine.set_provenance(w.provenance.clone());
    let mut want: BTreeMap<i64, (Option<u64>, Option<u64>)> = BTreeMap::new();
    for &(t, h) in &digests {
        want.entry(t).or_default().0 = Some(h);
    }
    for r in &ladder.rungs {
        want.entry(r.tick).or_default().1 = Some(r.hash);
    }
    for (t, (digest, rung)) in &want {
        engine
            .advance_until(SimTime::from_minutes(*t), |_, _| {})
            .unwrap();
        drop(engine.drain_events());
        let h = state_hash(&engine.snapshot());
        assert!(digest.is_none_or(|d| d == h), "STEP_DIGEST at tick {t}");
        assert!(rung.is_none_or(|r| r == h), "monthly rung at tick {t}");
    }
    println!(
        "T-09 MIN-10: hamlet, 12 years: all {} STEP_DIGESTs and {} monthly rungs equal the state hash of an independently driven engine",
        digests.len(),
        ladder.rungs.len()
    );
}
