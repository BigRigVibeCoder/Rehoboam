//! The live edge (`specs/stream/wire_v1.md` §6.1, §6.2, §9; SPR-026 T-02).
//!
//! - **Window bounds.** The reference's writer steps (12 instants a sim-year, 1 to 40 records a
//!   year, a seal that lands `delay` instants after its year's end) driven against `LiveEdge`:
//!   option (b) holds at most two sim-years while every seal lands within a sim-year, exactly
//!   `2 + ceil(k)` for a seal `k` sim-years late (the reference's figures: 4 at k = 1.5, 5 at
//!   k = 2.5, 3 at the reference's 20-of-12 scenario), option (a) one.
//! - **The seal race.** Every interleaving of a reader's loads with a seal's two steps (publish
//!   `s`, then drop and move `w0`) is enumerated: one snapshot never shows `s + 1 < w0`, nor does
//!   loading `w0` before `s`; loading `s` before `w0` does in some interleaving (the control).
//! - **The copy race.** A seal and a drop between a snapshot and its copy leave the copy carrying
//!   exactly its seqs; indexing the newer window by the older `w0` mislabels (the control).
//! - **Probe (b).** A reader frozen inside `LiveEdge::read` across a year-end drop, and a second
//!   reader holding a snapshot it never releases: the real writer appends and seals 10 sim-years
//!   on its seal thread before a deadline, and the frozen reader returns exactly `[c, c + n)` or
//!   `Behind`.
//! - **The window-gap control** (`--features mutant-window-gap`): a window that moves before the
//!   dropped year is on disk sends a slow reader behind with `s + 1 < w0`, which it reports as
//!   `I1 broken`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers fail loudly"
)]

mod support;

use std::sync::{Arc, Barrier, mpsc};
use std::time::Duration;

use rh_stream::live::{LiveEdge, Policy, SealEntry, Start};
use rh_stream::record::Body;
use rh_stream::seal::NoHook;
use rh_stream::writer::{LiveConfig, StreamWriter, WriterConfig};
use support::{MPY, dictionaries, history, provenance, scratch};

/// `SplitMix64` (Steele, Lea and Flood 2014).
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

const TICKS: i64 = 12;

fn line(seq: u64, tick: i64) -> String {
    format!("STEP_DIGEST 0 {seq} {tick} 1 0 0 0000000000000000 0000000000000000 0000000000000000")
}

/// The reference's writer steps over `years` (module docs): returns the edge, the peak number of
/// sim-years in the window, and the peak bytes.
fn simulate(seed: u64, years: i64, policy: Policy, delay: i64) -> (LiveEdge, usize, usize) {
    let mut g = SplitMix64(seed);
    let mut plan: Vec<(i64, i64)> = Vec::new(); // (year, step)
    for year in 0..years {
        let n = 1 + g.below(40);
        let mut steps: Vec<i64> = (0..n)
            .map(|_| year * TICKS + i64::try_from(g.below(12)).unwrap())
            .collect();
        steps.sort_unstable();
        plan.extend(steps.into_iter().map(|s| (year, s)));
    }
    let edge = LiveEdge::new(policy, Start::default());
    let mut next = 0u64;
    let mut cursor = 0usize;
    let mut pending: Vec<(i64, i64, u64, Option<u64>)> = Vec::new(); // (landing, year, first, last)
    let mut year_first = vec![0u64];
    let (mut peak, mut peak_bytes) = (0, 0);
    for t in 0..years * TICKS {
        let (mut bytes, mut ends) = (Vec::new(), Vec::new());
        let first = next;
        while cursor < plan.len() && plan[cursor].1 <= t {
            bytes.extend_from_slice(line(next, plan[cursor].1).as_bytes());
            bytes.push(b'\n');
            ends.push(u32::try_from(bytes.len()).unwrap());
            next += 1;
            cursor += 1;
        }
        edge.append(t / TICKS, first, bytes, ends, t);
        land(&edge, &mut pending, t);
        if t % TICKS == TICKS - 1 {
            let y = t / TICKS;
            edge.year_completes(y);
            let f = year_first[usize::try_from(y).unwrap()];
            pending.push((t + delay, y, f, next.checked_sub(1).filter(|&l| l >= f)));
            year_first.push(next);
            if policy == Policy::Sync {
                land(&edge, &mut pending, t + delay);
            }
        }
        let snap = edge.snapshot();
        peak = peak.max(snap.window_years());
        peak_bytes = peak_bytes.max(snap.bytes());
        assert!(
            snap.s.map_or(snap.w0 == 0, |s| s + 1 >= snap.w0),
            "I1 broken at instant {t}"
        );
    }
    (edge, peak, peak_bytes)
}

