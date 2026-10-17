//! The writer, sealing and the segment format on synthetic streams (SPR-013 T-09).
//!
//! - V-STR-01 within one build: the same records seal to byte-identical files, and decode back to
//!   the same records with equal canonical digests, over seeds and world sizes.
//! - `seq` is contiguous from 0 across segments; R4's order is enforced.
//! - Resume after a sealed year: the reproduced years verify against the sealed segments, and a
//!   resumed run that differs is refused; a `.partial` left by a crash is removed.
//! - The decoder refuses a segment with a file missing, a file extra, a stored digest altered or a
//!   file from another year.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers fail loudly"
)]

mod support;

use std::collections::BTreeMap;
use std::path::Path;

use rh_stream::canonical;
use rh_stream::record::Body;
use rh_stream::record::Channel;
use rh_stream::seal::{NoHook, channel_dir, read_segment, sealed_segments};
use rh_stream::segment::{self, CHANNEL_A};
use rh_stream::writer::{StreamWriter, WriterConfig, read_channel_a};
use support::{MPY, dictionaries, history, provenance, scratch};

fn config(root: &Path) -> WriterConfig {
    WriterConfig {
        root: root.to_path_buf(),
        timeline_id: 0,
        minutes_per_year: MPY,
        provenance: provenance(),
        dictionaries: dictionaries(),
    }
}

/// Feeds `records` to `w` at month stops, as the host's run loop does.
fn feed(w: &mut StreamWriter, records: &[(i64, Body)], from: i64, years: i64) {
    let mut stops: Vec<i64> = (0..years * 12).map(|m| m * 43_200).collect();
    stops.extend((1..=years).map(|y| y * MPY - 1));
    stops.sort_unstable();
    let mut prev = from;
    for stop in stops.into_iter().filter(|&s| s > from) {
        let batch: Vec<(i64, Body)> = records
            .iter()
            .filter(|(t, _)| *t > prev && *t <= stop)
            .cloned()
            .collect();
        w.push(stop, batch, &mut NoHook).unwrap();
        prev = stop;
    }
}

fn run(root: &Path, seed: u64, persons: u64, years: i64) {
    let records = history(seed, persons, years);
    let mut w = StreamWriter::create(config(root)).unwrap();
    // Tick 0 first (genesis and the instant at 0), then every later stop.
    let zero: Vec<(i64, Body)> = records.iter().filter(|(t, _)| *t == 0).cloned().collect();
    w.push(0, zero, &mut NoHook).unwrap();
    feed(&mut w, &records, 0, years);
    let s = w.finish(&mut NoHook).unwrap();
    assert_eq!(s.sealed, u64::try_from(years).unwrap());
}

fn files_of(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for (year, dir) in sealed_segments(&channel_dir(root, 0, Channel::A)).unwrap() {
        for (name, bytes) in read_segment(&dir).unwrap() {
            out.insert(format!("y{year}/{name}"), bytes);
        }
    }
    out
}

/// V-STR-01 within one build, over 4 seeds x 3 world sizes: two runs seal byte-identical files,
/// the segments decode, seq is contiguous from 0, and every table of the format is present.
#[test]
fn the_same_records_seal_byte_identical_segments_with_contiguous_seq() {
    for seed in [1u64, 2, 77, 1_000_003] {
        for persons in [1u64, 5, 60] {
            let (a, b) = (
                scratch(&format!("same-{seed}-{persons}-a")),
                scratch(&format!("same-{seed}-{persons}-b")),
            );
            run(&a, seed, persons, 3);
            run(&b, seed, persons, 3);
            let (fa, fb) = (files_of(&a), files_of(&b));
            assert_eq!(fa.len(), 3 * CHANNEL_A.len());
            assert_eq!(fa, fb, "seed {seed}, {persons} persons");
            let segments = read_channel_a(&a, 0, MPY).unwrap();
            let mut seq = 0;
            let mut deaths = 0;
            for s in &segments {
                assert_eq!(s.row.first_seq, seq);
                for r in &s.records {
                    assert_eq!(r.header.seq, seq);
                    seq += 1;
                    if let Body::LifeExit(x) = &r.body {
                        deaths += 1;
                        assert!(x.event_id.is_some(), "a death is linked to its DEATH event");
                    }
                }
                assert_eq!(
                    canonical::digest_records(&s.records).unwrap(),
                    s.row.canonical_digest
                );
            }
            assert_eq!(deaths, persons);
        }
    }
    println!(
        "T-09 writer: 4 seeds x 3 world sizes seal byte-identical segments, seq contiguous from 0"
    );
}

/// The digests count the life records since the previous digest; the writer fills them.
#[test]
fn step_digests_count_the_deaths_since_the_previous_digest() {
    let root = scratch("counts");
    run(&root, 9, 60, 2);
    let mut since = 0;
    let mut total = 0;
    for s in read_channel_a(&root, 0, MPY).unwrap() {
        for r in s.records {
            match r.body {
                Body::LifeExit(_) => since += 1,
                Body::StepDigest(d) => {
                    assert_eq!(d.deaths, since);
                    total += d.deaths;
                    since = 0;
                }
                _ => {}
            }
        }
    }
    assert_eq!(total + since, 60);
}

