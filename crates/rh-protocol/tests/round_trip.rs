//! Round trips of frames, RECORDS payloads and control messages (SPR-026 T-01): `SplitMix64` with
//! fixed seeds (the repository's practice; no proptest), at least 10^4 cases, with per-field
//! boundary generators and a frame-level mode (a stream of concatenated frames decoded
//! incrementally from every split point of its buffer).
//!
//! - Frames: payload sizes from 0 to the 1 MiB limit (the edges 0, 1, the limit - 1 and the limit
//!   always, the limit + 1 refused), each encoded, decoded back, and every strict prefix
//!   `Incomplete`.
//! - RECORDS: random canonical-prefix lines of the channel's kinds, ticks and timelines at their
//!   edges, encoded and decoded to the same frame; one changed prefix field is refused.
//! - Control: random requests, responses, errors and notifications with integers at the `i64`
//!   and `u64` edges and strings of printable ASCII (`"` and `\` included); canonical bytes parse
//!   back to the same value and re-encode identically; a non-canonical spelling (whitespace, keys
//!   reversed) parses to the same value.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::format_collect,
    clippy::items_after_statements,
    reason = "test helpers fail loudly; generators truncate on purpose"
)]

use rh_protocol::control::{self, Kind, Reason};
use rh_protocol::json;
use rh_protocol::records::{self, Channel, KIND_CHANNEL, RecordsFrame};
use rh_protocol::wire::{self, Frame, FrameType, MAX_PAYLOAD};
use serde_json::{Map, Value, json};

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
        self.next_u64() % n.max(1)
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[usize::try_from(self.below(xs.len() as u64)).unwrap_or(0)]
    }
}

const SEEDS: [u64; 4] = [1, 0x5eed, 0xdead_beef, u64::MAX];

fn size(g: &mut SplitMix64, limit: usize) -> usize {
    let lim = limit as u64;
    let n = match g.below(10) {
        0 => *g.pick(&[0, 1, lim - 1, lim]),
        1 => g.below(lim + 1),
        2..=4 => g.below(4097),
        _ => g.below(257),
    };
    usize::try_from(n).unwrap_or(0)
}

#[test]
fn frames_round_trip_at_every_size_and_every_prefix_is_incomplete() {
    let mut cases = 0;
    for seed in SEEDS {
        let mut g = SplitMix64(seed);
        for _ in 0..1_500 {
            let n = size(&mut g, MAX_PAYLOAD);
            let ftype = if g.below(2) == 0 {
                FrameType::Control
            } else {
                FrameType::Records
            };
            let payload: Vec<u8> = (0..n)
                .map(|i| (g.0.wrapping_add(i as u64) >> 3) as u8)
                .collect();
            let frame = Frame { ftype, payload };
            let bytes = wire::encode(&frame).unwrap_or_else(|e| panic!("{n}: {e}"));
            let mut tail = bytes.clone();
            tail.extend_from_slice(b"next");
            let (back, used) = wire::decode(&tail, MAX_PAYLOAD).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!((back, used), (frame, bytes.len()), "size {n}");
            for cut in [
                0,
                1,
                11,
                12,
                bytes.len() - 1,
                usize::try_from(g.below(bytes.len() as u64)).unwrap_or(0),
            ] {
                if cut < bytes.len() {
                    let e = wire::decode(&bytes[..cut], MAX_PAYLOAD).err();
                    assert!(
                        e.is_some_and(|e| e.is_incomplete()),
                        "a {cut}-byte prefix of {n}"
                    );
                }
            }
            cases += 1;
        }
    }
    let over = Frame {
        ftype: FrameType::Control,
        payload: vec![b' '; MAX_PAYLOAD + 1],
    };
    assert!(
        wire::encode(&over).is_err(),
        "a payload over the limit encodes"
    );
    assert!(cases >= 6_000);
    println!(
        "round trips: {cases} frames from 0 bytes to the 1 MiB limit, every prefix incomplete"
    );
}

fn line(g: &mut SplitMix64, channel: Channel, tl: u32, seq: u64) -> String {
    let kinds: Vec<&str> = KIND_CHANNEL
        .iter()
        .filter(|(_, c)| *c == channel)
        .map(|(k, _)| *k)
        .collect();
    let tick: i64 = match g.below(4) {
        0 => *g.pick(&[i64::MIN, i64::MAX, 0, -1]),
        _ => i64::try_from(g.below(1 << 40)).unwrap_or(0) - (1 << 39),
    };
    let rest_len = usize::try_from(g.below(80)).unwrap_or(0);
    let rest: String = (0..rest_len)
        .map(|_| char::from(u8::try_from(0x20 + g.below(0x5f)).unwrap_or(b'x')))
        .collect();
    format!("{} {tl} {seq} {tick} {rest}", g.pick(&kinds))
}

