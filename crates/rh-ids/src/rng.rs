//! Counter-based RNG streams and the purpose registry (SPR-002 T-03).
//!
//! Two kinds of stream, both counter-based: draw `k` of a key is a pure function of the key and
//! `k`, never of how many draws came before or in what order streams were read.
//!
//! - [`RngStream`]: `ChaCha8` keyed by [`crate::hash`]'s `H`. The default for every authoritative
//!   draw.
//! - [`LightStream`]: `SplitMix64` from a 64-bit hash of the key, for light, high-volume uses
//!   (ADR-0006: "`SplitMix64` or xxh3 for scalar hashes"), after the Starsim CRN pattern
//!   (`docs/14`).
//!
//! # `SplitMix64` scalar hash, layout version 1
//!
//! `mix` is the `SplitMix64` output function (Steele, Lea and Flood 2014) and
//! `GAMMA = 0x9e3779b97f4a7c15`. All integers little-endian.
//!
//! - `h64_words(domain, w[0..n])`: `h = mix(domain ^ n)`, then `h = mix((h + GAMMA) ^ w[i])` for each
//!   word in order.
//! - `h64_bytes(domain, b)`: as `h64_words`, with `n` the byte length and the words the bytes in
//!   8-byte little-endian chunks, the last one zero-padded.
//! - `tag(name) = h64_bytes(0, name)`: a domain tag from an ASCII name.
//! - [`LightStream`] draw `k` = `mix(base + (k + 1) · GAMMA)` with
//!   `base = h64_words(tag("rehoboam/rh-ids/light/v1"), [seed, salt, system, purpose, entity, t])`:
//!   the `SplitMix64` sequence started at `base`.
//!
//! TRADEOFF: a light stream compresses the 352-bit key to 64 bits, so two keys collide with
//! probability 2⁻⁶⁴ per pair; over 10¹⁰ keys (10⁶ persons × 10⁴ months) a few pairs are expected
//! to share draws. That is harmless noise for scalar uses, but anything whose draws must never
//! coincide (or that needs more than a handful of draws per key) uses [`RngStream`], whose keys
//! are 256-bit.
//!
//! # Purposes
//!
//! Every purpose is declared once, as a [`Purpose`] constant, and listed in its system's
//! [`PurposeRegistry`] (the `rng_purposes.rs` convention of `docs/13` §3). The registry is built in
//! a `const`, so a duplicate name or identifier is a compile error:
//!
//! ```compile_fail,E0080
//! use rh_ids::{Purpose, PurposeRegistry};
//! const DEATH_TIME: Purpose = Purpose::new("death_time", true);
//! const AGAIN: Purpose = Purpose::new("death_time", false);
//! const PURPOSES: PurposeRegistry = PurposeRegistry::new(&[DEATH_TIME, AGAIN]);
//! ```
//!
//! ```
//! use rh_ids::{Purpose, PurposeRegistry};
//! const DEATH_TIME: Purpose = Purpose::new("death_time", true);
//! const AGAIN: Purpose = Purpose::new("birth_time", false);
//! const PURPOSES: PurposeRegistry = PurposeRegistry::new(&[DEATH_TIME, AGAIN]);
//! ```
//!
//! (The `compile_fail` case has a compiling twin; see the `ids` module for why.)
//!
//! REF: ADR-0006 v1.1 items 2–3; `05.1` §6.2, §6.9; `docs/13` §3 (randomness); `docs/14`
//! (`rand_chacha`, Starsim CRN).

use rand_chacha::ChaCha8Rng;
use rand_core::Rng;

use crate::hash::{BranchSalt, PurposeId, RngKey, StreamKey, word_pos};
use crate::ids::SystemId;

/// The `SplitMix64` increment, the odd integer closest to 2⁶⁴/φ.
/// REF: Steele, Lea and Flood (2014), "Fast splittable pseudorandom number generators".
pub const GAMMA: u64 = 0x9e37_79b9_7f4a_7c15;
/// First multiplier of the `SplitMix64` output function (same REF).
const MIX_MUL_1: u64 = 0xbf58_476d_1ce4_e5b9;
/// Second multiplier of the `SplitMix64` output function (same REF).
const MIX_MUL_2: u64 = 0x94d0_49bb_1331_11eb;
/// Bytes per absorbed word.
const WORD_BYTES: usize = 8;