fn land(edge: &LiveEdge, pending: &mut Vec<(i64, i64, u64, Option<u64>)>, t: i64) {
    let due: Vec<_> = pending.iter().filter(|p| p.0 <= t).copied().collect();
    pending.retain(|p| p.0 > t);
    for (_, y, first, last) in due {
        let entry = SealEntry {
            sim_year: y,
            first_seq: first,
            last_seq: last,
            canonical_digest: [0; 16],
        };
        edge.seal_landed(y, Some(&entry));
    }
}

#[test]
fn the_window_bound_of_each_policy_and_a_late_seals_overrun() {
    for seed in [1, 2, 3, 0xdead_beef] {
        for delay in [0, 3, 11] {
            let (_, peak, _) = simulate(seed, 10, Policy::YearEnd, delay);
            assert_eq!(peak, 2, "(b), seed {seed}, a seal {delay} instants late");
        }
        let (_, peak, _) = simulate(seed, 10, Policy::Sync, 0);
        assert_eq!(peak, 1, "(a), seed {seed}");
        // A seal k sim-years later than a sim-year of writing: 2 + ceil(k) (R16).
        for (delay, bound) in [(20, 3), (30, 4), (42, 5)] {
            let (_, peak, _) = simulate(seed, 10, Policy::YearEnd, delay);
            assert_eq!(
                peak, bound,
                "(b), seed {seed}, a seal {delay} of 12 instants"
            );
        }
    }
    println!("window bounds: (b) 2 sim-years, 3/4/5 at the reference's late seals, (a) 1");
}

/// An edge with year 0 (seqs 0..10) and year 1 (10..20) complete, nothing sealed: year 0's seal
/// is about to land, and the policy then drops year 0.
fn racing_edge() -> LiveEdge {
    let edge = LiveEdge::new(Policy::YearEnd, Start::default());
    for (year, first) in [(0, 0u64), (1, 10)] {
        let mut bytes = Vec::new();
        let mut ends = Vec::new();
        for seq in first..first + 10 {
            bytes.extend_from_slice(line(seq, 0).as_bytes());
            bytes.push(b'\n');
            ends.push(u32::try_from(bytes.len()).unwrap());
        }
        edge.append(year, first, bytes, ends, 0);
        edge.year_completes(year);
    }
    edge
}

fn entry0() -> SealEntry {
    SealEntry {
        sim_year: 0,
        first_seq: 0,
        last_seq: Some(9),
        canonical_digest: [0; 16],
    }
}

/// Whether a reader's two loads (`order`), with the seal's steps interleaved, can see I1 broken.
fn interleaving_breaks(order: &str) -> bool {
    let steps: [&dyn Fn(&LiveEdge); 2] = [
        &|e: &LiveEdge| e.publish_seal(0, Some(&entry0())),
        &|e: &LiveEdge| e.drop_sealed(),
    ];
    for k in 0..=steps.len() {
        for j in k..=steps.len() {
            let edge = racing_edge();
            for s in &steps[..k] {
                s(&edge);
            }
            let first = edge.snapshot();
            for s in &steps[k..j] {
                s(&edge);
            }
            let second = edge.snapshot();
            let (s, w0) = match order {
                "snapshot" => (first.s, first.w0),
                "w0_then_s" => (second.s, first.w0),
                _ => (first.s, second.w0),
            };
            if s.map_or(0, |s| s + 1) < w0 {
                return true;
            }
            // The drop happened: the window starts at year 1.
            if j == steps.len() {
                assert_eq!(second.w0, 10);
            }
        }
    }
    false
}

