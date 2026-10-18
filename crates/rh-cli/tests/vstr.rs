//! The V-STR probes of the live edge (SPR-026 T-05; `specs/stream/wire_v1.md` §9). Release builds,
//! in `check_all`'s golden part.
//!
//! - **V-STR-01 / V-STR-03.** A world runs headless, then with four viewers attached: `rh-cli tail
//!   --live` at the edge, a scripted 100x playback (100 sim-years a second), a rewinder that
//!   re-subscribes from old seqs (300 seqs a pass, back 100), and a stalled viewer (connected, never reads until the run is
//!   over). Every sealed segment is byte-identical to the headless run's, the ladder is the
//!   baseline, `tail --live` printed exactly the sealed lines, and every viewer got `0..next_seq`.
//!   Wall time with and without viewers is printed (the gate is SPR-030's V-PERF-09). The stalled
//!   viewer's first read after the run, past the frames its socket held, is `stream.behind`.
//! - **V-STR-05, catch-up and jump.** A sleeper stops reading until another connection's
//!   `stream.status` shows the run finished, from sim-year 2 (solo runs 600 paced sim-years here,
//!   about 590 KB unread, far more than a Unix socket holds): it is sent behind (asserted), reads
//!   the sealed years through `rh-cli tail --canonical --from-seq --to-seq`, resubscribes and reaches `stream.end` with `0..next_seq`,
//!   its per-year XXH3-128 of the bodies received live equal to every `stream.sealed` digest and
//!   every `segment.canonical_digest`. A jumper jumps to live from 10 sim-years behind:
//!   `first_seq` is the tip at the answer, nothing duplicated after it.
//! - **V-STR-05, resume.** The host is killed with SIGKILL mid-year while a seal is in flight,
//!   then resumed from the 10-year keyframe with `--socket`. The attached viewer receives
//!   `future_seq` and backs off, verifies the re-emitted years against their `stream.sealed`, and
//!   ends with the headless run's seqs and digests; `Catchup` reports no protocol error.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    reason = "a tool-crate test that runs rh-cli as child processes with a deadline clock; one \
              probe in one place; helpers fail loudly"
)]

mod support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use rh_protocol::control::EndReason;
use serde_json::json;
use support::viewer::{Mode, Report, Viewer, check_seqs, segment_digests};
use support::{BIN, Client, Host, In, a_sock, baseline, hex, ladders_equal, scratch};

const MPY: i64 = 518_400;

fn stream_files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
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
                out.insert(
                    p.strip_prefix(root).unwrap().display().to_string(),
                    std::fs::read(&p).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(
        &dir.join("stream/t0000000000/A"),
        &dir.join("stream/t0000000000/A"),
        &mut out,
    );
    out
}

/// V-STR-01's host, headless and live, sleeps 1 ms at each stop (`--test-pace-ms`; at least 12
/// a sim-year): about 2.5 s for 200 sim-years however fast the disk, so the stalled viewer
/// subscribes while the run is young and the wall times compare like with like.
const PACE: [&str; 2] = ["--test-pace-ms", "1"];

fn headless(world: &str, dir: &Path) -> Duration {
    let t = Instant::now();
    let out = Command::new(BIN)
        .args(["run", world, "--years", "200", "--run-dir"])
        .arg(dir)
        .args(PACE)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    t.elapsed()
}

/// The per-year checks of V-STR-05 on one viewer: what it received live of each whole year
/// hashes to the segment's digest, and every `stream.sealed` it got names the segment's digest.
fn check_digests(r: &Report, segs: &BTreeMap<i64, String>) {
    let mut by_year: BTreeMap<i64, Vec<(u64, &String)>> = BTreeMap::new();
    for (seq, l) in &r.live {
        let tick: i64 = l.split(' ').nth(3).unwrap().parse().unwrap();
        by_year
            .entry(tick.div_euclid(MPY))
            .or_default()
            .push((*seq, l));
    }
    let mut whole = 0;
    for (year, (first, last, digest)) in &r.sealed {
        assert_eq!(
            segs.get(year),
            Some(digest),
            "{}: stream.sealed of {year}",
            r.name
        );
        let last = last.unwrap();
        let lines: Vec<&String> = by_year
            .get(year)
            .map(|v| {
                v.iter()
                    .filter(|(s, _)| (*first..=last).contains(s))
                    .map(|(_, l)| *l)
                    .collect()
            })
            .unwrap_or_default();
        if u64::try_from(lines.len()).unwrap() == last - first + 1 {
            let body: Vec<u8> = lines
                .iter()
                .flat_map(|l| [l.as_bytes(), b"\n"].concat())
                .collect();
            assert_eq!(
                &hex(&rh_snapshot::hash::xxh3_128(&body)),
                digest,
                "{}: the live body of year {year}",
                r.name
            );
            whole += 1;
        }
    }
    assert!(whole > 0, "{}: no year received whole and live", r.name);
}

