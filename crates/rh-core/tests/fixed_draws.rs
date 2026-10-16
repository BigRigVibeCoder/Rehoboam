//! A uniform `u64` draw against a `Q16_16` probability (SPR-010 T-03).
//!
//! The mapping (`crates/rh-core/src/fixed/draw.rs`): a draw `d` hits `p` iff `d / 2^64 < p`, so
//! `P(hit) = clamp(bits(p), 0, 2^16) / 2^16` exactly. These tests check it exhaustively over the
//! top 16 bits of the draw (which alone decide the outcome), at the edges (probability 0, 1, the
//! smallest step, and values outside `[0, 1]`), and by frequency against the exact probability.
//!
//! Integer arithmetic only: the frequency bound is computed without floats.
//!
//! Refs: DET-3; ADR-0006; `05.1` §6 items 2, 4.

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
