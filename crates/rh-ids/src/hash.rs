//! The keyed hash `H` (SPR-002 T-02).
//!
//! `u = H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)` (ADR-0006 v1.1 item 2),
//! plus the entity's `epoch` (ADR-0025: the re-sample key carries the count of material inputs that
//! have touched the entity), built on `ChaCha8` "with stream and position": the key fields select a
//! `ChaCha8` stream, `k` is the position in it.
//!
//! The normative statement of this layout is `05.1` §6 item 2; this module implements it.
//!
//! # Byte layout, version 2
//!
//! Version 2 added `epoch` (`u32`, in the high half of `K1`'s block counter) and changed the domain
//! tag to `v2`; version 1 had no epoch.
//!
//! All integers are **little-endian**; `t: i64` enters as its two's-complement `u64`.
//! `ChaCha8` is the original (Bernstein) `ChaCha` with 8 rounds, a 64-bit block counter in state
//! words 12–13 and a 64-bit nonce in words 14–15, as implemented by `rand_chacha`.
//!
//! | Step | Key (32 bytes) | Nonce (u64) | Block counter (u64) | Output |
//! |---|---|---|---|---|
//! | `K0` | — | — | — | `le64(world_seed) ‖ "rehoboam/rh-ids/H/v2" ‖ 0x00 × 4` |
//! | `K1` | `K0` | `branch_salt` | `system_id \| epoch << 32` (two `u32`s) | first 32 bytes of the block |
//! | `K2` | `K1` | `purpose_id` | `entity_id` | first 32 bytes of the block |
//! | stream | `K2` | `t` | `k / 8` | `H(…, k)` = stream words `2k` (low) and `2k + 1` (high) |
//!
//! Each step is a `ChaCha8` block used as a pseudo-random function of 128 input bits (nonce and
//! counter) under the previous step's key, so the full 64 + 64 + 32 + 32 + 64 + 64 + 64 key bits
//! are absorbed without truncation and distinct keys give unrelated streams. `epoch` and `k` sit in
//! different steps, so a draw index can never stand in for an epoch.
//!
//! DECISION: `epoch` shares `K1`'s block counter with `system_id` (disjoint 32-bit halves).
//! ALTERNATIVES CONSIDERED: a fourth cascade step `K3 = PRF(K2, nonce = t, block = epoch)` (one
//! more `ChaCha8` evaluation for every stream, about 1.5× the cost of opening one); folding it
//! into `entity` or `t` (rejected: lossless `u64` entity keys and `i64` ticks leave no room).
//! TRADEOFF: `K1` is cached per (salt, system, epoch): the common `epoch = 0` key stays cached per
//! system, and an entity re-sampled by a material input pays one extra `ChaCha8` block.
//!
//! DECISION: a `ChaCha8` cascade, not a general-purpose hash (`xxh3`) of serialised bytes.
//! ALTERNATIVES CONSIDERED: `xxh3`-128 of the 44 key bytes as the `ChaCha` key (needs a second
//! hash crate, and an independent reference implementation is impractical in the standard
//! library); `Philox-4x64` (ADR-0006 allows it; would need a port, docs/14 Random123).
//! TRADEOFF: one extra `ChaCha8` evaluation per (system, epoch) and per (purpose, entity) — callers cache
//! [`SystemKey`] and [`StreamKey`] — in exchange for a layout that `bin/gen_rng_golden.py`
//! re-implements independently from the standard library alone (V-DET-09 known answers).
//!
//! The layout is part of the determinism contract: changing any byte of it moves every
//! `state_hash` and is a C3 change. The golden vectors in `tests/golden/rh_ids_v2.txt` pin it.
//!
//! REF: ADR-0006 v1.1 items 2–3; `05.1` §6.2; `docs/14` (`rand_chacha`: DEPEND, pinned exactly).

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};

use crate::ids::SystemId;