/// The `SplitMix64` output function: a bijective 64-bit mixer.
#[must_use]
pub const fn mix(z: u64) -> u64 {
    let z = (z ^ (z >> 30)).wrapping_mul(MIX_MUL_1);
    let z = (z ^ (z >> 27)).wrapping_mul(MIX_MUL_2);
    z ^ (z >> 31)
}

/// Absorbs one word into the running hash.
const fn absorb(h: u64, w: u64) -> u64 {
    mix(h.wrapping_add(GAMMA) ^ w)
}

/// The `SplitMix64` scalar hash of `words` under `domain` (layout in the module docs).
#[must_use]
pub const fn h64_words(domain: u64, words: &[u64]) -> u64 {
    let mut h = mix(domain ^ words.len() as u64);
    let mut i = 0;
    while i < words.len() {
        h = absorb(h, words[i]);
        i += 1;
    }
    h
}

/// The `SplitMix64` scalar hash of `bytes` under `domain` (layout in the module docs).
#[must_use]
pub const fn h64_bytes(domain: u64, bytes: &[u8]) -> u64 {
    let mut h = mix(domain ^ bytes.len() as u64);
    let mut start = 0;
    while start < bytes.len() {
        let mut chunk = [0_u8; WORD_BYTES];
        let mut j = 0;
        while j < WORD_BYTES && start + j < bytes.len() {
            chunk[j] = bytes[start + j];
            j += 1;
        }
        h = absorb(h, u64::from_le_bytes(chunk));
        start += WORD_BYTES;
    }
    h
}

/// A domain tag from an ASCII name: `h64_bytes(0, name)`.
#[must_use]
pub const fn tag(name: &str) -> u64 {
    h64_bytes(0, name.as_bytes())
}

/// Domain of purpose identifiers.
pub const TAG_PURPOSE: u64 = tag("rehoboam/rh-ids/purpose/v1");
/// Domain of light-stream bases.
pub const TAG_LIGHT: u64 = tag("rehoboam/rh-ids/light/v1");

impl PurposeId {
    /// The identifier of a purpose name: `h64_bytes(TAG_PURPOSE, name)`. Depends on the name only.
    #[must_use]
    pub const fn of(name: &str) -> Self {
        Self::from_raw(h64_bytes(TAG_PURPOSE, name.as_bytes()))
    }
}

/// A declared RNG purpose: a name from the system spec's `[[rng]] purpose`, its identifier, and
/// whether draws made before a fork are re-drawn after it (`05.1` §6.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Purpose {
    name: &'static str,
    id: PurposeId,
    resample_on_fork: bool,
}

impl Purpose {
    /// Declares a purpose. Use it in a `const`, so that a bad name fails the build.
    ///
    /// # Panics
    ///
    /// If `name` is not `snake_case` ASCII (`[a-z][a-z0-9_]*`), the form of `[[rng]] purpose`
    /// in the system specs. In a `const` this is a compile error.
    #[must_use]
    pub const fn new(name: &'static str, resample_on_fork: bool) -> Self {
        assert!(
            is_snake_case(name.as_bytes()),
            "RNG purpose names are snake_case ASCII"
        );
        Self {
            name,
            id: PurposeId::of(name),
            resample_on_fork,
        }
    }

    /// The declared name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The stable identifier, the `purpose` field of the RNG key.
    #[must_use]
    pub const fn id(&self) -> PurposeId {
        self.id
    }

    /// Whether pre-fork draws of this purpose are re-drawn after a fork (vital timers, latent
    /// periods; the Registry flag of `05.1` §6.9).
    #[must_use]
    pub const fn resample_on_fork(&self) -> bool {
        self.resample_on_fork
    }
}

/// `[a-z][a-z0-9_]*`.
const fn is_snake_case(b: &[u8]) -> bool {
    if b.is_empty() || !b[0].is_ascii_lowercase() {
        return false;
    }
    let mut i = 1;
    while i < b.len() {
        if !(b[i].is_ascii_lowercase() || b[i].is_ascii_digit() || b[i] == b'_') {
            return false;
        }
        i += 1;
    }
    true
}

/// `a == b` for byte strings, usable in `const`.
const fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The purposes one system declares, checked for duplicates when the registry is built.
#[derive(Clone, Copy, Debug)]
pub struct PurposeRegistry {
    purposes: &'static [Purpose],
}

