// Ported from rand_distr 0.6.0, src/normal.rs and src/utils.rs (`ziggurat`):
//
// Copyright 2018 Developers of the Rand project.
// Copyright 2013 The Rust Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! The normal distribution `N(μ, σ²)`, by the ZIGNOR ziggurat.
//!
//! J. A. Doornik (2005), "An Improved Ziggurat Method to Generate Normal Random Samples", Nuffield
//! College, Oxford, as ported from `rand_distr` 0.6.0, with fixed tables (`ziggurat_tables`).
//! Changes for plane A:
//!
//! - **Keyed draws:** one `u64` per attempt from `rh_ids::RngStream`; its low 8 bits choose the box
//!   and its top 52 bits give `u ∈ [−1, 1)` (`uniform::symmetric`), as upstream.
//! - **`libm`** for `exp` (the density) and `log` (the tail).
//! - **Fixed-point parameters and result:** `μ` and `σ` are `Q16_16` (exact in `f64`); the result
//!   `μ + σ·z` is rounded half to even from its exact `f64` value to a `Q16_16`, saturating.
//!
//! # Draws consumed
//!
//! A sample reads the stream sequentially from its current position. Each attempt takes **one
//! draw**; an attempt that lands in a box's wedge takes **one more** for the density test; the base
//! box's tail takes **two per tail iteration**. About 1.02 draws per sample on average; the count is
//! deterministic given the stream and does not depend on `μ` or `σ` (a sample with `σ = 0` still
//! draws, so draw positions never depend on the parameters).
//!
//! REF: SPR-003 T-03; ADR-0006 item 3; `05.1` §6 item 3 ("normal (ziggurat with fixed tables)");
//! `docs/14` (`rand_distr`: ZIGNOR, PORT).

use rh_core::fixed::Q16_16;
use rh_ids::RngStream;

use crate::convert::{f64_from_q16_16, q16_16_from_f64};
use crate::error::SampleError;
use crate::uniform::{next_closed_open, next_open, symmetric};
use crate::ziggurat_tables::{ZIG_NORM_F, ZIG_NORM_R, ZIG_NORM_X};

/// The normal distribution with mean `μ` and standard deviation `σ`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Normal {
    mean: f64,
    sd: f64,
}

impl Normal {
    /// The normal distribution with mean `mean` and standard deviation `sd`.
    ///
    /// # Errors
    ///
    /// If `sd` is negative.
    pub fn new(mean: Q16_16, sd: Q16_16) -> Result<Self, SampleError> {
        if sd < Q16_16::ZERO {
            return Err(SampleError::new(
                "Normal",
                "the standard deviation must not be negative",
            ));
        }
        Ok(Self {
            mean: f64_from_q16_16(mean),
            sd: f64_from_q16_16(sd),
        })
    }

    /// Draws one value, `μ + σ·z` rounded to a `Q16_16` (saturating), reading `stream` from its
    /// current position (module docs: how many draws).
    pub fn sample(&self, stream: &mut RngStream) -> Q16_16 {
        q16_16_from_f64(self.mean + self.sd * standard_normal(stream))
    }
}

/// The standard normal density without its constant, `e^(−x²/2)`.
fn pdf(x: f64) -> f64 {
    libm::exp(-x * x / 2.0)
}

/// The base box's tail beyond `R` (Marsaglia's method), two draws per iteration; `u` is the
/// attempt's sign.
fn tail(stream: &mut RngStream, u: f64) -> f64 {
    // Initial values that make the condition true once (the loop is not a do-while).
    let mut x = 1.0;
    let mut y = 0.0;
    while -2.0 * y < x * x {
        x = libm::log(next_open(stream)) / ZIG_NORM_R;
        y = libm::log(next_open(stream));
    }
    if u < 0.0 {
        x - ZIG_NORM_R
    } else {
        ZIG_NORM_R - x
    }
}