/// Bytes of a `ChaCha` key.
const KEY_BYTES: usize = 32;
/// 32-bit words per `ChaCha` block.
const BLOCK_WORDS: u128 = 16;
/// The version-2 domain tag, padded to fill `K0` after the 8 seed bytes.
pub const DOMAIN_H_V2: [u8; 24] = *b"rehoboam/rh-ids/H/v2\0\0\0\0";

/// The `branch_salt(t)` field: the hash of the fork lineage covering `t` (`05.1` §6.9).
///
/// Produced by [`crate::salt::SaltLineage::branch_salt`]; the raw constructor exists for tests and tools that
/// replay recorded keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BranchSalt(u64);

impl BranchSalt {
    /// Wraps a raw salt value.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw salt value.
    #[must_use]
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

/// The `purpose` field: a stable 64-bit identifier of an RNG purpose name.
///
/// Derived from the purpose name alone ([`PurposeId::of`], [`crate::rng::Purpose`]), so adding a purpose never shifts the
/// draws of another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PurposeId(u64);

impl PurposeId {
    /// Wraps a raw purpose identifier (replay tools and golden vectors).
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw purpose identifier.
    #[must_use]
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

/// The full key of one stream: every field of `H` except the draw index `k`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RngKey {
    /// The world seed (`WorldConfig::seed`, `05.9` IF-1).
    pub world_seed: u64,
    /// `branch_salt(t)` of the timeline making the draw.
    pub branch_salt: BranchSalt,
    /// The drawing system.
    pub system: SystemId,
    /// The declared purpose of the draw.
    pub purpose: PurposeId,
    /// The entity drawn for ([`crate::EntityKey`]).
    pub entity: u64,
    /// The entity's epoch: the number of material inputs that have re-sampled it (ADR-0025;
    /// ADR-0006 v1.1 item 8). 0 unless a material input has touched the entity. A `u32` holds
    /// 4 × 10⁹ re-samples of one entity, far beyond any world's input journal.
    pub epoch: u32,
    /// The tick, in `i64` minutes from the epoch (ADR-0026 item 6).
    pub t: i64,
}

/// One cascade step: the first 32 bytes of `ChaCha8` block `block` with nonce `nonce` under `key`.
fn prf(key: [u8; KEY_BYTES], nonce: u64, block: u64) -> [u8; KEY_BYTES] {
    let mut rng = ChaCha8Rng::from_seed(key);
    rng.set_stream(nonce);
    rng.set_word_pos(u128::from(block) * BLOCK_WORDS);
    let mut out = [0_u8; KEY_BYTES];
    rng.fill_bytes(&mut out);
    out
}

/// `K0`: the world seed followed by the version-2 domain tag.
fn seed_key(world_seed: u64) -> [u8; KEY_BYTES] {
    let mut key = [0_u8; KEY_BYTES];
    let (seed, domain) = key.split_at_mut(8);
    seed.copy_from_slice(&world_seed.to_le_bytes());
    domain.copy_from_slice(&DOMAIN_H_V2);
    key
}

/// `K1`: the key of one system in one branch at one entity epoch. Cache it per (salt, system)
/// for `epoch = 0`, the common case.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SystemKey([u8; KEY_BYTES]);

impl SystemKey {
    /// Derives `K1 = PRF(K0(world_seed), nonce = branch_salt, block = system_id | epoch << 32)`.
    #[must_use]
    pub fn new(world_seed: u64, branch_salt: BranchSalt, system: SystemId, epoch: u32) -> Self {
        let block = u64::from(system.to_raw()) | (u64::from(epoch) << 32);
        Self(prf(seed_key(world_seed), branch_salt.to_raw(), block))
    }

    /// Derives `K2 = PRF(K1, nonce = purpose_id, block = entity_id)`.
    #[must_use]
    pub fn stream_key(&self, purpose: PurposeId, entity: u64) -> StreamKey {
        StreamKey(prf(self.0, purpose.to_raw(), entity))
    }
}

/// `K2`: the key of the streams of one (salt, system, purpose, entity); one stream per tick.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StreamKey([u8; KEY_BYTES]);

impl StreamKey {
    /// Derives the stream key for a full [`RngKey`].
    #[must_use]
    pub fn of(key: &RngKey) -> Self {
        SystemKey::new(key.world_seed, key.branch_salt, key.system, key.epoch)
            .stream_key(key.purpose, key.entity)
    }