#[test]
fn the_seal_race_never_shows_i1_broken_through_the_snapshot_api() {
    assert!(
        !interleaving_breaks("snapshot"),
        "a snapshot read showed I1 broken"
    );
    assert!(
        !interleaving_breaks("w0_then_s"),
        "reading w0 before s showed I1 broken"
    );
    assert!(
        interleaving_breaks("s_then_w0"),
        "the control: reading s then w0 never showed I1 broken (it must)"
    );
    println!("seal race: no interleaving shows I1 broken through the snapshot API");
}

#[test]
fn a_drop_between_a_snapshot_and_its_copy_leaves_the_copy_exact() {
    let edge = racing_edge();
    let c = 1u64;
    let lines = edge
        .read_with(c, 1 << 20, || edge.seal_landed(0, Some(&entry0())))
        .unwrap();
    let seqs: Vec<u64> = String::from_utf8(lines.body)
        .unwrap()
        .lines()
        .map(|l| l.split(' ').nth(2).unwrap().parse().unwrap())
        .collect();
    assert_eq!(
        seqs,
        (c..20).collect::<Vec<_>>(),
        "the copy carries its seqs"
    );
    assert_eq!((lines.first_seq, lines.count), (c, 19));
    // After the drop, the next read is behind.
    assert!(edge.read(c, 1 << 20).is_err());
    // The control: the older snapshot's offset into the newer window names another seq.
    let old = racing_edge();
    let before = old.snapshot();
    old.seal_landed(0, Some(&entry0()));
    let after = old.snapshot();
    let wrong = after.window_line(c - before.w0).unwrap();
    assert_ne!(
        String::from_utf8_lossy(wrong).split(' ').nth(2),
        Some("1"),
        "the control: the live window's line at the old offset is not seq 1 (it must not be)"
    );
}

#[test]
fn a_read_never_splits_a_line_and_honours_its_byte_cap() {
    let edge = racing_edge();
    let len = |seq: u64| line(seq, 0).len() + 1;
    let one = len(0);
    for max in [
        0,
        1,
        one - 1,
        one,
        one + 1,
        3 * one,
        12 * one,
        25 * one,
        1 << 20,
    ] {
        let l = edge.read(0, max).unwrap();
        let used: usize = (0..l.count).map(len).sum();
        assert_eq!(l.body.len(), used, "cap {max}: whole lines");
        assert!(l.count == 1 || used <= max, "cap {max}: {used} bytes");
        assert!(
            l.count == 20 || used + len(l.count) > max,
            "cap {max}: the next line fits"
        );
    }
    assert_eq!(edge.read(20, 100).unwrap().count, 0, "the tip: nothing yet");
}

fn config(root: &std::path::Path) -> WriterConfig {
    WriterConfig {
        root: root.to_path_buf(),
        timeline_id: 0,
        minutes_per_year: MPY,
        provenance: provenance(),
        dictionaries: dictionaries(),
    }
}

