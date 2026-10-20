//! `rh-ids`: typed identifiers, the keyed hash `H` and the counter-based RNG.
//!
//! Plane A, determinism machinery. Every random draw in the engine is keyed:
//! `H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)`; there is no other RNG.
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; ADR-0006 (determinism contract);
//! `CODEX/05_PROJECT/SPR-002_IdsAndRng.md`; `CLAUDE.md`.
//!
//! Created by SPR-001 (repository bootstrap); SPR-005 adds the RNG context; SPR-011 adds
//! epoch-keyed draws (T-02) and `PersonId::entity_key` (BCK-002 A-29); SPR-014 adds
//! `H64_MAX`; SPR-015 adds `HouseholdId`.
#![forbid(unsafe_code)]

pub mod entity;
pub mod hash;
pub mod household;
pub mod ids;
pub mod person;
pub mod rng;
pub mod salt;

pub use entity::{EntityKind, EntityRef};
pub use hash::{BranchSalt, PurposeId, RngKey, StreamKey, SystemKey, h};
pub use ids::{
    EntityKey, EventId, HouseholdId, MatchId, PersonId, PlaceId, RegionId, SystemId, TimelineId,
    WorldKey,
};
pub use person::{IdError, PersonIdTable};
pub use rng::{LightStream, Purpose, PurposeRegistry, RngStream};
pub use salt::{CANONICAL_SALT, ForkError, ForkPolicy, ForkStep, SaltLineage};
