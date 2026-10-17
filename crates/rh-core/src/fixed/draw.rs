//! Thresholds against random draws: does a uniform `u64` draw fall under a `Q16_16` probability?
//!
//! **The exact mapping.** A draw `d` stands for the real number `u = d / 2^64`, uniform on `[0, 1)`
//! over the 2^64 equally likely draws. A probability `p` (a `Q16_16` with bits `b`, value
//! `b / 2^16`) **hits** if and only if `u < p`. With integers only:
//!
//! - `b <= 0` (`p <= 0`): never hits.
//! - `b >= 2^16` (`p >= 1`): always hits.
//! - otherwise: hits iff `d < b * 2^48`, which is `u < p` multiplied through by `2^64`.
//!
//! So `P(hit) = clamp(b, 0, 2^16) / 2^16` **exactly**: the 2^64 draws split into whole blocks of
//! 2^48, one per step of `p`, and `p` hits in exactly `b` of them. The smallest non-zero
//! probability, `Q16_16::EPSILON`, hits for the 2^48 draws below `2^48` (the draws whose top 16
//! bits are zero). A probability outside `[0, 1]` is clamped by this definition, not by a
//! special case: it is what `u < p` means for such a `p`.
//!
//! DECISION: compare `u < p` (strict), so that `p = 0` never hits and `p = 1` always hits; with
//! `u <= p`, `p = 0` would hit for the one draw `d = 0`. The comparison uses the draw's top 16 bits
//! (the high bits of `d`), the convention for counter-based generators.
//!
//! The draw comes from the keyed RNG (ADR-0006: `H(world_seed, branch_salt(t), system_id, epoch,
//! purpose, entity_id, t, k)`), built in `rh-ids` by SPR-002. `UniformDraw` is the interface:
//! anything that yields uniform `u64` draws; `rh_ids::RngStream` implements it (BCK-002 A-21,
//! SPR-011 T-01), reading its next draw. This module never creates randomness itself.
//!
//! `draw_hits` and `chance` take either probability type, `Q16_16` or `Prob32` (SPR-011 ruling 4,
//! `prob.rs`: the same mapping at 32 fraction bits), through the sealed [`Probability`] trait. The
//! trait gives no conversion between them: a draw is compared with one probability of one type.
//!
//! REF: `CODEX/05_PROJECT/SPR-010_FixedPoint.md` T-03; `CODEX/05_PROJECT/SPR-011_WalkingSkeleton.md`
//! T-01; ADR-0006; `05.1` §6 items 2, 4.

use rh_ids::RngStream;

use super::prob::Prob32;
use super::q::Q16_16;

/// A source of uniform `u64` draws: every value equally likely, draws independent.
///
/// SPR-002's keyed RNG (`rh-ids`) will implement it (BCK-002 A-21), keyed by
/// `(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)` with `k` advancing per
/// draw.
/// Plane A must never implement it with an unkeyed RNG (V-DET-19).
pub trait UniformDraw {
    /// Returns the next uniform draw.
    fn draw_u64(&mut self) -> u64;
}

/// The keyed stream's next draw: [`RngStream::next_u64`] (BCK-002 A-21). A stream from
/// `StepCtx::stream` starts at draw 0, so the `n`-th call reads draw `n - 1` of its key.
impl UniformDraw for RngStream {
    fn draw_u64(&mut self) -> u64 {
        self.next_u64()
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Q16_16 {}
    impl Sealed for super::Prob32 {}
}

/// A probability a uniform draw can be compared with: `Q16_16` or `Prob32`. Sealed: the mapping
/// of each type is exact and documented (this module, `prob.rs`), and no other type may join.
pub trait Probability: Copy + sealed::Sealed {
    /// Whether `draw / 2^64 < self`.
    fn hit_by(self, draw: u64) -> bool;
}

impl Probability for Q16_16 {
    fn hit_by(self, draw: u64) -> bool {
        q16_hits(draw, self)
    }
}

impl Probability for Prob32 {
    fn hit_by(self, draw: u64) -> bool {
        self.hits(draw)
    }
}

/// `draw / 2^64 < p` for a `Q16_16` (module docs).
const fn q16_hits(draw: u64, p: Q16_16) -> bool {
    let bits = p.to_bits();
    if bits <= 0 {
        false
    } else if bits >= 1 << Q16_16::FRAC_BITS {
        true
    } else {
        // 0 < bits < 2^16, so the cast is exact and the shift cannot overflow.
        draw < (bits.cast_unsigned() as u64) << (64 - Q16_16::FRAC_BITS)
    }
}

/// Returns whether the draw `draw` hits the probability `p`: `draw / 2^64 < p` (module docs for
/// `Q16_16`, `prob.rs` for `Prob32`).
#[must_use]
pub fn draw_hits<P: Probability>(draw: u64, p: P) -> bool {
    p.hit_by(draw)
}

/// Takes one draw from `source` and returns whether it hits `p` (`draw_hits`).
///
/// Exactly one draw is taken whatever `p` is, even for `p <= 0` or `p >= 1`, so the draw counter of
/// a keyed RNG advances the same way on every branch of the history.
pub fn chance<D: UniformDraw + ?Sized, P: Probability>(source: &mut D, p: P) -> bool {
    draw_hits(source.draw_u64(), p)
}

#[cfg(test)]
mod tests {
    use rh_ids::{BranchSalt, PurposeId, RngStream, SystemId, SystemKey};

