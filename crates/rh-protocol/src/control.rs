//! The control plane (`wire_v1.md` §4): JSON-RPC 2.0 objects, the three read methods, the
//! notifications, the error codes, and the host's dispatcher.
//!
//! **The method table** ([`METHOD_TABLE`]) is exactly `stream.status`, `stream.subscribe` and
//! `stream.unsubscribe`, each a read (§4.5's invariant for every version): every other method, in
//! any namespace, is answered `method_not_found` with the read-only message. Nothing a viewer
//! sends reaches the engine: [`dispatch`] reads the host through a [`Source`] and writes only the
//! viewer's own [`Connection`].
//!
//! **Refusals** follow the reference (`specs/stream/selftest_wire_v1.py` `Host.dispatch`) byte for
//! byte on the golden lines: the 12 header bytes are judged first (a RECORDS frame, a bad version
//! or a payload over 4 KiB is refused before the payload is read); a payload that does not decode
//! is answered with `id` null (`parse_error`, `invalid_request`, or `invalid_params` for a float);
//! a request's parameter errors carry its `id`.
//!
//! **Closing.** The host closes the connection after every answer whose `id` is null: a frame
//! error, a payload that does not parse, or an object that is not a request (the request could
//! not be identified, so nothing after it can be trusted). A refused parameter or an unknown method
//! of a well-formed request keeps the connection open.

use serde_json::{Map, Value, json};

use crate::json::{canonical, parse};
use crate::records::Channel;
use crate::wire::{self, Frame, FrameType, VIEWER_MAX_PAYLOAD, WireError, WireErrorKind};

/// The JSON-RPC version string.
pub const JSONRPC: &str = "2.0";
/// This wire's format string.
pub const WIRE_FORMAT: &str = "rehoboam/stream/wire/v1";
/// The canonical form the RECORDS bodies follow.
pub const CANONICAL_FORMAT: &str = "rehoboam/stream/canonical/v0";
/// The one answer to an attempt to act on a stream socket (§4.4, §4.5).
pub const READ_ONLY_MESSAGE: &str =
    "this socket carries the stream only and accepts no input; inputs go through the input journal";
/// The host's default connection cap (§1).
pub const DEFAULT_CAP: usize = 64;

/// What a method of the table does to the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// It reads (the only kind a stream socket may have, §4.5).
    Read,
    /// It would change something (never on a stream socket; the planted control only).
    Write,
}

/// A method of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// `stream.status`.
    Status,
    /// `stream.subscribe`.
    Subscribe,
    /// `stream.unsubscribe`.
    Unsubscribe,
    /// The planted control's fourth method (`mutant-pace-method`): never in a real build.
    #[cfg(feature = "mutant-pace-method")]
    Pace,
}

/// The dispatcher's method table: name, method, access (§4.5).
#[cfg(not(feature = "mutant-pace-method"))]
pub const METHOD_TABLE: &[(&str, Method, Access)] = &[
    ("stream.status", Method::Status, Access::Read),
    ("stream.subscribe", Method::Subscribe, Access::Read),
    ("stream.unsubscribe", Method::Unsubscribe, Access::Read),
];
/// The dispatcher's method table, with the planted fourth method (`mutant-pace-method`).
#[cfg(feature = "mutant-pace-method")]
pub const METHOD_TABLE: &[(&str, Method, Access)] = &[
    ("stream.status", Method::Status, Access::Read),
    ("stream.subscribe", Method::Subscribe, Access::Read),
    ("stream.unsubscribe", Method::Unsubscribe, Access::Read),
    ("stream.pace", Method::Pace, Access::Write),
];

/// The machine-readable reasons of §4.4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// −32700.
    ParseError,
    /// −32600.
    InvalidRequest,
    /// −32601.
    MethodNotFound,
    /// −32602.
    InvalidParams,
    /// −32110.
    UnsupportedVersion,
    /// −32111.
    InvalidFrame,
    /// −32112.
    BehindLiveEdge,
    /// −32113.
    FutureSeq,
    /// −32114.
    AlreadySubscribed,
    /// −32115.
    NotSubscribed,
    /// −32116.
    TooManyViewers,
}