impl PurposeRegistry {
    /// Builds a registry. Build it in a `const` so that the check runs at compile time.
    ///
    /// # Panics
    ///
    /// If two purposes share a name or an identifier (a 64-bit hash collision between two
    /// names). In a `const` this is a compile error.
    #[must_use]
    pub const fn new(purposes: &'static [Purpose]) -> Self {
        let mut i = 0;
        while i < purposes.len() {
            let mut j = i + 1;
            while j < purposes.len() {
                assert!(
                    !bytes_eq(purposes[i].name.as_bytes(), purposes[j].name.as_bytes()),
                    "duplicate RNG purpose name"
                );
                assert!(
                    purposes[i].id.to_raw() != purposes[j].id.to_raw(),
                    "RNG purpose identifiers collide"
                );
                j += 1;
            }
            i += 1;
        }
        Self { purposes }
    }

    /// The purpose with this identifier, if declared.
    #[must_use]
    pub fn get(&self, id: PurposeId) -> Option<&'static Purpose> {
        self.purposes.iter().find(|p| p.id == id)
    }

    /// The purpose with this name, if declared.
    #[must_use]
    pub fn by_name(&self, name: &str) -> Option<&'static Purpose> {
        self.purposes.iter().find(|p| p.name == name)
    }

    /// The purposes in declaration order.
    #[must_use]
    pub fn purposes(&self) -> &'static [Purpose] {
        self.purposes
    }
}

/// A `ChaCha8` stream: the draws `H(key, 0)`, `H(key, 1)`, … of one [`RngKey`].
///
/// [`RngStream::draw`] seeks, so its result never depends on earlier calls;
/// [`RngStream::next_u64`] reads sequentially from the current position and gives the same
/// values, faster.
#[derive(Clone)]
pub struct RngStream {
    rng: ChaCha8Rng,
}

impl RngStream {
    /// The stream of `key`, positioned at draw 0.
    #[must_use]
    pub fn new(key: &RngKey) -> Self {
        Self::from_stream_key(&StreamKey::of(key), key.t)
    }

    /// The stream of tick `t` under a cached [`StreamKey`], positioned at draw 0.
    #[must_use]
    pub fn from_stream_key(stream_key: &StreamKey, t: i64) -> Self {
        Self {
            rng: stream_key.chacha(t),
        }
    }

    /// Draw `k`, wherever the stream is positioned; afterwards the stream is at `k + 1`.
    pub fn draw(&mut self, k: u64) -> u64 {
        self.seek(k);
        self.rng.next_u64()
    }

    /// The draw at the current position; then advances by one.
    pub fn next_u64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    /// Moves to draw `k`.
    pub fn seek(&mut self, k: u64) {
        self.rng.set_word_pos(word_pos(k));
    }

    /// The index of the next draw [`RngStream::next_u64`] returns.
    ///
    /// Positions past `u64::MAX` draws wrap (the `ChaCha` counter allows 2⁶⁵ words per stream).
    #[must_use]
    pub fn position(&self) -> u64 {
        // INVARIANT: the stream only ever moves by whole draws, so the word position is even;
        // its upper bits beyond 2^65 words are unreachable from a u64 draw index.
        u64::try_from(self.rng.get_word_pos() / 2).unwrap_or(u64::MAX)
    }
}

/// A `SplitMix64` stream for light uses (module docs: layout and collision trade-off).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LightStream {
    base: u64,
}

impl LightStream {
    /// The light stream of `key`.
    #[must_use]
    pub const fn new(key: &RngKey) -> Self {
        Self::from_parts(
            key.world_seed,
            key.branch_salt,
            key.system,
            key.purpose,
            key.entity,
            key.t,
        )
    }

    /// The light stream of the key given field by field.
    #[must_use]
    pub const fn from_parts(
        world_seed: u64,
        branch_salt: BranchSalt,
        system: SystemId,
        purpose: PurposeId,
        entity: u64,
        t: i64,
    ) -> Self {
        let words = [
            world_seed,
            branch_salt.to_raw(),
            system.to_raw() as u64,
            purpose.to_raw(),
            entity,
            t.cast_unsigned(),
        ];
        Self {
            base: h64_words(TAG_LIGHT, &words),
        }
    }

