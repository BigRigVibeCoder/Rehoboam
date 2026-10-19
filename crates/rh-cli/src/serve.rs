//! The World Stream's live edge over a Unix socket (`specs/stream/wire_v1.md`; SPR-026 T-03 to
//! T-05): the one file of `rh-cli` allowed a socket (`bin/lint_forbidden.py` `NETWORK_FILES`).
//!
//! **The host** (`rh-cli run <world> … --run-dir D --socket`): the engine host serves channel A
//! on `D/stream/t0000000000/A.sock` (§1):
//! - the run directory is mode `0700` and the socket `0600`; `A.lock`, beside the socket, is held
//!   with `flock` for the host's life, so a second server cannot start and nobody unlinks a live
//!   socket; a stale socket (a killed host's) is removed under the lock; a socket path over 107
//!   bytes is refused before anything is written, naming the path;
//! - each connection gets its own serving thread, up to `--max-viewers` (64; `too_many_viewers`,
//!   then close); control frames go through `rh_protocol::control::dispatch`; RECORDS frames are
//!   byte ranges of `rh_stream`'s `LiveEdge`, at most 1 MiB each; `stream.sealed`, `stream.behind`
//!   and `stream.end` are pulled from the edge's seal log and end, one snapshot per step;
//! - a serving thread that has nothing to send polls with a short sleep: the writer never waits on
//!   it, and a viewer that never reads blocks its own thread only;
//! - `run_nonce` is the wall clock in nanoseconds (never a pid, never in plane A);
//! - after the run, the host keeps serving until every viewer has disconnected or `--linger`
//!   seconds (10) have passed, then removes its socket;
//! - every seal's latency is logged against its sim-year's wall time (`--seal-log` prints each),
//!   with the window policy `--window year-end` (D42 (b), the default) or `sync` (a).
//!
//! **Channel C** (`rh-cli sift D --follow --socket`): the sifter sifts each channel-A segment as
//! it seals and serves `C.sock` under `C.lock`, through the same server (§6.4).
//!
//! **The viewer** (`rh-cli tail D --live [--channel A|C]`): `rh_protocol::cursor::Catchup` over
//! the socket, the sealed ranges read with `rh-stream`'s reader (§6.3).
//!
//! REF: `specs/stream/wire_v1.md` §1, §4, §6; REV-SPR-026 R11–R14, R20, R21, R24; ADR-0030.

use std::cell::RefCell;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use rh_golden::governor::Governor;
use rh_golden::run::{Error, LiveHost};
use rh_golden::stream::{RunToml, TIMELINE};
use rh_protocol::control::{self, Connection, ControlConn, EndReason, History, HostView, Source};
use rh_protocol::records::{self, Channel, SUBHEADER_LEN};
use rh_protocol::wire::{self, Frame, FrameType, MAX_PAYLOAD};
use rh_stream::live::{End, LiveEdge, Policy, Snapshot};
use rh_stream::writer::SealClock;

/// The longest Unix socket path (`sun_path` holds 108 bytes with its NUL).
pub const SUN_PATH_MAX: usize = 107;
/// How long an idle serving thread sleeps between polls.
const POLL: Duration = Duration::from_millis(1);
/// How long the accept loop sleeps between polls.
const ACCEPT_POLL: Duration = Duration::from_millis(2);

/// The directory of a timeline's sockets and locks (`<run-dir>/stream/t<id, 10 digits>`).
#[must_use]
pub fn timeline_dir(run_dir: &Path) -> PathBuf {
    run_dir.join("stream").join(format!("t{TIMELINE:010}"))
}

/// The socket and lock paths of a channel.
#[must_use]
pub fn socket_paths(run_dir: &Path, channel: Channel) -> (PathBuf, PathBuf) {
    let dir = timeline_dir(run_dir);
    let c = channel.letter();
    (dir.join(format!("{c}.sock")), dir.join(format!("{c}.lock")))
}

/// The socket and lock paths of the control channel (`control_v1.md` §1).
#[must_use]
pub fn control_paths(run_dir: &Path) -> (PathBuf, PathBuf) {
    let dir = timeline_dir(run_dir);
    (dir.join("control.sock"), dir.join("control.lock"))
}

/// Refuses a socket path the platform cannot bind (§1: never a silent fallback).
fn check_path(sock: &Path) -> Result<(), String> {
    let len = sock.as_os_str().len();
    if len > SUN_PATH_MAX {
        return Err(format!(
            "the socket path {} is {len} bytes, over the {SUN_PATH_MAX} a Unix socket allows: \
             choose a shorter run directory; nothing was written",
            sock.display()
        ));
    }
    Ok(())
}

/// Takes the channel's server lock, exclusively, for the server's life (R20).
fn take_lock(lock: &Path) -> Result<File, String> {
    let f = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(lock)
        .map_err(|e| format!("{}: {e}", lock.display()))?;
    match f.try_lock() {
        Ok(()) => Ok(f),
        Err(std::fs::TryLockError::WouldBlock) => Err(format!(
            "{}: another server holds this socket's lock; a second server for one socket cannot \
             start",
            lock.display()
        )),
        Err(std::fs::TryLockError::Error(e)) => Err(format!("{}: {e}", lock.display())),
    }
}

/// The static half of a socket's `stream.status`.
#[derive(Clone, Debug)]
struct Info {
    channel: Channel,
    record_format: String,
    history: History,
    minutes_per_year: i64,
    run_nonce: u64,
}

/// What the serving threads share: read-only but for counters.
struct Shared {
    info: Info,
    edge: LiveEdge,
    governor: Option<Arc<Governor>>,
    cap: usize,
    active: AtomicUsize,
    connections: AtomicU64,
    /// Viewer frames answered (`--test-wait-viewer` waits for the first).
    requests: AtomicU64,
    frames: AtomicU64,
    largest_frame: AtomicUsize,
    peak_bytes: AtomicUsize,
    shutdown: AtomicBool,
}

impl Shared {
    fn view_of(&self, snap: &Snapshot) -> HostView {
        let finished = snap.end.is_some();
        HostView {
            channel: self.info.channel,
            timeline_id: TIMELINE,
            run_nonce: self.info.run_nonce,
            record_format: self.info.record_format.clone(),
            history: self.info.history.clone(),
            minutes_per_year: self.info.minutes_per_year,
            tip_tick: snap.tip_tick,
            next_seq: snap.next_seq,
            live_first_seq: snap.w0,
            sealed_last_year: snap.last_year,
            sealed_last_seq: snap.s,
            pace: (self.info.channel == Channel::A).then(|| {
                if let Some(gov) = &self.governor {
                    let s = gov.snapshot();
                    (s.band.clone(), s.state.clone())
                } else {
                    let state = if finished { "finished" } else { "running" };
                    ("max".to_owned(), state.to_owned())
                }
            }),
            sifted_through: (self.info.channel == Channel::C).then(|| {
                let u = snap.upstream.unwrap_or_default();
                (u.a_seq, u.a_year)
            }),
            finished,
            seal_log_len: snap.seal_log.len(),
        }
    }