/// Every reason, in code order.
pub const REASONS: [Reason; 11] = [
    Reason::ParseError,
    Reason::InvalidRequest,
    Reason::MethodNotFound,
    Reason::InvalidParams,
    Reason::UnsupportedVersion,
    Reason::InvalidFrame,
    Reason::BehindLiveEdge,
    Reason::FutureSeq,
    Reason::AlreadySubscribed,
    Reason::NotSubscribed,
    Reason::TooManyViewers,
];

impl Reason {
    /// The `data.reason` token.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ParseError => "parse_error",
            Self::InvalidRequest => "invalid_request",
            Self::MethodNotFound => "method_not_found",
            Self::InvalidParams => "invalid_params",
            Self::UnsupportedVersion => "unsupported_version",
            Self::InvalidFrame => "invalid_frame",
            Self::BehindLiveEdge => "behind_live_edge",
            Self::FutureSeq => "future_seq",
            Self::AlreadySubscribed => "already_subscribed",
            Self::NotSubscribed => "not_subscribed",
            Self::TooManyViewers => "too_many_viewers",
        }
    }

    /// The code.
    #[must_use]
    pub const fn code(self) -> i64 {
        match self {
            Self::ParseError => -32700,
            Self::InvalidRequest => -32600,
            Self::MethodNotFound => -32601,
            Self::InvalidParams => -32602,
            Self::UnsupportedVersion => -32110,
            Self::InvalidFrame => -32111,
            Self::BehindLiveEdge => -32112,
            Self::FutureSeq => -32113,
            Self::AlreadySubscribed => -32114,
            Self::NotSubscribed => -32115,
            Self::TooManyViewers => -32116,
        }
    }

    /// The message for people.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::ParseError => "the payload is not JSON",
            Self::InvalidRequest => "not a JSON-RPC 2.0 request object",
            Self::MethodNotFound => READ_ONLY_MESSAGE,
            Self::InvalidParams => "a parameter is missing, of the wrong type or unknown",
            Self::UnsupportedVersion => "the frame's version is not one this host speaks",
            Self::InvalidFrame => "a bad magic, type, flag, length, sub-header or record line",
            Self::BehindLiveEdge => {
                "the requested seq is below the live window; read sealed segments"
            }
            Self::FutureSeq => "the requested seq is above the tip",
            Self::AlreadySubscribed => "this connection already holds a subscription",
            Self::NotSubscribed => "this connection holds no subscription",
            Self::TooManyViewers => "the host's connection cap is reached",
        }
    }

    /// The reason of a token.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        REASONS.into_iter().find(|r| r.name() == name)
    }

    /// The reason of a code.
    #[must_use]
    pub fn from_code(code: i64) -> Option<Self> {
        REASONS.into_iter().find(|r| r.code() == code)
    }

    /// Whether a viewer retries it with backoff (§6.3).
    #[must_use]
    pub const fn retriable(self) -> bool {
        matches!(self, Self::FutureSeq | Self::TooManyViewers)
    }
}

/// A request id: an integer or a string (null only in a host's answer to an unidentified frame).
pub type Id = Option<Value>;

/// A request object.
#[must_use]
pub fn request(id: &Value, method: &str, params: Option<Value>) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), id.clone());
    m.insert("jsonrpc".into(), JSONRPC.into());
    m.insert("method".into(), method.into());
    if let Some(p) = params {
        m.insert("params".into(), p);
    }
    Value::Object(m)
}

/// A success response.
#[must_use]
pub fn response(id: &Id, result: Value) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), id.clone().unwrap_or(Value::Null));
    m.insert("jsonrpc".into(), JSONRPC.into());
    m.insert("result".into(), result);
    Value::Object(m)
}

/// An error response for `reason`, with its extra `data` fields.
#[must_use]
pub fn error(id: &Id, reason: Reason, data: Map<String, Value>) -> Value {
    let mut d = data;
    d.insert("reason".into(), reason.name().into());
    json!({
        "error": {"code": reason.code(), "data": Value::Object(d), "message": reason.message()},
        "id": id.clone().unwrap_or(Value::Null),
        "jsonrpc": JSONRPC,
    })
}

