//! Shared helpers of the live-edge tests (SPR-026): short scratch run directories, a host started
//! as a child process, and a raw wire client built on `rh-protocol` (each test drives its own
//! loop, independently of `rh-cli tail --live`).
#![allow(
    dead_code,
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers: each test binary uses some; child processes and a deadline clock; \
              helpers fail loudly"
)]

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use rh_protocol::control::{self, Kind};
use rh_protocol::records::{self, RecordsFrame};
use rh_protocol::wire::{self, Frame, FrameType, MAX_PAYLOAD};
use serde_json::{Value, json};

pub mod viewer;

pub const BIN: &str = env!("CARGO_BIN_EXE_rh-cli");
pub const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A fresh run directory under a short path this test creates (the 107-byte socket limit never
/// depends on the runner's working directory). Returns the parent to create it in; the run
/// directory itself does not exist yet.
pub fn scratch(name: &str) -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    // `RH_TEST_SCRATCH` moves the scratch directories (e.g. to /dev/shm, a disk whose fsync is as
    // fast as CI's, to reproduce its timings); short, for the 107-byte socket path limit.
    let root = std::env::var("RH_TEST_SCRATCH").unwrap_or_else(|_| "/tmp".to_owned());
    let base = PathBuf::from(format!("{root}/rhl{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    base.join(name)
}

/// The channel-A socket of a run directory.
pub fn a_sock(dir: &Path) -> PathBuf {
    dir.join("stream/t0000000000/A.sock")
}

/// The channel-C socket of a run directory.
pub fn c_sock(dir: &Path) -> PathBuf {
    dir.join("stream/t0000000000/C.sock")
}

/// A host child process, its stdout lines collected on a thread.
pub struct Host {
    pub child: Child,
    pub lines: mpsc::Receiver<String>,
    pub seen: Vec<String>,
}

impl Host {
    /// Starts `rh-cli <args…>`.
    pub fn start(args: &[&str]) -> Self {
        let mut child = Command::new(BIN)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let out = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for l in BufReader::new(out).lines() {
                let Ok(l) = l else { break };
                if tx.send(l).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            lines: rx,
            seen: Vec::new(),
        }
    }

    /// Waits for a stdout line starting with `prefix` (60 s).
    pub fn wait_line(&mut self, prefix: &str) -> String {
        let deadline = Instant::now() + Duration::from_mins(1);
        while Instant::now() < deadline {
            if let Ok(l) = self.lines.recv_timeout(Duration::from_millis(100)) {
                self.seen.push(l.clone());
                if l.starts_with(prefix) || l.contains(prefix) {
                    return l;
                }
            }
        }
        panic!("no line {prefix:?}; saw {:?}", self.seen);
    }

    /// Waits for the host to exit (120 s); its exit code and every stdout line.
    pub fn finish(mut self) -> (i32, Vec<String>, String) {
        let deadline = Instant::now() + Duration::from_mins(2);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                while let Ok(l) = self.lines.recv_timeout(Duration::from_millis(200)) {
                    self.seen.push(l);
                }
                let mut err = String::new();
                if let Some(mut e) = self.child.stderr.take() {
                    let _ = e.read_to_string(&mut err);
                }
                return (status.code().unwrap_or(-1), self.seen, err);
            }
            assert!(Instant::now() < deadline, "the host did not exit");
            while let Ok(l) = self.lines.try_recv() {
                self.seen.push(l);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// SIGKILL.
    pub fn kill(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Waits for a socket file to appear (60 s).
pub fn wait_socket(path: &Path) {
    let deadline = Instant::now() + Duration::from_mins(1);
    while !path.exists() {
        assert!(Instant::now() < deadline, "no socket at {}", path.display());
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// One frame from the host.
#[derive(Debug, Clone)]
pub enum In {
    Records(RecordsFrame, Vec<u8>),
    Control(Kind, Value),
}

/// A raw client of one connection.
pub struct Client {
    pub s: UnixStream,
    buf: Vec<u8>,
    next_id: i64,
}

impl Client {
    pub fn connect(path: &Path) -> Self {
        wait_socket(path);
        let s = UnixStream::connect(path).unwrap();
        s.set_read_timeout(Some(Duration::from_mins(1))).unwrap();
        Self {
            s,
            buf: Vec::new(),
            next_id: 0,
        }
    }

    /// A connection now, or `None` (no socket, or no server behind it).
    pub fn try_connect(path: &Path) -> Option<Self> {
        let s = UnixStream::connect(path).ok()?;
        s.set_read_timeout(Some(Duration::from_mins(1))).ok()?;
        Some(Self {
            s,
            buf: Vec::new(),
            next_id: 0,
        })
    }

    pub fn send_raw(&mut self, bytes: &[u8]) {
        self.s.write_all(bytes).unwrap();
    }

    /// Sends a request; its id.
    pub fn request(&mut self, method: &str, params: Option<Value>) -> i64 {
        self.next_id += 1;
        let frame = control::control_frame(&control::request(&json!(self.next_id), method, params))
            .unwrap();
        self.send_raw(&frame);
        self.next_id
    }

    /// The next frame, or `None` when the host closed the connection.
    pub fn next(&mut self) -> Option<In> {
        loop {
            match wire::decode(&self.buf, MAX_PAYLOAD) {
                Ok((f, used)) => {
                    self.buf.drain(..used);
                    return Some(Self::frame(&f));
                }
                Err(e) if e.is_incomplete() => {}
                Err(e) => panic!("a bad frame from the host: {e}"),
            }
            let mut tmp = vec![0u8; 1 << 16];
            match self.s.read(&mut tmp) {
                Ok(0) => return None,
                Ok(n) => self.buf.extend_from_slice(&tmp[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => return None,
                Err(e) => panic!("read: {e}"),
            }
        }
    }

    fn frame(f: &Frame) -> In {
        match f.ftype {
            FrameType::Records => {
                let r = records::decode(&f.payload).unwrap();
                In::Records(r, f.payload[records::SUBHEADER_LEN..].to_vec())
            }
            FrameType::Control => {
                let (k, v) = control::decode_control(&f.payload).unwrap();
                In::Control(k, v)
            }
        }
    }

    /// Sends a request and returns its reply (frames before it are collected in `skipped`).
    pub fn call(&mut self, method: &str, params: Option<Value>) -> (Value, Vec<In>) {
        let id = self.request(method, params);
        let mut skipped = Vec::new();
        loop {
            match self.next() {
                Some(In::Control(k, v)) if k != Kind::Notification && v["id"] == json!(id) => {
                    return (v, skipped);
                }
                Some(other) => skipped.push(other),
                None => panic!("closed before the reply to {method}"),
            }
        }
    }
}

/// A run's ladder file.
pub fn ladder(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("run.ladder")).unwrap()
}

/// `rh-cli ladder compare`.
pub fn ladders_equal(a: &Path, b: &Path) -> bool {
    Command::new(BIN)
        .args(["ladder", "compare"])
        .arg(a)
        .arg(b)
        .status()
        .unwrap()
        .success()
}

/// The world's committed baseline ladder.
pub fn baseline(world: &str) -> PathBuf {
    Path::new(REPO).join(format!(
        "worlds/{world}/baselines/{world}_yearly_200y.ladder"
    ))
}

/// Lowercase hex.
pub fn hex(b: &[u8]) -> String {
    use std::fmt::Write as _;
    b.iter().fold(String::new(), |mut s, x| {
        let _ = write!(s, "{x:02x}");
        s
    })
}