    use super::{Prob32, Q16_16, UniformDraw, chance, draw_hits};

    /// A fixed sequence of draws: the trait's test double.
    struct Fixed(Vec<u64>);

    impl UniformDraw for Fixed {
        fn draw_u64(&mut self) -> u64 {
            self.0.remove(0)
        }
    }

    #[test]
    fn the_smallest_step_hits_exactly_the_draws_below_two_to_the_48() {
        assert!(draw_hits(0, Q16_16::EPSILON));
        assert!(draw_hits((1 << 48) - 1, Q16_16::EPSILON));
        assert!(!draw_hits(1 << 48, Q16_16::EPSILON));
        let almost_one = Q16_16::from_bits(65_535);
        assert!(draw_hits((65_535 << 48) - 1, almost_one));
        assert!(!draw_hits(65_535 << 48, almost_one));
        assert!(!draw_hits(u64::MAX, almost_one));
    }

    #[test]
    fn zero_never_hits_and_one_always_hits() {
        for draw in [0, 1, 1 << 48, u64::MAX - 1, u64::MAX] {
            assert!(!draw_hits(draw, Q16_16::ZERO));
            assert!(!draw_hits(draw, Q16_16::MIN));
            assert!(draw_hits(draw, Q16_16::ONE));
            assert!(draw_hits(draw, Q16_16::MAX));
        }
    }

    #[test]
    fn chance_takes_exactly_one_draw_per_call() {
        let mut source = Fixed(vec![0, u64::MAX, 1 << 47, 1 << 63]);
        let half = Q16_16::from_bits(32_768);
        assert!(!chance(&mut source, Q16_16::ZERO)); // consumes 0 even though p = 0
        assert!(chance(&mut source, Q16_16::ONE)); // consumes u64::MAX
        assert!(chance(&mut source, half)); // 2^47 < 2^63
        assert!(!chance(&mut source, half)); // 2^63 is not below 2^63
        assert!(source.0.is_empty());
    }

    #[test]
    fn prob32_chance_takes_exactly_one_draw_per_call_whatever_p_is() {
        let mut source = Fixed(vec![0, u64::MAX, (1 << 32) - 1, 1 << 32]);
        assert!(!chance(&mut source, Prob32::ZERO)); // consumes 0 even though p = 0
        assert!(chance(&mut source, Prob32::ONE)); // consumes u64::MAX
        assert!(chance(&mut source, Prob32::EPSILON)); // 2^32 - 1 < 2^32
        assert!(!chance(&mut source, Prob32::EPSILON)); // 2^32 is not below 2^32
        assert!(source.0.is_empty());
    }

    #[test]
    fn a_keyed_stream_is_a_uniform_draw_source_read_in_order() {
        let key = SystemKey::new(7, BranchSalt::from_raw(0), SystemId::from_raw(3), 0)
            .stream_key(PurposeId::of("death_time"), 11);
        let mut stream = RngStream::from_stream_key(&key, 43_200);
        for k in 0..5 {
            assert_eq!(stream.draw_u64(), key.draw(43_200, k));
        }
        for p in [Prob32::ZERO, Prob32::ONE] {
            let before = stream.position();
            let _ = chance(&mut stream, p);
            assert_eq!(stream.position(), before + 1, "one draw whatever p is");
        }
    }
}
