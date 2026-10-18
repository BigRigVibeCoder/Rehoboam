//! The viewer harness of the V-STR probes (SPR-026 T-05): `rh_protocol::cursor::Catchup` driven
//! over a raw connection, with the behaviours of T-05's table, recording everything a probe
//! checks. Sealed reads go through `rh-cli tail --canonical --from-seq --to-seq` (a subprocess, as
//! a standard-library viewer reads them, R23).
#![allow(
    dead_code,
    clippy::collapsible_if,
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    reason = "test helpers: each test binary uses some; a deadline clock; one loop in one place"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rh_protocol::control::{self, EndReason, Kind, Notice, SubscribeFrom};
use rh_protocol::cursor::{Action, Catchup, Event};
use serde_json::{Value, json};

use super::{BIN, Client, In, wait_socket};

/// What a viewer does besides following the loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Follows the live edge.
    Edge,
    /// Delivers at a scripted pace: `years_per_sec` sim-years of ticks per second of wall time.
    Playback { years_per_sec: u64 },
    /// Subscribed, never reads until the run is over (another connection says `finished`).
    Stalled,
    /// Reads live to sim-year 2, then stops reading until the run is over (another connection
    /// says `finished`); then catches up: what its socket held, `stream.behind` once the host's
    /// window has passed its cursor, the sealed years, the live window, `stream.end`.
    Sleeper,
    /// Reads to sim-year 5, stops until the tip is 10 sim-years ahead, then jumps to live.
    Jumper,
}

/// Everything a viewer saw.
#[derive(Debug, Default)]
pub struct Report {
    pub name: String,
    /// Every line delivered, in order, with its seq (live and sealed).
    pub received: Vec<(u64, String)>,
    /// The lines received live, by seq.
    pub live: BTreeMap<u64, String>,
    /// `stream.sealed` notifications: year -> (first, last, digest).
    pub sealed: BTreeMap<i64, (u64, Option<u64>, String)>,
    pub behinds: u64,
    pub behind_errors: u64,
    pub future_seqs: u64,
    pub retries: u64,
    pub reconnects: u64,
    pub resumed: bool,
    pub verified_resumed: u64,
    pub sealed_reads: u64,
    /// `(cursor before, first_seq after)` of a jump.
    pub jump: Option<(u64, u64)>,
    /// The tip `stream.status` showed just after the jump's subscribe was answered.
    pub jump_tip: Option<u64>,
    pub end: Option<EndReason>,
    pub errors: Vec<String>,
    /// For the stalled viewer: the frames it read after the run, before `stream.behind`.
    pub stalled_first: Vec<String>,
}

/// One viewer of a run directory's socket.
pub struct Viewer {
    pub mode: Mode,
    run_dir: PathBuf,
    sock: PathBuf,
    channel: &'static str,
    mpy: i64,
    lp: Catchup,
    conn: Option<Client>,
    pending: Option<(i64, &'static str)>,
    history: Option<Value>,
    nonce: Option<u64>,
    pub report: Report,
    started: Instant,
    slept: bool,
    jumped: bool,
    /// One past the highest seq delivered so far, for a test to wait on ([`Viewer::progress`]).
    progress: Arc<AtomicU64>,
}

/// One poll of a backoff (`Action::Wait`).
const BACKOFF_POLL: Duration = Duration::from_millis(2);

fn year_of(line: &str, mpy: i64) -> i64 {
    line.split(' ')
        .nth(3)
        .unwrap()
        .parse::<i64>()
        .unwrap()
        .div_euclid(mpy)
}

impl Viewer {
    pub fn new(name: &str, run_dir: &Path, mode: Mode) -> Self {
        Self {
            mode,
            run_dir: run_dir.to_path_buf(),
            sock: run_dir.join("stream/t0000000000/A.sock"),
            channel: "A",
            mpy: 518_400,
            lp: Catchup::new(0),
            conn: None,
            pending: None,
            history: None,
            nonce: None,
            report: Report {
                name: name.to_owned(),
                ..Report::default()
            },
            started: Instant::now(),
            slept: false,
            jumped: false,
            progress: Arc::new(AtomicU64::new(0)),
        }
    }

    /// One past the highest seq this viewer has delivered, readable while it runs on its thread.
    pub fn progress(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.progress)
    }

    /// Another connection's `stream.status` result.
    fn status(&self) -> Option<Value> {
        let mut c = Client::try_connect(&self.sock)?;
        let (v, _) = c.call("stream.status", None);
        v.get("result").cloned()
    }

