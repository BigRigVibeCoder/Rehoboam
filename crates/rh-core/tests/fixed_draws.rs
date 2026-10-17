//! A uniform `u64` draw against a `Q16_16` probability (SPR-010 T-03).
//!
//! The mapping (`crates/rh-core/src/fixed/draw.rs`): a draw `d` hits `p` iff `d / 2^64 < p`, so
//! `P(hit) = clamp(bits(p), 0, 2^16) / 2^16` exactly. These tests check it exhaustively over the
//! top 16 bits of the draw (which alone decide the outcome), at the edges (probability 0, 1, the
//! smallest step, and values outside `[0, 1]`), and by frequency against the exact probability.
//!
//! Integer arithmetic only: the frequency bound is computed without floats.
//!
//! `Prob32` (SPR-011 T-01, ruling 4) gets the same checks at 32 fraction bits, from a keyed stream.
//!
//! Refs: DET-3; ADR-0006; `05.1` §6 items 2, 4.
#![allow(
    clippy::panic,
    reason = "test helper: a bad bit pattern must fail the test loudly"
)]

use rh_core::fixed::{Q16_16, UniformDraw, chance, draw_hits};

/// `SplitMix64` (Steele, Lea and Flood 2014): a stand-in uniform source until SPR-002's keyed RNG.
struct SplitMix64(u64);

