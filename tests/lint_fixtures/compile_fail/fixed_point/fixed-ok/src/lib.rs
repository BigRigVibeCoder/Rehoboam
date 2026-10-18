//! Control: well-typed use of rh-core::fixed from a plane-A crate. It must compile, which shows the
//! failures of its sibling crates come from the type barrier, not from a broken fixture.

use rh_core::fixed::{Money, Prob32, Q8_8, Q16_16, Q32_32, draw_hits, exp, prob_from_hazard};

/// Uses every type through its named conversions only.
pub fn well_typed() -> (Q16_16, Money) {
    let threshold = Q8_8::from_int(1).widen();
    let money = Money::from_minor(Q16_16::ONE.to_bits().into());
    (threshold, money)
}

/// Uses Prob32 through its named conversions and the draw mapping (SPR-011 T-01).
pub fn probabilities(draw: u64) -> (bool, Q16_16, Option<Prob32>) {
    let p = Prob32::from_ratio(1, 3).unwrap_or(Prob32::ZERO);
    (draw_hits(draw, p), p.to_q16_16(), Prob32::from_q16_16(Q16_16::ONE))
}

/// Uses Q32_32 through its named conversions and the transcendentals (SPR-016 T-01).
pub fn wide() -> (Q16_16, Option<Prob32>, Option<Prob32>) {
    let y = Q16_16::ONE.widen() * Q32_32::from_int(-1);
    let survival = exp(y).unwrap_or(Q32_32::ZERO);
    (y.narrow_saturating(), Prob32::from_q32(survival), prob_from_hazard(-y).ok())
}