    fn deliver(&mut self, first_seq: u64, lines: &[String], live: bool) {
        for (i, l) in lines.iter().enumerate() {
            let seq = first_seq + u64::try_from(i).unwrap();
            if live {
                self.report.live.insert(seq, l.clone());
            }
            self.report.received.push((seq, l.clone()));
            self.progress.fetch_max(seq + 1, Ordering::Relaxed);
        }
    }

    fn request(&mut self, method: &'static str, params: Option<Value>) -> bool {
        let Some(c) = self.conn.as_mut() else {
            return false;
        };
        let id = c.request(method, params);
        self.pending = Some((id, method));
        true
    }

    /// `stream.status` on a new connection (`Action::Status`).
    fn reconnect(&mut self) -> Event {
        self.conn = None;
        self.report.reconnects += 1;
        let deadline = Instant::now() + Duration::from_mins(2);
        let mut c = loop {
            if let Some(c) = Client::try_connect(&self.sock) {
                break c;
            }
            assert!(
                Instant::now() < deadline,
                "{}: no host to reconnect to",
                self.report.name
            );
            std::thread::sleep(Duration::from_millis(2));
        };
        let id = c.request("stream.status", None);
        loop {
            match c.next() {
                None => return Event::Closed,
                Some(In::Control(_, v)) if v["id"].is_null() => {
                    return Event::Refused {
                        reason: control::reply_reason(&v).unwrap_or("?").to_owned(),
                        finished: false,
                    };
                }
                Some(In::Control(_, v)) if v["id"] == json!(id) => {
                    let st = control::check_status(&v["result"]).unwrap();
                    let same = self.history.as_ref().is_none_or(|h| h == st.history());
                    if same && self.nonce.is_some_and(|n| n != st.run_nonce) {
                        self.report.resumed = true;
                    }
                    self.history = Some(st.history().clone());
                    self.nonce = Some(st.run_nonce);
                    self.conn = Some(c);
                    return Event::Status { same_run: same };
                }
                Some(_) => {}
            }
        }
    }

    fn sealed_read(&mut self, from: u64, to: u64) -> Vec<String> {
        self.report.sealed_reads += 1;
        let out = Command::new(BIN)
            .args(["tail"])
            .arg(&self.run_dir)
            .args(["--canonical", "--channel", self.channel])
            .args(["--from-seq", &from.to_string(), "--to-seq", &to.to_string()])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(str::to_owned)
            .collect()
    }

    /// R21: a year a resumed host re-emitted must be what this viewer received of it.
    fn verify(&mut self, year: i64, first: u64, last: Option<u64>, digest: &str) {
        self.report
            .sealed
            .insert(year, (first, last, digest.to_owned()));
        let Some(last) = last.filter(|_| self.report.resumed) else {
            return;
        };
        let lines: Option<Vec<&String>> =
            (first..=last).map(|q| self.report.live.get(&q)).collect();
        if let Some(lines) = lines {
            let body: Vec<u8> = lines
                .iter()
                .flat_map(|l| [l.as_bytes(), b"\n"].concat())
                .collect();
            if super::hex(&rh_snapshot::hash::xxh3_128(&body)) != digest {
                self.report.errors.push(format!(
                    "the resumed year {year} differs from what was received"
                ));
            }
            self.report.verified_resumed += 1;
        }
    }