    /// Draw `k`: `mix(base + (k + 1) · GAMMA)`.
    #[must_use]
    pub const fn draw(&self, k: u64) -> u64 {
        mix(self
            .base
            .wrapping_add(k.wrapping_add(1).wrapping_mul(GAMMA)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::h;

    fn key(entity: u64) -> RngKey {
        RngKey {
            world_seed: 7,
            branch_salt: BranchSalt::from_raw(0),
            system: SystemId::from_raw(2),
            purpose: PurposeId::of("death_time"),
            entity,
            t: 43_200,
        }
    }

    #[test]
    fn mix_is_splitmix64() {
        // The first output of SplitMix64 seeded with 0 is mix(GAMMA) = 0xe220a8397b1dcdaf
        // (the published reference sequence, e.g. Vigna's splitmix64.c).
        assert_eq!(mix(GAMMA), 0xe220_a839_7b1d_cdaf);
    }

    #[test]
    fn stream_matches_h_sequentially_and_by_seek() {
        let mut s = RngStream::new(&key(5));
        for k in 0..40 {
            assert_eq!(s.position(), k);
            assert_eq!(s.next_u64(), h(&key(5), k));
        }
        assert_eq!(s.draw(3), h(&key(5), 3));
        assert_eq!(s.position(), 4);
        assert_eq!(s.draw(1 << 50), h(&key(5), 1 << 50));
    }

    /// T-03 acceptance: draws do not depend on call order. Several streams are read in a
    /// shuffled interleaving of (stream, k) requests; every value equals the in-order value.
    #[test]
    fn draws_are_independent_of_call_order() {
        const STREAMS: u64 = 6;
        const DRAWS: u64 = 50;
        let shuffle = LightStream::new(&key(999));
        for trial in 0..20_u64 {
            let mut requests: Vec<(u64, u64)> = (0..STREAMS)
                .flat_map(|e| (0..DRAWS).map(move |k| (e, k)))
                .collect();
            // Fisher-Yates from a keyed stream: the test is itself deterministic.
            for i in (1..requests.len()).rev() {
                let r = shuffle.draw(trial * 10_000 + i as u64);
                let j = usize::try_from(r % (i as u64 + 1)).unwrap();
                requests.swap(i, j);
            }
            let mut streams: Vec<RngStream> =
                (0..STREAMS).map(|e| RngStream::new(&key(e))).collect();
            for &(e, k) in &requests {
                let s = &mut streams[usize::try_from(e).unwrap()];
                assert_eq!(s.draw(k), h(&key(e), k), "stream {e} draw {k}");
                let light = LightStream::new(&key(e));
                assert_eq!(light.draw(k), LightStream::new(&key(e)).draw(k));
            }
        }
    }

    #[test]
    fn light_stream_is_splitmix_from_base() {
        let s = LightStream::new(&key(1));
        let mut state = s.base;
        for k in 0..10 {
            state = state.wrapping_add(GAMMA);
            assert_eq!(s.draw(k), mix(state));
        }
    }

    #[test]
    fn purpose_ids_depend_on_the_name_only() {
        assert_eq!(
            PurposeId::of("death_time"),
            Purpose::new("death_time", true).id()
        );
        assert_ne!(PurposeId::of("death_time"), PurposeId::of("birth_time"));
        let p = Purpose::new("encounter_outcome", false);
        assert_eq!(p.name(), "encounter_outcome");
        assert!(!p.resample_on_fork());
    }

    #[test]
    fn registry_lookups() {
        const A: Purpose = Purpose::new("death_time", true);
        const B: Purpose = Purpose::new("activity_block", false);
        const REG: PurposeRegistry = PurposeRegistry::new(&[A, B]);
        assert_eq!(REG.get(A.id()), Some(&A));
        assert_eq!(REG.by_name("activity_block"), Some(&B));
        assert_eq!(REG.by_name("missing"), None);
        assert_eq!(REG.purposes().len(), 2);
    }

    #[test]
    fn snake_case_names_only() {
        assert!(is_snake_case(b"death_time2"));
        for bad in [&b""[..], b"Death", b"2x", b"a-b", b"a b", b"_a"] {
            assert!(!is_snake_case(bad), "{bad:?}");
        }
    }

    #[test]
    #[should_panic(expected = "duplicate RNG purpose name")]
    fn runtime_duplicate_is_rejected() {
        static PURPOSES: [Purpose; 2] = [
            Purpose::new("death_time", true),
            Purpose::new("death_time", false),
        ];
        let _ = PurposeRegistry::new(&PURPOSES);
    }
}