/// Probe (b) of §9 (module docs).
#[test]
fn probe_b_a_frozen_reader_never_holds_the_writer() {
    let root = scratch("live_probe_b");
    let years = 10;
    let records = history(11, 40, years);
    let mut w = StreamWriter::create(config(&root)).unwrap();
    let edge = w
        .attach_live(LiveConfig {
            policy: Policy::YearEnd,
            clock: None,
            hook: Box::new(NoHook),
        })
        .unwrap();
    // Year 0 in the window first, so the frozen reader starts inside it.
    let zero: Vec<(i64, Body)> = records.iter().filter(|(t, _)| *t == 0).cloned().collect();
    w.push(0, zero, &mut NoHook).unwrap();
    let c = 3;
    let frozen = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let reader = {
        let (edge, frozen, release) = (edge.clone(), frozen.clone(), release.clone());
        std::thread::spawn(move || {
            edge.read_with(c, 1 << 20, || {
                frozen.wait();
                release.wait();
            })
        })
    };
    frozen.wait();
    // A second reader holds a snapshot of year 0 and never lets it go until the end.
    let held = edge.snapshot();
    let (done_tx, done_rx) = mpsc::channel();
    let writer = std::thread::spawn(move || {
        let mut stops: Vec<i64> = (1..years * 12).map(|m| m * 43_200).collect();
        stops.extend((1..=years).map(|y| y * MPY - 1));
        stops.sort_unstable();
        let mut prev = 0;
        for stop in stops {
            let batch: Vec<(i64, Body)> = records
                .iter()
                .filter(|(t, _)| *t > prev && *t <= stop)
                .cloned()
                .collect();
            w.push(stop, batch, &mut NoHook).unwrap();
            prev = stop;
        }
        let summary = w.finish(&mut NoHook).unwrap();
        done_tx.send(summary).unwrap();
    });
    let summary = done_rx
        .recv_timeout(Duration::from_mins(2))
        .expect("the writer did not append and seal 10 sim-years while a reader was frozen");
    writer.join().unwrap();
    assert_eq!(summary.sealed, 10, "10 sim-years sealed by the seal thread");
    let end = edge.snapshot();
    assert_eq!(end.seal_log.len(), 10);
    assert!(
        end.window_years() <= 2,
        "after the run: {} years",
        end.window_years()
    );
    assert!(
        end.w0 > c,
        "the year the frozen reader was in has been dropped"
    );
    release.wait();
    let got = reader.join().unwrap();
    match got {
        Ok(lines) => {
            let seqs: Vec<u64> = String::from_utf8(lines.body)
                .unwrap()
                .lines()
                .map(|l| l.split(' ').nth(2).unwrap().parse().unwrap())
                .collect();
            let want: Vec<u64> = (c..c + lines.count).collect();
            assert_eq!(seqs, want, "the frozen reader returns exactly [c, c + n)");
            assert!(lines.count > 0);
        }
        Err(b) => assert!(b.s.unwrap() + 1 >= b.w0),
    }
    assert!(
        held.read(c, 1 << 20).is_ok(),
        "a held snapshot still reads its own window"
    );
    // A fresh read from c is now behind, with I1.
    let behind = edge.read(c, 1 << 20).unwrap_err();
    assert!(behind.s.unwrap() + 1 >= behind.w0, "I1");
    println!(
        "probe (b): 10 sim-years appended and sealed on the seal thread while a reader was frozen \
         inside LiveEdge::read across a year-end drop; it returned exactly its seqs"
    );
}

/// The window-gap control's detection, and the real policy passing it: a reader that polls once
/// every `period` instants, sent behind, must find `s + 1 >= w0`.
#[test]
fn a_slow_reader_sent_behind_always_finds_the_disk_reaching_the_window() {
    for seed in [1, 2] {
        let (edge, _, _) = simulate(seed, 6, Policy::YearEnd, 3);
        let snap = edge.snapshot();
        assert!(snap.w0 > 0, "the window moved");
        match edge.read(0, 1 << 20) {
            Err(b) => assert!(
                b.s.is_some_and(|s| s + 1 >= b.w0),
                "I1 broken: sealed_last_seq {:?} + 1 < live_first {} (seed {seed})",
                b.s,
                b.w0
            ),
            Ok(_) => panic!("seq 0 is still in the window"),
        }
    }
}

fn files_of(root: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    use rh_stream::seal::{channel_dir, read_segment, sealed_segments};
    let mut out = std::collections::BTreeMap::new();
    for (year, dir) in
        sealed_segments(&channel_dir(root, 0, rh_stream::record::Channel::A)).unwrap()
    {
        for (name, bytes) in read_segment(&dir).unwrap() {
            out.insert(format!("y{year}/{name}"), bytes);
        }
    }
    out
}

