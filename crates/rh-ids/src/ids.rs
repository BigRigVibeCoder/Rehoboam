//! Typed identifiers (SPR-002 T-01).
//!
//! Every identifier is a newtype over a fixed-width unsigned integer. There is no `From`/`Into`
//! between an identifier and an integer, nor between two identifier types: the only crossings are
//! the explicit `from_raw` and `to_raw`, so a `RegionId` can never be passed where a `PersonId` is
//! expected, and arithmetic on an identifier does not compile.
//!
//! The widths follow the plane-A ledger (`05.1` §5: `person_id u64`, `region u16`) and the input
//! journal (`05.9` IF-2: `timeline_id u32`).
//!
//! REF: `CODEX/05_PROJECT/SPR-002_IdsAndRng.md` T-01; ADR-0026 (genealogical `PersonId`);
//! ADR-0025 (`EventId` order is emission order); `05.9` IF-1, IF-2.
//!
//! # Compile-fail tests (T-01 acceptance)
//!
//! Each `compile_fail` case below is paired with a twin that differs only in the line under test
//! and must compile. Stable rustdoc does not check the error code of a `compile_fail` test, so the
//! twin is what shows that the case fails for the reason it claims (a typo would fail both).
//!
//! Mixing identifier types is a compile error:
//!
//! ```compile_fail,E0308
//! use rh_ids::{PersonId, RegionId};
//! fn takes_person(_: PersonId) {}
//! takes_person(RegionId::from_raw(7));
//! ```
//!
//! ```
//! use rh_ids::{PersonId, RegionId};
//! fn takes_person(_: PersonId) {}
//! takes_person(PersonId::from_raw(7));
//! ```
//!
//! An integer is not an identifier:
//!
//! ```compile_fail,E0308
//! use rh_ids::PersonId;
//! fn takes_person(_: PersonId) {}
//! takes_person(7_u64);
//! ```
//!
//! ```
//! use rh_ids::PersonId;
//! fn takes_person(_: PersonId) {}
//! takes_person(PersonId::from_raw(7_u64));
//! ```
//!
//! nor is an identifier an integer:
//!
//! ```compile_fail,E0308
//! use rh_ids::EventId;
//! let raw: u64 = EventId::from_raw(1);
//! ```
//!
//! ```
//! use rh_ids::EventId;
//! let raw: u64 = EventId::from_raw(1).to_raw();
//! ```
//!
//! and there is no implicit conversion from an integer or from another identifier:
//!
//! ```compile_fail,E0277
//! use rh_ids::MatchId;
//! let id: MatchId = 3_u64.into();
//! ```
//!
//! ```
//! use rh_ids::MatchId;
//! let id: MatchId = MatchId::from_raw(3_u64);
//! ```
//!
//! The only `From` an identifier has is the reflexive `From<T> for T`, so converting another
//! identifier is a type mismatch (E0308), not a missing impl:
//!
//! ```compile_fail,E0308
//! use rh_ids::{PersonId, TimelineId};
//! let id: TimelineId = TimelineId::from(PersonId::from_raw(1));
//! ```
//!
//! ```
//! use rh_ids::{PersonId, TimelineId};
//! let id: TimelineId = TimelineId::from(TimelineId::from_raw(1));
//! ```
//!
//! Identifiers do not support arithmetic:
//!
//! ```compile_fail,E0369
//! use rh_ids::PersonId;
//! let next = PersonId::from_raw(1) + PersonId::from_raw(1);
//! ```
//!
//! ```
//! use rh_ids::PersonId;
//! let next = PersonId::from_raw(1).to_raw() + PersonId::from_raw(1).to_raw();
//! ```
//!
//! Comparing identifiers of different types does not compile:
//!
//! ```compile_fail,E0308
//! use rh_ids::{MatchId, PersonId};
//! let same = PersonId::from_raw(1) == MatchId::from_raw(1);
//! ```
//!
//! ```
//! use rh_ids::{MatchId, PersonId};
//! let same = PersonId::from_raw(1) == PersonId::from_raw(1);
//! ```

use core::fmt;

