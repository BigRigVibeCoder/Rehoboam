//! Genealogical, content-addressed `PersonId`s (SPR-002 T-05).
//!
//! A person's identifier is derived from who they are, not from a global birth counter, so one
//! extra birth in a branch shifts no other identifier and common random numbers survive across
//! branches for every unaffected lineage (ADR-0026 item 1; RT-C3).
//!
//! # Derivation, layout version 1
//!
//! With the `SplitMix64` hash `h64_words` and `tag` of [`crate::rng`]:
//!
//! | Who | Base identifier |
//! |---|---|
//! | A child | `h64_words(tag("rehoboam/rh-ids/person/child/v1"), [mother, parity])` |
//! | A founder | `h64_words(tag("rehoboam/rh-ids/person/founder/v1"), [world_seed, k])` |
//! | An immigrant or off-map materialisation | `h64_words(tag("rehoboam/rh-ids/person/immigrant/v1"), [world_seed, origin, t, k])` |
//! | Rehash `c` of a colliding base `b` (c = 1, 2, …) | `h64_words(tag("rehoboam/rh-ids/person/rehash/v1"), [b, c])` |
//!
//! `parity` is the number of children the mother bore before this one (0 for her first), so
//! "the same mother's n-th child" is the same person in every branch. `t` enters as its
//! two's-complement `u64`.
//!
//! # Collisions
//!
//! [`PersonIdTable`] holds every identifier ever issued (the living and the dead: the life-course
//! log keeps everyone, ADR-0026 item 3). A base that is already taken is rehashed with
//! `c = 1, 2, …` until a free identifier is found. The table is a `BTreeSet`, and births are
//! issued in the engine's deterministic order, so the outcome is a pure function of the history.
//! The birthday bound is about 4 × 10⁻⁵ over 4 × 10⁷ lifetimes (ADR-0026), so collisions are rare
//! but handled.
//!
//! REF: ADR-0026 items 1, 3 and 6; `05.1` §5 (ledger `person_id u64`, `mother u64`); RT-C3.

use std::collections::BTreeSet;

use core::fmt;

use crate::ids::PersonId;
use crate::rng::{h64_words, tag};

/// Domain of children's identifiers.
pub const TAG_CHILD: u64 = tag("rehoboam/rh-ids/person/child/v1");
/// Domain of founders' identifiers.
pub const TAG_FOUNDER: u64 = tag("rehoboam/rh-ids/person/founder/v1");
/// Domain of immigrants' identifiers.
pub const TAG_IMMIGRANT: u64 = tag("rehoboam/rh-ids/person/immigrant/v1");
/// Domain of collision rehashes.
pub const TAG_REHASH: u64 = tag("rehoboam/rh-ids/person/rehash/v1");

/// Rehash attempts before giving up.
///
/// Each attempt collides with probability (issued / 2⁶⁴), below 10⁻¹¹ for 10⁸ issued
/// identifiers, so reaching this bound means the table or the hash is broken, not bad luck.
pub const MAX_REHASH: u64 = 64;

/// The base identifier of the `parity`-th child (0-based) of `mother`.
#[must_use]
pub const fn child_base(mother: PersonId, parity: u32) -> PersonId {
    PersonId::from_raw(h64_words(TAG_CHILD, &[mother.to_raw(), parity as u64]))
}

/// The base identifier of founder `k` of a world.
#[must_use]
pub const fn founder_base(world_seed: u64, k: u64) -> PersonId {
    PersonId::from_raw(h64_words(TAG_FOUNDER, &[world_seed, k]))
}

/// The base identifier of the `k`-th immigrant from `origin` at tick `t`.
///
/// `origin` is the raw key of where the person comes from: a region's raw identifier or an
/// off-map source declared by the world.
#[must_use]
pub const fn immigrant_base(world_seed: u64, origin: u64, t: i64, k: u64) -> PersonId {
    PersonId::from_raw(h64_words(
        TAG_IMMIGRANT,
        &[world_seed, origin, t.cast_unsigned(), k],
    ))
}

/// Rehash `c` (1-based) of a colliding base identifier.
#[must_use]
pub const fn rehash(base: PersonId, c: u64) -> PersonId {
    PersonId::from_raw(h64_words(TAG_REHASH, &[base.to_raw(), c]))
}

/// Why an identifier could not be issued.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdError {
    /// The base and all [`MAX_REHASH`] rehashes were already issued.
    RehashExhausted(PersonId),
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RehashExhausted(base) => write!(
                f,
                "person id base {base} and its {MAX_REHASH} rehashes are all taken"
            ),
        }
    }
}

impl std::error::Error for IdError {}

/// Every `PersonId` issued in a timeline: the collision table of ADR-0026 item 1.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PersonIdTable {
    issued: BTreeSet<PersonId>,
}

impl PersonIdTable {
    /// An empty table (a new world).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A table holding identifiers issued earlier (loaded from a keyframe).
    pub fn with_issued(issued: impl IntoIterator<Item = PersonId>) -> Self {
        Self {
            issued: issued.into_iter().collect(),
        }
    }

    /// Whether `id` was issued.
    #[must_use]
    pub fn contains(&self, id: PersonId) -> bool {
        self.issued.contains(&id)
    }

    /// The number of identifiers issued.
    #[must_use]
    pub fn len(&self) -> usize {
        self.issued.len()
    }