/// The rewinder: re-subscribes from old seqs on one connection; every line it sees again is the
/// line it saw before (I3). Returns the number of rewinds and the final cursor.
fn rewinder(sock: &Path, run_dir: &Path) -> (u64, u64, BTreeMap<u64, String>) {
    let mut c = Client::connect(sock);
    let mut seen: BTreeMap<u64, String> = BTreeMap::new();
    let (mut from, mut rewinds) = (0u64, 0u64);
    let mut rewound_at: Option<u64> = None;
    loop {
        let (reply, _) = c.call("stream.subscribe", Some(json!({"from": {"seq": from}})));
        if let Some(reason) = rh_protocol::control::reply_reason(&reply) {
            assert_eq!(reason, "behind_live_edge", "{reply}");
            let s = reply["error"]["data"]["sealed_last_seq"].as_u64().unwrap();
            // At most 300 seqs per pass, then a rewind of 100; past the last sealed seq, once per
            // sealed tip (then on to the window). The rewinds do not depend on the host's speed.
            let to = s.min(from + 299);
            let out = Command::new(BIN)
                .args(["tail"])
                .arg(run_dir)
                .args([
                    "--canonical",
                    "--from-seq",
                    &from.to_string(),
                    "--to-seq",
                    &to.to_string(),
                ])
                .output()
                .unwrap();
            for (i, l) in String::from_utf8(out.stdout)
                .unwrap()
                .lines()
                .filter(|l| !l.starts_with('#'))
                .enumerate()
            {
                let seq = from + u64::try_from(i).unwrap();
                assert_eq!(
                    seen.entry(seq).or_insert_with(|| l.to_owned()),
                    l,
                    "seq {seq}"
                );
            }
            if to < s || rewound_at != Some(s) {
                rewinds += 1;
                rewound_at = Some(s);
                from = (to + 1).saturating_sub(100);
            } else {
                from = s + 1;
            }
            continue;
        }
        let mut cursor = from;
        let target = from + 300;
        let mut ended = false;
        while cursor < target {
            match c.next().expect("closed") {
                In::Records(r, _) => {
                    assert_eq!(r.first_seq, cursor);
                    for (i, l) in r.lines.iter().enumerate() {
                        let seq = cursor + u64::try_from(i).unwrap();
                        assert_eq!(seen.entry(seq).or_insert_with(|| l.clone()), l, "seq {seq}");
                    }
                    cursor += u64::try_from(r.lines.len()).unwrap();
                }
                In::Control(_, v) if v["method"] == "stream.end" => {
                    ended = true;
                    break;
                }
                In::Control(_, v) if v["method"] == "stream.behind" => {
                    cursor = v["params"]["next_seq"].as_u64().unwrap();
                    break;
                }
                In::Control(..) => {}
            }
        }
        if ended {
            return (rewinds, cursor, seen);
        }
        // Unsubscribe (frames in flight are fine to drop here: they are re-read) and rewind.
        let (reply, _) = c.call("stream.unsubscribe", None);
        let _ = reply;
        rewinds += 1;
        from = cursor.saturating_sub(100);
    }
}

