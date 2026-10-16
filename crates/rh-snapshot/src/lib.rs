//! `rh-snapshot`: keyframes, the plane hashes and input-journal I/O.
//!
//! Plane A, machinery. History is `f(engine_build, registry_version, manifest_hash, seed,
//! input_journal)`; this crate stores and checks that function's inputs and outputs. Plane B and
//! plane C crates may call its hashing; it never depends on them.
//!
//! - [`hash`]: the three plane hashes, chunked Merkle XXH3-64 with dirty tracking (T-01).
//! - [`journal`]: the `SQLite` (WAL) input journal of push inputs and its hard-failing replay (T-03).
//! - [`keyframe`]: keyframes, Arrow IPC with zstd, written from and loaded into an engine (T-02).
//! - [`state`]: `state_hash` over all plane-A state, world columns and engine tables (T-02).
//! - [`store`]: the keyframe store's policy, a permanent 10-year cadence and a yearly LRU (T-02).
//! - [`guard`]: the unsynced guard, a plane-A hash check around every read-only call in debug builds (T-04).
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.2_world_model_and_data.md`;
//! `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md`.
//!
//! Stub created by SPR-001 (repository bootstrap); SPR-007 fills it.
#![forbid(unsafe_code)]

pub mod guard;
pub mod hash;
pub mod journal;
pub mod keyframe;
pub mod state;
pub mod store;

pub use hash::{Plane, tables_hash};
pub use state::state_hash;
