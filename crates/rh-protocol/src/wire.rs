//! Framing (`wire_v1.md` §2): a 12-byte header, then `length` bytes of payload.
//!
//! ```text
//! offset  size  field     value
//! 0       4     magic     "RHWS"
//! 4       2     version   u16 = 1
//! 6       1     type      u8: 1 = CONTROL, 2 = RECORDS
//! 7       1     flags     u8: 0 in v1 (a nonzero value is refused)
//! 8       4     length    u32: 0 <= length <= 1 MiB (4 KiB from a viewer)
//! ```
//!
//! Little-endian. [`decode`] judges the header before the payload, so a frame over its limit is
//! refused on its 12 header bytes, and a short buffer is [`WireError::Incomplete`]: a reader
//! accumulates and decodes again.

use core::fmt;
use core::fmt::Write as _;

/// The magic of every frame (frozen across versions).
pub const MAGIC: [u8; 4] = *b"RHWS";
/// The version this crate speaks (frozen position, `rehoboam/stream/wire/v1`).
pub const VERSION: u16 = 1;
/// The header's length.
pub const HEADER_LEN: usize = 12;
/// The largest payload of any frame (1 MiB).
pub const MAX_PAYLOAD: usize = 1024 * 1024;
/// The largest payload of a frame from a viewer (4 KiB; REV-SPR-026 R9).
pub const VIEWER_MAX_PAYLOAD: usize = 4096;

/// A frame's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameType {
    /// One JSON-RPC object in canonical JSON (§2.1).
    Control,
    /// A records sub-header and canonical lines (§2.2).
    Records,
}

impl FrameType {
    /// The type byte.
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Self::Control => 1,
            Self::Records => 2,
        }
    }

    /// The type of a byte.
    #[must_use]
    pub const fn from_byte(b: u8) -> Option<Self> {
        match b {
            1 => Some(Self::Control),
            2 => Some(Self::Records),
            _ => None,
        }
    }
}

/// One frame: its type and payload (the header is derived).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// The type.
    pub ftype: FrameType,
    /// The payload.
    pub payload: Vec<u8>,
}

/// What kind of refusal a [`WireError`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireErrorKind {
    /// The buffer ends before the header or the frame does: read more and decode again.
    Incomplete,
    /// The frame's version is not 1 (`unsupported_version`).
    UnsupportedVersion,
    /// A bad magic, type, flag, length, sub-header or line (`invalid_frame`).
    Frame,
    /// The payload is not JSON (`parse_error`).
    Parse,
    /// Not a JSON-RPC 2.0 object of this wire (`invalid_request`).
    Request,
    /// A parameter or a float (`invalid_params`).
    Params,
}

/// A frame, payload or message the wire refuses (§2, §3, §4), with a message naming why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireError {
    /// The refusal's kind.
    pub kind: WireErrorKind,
    /// The message (it names the reason).
    pub message: String,
}

impl WireError {
    /// A refusal of `kind`.
    #[must_use]
    pub fn new(kind: WireErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// An `invalid_frame` refusal.
    #[must_use]
    pub fn frame(message: impl Into<String>) -> Self {
        Self::new(WireErrorKind::Frame, message)
    }

    /// Whether the buffer was only short.
    #[must_use]
    pub fn is_incomplete(&self) -> bool {
        self.kind == WireErrorKind::Incomplete
    }
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for WireError {}

/// Python's `repr` of a byte string, as the reference's messages print it (`b'RHWX'`).
#[must_use]
pub fn bytes_repr(b: &[u8]) -> String {
    let mut s = String::from("b'");
    for &c in b {
        match c {
            b'\'' => s.push_str("\\'"),
            b'\\' => s.push_str("\\\\"),
            b'\t' => s.push_str("\\t"),
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(char::from(c)),
            _ => {
                let _ = write!(s, "\\x{c:02x}");
            }
        }
    }
    s.push('\'');
    s
}

/// The bytes of `frame`: the header, then the payload.
///
/// # Errors
/// [`WireError`] if the payload exceeds [`MAX_PAYLOAD`].
pub fn encode(frame: &Frame) -> Result<Vec<u8>, WireError> {
    if frame.payload.len() > MAX_PAYLOAD {
        return Err(WireError::frame(format!(
            "payload of {} bytes exceeds {MAX_PAYLOAD}",
            frame.payload.len()
        )));
    }
    let length = u32::try_from(frame.payload.len())
        .map_err(|_| WireError::frame("a payload length that is not a u32"))?;
    let mut out = Vec::with_capacity(HEADER_LEN + frame.payload.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.push(frame.ftype.byte());
    out.push(0);
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&frame.payload);
    Ok(out)
}

/// The type and payload length of the header at the start of `buf`, refusing a bad one with a
/// payload limit of `limit` (in this order: magic, version, type, flags, length).
///
/// # Errors
/// [`WireError`]: [`WireErrorKind::Incomplete`] for fewer than 12 bytes; otherwise the refusal.
pub fn decode_header(buf: &[u8], limit: usize) -> Result<(FrameType, usize), WireError> {
    let Some(head) = buf.get(..HEADER_LEN) else {
        return Err(WireError::new(
            WireErrorKind::Incomplete,
            format!("{} bytes, a header needs {HEADER_LEN}", buf.len()),
        ));
    };
    let field = |a: usize, b: usize| head.get(a..b).unwrap_or_default();
    let magic = field(0, 4);
    if magic != MAGIC {
        return Err(WireError::frame(format!(
            "magic {} is not {}",
            bytes_repr(magic),
            bytes_repr(&MAGIC)
        )));
    }
    let version = u16::from_le_bytes([head[4], head[5]]);
    if version != VERSION {
        return Err(WireError::new(
            WireErrorKind::UnsupportedVersion,
            format!("unsupported_version: {version}, this host speaks {VERSION}"),
        ));
    }
    let Some(ftype) = FrameType::from_byte(head[6]) else {
        return Err(WireError::frame(format!(
            "frame type {} is not CONTROL (1) or RECORDS (2)",
            head[6]
        )));
    };
    let flags = head[7];
    if flags != 0 && !cfg!(feature = "mutant-accept-flags") {
        return Err(WireError::frame(format!(
            "flags {flags:#x}: every flag is reserved in v1"
        )));
    }
    let length = u32::from_le_bytes([head[8], head[9], head[10], head[11]]);
    let length = usize::try_from(length).unwrap_or(usize::MAX);
    if length > limit {
        return Err(WireError::frame(format!("length {length} exceeds {limit}")));
    }
    Ok((ftype, length))
}

/// The frame at the start of `buf` and the bytes it used.
///
/// # Errors
/// [`WireError`]: [`WireErrorKind::Incomplete`] when the buffer ends before the frame does.
pub fn decode(buf: &[u8], limit: usize) -> Result<(Frame, usize), WireError> {
    let (ftype, length) = decode_header(buf, limit)?;
    let end = HEADER_LEN + length;
    let Some(payload) = buf.get(HEADER_LEN..end) else {
        return Err(WireError::new(
            WireErrorKind::Incomplete,
            format!("{} bytes, the frame needs {end}", buf.len()),
        ));
    };
    Ok((
        Frame {
            ftype,
            payload: payload.to_vec(),
        },
        end,
    ))
}
