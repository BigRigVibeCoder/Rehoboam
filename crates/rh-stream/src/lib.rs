//! `rh-stream`: World Stream v0, channel A, and the sealed chronicle (SPR-013 T-09; SPR-011
//! ruling 13). Plane A, machinery: the one writer of channel A (`05.10` §7 item 1), so the
//! forbidden-API gate scans it.
//!
//! - [`record`]: the record header and kinds of `specs/stream/channel_a_v0.sql` (and channel C's
//!   `MATCH`), and the in-instant order (R4).
//! - [`canonical`]: the lossless one-line canonical text form, its parser, `canonical_digest`
//!   and `MatchId` (`specs/stream/canonical_v0.md`).
//! - [`parquet_io`]: typed tables to Parquet bytes and back, with the writer properties pinned.
//! - [`segment`]: a sim-year's records to the files of a channel-A segment and back, with every
//!   check a reader can make.
//! - [`seal`]: atomic sealing on disk and reading back; the one file allowed file I/O.
//! - [`convert`]: the drained events, the applied journal rows and the life-course delta as
//!   records.
//! - [`live`]: the live edge (SPR-026): one `ArcSwap` snapshot of the window, read by the
//!   socket host's serving threads, published by the writer and its seal thread.
//! - [`writer`]: the per-timeline writer the host feeds after each instant; resume from a
//!   keyframe; the reader of a timeline's sealed segments.
//!
//! No float anywhere (R2): every column is an integer, a hash, a blob or a string.
//!
//! REF: `docs/05_architecture/05.10_world_stream.md` §4, §5, §7; `05.9` IF-0;
//! `CODEX/05_PROJECT/SPR-013_WalkingSkeletonC.md` T-09; SPR-011 rulings 11–13 and 17.
#![forbid(unsafe_code)]

use core::fmt;

pub mod canonical;
pub mod convert;
pub mod live;
pub mod parquet_io;
pub mod record;
pub mod seal;
pub mod segment;
pub mod writer;

/// Why the stream could not be written or read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamError {
    /// A canonical line or file (`canonical`).
    Canonical(canonical::CanonicalError),
    /// Parquet failed.
    Parquet(String),
    /// A table's columns or a file's schema did not match its spec.
    Table(String),
    /// A segment's records, rows or files are inconsistent.
    Segment(String),
    /// The dictionaries are malformed.
    Dictionary(String),
    /// A record would break R4's order.
    Order(String),
    /// An engine output could not be turned into a record.
    Convert(String),
    /// File I/O.
    Io(String),
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canonical(e) => write!(f, "canonical form: {e}"),
            Self::Parquet(e) => write!(f, "parquet: {e}"),
            Self::Table(e) => write!(f, "table: {e}"),
            Self::Segment(e) => write!(f, "segment: {e}"),
            Self::Dictionary(e) => write!(f, "dictionary: {e}"),
            Self::Order(e) => write!(f, "order (R4): {e}"),
            Self::Convert(e) => write!(f, "record: {e}"),
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for StreamError {}

impl From<canonical::CanonicalError> for StreamError {
    fn from(e: canonical::CanonicalError) -> Self {
        Self::Canonical(e)
    }
}

/// The planted-control features compiled into this build (none in a real one; BCK-002 A-52).
pub const MUTANTS: &[&str] = &[
    #[cfg(feature = "mutant-window-gap")]
    "mutant-window-gap",
];
