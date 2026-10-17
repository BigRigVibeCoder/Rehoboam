//! What the smoke systems share: the world's size, person ids and Q16.16 column helpers.

use rh_core::fixed::Q16_16;
use rh_core::system::SimError;
use rh_ids::rng::{h64_words, tag};
use rh_ids::{EntityRef, PersonId, RegionId};

/// Person rows.
pub const PERSONS: usize = 64;
/// Region rows.
pub const REGIONS: usize = 4;

const TAG_PERSON: u64 = tag("rehoboam/rh-sys-smoke/person/v1");

/// The `PersonId` of person row `row`: hashed, so `PersonId` order is not row order (the command
/// queues order by `PersonId`).
#[must_use]
pub fn person(row: usize) -> PersonId {
    PersonId::from_raw(h64_words(TAG_PERSON, &[row as u64]) >> 4)
}

/// The entity of person row `row`.
#[must_use]
pub fn person_entity(row: usize) -> EntityRef {
    EntityRef::person(person(row))
}

/// The `RegionId` of region row `row`.
pub(crate) fn region(row: usize) -> RegionId {
    RegionId::from_raw(u16::try_from(row).unwrap_or(u16::MAX))
}

/// The key of world-level draws: a region id no region row has.
pub(crate) const WORLD: RegionId = RegionId::from_raw(u16::MAX);

/// `n` whole units as the Q16.16 bits a column stores.
pub(crate) const fn degrees(n: i16) -> i64 {
    Q16_16::from_int(n).to_bits() as i64
}

/// A column's row as Q16.16. Rows hold Q16.16 bits; anything else is a broken table.
pub(crate) fn fixed(system: &'static str, bits: i64) -> Result<Q16_16, SimError> {
    i32::try_from(bits)
        .map(Q16_16::from_bits)
        .map_err(|_| SimError::System {
            system,
            message: format!("{bits} is not Q16.16 bits"),
        })
}

/// A row index from a payload or command argument.
pub(crate) fn row(system: &'static str, value: i64, rows: usize) -> Result<usize, SimError> {
    usize::try_from(value)
        .ok()
        .filter(|&r| r < rows)
        .ok_or_else(|| SimError::System {
            system,
            message: format!("row {value} is out of range 0..{rows}"),
        })
}

/// `r` as an `i64` payload.
pub(crate) fn r64(r: usize) -> i64 {
    i64::try_from(r).unwrap_or(i64::MAX)
}

/// A failed invariant or check, as a step error.
pub(crate) fn fail(system: &'static str, message: String) -> SimError {
    SimError::System { system, message }
}

/// Implements `SpecPurpose` for the generated `RngPurpose` of the module it is invoked in, with
/// the generated `name()` (SPR-005 ruling 2). Only the variant list is written by hand, and
/// registration checks it against the spec's `[[rng]]`.
macro_rules! purposes {
    ($($variant:ident),* $(,)?) => {
        impl rh_core::system::SpecPurpose for RngPurpose {
            const ALL: &'static [Self] = &[$(Self::$variant),*];
            fn name(&self) -> &'static str {
                RngPurpose::name(*self)
            }
        }
    };
}
pub(crate) use purposes;

/// The `SystemModel` items every smoke system takes from its generated skeleton: the descriptor
/// (`SYSTEM_ID`, `SPEC_VERSION`), the bindings (`SCHEDULE`), the purposes (`RngPurpose`), the role
/// constants (`ROLES`, SPR-020) and no
/// parameters or aggregates.
macro_rules! from_skeleton {
    () => {
        fn descriptor(&self) -> rh_core::system::SystemDescriptor {
            rh_core::system::SystemDescriptor {
                id: SYSTEM_ID,
                fidelity: "F0",
                spec_version: SPEC_VERSION,
            }
        }
        fn schedule(&self) -> Vec<rh_core::system::PulseBinding> {
            rh_core::system::PulseBinding::from_schedule(SCHEDULE)
        }
        fn rng_purposes(&self) -> Vec<&'static str> {
            <RngPurpose as rh_core::system::SpecPurpose>::names()
        }
        fn roles(&self) -> &'static [(&'static str, rh_core::RoleId)] {
            ROLES
        }
        fn params_schema(&self) -> rh_core::system::ParamSchema {
            rh_core::system::ParamSchema::default()
        }
        fn declared_aggregates(&self) -> Vec<rh_core::system::AggregateSpec> {
            Vec::new()
        }
    };
}
pub(crate) use from_skeleton;

/// One invariant result.
pub(crate) fn check(
    name: &'static str,
    holds: bool,
    detail: impl FnOnce() -> String,
) -> rh_core::system::InvariantResult {
    rh_core::system::InvariantResult {
        name,
        holds,
        detail: if holds { String::new() } else { detail() },
    }
}
