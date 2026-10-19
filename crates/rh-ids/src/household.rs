//! Household identifiers (SPR-015).
//!
//! A household's identifier is derived from its head and formation instant, so the same
//! household formed at the same instant in any branch gets the same id (ADR-0026 item 4).
//!
//! # Derivation, layout version 1
//!
//! With the `SplitMix64` hash `h64_words` and `tag` of [`crate::rng`]:
//!
//! | What | Base identifier |
//! |---|---|
//! | A new household | `h64_words(tag("rehoboam/rh-ids/household/v1"), [head, formed_tick])` |
//! | Rehash `c` of a colliding base `b` (c = 1, 2, …) | `h64_words(tag("rehoboam/rh-ids/household/rehash/v1"), [b, c])` |
//!
//! `formed_tick` enters as its two's-complement `u64`.
//!
//! REF: ADR-0026 item 4; SPR-015_BirthsF1.md § H1.

use crate::ids::HouseholdId;
use crate::rng::{h64_words, tag};

/// Domain of household identifiers.
pub const TAG_HOUSEHOLD: u64 = tag("rehoboam/rh-ids/household/v1");
/// Domain of household collision rehashes.
pub const TAG_HOUSEHOLD_REHASH: u64 = tag("rehoboam/rh-ids/household/rehash/v1");

/// Rehash attempts before giving up (same bound as PersonId).
pub const MAX_REHASH: u64 = 64;

/// The base identifier of a household formed at `formed_tick` by `head`.
#[must_use]
pub fn household_base(head: u64, formed_tick: i64) -> HouseholdId {
    HouseholdId::from_raw(h64_words(
        TAG_HOUSEHOLD,
        &[head, formed_tick.cast_unsigned()],
    ))
}

/// Rehash `c` (1-based) of a colliding base identifier.
#[must_use]
pub fn household_rehash(base: HouseholdId, c: u64) -> HouseholdId {
    HouseholdId::from_raw(h64_words(TAG_HOUSEHOLD_REHASH, &[base.to_raw(), c]))
}