//! `rh-sifter` v0: the story sifter, downstream of the engine (SPR-011 ruling 14; SPR-013 T-11).
//!
//! Plane C, machinery. It reads sealed channel-A segments in `seq` order and writes channel-C
//! segments of sift matches; it never runs inside the engine process (the live sifter on a
//! worker thread is P2.1), and no plane-A crate depends on it (V-PLN-01, `bin/lint_arch.py`).
//!
//! - [`pattern`]: pattern packs, DSL v0, single-clause patterns only.
//! - [`sift`]: matching, channel-C segments and their reader.
//!
//! REF: `docs/05_architecture/05.4_narrative_intelligence.md` §2; `specs/stream/channel_c_v0.sql`;
//! ADR-0024; `CODEX/05_PROJECT/SPR-013_WalkingSkeletonC.md` T-11.
#![forbid(unsafe_code)]

use core::fmt;

pub mod pattern;
pub mod sift;

/// Why a sift failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SiftError {
    /// A pattern pack does not compile for v0.
    Pattern(String),
    /// A segment could not be read or written.
    Segment(String),
    /// File I/O.
    Io(String),
}

impl fmt::Display for SiftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pattern(e) => write!(f, "pattern: {e}"),
            Self::Segment(e) => write!(f, "segment: {e}"),
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for SiftError {}