#[test]
fn v_str_01_and_03_four_viewers_change_no_byte_of_the_stream() {
    for world in ["solo", "hamlet"] {
        let plain = scratch(&format!("vstr01-{world}-headless"));
        let t_headless = headless(world, &plain);
        let dir = scratch(&format!("vstr01-{world}-live"));
        let d = dir.display().to_string();
        let t0 = Instant::now();
        let mut host = Host::start(&[
            "run",
            world,
            "--years",
            "200",
            "--run-dir",
            &d,
            "--socket",
            "--linger",
            "60",
            PACE[0],
            PACE[1],
        ]);
        let sock = a_sock(&dir);
        support::wait_socket(&sock);
        let edge = std::thread::spawn({
            let d = d.clone();
            move || {
                Command::new(BIN)
                    .args(["tail", &d, "--live", "--canonical"])
                    .output()
                    .unwrap()
            }
        });
        let spawn = |name: &str, mode: Mode| {
            let v = Viewer::new(name, &dir, mode);
            std::thread::spawn(move || v.run(Duration::from_mins(3)))
        };
        let playback = spawn("playback", Mode::Playback { years_per_sec: 100 });
        let stalled = spawn("stalled", Mode::Stalled);
        let rew = std::thread::spawn({
            let (sock, dir) = (sock.clone(), dir.clone());
            move || rewinder(&sock, &dir)
        });
        host.wait_line(&format!("{world}: 200 yearly rungs"));
        let t_live = t0.elapsed();
        let edge = edge.join().unwrap();
        let reports = [playback.join().unwrap(), stalled.join().unwrap()];
        let (rewinds, rew_end, rew_seen) = rew.join().unwrap();
        let (code, lines, err) = host.finish();
        assert_eq!(code, 0, "{lines:?} {err}");
        // V-STR-01: the stream and the ladder are the headless run's.
        assert_eq!(
            stream_files(&dir),
            stream_files(&plain),
            "{world}: a sealed byte differs"
        );
        assert!(ladders_equal(&dir.join("run.ladder"), &baseline(world)));
        let (segs, next) = segment_digests(&dir);
        let sealed = Command::new(BIN)
            .args(["tail", &d, "--canonical"])
            .output()
            .unwrap();
        let strip = |b: &[u8]| -> Vec<String> {
            String::from_utf8_lossy(b)
                .lines()
                .filter(|l| !l.starts_with('#'))
                .map(str::to_owned)
                .collect()
        };
        assert!(
            edge.status.success(),
            "{}",
            String::from_utf8_lossy(&edge.stderr)
        );
        assert_eq!(
            strip(&edge.stdout),
            strip(&sealed.stdout),
            "{world}: tail --live = the sealed tail"
        );
        for r in &reports {
            assert!(r.errors.is_empty(), "{}: {:?}", r.name, r.errors);
            assert_eq!(r.end, Some(EndReason::Finished), "{}", r.name);
            check_seqs(r, next).unwrap();
        }
        assert!(
            rewinds > 2 && rew_end == next && u64::try_from(rew_seen.len()).unwrap() == next,
            "{world}: the rewinder rewound {rewinds} time(s), ended at seq {rew_end} and saw {} \
             distinct seqs; the stream has {next}",
            rew_seen.len()
        );
        // The stalled viewer: what its socket held, then stream.behind; then everything from disk.
        let st = &reports[1];
        assert!(
            st.behinds >= 1,
            "the stalled viewer was never sent behind: {:?}",
            st.stalled_first
        );
        assert!(
            st.stalled_first
                .last()
                .is_some_and(|f| f == "stream.behind")
                && st.stalled_first[..st.stalled_first.len() - 1]
                    .iter()
                    .all(|f| f.starts_with("records") || f == "stream.sealed" || f == "reply"),
            "before its stream.behind: {:?}",
            st.stalled_first
        );
        assert!(st.sealed_reads >= 1);
        check_digests(&reports[0], &segs);
        println!(
            "V-STR-03 {world}: wall time headless {} ms, with four viewers {} ms; the stalled \
             viewer held {} frames in its socket before stream.behind; host: {}",
            t_headless.as_millis(),
            t_live.as_millis(),
            st.stalled_first.len().saturating_sub(1),
            lines
                .iter()
                .filter(|l| l.contains("served") || l.contains("seal latency"))
                .cloned()
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
}

#[test]
fn v_str_05_a_sleeper_catches_up_and_a_jumper_jumps_to_live() {
    let dir = scratch("vstr05");
    let d = dir.display().to_string();
    // 600 sim-years, paced (about 20 s): what the sleeper leaves unread is about 590 KB of solo's
    // stream, far past a Unix socket's send buffer, so the host blocks behind its window however
    // fast its disk and however late a loaded runner starts the sleeper.
    let host = Host::start(&[
        "run",
        "solo",
        "--years",
        "600",
        PACE[0],
        PACE[1],
        "--run-dir",
        &d,
        "--socket",
        "--linger",
        "60",
    ]);
    support::wait_socket(&a_sock(&dir));
    let spawn = |name: &str, mode: Mode| {
        let v = Viewer::new(name, &dir, mode);
        std::thread::spawn(move || v.run(Duration::from_mins(3)))
    };
    let sleeper = spawn("sleeper", Mode::Sleeper);
    let jumper = spawn("jumper", Mode::Jumper);
    let (sleeper, jumper) = (sleeper.join().unwrap(), jumper.join().unwrap());
    let (code, lines, err) = host.finish();
    assert_eq!(code, 0, "{lines:?} {err}");
    let (segs, next) = segment_digests(&dir);
    for r in [&sleeper, &jumper] {
        assert!(r.errors.is_empty(), "{}: {:?}", r.name, r.errors);
        assert_eq!(r.end, Some(EndReason::Finished), "{}", r.name);
        check_seqs(r, next).unwrap();
    }
    assert!(
        sleeper.behinds >= 1,
        "the sleeper was never sent behind (stream.behind)"
    );
    assert!(
        sleeper.sealed_reads >= 1,
        "the sleeper never read the sealed years"
    );
    check_digests(&sleeper, &segs);
    let (from, to) = jumper.jump.expect("the jumper jumped");
    assert!(to >= from, "jump {from} -> {to}");
    let tip = jumper.jump_tip.unwrap();
    assert!(
        to <= tip,
        "first_seq {to} is the tip at the answer (status after: {tip})"
    );
    let ten_years = segs.len() >= 10;
    assert!(ten_years);
    println!(
        "V-STR-05: the sleeper was sent behind {} time(s) and read the sealed segments {} time(s); \
         the jumper skipped seqs {from}..{to}",
        sleeper.behinds, sleeper.sealed_reads
    );
}

#[test]
fn v_str_05_a_viewer_survives_a_kill_and_resume_with_a_seal_in_flight() {
    let plain = scratch("vstr05-kill-headless");
    headless("solo", &plain);
    let dir = scratch("vstr05-kill");
    let d = dir.display().to_string();
    // The seal thread blocks inside year 14's seal; the host thread blocks in the middle of
    // year 15: a SIGKILL there is mid-year with a seal in flight.
    let mid = (15 * MPY + 5 * 43_200).to_string();
    let stall = format!("seal:14:2+mid:{mid}");
    let mut host = Host::start(&[
        "run",
        "solo",
        "--years",
        "200",
        "--run-dir",
        &d,
        "--socket",
        "--test-stall",
        &stall,
        // The viewer is attached before anything is emitted, so it receives every year live
        // (DEF-1: a late viewer read the early years from disk and verified none).
        "--test-wait-viewer",
    ]);
    support::wait_socket(&a_sock(&dir));
    let (viewer, progress) = {
        let v = Viewer::new("survivor", &dir, Mode::Edge);
        let progress = v.progress();
        (
            std::thread::spawn(move || v.run(Duration::from_mins(3))),
            progress,
        )
    };
    let a = host.wait_line("STALL");
    let b = host.wait_line("STALL");
    assert!(
        [&a, &b].iter().any(|l| l.contains("seal y000014")),
        "{a} {b}"
    );
    assert!([&a, &b].iter().any(|l| l.contains("mid tick")), "{a} {b}");
    // The viewer reaches the stalled host's edge before the kill: a wait on what it received, not
    // a sleep (DEF-1).
    let mut probe = support::Client::connect(&a_sock(&dir));
    let (status, _) = probe.call("stream.status", None);
    let tip = status["result"]["live"]["next_seq"].as_u64().unwrap();
    drop(probe);
    let deadline = Instant::now() + Duration::from_mins(1);
    while progress.load(std::sync::atomic::Ordering::Relaxed) < tip {
        assert!(
            Instant::now() < deadline,
            "the viewer never reached seq {tip}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    host.kill();
    assert!(
        !dir.join("stream/t0000000000/A/y000014").exists(),
        "year 14's seal was in flight"
    );
    let kf = dir.join("keyframes/year-0010.rhkf").display().to_string();
    // The resumed host waits for the viewer's first request before it emits anything
    // (--test-wait-viewer, a handshake rather than a sleep: DEF-1), and is slow (5 ms a stop), so
    // the viewer is back before the host re-emits its cursor however loaded the machine.
    let resumed = Host::start(&[
        "run",
        "solo",
        "--years",
        "200",
        "--run-dir",
        &d,
        "--resume",
        &kf,
        "--socket",
        "--linger",
        "60",
        "--test-pace-ms",
        "5",
        "--test-wait-viewer",
    ]);
    let r = viewer.join().unwrap();
    let (code, lines, err) = resumed.finish();
    assert_eq!(code, 0, "{lines:?} {err}");
    assert!(r.errors.is_empty(), "Catchup reported {:?}", r.errors);
    assert_eq!(r.end, Some(EndReason::Finished));
    assert!(
        r.reconnects >= 2,
        "the viewer reconnected to the resumed host"
    );
    assert!(r.resumed, "the viewer saw a new run_nonce");
    assert!(r.future_seqs >= 1, "the viewer never saw future_seq");
    assert!(r.retries >= 1, "the viewer never backed off");
    assert!(
        r.verified_resumed >= 1,
        "no re-emitted year verified against its stream.sealed: reconnects {}, future_seq {}, \
         retries {}, received {}, live {}, sealed years {:?}, host {:?}",
        r.reconnects,
        r.future_seqs,
        r.retries,
        r.received.len(),
        r.live.len(),
        r.sealed.keys().collect::<Vec<_>>(),
        lines
    );
    let (segs, next) = segment_digests(&dir);
    assert_eq!(
        segment_digests(&plain),
        (segs.clone(), next),
        "the resumed stream's digests"
    );
    check_seqs(&r, next).unwrap();
    assert_eq!(
        stream_files(&dir),
        stream_files(&plain),
        "the resumed stream is the unbroken one's"
    );
    assert!(ladders_equal(&dir.join("run.ladder"), &baseline("solo")));
    println!(
        "V-STR-05 resume: future_seq {} time(s), {} retries, {} re-emitted year(s) verified",
        r.future_seqs, r.retries, r.verified_resumed
    );
}
