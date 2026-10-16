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
//! anything that yields uniform `u64` draws. Implementing it for `rh_ids::RngStream` needs a crate
//! edge and waits for the first sprint that draws against a probability (BCK-002 A-21). This
//! module does not depend on `rh-ids` and never creates randomness itself.
//!
//! REF: `CODEX/05_PROJECT/SPR-010_FixedPoint.md` T-03; ADR-0006; `05.1` §6 items 2, 4.

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

/// Returns whether the draw `draw` hits the probability `p`: `draw / 2^64 < p` (module docs).
#[must_use]
pub const fn draw_hits(draw: u64, p: Q16_16) -> bool {
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

/// Takes one draw from `source` and returns whether it hits `p` (`draw_hits`).
///
/// Exactly one draw is taken whatever `p` is, even for `p <= 0` or `p >= 1`, so the draw counter of
/// a keyed RNG advances the same way on every branch of the history.
pub fn chance<D: UniformDraw + ?Sized>(source: &mut D, p: Q16_16) -> bool {
    draw_hits(source.draw_u64(), p)
}

#[cfg(test)]
mod tests {
    use super::{Q16_16, UniformDraw, chance, draw_hits};

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
}