    /// The `ChaCha8` generator of tick `t`, positioned at draw 0.
    pub(crate) fn chacha(&self, t: i64) -> ChaCha8Rng {
        let mut rng = ChaCha8Rng::from_seed(self.0);
        // Two's complement: the documented layout of `t` (module docs).
        rng.set_stream(t.cast_unsigned());
        rng
    }

    /// Draw `k` of the stream of tick `t`.
    #[must_use]
    pub fn draw(&self, t: i64, k: u64) -> u64 {
        let mut rng = self.chacha(t);
        rng.set_word_pos(word_pos(k));
        rng.next_u64()
    }
}

/// The `ChaCha` word position of draw `k` (two 32-bit words per draw).
pub(crate) fn word_pos(k: u64) -> u128 {
    u128::from(k) * 2
}

/// `H(world_seed, branch_salt, system_id, epoch, purpose, entity_id, t, k)`.
///
/// A pure function of its arguments: it does not depend on any other draw or on call order.
/// Systems that draw many values for one key use [`crate::rng::RngStream`], which gives the
/// same values without re-deriving the key.
#[must_use]
pub fn h(key: &RngKey, k: u64) -> u64 {
    StreamKey::of(key).draw(key.t, k)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> RngKey {
        RngKey {
            world_seed: 42,
            branch_salt: BranchSalt::from_raw(0),
            system: SystemId::from_raw(3),
            purpose: PurposeId::from_raw(7),
            entity: 1_000,
            epoch: 0,
            t: -60,
        }
    }

    #[test]
    fn seed_key_layout_is_seed_then_domain() {
        let k0 = seed_key(0x0102_0304_0506_0708);
        assert_eq!(&k0[..8], &[8, 7, 6, 5, 4, 3, 2, 1], "little-endian seed");
        assert_eq!(&k0[8..], &DOMAIN_H_V2);
    }

    #[test]
    fn every_field_changes_the_draw() {
        let base = h(&key(), 0);
        let mut k = key();
        k.world_seed += 1;
        assert_ne!(h(&k, 0), base);
        let mut k = key();
        k.branch_salt = BranchSalt::from_raw(1);
        assert_ne!(h(&k, 0), base);
        let mut k = key();
        k.system = SystemId::from_raw(4);
        assert_ne!(h(&k, 0), base);
        let mut k = key();
        k.purpose = PurposeId::from_raw(8);
        assert_ne!(h(&k, 0), base);
        let mut k = key();
        k.entity += 1;
        assert_ne!(h(&k, 0), base);
        let mut k = key();
        k.t += 1;
        assert_ne!(h(&k, 0), base);
        let mut k = key();
        k.epoch = 1;
        assert_ne!(h(&k, 0), base);
        assert_ne!(h(&key(), 1), base);
    }

    /// GAP-2: the epoch and the system share K1's block counter in disjoint halves, so no
    /// (system, epoch) pair stands in for another, and the draw index never reaches K1.
    #[test]
    fn epoch_and_system_do_not_alias() {
        let at = |system: u32, epoch: u32, k: u64| {
            let mut key = key();
            key.system = SystemId::from_raw(system);
            key.epoch = epoch;
            h(&key, k)
        };
        assert_ne!(at(1, 0, 0), at(0, 1, 0));
        assert_ne!(at(u32::MAX, 0, 0), at(0, u32::MAX, 0));
        assert_ne!(at(3, 1, 0), at(3, 0, 1));
    }

    #[test]
    fn h_is_a_pure_function() {
        assert_eq!(h(&key(), 5), h(&key(), 5));
        assert_eq!(h(&key(), 5), StreamKey::of(&key()).draw(key().t, 5));
    }
}