/// An error response whose only extra field is `detail`.
#[must_use]
pub fn error_detail(id: &Id, reason: Reason, detail: &str) -> Value {
    let mut d = Map::new();
    d.insert("detail".into(), printable_detail(detail).into());
    error(id, reason, d)
}

/// `text` with every character outside printable ASCII written `\u{..}`, so a detail always fits
/// the wire's string rule (§3.1).
#[must_use]
pub fn printable_detail(text: &str) -> String {
    text.chars()
        .map(|c| {
            if (' '..='~').contains(&c) {
                c.to_string()
            } else {
                c.escape_unicode().to_string()
            }
        })
        .collect()
}

/// A notification.
#[must_use]
pub fn notification(method: &str, params: Value) -> Value {
    let mut m = Map::new();
    m.insert("jsonrpc".into(), JSONRPC.into());
    m.insert("method".into(), method.into());
    m.insert("params".into(), params);
    Value::Object(m)
}

/// The bytes of a CONTROL frame carrying `value` in canonical JSON.
///
/// # Errors
/// [`WireError`] if the value is outside the wire or the frame too long.
pub fn control_frame(value: &Value) -> Result<Vec<u8>, WireError> {
    wire::encode(&Frame {
        ftype: FrameType::Control,
        payload: canonical(value)?,
    })
}

/// The JSON-RPC class of a control object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A request (`id`, `method`).
    Request,
    /// A success response.
    Response,
    /// An error response.
    Error,
    /// A notification (no `id`).
    Notification,
}

impl Kind {
    /// The reference's name of the class.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Response => "response",
            Self::Error => "error",
            Self::Notification => "notification",
        }
    }
}

fn invalid_request(m: impl Into<String>) -> WireError {
    WireError::new(
        WireErrorKind::Request,
        format!("invalid_request: {}", m.into()),
    )
}

fn invalid_params(m: impl Into<String>) -> WireError {
    WireError::new(
        WireErrorKind::Params,
        format!("invalid_params: {}", m.into()),
    )
}

fn is_id(v: &Value) -> bool {
    matches!(v, Value::Null | Value::String(_)) || v.as_i64().is_some() || v.as_u64().is_some()
}

fn only_keys(obj: &Map<String, Value>, allowed: &[&str], what: &str) -> Result<(), WireError> {
    // serde_json's map is sorted, so the first unknown key is the smallest (as the reference's
    // `sorted(extra)[0]`).
    match obj.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(k) => Err(invalid_request(format!("unknown member '{k}' in a {what}"))),
        None => Ok(()),
    }
}

/// The class of a parsed control object (§3.1, §4).
///
/// # Errors
/// [`WireError`] for anything that is not a JSON-RPC 2.0 object of this wire.
pub fn classify(v: &Value) -> Result<Kind, WireError> {
    let Value::Object(obj) = v else {
        return Err(invalid_request("not a JSON object (a batch is refused)"));
    };
    if obj.get("jsonrpc").and_then(Value::as_str) != Some(JSONRPC) {
        return Err(invalid_request("jsonrpc is not '2.0'"));
    }
    if let Some(method) = obj.get("method") {
        if !method.is_string() {
            return Err(invalid_request("method is not a string"));
        }
        if obj.get("params").is_some_and(|p| !p.is_object()) {
            return Err(invalid_params("params is not an object"));
        }
        let Some(id) = obj.get("id") else {
            only_keys(obj, &["jsonrpc", "method", "params"], "notification")?;
            return Ok(Kind::Notification);
        };
        only_keys(obj, &["jsonrpc", "id", "method", "params"], "request")?;
        if id.is_null() || !is_id(id) {
            return Err(invalid_request("a request id is an integer or a string"));
        }
        return Ok(Kind::Request);
    }
    if !obj.get("id").is_some_and(is_id) {
        return Err(invalid_request("a response carries an id"));
    }
    if obj.contains_key("result") == obj.contains_key("error") {
        return Err(invalid_request(
            "a response has exactly one of result and error",
        ));
    }
    if !obj.contains_key("error") {
        only_keys(obj, &["jsonrpc", "id", "result"], "response")?;
        return Ok(Kind::Response);
    }
    only_keys(obj, &["jsonrpc", "id", "error"], "error response")?;
    let Some(err) = obj.get("error").and_then(Value::as_object) else {
        return Err(invalid_request("error without an integer code"));
    };
    let Some(code) = err.get("code").and_then(Value::as_i64) else {
        return Err(invalid_request("error without an integer code"));
    };
    only_keys(err, &["code", "message", "data"], "error object")?;
    let reason = err
        .get("data")
        .and_then(Value::as_object)
        .and_then(|d| d.get("reason"))
        .and_then(Value::as_str);
    if reason.is_none() || Reason::from_code(code).map(Reason::name) != reason {
        return Err(invalid_request(
            "error.data.reason does not match error.code",
        ));
    }
    Ok(Kind::Error)
}

