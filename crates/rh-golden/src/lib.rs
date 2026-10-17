//! `rh-golden`: golden runs (SPR-008). A tool around the engine, never linked into plane A.
//!
//! - [`run`]: the smoke runner. A full run journals its synthetic material inputs in SPR-007's
//!   host order and writes keyframes; a resumed run goes through `rh_snapshot::resume`. Both
//!   write a hash ladder (T-01, T-02).
//! - [`ladder`]: the portable ladder format `rehoboam-ladder v1` and its comparison (ruling 4).
//! - [`dump`]: a whole plane-A state as text, for the bisect's row-level step.
//! - [`bisect`]: bisect v0, which names the first divergent month, table, row and column between
//!   two builds (T-03, ruling 7; V-DET-11).
//! - [`leak`]: the planted thread-identity leak behind `--leak-thread` (DEF-2 control).
//! - [`cli`]: the command-line form `rh-cli` exposes and spawned builds read.
//! - [`stream`]: the World Stream of a run (SPR-013 T-09): feeding `rh-stream`'s writer, the run
//!   directory.
//! - [`sift`]: `rh-cli sift`, channel C from sealed channel A (SPR-013 T-11).
//! - [`tail`]: `rh-cli tail`, the sealed stream as display or canonical lines (SPR-013 T-10).
//!
//! The baselines are ladders this crate writes, never typed; `bin/check_baseline.py` gates them
//! (T-04, ruling 6).
//!
//! REF: `CODEX/05_PROJECT/SPR-008_GoldenRunCI.md`; `docs/13` §4.3–4.5; `05.1` §6 item 6.
#![forbid(unsafe_code)]

pub mod bisect;
pub mod cli;
pub mod dump;
pub mod ladder;
pub mod leak;
pub mod replay;
pub mod run;
pub mod sha256;
pub mod sift;
pub mod stream;
pub mod tail;
pub mod vitals;
pub mod world;