#[test]
fn a_record_out_of_r4_order_is_refused() {
    let root = scratch("order");
    let records = history(3, 5, 1);
    let mut w = StreamWriter::create(config(&root)).unwrap();
    w.push(
        43_200,
        records
            .iter()
            .filter(|(t, _)| *t <= 43_200)
            .cloned()
            .collect(),
        &mut NoHook,
    )
    .unwrap();
    let late: Vec<(i64, Body)> = records
        .iter()
        .filter(|(t, _)| *t == 0)
        .take(1)
        .cloned()
        .collect();
    let e = w.push(86_400, late, &mut NoHook).unwrap_err();
    assert!(
        e.to_string().contains("does not follow the last record"),
        "{e}"
    );
    let early: Vec<(i64, Body)> = records
        .iter()
        .filter(|(t, _)| *t > 100_000)
        .take(1)
        .cloned()
        .collect();
    let e = w.push(90_000, early, &mut NoHook).unwrap_err();
    assert!(e.to_string().contains("pushed with the stop"), "{e}");
}

#[test]
fn a_new_run_refuses_a_directory_that_already_holds_a_stream() {
    let root = scratch("exists");
    run(&root, 4, 5, 1);
    let e = StreamWriter::create(config(&root)).err().unwrap();
    assert!(
        e.to_string().contains("already holds sealed segment"),
        "{e}"
    );
}

/// Resume after year 1's last minute: years 2 and 3 are sealed already and the reproduced
/// records verify against them; a crash's `.partial` is removed; a different history is refused.
#[test]
fn a_resumed_run_verifies_the_sealed_years_and_refuses_a_different_history() {
    for persons in [5u64, 60] {
        let root = scratch(&format!("resume-{persons}"));
        run(&root, 11, persons, 3);
        let before = files_of(&root);
        let dir = channel_dir(&root, 0, Channel::A);
        std::fs::create_dir_all(dir.join(".y000002.partial")).unwrap();
        std::fs::write(dir.join(".y000002.partial/event.parquet"), b"torn").unwrap();
        let records = history(11, persons, 3);
        let at = MPY - 1;
        let mut w = StreamWriter::resume(config(&root), at).unwrap();
        assert!(!dir.join(".y000002.partial").exists());
        feed(&mut w, &records, at, 3);
        let s = w.finish(&mut NoHook).unwrap();
        assert_eq!((s.sealed, s.verified), (0, 2));
        assert_eq!(files_of(&root), before);

        // A different later history (another seed) is refused at the first sealed year.
        let other = history(12, persons, 3);
        let mut w = StreamWriter::resume(config(&root), at).unwrap();
        let mut refused = false;
        for stop in (13..=36).map(|m| m * 43_200 - 1) {
            let batch: Vec<(i64, Body)> = other
                .iter()
                .filter(|(t, _)| *t > stop - 43_200 && *t <= stop && *t > at)
                .cloned()
                .collect();
            if w.push(stop, batch, &mut NoHook).is_err() {
                refused = true;
                break;
            }
        }
        assert!(refused, "a different history must not verify");
    }
}

/// A resume needs the sealed segment of its own year, at a year's last minute.
#[test]
fn a_resume_point_must_be_a_sealed_years_last_minute() {
    let root = scratch("resume-points");
    run(&root, 5, 5, 2);
    let e = StreamWriter::resume(config(&root), MPY).err().unwrap();
    assert!(e.to_string().contains("last minute of a sim-year"), "{e}");
    let e = StreamWriter::resume(config(&root), 3 * MPY - 1)
        .err()
        .unwrap();
    assert!(e.to_string().contains("is not sealed"), "{e}");
}

/// The decoder refuses a segment that is not exactly the format's.
#[test]
fn a_tampered_segment_is_refused() {
    let root = scratch("tamper");
    run(&root, 6, 5, 2);
    let dirs = sealed_segments(&channel_dir(&root, 0, Channel::A)).unwrap();
    let y0 = read_segment(&dirs[&0]).unwrap();
    let y1 = read_segment(&dirs[&1]).unwrap();
    assert!(segment::decode(&y0, MPY).is_ok());
    let mut missing = y0.clone();
    missing.remove("event.parquet");
    let mut extra = y0.clone();
    extra.insert("notes.parquet".to_owned(), Vec::new());
    let mut swapped = y0.clone();
    swapped.insert(
        "step_digest.parquet".to_owned(),
        y1["step_digest.parquet"].clone(),
    );
    let mut garbage = y0.clone();
    garbage.insert("input.parquet".to_owned(), b"PAR1 not parquet".to_vec());
    let mut schema = y0.clone();
    schema.insert(
        "input.parquet".to_owned(),
        y0["keyframe_marker.parquet"].clone(),
    );
    for (what, files) in [
        ("missing", missing),
        ("extra", extra),
        ("swapped", swapped),
        ("garbage", garbage),
        ("schema", schema),
    ] {
        assert!(segment::decode(&files, MPY).is_err(), "{what} accepted");
    }
}
