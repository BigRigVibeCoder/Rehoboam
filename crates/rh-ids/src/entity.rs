//! A typed entity key with a total order: the entity kind first, then the raw id (SPR-006).
//!
//! The agenda key `(time, phase, entity, seq)` (`05.1` §4.3) and a material input's dependency
//! scope (ADR-0025 item 3) both name an entity of any kind. [`EntityRef`] is that name: an
//! [`EntityKind`] tag and the id's integer value. Its derived order compares the tag first, then
//! the value, so two entities of different kinds never compare equal even when their raw values
//! coincide, and the order never depends on how the id was produced (SPR-006 ruling 2).
//!
//! The tag values are part of the determinism contract (they order agenda keys): changing one is
//! a C3 change.
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §4.3; ADR-0025 item 3;
//! `CODEX/05_PROJECT/SPR-006_AgendaEventsInvariants.md` ruling 2.

use core::fmt;

use crate::ids::{PersonId, RegionId};

/// The kind of an entity, in tag order. The discriminant is the tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EntityKind {
    /// No particular entity: the world as a whole (raw value 0 by convention).
    World = 0,
    /// A person ([`PersonId`]).
    Person = 1,
    /// A household (ADR-0025 scope; no typed id yet).
    Household = 2,
    /// A settlement (ADR-0025 scope; no typed id yet).
    Settlement = 3,
    /// A region ([`RegionId`]).
    Region = 4,
}

impl EntityKind {
    /// Every kind, in tag order.
    pub const ALL: [Self; 5] = [
        Self::World,
        Self::Person,
        Self::Household,
        Self::Settlement,
        Self::Region,
    ];

    /// The tag.
    #[must_use]
    pub const fn tag(self) -> u8 {
        self as u8
    }

    /// The kind with this tag, if any.
    #[must_use]
    pub fn from_tag(tag: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.tag() == tag)
    }

    /// The lower-case name used in text dumps.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::World => "world",
            Self::Person => "person",
            Self::Household => "household",
            Self::Settlement => "settlement",
            Self::Region => "region",
        }
    }
}

/// An entity of any kind: the kind tag, then the raw id. Ordered by `(kind, raw)`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityRef {
    kind: EntityKind,
    raw: u64,
}

impl EntityRef {
    /// The world as a whole.
    pub const WORLD: Self = Self {
        kind: EntityKind::World,
        raw: 0,
    };

    /// An entity of `kind` with raw id `raw`.
    #[must_use]
    pub const fn new(kind: EntityKind, raw: u64) -> Self {
        Self { kind, raw }
    }

    /// A person.
    #[must_use]
    pub const fn person(id: PersonId) -> Self {
        Self::new(EntityKind::Person, id.to_raw())
    }

    /// A region.
    #[must_use]
    pub fn region(id: RegionId) -> Self {
        Self::new(EntityKind::Region, u64::from(id.to_raw()))
    }

    /// The kind.
    #[must_use]
    pub const fn kind(self) -> EntityKind {
        self.kind
    }

    /// The raw id: the `entity_id` of an RNG key drawn for this entity.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.raw
    }
}

impl fmt::Debug for EntityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{:#x}", self.kind.name(), self.raw)
    }
}

impl fmt::Display for EntityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::{EntityKind, EntityRef};
    use crate::ids::{PersonId, RegionId};

    #[test]
    fn kind_orders_before_value() {
        let p = EntityRef::person(PersonId::from_raw(u64::MAX));
        let r = EntityRef::region(RegionId::from_raw(0));
        assert!(EntityRef::WORLD < p);
        assert!(
            p < r,
            "a person sorts before any region, whatever the raw values"
        );
        assert!(
            EntityRef::new(EntityKind::Person, 1) < EntityRef::new(EntityKind::Person, 2),
            "within a kind, by raw value"
        );
        assert_ne!(
            EntityRef::new(EntityKind::Household, 7),
            EntityRef::new(EntityKind::Settlement, 7)
        );
    }

    #[test]
    fn tags_are_pinned() {
        let tags: Vec<(u8, &str)> = EntityKind::ALL
            .iter()
            .map(|k| (k.tag(), k.name()))
            .collect();
        assert_eq!(
            tags,
            [
                (0, "world"),
                (1, "person"),
                (2, "household"),
                (3, "settlement"),
                (4, "region")
            ]
        );
        for k in EntityKind::ALL {
            assert_eq!(EntityKind::from_tag(k.tag()), Some(k));
        }
        assert_eq!(EntityKind::from_tag(5), None);
    }
}
