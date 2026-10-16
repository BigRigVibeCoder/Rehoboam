//! `rh-snapshot`: keyframes, the plane hashes and input-journal I/O.
//!
//! Plane A, machinery. History is `f(engine_build, registry_version, manifest_hash, seed,
//! input_journal)`; this crate stores and checks that function's inputs and outputs. Plane B and
//! plane C crates may call its hashing; it never depends on them.
//!
//! - [`hash`]: the three plane hashes, chunked Merkle XXH3-64 with dirty tracking (T-01).
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.2_world_model_and_data.md`;
//! `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md`.
//!
//! Stub created by SPR-001 (repository bootstrap); SPR-007 fills it.
#![forbid(unsafe_code)]

pub mod hash;

pub use hash::{Plane, state_hash};