/// The class and object of a CONTROL payload.
///
/// # Errors
/// [`WireError`] from [`parse`] or [`classify`].
pub fn decode_control(payload: &[u8]) -> Result<(Kind, Value), WireError> {
    let v = parse(payload)?;
    let kind = classify(&v)?;
    Ok((kind, v))
}

/// The reason of a reply, or `None` for a success.
#[must_use]
pub fn reply_reason(reply: &Value) -> Option<&str> {
    reply.get("error")?.get("data")?.get("reason")?.as_str()
}

/// Where a `stream.subscribe` starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubscribeFrom {
    /// From this seq.
    Seq(u64),
    /// From the tip (`"live"`: jump to live).
    Live,
}

impl SubscribeFrom {
    /// The request's `params`.
    #[must_use]
    pub fn params(self) -> Value {
        match self {
            Self::Seq(n) => json!({"from": {"seq": n}}),
            Self::Live => json!({"from": "live"}),
        }
    }
}

/// The `from` of a `stream.subscribe`'s params (§4.2).
///
/// # Errors
/// [`WireError`] (`invalid_params`) for anything but `{from: {seq: u64}}` or `{from: "live"}`.
pub fn subscribe_from(params: Option<&Value>) -> Result<SubscribeFrom, WireError> {
    let Some(Value::Object(p)) = params else {
        return Err(invalid_params("params are exactly {from}"));
    };
    if p.len() != 1 || !p.contains_key("from") {
        return Err(invalid_params("params are exactly {from}"));
    }
    match &p["from"] {
        Value::String(s) if s == "live" => Ok(SubscribeFrom::Live),
        Value::Object(f) if f.len() == 1 && f.contains_key("seq") => f["seq"]
            .as_u64()
            .map(SubscribeFrom::Seq)
            .ok_or_else(|| invalid_params("from.seq is not a u64")),
        _ => Err(invalid_params("from is {seq: u64} or 'live'")),
    }
}

fn no_params(params: Option<&Value>) -> Result<(), WireError> {
    match params {
        None => Ok(()),
        Some(Value::Object(m)) if m.is_empty() => Ok(()),
        Some(_) => Err(invalid_params("this method takes no parameter")),
    }
}

/// The history tuple a socket's segments carry (§4.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History {
    /// The engine build.
    pub engine_build: String,
    /// The Registry version.
    pub registry_version: String,
    /// The manifest hash (32 hex).
    pub manifest_hash: String,
    /// The world seed.
    pub world_seed: u64,
}

impl History {
    /// Its JSON.
    #[must_use]
    pub fn value(&self) -> Value {
        json!({
            "engine_build": self.engine_build,
            "manifest_hash": self.manifest_hash,
            "registry_version": self.registry_version,
            "world_seed": self.world_seed,
        })
    }
}

