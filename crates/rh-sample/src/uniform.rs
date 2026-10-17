//! Uniform floats from keyed `u64` draws: the only way randomness enters the float zone.
//!
//! Each function maps one draw to one float through its top 52 bits, by placing them in the
//! significand of a number in `[1, 2)` and subtracting a constant. The subtraction is exact
//! (Sterbenz), so the result is a pure function of the draw's bits, with no rounding.
//!
//! These are the conversions `rand` 0.9 uses for `Standard`/`Open01` and the ziggurat
//! (`rand_distr`, MIT OR Apache-2.0), rewritten here on `rh_ids::RngStream`.
//!
//! REF: SPR-003 ruling item 4 (draws come from `rh-ids`'s keyed streams); ADR-0006 item 2.

use rh_ids::RngStream;

/// The bit pattern of `1.0`: exponent 0, empty significand.
const ONE_BITS: u64 = 0x3ff0_0000_0000_0000;
/// The bit pattern of `2.0`: exponent 1, empty significand.
const TWO_BITS: u64 = 0x4000_0000_0000_0000;
/// How far a draw is shifted to leave its top 52 bits.
const SIGNIFICAND_SHIFT: u32 = 12;
/// `1 − 2^-53`: subtracted from `[1, 2)` to land in the open interval `(0, 1)`.
const ONE_MINUS_HALF_EPSILON: f64 = 1.0 - f64::EPSILON / 2.0;

/// `[1, 2)` from the top 52 bits of `draw`.
fn one_to_two(draw: u64) -> f64 {
    f64::from_bits(ONE_BITS | (draw >> SIGNIFICAND_SHIFT))
}

/// A uniform value in `[0, 1)` on the grid of `2^-52`.
pub(crate) fn closed_open(draw: u64) -> f64 {
    one_to_two(draw) - 1.0
}

/// A uniform value in the open interval `(0, 1)`: `[0, 1)` shifted up by `2^-53`. Never 0, so its
/// logarithm is always finite.
pub(crate) fn open(draw: u64) -> f64 {
    one_to_two(draw) - ONE_MINUS_HALF_EPSILON
}

/// A uniform value in `[-1, 1)` on the grid of `2^-51`, from the top 52 bits of `draw` (the
/// ziggurat's symmetric `u`).
pub(crate) fn symmetric(draw: u64) -> f64 {
    f64::from_bits(TWO_BITS | (draw >> SIGNIFICAND_SHIFT)) - 3.0
}

/// The next draw of `stream`, mapped by [`closed_open`]. Consumes one draw.
pub(crate) fn next_closed_open(stream: &mut RngStream) -> f64 {
    closed_open(stream.next_u64())
}

/// The next draw of `stream`, mapped by [`open`]. Consumes one draw.
pub(crate) fn next_open(stream: &mut RngStream) -> f64 {
    open(stream.next_u64())
}

#[cfg(test)]
mod tests {
    use super::{closed_open, open, symmetric};

    #[test]
    fn the_extremes_are_exact() {
        assert_eq!(closed_open(0), 0.0);
        assert_eq!(closed_open(u64::MAX), 1.0 - f64::EPSILON);
        assert_eq!(open(0), f64::EPSILON / 2.0);
        assert_eq!(open(u64::MAX), 1.0 - f64::EPSILON / 2.0);
        assert_eq!(symmetric(0), -1.0);
        assert_eq!(symmetric(1 << 63), 0.0);
        assert_eq!(symmetric(u64::MAX), 1.0 - 2.0 * f64::EPSILON);
    }

    #[test]
    fn only_the_top_52_bits_count() {
        assert_eq!(closed_open(0xfff), 0.0);
        assert_eq!(closed_open(1 << 12), f64::EPSILON);
        assert_eq!(closed_open(1 << 63), 0.5);
    }
}
