//! `rh-protocol`: World Stream wire format v1, the live edge of IF-0 (`specs/stream/wire_v1.md`;
//! SPR-026 T-01).
//!
//! - [`wire`]: the 12-byte frame header, the 1 MiB and 4 KiB limits, [`wire::decode`] returning
//!   the bytes consumed or `Incomplete`.
//! - [`records`]: the RECORDS sub-header and the frozen-prefix checks of a canonical line, and
//!   the payload a serving thread builds from a byte range without a parse.
//! - [`json`]: canonical JSON, and a parser that refuses floats, repeated keys, integers outside
//!   `i64..u64`, non-ASCII strings and nesting deeper than 4.
//! - [`control`]: the JSON-RPC messages, the method table of exactly three reads, the error
//!   codes, the notifications, and the host's dispatcher.
//! - [`cursor`]: `Catchup`, the viewer's catch-up state machine.
//!
//! A leaf (rule `protocol-leaf`): no workspace crate, `serde` and `serde_json` only, no socket and
//! no async runtime. The host owns the socket (`rh-cli`'s `serve.rs`); plane A cannot name a frame.
//!
//! REF: `specs/stream/wire_v1.md` §2–§7; `specs/stream/selftest_wire_v1.py` (the reference);
//! `specs/stream/golden/wire_v1.txt`; REV-SPR-026 R6–R11, R15, R21, R24.
#![forbid(unsafe_code)]

pub mod control;
pub mod cursor;
pub mod json;
pub mod records;
pub mod wire;

/// The planted-control features compiled into this build (none in a real one; BCK-002 A-52).
pub const MUTANTS: &[&str] = &[
    #[cfg(feature = "mutant-pace-method")]
    "mutant-pace-method",
    #[cfg(feature = "mutant-accept-flags")]
    "mutant-accept-flags",
];