    fn sample(&self) {
        self.peak_bytes
            .fetch_max(self.edge.bytes(), Ordering::Relaxed);
    }
}

impl Source for Shared {
    fn view(&self) -> HostView {
        self.view_of(&self.edge.snapshot())
    }

    fn seal_index(&self, from_seq: Option<u64>) -> usize {
        let Some(n) = from_seq else {
            return self.edge.snapshot().seal_log.len();
        };
        let snap = self.edge.snapshot();
        snap.seal_log
            .iter()
            .position(|e| match e.last_seq {
                Some(l) => l >= n,
                None => e.first_seq >= n,
            })
            .unwrap_or(snap.seal_log.len())
    }
}

fn send(stream: &mut UnixStream, shared: &Shared, frame: &[u8]) -> std::io::Result<()> {
    stream.write_all(frame)?;
    shared.frames.fetch_add(1, Ordering::Relaxed);
    shared
        .largest_frame
        .fetch_max(frame.len(), Ordering::Relaxed);
    Ok(())
}

fn send_value(
    stream: &mut UnixStream,
    shared: &Shared,
    v: &serde_json::Value,
) -> std::io::Result<()> {
    let frame = control::control_frame(v).map_err(|e| std::io::Error::other(e.message))?;
    send(stream, shared, &frame)
}

/// One step of a subscription (§6.2): one snapshot; `stream.behind` if the cursor left the
/// window; the seal-log entries it has passed; one RECORDS frame copied from that snapshot, or
/// `stream.end` at the end. Returns whether it sent anything.
fn serve_step(
    stream: &mut UnixStream,
    conn: &mut Connection,
    shared: &Shared,
) -> std::io::Result<bool> {
    let Some(c) = conn.cursor else {
        return Ok(false);
    };
    let snap = shared.edge.snapshot();
    if c < snap.w0 {
        conn.cursor = None;
        let v = control::notify_behind(&shared.view_of(&snap), c);
        drop(snap);
        send_value(stream, shared, &v)?;
        return Ok(true);
    }
    let mut sealed_notifs = Vec::new();
    while let Some(e) = snap.seal_log.get(conn.seal_index) {
        if e.last_seq.is_some_and(|l| l >= c) {
            break;
        }
        if e.last_seq.is_none() && e.first_seq > c {
            break;
        }
        let sealed = control::Sealed {
            sim_year: e.sim_year,
            first_seq: e.first_seq,
            last_seq: e.last_seq,
            canonical_digest: hex16(&e.canonical_digest),
        };
        sealed_notifs.push(control::notify_sealed(
            shared.info.channel,
            TIMELINE,
            &sealed,
        ));
        conn.seal_index += 1;
    }
    if !sealed_notifs.is_empty() {
        drop(snap);
        for notif in sealed_notifs {
            send_value(stream, shared, &notif)?;
        }
        return Ok(true);
    }
    let lines = snap
        .read(c, MAX_PAYLOAD - SUBHEADER_LEN)
        .unwrap_or_default();
    if lines.count > 0 {
        let count = u32::try_from(lines.count).unwrap_or(u32::MAX);
        let frame = records::payload(TIMELINE, shared.info.channel, c, count, &lines.body)
            .and_then(|payload| {
                wire::encode(&Frame {
                    ftype: FrameType::Records,
                    payload,
                })
            })
            .map_err(|e| std::io::Error::other(e.message))?;
        let next_c = c + lines.count;
        drop(lines);
        drop(snap);
        send(stream, shared, &frame)?;
        conn.cursor = Some(next_c);
        return Ok(true);
    }
    if let Some(end) = snap.end
        && c >= snap.next_seq
    {
        conn.cursor = None;
        let reason = match end {
            End::Finished => EndReason::Finished,
            End::Aborted => EndReason::Aborted,
        };
        let notif = control::notify_end(shared.info.channel, TIMELINE, c, reason);
        drop(snap);
        send_value(stream, shared, &notif)?;
        return Ok(true);
    }
    Ok(false)
}

fn hex16(b: &[u8; 16]) -> String {
    use std::fmt::Write as _;
    b.iter().fold(String::with_capacity(32), |mut s, x| {
        let _ = write!(s, "{x:02x}");
        s
    })
}

struct ActiveGuard<'a>(&'a AtomicUsize);

impl Drop for ActiveGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// A serving thread: the viewer's frames dispatched, its subscription served, until it closes,
/// the host closes it after a refusal, or the host shuts down.
fn serve_connection(mut stream: UnixStream, shared: &Shared) {
    let _active = ActiveGuard(&shared.active);
    let mut conn = Connection::default();
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = vec![0u8; 8192];
    'serve: while !shared.shutdown.load(Ordering::Relaxed) {
        let mut idle = true;
        if stream.set_nonblocking(true).is_err() {
            break;
        }
        let got = stream.read(&mut tmp);
        if stream.set_nonblocking(false).is_err() {
            break;
        }
        match got {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                idle = false;
            }
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {}
            Err(_) => break,
        }
        while !buf.is_empty() {
            let reply = control::dispatch(&mut conn, shared, &buf);
            if reply.frames.is_empty() && reply.used == 0 {
                break; // an incomplete frame: read more
            }
            shared.requests.fetch_add(1, Ordering::Relaxed);
            for f in &reply.frames {
                if send(&mut stream, shared, f).is_err() {
                    break 'serve;
                }
            }
            if reply.close {
                let _ = stream.shutdown(std::net::Shutdown::Both);
                break 'serve;
            }
            buf.drain(..reply.used.min(buf.len()));
        }
        match serve_step(&mut stream, &mut conn, shared) {
            Ok(true) => idle = false,
            Ok(false) => {}
            Err(_) => break,
        }
        if idle {
            std::thread::sleep(POLL);
        }
    }
}