impl UniformDraw for SplitMix64 {
    fn draw_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// The exact number of hits among the 2^16 blocks of draws: `clamp(bits, 0, 2^16)`.
fn exact_hits(p: Q16_16) -> u64 {
    u64::from(p.to_bits().clamp(0, 1 << 16).cast_unsigned())
}

fn edge_probabilities() -> Vec<Q16_16> {
    [
        i32::MIN,
        -65_536,
        -1,
        0,
        1,
        2,
        32_767,
        32_768,
        65_534,
        65_535,
        65_536,
        65_537,
        i32::MAX,
    ]
    .into_iter()
    .map(Q16_16::from_bits)
    .collect()
}

#[test]
fn every_block_of_draws_hits_exactly_bits_times() {
    // Exhaustive over the top 16 bits; the low 48 bits are checked at both of their extremes, and
    // at a middle value, to show they never change the outcome.
    for p in edge_probabilities() {
        for low in [0_u64, 1 << 47, (1 << 48) - 1] {
            let hits = (0..=u64::from(u16::MAX))
                .filter(|&top| draw_hits((top << 48) | low, p))
                .count();
            assert_eq!(
                hits as u64,
                exact_hits(p),
                "p bits {} with low bits {low}",
                p.to_bits()
            );
        }
    }
}

#[test]
fn the_hits_are_the_lowest_blocks() {
    // u < p: the draws that hit are exactly those below bits * 2^48.
    for p in edge_probabilities() {
        let first_miss = exact_hits(p);
        for top in [
            0,
            first_miss.saturating_sub(1),
            first_miss,
            first_miss + 1,
            65_535,
        ] {
            if top > 65_535 {
                continue;
            }
            assert_eq!(
                draw_hits(top << 48, p),
                top < first_miss,
                "p bits {}",
                p.to_bits()
            );
        }
    }
}

#[test]
fn frequency_matches_the_exact_probability() {
    // N draws per probability; |hits - N p| <= 5 standard deviations, in integers:
    // (hits * 2^16 - N b)^2 <= 25 N b (2^16 - b), with b = clamp(bits, 0, 2^16).
    // Seeded, so the outcome is fixed; the bound documents how far from exact a run may be.
    const N: u64 = 1 << 20;
    let mut source = SplitMix64(0x5EED_D2A0);
    for bits in [1, 7, 655, 6_554, 21_845, 32_768, 58_982, 65_535] {
        let p = Q16_16::from_bits(bits);
        let hits = (0..N).filter(|_| chance(&mut source, p)).count() as u128;
        let b = u128::from(exact_hits(p));
        let deviation = (hits << 16).abs_diff(u128::from(N) * b);
        let bound = 25 * u128::from(N) * b * ((1 << 16) - b);
        assert!(
            deviation * deviation <= bound,
            "p bits {bits}: {hits} hits in {N}"
        );
    }
    for p in [Q16_16::ZERO, Q16_16::MIN, -Q16_16::EPSILON] {
        assert!(
            (0..1_000).all(|_| !chance(&mut source, p)),
            "p <= 0 never hits"
        );
    }
    for p in [Q16_16::ONE, Q16_16::MAX, Q16_16::ONE + Q16_16::EPSILON] {
        assert!(
            (0..1_000).all(|_| chance(&mut source, p)),
            "p >= 1 always hits"
        );
    }
}

// --- Prob32 (SPR-011 T-01, ruling 4; `05.1` §6 item 4) ----------------------------------------

use rh_core::fixed::Prob32;
use rh_ids::{BranchSalt, PurposeId, RngStream, SystemId, SystemKey};

fn prob32(bits: u64) -> Prob32 {
    Prob32::from_bits(bits).unwrap_or_else(|| panic!("{bits} is not a Prob32"))
}

/// Probabilities at the edges of the type, and a spread across it.
fn prob32_edges() -> Vec<u64> {
    let mut bits = vec![0, 1, 2, 3, 1 << 31, (1 << 32) - 2, (1 << 32) - 1, 1 << 32];
    bits.extend((1..64_u64).map(|i| i.wrapping_mul(0x9E37_79B9) & 0xFFFF_FFFF));
    bits
}

#[test]
fn prob32_hits_exactly_the_draws_below_bits_times_two_to_the_32() {
    // The hitting draws are exactly [0, bits · 2^32): `P(hit) = bits · 2^32 / 2^64 = bits / 2^32`.
    // A draw's low 32 bits never change the outcome inside a block; the boundary is exact.
    for bits in prob32_edges() {
        let p = prob32(bits);
        if bits == 1 << 32 {
            assert!(
                draw_hits(u64::MAX, p) && draw_hits(0, p),
                "p = 1 always hits"
            );
            continue;
        }
        let first_miss = bits << 32;
        for low in [0_u64, 1, 1 << 31, (1 << 32) - 1] {
            if let Some(d) = first_miss.checked_sub((1 << 32) - low) {
                assert!(
                    draw_hits(d, p),
                    "bits {bits}: draw {d} is in the last hitting block"
                );
            }
            if let Some(d) = first_miss.checked_add(low) {
                assert!(
                    !draw_hits(d, p),
                    "bits {bits}: draw {d} is in the first missing block"
                );
            }
        }
        assert_eq!(draw_hits(0, p), bits > 0);
        assert!(!draw_hits(u64::MAX, p));
    }
}

#[test]
fn prob32_frequency_from_a_keyed_stream_matches_the_exact_probability() {
    // |hits - N p| <= 5 standard deviations, in integers, with p = b / 2^32:
    // (hits · 2^32 - N b)^2 <= 25 N b (2^32 - b). The stream is rh-ids' keyed ChaCha8.
    const N: u64 = 1 << 18;
    let key = SystemKey::new(0x5EED, BranchSalt::from_raw(0), SystemId::from_raw(9), 0)
        .stream_key(PurposeId::of("frequency"), 1);
    let mut source = RngStream::from_stream_key(&key, 0);
    for bits in [
        1 << 12,
        214_748,
        21_474_836,
        1 << 31,
        3_865_470_566,
        (1 << 32) - 4_096,
    ] {
        let p = prob32(bits);
        let hits = u128::from((0..N).filter(|_| chance(&mut source, p)).count() as u64);
        let b = u128::from(bits);
        let deviation = (hits << 32).abs_diff(u128::from(N) * b);
        let bound = 25 * u128::from(N) * b * ((1 << 32) - b);
        assert!(
            deviation * deviation <= bound,
            "bits {bits}: {hits} hits in {N}"
        );
    }
    let before = source.position();
    assert!((0..1_000).all(|_| !chance(&mut source, Prob32::ZERO)));
    assert!((0..1_000).all(|_| chance(&mut source, Prob32::ONE)));
    assert_eq!(
        source.position(),
        before + 2_000,
        "one draw per call, whatever p (CRN)"
    );
}