/// What a host knows of its stream at one moment: every field from one snapshot (§6.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostView {
    /// The channel this socket serves.
    pub channel: Channel,
    /// The one timeline it serves.
    pub timeline_id: u32,
    /// This host's start (§3.2).
    pub run_nonce: u64,
    /// The segments' `segment.format`.
    pub record_format: String,
    /// The history tuple.
    pub history: History,
    /// The world calendar's year in minutes.
    pub minutes_per_year: i64,
    /// The last instant whose records are written (`None` before the first).
    pub tip_tick: Option<i64>,
    /// The seq the next record will get (`w1`).
    pub next_seq: u64,
    /// The window's first seq (`w0`).
    pub live_first_seq: u64,
    /// The newest sealed sim-year, empty or not.
    pub sealed_last_year: Option<i64>,
    /// The last seq of the newest non-empty sealed segment (`s`).
    pub sealed_last_seq: Option<u64>,
    /// Channel A: the pace band and state; channel C: `None`.
    pub pace: Option<(String, String)>,
    /// Channel C: the channel-A seq and sealed sim-year sifted through; channel A: `None`.
    pub sifted_through: Option<(Option<u64>, Option<i64>)>,
    /// Whether the writer has stopped (`future_seq` is then terminal).
    pub finished: bool,
    /// The seal log's length (a new subscription pulls the entries after it).
    pub seal_log_len: usize,
}

impl HostView {
    /// The `stream.status` result (§4.1).
    #[must_use]
    pub fn status(&self) -> Value {
        let pace = self.pace.as_ref().map_or(
            Value::Null,
            |(band, state)| json!({"band": band, "state": state}),
        );
        let sifted = self.sifted_through.map_or(
            Value::Null,
            |(a_seq, a_year)| json!({"a_seq": a_seq, "a_year": a_year}),
        );
        json!({
            "calendar": {"minutes_per_year": self.minutes_per_year},
            "canonical_format": CANONICAL_FORMAT,
            "channel": self.channel.letter(),
            "history": self.history.value(),
            "live": {"first_seq": self.live_first_seq, "next_seq": self.next_seq},
            "pace": pace,
            "record_format": self.record_format,
            "run_nonce": self.run_nonce,
            "sealed": {"last_seq": self.sealed_last_seq, "last_year": self.sealed_last_year},
            "sifted_through": sifted,
            "timeline_id": self.timeline_id,
            "tip": {"next_seq": self.next_seq, "tick": self.tip_tick},
            "wire": {"format": WIRE_FORMAT, "version": wire::VERSION},
        })
    }

    /// The `behind_live_edge` and `stream.behind` fields (from this one snapshot).
    #[must_use]
    pub fn behind_data(&self) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("live_first_seq".into(), self.live_first_seq.into());
        m.insert("sealed_last_seq".into(), json!(self.sealed_last_seq));
        m.insert("sealed_last_year".into(), json!(self.sealed_last_year));
        m
    }
}

/// How a host reads its stream: one view per call, each from one snapshot.
pub trait Source {
    /// The stream's state now.
    fn view(&self) -> HostView;

    /// The index in the seal log for a subscription starting at `from_seq` (`None` for live).
    fn seal_index(&self, from_seq: Option<u64>) -> usize {
        let _ = from_seq;
        self.view().seal_log_len
    }
}

/// One viewer's connection as the host sees it: a cursor and its place in the seal log.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Connection {
    /// The next seq to send (`None`: no subscription).
    pub cursor: Option<u64>,
    /// The seal-log entries already sent or skipped.
    pub seal_index: usize,
    /// Whether the host closes the connection.
    pub closed: bool,
}

/// The host's answer to one frame from a viewer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reply {
    /// The frames to send, in order.
    pub frames: Vec<Vec<u8>>,
    /// Whether the host then closes the connection.
    pub close: bool,
    /// The bytes of the viewer's buffer the frame used (0: incomplete, read more).
    pub used: usize,
}

fn frame_or_internal(v: &Value) -> Vec<u8> {
    // Every value built here is on the wire (integers, printable ASCII), so encoding cannot fail.
    control_frame(v).unwrap_or_default()
}

/// The `too_many_viewers` frame a host sends first on a connection over its cap, then closes.
#[must_use]
pub fn too_many_viewers(limit: usize) -> Vec<u8> {
    let mut d = Map::new();
    d.insert("limit".into(), json!(limit));
    frame_or_internal(&error(&None, Reason::TooManyViewers, d))
}