fn accept_loop(listener: &UnixListener, shared: &Arc<Shared>) {
    while !shared.shutdown.load(Ordering::Relaxed) {
        shared.sample();
        match listener.accept() {
            Ok((mut stream, _)) => {
                if stream.set_nonblocking(false).is_err() {
                    continue;
                }
                shared.connections.fetch_add(1, Ordering::Relaxed);
                if shared.active.load(Ordering::Relaxed) >= shared.cap {
                    let _ = stream.write_all(&control::too_many_viewers(shared.cap));
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    continue;
                }
                shared.active.fetch_add(1, Ordering::Relaxed);
                let s = Arc::clone(shared);
                let spawned = std::thread::Builder::new()
                    .name("rh-serve".to_owned())
                    .spawn(move || serve_connection(stream, &s));
                if spawned.is_err() {
                    shared.active.fetch_sub(1, Ordering::Relaxed);
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => std::thread::sleep(ACCEPT_POLL),
            Err(_) => std::thread::sleep(ACCEPT_POLL),
        }
    }
}

/// A running socket server (module docs).
pub struct Server {
    shared: Arc<Shared>,
    sock: PathBuf,
    accept: Option<JoinHandle<()>>,
    _lock: File,
}

impl Server {
    /// Binds `sock` under the held `lock` (a stale socket removed first), mode `0600`, and starts
    /// accepting.
    fn start(
        sock: &Path,
        lock: File,
        info: Info,
        edge: LiveEdge,
        cap: usize,
        governor: Option<Arc<Governor>>,
    ) -> Result<Self, String> {
        check_path(sock)?;
        if sock.exists() {
            // Stale: its server died (we hold the lock, so none is alive).
            std::fs::remove_file(sock).map_err(|e| format!("{}: {e}", sock.display()))?;
        }
        let listener = UnixListener::bind(sock).map_err(|e| format!("{}: {e}", sock.display()))?;
        std::fs::set_permissions(sock, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("{}: {e}", sock.display()))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("{}: {e}", sock.display()))?;
        let shared = Arc::new(Shared {
            info,
            edge,
            governor,
            cap,
            active: AtomicUsize::new(0),
            connections: AtomicU64::new(0),
            requests: AtomicU64::new(0),
            frames: AtomicU64::new(0),
            largest_frame: AtomicUsize::new(0),
            peak_bytes: AtomicUsize::new(0),
            shutdown: AtomicBool::new(false),
        });
        let s = Arc::clone(&shared);
        let accept = std::thread::Builder::new()
            .name("rh-accept".to_owned())
            .spawn(move || accept_loop(&listener, &s))
            .map_err(|e| format!("the accept thread: {e}"))?;
        Ok(Self {
            shared,
            sock: sock.to_path_buf(),
            accept: Some(accept),
            _lock: lock,
        })
    }

    /// Blocks until a viewer's first frame has been answered, or `limit` has passed (`false`).
    fn wait_for_request(&self, limit: Duration) -> bool {
        // A count of polls, not a wall-clock deadline (the clock is the Clock's alone).
        let polls = limit.as_millis() / POLL.as_millis().max(1);
        for _ in 0..polls {
            if self.shared.requests.load(Ordering::Relaxed) > 0 {
                return true;
            }
            std::thread::sleep(POLL);
        }
        self.shared.requests.load(Ordering::Relaxed) > 0
    }

    /// Serves until every viewer has gone or `linger` has passed, then stops and removes the
    /// socket. Prints the server's figures (V-PERF-09's ingredients).
    fn finish(mut self, linger: Duration, name: &str) {
        let step = Duration::from_millis(10);
        let mut waited = Duration::ZERO;
        let mut quiet = Duration::ZERO;
        let grace = if linger < Duration::from_millis(500) {
            linger
        } else {
            Duration::from_millis(500)
        };
        // An unattended run (no connection ever accepted) exits without lingering.
        if self.shared.connections.load(Ordering::Relaxed) > 0 {
            while waited < linger {
                self.shared.sample();
                if self.shared.active.load(Ordering::Relaxed) > 0 {
                    quiet = Duration::ZERO;
                } else {
                    quiet += step;
                    if quiet >= grace {
                        break;
                    }
                }
                std::thread::sleep(step);
                waited += step;
            }
        }
        self.shared.sample();
        self.shared.shutdown.store(true, Ordering::Relaxed);
        if let Some(a) = self.accept.take() {
            let _ = a.join();
        }
        let _ = std::fs::remove_file(&self.sock);
        let sh = &self.shared;
        println!(
            "{name}: served {} connection(s), {} frame(s), the largest {} bytes (cap {} per frame); \
             peak LiveEdge::bytes() {}; {} viewer(s) still connected at exit",
            sh.connections.load(Ordering::Relaxed),
            sh.frames.load(Ordering::Relaxed),
            sh.largest_frame.load(Ordering::Relaxed),
            wire::HEADER_LEN + MAX_PAYLOAD,
            sh.peak_bytes.load(Ordering::Relaxed),
            sh.active.load(Ordering::Relaxed),
        );
    }
}

/// A running control socket server (`control_v1.md` §1).
pub struct ControlServer {
    governor: Arc<Governor>,
    sock: PathBuf,
    accept: Option<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
    _lock: File,
}

impl ControlServer {
    /// Binds `control.sock` under the held `lock`, mode `0600`, and starts accepting.
    fn start(sock: &Path, lock: File, governor: Arc<Governor>) -> Result<Self, String> {
        check_path(sock)?;
        if sock.exists() {
            std::fs::remove_file(sock).map_err(|e| format!("{}: {e}", sock.display()))?;
        }
        let listener = UnixListener::bind(sock).map_err(|e| format!("{}: {e}", sock.display()))?;
        std::fs::set_permissions(sock, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("{}: {e}", sock.display()))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("{}: {e}", sock.display()))?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let s_shut = Arc::clone(&shutdown);
        let s_gov = Arc::clone(&governor);
        let accept = std::thread::Builder::new()
            .name("rh-ctrl-accept".to_owned())
            .spawn(move || control_accept_loop(&listener, &s_gov, &s_shut))
            .map_err(|e| format!("the control accept thread: {e}"))?;
        Ok(Self {
            governor,
            sock: sock.to_path_buf(),
            accept: Some(accept),
            shutdown,
            _lock: lock,
        })
    }

    /// Serves for grace period (up to `linger`, default 2s), ending early when 0 controllers connected (C15).
    fn finish(mut self, linger: Duration) {
        let poll_interval = Duration::from_millis(10);
        let mut waited = Duration::ZERO;
        while waited < linger {
            if self.governor.connection_count() == 0 {
                break;
            }
            std::thread::sleep(poll_interval);
            waited += poll_interval;
        }
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(h) = self.accept.take() {
            let _ = h.join();
        }
        if self.sock.exists() {
            let _ = std::fs::remove_file(&self.sock);
        }
    }
}

fn control_accept_loop(
    listener: &UnixListener,
    governor: &Arc<Governor>,
    shutdown: &Arc<AtomicBool>,
) {
    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                if stream.set_nonblocking(false).is_err() {
                    continue;
                }
                let Some(conn) = governor.connect(None) else {
                    let _ = stream.write_all(&control::too_many_controllers(governor.cap()));
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    continue;
                };
                if conn.closed {
                    let _ = stream.write_all(&control::too_many_controllers(governor.cap()));
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    governor.close(&conn);
                    continue;
                }
                let g = Arc::clone(governor);
                let shut = Arc::clone(shutdown);
                let _ = std::thread::Builder::new()
                    .name("rh-ctrl-conn".to_owned())
                    .spawn(move || serve_control_connection(stream, conn, &g, &shut));
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => std::thread::sleep(ACCEPT_POLL),
            Err(_) => std::thread::sleep(ACCEPT_POLL),
        }
    }
}