    /// The behaviour before reading the next frame (module docs).
    fn before_read(&mut self) {
        let year = self
            .report
            .received
            .last()
            .map_or(0, |(_, l)| year_of(l, self.mpy));
        match self.mode {
            Mode::Stalled if !self.slept => {
                while self
                    .status()
                    .is_some_and(|s| s["pace"]["state"] != "finished")
                {
                    std::thread::sleep(Duration::from_millis(20));
                }
                self.slept = true;
            }
            Mode::Sleeper
                if !self.slept
                    && year >= 2
                    && self.lp.state == rh_protocol::cursor::State::Live =>
            {
                while self
                    .status()
                    .is_some_and(|s| s["pace"]["state"] != "finished")
                {
                    std::thread::sleep(Duration::from_millis(20));
                }
                self.slept = true;
            }
            Mode::Jumper
                if !self.jumped
                    && year >= 5
                    && self.lp.state == rh_protocol::cursor::State::Live =>
            {
                let cursor_year = year;
                while self.status().is_some_and(|s| {
                    s["tip"]["tick"].as_i64().unwrap_or(0).div_euclid(self.mpy) < cursor_year + 10
                        && s["pace"]["state"] != "finished"
                }) {
                    std::thread::sleep(Duration::from_millis(2));
                }
                self.jumped = true;
                let before = self.lp.cursor;
                let action = self.lp.next(Event::Jump);
                assert_eq!(action, Action::Unsubscribe);
                self.request("stream.unsubscribe", None);
                self.report.jump = Some((before, 0));
            }
            Mode::Playback { years_per_sec } => {
                if let Some((_, l)) = self.report.received.last() {
                    let tick: i64 = l.split(' ').nth(3).unwrap().parse().unwrap();
                    let per_sec = self.mpy * i64::try_from(years_per_sec).unwrap();
                    let due_ms = u64::try_from(tick.max(0) * 1000 / per_sec).unwrap();
                    if let Some(rest) =
                        Duration::from_millis(due_ms).checked_sub(self.started.elapsed())
                    {
                        std::thread::sleep(rest.min(Duration::from_millis(200)));
                    }
                }
            }
            _ => {}
        }
    }

    fn next_event(&mut self) -> Action {
        self.before_read();
        let Some(conn) = self.conn.as_mut() else {
            return self.lp.next(Event::Closed);
        };
        let Some(frame) = conn.next() else {
            return self.lp.next(Event::Closed);
        };
        if self.mode == Mode::Stalled && self.report.behinds == 0 {
            self.report.stalled_first.push(match &frame {
                In::Records(r, _) => format!("records {} {}", r.first_seq, r.lines.len()),
                In::Control(_, v) => v["method"].as_str().unwrap_or("reply").to_owned(),
            });
        }
        match frame {
            In::Records(r, _) => {
                let count = u64::try_from(r.lines.len()).unwrap();
                let action = self.lp.next(Event::Records {
                    first_seq: r.first_seq,
                    count,
                });
                if matches!(action, Action::Deliver { count, .. } if count > 0) {
                    self.deliver(r.first_seq, &r.lines, true);
                }
                action
            }
            In::Control(Kind::Notification, v) => match control::read_notice(&v).unwrap() {
                Notice::Sealed(s) => {
                    self.verify(s.sim_year, s.first_seq, s.last_seq, &s.canonical_digest);
                    self.lp.next(Event::Sealed)
                }
                Notice::Behind {
                    next_seq,
                    sealed_last_seq,
                    live_first_seq,
                    ..
                } => {
                    self.report.behinds += 1;
                    self.lp.next(Event::Behind {
                        next_seq,
                        s: sealed_last_seq,
                        w0: live_first_seq,
                    })
                }
                Notice::End { reason, .. } => self.lp.next(Event::End(reason)),
                Notice::Unknown(_) => Action::Ignore(String::new()),
            },
            In::Control(_, v) => {
                let method = self.pending.take().map(|(_, m)| m).unwrap_or_default();
                let reason = control::reply_reason(&v).map(str::to_owned);
                let res = |k: &str| v["result"][k].as_u64().unwrap_or_default();
                match (method, reason.as_deref()) {
                    ("stream.subscribe", None) => {
                        let first = res("first_seq");
                        if let Some((before, 0)) = self.report.jump {
                            if self.lp.state == rh_protocol::cursor::State::Jumping {
                                self.report.jump = Some((before, first));
                                self.report.jump_tip =
                                    self.status().and_then(|s| s["live"]["next_seq"].as_u64());
                            }
                        }
                        self.lp.next(Event::Subscribed(first))
                    }
                    ("stream.unsubscribe", None) => {
                        self.lp.next(Event::Unsubscribed(res("next_seq")))
                    }
                    ("stream.subscribe", Some("behind_live_edge")) => {
                        self.report.behind_errors += 1;
                        let d = &v["error"]["data"];
                        self.lp.next(Event::Behind {
                            next_seq: self.lp.cursor,
                            s: d["sealed_last_seq"].as_u64(),
                            w0: d["live_first_seq"].as_u64().unwrap(),
                        })
                    }
                    (_, Some(r)) => {
                        if r == "future_seq" {
                            self.report.future_seqs += 1;
                        }
                        self.lp.next(Event::Refused {
                            reason: r.to_owned(),
                            finished: v["error"]["data"]["finished"].as_bool().unwrap_or(false),
                        })
                    }
                    _ => Action::Ignore(String::new()),
                }
            }
        }
    }