/// Answers the frame at the start of `data` from a viewer (§4): the host's dispatcher.
///
/// The header is judged first; a short buffer is answered with nothing and `used` 0 (read more).
#[must_use]
pub fn dispatch(conn: &mut Connection, source: &dyn Source, data: &[u8]) -> Reply {
    let header = wire::decode_header(data, VIEWER_MAX_PAYLOAD).and_then(|(ftype, _)| {
        if ftype == FrameType::Records {
            return Err(WireError::frame("a RECORDS frame from a viewer"));
        }
        wire::decode(data, VIEWER_MAX_PAYLOAD)
    });
    let (frame, used) = match header {
        Ok(x) => x,
        Err(e) if e.is_incomplete() => return Reply::default(),
        Err(e) => {
            conn.closed = true;
            let v = if e.kind == WireErrorKind::UnsupportedVersion {
                let mut d = Map::new();
                d.insert("supported".into(), json!([wire::VERSION]));
                error(&None, Reason::UnsupportedVersion, d)
            } else {
                error_detail(&None, Reason::InvalidFrame, &e.message)
            };
            return Reply {
                frames: vec![frame_or_internal(&v)],
                close: true,
                used: data.len(),
            };
        }
    };
    let answer = answer(conn, source, &frame.payload);
    let close = answer.get("id").is_none_or(Value::is_null);
    conn.closed |= close;
    Reply {
        frames: vec![frame_or_internal(&answer)],
        close,
        used,
    }
}

fn answer(conn: &mut Connection, source: &dyn Source, payload: &[u8]) -> Value {
    let (kind, obj) = match decode_control(payload) {
        Ok(x) => x,
        Err(e) => {
            let reason = match e.kind {
                WireErrorKind::Parse => Reason::ParseError,
                WireErrorKind::Params => Reason::InvalidParams,
                _ => Reason::InvalidRequest,
            };
            return error_detail(&None, reason, &e.message);
        }
    };
    if kind != Kind::Request {
        return error_detail(
            &None,
            Reason::InvalidRequest,
            &format!("a {} from a viewer", kind.name()),
        );
    }
    let id = obj.get("id").cloned();
    let method = obj
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let Some((_, m, _)) = METHOD_TABLE.iter().find(|(name, _, _)| *name == method) else {
        return error(&id, Reason::MethodNotFound, Map::new());
    };
    let params = obj.get("params");
    let routed = match m {
        Method::Subscribe => subscribe_from(params).map(|from| subscribe(conn, source, &id, from)),
        Method::Status => no_params(params).map(|()| response(&id, source.view().status())),
        Method::Unsubscribe => no_params(params).map(|()| unsubscribe(conn, &id)),
        #[cfg(feature = "mutant-pace-method")]
        Method::Pace => Ok(response(&id, json!({}))),
    };
    routed.unwrap_or_else(|e| error_detail(&id, Reason::InvalidParams, &e.message))
}

/// Answers `stream.subscribe` against one view (§4.2, §6.2).
fn subscribe(conn: &mut Connection, source: &dyn Source, id: &Id, from: SubscribeFrom) -> Value {
    if conn.cursor.is_some() {
        return error(id, Reason::AlreadySubscribed, Map::new());
    }
    let view = source.view();
    let n = match from {
        SubscribeFrom::Seq(n) => n,
        SubscribeFrom::Live => view.next_seq,
    };
    if n < view.live_first_seq {
        return error(id, Reason::BehindLiveEdge, view.behind_data());
    }
    if n > view.next_seq {
        let mut d = Map::new();
        d.insert("finished".into(), view.finished.into());
        d.insert("next_seq".into(), view.next_seq.into());
        return error(id, Reason::FutureSeq, d);
    }
    conn.cursor = Some(n);
    conn.seal_index = match from {
        SubscribeFrom::Seq(n) => source.seal_index(Some(n)),
        SubscribeFrom::Live => source.seal_index(None),
    };
    response(id, json!({"first_seq": n}))
}

fn unsubscribe(conn: &mut Connection, id: &Id) -> Value {
    match conn.cursor.take() {
        None => error(id, Reason::NotSubscribed, Map::new()),
        Some(n) => response(id, json!({"next_seq": n})),
    }
}

