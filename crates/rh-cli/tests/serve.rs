//! The host of `rh-cli run … --socket` (SPR-026 T-03; `specs/stream/wire_v1.md` §1, §4, §9).
//!
//! - **Read-only:** `player.act`, `timeline.fork`, `timeline.run`, `oracle.whisper` and
//!   `stream.pace` on `A.sock` are each `method_not_found` with the read-only message; the run's
//!   ladder equals the committed baseline and `rh-cli replay` of its directory reports
//!   `REPLAY EQUAL` (the journal's rows and chain head are run.toml's); modes `0700` and `0600`.
//! - **The fuzz** (V-IF-02 style): every golden `refuse` and `dispatch` frame, the corrupt frames
//!   of §9 and seeded byte mutations, each on its own connection: the named error, then the host
//!   closes; the run continues to its end and its ladder is the baseline.
//! - **Locking:** a held `A.lock` refuses a second host before it writes anything; a killed
//!   host's stale socket is removed by the next.
//! - **The cap:** a connection over `--max-viewers` gets `too_many_viewers`, then a close.
//! - **Abort:** a poisoned engine ends the subscription with `stream.end{aborted}`, and the year
//!   it was in stays unsealed (BCK-002 A-42).
//! - **The path limit:** a socket path over 107 bytes is refused, naming it, with nothing written.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::many_single_char_names,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a tool-crate test that runs rh-cli as a child process with a deadline clock; helpers \
              fail loudly"
)]

mod support;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use rh_protocol::control::{self, Kind, READ_ONLY_MESSAGE};
use serde_json::{Value, json};
use support::{BIN, Client, Host, In, a_sock, baseline, ladders_equal, scratch};

fn host(world: &str, dir: &Path, extra: &[&str]) -> Host {
    let d = dir.display().to_string();
    let mut args = vec!["run", world, "--years", "200", "--run-dir", &d, "--socket"];
    args.extend_from_slice(extra);
    Host::start(&args)
}

/// Reads a subscription from seq 0 to its end: (records received, the end's reason).
fn drain(c: &mut Client) -> (u64, String) {
    drain_from(c, &json!({"seq": 0}))
}

fn drain_from(c: &mut Client, from: &Value) -> (u64, String) {
    let (reply, _) = c.call("stream.subscribe", Some(json!({ "from": from })));
    let mut next = reply["result"]["first_seq"]
        .as_u64()
        .unwrap_or_else(|| panic!("{reply}"));
    if from.is_object() {
        assert_eq!(json!(next), from["seq"], "{reply}");
    }
    loop {
        match c.next().expect("the host closed before stream.end") {
            In::Records(r, _) => {
                assert_eq!(r.first_seq, next, "contiguous");
                next += u64::try_from(r.lines.len()).unwrap();
            }
            In::Control(Kind::Notification, v) if v["method"] == "stream.end" => {
                return (next, v["params"]["reason"].as_str().unwrap().to_owned());
            }
            In::Control(Kind::Notification, v) if v["method"] == "stream.behind" => {
                panic!("a reader at the edge was sent behind: {v}");
            }
            In::Control(..) => {}
        }
    }
}

fn replay_equal(dir: &Path) -> String {
    let into = dir.with_extension("replay");
    let out = Command::new(BIN)
        .args(["replay"])
        .arg(dir)
        .arg("--into")
        .arg(&into)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn the_socket_refuses_every_attempt_to_act_and_the_run_is_unchanged() {
    let dir = scratch("readonly");
    let h = host("solo", &dir, &["--linger", "30"]);
    let mut c = Client::connect(&a_sock(&dir));
    let (status, _) = c.call("stream.status", None);
    let st = control::check_status(&status["result"]).unwrap();
    assert_eq!(status["result"]["channel"], "A");
    assert_eq!(status["result"]["pace"]["band"], "max");
    assert!(st.run_nonce > 1, "run_nonce is the wall clock");
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&dir), 0o700, "the run directory");
    assert_eq!(mode(&a_sock(&dir)), 0o600, "the socket");
    for method in [
        "player.act",
        "timeline.fork",
        "timeline.run",
        "oracle.whisper",
        "stream.pace",
    ] {
        let (reply, _) = c.call(method, Some(json!({"verb_id": 3})));
        assert_eq!(
            control::reply_reason(&reply),
            Some("method_not_found"),
            "{method}"
        );
        assert_eq!(reply["error"]["message"], READ_ONLY_MESSAGE, "{method}");
    }
    let (records, reason) = drain(&mut c);
    assert_eq!(reason, "finished");
    assert!(records > 1000, "{records} records");
    drop(c);
    let (code, lines, err) = h.finish();
    assert_eq!(code, 0, "{lines:?} {err}");
    assert!(ladders_equal(&dir.join("run.ladder"), &baseline("solo")));
    assert!(
        !a_sock(&dir).exists(),
        "the socket is removed at a normal exit"
    );
    let replay = replay_equal(&dir);
    assert!(replay.contains("REPLAY EQUAL"), "{replay}");
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn frame(ftype: u8, flags: u8, version: u16, payload: &[u8]) -> Vec<u8> {
    let mut f = b"RHWS".to_vec();
    f.extend_from_slice(&version.to_le_bytes());
    f.push(ftype);
    f.push(flags);
    f.extend_from_slice(&u32::try_from(payload.len()).unwrap().to_le_bytes());
    f.extend_from_slice(payload);
    f
}