/// Declares an identifier newtype with explicit raw conversions only.
macro_rules! typed_id {
    ($(#[$doc:meta])* $name:ident($raw:ty)) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($raw);

        impl $name {
            /// Wraps a raw value. Explicit by design: there is no `From` conversion.
            #[must_use]
            pub const fn from_raw(raw: $raw) -> Self {
                Self(raw)
            }

            /// Returns the raw value. Explicit by design: there is no `Into` conversion.
            #[must_use]
            pub const fn to_raw(self) -> $raw {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({:#x})"), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:#x}", self.0)
            }
        }
    };
}

typed_id! {
    /// A person, for life (ADR-0026): a genealogical hash of mother and parity, so the same person
    /// has the same identifier in every branch. New ones are derived by [`crate::person`].
    PersonId(u64)
}

typed_id! {
    /// A region of the region graph (`05.1` §5; the ledger's `region u16` column).
    RegionId(u16)
}

typed_id! {
    /// An event in the chronicle. Allocated by the emitter in emission order, so the order of
    /// `EventId`s is the emission order `(tick, phase, seq)` (ADR-0025).
    EventId(u64)
}

typed_id! {
    /// A sifting-pattern match (plane C; ADR-0008). Defined here so that every crate shares one
    /// identifier vocabulary.
    MatchId(u64)
}

typed_id! {
    /// A timeline: the canonical one, a fork, a Foresight rollout (`05.9` IF-2 `timeline_id: u32`).
    /// Allocated uniquely by the Timeline Manager, which is what makes salt lineages unique
    /// (`05.1` §6.9; [`crate::salt`]).
    TimelineId(u32)
}

typed_id! {
    /// A simulation system (`rh-sys-*`), the `system_id` field of the keyed RNG key (ADR-0006).
    SystemId(u32)
}

/// The `entity_id` field of the keyed RNG key: an explicit, lossless widening to `u64`.
///
/// Implemented for the identifier types an RNG draw can be keyed on. It is a named method, not a
/// conversion, so a call site always says which entity it keys on. Two identifier types may share
/// a raw value; the RNG purpose, which declares its entity kind (`[[rng]] key` in the system spec),
/// keeps their draws apart.
pub trait EntityKey {
    /// The value used as `entity_id` in `H(world_seed, branch_salt(t), system_id, epoch, purpose,
    /// entity_id, t, k)`.
    fn entity_key(self) -> u64;
}

impl EntityKey for PersonId {
    fn entity_key(self) -> u64 {
        self.0
    }
}

impl EntityKey for RegionId {
    fn entity_key(self) -> u64 {
        u64::from(self.0)
    }
}

impl EntityKey for EventId {
    fn entity_key(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_round_trip() {
        assert_eq!(PersonId::from_raw(u64::MAX).to_raw(), u64::MAX);
        assert_eq!(RegionId::from_raw(u16::MAX).to_raw(), u16::MAX);
        assert_eq!(EventId::from_raw(9).to_raw(), 9);
        assert_eq!(MatchId::from_raw(10).to_raw(), 10);
        assert_eq!(TimelineId::from_raw(11).to_raw(), 11);
        assert_eq!(SystemId::from_raw(12).to_raw(), 12);
    }

    #[test]
    fn order_is_the_raw_order() {
        // EventId order is emission order only because the emitter allocates in that order and
        // the derived Ord is the raw order (ADR-0025).
        assert!(EventId::from_raw(1) < EventId::from_raw(2));
        assert!(PersonId::from_raw(0) < PersonId::from_raw(u64::MAX));
    }

    #[test]
    fn entity_keys_are_lossless() {
        assert_eq!(RegionId::from_raw(0xBEEF).entity_key(), 0xBEEF);
        assert_eq!(PersonId::from_raw(u64::MAX).entity_key(), u64::MAX);
    }

    #[test]
    fn debug_names_the_type() {
        assert_eq!(format!("{:?}", RegionId::from_raw(255)), "RegionId(0xff)");
        assert_eq!(format!("{}", PersonId::from_raw(255)), "0xff");
    }
}