fn serve_control_connection(
    mut stream: UnixStream,
    mut conn: ControlConn,
    governor: &Arc<Governor>,
    shutdown: &Arc<AtomicBool>,
) {
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = vec![0u8; 4096];
    'serve: while !shutdown.load(Ordering::Relaxed) {
        let mut idle = true;
        if stream.set_nonblocking(true).is_err() {
            break;
        }
        let got = stream.read(&mut tmp);
        if stream.set_nonblocking(false).is_err() {
            break;
        }
        match got {
            Ok(0) => {
                // EOF: L9 rule - drain remaining requests in buffer before exiting!
                while !buf.is_empty() {
                    let reply = control::dispatch_control(&mut conn, governor.as_ref(), &buf);
                    if reply.frames.is_empty() && reply.used == 0 {
                        break;
                    }
                    buf.drain(..reply.used.min(buf.len()));
                    if reply.close {
                        break;
                    }
                }
                break;
            }
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                idle = false;
            }
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {}
            Err(_) => break,
        }
        while !buf.is_empty() {
            let reply = control::dispatch_control(&mut conn, governor.as_ref(), &buf);
            if reply.frames.is_empty() && reply.used == 0 {
                break;
            }
            for f in &reply.frames {
                if stream.write_all(f).is_err() {
                    buf.drain(..reply.used.min(buf.len()));
                    while !buf.is_empty() {
                        let rep = control::dispatch_control(&mut conn, governor.as_ref(), &buf);
                        if rep.frames.is_empty() && rep.used == 0 {
                            break;
                        }
                        buf.drain(..rep.used.min(buf.len()));
                        if rep.close {
                            break;
                        }
                    }
                    break 'serve;
                }
            }
            buf.drain(..reply.used.min(buf.len()));
            if reply.close {
                let _ = stream.shutdown(std::net::Shutdown::Both);
                break 'serve;
            }
        }
        if idle {
            std::thread::sleep(POLL);
        }
    }
    governor.close(&conn);
}

/// The host's seal clock: nanoseconds since the host started (tool plane: plane A reads none).
struct Clock {
    #[allow(
        clippy::disallowed_types,
        reason = "tool plane: the seal-latency log is wall time by definition (wire_v1.md §6.2)"
    )]
    start: std::time::Instant,
    log: Mutex<Vec<(i64, u64, u64)>>,
}

impl Clock {
    #[allow(
        clippy::disallowed_types,
        clippy::disallowed_methods,
        reason = "tool plane: the seal-latency log is wall time by definition (wire_v1.md §6.2)"
    )]
    fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
            log: Mutex::new(Vec::new()),
        }
    }

    /// The log as lines and a summary: each seal's latency against its sim-year's wall time (the
    /// time from the previous year's end to its own).
    fn report(&self, all: bool) -> Vec<String> {
        let log = self.log.lock().map(|l| l.clone()).unwrap_or_default();
        let mut out = Vec::new();
        let mut latencies: Vec<u64> = Vec::new();
        let mut slow = 0;
        let mut prev_close = 0;
        for &(year, closed, landed) in &log {
            let latency = landed.saturating_sub(closed);
            let year_wall = closed.saturating_sub(prev_close);
            prev_close = closed;
            if latency > year_wall {
                slow += 1;
            }
            latencies.push(latency);
            if all {
                out.push(format!(
                    "seal y{year:06}: {} us after its year's end (the sim-year took {} us)",
                    latency / 1000,
                    year_wall / 1000
                ));
            }
        }
        latencies.sort_unstable();
        let q = |p: usize| {
            latencies
                .get((latencies.len().saturating_sub(1)) * p / 100)
                .map_or(0, |n| n / 1000)
        };
        out.push(format!(
            "seal latency over {} seal(s): p50 {} us, p90 {} us, max {} us; {slow} slower than \
             their sim-year's wall time (the overrun of wire_v1.md §6.1 (b))",
            latencies.len(),
            q(50),
            q(90),
            q(100)
        ));
        out
    }
}

impl SealClock for Clock {
    fn now(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    fn landed(&self, year: i64, closed: u64, landed: u64) {
        if let Ok(mut l) = self.log.lock() {
            l.push((year, closed, landed));
        }
    }
}

/// A fresh `run_nonce` (§3.2): the wall clock in nanoseconds.
#[allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    reason = "tool plane: run_nonce is the wall clock by definition, never in plane A (R21)"
)]
fn run_nonce() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
}