    /// Whether no identifier was issued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.issued.is_empty()
    }

    /// Issues `base`, or its first free rehash if `base` is taken.
    ///
    /// # Errors
    ///
    /// [`IdError::RehashExhausted`] if the base and [`MAX_REHASH`] rehashes are all taken.
    pub fn issue(&mut self, base: PersonId) -> Result<PersonId, IdError> {
        let mut candidate = base;
        let mut c = 0;
        while self.issued.contains(&candidate) {
            if c == MAX_REHASH {
                return Err(IdError::RehashExhausted(base));
            }
            c += 1;
            candidate = rehash(base, c);
        }
        self.issued.insert(candidate);
        Ok(candidate)
    }

    /// Issues the identifier of the `parity`-th child (0-based) of `mother`.
    ///
    /// # Errors
    ///
    /// As [`PersonIdTable::issue`].
    pub fn child(&mut self, mother: PersonId, parity: u32) -> Result<PersonId, IdError> {
        self.issue(child_base(mother, parity))
    }

    /// Issues the identifier of founder `k`.
    ///
    /// # Errors
    ///
    /// As [`PersonIdTable::issue`].
    pub fn founder(&mut self, world_seed: u64, k: u64) -> Result<PersonId, IdError> {
        self.issue(founder_base(world_seed, k))
    }

    /// Issues the identifier of the `k`-th immigrant from `origin` at tick `t`.
    ///
    /// # Errors
    ///
    /// As [`PersonIdTable::issue`].
    pub fn immigrant(
        &mut self,
        world_seed: u64,
        origin: u64,
        t: i64,
        k: u64,
    ) -> Result<PersonId, IdError> {
        self.issue(immigrant_base(world_seed, origin, t, k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_genealogical() {
        let mother = founder_base(1, 0);
        assert_eq!(child_base(mother, 0), child_base(mother, 0));
        assert_ne!(child_base(mother, 0), child_base(mother, 1));
        assert_ne!(child_base(mother, 0), child_base(founder_base(1, 1), 0));
        assert_ne!(founder_base(1, 0), founder_base(2, 0));
        assert_ne!(immigrant_base(1, 5, -10, 0), immigrant_base(1, 5, -9, 0));
    }

    #[test]
    fn an_extra_birth_shifts_no_other_identifier() {
        // Branch A: two mothers each have two children. Branch B: mother 0 has an extra child
        // first in time, before mother 1's children. Mother 1's children are unchanged.
        let (m0, m1) = (founder_base(9, 0), founder_base(9, 1));
        let mut a = PersonIdTable::new();
        let a_ids = [a.child(m0, 0), a.child(m1, 0), a.child(m1, 1)].map(Result::unwrap);
        let mut b = PersonIdTable::new();
        b.child(m0, 0).unwrap();
        b.child(m0, 1).unwrap();
        let b_ids = [b.child(m1, 0), b.child(m1, 1)].map(Result::unwrap);
        assert_eq!(a_ids[1..], b_ids);
    }

    /// T-05 acceptance: forced collisions are resolved by rehashing with a counter, the same way
    /// in every run.
    #[test]
    fn forced_collisions_rehash_deterministically() {
        let mother = founder_base(3, 7);
        let base = child_base(mother, 2);
        for taken in 0..4_u64 {
            // Pre-issue the base and its first `taken - 1` rehashes.
            let mut preissued = vec![];
            if taken > 0 {
                preissued.push(base);
                preissued.extend((1..taken).map(|c| rehash(base, c)));
            }
            let mut t1 = PersonIdTable::with_issued(preissued.clone());
            let mut t2 = PersonIdTable::with_issued(preissued.into_iter().rev());
            let id1 = t1.child(mother, 2).unwrap();
            let id2 = t2.child(mother, 2).unwrap();
            let expected = if taken == 0 {
                base
            } else {
                rehash(base, taken)
            };
            assert_eq!(id1, expected, "{taken} taken");
            assert_eq!(id1, id2, "independent of the table's load order");
            assert!(t1.contains(id1));
        }
    }

    #[test]
    fn exhausted_rehash_is_an_error() {
        let base = founder_base(0, 0);
        let all = std::iter::once(base).chain((1..=MAX_REHASH).map(|c| rehash(base, c)));
        let mut table = PersonIdTable::with_issued(all);
        assert_eq!(table.issue(base), Err(IdError::RehashExhausted(base)));
        assert_eq!(table.len(), usize::try_from(MAX_REHASH).unwrap() + 1);
        assert!(
            IdError::RehashExhausted(base)
                .to_string()
                .contains("64 rehashes")
        );
    }

    #[test]
    fn issued_ids_are_unique() {
        let mut table = PersonIdTable::new();
        assert!(table.is_empty());
        let mut ids = BTreeSet::new();
        for k in 0..200 {
            let f = table.founder(11, k).unwrap();
            assert!(ids.insert(f));
            for parity in 0..5 {
                assert!(ids.insert(table.child(f, parity).unwrap()));
            }
        }
        // Issuing the same derivations again collides with every one and rehashes.
        for k in 0..200 {
            let again = table.founder(11, k).unwrap();
            assert_eq!(again, rehash(founder_base(11, k), 1));
        }
        assert_eq!(table.len(), 1_400);
    }
}