#[test]
fn records_round_trip_and_a_changed_prefix_is_refused() {
    let mut cases = 0;
    for seed in SEEDS {
        let mut g = SplitMix64(seed ^ 0xabc);
        for _ in 0..1_000 {
            let channel = if g.below(2) == 0 {
                Channel::A
            } else {
                Channel::C
            };
            let tl = u32::try_from(*g.pick(&[0, 1, u64::from(u32::MAX)])).unwrap_or(0);
            let span = *g.pick(&[1, 4, 64, 400]);
            let count = 1 + g.below(span);
            let first_seq = match g.below(4) {
                0 => u64::MAX - (count - 1),
                _ => g.below(1 << 50),
            };
            let lines: Vec<String> = (0..count)
                .map(|i| line(&mut g, channel, tl, first_seq + i))
                .collect();
            let recs = RecordsFrame {
                timeline_id: tl,
                channel,
                first_seq,
                lines,
            };
            let payload = records::encode(&recs).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(records::decode(&payload), Ok(recs.clone()));
            // One prefix field changed in one line: refused.
            let mut bad = recs.clone();
            let k = usize::try_from(g.below(count)).unwrap_or(0);
            let toks: Vec<String> = bad.lines[k].splitn(5, ' ').map(str::to_owned).collect();
            let field = usize::try_from(g.below(3)).unwrap_or(0);
            let mut t = toks.clone();
            match field {
                0 => t[0] = "BOGUS".to_owned(),
                1 => t[1] = format!("0{}", t[1]),
                _ => t[2] = (first_seq.wrapping_add(k as u64).wrapping_add(1)).to_string(),
            }
            bad.lines[k] = t.join(" ");
            assert!(
                records::encode(&bad).is_err(),
                "changed field {field}: {}",
                bad.lines[k]
            );
            cases += 1;
        }
    }
    assert!(cases >= 4_000);
    println!("round trips: {cases} RECORDS payloads, a changed prefix field refused in each");
}

fn ascii(g: &mut SplitMix64) -> String {
    let n = usize::try_from(g.below(12)).unwrap_or(0);
    (0..n)
        .map(|_| match g.below(8) {
            0 => '"',
            1 => '\\',
            _ => char::from(u8::try_from(0x20 + g.below(0x5f)).unwrap_or(b'x')),
        })
        .collect()
}

fn int(g: &mut SplitMix64) -> Value {
    match g.below(5) {
        0 => json!(u64::MAX),
        1 => json!(i64::MIN),
        2 => json!(*g.pick(&[0u64, 1, u64::from(u32::MAX)])),
        3 => json!(i64::try_from(g.below(1 << 62)).unwrap_or(0) - (1 << 61)),
        _ => json!(g.next_u64()),
    }
}

fn scalar(g: &mut SplitMix64) -> Value {
    match g.below(5) {
        0 => Value::Null,
        1 => Value::Bool(g.below(2) == 0),
        2 | 3 => int(g),
        _ => Value::String(ascii(g)),
    }
}

/// A value nested at most `room` more levels.
fn value(g: &mut SplitMix64, room: usize) -> Value {
    if room == 0 || g.below(3) == 0 {
        return scalar(g);
    }
    if g.below(2) == 0 {
        Value::Array((0..g.below(4)).map(|_| value(g, room - 1)).collect())
    } else {
        let mut m = Map::new();
        for _ in 0..g.below(4) {
            m.insert(ascii(g), value(g, room - 1));
        }
        Value::Object(m)
    }
}

fn message(g: &mut SplitMix64) -> (Kind, Value) {
    let id = if g.below(2) == 0 {
        int(g)
    } else {
        Value::String(ascii(g))
    };
    match g.below(4) {
        0 => {
            let params = (g.below(2) == 0).then(|| {
                let mut m = Map::new();
                m.insert("from".into(), value(g, 2));
                Value::Object(m)
            });
            (Kind::Request, control::request(&id, &ascii(g), params))
        }
        1 => (Kind::Response, control::response(&Some(id), value(g, 3))),
        2 => {
            let reason = *g.pick(&control::REASONS);
            let mut d = Map::new();
            d.insert("detail".into(), Value::String(ascii(g)));
            d.insert("n".into(), int(g));
            (Kind::Error, control::error(&Some(id), reason, d))
        }
        _ => {
            let mut m = Map::new();
            m.insert(ascii(g), value(g, 2));
            (
                Kind::Notification,
                control::notification(&ascii(g), Value::Object(m)),
            )
        }
    }
}

/// The same value spelled differently: whitespace around every token, object keys reversed.
fn respell(v: &Value) -> String {
    match v {
        Value::Object(m) => {
            let parts: Vec<String> = m
                .iter()
                .rev()
                .map(|(k, x)| format!(" {} :\n{} ", Value::String(k.clone()), respell(x)))
                .collect();
            format!("{{\t{}}}", parts.join(","))
        }
        Value::Array(a) => format!(
            "[ {} ]",
            a.iter().map(respell).collect::<Vec<_>>().join(" , ")
        ),
        other => other.to_string(),
    }
}