/// Feeds the synthetic history of `years` at month stops, through `until` (a stop), live or not.
fn feed(w: &mut StreamWriter, records: &[(i64, Body)], from: i64, until: i64) {
    let mut stops: Vec<i64> = (0..=until / 43_200).map(|m| m * 43_200).collect();
    stops.extend((1..=until / MPY + 1).map(|y| y * MPY - 1));
    stops.sort_unstable();
    stops.dedup();
    let mut prev = from;
    for stop in stops.into_iter().filter(|&s| s > from && s <= until) {
        let batch: Vec<(i64, Body)> = records
            .iter()
            .filter(|(t, _)| *t > prev && *t <= stop)
            .cloned()
            .collect();
        w.push(stop, batch, &mut NoHook).unwrap();
        prev = stop;
    }
}

fn live(w: &mut StreamWriter, policy: Policy) -> LiveEdge {
    w.attach_live(LiveConfig {
        policy,
        clock: None,
        hook: Box::new(NoHook),
    })
    .unwrap()
}

/// The live edge changes no sealed byte: headless, (a) and (b) seal identical files, and every
/// seal-log entry is the sealed row's (V-STR-01 within the crate).
#[test]
fn live_and_headless_runs_seal_the_same_bytes() {
    for (seed, persons, years) in [(1, 10, 3), (2, 60, 5)] {
        let records = history(seed, persons, years);
        let end = years * MPY - 1;
        let mut trees = Vec::new();
        for mode in [None, Some(Policy::Sync), Some(Policy::YearEnd)] {
            let root = scratch(&format!("live_equal_{seed}_{mode:?}"));
            let mut w = StreamWriter::create(config(&root)).unwrap();
            let edge = mode.map(|p| live(&mut w, p));
            let zero: Vec<(i64, Body)> = records.iter().filter(|(t, _)| *t == 0).cloned().collect();
            w.push(0, zero, &mut NoHook).unwrap();
            feed(&mut w, &records, 0, end);
            let summary = w.finish(&mut NoHook).unwrap();
            assert_eq!(summary.sealed, u64::try_from(years).unwrap(), "{mode:?}");
            if let Some(edge) = edge {
                let snap = edge.snapshot();
                assert_eq!(snap.end, Some(rh_stream::live::End::Finished));
                assert_eq!(snap.seal_log.len(), usize::try_from(years).unwrap());
                let segs = rh_stream::writer::read_channel_a(&root, 0, MPY).unwrap();
                for (e, seg) in snap.seal_log.iter().zip(&segs) {
                    assert_eq!(
                        (e.first_seq, e.last_seq, e.canonical_digest),
                        (
                            seg.row.first_seq,
                            Some(seg.row.last_seq),
                            seg.row.canonical_digest
                        )
                    );
                }
                assert_eq!(snap.next_seq, summary.next_seq);
            }
            trees.push(files_of(&root));
        }
        assert_eq!(trees[0], trees[1], "headless = (a), seed {seed}");
        assert_eq!(trees[0], trees[2], "headless = (b), seed {seed}");
    }
}

/// R18: a resumed host's edge is empty at the first seq of the first sim-year not on disk, with
/// `s` and `last_year` from disk, past the years the resume re-verifies.
#[test]
fn a_resumed_edge_starts_empty_at_the_first_year_not_on_disk() {
    let root = scratch("live_resume_start");
    let (years, records) = (6, history(5, 30, 6));
    let mut w = StreamWriter::create(config(&root)).unwrap();
    let zero: Vec<(i64, Body)> = records.iter().filter(|(t, _)| *t == 0).cloned().collect();
    w.push(0, zero, &mut NoHook).unwrap();
    // A kill mid-year 4: years 0-3 sealed.
    feed(&mut w, &records, 0, 4 * MPY + 3 * 43_200);
    drop(w);
    let segs = rh_stream::writer::read_channel_a(&root, 0, MPY).unwrap();
    assert_eq!(segs.len(), 4);
    // Resume from the keyframe at the end of year 1: years 2 and 3 are re-verified.
    let mut w = StreamWriter::resume(config(&root), 2 * MPY - 1).unwrap();
    let edge = live(&mut w, Policy::YearEnd);
    let snap = edge.snapshot();
    let last = segs[3].row.last_seq;
    assert_eq!(
        (snap.w0, snap.next_seq),
        (last + 1, last + 1),
        "an empty window"
    );
    assert_eq!((snap.s, snap.last_year), (Some(last), Some(3)));
    feed(&mut w, &records, 2 * MPY - 1, years * MPY - 1);
    let summary = w.finish(&mut NoHook).unwrap();
    assert_eq!((summary.verified, summary.sealed), (2, 2));
    let snap = edge.snapshot();
    let years_logged: Vec<i64> = snap.seal_log.iter().map(|e| e.sim_year).collect();
    assert_eq!(
        years_logged,
        vec![4, 5],
        "only the re-emitted years are announced"
    );
    assert!(snap.w0 > last, "nothing below the resume point is served");
}

