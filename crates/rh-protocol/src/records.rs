//! RECORDS payloads (`wire_v1.md` §2.2): a 20-byte sub-header, then `count` canonical lines,
//! each terminated by LF, in ascending `seq` with no gap.
//!
//! ```text
//! offset  size  field        value
//! 0       4     timeline_id  u32
//! 4       8     first_seq    u64
//! 12      4     count        u32 >= 1
//! 16      1     channel      'A' (0x41) or 'C' (0x43)
//! 17      3     reserved     0 0 0
//! 20      ...   body         count canonical lines, each followed by LF
//! ```
//!
//! The wire reads only the frozen prefix of a line, `<KIND> <timeline_id> <seq> <tick> `
//! (`canonical_v0.md` §1): the kind must be one the canonical format assigns to the frame's
//! channel ([`KIND_CHANNEL`]), the timeline the frame's, the seq `first_seq + i`, the tick a
//! canonical `i64`. Everything after the prefix is the canonical format's business. A serving
//! thread never parses: [`payload`] wraps a byte range of the window in a sub-header.

use crate::wire::WireError;

/// The sub-header's length.
pub const SUBHEADER_LEN: usize = 20;

/// A channel of the World Stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Channel {
    /// Channel A: the world (plane A), served by the engine host on `A.sock`.
    A,
    /// Channel C: sift matches (plane C), served by the sifter on `C.sock`.
    C,
}

impl Channel {
    /// The sub-header byte (`'A'` or `'C'`).
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Self::A => b'A',
            Self::C => b'C',
        }
    }

    /// The channel of a byte.
    #[must_use]
    pub const fn from_byte(b: u8) -> Option<Self> {
        match b {
            b'A' => Some(Self::A),
            b'C' => Some(Self::C),
            _ => None,
        }
    }

    /// `"A"` or `"C"`, as the control messages spell it.
    #[must_use]
    pub const fn letter(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::C => "C",
        }
    }
}

/// The canonical format's kind table (`canonical_v0.py` `CHANNEL`), copied here because this crate
/// depends on no workspace crate; the conformance test checks it against the canonical golden file.
/// A later canonical version may add kinds to either channel without a wire change.
pub const KIND_CHANNEL: &[(&str, Channel)] = &[
    ("EVENT", Channel::A),
    ("INPUT", Channel::A),
    ("KEYFRAME", Channel::A),
    ("LIFE_ENTRY", Channel::A),
    ("LIFE_EXIT", Channel::A),
    ("MATCH", Channel::C),
    ("STEP_DIGEST", Channel::A),
];

/// The channel of a canonical kind name, if the format has the kind.
#[must_use]
pub fn kind_channel(kind: &str) -> Option<Channel> {
    KIND_CHANNEL
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, c)| *c)
}

/// A decoded RECORDS payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordsFrame {
    /// The records' timeline.
    pub timeline_id: u32,
    /// Their channel.
    pub channel: Channel,
    /// The seq of the first record.
    pub first_seq: u64,
    /// The lines, without their LF.
    pub lines: Vec<String>,
}

/// The longest canonical integer token ("-9223372036854775808" and "18446744073709551615").
const INT_MAX_CHARS: usize = 20;

/// Whether `tok` is a canonical integer: `0|-?[1-9][0-9]*`.
fn canonical_int(tok: &str) -> bool {
    let digits = tok.strip_prefix('-').unwrap_or(tok);
    match digits.as_bytes() {
        [b'0'] => !tok.starts_with('-'),
        [b'1'..=b'9', rest @ ..] => rest.iter().all(u8::is_ascii_digit),
        _ => false,
    }
}

fn int_token(tok: &str, lo: i128, hi: i128, what: &str) -> Result<i128, WireError> {
    if tok.len() > INT_MAX_CHARS || !canonical_int(tok) {
        return Err(WireError::frame(format!(
            "{what} '{tok}' is not a canonical integer"
        )));
    }
    let value: i128 = tok
        .parse()
        .map_err(|_| WireError::frame(format!("{what} '{tok}' is not a canonical integer")))?;
    if value < lo || value > hi {
        return Err(WireError::frame(format!(
            "{what} {value} is outside [{lo}, {hi}]"
        )));
    }
    Ok(value)
}