/// The run directory: set mode 0700 if created by this host; an existing directory keeps its mode
/// with a warning if it is group- or world-accessible (C4).
fn private_dir(dir: &Path, existed: bool) -> Result<(), String> {
    let meta = std::fs::metadata(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mode = meta.permissions().mode();
    if existed {
        if mode & 0o077 != 0 {
            eprintln!(
                "warning: run directory {} is accessible to other users ({:#05o})",
                dir.display(),
                mode & 0o777
            );
        }
    } else {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    Ok(())
}

fn info_of(run: &RunToml, channel: Channel, record_format: &str, nonce: u64) -> Info {
    Info {
        channel,
        record_format: record_format.to_owned(),
        history: History {
            engine_build: run.engine_build.clone(),
            registry_version: run.registry_version.clone(),
            manifest_hash: run.manifest_hash.clone(),
            world_seed: run.seed,
        },
        minutes_per_year: run.minutes_per_year,
        run_nonce: nonce,
    }
}

/// The serving options a host takes out of its command line.
struct Opts {
    linger: Duration,
    policy: Policy,
    cap: usize,
    seal_log: bool,
    /// Bounded wait for run.toml (default 60 s; C9).
    timeout: u64,
    /// A test hook (`--test-wait-viewer`; SPR-026 DEF-1): the run starts only once a viewer's
    /// first request has been answered, so a test's viewer is attached before anything is
    /// emitted, however loaded the machine.
    wait_viewer: bool,
}

/// Takes `--socket`, `--linger S`, `--window year-end|sync`, `--max-viewers N`, `--seal-log`,
/// `--timeout S` and the test hook `--test-wait-viewer` out of `words`; the rest is the run's.
fn take_opts(words: &[&str]) -> Result<(Opts, Vec<String>), String> {
    let mut opts = Opts {
        linger: Duration::from_secs(10),
        policy: Policy::YearEnd,
        cap: control::DEFAULT_CAP,
        seal_log: false,
        timeout: 60,
        wait_viewer: false,
    };
    let mut rest = Vec::new();
    let mut it = words.iter();
    while let Some(&w) = it.next() {
        let mut value = || {
            it.next()
                .copied()
                .ok_or_else(|| format!("{w} needs a value"))
        };
        match w {
            "--socket" => {}
            "--seal-log" => opts.seal_log = true,
            "--test-wait-viewer" => opts.wait_viewer = true,
            "--timeout" => {
                let v = value()?;
                opts.timeout = v
                    .parse()
                    .map_err(|_| format!("--timeout {v} is not a number of seconds"))?;
            }
            "--linger" => {
                let v = value()?;
                opts.linger = Duration::from_secs(
                    v.parse()
                        .map_err(|_| format!("--linger {v} is not a number of seconds"))?,
                );
            }
            "--window" => {
                opts.policy = match value()? {
                    "year-end" => Policy::YearEnd,
                    "sync" => Policy::Sync,
                    v => return Err(format!("--window {v}: year-end (D42 option b) or sync (a)")),
                };
            }
            "--max-viewers" => {
                let v = value()?;
                opts.cap = v
                    .parse()
                    .ok()
                    .filter(|&n| n > 0)
                    .ok_or_else(|| format!("--max-viewers {v} is not a positive number"))?;
            }
            other => rest.push(other.to_owned()),
        }
    }
    Ok((opts, rest))
}

/// Waits up to `timeout_secs` (default 60 s) for `run.toml` to appear in `dir`.
/// Prints the file it waits for if not immediately present (C9).
fn wait_run_toml(dir: &Path, timeout_secs: u64) -> Result<(), String> {
    let path = dir.join(rh_golden::stream::RUN_TOML);
    if path.is_file() {
        return Ok(());
    }
    eprintln!(
        "waiting for {} (timeout {timeout_secs} s)...",
        path.display()
    );
    let max_iters = timeout_secs.saturating_mul(20);
    let mut iters = 0u64;
    while !path.is_file() {
        if iters >= max_iters {
            return Err(format!(
                "timed out after {timeout_secs} s waiting for {}",
                path.display()
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
        iters += 1;
    }
    Ok(())
}

fn flag_value(words: &[String], flag: &str) -> Option<String> {
    words
        .iter()
        .position(|w| w == flag)
        .and_then(|i| words.get(i + 1).cloned())
}

/// `rh-cli run <world> … --run-dir D --socket`: the run, serving channel A live (module docs).
/// Returns the exit code.
#[must_use]
pub fn run(world: &str, words: &[&str], repo: &Path) -> u8 {
    match run_inner(world, words, repo) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            rh_golden::cli::TOOL
        }
    }
}

fn run_inner(world: &str, words: &[&str], repo: &Path) -> Result<u8, String> {
    let (opts, rest) = take_opts(words)?;
    let run_dir = flag_value(&rest, "--run-dir")
        .map(PathBuf::from)
        .ok_or("--socket serves a run directory's stream: it needs --run-dir")?;
    let existed = run_dir.exists();
    let (sock, lock_path) = socket_paths(&run_dir, Channel::A);
    let (control_sock, control_lock_path) = control_paths(&run_dir);
    // Refused before anything is written (§1): check control_sock (the longer of the two paths: C24).
    check_path(&control_sock)?;
    // A resumed run's directory holds lock files: they are taken first, before the run touches
    // anything (a refusal writes nothing). Missing ones are created once the run's own checks
    // have passed.
    let early = if lock_path.is_file() {
        Some(take_lock(&lock_path)?)
    } else {
        None
    };
    let early_ctrl = if control_lock_path.is_file() {
        Some(take_lock(&control_lock_path)?)
    } else {
        None
    };
    let clock = Arc::new(Clock::new());
    let server: Rc<RefCell<Option<Server>>> = Rc::new(RefCell::new(None));
    let slot = Rc::clone(&server);
    let ctrl_server: Rc<RefCell<Option<ControlServer>>> = Rc::new(RefCell::new(None));
    let ctrl_slot = Rc::clone(&ctrl_server);
    let cap = opts.cap;
    let wait_viewer = opts.wait_viewer;
    let name = world.to_owned();
    let nonce = run_nonce();
    let on_edge =
        move |edge: LiveEdge, dir: &Path, governor: Option<Arc<Governor>>| -> Result<(), Error> {
            let run = RunToml::read(dir)?;
            private_dir(dir, existed).map_err(Error)?;
            let lock = if let Some(l) = early {
                l
            } else {
                std::fs::create_dir_all(timeline_dir(dir))
                    .map_err(|e| Error(format!("{}: {e}", dir.display())))?;
                take_lock(&lock_path).map_err(Error)?
            };
            let info = info_of(&run, Channel::A, rh_stream::segment::FORMAT_A, nonce);
            let s = Server::start(&sock, lock, info, edge, cap, governor.clone()).map_err(Error)?;
            println!("{name}: serving channel A live on {}", sock.display());
            if let Some(gov) = governor {
                let ctrl_lock = if let Some(l) = early_ctrl {
                    l
                } else {
                    std::fs::create_dir_all(timeline_dir(dir))
                        .map_err(|e| Error(format!("{}: {e}", dir.display())))?;
                    take_lock(&control_lock_path).map_err(Error)?
                };
                let cs = ControlServer::start(&control_sock, ctrl_lock, gov).map_err(Error)?;
                println!(
                    "{name}: serving control endpoint live on {}",
                    control_sock.display()
                );
                *ctrl_slot.borrow_mut() = Some(cs);
            }
            if wait_viewer {
                println!("{name}: waiting for a viewer's first request (--test-wait-viewer)");
                if !s.wait_for_request(Duration::from_mins(1)) {
                    return Err(Error(
                        "--test-wait-viewer: no viewer sent a request within 60 s".to_owned(),
                    ));
                }
            }
            *slot.borrow_mut() = Some(s);
            Ok(())
        };
    let live = LiveHost {
        policy: opts.policy,
        clock: Some(clock.clone()),
        nonce: Some(nonce),
        on_edge: Box::new(on_edge),
    };
    let code = rh_golden::cli::run_world_live(world, &rest, repo, Some(live));
    for line in clock.report(opts.seal_log) {
        println!("{world}: {line}");
    }
    if let Some(cs) = ctrl_server.borrow_mut().take() {
        cs.finish(Duration::from_secs(2));
    }
    if let Some(s) = server.borrow_mut().take() {
        s.finish(opts.linger, world);
    }
    Ok(code)
}

/// `rh-cli sift D --follow [--socket]` (SPR-026 T-04, §6.4): sifts each channel-A segment as it
/// seals and, with `--socket`, serves channel C on `C.sock` under `C.lock`. An empty sim-year is
/// sealed and announced (`stream.sealed` with `last_seq` null); `stream.status` carries
/// `sifted_through`. The engine process never serves channel C. Returns the exit code.
#[must_use]
pub fn sift(words: &[&str], repo: &Path) -> u8 {
    match sift_inner(words, repo) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            rh_golden::cli::TOOL
        }
    }
}

fn sift_inner(words: &[&str], repo: &Path) -> Result<u8, String> {
    let socket = words.contains(&"--socket");
    let words: Vec<&str> = words.iter().copied().filter(|w| *w != "--follow").collect();
    let (opts, rest) = take_opts(&words)?;
    if opts.wait_viewer {
        return Err(
            "--test-wait-viewer is a hook of `rh-cli run --socket`, not of sift".to_owned(),
        );
    }
    let [dir] = rest.as_slice() else {
        return Err(
            "usage: rh-cli sift <run-dir> --follow [--socket] [--linger S] \
                    [--max-viewers N]"
                .to_owned(),
        );
    };
    let run_dir = PathBuf::from(dir);
    let (sock, lock_path) = socket_paths(&run_dir, Channel::C);
    if socket {
        check_path(&sock)?;
    }
    // The run must have started (its run.toml written after its own checks), so that nothing
    // the sifter creates can make the run refuse its directory (C9: bounded wait).
    wait_run_toml(&run_dir, opts.timeout)?;
    let edge = LiveEdge::new(Policy::YearEnd, rh_stream::live::Start::default());
    let server = if socket {
        let run = RunToml::read(&run_dir).map_err(|e| e.0)?;
        std::fs::create_dir_all(timeline_dir(&run_dir))
            .map_err(|e| format!("{}: {e}", run_dir.display()))?;
        let lock = take_lock(&lock_path)?;
        let info = info_of(&run, Channel::C, rh_sifter::sift::FORMAT_C, run_nonce());
        let s = Server::start(&sock, lock, info, edge.clone(), opts.cap, None)?;
        println!("sift: serving channel C live on {}", sock.display());
        Some(s)
    } else {
        None
    };
    let mut on_year = |s: &rh_sifter::sift::Sifted, lines: &[String]| -> Result<(), Error> {
        let year = i64::from(s.sim_year);
        let (mut bytes, mut ends) = (Vec::new(), Vec::new());
        for l in lines {
            bytes.extend_from_slice(l.as_bytes());
            bytes.push(b'\n');
            ends.push(u32::try_from(bytes.len()).map_err(|_| Error("a chunk over 4 GiB".into()))?);
        }
        let n = u64::try_from(lines.len()).unwrap_or(u64::MAX);
        let tick = s.records.last().map_or(0, |r| r.header.tick);
        edge.append(year, s.first_seq, bytes, ends, tick);
        edge.year_completes(year);
        // The sifter sealed the segment before handing it over: it is on disk.
        edge.seal_landed(
            year,
            Some(&rh_stream::live::SealEntry {
                sim_year: year,
                first_seq: s.first_seq,
                last_seq: (n > 0).then(|| s.first_seq + n - 1),
                canonical_digest: s.canonical_digest,
            }),
        );
        edge.set_upstream(rh_stream::live::Upstream {
            a_seq: Some(s.a_last_seq),
            a_year: Some(year),
        });
        Ok(())
    };
    let result = rh_golden::sift::sift_follow(repo, &run_dir, &mut on_year);
    edge.end(if result.is_ok() {
        End::Finished
    } else {
        End::Aborted
    });
    let code = match &result {
        Ok(s) => {
            println!(
                "sift: {} segments sealed, {} matches from {} channel-A records (followed)",
                s.segments, s.matches, s.a_records
            );
            rh_golden::cli::OK
        }
        Err(e) => {
            eprintln!("{e}");
            rh_golden::cli::TOOL
        }
    };
    if let Some(s) = server {
        s.finish(opts.linger, "sift");
    }
    Ok(code)
}

/// A viewer's connection: the socket and what it has read.
struct ViewerConn {
    s: UnixStream,
    buf: Vec<u8>,
}

/// What the viewer's socket gave it next.
enum Got {
    Frame(Frame),
    Closed,
}

impl ViewerConn {
    fn connect(sock: &Path) -> Option<Self> {
        let s = UnixStream::connect(sock).ok()?;
        s.set_read_timeout(Some(Duration::from_secs(30))).ok()?;
        Some(Self { s, buf: Vec::new() })
    }

    fn send(&mut self, v: &serde_json::Value) -> bool {
        control::control_frame(v)
            .ok()
            .is_some_and(|f| self.s.write_all(&f).is_ok())
    }

    fn next(&mut self) -> Result<Got, String> {
        loop {
            match wire::decode(&self.buf, MAX_PAYLOAD) {
                Ok((f, used)) => {
                    self.buf.drain(..used);
                    return Ok(Got::Frame(f));
                }
                Err(e) if e.is_incomplete() => {}
                Err(e) => return Err(format!("a protocol error from the host: {e}")),
            }
            let mut tmp = vec![0u8; 1 << 16];
            match self.s.read(&mut tmp) {
                Ok(0) => return Ok(Got::Closed),
                Ok(n) => self.buf.extend_from_slice(&tmp[..n]),
                Err(e)
                    if matches!(
                        e.kind(),
                        ErrorKind::Interrupted | ErrorKind::WouldBlock | ErrorKind::TimedOut
                    ) =>
                {
                    // C3: treat a read timeout as idle and keep waiting; a paused or slow host is not fatal.
                }
                Err(_) => return Ok(Got::Closed),
            }
        }
    }
}

/// `rh-cli tail D --live` (module docs; `wire_v1.md` §6.3).
struct LiveTail<'a> {
    run_dir: PathBuf,
    sock: PathBuf,
    channel_c: bool,
    canonical: bool,
    mpy: i64,
    display: Option<rh_golden::tail::Display>,
    filters: rh_golden::tail::Filters,
    out: &'a mut dyn Write,
    conn: Option<ViewerConn>,
    next_id: i64,
    /// `(id, method)` of the request in flight.
    pending: Option<(i64, &'static str)>,
    history: Option<serde_json::Value>,
    nonce: Option<u64>,
    /// Lines received live, by seq, of the sim-years a resumed host may re-emit (R21).
    recent: std::collections::BTreeMap<u64, String>,
    resumed: bool,
    verified: u64,
    printed: u64,
}

impl LiveTail<'_> {
    fn print(&mut self, first_seq: u64, lines: &[String]) -> Result<(), String> {
        for (i, l) in lines.iter().enumerate() {
            let seq = first_seq + u64::try_from(i).unwrap_or(u64::MAX);
            if !self.filters.matches_live_line(l) {
                continue;
            }
            let shown = match &self.display {
                Some(d) => rh_golden::tail::display_line(
                    &rh_stream::canonical::parse(l).map_err(|e| e.to_string())?,
                    d,
                ),
                None => l.clone(),
            };
            writeln!(self.out, "{shown}").map_err(|e| format!("tail: {e}"))?;
            self.recent.insert(seq, l.clone());
            self.printed += 1;
        }
        // Keep about two sim-years' worth: what a resumed host can re-emit.
        while self.recent.len() > 1 << 16 {
            self.recent.pop_first();
        }
        self.out.flush().map_err(|e| format!("tail: {e}"))
    }

    fn request(&mut self, method: &'static str, params: Option<serde_json::Value>) -> bool {
        self.next_id += 1;
        self.pending = Some((self.next_id, method));
        let v = control::request(&serde_json::json!(self.next_id), method, params);
        self.conn.as_mut().is_some_and(|c| c.send(&v))
    }

    /// Reconnects and asks `stream.status` (the `Status` action): the event it gives, or `None`
    /// when no host serves the socket any more and the writer is gone.
    fn reconnect(&mut self) -> Result<Option<Event>, String> {
        self.conn = None;
        let mut tries = 0u32;
        loop {
            if let Some(c) = ViewerConn::connect(&self.sock) {
                self.conn = Some(c);
                break;
            }
            if rh_golden::stream::writer_gone(&self.run_dir) && !self.sock.exists() {
                return Ok(None);
            }
            tries += 1;
            if tries > 3000 {
                return Err(format!("no host at {} for 5 minutes", self.sock.display()));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if !self.request("stream.status", None) {
            return Ok(Some(Event::Closed));
        }
        loop {
            let Some(conn) = self.conn.as_mut() else {
                return Ok(Some(Event::Closed));
            };
            let f = match conn.next()? {
                Got::Closed => return Ok(Some(Event::Closed)),
                Got::Frame(f) => f,
            };
            let (_, v) = control::decode_control(&f.payload).map_err(|e| e.message)?;
            if let Some(reason) = control::reply_reason(&v) {
                // The cap: too_many_viewers, then the host closes.
                return Ok(Some(Event::Refused {
                    reason: reason.to_owned(),
                    finished: false,
                }));
            }
            let Some(result) = v.get("result") else {
                continue;
            };
            let st = control::check_status(result).map_err(|e| e.message)?;
            let same = self.history.as_ref().is_none_or(|h| h == st.history());
            if same && self.nonce.is_some_and(|n| n != st.run_nonce) {
                self.resumed = true;
            }
            self.history = Some(st.history().clone());
            self.nonce = Some(st.run_nonce);
            self.pending = None;
            return Ok(Some(Event::Status { same_run: same }));
        }
    }

    /// The next event from the socket, printing the records the loop delivers.
    fn event(&mut self, lp: &mut Catchup) -> Result<Action, String> {
        let Some(conn) = self.conn.as_mut() else {
            return Ok(lp.next(Event::Closed));
        };
        let f = match conn.next()? {
            Got::Closed => return Ok(lp.next(Event::Closed)),
            Got::Frame(f) => f,
        };
        if f.ftype == FrameType::Records {
            let r = match records::decode(&f.payload) {
                Ok(r) => r,
                Err(e) => return Ok(lp.next_failed(&e.message)),
            };
            let count = u64::try_from(r.lines.len()).unwrap_or(u64::MAX);
            let action = lp.next(Event::Records {
                first_seq: r.first_seq,
                count,
            });
            if matches!(action, Action::Deliver { count, .. } if count > 0) {
                self.print(r.first_seq, &r.lines)?;
            }
            return Ok(action);
        }
        let (kind, v) = control::decode_control(&f.payload).map_err(|e| e.message)?;
        if kind == control::Kind::Notification {
            return Ok(match control::read_notice(&v).map_err(|e| e.message)? {
                control::Notice::Sealed(s) => {
                    self.verify_resumed(&s)?;
                    lp.next(Event::Sealed)
                }
                control::Notice::Behind {
                    next_seq,
                    sealed_last_seq,
                    live_first_seq,
                    ..
                } => lp.next(Event::Behind {
                    next_seq,
                    s: sealed_last_seq,
                    w0: live_first_seq,
                }),
                control::Notice::End { reason, .. } => lp.next(Event::End(reason)),
                control::Notice::Unknown(_) => Action::Ignore(String::new()),
            });
        }
        let method = self.pending.take().map(|(_, m)| m).unwrap_or_default();
        let reason = control::reply_reason(&v).map(str::to_owned);
        let result = |key: &str| v["result"][key].as_u64().unwrap_or_default();
        Ok(match (method, reason.as_deref()) {
            ("stream.subscribe", None) => lp.next(Event::Subscribed(result("first_seq"))),
            ("stream.unsubscribe", None) => lp.next(Event::Unsubscribed(result("next_seq"))),
            ("stream.subscribe", Some("behind_live_edge")) => {
                let d = &v["error"]["data"];
                lp.next(Event::Behind {
                    next_seq: lp.cursor,
                    s: d["sealed_last_seq"].as_u64(),
                    w0: d["live_first_seq"].as_u64().unwrap_or_default(),
                })
            }
            (_, Some(r)) => lp.next(Event::Refused {
                reason: r.to_owned(),
                finished: v["error"]["data"]["finished"].as_bool().unwrap_or(false),
            }),
            _ => Action::Ignore(String::new()),
        })
    }

    /// R21: after a resumed host, what this viewer received of a re-emitted year must be what the
    /// host sealed.
    fn verify_resumed(&mut self, s: &control::Sealed) -> Result<(), String> {
        let Some(last) = s.last_seq.filter(|_| self.resumed) else {
            return Ok(());
        };
        let lines: Option<Vec<&String>> =
            (s.first_seq..=last).map(|q| self.recent.get(&q)).collect();
        if let Some(lines) = lines {
            let digest = hex16(&rh_stream::canonical::digest(&lines));
            if digest != s.canonical_digest {
                return Err(format!(
                    "the resumed host's sim-year {} differs from what this viewer received",
                    s.sim_year
                ));
            }
            self.verified += 1;
        }
        Ok(())
    }
}

trait FailNext {
    fn next_failed(&mut self, detail: &str) -> Action;
}

impl FailNext for Catchup {
    /// A frame that does not decode is a protocol error: the loop closes, as for any (C13).
    fn next_failed(&mut self, detail: &str) -> Action {
        self.fail(detail.to_owned())
    }
}

use rh_protocol::cursor::{Action, Catchup, Event};

/// `rh-cli tail D --live [--channel A|C] [--from-seq N] [--canonical]` (module docs). Returns the
/// exit code: 0 at `stream.end` (or when the host is gone and the sealed tail is printed), 1 when
/// the stream ended aborted, 2 on an error.
#[must_use]
pub fn tail_live(words: &[&str], repo: &Path) -> u8 {
    match tail_live_inner(words, repo) {
        Ok(code) => code,
        Err(e) if e.contains("Broken pipe") => rh_golden::cli::OK,
        Err(e) => {
            eprintln!("{e}");
            rh_golden::cli::TOOL
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the viewer's loop in one place: every action of Catchup and what it does"
)]
fn tail_live_inner(words: &[&str], repo: &Path) -> Result<u8, String> {
    let (mut run_dir, mut channel_c, mut canonical, mut from) = (None, false, false, 0u64);
    let mut channel_specified = false;
    let mut to_seq: Option<u64> = None;
    let mut follow: Option<String> = None;
    let mut kinds: Vec<String> = Vec::new();
    let mut place: Option<String> = None;
    let mut pattern: Option<String> = None;
    let mut min_tellability: Option<u8> = None;
    let mut digest = false;
    let mut timeout = 60u64;
    let mut repo = repo.to_path_buf();
    let mut it = words.iter();
    while let Some(&w) = it.next() {
        let mut value = || {
            it.next()
                .copied()
                .ok_or_else(|| format!("{w} needs a value"))
        };
        match w {
            "--live" => {}
            "--canonical" => canonical = true,
            "--channel" => {
                channel_specified = true;
                channel_c = match value()? {
                    "A" => false,
                    "C" => true,
                    v => return Err(format!("--channel {v}: A or C")),
                };
            }
            "--from-seq" => {
                let v = value()?;
                from = v
                    .parse()
                    .map_err(|_| format!("--from-seq {v} is not a number"))?;
            }
            "--to-seq" => {
                let v = value()?;
                to_seq = Some(
                    v.parse()
                        .map_err(|_| format!("--to-seq {v} is not a number"))?,
                );
            }
            "--follow" => follow = Some(value()?.to_owned()),
            "--kind" => kinds.push(value()?.to_owned()),
            "--place" => place = Some(value()?.to_owned()),
            "--pattern" => {
                pattern = Some(value()?.to_owned());
                if !channel_specified {
                    channel_c = true;
                }
            }
            "--min-tellability" => {
                let v = value()?;
                min_tellability = Some(
                    v.parse()
                        .map_err(|_| format!("--min-tellability {v} is a number (0..255)"))?,
                );
            }
            "--digest" => {
                if it.clone().next() == Some(&"region") {
                    let _ = it.next();
                    return Err("tail: --digest region waits for regions in SPR-018".to_owned());
                }
                digest = true;
            }
            "--digest=region" => {
                return Err("tail: --digest region waits for regions in SPR-018".to_owned());
            }
            s if s == "--sig" || s.starts_with("--sig") => {
                return Err(
                    "tail: --sig waits for channel-C significance in P2.1; use --min-tellability N instead"
                        .to_owned(),
                );
            }
            "--timeout" => {
                let v = value()?;
                timeout = v
                    .parse()
                    .map_err(|_| format!("--timeout {v} is not a number"))?;
            }
            "--repo" => repo = PathBuf::from(value()?),
            flag if flag.starts_with("--") => {
                return Err(format!("tail --live: unknown flag {flag}"));
            }
            dir => run_dir = Some(PathBuf::from(dir)),
        }
    }
    let run_dir = run_dir.ok_or("tail --live needs a run directory")?;
    wait_run_toml(&run_dir, timeout)?;
    let run = RunToml::read(&run_dir).map_err(|e| e.0)?;
    let display = if canonical {
        None
    } else {
        Some(rh_golden::tail::display_of(&repo, &run).map_err(|e| e.0)?)
    };
    let tail_args = rh_golden::tail::TailArgs {
        run_dir: run_dir.clone(),
        repo: repo.clone(),
        from_seq: from,
        to_seq,
        follow,
        kinds,
        canonical,
        wait: false,
        timeout: Some(timeout),
        channel_c,
        place,
        pattern,
        min_tellability,
        digest,
    };
    let filters =
        rh_golden::tail::resolve_filters(&tail_args, display.as_ref()).map_err(|e| e.0)?;
    let channel = if channel_c { Channel::C } else { Channel::A };
    let (sock, _) = socket_paths(&run_dir, channel);
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    if canonical {
        let format = if channel_c {
            rh_sifter::sift::FORMAT_C
        } else {
            rh_stream::segment::FORMAT_A
        };
        let h = rh_stream::canonical::FileHeader {
            timeline_id: TIMELINE,
            channel: if channel_c {
                rh_stream::record::Channel::C
            } else {
                rh_stream::record::Channel::A
            },
            registry_version: run.registry_version.clone(),
            format: format.to_owned(),
        };
        writeln!(lock, "{}", h.line()).map_err(|e| format!("tail: {e}"))?;
    }
    let mut t = LiveTail {
        run_dir: run_dir.clone(),
        sock,
        channel_c,
        canonical,
        mpy: run.minutes_per_year,
        display,
        filters,
        out: &mut lock,
        conn: None,
        next_id: 0,
        pending: None,
        history: None,
        nonce: None,
        recent: std::collections::BTreeMap::new(),
        resumed: false,
        verified: 0,
        printed: 0,
    };
    let mut lp = Catchup::new(from);
    let mut protocol_errors = 0;
    let _ = lp.start();
    // Connect first: the loop's first subscribe needs a connection.
    let mut action = match t.reconnect()? {
        Some(Event::Status { .. }) => Action::Subscribe(from),
        Some(e) => lp.next(e),
        None => Action::Done(from, EndReason::Finished),
    };
    let prefix = if t.canonical { "# " } else { "" };
    loop {
        if let Some(m) = to_seq
            && lp.cursor > m
        {
            break Ok(rh_golden::cli::OK);
        }
        action = match action {
            Action::Subscribe(seq) => {
                if t.request(
                    "stream.subscribe",
                    Some(control::SubscribeFrom::Seq(seq).params()),
                ) {
                    t.event(&mut lp)?
                } else {
                    lp.next(Event::Closed)
                }
            }
            Action::SubscribeLive => {
                if t.request(
                    "stream.subscribe",
                    Some(control::SubscribeFrom::Live.params()),
                ) {
                    t.event(&mut lp)?
                } else {
                    lp.next(Event::Closed)
                }
            }
            Action::Unsubscribe => {
                if t.request("stream.unsubscribe", None) {
                    t.event(&mut lp)?
                } else {
                    lp.next(Event::Closed)
                }
            }
            Action::ReadSealed { from, to } => {
                let effective_to = match to_seq {
                    Some(m) => to.min(m),
                    None => to,
                };
                if from > effective_to {
                    lp.next(Event::SealedRead {
                        first_seq: from,
                        count: 0,
                    });
                    break Ok(rh_golden::cli::OK);
                }
                let lines = rh_golden::tail::sealed_lines(
                    &t.run_dir,
                    t.channel_c,
                    t.mpy,
                    from,
                    effective_to,
                )
                .map_err(|e| e.0)?;
                t.print(from, &lines)?;
                lp.next(Event::SealedRead {
                    first_seq: from,
                    count: u64::try_from(lines.len()).unwrap_or(u64::MAX),
                })
            }
            Action::Wait(polls, _) => {
                std::thread::sleep(Duration::from_millis(100) * u32::try_from(polls).unwrap_or(64));
                lp.next(Event::RetryElapsed)
            }
            Action::Status => {
                let Some(e) = t.reconnect()? else {
                    // No host any more, and the writer is gone: the rest is on disk.
                    let end_to = to_seq.unwrap_or(u64::MAX);
                    let mut added = 0;
                    if lp.cursor <= end_to {
                        let lines = rh_golden::tail::sealed_lines(
                            &t.run_dir,
                            t.channel_c,
                            t.mpy,
                            lp.cursor,
                            end_to,
                        )
                        .map_err(|e| e.0)?;
                        added = lines.len();
                        t.print(lp.cursor, &lines)?;
                    }
                    writeln!(
                        t.out,
                        "{prefix}STATUS live: no host serves {}; the sealed segments are printed \
                         through seq {}",
                        t.sock.display(),
                        lp.cursor + u64::try_from(added).unwrap_or(0)
                    )
                    .map_err(|e| format!("tail: {e}"))?;
                    return Ok(rh_golden::cli::OK);
                };
                lp.next(e)
            }
            Action::Done(cursor, reason) => {
                writeln!(
                    t.out,
                    "{prefix}STATUS live end: {} | cursor {cursor} | {} lines | {} resumed year(s) \
                     verified",
                    reason.name(),
                    t.printed,
                    t.verified
                )
                .map_err(|e| format!("tail: {e}"))?;
                return Ok(match reason {
                    EndReason::Finished => rh_golden::cli::OK,
                    EndReason::Aborted => rh_golden::cli::DIFFERENT,
                });
            }
            Action::Close(detail) => {
                protocol_errors += 1;
                eprintln!("tail --live: protocol error: {detail}");
                if protocol_errors > 3 {
                    return Err("tail --live: more than three protocol errors".to_owned());
                }
                t.conn = None;
                lp.next(Event::Closed)
            }
            Action::Stop(detail) => return Err(format!("tail --live: {detail}")),
            Action::Deliver { .. } | Action::Ignore(_) => t.event(&mut lp)?,
        };
    }
}
