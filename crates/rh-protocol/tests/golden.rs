//! Conformance (`wire_v1.md` §7, §9; SPR-026 T-01): every line of `specs/stream/golden/wire_v1.txt`.
//!
//! - `frame`: the decoded form is encoded to exactly the hex, and the hex decodes back to it;
//! - `refuse`: the frame is refused with a message naming the reason;
//! - `dispatch`: the request is answered with exactly the reply bytes, on a fresh connection to a
//!   host with no record (the reference's `Host(Writer("A", []))`);
//! - `scenario catchup`: `Catchup` produces the transcript line for line;
//! - the kind table equals the kinds of `specs/stream/golden/canonical_v0.txt`, by channel.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::format_collect,
    clippy::items_after_statements,
    reason = "test helpers fail loudly; generators truncate on purpose"
)]

use std::path::PathBuf;

use rh_protocol::control::{self, Connection, History, HostView, Kind, Source};
use rh_protocol::cursor::{Action, Catchup, Event};
use rh_protocol::records::{self, Channel, KIND_CHANNEL, RecordsFrame};
use rh_protocol::wire::{self, Frame, FrameType, MAX_PAYLOAD};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn golden() -> Vec<String> {
    let path = repo().join("specs/stream/golden/wire_v1.txt");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .lines()
        .map(str::to_owned)
        .collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or_else(|e| panic!("hex {s}: {e}")))
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The canonical form's string token (`canonical_v0.md` §2): `%` alone is the empty string, and
/// every byte outside the safe set is `%XX`.
fn decode_token(t: &str) -> String {
    if t == "%" {
        return String::new();
    }
    let b = t.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            out.push(u8::from_str_radix(&t[i + 1..i + 3], 16).unwrap_or(b'?'));
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

fn encode_token(s: &str) -> String {
    if s.is_empty() {
        return "%".to_owned();
    }
    let safe = |c: u8| c.is_ascii_alphanumeric() || b"._-/:+@".contains(&c);
    s.bytes()
        .map(|c| {
            if safe(c) {
                char::from(c).to_string()
            } else {
                format!("%{c:02X}")
            }
        })
        .collect()
}

/// The reference's empty channel-A host (`Host(Writer("A", []))`).
struct EmptyHost;

impl Source for EmptyHost {
    fn view(&self) -> HostView {
        HostView {
            channel: Channel::A,
            timeline_id: 0,
            run_nonce: 1,
            record_format: "rehoboam/stream/channel-a/v0".to_owned(),
            history: History {
                engine_build: "reference".to_owned(),
                registry_version: "0.0.0".to_owned(),
                manifest_hash: "00".repeat(16),
                world_seed: 0,
            },
            minutes_per_year: 518_400,
            tip_tick: None,
            next_seq: 0,
            live_first_seq: 0,
            sealed_last_year: None,
            sealed_last_seq: None,
            pace: Some(("max".to_owned(), "running".to_owned())),
            sifted_through: None,
            finished: false,
            seal_log_len: 0,
        }
    }
}

fn expected_kind(name: &str) -> Kind {
    if name.starts_with("error") {
        Kind::Error
    } else if name.starts_with("notify") {
        Kind::Notification
    } else if name.contains("result") {
        Kind::Response
    } else {
        Kind::Request
    }
}

#[test]
fn every_golden_frame_encodes_to_its_bytes_and_decodes_back() {
    let lines = golden();
    let mut n = 0;
    let mut i = 0;
    while i < lines.len() {
        let parts: Vec<&str> = lines[i].split(' ').collect();
        if parts[0] != "frame" {
            i += 1;
            continue;
        }
        let (name, bytes) = (parts[1], unhex(parts[2]));
        let desc = &lines[i + 1];
        let (decoded, used) =
            wire::decode(&bytes, MAX_PAYLOAD).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(used, bytes.len(), "{name}: bytes used");
        if let Some(json) = desc.strip_prefix("control ") {
            let frame = Frame {
                ftype: FrameType::Control,
                payload: json.as_bytes().to_vec(),
            };
            assert_eq!(
                hex(&wire::encode(&frame).unwrap_or_default()),
                parts[2],
                "{name}: encode"
            );
            assert_eq!(decoded, frame, "{name}: decode");
            let (kind, value) =
                control::decode_control(json.as_bytes()).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(kind, expected_kind(name), "{name}: class");
            assert_eq!(
                rh_protocol::json::canonical(&value).unwrap_or_default(),
                json.as_bytes(),
                "{name}: not canonical after a round trip"
            );
            if name.starts_with("status_result") {
                control::check_status(&value["result"]).unwrap_or_else(|e| panic!("{name}: {e}"));
            }
            i += 2;
        } else {
            let head: Vec<&str> = desc.split(' ').collect();
            assert_eq!(head[0], "records", "{name}");
            let count: usize = head[4].parse().unwrap_or_default();
            let recs = RecordsFrame {
                timeline_id: head[1].parse().unwrap_or_default(),
                channel: Channel::from_byte(head[2].as_bytes()[0]).unwrap_or(Channel::A),
                first_seq: head[3].parse().unwrap_or_default(),
                lines: lines[i + 2..i + 2 + count]
                    .iter()
                    .map(|l| l.strip_prefix("line ").unwrap_or_default().to_owned())
                    .collect(),
            };
            let payload = records::encode(&recs).unwrap_or_else(|e| panic!("{name}: {e}"));
            let frame = Frame {
                ftype: FrameType::Records,
                payload,
            };
            assert_eq!(
                hex(&wire::encode(&frame).unwrap_or_default()),
                parts[2],
                "{name}: encode"
            );
            assert_eq!(decoded, frame, "{name}: decode");
            assert_eq!(
                records::decode(&decoded.payload),
                Ok(recs),
                "{name}: records"
            );
            i += 2 + count;
        }
        n += 1;
    }
    assert_eq!(n, 26, "the golden file's frame lines");
    println!("wire_v1 conformance: {n} golden frames encoded byte for byte and decoded back");
}

fn decode_any(data: &[u8]) -> Result<(), wire::WireError> {
    let (frame, _) = wire::decode(data, MAX_PAYLOAD)?;
    match frame.ftype {
        FrameType::Records => records::decode(&frame.payload).map(drop),
        FrameType::Control => control::decode_control(&frame.payload).map(drop),
    }
}

#[test]
fn every_refuse_line_is_refused_naming_its_reason() {
    let mut n = 0;
    for line in golden().iter().filter(|l| l.starts_with("refuse ")) {
        let parts: Vec<&str> = line.split(' ').collect();
        let (name, data, reason) = (parts[1], unhex(parts[2]), decode_token(parts[3]));
        match decode_any(&data) {
            Ok(()) => panic!("refuse {name}: accepted"),
            Err(e) => assert!(
                e.message.contains(&reason),
                "refuse {name}: refused for {:?}, expected {reason:?}",
                e.message
            ),
        }
        n += 1;
    }
    assert_eq!(n, 28, "the golden file's refuse lines");
    println!("wire_v1 conformance: {n} golden refusals refused by name");
}

#[test]
fn every_dispatch_line_is_answered_byte_for_byte_on_a_fresh_connection() {
    let (mut n, mut closed) = (0, 0);
    for line in golden().iter().filter(|l| l.starts_with("dispatch ")) {
        let parts: Vec<&str> = line.split(' ').collect();
        let (name, request, want) = (parts[1], unhex(parts[2]), parts[3]);
        let mut conn = Connection::default();
        let reply = control::dispatch(&mut conn, &EmptyHost, &request);
        assert_eq!(hex(&reply.frames.concat()), want, "dispatch {name}");
        assert_eq!(reply.frames.len(), 1, "dispatch {name}: one reply frame");
        // C16: the connection closes after a header or frame refusal and after every answer whose
        // id is null; an answer to a request with an id leaves it open (selftest_wire_v1.py's
        // EXPECTED_CLOSED, one case per class).
        let want_close = matches!(
            name,
            "dispatch_subscribe_float"
                | "dispatch_notification"
                | "dispatch_batch"
                | "dispatch_not_json"
                | "dispatch_version_2"
                | "dispatch_records_from_viewer"
                | "dispatch_oversize"
        );
        assert_eq!(
            reply.close, want_close,
            "dispatch {name}: closes the connection"
        );
        n += 1;
        closed += usize::from(want_close);
    }
    assert_eq!(n, 15, "the golden file's dispatch lines");
    println!(
        "wire_v1 conformance: {n} dispatch answers byte for byte on a fresh connection; {closed} \
         close it (C16), {} keep it open",
        n - closed
    );
}

fn event(line: &str) -> Event {
    let p: Vec<&str> = line.split(' ').collect();
    let num = |i: usize| -> u64 { p[i].parse().unwrap_or_else(|e| panic!("{line}: {e}")) };
    let reason = decode_token(p[5]);
    match p[0] {
        "subscribed" => Event::Subscribed(num(1)),
        "behind" => Event::Behind {
            next_seq: num(1),
            s: Some(num(3)),
            w0: num(4),
        },
        "records" => Event::Records {
            first_seq: num(1),
            count: num(2),
        },
        "sealed" => Event::Sealed,
        "end" => Event::End(
            control::EndReason::from_name(&reason).unwrap_or(control::EndReason::Aborted),
        ),
        "closed" => Event::Closed,
        "read_sealed_done" => Event::SealedRead {
            first_seq: num(1),
            count: num(2),
        },
        "refused" => Event::Refused {
            reason,
            finished: p[7] == "1",
        },
        "retry_elapsed" => Event::RetryElapsed,
        "status" => Event::Status {
            same_run: p[6] == "1",
        },
        "jump" => Event::Jump,
        "unsubscribed" => Event::Unsubscribed(num(1)),
        other => panic!("unknown golden event {other}"),
    }
}

fn action_line(a: &Action) -> String {
    let (kind, x, y, detail) = a.golden();
    format!("action {kind} {x} {y} {}", encode_token(&detail))
}

#[test]
fn catchup_reproduces_the_golden_transcript_line_for_line() {
    let lines = golden();
    let start = lines
        .iter()
        .position(|l| l == "scenario catchup")
        .unwrap_or_else(|| panic!("no scenario catchup"));
    let transcript = &lines[start + 1..];
    let mut loop_ = Catchup::new(0);
    let mut got = vec![action_line(&loop_.start())];
    for l in transcript.iter().filter_map(|l| l.strip_prefix("event ")) {
        got.push(format!("event {l}"));
        got.push(action_line(&loop_.next(event(l))));
    }
    assert_eq!(got.len(), transcript.len(), "transcript length");
    for (g, w) in got.iter().zip(transcript) {
        assert_eq!(g, w, "Catchup transcript");
    }
    // R15's four orderings are in it (the reference's checks).
    for needle in [
        "action ignore 0 0 reconnect%20pending",
        "action ignore 0 0 a%20stale%20retry%20timer",
        "action ignore 0 0 behind%20during%20a%20jump",
        "event records 404 2 0 0 % 1 0",
    ] {
        assert!(transcript.iter().any(|l| l == needle), "missing {needle}");
    }
    println!(
        "wire_v1 conformance: the Catchup transcript reproduced line for line ({} lines)",
        transcript.len()
    );
    // R21: future_seq on a finished run stops the viewer.
    let mut fin = Catchup::new(0);
    fin.start();
    assert!(matches!(
        fin.next(Event::Refused {
            reason: "future_seq".to_owned(),
            finished: true
        }),
        Action::Stop(_)
    ));
}

#[test]
fn the_kind_table_is_the_canonical_golden_files() {
    let path = repo().join("specs/stream/golden/canonical_v0.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lines: Vec<&str> = text.lines().collect();
    let mut seen = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < lines.len() {
        let p: Vec<&str> = lines[i].split(' ').collect();
        let (channel, n) = match p[0] {
            "segment" => (
                Channel::from_byte(p[2].as_bytes()[0]),
                p[4].parse().unwrap_or(0),
            ),
            "lines" => (None, p[1].parse().unwrap_or(0)),
            _ => {
                i += 1;
                continue;
            }
        };
        for l in &lines[i + 1..i + 1 + n] {
            let kind = l.split(' ').next().unwrap_or_default();
            let ch = records::kind_channel(kind);
            assert!(ch.is_some(), "kind {kind} is not in the table");
            if let Some(want) = channel {
                assert_eq!(ch, Some(want), "kind {kind} of a channel-{want:?} segment");
            }
            assert_eq!(ch == Some(Channel::C), kind == "MATCH", "kind {kind}");
            seen.insert(kind.to_owned());
        }
        i += 1 + n;
    }
    let table: std::collections::BTreeSet<String> =
        KIND_CHANNEL.iter().map(|(k, _)| (*k).to_owned()).collect();
    assert_eq!(
        seen, table,
        "every kind of the table appears in the canonical golden file"
    );
}

fn golden_control() -> Vec<String> {
    let path = repo().join("specs/stream/golden/control_v1.txt");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_control_golden_frame_encodes_to_its_bytes_and_decodes_back() {
    let lines = golden_control();
    let mut i = 0;
    let mut n = 0;
    while i < lines.len() {
        if !lines[i].starts_with("frame ") {
            i += 1;
            continue;
        }
        let parts: Vec<&str> = lines[i].split(' ').collect();
        let (name, data) = (parts[1], unhex(parts[2]));
        let (decoded, used) =
            wire::decode(&data, MAX_PAYLOAD).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(used, data.len(), "{name}: unused bytes");
        assert_eq!(decoded.ftype, FrameType::Control, "{name}: not CONTROL");
        let next = lines[i + 1]
            .strip_prefix("control ")
            .unwrap_or_else(|| panic!("{name}: no control line"));
        assert_eq!(
            hex(&wire::encode(&decoded).unwrap_or_default()),
            parts[2],
            "{name}: encode"
        );
        let (_kind, value) =
            control::decode_control(next.as_bytes()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            rh_protocol::json::canonical(&value).unwrap_or_default(),
            next.as_bytes(),
            "{name}: not canonical"
        );
        i += 2;
        n += 1;
    }
    assert_eq!(n, 18, "the control golden file's frame lines");
    println!("control_v1 conformance: {n} golden frames encoded byte for byte and decoded back");
}

struct SimulatedControlHost {
    tip: Option<i64>,
    next_tick: Option<i64>,
    next_seq: u64,
    band: std::cell::RefCell<String>,
    state: std::cell::RefCell<String>,
    step_target: std::cell::RefCell<Option<i64>>,
    end: Option<String>,
}

impl SimulatedControlHost {
    fn new(years: usize, finished: bool, with_step: bool) -> Self {
        const INSTANTS_PER_YEAR: usize = 12;
        const INSTANT_MINUTES: i64 = 43_200;
        const FIRST_TICK: i64 = 0;
        const HORIZON_YEARS: usize = 4;
        let instants = if finished {
            HORIZON_YEARS * INSTANTS_PER_YEAR
        } else {
            years * INSTANTS_PER_YEAR
        };
        let tip = if instants == 0 {
            None
        } else {
            Some(FIRST_TICK + (instants as i64 - 1) * INSTANT_MINUTES)
        };
        let next_tick = if finished {
            None
        } else {
            Some(FIRST_TICK + (instants as i64) * INSTANT_MINUTES)
        };
        let keyframes = instants / (2 * INSTANTS_PER_YEAR);
        let next_seq = if instants == 0 {
            0
        } else {
            (instants * 3 + (instants / INSTANTS_PER_YEAR) + keyframes) as u64
        };
        let (band, state, step_target, end) = if finished {
            ("max", "finished", None, Some("finished".to_owned()))
        } else if with_step {
            let last_instant_tick =
                FIRST_TICK + ((HORIZON_YEARS * INSTANTS_PER_YEAR - 1) as i64) * INSTANT_MINUTES;
            ("max", "running", Some(last_instant_tick), None)
        } else {
            ("max", "running", None, None)
        };
        Self {
            tip,
            next_tick,
            next_seq,
            band: std::cell::RefCell::new(band.to_owned()),
            state: std::cell::RefCell::new(state.to_owned()),
            step_target: std::cell::RefCell::new(step_target),
            end,
        }
    }

    fn pace(&self) -> serde_json::Value {
        let step = self
            .step_target
            .borrow()
            .map(|t| serde_json::json!({"until_tick": t}));
        serde_json::json!({
            "band": *self.band.borrow(),
            "next_seq": self.next_seq,
            "state": *self.state.borrow(),
            "step": step,
            "tick": self.tip,
        })
    }

    fn status(&self) -> serde_json::Value {
        let end = self.end.as_ref().map(|r| serde_json::json!({"reason": r}));
        serde_json::json!({
            "calendar": {"minutes_per_year": 518_400},
            "control": {"format": control::CONTROL_FORMAT, "version": control::CONTROL_VERSION},
            "controllers": 1,
            "end": end,
            "history": {
                "engine_build": "reference",
                "manifest_hash": "00".repeat(16),
                "registry_version": "0.0.0",
                "world_seed": 0,
            },
            "pace": self.pace(),
            "run_nonce": 1,
            "timeline_id": 0,
            "tip": {"next_seq": self.next_seq, "tick": self.tip},
        })
    }
}

impl control::ControlHost for SimulatedControlHost {
    fn submit(
        &self,
        _conn: &control::ControlConn,
        id: &serde_json::Value,
        cmd: control::ControlCommand,
    ) -> serde_json::Value {
        if cmd == control::ControlCommand::Status {
            return control::response(&Some(id.clone()), self.status());
        }
        if self.state.borrow().as_str() == "finished" {
            let mut d = serde_json::Map::new();
            if let Some(ref end) = self.end {
                d.insert("end".into(), serde_json::json!(end));
            }
            return control::error(&Some(id.clone()), control::Reason::RunFinished, d);
        }
        match cmd {
            control::ControlCommand::Status => unreachable!(),
            control::ControlCommand::Pace { band } => {
                let changed = band != *self.band.borrow();
                if changed {
                    *self.band.borrow_mut() = band;
                }
                let mut res = self.pace();
                res.as_object_mut()
                    .unwrap()
                    .insert("changed".into(), serde_json::json!(changed));
                control::response(&Some(id.clone()), res)
            }
            control::ControlCommand::Pause => {
                let changed = *self.state.borrow() == "running";
                if changed {
                    *self.state.borrow_mut() = "paused".to_owned();
                    *self.step_target.borrow_mut() = None;
                }
                let mut res = self.pace();
                res.as_object_mut()
                    .unwrap()
                    .insert("changed".into(), serde_json::json!(changed));
                control::response(&Some(id.clone()), res)
            }
            control::ControlCommand::Resume => {
                let changed =
                    *self.state.borrow() == "paused" || self.step_target.borrow().is_some();
                if changed {
                    *self.state.borrow_mut() = "running".to_owned();
                    *self.step_target.borrow_mut() = None;
                }
                let mut res = self.pace();
                res.as_object_mut()
                    .unwrap()
                    .insert("changed".into(), serde_json::json!(changed));
                control::response(&Some(id.clone()), res)
            }
            control::ControlCommand::Step { until } => {
                let Some(nxt) = self.next_tick else {
                    let mut d = serde_json::Map::new();
                    if let Some(ref end) = self.end {
                        d.insert("end".into(), serde_json::json!(end));
                    }
                    return control::error(&Some(id.clone()), control::Reason::RunFinished, d);
                };
                let target = match until {
                    control::StepUntil::Instant => nxt,
                    control::StepUntil::Tick(t) => t,
                };
                if (self.tip.is_none() && target < 0)
                    || (self.tip.is_some() && target <= self.tip.unwrap())
                {
                    let mut d = serde_json::Map::new();
                    d.insert("tip_tick".into(), serde_json::json!(self.tip));
                    return control::error(&Some(id.clone()), control::Reason::PastTick, d);
                }
                let changed =
                    *self.step_target.borrow() != Some(target) || *self.state.borrow() != "running";
                if changed {
                    *self.state.borrow_mut() = "running".to_owned();
                    *self.step_target.borrow_mut() = Some(target);
                    if nxt > target {
                        *self.state.borrow_mut() = "paused".to_owned();
                        *self.step_target.borrow_mut() = None;
                    }
                }
                let mut res = self.pace();
                res.as_object_mut()
                    .unwrap()
                    .insert("changed".into(), serde_json::json!(changed));
                control::response(&Some(id.clone()), res)
            }
        }
    }
}

#[test]
fn every_control_dispatch_line_is_answered_byte_for_byte() {
    let (mut n, mut closed) = (0, 0);
    for line in golden_control()
        .iter()
        .filter(|l| l.starts_with("dispatch "))
    {
        let parts: Vec<&str> = line.split(' ').collect();
        let (name, request, want) = (parts[1], unhex(parts[2]), parts[3]);
        let finished = name.ends_with("_finished");
        let with_step = name.contains("_with_step") || name == "dispatch_step_replace";
        let years = if name == "dispatch_step_genesis" || name == "dispatch_step_before_first" {
            0
        } else {
            1
        };
        let host = SimulatedControlHost::new(years, finished, with_step);
        let mut conn = control::ControlConn::default();
        let reply = control::dispatch_control(&mut conn, &host, &request);
        assert_eq!(hex(&reply.frames.concat()), want, "dispatch {name}");
        assert_eq!(reply.frames.len(), 1, "dispatch {name}: one reply frame");
        let want_close = matches!(
            name,
            "dispatch_version_2"
                | "dispatch_records_frame"
                | "dispatch_oversize"
                | "dispatch_not_json"
                | "dispatch_batch"
                | "dispatch_notification"
                | "dispatch_pace_float"
        );
        assert_eq!(conn.closed, want_close, "dispatch {name}: conn.closed");
        assert_eq!(reply.close, want_close, "dispatch {name}: reply.close");
        if conn.closed {
            closed += 1;
        }
        n += 1;
    }
    assert_eq!(n, 41, "the control golden file's dispatch lines");
    assert_eq!(closed, 7, "closed connections");
    println!(
        "control_v1 conformance: {n} golden dispatch lines answered byte for byte ({closed} closed)"
    );
}
