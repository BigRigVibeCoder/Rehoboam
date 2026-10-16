// Ported from rand_distr 0.6.0, src/gamma.rs:
//
// Copyright 2018 Developers of the Rand project.
// Copyright 2013 The Rust Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! The gamma distribution `Gamma(k, θ)` (shape `k`, scale `θ`), by Marsaglia and Tsang.
//!
//! G. Marsaglia and W. W. Tsang (2000), "A Simple Method for Generating Gamma Variables", *ACM
//! TOMS* 26(3), 363–372, as ported from `rand_distr` 0.6.0. For `k ≥ 1`, with `d = k − 1/3` and
//! `c = 1/√(9d)`: draw `z ~ N(0, 1)` (`normal::standard_normal`), let `v = (1 + c·z)³`, reject if
//! `v ≤ 0`; draw `u ~ U(0, 1)` and accept `d·v` if `u < 1 − 0.0331·z⁴` or
//! `ln u < z²/2 + d·(1 − v + ln v)`. For `k < 1`, the boost of the paper: `Gamma(k) =
//! Gamma(k + 1)·u^(1/k)` with a further uniform `u`, drawn first.
//!
//! Changes for plane A:
//! - **Fixed-point parameters and result**, both `Q16_16`; the result is rounded half to even from
//!   its exact `f64` value and saturates at `Q16_16::MAX`.
//! - **`k = 1` uses Marsaglia–Tsang too.** `rand_distr` switches to an exponential ziggurat at
//!   `k = 1`; one method for every `k ≥ 1` keeps one code path and one set of tables. The method
//!   is valid for every `k ≥ 1` (the paper's condition).
//! - **Keyed draws and `libm`** (`log`, `pow`), as everywhere in this crate.
//!
//! # Draws consumed
//!
//! A sample reads the stream sequentially from its current position. `k < 1`: **one draw** for
//! the boost's `u`, then as for `k + 1`. `k ≥ 1`: each attempt takes the standard normal's draws
//! (about 1.02, see `normal`) and, unless `v ≤ 0`, **one draw** for `u`; the acceptance rate is above
//! 0.95 for every `k ≥ 1`, so about 2.1 draws per sample. The count is deterministic given the
//! stream and does not depend on `θ`.
//!
//! REF: SPR-003 T-04; ADR-0006 item 3; `05.1` §6 item 3; `docs/14` (`rand_distr`: PORT).

use rh_core::fixed::Q16_16;
use rh_ids::RngStream;

use crate::convert::{f64_from_q16_16, q16_16_from_f64};
use crate::error::SampleError;
use crate::normal::standard_normal;
use crate::uniform::next_open;

/// The gamma distribution with shape `k` and scale `θ` (mean `k·θ`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gamma {
    unit: UnitGamma,
    scale: f64,
}

impl Gamma {
    /// The gamma distribution with shape `shape` and scale `scale`.
    ///
    /// # Errors
    ///
    /// If `shape` or `scale` is not positive.
    pub fn new(shape: Q16_16, scale: Q16_16) -> Result<Self, SampleError> {
        if scale <= Q16_16::ZERO {
            return Err(SampleError::new("Gamma", "the scale must be positive"));
        }
        Ok(Self {
            unit: UnitGamma::new(shape, "Gamma")?,
            scale: f64_from_q16_16(scale),
        })
    }

    /// Draws one value, rounded to a `Q16_16` (saturating), reading `stream` from its current
    /// position (module docs: how many draws).
    pub fn sample(&self, stream: &mut RngStream) -> Q16_16 {
        q16_16_from_f64(self.unit.sample_scaled(stream, self.scale))
    }
}

/// `Gamma(k, 1)` inside the float zone: what `Gamma` and `Beta` share.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UnitGamma {
    /// `1/k` for `k < 1` (the boost's exponent); `None` for `k ≥ 1`.
    inv_shape: Option<f64>,
    /// Marsaglia–Tsang's `d = k' − 1/3`, with `k' = k` or `k + 1` (boosted).
    d: f64,
    /// Marsaglia–Tsang's `c = 1/√(9d)`.
    c: f64,
}