/// A seal-log entry as `stream.sealed` carries it (§4.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sealed {
    /// The sim-year.
    pub sim_year: i64,
    /// Its first seq (an empty year: the seq its first record would have had).
    pub first_seq: u64,
    /// Its last seq (`None`: an empty year).
    pub last_seq: Option<u64>,
    /// Its `canonical_digest`, 32 lowercase hex.
    pub canonical_digest: String,
}

/// The `stream.sealed` notification.
#[must_use]
pub fn notify_sealed(channel: Channel, timeline_id: u32, s: &Sealed) -> Value {
    notification(
        "stream.sealed",
        json!({
            "canonical_digest": s.canonical_digest,
            "channel": channel.letter(),
            "first_seq": s.first_seq,
            "last_seq": s.last_seq,
            "sim_year": s.sim_year,
            "timeline_id": timeline_id,
        }),
    )
}

/// The `stream.behind` notification: the subscription has ended at `next_seq` (`view`'s fields).
#[must_use]
pub fn notify_behind(view: &HostView, next_seq: u64) -> Value {
    let mut p = view.behind_data();
    p.insert("channel".into(), view.channel.letter().into());
    p.insert("timeline_id".into(), view.timeline_id.into());
    p.insert("next_seq".into(), next_seq.into());
    notification("stream.behind", Value::Object(p))
}

/// Why a writer stopped (`stream.end`'s `reason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    /// The run ended normally and its last segment is sealed.
    Finished,
    /// The host stopped on an error; the unsealed year is not sealed.
    Aborted,
}

impl EndReason {
    /// The token.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Finished => "finished",
            Self::Aborted => "aborted",
        }
    }

    /// The reason of a token.
    #[must_use]
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "finished" => Some(Self::Finished),
            "aborted" => Some(Self::Aborted),
            _ => None,
        }
    }
}

/// The `stream.end` notification.
#[must_use]
pub fn notify_end(channel: Channel, timeline_id: u32, next_seq: u64, reason: EndReason) -> Value {
    notification(
        "stream.end",
        json!({
            "channel": channel.letter(),
            "next_seq": next_seq,
            "reason": reason.name(),
            "timeline_id": timeline_id,
        }),
    )
}

fn field_u64(v: &Value, key: &str) -> Result<u64, WireError> {
    v.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| WireError::new(WireErrorKind::Request, format!("{key} is not a u64")))
}

fn field_opt_u64(v: &Value, key: &str) -> Result<Option<u64>, WireError> {
    match v.get(key) {
        Some(Value::Null) => Ok(None),
        _ => field_u64(v, key).map(Some),
    }
}

fn field_opt_i64(v: &Value, key: &str) -> Result<Option<i64>, WireError> {
    match v.get(key) {
        Some(Value::Null) => Ok(None),
        Some(x) => x
            .as_i64()
            .map(Some)
            .ok_or_else(|| WireError::new(WireErrorKind::Request, format!("{key} is not an int"))),
        None => Err(WireError::new(
            WireErrorKind::Request,
            format!("missing {key}"),
        )),
    }
}

/// A checked `stream.status` result, as a viewer reads it (§4.1). Unknown fields are ignored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    /// The whole result.
    pub value: Value,
    /// `run_nonce`.
    pub run_nonce: u64,
    /// `live.first_seq`.
    pub live_first_seq: u64,
    /// `live.next_seq`.
    pub next_seq: u64,
    /// `sealed.last_seq`.
    pub sealed_last_seq: Option<u64>,
    /// `sealed.last_year`.
    pub sealed_last_year: Option<i64>,
}

impl Status {
    /// The history object (a viewer refuses to mix runs).
    #[must_use]
    pub fn history(&self) -> &Value {
        &self.value["history"]
    }
}