/// A-42: a host dropped mid-year (an error) lets the completed years' seals land, never seals the
/// open year, and ends the edge aborted.
#[test]
fn an_aborted_writer_never_seals_its_open_year() {
    let root = scratch("live_abort");
    let records = history(3, 20, 5);
    let mut w = StreamWriter::create(config(&root)).unwrap();
    let edge = live(&mut w, Policy::YearEnd);
    let zero: Vec<(i64, Body)> = records.iter().filter(|(t, _)| *t == 0).cloned().collect();
    w.push(0, zero, &mut NoHook).unwrap();
    feed(&mut w, &records, 0, 3 * MPY + 6 * 43_200);
    let open = w.open_year();
    drop(w);
    let snap = edge.snapshot();
    assert_eq!(snap.end, Some(rh_stream::live::End::Aborted));
    let sealed: Vec<i64> = snap.seal_log.iter().map(|e| e.sim_year).collect();
    assert_eq!(sealed, vec![0, 1, 2]);
    assert_eq!(open, 3);
    assert!(
        !files_of(&root).keys().any(|k| k.starts_with("y3/")),
        "year 3 is not sealed"
    );
}

/// `after_seal` runs behind its year's seal, never before (the keyframe file follows its
/// segment), and the clock times every seal.
#[test]
fn work_queued_after_a_seal_runs_once_the_segment_is_on_disk() {
    #[derive(Default)]
    struct Clock(
        std::sync::Mutex<Vec<(i64, u64, u64)>>,
        std::sync::atomic::AtomicU64,
    );
    impl rh_stream::writer::SealClock for Clock {
        fn now(&self) -> u64 {
            self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        }
        fn landed(&self, year: i64, closed: u64, landed: u64) {
            self.0.lock().unwrap().push((year, closed, landed));
        }
    }
    let root = scratch("live_after_seal");
    let records = history(9, 15, 3);
    let clock = Arc::new(Clock::default());
    let mut w = StreamWriter::create(config(&root)).unwrap();
    w.attach_live(LiveConfig {
        policy: Policy::YearEnd,
        clock: Some(clock.clone()),
        hook: Box::new(NoHook),
    })
    .unwrap();
    let zero: Vec<(i64, Body)> = records.iter().filter(|(t, _)| *t == 0).cloned().collect();
    w.push(0, zero, &mut NoHook).unwrap();
    feed(&mut w, &records, 0, 2 * MPY - 1);
    let (tx, rx) = mpsc::channel();
    let dir = root.clone();
    w.after_seal(Box::new(move || {
        tx.send(files_of(&dir).keys().any(|k| k.starts_with("y1/")))
            .map_err(|e| e.to_string())
    }))
    .unwrap();
    feed(&mut w, &records, 2 * MPY - 1, 3 * MPY - 1);
    w.finish(&mut NoHook).unwrap();
    assert!(
        rx.recv().unwrap(),
        "year 1 was on disk when the queued work ran"
    );
    let log = clock.0.lock().unwrap().clone();
    assert_eq!(log.iter().map(|x| x.0).collect::<Vec<_>>(), vec![0, 1, 2]);
    assert!(
        log.iter().all(|(_, c, l)| l > c),
        "each seal timed from hand-over to landing"
    );
}