/// Checks a line's frozen prefix `<KIND> <tl> <seq> <tick> ` and returns its kind.
///
/// # Errors
/// [`WireError`] (`invalid_frame`) naming the check that failed.
pub fn check_prefix(
    line: &str,
    channel: Channel,
    timeline_id: u32,
    seq: u64,
) -> Result<&str, WireError> {
    if line.is_empty() || !line.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return Err(WireError::frame(format!(
            "seq {seq}: a record line is empty or not printable ASCII"
        )));
    }
    let tokens: Vec<&str> = line.splitn(5, ' ').collect();
    let [kind, tl, sq, tick, ..] = tokens.as_slice() else {
        return Err(WireError::frame(format!(
            "seq {seq}: a record line needs the prefix <KIND> <tl> <seq> <tick>"
        )));
    };
    if kind_channel(kind) != Some(channel) {
        return Err(WireError::frame(format!(
            "seq {seq}: kind '{kind}' is not a channel-{} kind",
            channel.letter()
        )));
    }
    if int_token(tl, 0, i128::from(u32::MAX), "timeline")? != i128::from(timeline_id) {
        return Err(WireError::frame(format!(
            "seq {seq}: the line's timeline {tl} is not {timeline_id}"
        )));
    }
    if int_token(sq, 0, i128::from(u64::MAX), "seq")? != i128::from(seq) {
        return Err(WireError::frame(format!(
            "seq {seq}: the line's seq is {sq}"
        )));
    }
    int_token(tick, i128::from(i64::MIN), i128::from(i64::MAX), "tick")?;
    Ok(kind)
}

/// A RECORDS payload from a ready body (what a serving thread does: bytes in, bytes out, no parse).
///
/// # Errors
/// [`WireError`] if the payload would exceed the 1 MiB frame limit.
pub fn payload(
    timeline_id: u32,
    channel: Channel,
    first_seq: u64,
    count: u32,
    body: &[u8],
) -> Result<Vec<u8>, WireError> {
    let len = SUBHEADER_LEN + body.len();
    if len > crate::wire::MAX_PAYLOAD {
        return Err(WireError::frame(format!(
            "a RECORDS payload of {len} bytes exceeds {}",
            crate::wire::MAX_PAYLOAD
        )));
    }
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(&timeline_id.to_le_bytes());
    out.extend_from_slice(&first_seq.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.push(channel.byte());
    out.extend_from_slice(&[0, 0, 0]);
    out.extend_from_slice(body);
    Ok(out)
}

/// The payload of `records`, checked as a receiver checks it.
///
/// # Errors
/// [`WireError`] for no line, a seq past `u64`, a bad prefix, or a payload over the limit.
pub fn encode(records: &RecordsFrame) -> Result<Vec<u8>, WireError> {
    if records.lines.is_empty() {
        return Err(WireError::frame(
            "a RECORDS frame holds at least one record (count is 0)",
        ));
    }
    let n = u64::try_from(records.lines.len()).unwrap_or(u64::MAX);
    if records.first_seq.checked_add(n - 1).is_none() {
        return Err(WireError::frame("the frame's last seq exceeds u64"));
    }
    let count =
        u32::try_from(records.lines.len()).map_err(|_| WireError::frame("count exceeds u32"))?;
    let mut body = Vec::new();
    for (i, line) in records.lines.iter().enumerate() {
        let seq = records.first_seq + u64::try_from(i).unwrap_or(u64::MAX);
        check_prefix(line, records.channel, records.timeline_id, seq)?;
        body.extend_from_slice(line.as_bytes());
        body.push(b'\n');
    }
    payload(
        records.timeline_id,
        records.channel,
        records.first_seq,
        count,
        &body,
    )
}

/// The records of a RECORDS payload.
///
/// # Errors
/// [`WireError`] (`invalid_frame`) naming the first check that fails.
pub fn decode(payload: &[u8]) -> Result<RecordsFrame, WireError> {
    let Some(head) = payload.get(..SUBHEADER_LEN) else {
        return Err(WireError::frame(format!(
            "a RECORDS payload needs {SUBHEADER_LEN} bytes of sub-header"
        )));
    };
    let u32_at = |a: usize| u32::from_le_bytes([head[a], head[a + 1], head[a + 2], head[a + 3]]);
    let timeline_id = u32_at(0);
    let mut seq8 = [0u8; 8];
    seq8.copy_from_slice(&head[4..12]);
    let first_seq = u64::from_le_bytes(seq8);
    let count = u32_at(12);
    if head[17..20] != [0, 0, 0] {
        return Err(WireError::frame("the reserved bytes are not zero"));
    }
    let Some(channel) = Channel::from_byte(head[16]) else {
        return Err(WireError::frame(format!(
            "channel byte {:#x} is not 'A' or 'C'",
            head[16]
        )));
    };
    if count == 0 {
        return Err(WireError::frame("count is 0"));
    }
    let body = &payload[SUBHEADER_LEN..];
    let Some(inner) = body.strip_suffix(b"\n") else {
        return Err(WireError::frame("the body does not end with LF"));
    };
    let raw: Vec<&[u8]> = inner.split(|&b| b == b'\n').collect();
    if raw.len() != usize::try_from(count).unwrap_or(usize::MAX) {
        return Err(WireError::frame(format!(
            "count says {count} records, the body holds {}",
            raw.len()
        )));
    }
    let lines = raw
        .iter()
        .map(|part| {
            if part.is_ascii() {
                Ok(String::from_utf8_lossy(part).into_owned())
            } else {
                Err(WireError::frame("a record line is not ASCII"))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let records = RecordsFrame {
        timeline_id,
        channel,
        first_seq,
        lines,
    };
    encode(&records)?;
    Ok(records)
}