/// Checks a `stream.status` result's shape and invariants (§4.1; the reference's `check_status`).
///
/// # Errors
/// [`WireError`] naming the field or the invariant (I1) that fails.
pub fn check_status(result: &Value) -> Result<Status, WireError> {
    let bad = |m: &str| WireError::new(WireErrorKind::Request, format!("status: {m}"));
    for key in [
        "wire",
        "channel",
        "timeline_id",
        "tip",
        "sealed",
        "live",
        "history",
        "calendar",
        "run_nonce",
    ] {
        if result.get(key).is_none() {
            return Err(bad(&format!("missing {key}")));
        }
    }
    if result["wire"] != json!({"format": WIRE_FORMAT, "version": wire::VERSION}) {
        return Err(bad("wire is not this format and version"));
    }
    let (live, sealed, tip) = (&result["live"], &result["sealed"], &result["tip"]);
    if !(live.is_object() && sealed.is_object() && tip.is_object()) {
        return Err(bad("live, sealed and tip are objects"));
    }
    let run_nonce = field_u64(result, "run_nonce")?;
    let w0 = field_u64(live, "first_seq")?;
    let w1 = field_u64(live, "next_seq")?;
    if w0 > w1 || field_u64(tip, "next_seq")? != w1 {
        return Err(bad("live.first_seq <= live.next_seq = tip.next_seq"));
    }
    let s = field_opt_u64(sealed, "last_seq")?;
    match s {
        None if w0 != 0 => {
            return Err(bad(
                "live.first_seq is 0 before the first non-empty seal (I1)",
            ));
        }
        None => {}
        Some(s) => match s.checked_add(1) {
            Some(next) if next < w0 => {
                return Err(bad("sealed.last_seq + 1 >= live.first_seq (I1)"));
            }
            Some(_) => {}
            None => return Err(bad("sealed.last_seq overflows u64")),
        },
    }
    Ok(Status {
        value: result.clone(),
        run_nonce,
        live_first_seq: w0,
        next_seq: w1,
        sealed_last_seq: s,
        sealed_last_year: field_opt_i64(sealed, "last_year")?,
    })
}

/// A notification from the host, as a viewer reads it (§4.3); an unknown method is ignored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// `stream.sealed`.
    Sealed(Sealed),
    /// `stream.behind`: the subscription ended at `next_seq`.
    Behind {
        /// The viewer's cursor, as the host saw it.
        next_seq: u64,
        /// `sealed_last_seq`.
        sealed_last_seq: Option<u64>,
        /// `sealed_last_year`.
        sealed_last_year: Option<i64>,
        /// `live_first_seq`.
        live_first_seq: u64,
    },
    /// `stream.end`.
    End {
        /// The next seq the writer would have written.
        next_seq: u64,
        /// Why it stopped.
        reason: EndReason,
    },
    /// A notification this version does not know (§5: ignored).
    Unknown(String),
}

/// Reads a notification object.
///
/// # Errors
/// [`WireError`] for a known method with fields of the wrong type.
pub fn read_notice(v: &Value) -> Result<Notice, WireError> {
    let method = v.get("method").and_then(Value::as_str).unwrap_or_default();
    let p = v.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "stream.sealed" => Ok(Notice::Sealed(Sealed {
            sim_year: field_opt_i64(&p, "sim_year")?
                .ok_or_else(|| WireError::new(WireErrorKind::Request, "sim_year is null"))?,
            first_seq: field_u64(&p, "first_seq")?,
            last_seq: field_opt_u64(&p, "last_seq")?,
            canonical_digest: p
                .get("canonical_digest")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        })),
        "stream.behind" => Ok(Notice::Behind {
            next_seq: field_u64(&p, "next_seq")?,
            sealed_last_seq: field_opt_u64(&p, "sealed_last_seq")?,
            sealed_last_year: field_opt_i64(&p, "sealed_last_year")?,
            live_first_seq: field_u64(&p, "live_first_seq")?,
        }),
        "stream.end" => Ok(Notice::End {
            next_seq: field_u64(&p, "next_seq")?,
            reason: p
                .get("reason")
                .and_then(Value::as_str)
                .and_then(EndReason::from_name)
                .ok_or_else(|| WireError::new(WireErrorKind::Request, "an unknown end reason"))?,
        }),
        other => Ok(Notice::Unknown(other.to_owned())),
    }
}