/// A standard normal variate `z`, as an `f64` (inside the float zone only).
pub(crate) fn standard_normal(stream: &mut RngStream) -> f64 {
    loop {
        let bits = stream.next_u64();
        let i = usize::from(bits.to_le_bytes()[0]); // the low 8 bits pick the box
        let u = symmetric(bits); // the top 52 bits
        let x = u * ZIG_NORM_X[i];
        // Equivalent to |u| < X[i+1]/X[i]: inside the box's rectangle.
        if x.abs() < ZIG_NORM_X[i + 1] {
            return x;
        }
        if i == 0 {
            return tail(stream, u);
        }
        // Equivalent to f1 + U·(f0 − f1) < f(x): under the density in the wedge.
        let f0 = ZIG_NORM_F[i];
        let f1 = ZIG_NORM_F[i + 1];
        if f1 + (f0 - f1) * next_closed_open(stream) < pdf(x) {
            return x;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Normal, standard_normal};
    use crate::ziggurat_tables::{ZIG_NORM_F, ZIG_NORM_R, ZIG_NORM_X};
    use rh_core::fixed::Q16_16;
    use rh_ids::{BranchSalt, PurposeId, RngKey, RngStream, SystemId};

    fn stream(entity: u64) -> RngStream {
        RngStream::new(&RngKey {
            world_seed: 3,
            branch_salt: BranchSalt::from_raw(0),
            system: SystemId::from_raw(1),
            purpose: PurposeId::of("normal_unit_test"),
            entity,
            epoch: 0,
            t: 0,
        })
    }

    /// The tables' bits match the hash `bin/gen_sampler_reference.py` computes from the same
    /// literals with Python's correctly rounded `float()`: rustc reads them to the same bits.
    #[test]
    fn tables_match_the_independent_hash() {
        let reference = include_str!("../tests/golden/reference.txt");
        let pinned: u64 = reference
            .lines()
            .find_map(|l| l.strip_prefix("table "))
            .and_then(|h| h.parse().ok())
            .unwrap();
        let words: Vec<u64> = core::iter::once(ZIG_NORM_R)
            .chain(ZIG_NORM_X)
            .chain(ZIG_NORM_F)
            .map(f64::to_bits)
            .collect();
        assert_eq!(rh_ids::rng::h64_words(0, &words), pinned);
    }

    /// The tables' shape.
    #[test]
    fn tables_are_well_formed() {
        assert_eq!(ZIG_NORM_X[1], ZIG_NORM_R);
        assert_eq!((ZIG_NORM_X[256], ZIG_NORM_F[256]), (0.0, 1.0));
        assert!(ZIG_NORM_X.windows(2).all(|w| w[0] > w[1]));
        assert!(ZIG_NORM_F.windows(2).all(|w| w[0] < w[1]));
        for (x, f) in ZIG_NORM_X.iter().zip(ZIG_NORM_F).skip(1) {
            let density = super::pdf(*x);
            assert!((density - f).abs() < 1e-15, "{x}: {density} vs {f}");
        }
    }

    #[test]
    fn negative_sd_is_refused() {
        let err = Normal::new(Q16_16::ZERO, Q16_16::from_bits(-1)).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Normal: the standard deviation must not be negative"
        );
    }

    #[test]
    fn sd_zero_returns_the_mean_and_still_draws() {
        let mean = Q16_16::from_ratio(7, 3).unwrap();
        let mut s = stream(1);
        assert_eq!(
            Normal::new(mean, Q16_16::ZERO).unwrap().sample(&mut s),
            mean
        );
        assert!(s.position() >= 1);
    }

    #[test]
    fn draw_counts_are_one_per_attempt_plus_wedges_and_tails() {
        let mut total = 0;
        for entity in 0..2000 {
            let mut s = stream(entity);
            let z = standard_normal(&mut s);
            assert!(z.is_finite());
            assert!(s.position() >= 1);
            total += s.position();
        }
        assert!(total < 2000 * 11 / 10, "{total} draws for 2000 samples");
    }
}