    /// Runs the loop to `stream.end` (or a stop), within `limit`.
    pub fn run(mut self, limit: Duration) -> Report {
        wait_socket(&self.sock);
        let deadline = Instant::now() + limit;
        let _ = self.lp.start();
        let mut action = match self.reconnect() {
            Event::Status { .. } => Action::Subscribe(0),
            e => self.lp.next(e),
        };
        loop {
            assert!(
                Instant::now() < deadline,
                "{}: no stream.end within {limit:?}",
                self.report.name
            );
            action = match action {
                Action::Subscribe(seq) => {
                    if self.request("stream.subscribe", Some(SubscribeFrom::Seq(seq).params())) {
                        self.next_event()
                    } else {
                        self.lp.next(Event::Closed)
                    }
                }
                Action::SubscribeLive => {
                    if self.request("stream.subscribe", Some(SubscribeFrom::Live.params())) {
                        self.next_event()
                    } else {
                        self.lp.next(Event::Closed)
                    }
                }
                Action::Unsubscribe => {
                    if self.request("stream.unsubscribe", None) {
                        self.next_event()
                    } else {
                        self.lp.next(Event::Closed)
                    }
                }
                Action::ReadSealed { from, to } => {
                    let lines = self.sealed_read(from, to);
                    self.deliver(from, &lines, false);
                    self.lp.next(Event::SealedRead {
                        first_seq: from,
                        count: u64::try_from(lines.len()).unwrap(),
                    })
                }
                Action::Wait(polls, _) => {
                    self.report.retries += 1;
                    // One poll is 2 ms: the longest backoff (64 polls, wire_v1.md §6.3) is then
                    // 128 ms, inside the window of a resumed host paced at 5 ms a stop, so the
                    // viewer's retry lands before that host's window passes its cursor (DEF-1).
                    std::thread::sleep(BACKOFF_POLL * u32::try_from(polls).unwrap());
                    self.lp.next(Event::RetryElapsed)
                }
                Action::Status => {
                    let e = self.reconnect();
                    self.lp.next(e)
                }
                Action::Done(_, reason) => {
                    self.report.end = Some(reason);
                    if self.report.jump.is_some() {
                        // The seqs skipped, as Catchup counts them (frames in flight under the
                        // jump's unsubscribe were delivered, R15).
                        self.report.jump = self.lp.skipped;
                    }
                    return self.report;
                }
                Action::Close(detail) => {
                    self.report.errors.push(format!("protocol error: {detail}"));
                    self.conn = None;
                    self.lp.next(Event::Closed)
                }
                Action::Stop(detail) => {
                    self.report.errors.push(format!("stopped: {detail}"));
                    return self.report;
                }
                Action::Deliver { .. } | Action::Ignore(_) => self.next_event(),
            };
        }
    }
}

/// The seqs a report received are `0..next_seq` exactly (a jumper: contiguous on each side).
pub fn check_seqs(r: &Report, next_seq: u64) -> Result<(), String> {
    let seqs: Vec<u64> = r.received.iter().map(|(s, _)| *s).collect();
    match r.jump {
        None => {
            if seqs != (0..next_seq).collect::<Vec<_>>() {
                return Err(format!(
                    "{}: seqs are not 0..{next_seq} ({} received)",
                    r.name,
                    seqs.len()
                ));
            }
        }
        Some((from, to)) => {
            let before: Vec<u64> = seqs.iter().copied().filter(|&s| s < from).collect();
            let after: Vec<u64> = seqs.iter().copied().filter(|&s| s >= from).collect();
            if before != (0..from).collect::<Vec<_>>()
                || after != (to..next_seq).collect::<Vec<_>>()
            {
                return Err(format!(
                    "{}: seqs around the jump {from}->{to} are not contiguous",
                    r.name
                ));
            }
        }
    }
    Ok(())
}

/// The sealed channel-A segments' canonical digests of a run directory, by sim-year, and the next
/// seq.
pub fn segment_digests(run_dir: &Path) -> (BTreeMap<i64, String>, u64) {
    let segs = rh_stream::writer::read_channel_a(&run_dir.join("stream"), 0, 518_400).unwrap();
    let next = segs.last().map_or(0, |s| s.row.last_seq + 1);
    (
        segs.iter()
            .map(|s| {
                (
                    i64::from(s.row.sim_year),
                    super::hex(&s.row.canonical_digest),
                )
            })
            .collect(),
        next,
    )
}