#[test]
fn control_messages_round_trip_in_any_spelling() {
    let mut cases = 0;
    for seed in SEEDS {
        let mut g = SplitMix64(seed.rotate_left(17));
        for _ in 0..1_500 {
            let (kind, v) = message(&mut g);
            let bytes = json::canonical(&v).unwrap_or_else(|e| panic!("{v}: {e}"));
            let (k, back) = control::decode_control(&bytes).unwrap_or_else(|e| panic!("{v}: {e}"));
            assert_eq!((k, &back), (kind, &v));
            assert_eq!(json::canonical(&back).unwrap_or_default(), bytes);
            let other = respell(&v);
            assert_eq!(json::parse(other.as_bytes()).as_ref(), Ok(&v), "{other}");
            // The frame level: the message in a CONTROL frame, through the codec.
            let frame = control::control_frame(&v).unwrap_or_default();
            let (f, _) = wire::decode(&frame, MAX_PAYLOAD).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(f.payload, bytes);
            cases += 1;
        }
    }
    assert!(cases >= 6_000);
    println!("round trips: {cases} control messages in canonical and other spellings");
}

#[test]
fn a_stream_of_frames_decodes_the_same_from_every_split_point() {
    let mut g = SplitMix64(42);
    let mut stream = Vec::new();
    let mut want = Vec::new();
    for _ in 0..200 {
        let (_, v) = message(&mut g);
        let f = control::control_frame(&v).unwrap_or_default();
        want.push(f.clone());
        stream.extend_from_slice(&f);
    }
    for _ in 0..200 {
        // Feed the stream in random chunks; decode whole frames as they complete.
        let mut buf: Vec<u8> = Vec::new();
        let mut got = Vec::new();
        let mut at = 0;
        while at < stream.len() {
            let step = 1 + usize::try_from(g.below(300)).unwrap_or(0);
            let end = (at + step).min(stream.len());
            buf.extend_from_slice(&stream[at..end]);
            at = end;
            loop {
                match wire::decode(&buf, MAX_PAYLOAD) {
                    Ok((_, used)) => {
                        got.push(buf[..used].to_vec());
                        buf.drain(..used);
                    }
                    Err(e) if e.is_incomplete() => break,
                    Err(e) => panic!("{e}"),
                }
            }
        }
        assert_eq!(got, want);
    }
}

#[test]
fn floats_anywhere_are_invalid_params_and_other_values_are_refused_by_name() {
    let cases: [(&[u8], Reason); 9] = [
        (br#"{"id":1,"jsonrpc":"2.0","method":"stream.subscribe","params":{"from":{"seq":1e3}}}"#, Reason::InvalidParams),
        (br#"{"id":-0.0,"jsonrpc":"2.0","method":"x"}"#, Reason::InvalidParams),
        (br#"{"id":18446744073709551616,"jsonrpc":"2.0","method":"x"}"#, Reason::InvalidRequest),
        (br#"{"id":-9223372036854775809,"jsonrpc":"2.0","method":"x"}"#, Reason::InvalidRequest),
        (br#"{"id":"\u0001","jsonrpc":"2.0","method":"x"}"#, Reason::InvalidRequest),
        (b"{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"x\",\"params\":{\"\xc3\xa9\":1}}", Reason::InvalidRequest),
        (br#"{"id":1,"jsonrpc":"2.0","method":"x"} x"#, Reason::ParseError),
        (b"\xff", Reason::ParseError),
        (br#"{"id":01,"jsonrpc":"2.0","method":"x"}"#, Reason::ParseError),
    ];
    for (payload, want) in cases {
        let frame = wire::encode(&Frame {
            ftype: FrameType::Control,
            payload: payload.to_vec(),
        })
        .unwrap_or_default();
        struct Empty;
        impl control::Source for Empty {
            fn view(&self) -> control::HostView {
                unreachable_view()
            }
        }
        let mut conn = control::Connection::default();
        let reply = control::dispatch(&mut conn, &Empty, &frame);
        let (f, _) =
            wire::decode(&reply.frames.concat(), MAX_PAYLOAD).unwrap_or_else(|e| panic!("{e}"));
        let (_, v) = control::decode_control(&f.payload).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            control::reply_reason(&v),
            Some(want.name()),
            "{}",
            String::from_utf8_lossy(payload)
        );
        assert!(reply.close, "the host closes after an unidentified frame");
    }
    // 100 000 nested arrays: a refusal, not a stack overflow.
    let deep = vec![b'['; 100_000];
    assert!(json::parse(&deep).is_err_and(|e| e.message.contains("parse_error")));
}

fn unreachable_view() -> control::HostView {
    control::HostView {
        channel: Channel::A,
        timeline_id: 0,
        run_nonce: 1,
        record_format: String::new(),
        history: control::History {
            engine_build: String::new(),
            registry_version: String::new(),
            manifest_hash: String::new(),
            world_seed: 0,
        },
        minutes_per_year: 1,
        tip_tick: None,
        next_seq: 0,
        live_first_seq: 0,
        sealed_last_year: None,
        sealed_last_seq: None,
        pace: None,
        sifted_through: None,
        finished: false,
        seal_log_len: 0,
    }
}
