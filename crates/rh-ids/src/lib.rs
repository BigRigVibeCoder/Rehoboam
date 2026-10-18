//! `rh-ids`: typed identifiers, the keyed hash `H` and the counter-based RNG.
//!
//! Plane A, determinism machinery. Every random draw in the engine is keyed:
//! `H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)`; there is no other RNG.
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; ADR-0006 (determinism contract);
//! `CODEX/05_PROJECT/SPR-002_IdsAndRng.md`; `CLAUDE.md`.
#![forbid(unsafe_code)]

pub mod entity;
pub mod hash;
pub mod ids;
pub mod person;
pub mod rng;
pub mod salt;

pub use entity::{EntityKind, EntityRef};
pub use hash::{BranchSalt, PurposeId, RngKey, StreamKey, SystemKey, h};
pub use ids::{EntityKey, EventId, MatchId, PersonId, RegionId, SystemId, TimelineId, WorldKey};
pub use person::{IdError, PersonIdTable};
pub use rng::{LightStream, Purpose, PurposeRegistry, RngStream};
pub use salt::{CANONICAL_SALT, ForkError, ForkPolicy, ForkStep, SaltLineage};
