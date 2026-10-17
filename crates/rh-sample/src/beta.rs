//! The beta distribution `Beta(α, β)`, from two gamma variates.
//!
//! If `X ~ Gamma(α, 1)` and `Y ~ Gamma(β, 1)` are independent, `X/(X + Y) ~ Beta(α, β)` (the
//! construction the sprint names; `rand_distr` uses Cheng's BB/BC algorithms instead, so nothing
//! here is ported). Both gammas come from `gamma::UnitGamma` (Marsaglia–Tsang).
//!
//! DECISION: compute in logarithms, `B = 1/(1 + e^(ln Y − ln X))`. For a shape near `2^-16` a
//! gamma value is often below the smallest `f64` (its boost `u^(1/k)` underflows), so `X/(X + Y)`
//! would be `0/0`; `ln X` and `ln Y` stay finite. ALTERNATIVES CONSIDERED: `X/(X + Y)` with a
//! re-draw on `0/0` (draw counts would depend on underflow, and the result is biased exactly where
//! underflow happens).
//!
//! The result is a `Q16_16` in `[0, 1]`, rounded half to even from its exact `f64` value.
//!
//! # Draws consumed
//!
//! A sample reads the stream sequentially from its current position: first all of `X`'s draws,
//! then all of `Y`'s (each as `gamma` documents), deterministic given the stream. Measured
//! (VER-SPR-003-TEST §2.2, confirmed here): about 4.0 to 4.2 draws per sample when both shapes are
//! at least 1, and 5.1 to 6.2 when either is below 1 (its boost adds a draw, and Marsaglia–Tsang
//! at `k + 1` rejects more often).
//!
//! Because the count varies with the parameters, a call site gives each sample its own keyed stream
//! (or its own fixed draw offset `k`), never several samples from one stream: otherwise a parameter
//! that differs between two branches shifts every later sample (ADR-0006 item 3, common random
//! numbers; `05.1` §6 item 3).
//!
//! REF: SPR-003 T-04 ("Beta from two Gammas"); ADR-0006 item 3; `05.1` §6 item 3.

use rh_core::fixed::Q16_16;
use rh_ids::RngStream;

use crate::convert::q16_16_from_f64;
use crate::error::SampleError;
use crate::gamma::UnitGamma;

/// The beta distribution with shapes `α` and `β` (mean `α/(α + β)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beta {
    x: UnitGamma,
    y: UnitGamma,
}

impl Beta {
    /// The beta distribution with shapes `alpha` and `beta`.
    ///
    /// # Errors
    ///
    /// If either shape is not positive.
    pub fn new(alpha: Q16_16, beta: Q16_16) -> Result<Self, SampleError> {
        Ok(Self {
            x: UnitGamma::new(alpha, "Beta")?,
            y: UnitGamma::new(beta, "Beta")?,
        })
    }

    /// Draws one value in `[0, 1]`, rounded to a `Q16_16`, reading `stream` from its current
    /// position (module docs: how many draws).
    pub fn sample(&self, stream: &mut RngStream) -> Q16_16 {
        let ln_x = self.x.sample_ln(stream);
        let ln_y = self.y.sample_ln(stream);
        // e^(ln_y - ln_x) may overflow to +inf, giving exactly 0: the right limit.
        q16_16_from_f64(1.0 / (1.0 + libm::exp(ln_y - ln_x)))
    }
}

#[cfg(test)]
mod tests {
    use super::Beta;
    use rh_core::fixed::Q16_16;
    use rh_ids::{BranchSalt, PurposeId, RngKey, RngStream, SystemId};

    fn stream(entity: u64) -> RngStream {
        RngStream::new(&RngKey {
            world_seed: 3,
            branch_salt: BranchSalt::from_raw(0),
            system: SystemId::from_raw(1),
            purpose: PurposeId::of("beta_unit_test"),
            entity,
            epoch: 0,
            t: 0,
        })
    }

    #[test]
    fn non_positive_shapes_are_refused() {
        let err = Beta::new(Q16_16::ONE, Q16_16::ZERO).unwrap_err();
        assert_eq!(err.to_string(), "Beta: the shape must be positive");
        assert!(Beta::new(Q16_16::from_bits(-5), Q16_16::ONE).is_err());
    }

    #[test]
    fn tiny_shapes_stay_in_the_unit_interval() {
        for entity in 0..500 {
            let mut s = stream(entity);
            for (a, b) in [
                (Q16_16::EPSILON, Q16_16::EPSILON),
                (Q16_16::EPSILON, Q16_16::from_int(1000)),
                (Q16_16::from_int(1000), Q16_16::EPSILON),
            ] {
                let x = Beta::new(a, b).unwrap().sample(&mut s);
                assert!((Q16_16::ZERO..=Q16_16::ONE).contains(&x), "{x:?}");
            }
        }
    }
}