/// The corrupt frames of §9 and the golden file, with the reason the host must name.
fn corrupt_frames() -> Vec<(String, Vec<u8>, &'static str)> {
    let status = br#"{"id":1,"jsonrpc":"2.0","method":"stream.status"}"#;
    let mut out: Vec<(String, Vec<u8>, &'static str)> = vec![
        ("wrong magic".into(), [b"RHWX".as_slice(), &frame(1, 0, 1, status)[4..]].concat(), "invalid_frame"),
        ("version 2".into(), frame(1, 0, 2, status), "unsupported_version"),
        ("type 3".into(), frame(3, 0, 1, status), "invalid_frame"),
        ("flag 1".into(), frame(1, 1, 1, status), "invalid_frame"),
        ("over 4 KiB".into(), frame(1, 0, 1, &vec![b' '; 4097]), "invalid_frame"),
        ("records from a viewer".into(), frame(2, 0, 1, &[0; 24]), "invalid_frame"),
        (
            "a bad prefix".into(),
            frame(2, 0, 1, &[[0u8; 12].as_slice(), &[1, 0, 0, 0, b'A', 0, 0, 0], b"BOGUS 0 0 0\n"].concat()),
            "invalid_frame",
        ),
        ("a JSON batch".into(), frame(1, 0, 1, b"[]"), "invalid_request"),
        (
            "a float seq".into(),
            frame(1, 0, 1, br#"{"id":7,"jsonrpc":"2.0","method":"stream.subscribe","params":{"from":{"seq":1.5}}}"#),
            "invalid_params",
        ),
        (
            "depth 5".into(),
            frame(1, 0, 1, br#"{"id":1,"jsonrpc":"2.0","method":"m","params":{"a":{"a":{"a":{"a":1}}}}}"#),
            "invalid_request",
        ),
        (
            "an unknown member".into(),
            frame(1, 0, 1, br#"{"id":1,"jsonrpc":"2.0","method":"stream.status","x":1}"#),
            "invalid_request",
        ),
        ("100 000 nested arrays".into(), frame(1, 0, 1, &vec![b'['; 100_000]), "invalid_frame"),
        ("not JSON".into(), frame(1, 0, 1, b"{"), "parse_error"),
    ];
    let golden =
        std::fs::read_to_string(Path::new(support::REPO).join("specs/stream/golden/wire_v1.txt"))
            .unwrap();
    for line in golden.lines() {
        let p: Vec<&str> = line.split(' ').collect();
        match p[0] {
            "refuse" => out.push((format!("golden refuse {}", p[1]), unhex(p[2]), "")),
            "dispatch"
                if p[1] != "dispatch_status"
                    && p[1] != "dispatch_subscribe_future"
                    && p[1] != "dispatch_unsubscribe_none"
                    && !p[1].starts_with("dispatch_player")
                    && !p[1].starts_with("dispatch_timeline")
                    && p[1] != "dispatch_stream_pace"
                    && p[1] != "dispatch_status_with_param"
                    && p[1] != "dispatch_subscribe_filter" =>
            {
                out.push((format!("golden dispatch {}", p[1]), unhex(p[2]), ""));
            }
            _ => {}
        }
    }
    out
}

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
}

/// The reply to `bytes` on a fresh connection, and whether the host then closed it.
fn one_shot(sock: &Path, bytes: &[u8]) -> (Option<Value>, bool) {
    let mut c = Client::connect(sock);
    c.send_raw(bytes);
    let reply = match c.next() {
        Some(In::Control(_, v)) => Some(v),
        Some(In::Records(..)) => panic!("records to an unsubscribed connection"),
        None => None,
    };
    c.s.set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let closed = reply.as_ref().is_some_and(|v| v["id"].is_null()) && c.next().is_none();
    (reply, closed)
}

#[test]
fn corrupt_frames_get_their_named_error_and_a_close_and_the_run_goes_on() {
    let dir = scratch("fuzz");
    let h = host("solo", &dir, &["--linger", "60"]);
    let sock = a_sock(&dir);
    support::wait_socket(&sock);
    let mut cases = 0;
    for (name, bytes, want) in corrupt_frames() {
        let (reply, closed) = one_shot(&sock, &bytes);
        let reply = reply.unwrap_or_else(|| panic!("{name}: no reply"));
        let reason = control::reply_reason(&reply).unwrap_or("ok").to_owned();
        if !want.is_empty() {
            assert_eq!(reason, want, "{name}: {reply}");
        }
        assert!(
            reply["id"].is_null(),
            "{name}: an unidentified frame is answered with id null"
        );
        assert!(closed, "{name}: the host closes the connection");
        cases += 1;
    }
    // A truncated frame: the host waits for the rest; the viewer gives up and closes.
    {
        let mut c = Client::connect(&sock);
        let f = frame(
            1,
            0,
            1,
            br#"{"id":1,"jsonrpc":"2.0","method":"stream.status"}"#,
        );
        c.send_raw(&f[..f.len() - 5]);
        c.s.shutdown(std::net::Shutdown::Write).unwrap();
        assert!(
            c.next().is_none(),
            "a truncated frame gets no answer, then the close"
        );
    }
    // Seeded mutations of a status request: any answer, never a hang or a crash.
    let base = frame(
        1,
        0,
        1,
        br#"{"id":1,"jsonrpc":"2.0","method":"stream.status"}"#,
    );
    let mut g = SplitMix64(0x5eed);
    for _ in 0..200 {
        let mut b = base.clone();
        for _ in 0..=(g.next_u64() % 4) {
            let i = usize::try_from(g.next_u64() % u64::try_from(b.len()).unwrap()).unwrap();
            b[i] = u8::try_from(g.next_u64() & 0xff).unwrap();
        }
        // A length grown past the bytes sent is a frame still arriving: close our side.
        let mut c = Client::connect(&sock);
        c.send_raw(&b);
        c.s.shutdown(std::net::Shutdown::Write).unwrap();
        c.s.set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        while c.next().is_some() {}
        cases += 1;
    }
    assert!(cases >= 240, "{cases} cases");
    // The host is alive and serves: a reader follows the edge from live to the run's end.
    let mut c = Client::connect(&sock);
    let (_, reason) = drain_from(&mut c, &json!("live"));
    assert_eq!(reason, "finished");
    drop(c);
    let (code, lines, err) = h.finish();
    assert_eq!(code, 0, "{lines:?} {err}");
    assert!(!err.contains("panicked"), "{err}");
    assert!(ladders_equal(&dir.join("run.ladder"), &baseline("solo")));
}

#[test]
fn the_connection_cap_refuses_one_more_viewer() {
    let dir = scratch("cap");
    let h = host("solo", &dir, &["--linger", "30", "--max-viewers", "2"]);
    let sock = a_sock(&dir);
    let mut a = Client::connect(&sock);
    let mut b = Client::connect(&sock);
    let _ = a.call("stream.status", None);
    let _ = b.call("stream.status", None);
    let mut c = Client::connect(&sock);
    match c.next() {
        Some(In::Control(_, v)) => {
            assert_eq!(control::reply_reason(&v), Some("too_many_viewers"), "{v}");
            assert_eq!(v["error"]["data"]["limit"], 2);
            assert!(v["id"].is_null());
        }
        other => panic!("{other:?}"),
    }
    assert!(c.next().is_none(), "then the host closes");
    drop((a, b, c));
    let (code, _, _) = h.finish();
    assert_eq!(code, 0);
}

#[test]
fn a_poisoned_engine_ends_every_subscription_aborted_and_seals_no_partial_year() {
    let dir = scratch("abort");
    // Sim-year 30 of the 360-day calendar: the poison at its sixth month.
    let tick = (30 * 518_400 + 5 * 43_200).to_string();
    let h = host("solo", &dir, &["--linger", "30", "--test-poison", &tick]);
    let mut c = Client::connect(&a_sock(&dir));
    let (_, reason) = drain(&mut c);
    assert_eq!(reason, "aborted");
    drop(c);
    let (code, lines, _) = h.finish();
    assert_eq!(code, 1, "the simulation failed: {lines:?}");
    let a = dir.join("stream/t0000000000/A");
    assert!(a.join("y000029").is_dir(), "the completed years are sealed");
    assert!(
        !a.join("y000030").exists(),
        "the poisoned year stays unsealed"
    );
}

#[test]
fn a_held_lock_refuses_a_second_server_and_a_stale_socket_is_removed() {
    let dir = scratch("lock");
    // A killed host: its run directory, its stale socket.
    let mut h = host(
        "solo",
        &dir,
        &["--test-stall", &format!("mid:{}", 15 * 518_400 + 43_200)],
    );
    h.wait_line("STALL");
    assert!(a_sock(&dir).exists());
    // The host thread is stalled; the seal thread writes year 10's keyframe behind its segment.
    let kf_path = dir.join("keyframes/year-0010.rhkf");
    let t0 = std::time::Instant::now();
    while !kf_path.exists() {
        assert!(t0.elapsed().as_secs() < 120, "no year-10 keyframe");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    h.kill();
    assert!(a_sock(&dir).exists(), "the killed host left its socket");
    let before = support::ladder(&dir);
    // Someone else serves this socket: the resumed host is refused before it writes anything.
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .open(dir.join("stream/t0000000000/A.lock"))
        .unwrap();
    lock.lock().unwrap();
    let d = dir.display().to_string();
    let kf = dir.join("keyframes/year-0010.rhkf").display().to_string();
    let resume = [
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
        "1",
    ];
    let refused = Host::start(&resume);
    let (code, _, err) = refused.finish();
    assert_eq!(code, 2, "{err}");
    assert!(
        err.contains("A.lock") && err.contains("another server"),
        "{err}"
    );
    assert_eq!(
        support::ladder(&dir),
        before,
        "a refused host writes nothing"
    );
    drop(lock);
    // Released: the next host removes the stale socket, binds its own and runs to the end.
    let mut ok = Host::start(&resume);
    ok.wait_line("serving channel A live");
    let (code, lines, err) = ok.finish();
    assert_eq!(code, 0, "{lines:?} {err}");
    assert!(ladders_equal(&dir.join("run.ladder"), &baseline("solo")));
}

#[test]
fn a_socket_path_over_107_bytes_is_refused_naming_it() {
    let base = scratch("long");
    let dir = base.join("x".repeat(90));
    let d = dir.display().to_string();
    let h = Host::start(&["run", "solo", "--years", "200", "--run-dir", &d, "--socket"]);
    let (code, _, err) = h.finish();
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("A.sock") && err.contains("107"), "{err}");
    assert!(!dir.exists(), "nothing was written");
}

/// Probe (c) of §9, in the observer-invariance style: the state ladder with viewers attached (one
/// at the edge, one from seq 0, one that only asks status) is the committed baseline, the ladder
/// without, on `solo` and `hamlet`.
#[test]
fn probe_c_the_ladder_with_viewers_attached_is_the_ladder_without() {
    for world in ["solo", "hamlet"] {
        let dir = scratch(&format!("probe-c-{world}"));
        let h = host(world, &dir, &["--linger", "30"]);
        let sock = a_sock(&dir);
        let threads: Vec<_> = [json!({"seq": 0}), json!("live")]
            .into_iter()
            .map(|from| {
                let sock = sock.clone();
                std::thread::spawn(move || {
                    let mut c = Client::connect(&sock);
                    drain_from_any(&mut c, &from)
                })
            })
            .collect();
        let mut idle = Client::connect(&sock);
        let _ = idle.call("stream.status", None);
        for t in threads {
            let reason = t.join().unwrap();
            assert!(
                reason == "finished" || reason == "behind",
                "{world}: {reason}"
            );
        }
        drop(idle);
        let (code, lines, err) = h.finish();
        assert_eq!(code, 0, "{world}: {lines:?} {err}");
        assert!(
            ladders_equal(&dir.join("run.ladder"), &baseline(world)),
            "{world}: the ladder with viewers differs from the baseline"
        );
    }
}

/// Like `drain_from`, but a reader sent behind stops there ("behind"): probe (c) needs readers,
/// not the catch-up (T-05's).
fn drain_from_any(c: &mut Client, from: &Value) -> String {
    let (reply, _) = c.call("stream.subscribe", Some(json!({ "from": from })));
    if control::reply_reason(&reply).is_some() {
        return "behind".to_owned();
    }
    loop {
        match c.next() {
            Some(In::Control(Kind::Notification, v)) if v["method"] == "stream.end" => {
                return v["params"]["reason"].as_str().unwrap().to_owned();
            }
            Some(In::Control(Kind::Notification, v)) if v["method"] == "stream.behind" => {
                return "behind".to_owned();
            }
            Some(_) => {}
            None => return "closed".to_owned(),
        }
    }
}