impl UnitGamma {
    /// `Gamma(shape, 1)`; `sampler` names the public type in an error.
    pub(crate) fn new(shape: Q16_16, sampler: &'static str) -> Result<Self, SampleError> {
        if shape <= Q16_16::ZERO {
            return Err(SampleError::new(sampler, "the shape must be positive"));
        }
        let k = f64_from_q16_16(shape);
        let (inv_shape, boosted) = if k < 1.0 {
            (Some(1.0 / k), k + 1.0)
        } else {
            (None, k)
        };
        let d = boosted - 1.0 / 3.0;
        Ok(Self {
            inv_shape,
            d,
            c: 1.0 / (9.0 * d).sqrt(),
        })
    }

    /// Marsaglia–Tsang's `v` for shape `d + 1/3 ≥ 1` (the sample is `d·v`).
    fn sample_v(&self, stream: &mut RngStream) -> f64 {
        loop {
            let z = standard_normal(stream);
            let v_cbrt = 1.0 + self.c * z;
            if v_cbrt <= 0.0 {
                continue;
            }
            let v = v_cbrt * v_cbrt * v_cbrt;
            let u = next_open(stream);
            let z2 = z * z;
            if u < 1.0 - 0.0331 * z2 * z2
                || libm::log(u) < 0.5 * z2 + self.d * (1.0 - v + libm::log(v))
            {
                return v;
            }
        }
    }

    /// A `Gamma(k, θ)` value, `θ` applied last (as upstream: no `∞·0` from an early product).
    pub(crate) fn sample_scaled(&self, stream: &mut RngStream, scale: f64) -> f64 {
        match self.inv_shape {
            None => self.sample_v(stream) * (self.d * scale),
            Some(inv_shape) => {
                let u = next_open(stream);
                let v = self.sample_v(stream);
                (v * libm::pow(u, inv_shape) * self.d) * scale
            }
        }
    }

    /// `ln` of a `Gamma(k, 1)` value, without underflow: `u^(1/k)` for a small `k` is often below
    /// the smallest `f64`, but its logarithm `ln(u)/k` is not. The draws are those of
    /// [`UnitGamma::sample_scaled`].
    pub(crate) fn sample_ln(&self, stream: &mut RngStream) -> f64 {
        match self.inv_shape {
            None => libm::log(self.sample_v(stream) * self.d),
            Some(inv_shape) => {
                let u = next_open(stream);
                let v = self.sample_v(stream);
                libm::log(v * self.d) + libm::log(u) * inv_shape
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Gamma;
    use rh_core::fixed::Q16_16;
    use rh_ids::{BranchSalt, PurposeId, RngKey, RngStream, SystemId};

    fn stream(entity: u64) -> RngStream {
        RngStream::new(&RngKey {
            world_seed: 3,
            branch_salt: BranchSalt::from_raw(0),
            system: SystemId::from_raw(1),
            purpose: PurposeId::of("gamma_unit_test"),
            entity,
            epoch: 0,
            t: 0,
        })
    }

    #[test]
    fn non_positive_parameters_are_refused() {
        let one = Q16_16::ONE;
        assert_eq!(
            Gamma::new(Q16_16::ZERO, one).unwrap_err().to_string(),
            "Gamma: the shape must be positive"
        );
        assert_eq!(
            Gamma::new(one, Q16_16::from_bits(-1))
                .unwrap_err()
                .to_string(),
            "Gamma: the scale must be positive"
        );
    }

    #[test]
    fn samples_are_non_negative_and_saturate_at_max() {
        for entity in 0..500 {
            let mut s = stream(entity);
            for (shape, scale) in [
                (Q16_16::EPSILON, Q16_16::ONE),
                (Q16_16::ONE, Q16_16::ONE),
                (Q16_16::from_int(3), Q16_16::EPSILON),
            ] {
                assert!(Gamma::new(shape, scale).unwrap().sample(&mut s) >= Q16_16::ZERO);
            }
            let huge = Gamma::new(Q16_16::from_int(30_000), Q16_16::from_int(30_000)).unwrap();
            assert_eq!(huge.sample(&mut s), Q16_16::MAX);
        }
    }
}
