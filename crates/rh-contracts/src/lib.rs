//! `rh-contracts`: column types and read accessors, one module per owning system.
//!
//! Plane A, contract crate. It holds no implementation: systems read columns they do not own
//! through these accessors and request changes through lifecycle commands (single writer).
//! Changing it is a C2 change (docs/13 §4.2).
//!
//! - [`m2`]: `M2.demography`'s columns, `person.*` and `life_course.*` (SPR-014 T-06).
//!
//! Each accessor reads through the caller's [`rh_core::world::SystemView`] (its keyed lookup,
//! [`rh_core::world::SystemView::value`]), so the caller's `AccessSpec` applies, a lagged read sees
//! its lag, and a read registration dropped because its owner is absent gives the declared neutral
//! (`05.1` §2 rule 2). `tests/drift.rs` compares every accessor's column, type and enum codes
//! with the owner's spec.
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; `docs/05_architecture/05.1_simulation_core.md`
//! §2; `docs/13` §4.1, §4.3; `CODEX/05_PROJECT/SPR-014_LifecycleQueues.md` T-06.
#![forbid(unsafe_code)]

pub mod m2;
