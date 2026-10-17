// Ported from rand_distr 0.6.0, src/binomial.rs:
//
// Copyright 2018 Developers of the Rand project.
// Copyright 2016-2017 The Rust Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! The binomial distribution `Binomial(n, p)`: BINV inversion for small `n·p`, BTPE otherwise.
//!
//! Kachitvichyanukul and Schmeiser (1988), "Binomial random variate generation", Commun. ACM 31(2),
//! 216–222, as ported from `rand_distr` 0.6.0, with these changes for plane A:
//!
//! - **Fixed-point parameters, integer result.** `p` is a `Q16_16` in `[0, 1]`, so it and `1 − p`
//!   are exact in `f64`, and `n` is a `u32`, so every count is exact too. The result is a `u32`.
//! - **Keyed draws.** Uniforms come from `rh_ids::RngStream` through `uniform` (one draw each).
//! - **`libm`** for `pow` and `ln`; `std`'s `sqrt` (correctly rounded).
//! - **Exact truncations.** `rand_distr` converts with `as u64`; here [`floor_to_i64`] reads the
//!   exact integer, and a value outside `[0, n]` is rejected and re-drawn instead of saturated.
//! - **No Poisson branch.** `rand_distr` switches to a Poisson sampler when `1 − p` rounds to 1;
//!   with `p ≥ 2^-16` that never happens here.
//! - **BINV stops at `n`.** A BINV attempt restarts if its count passes `min(110, n)` (`rand_distr`:
//!   110 only), so the result never exceeds `n`, even through a rounding accident.
//!
//! # Draws consumed
//!
//! A sample reads the stream sequentially from its current position:
//! - `p = 0`, `p = 1` or `n = 0`: **no draw**.
//! - BINV (`n·min(p, 1 − p) < 10`): **one draw per attempt**; an attempt fails only if its count
//!   passes `min(110, n)`, which for these parameters is a > 30-standard-deviation event, so in
//!   practice one draw.
//! - BTPE (otherwise): **two draws per attempt** (`u`, then `v`), and a variable number of attempts,
//!   deterministic given the stream. Measured (SPR-003 VER-SPR-003-TEST §2.2, confirmed here):
//!   about 2 attempts (4 draws) for `n·p` near 10 to 20, 1.5 (3 draws) at `n·p = 50`, and about
//!   1.15 (2.3 draws) from `n·p ≈ 10^3` on.
//!
//! Because the count varies with the parameters, a call site gives each sample its own keyed stream
//! (or its own fixed draw offset `k`), never several samples from one stream: otherwise a parameter
//! that differs between two branches shifts every later sample (ADR-0006 item 3, common random
//! numbers; `05.1` §6 item 3).
//!
//! REF: SPR-003 T-01; ADR-0006 item 3; `05.1` §6 item 3; `docs/14` (`rand_distr`: PORT).

use core::cmp::Ordering;

use rh_core::fixed::Q16_16;
use rh_ids::RngStream;

use crate::convert::{f64_from_q16_16, floor_to_i64};
use crate::error::SampleError;
use crate::uniform::next_closed_open;

/// Below this `n·p`, BINV (inversion) is used; at or above it, BTPE. The paper suggests 10.
const BINV_THRESHOLD: f64 = 10.0;
/// A BINV attempt restarts past this count (GSL's value, as in `rand_distr`).
const BINV_MAX_X: u32 = 110;
/// BTPE's squeeze is used for `|y − m|` above this (Ranlib and GSL use 20).
const SQUEEZE_THRESHOLD: u32 = 20;

/// The binomial distribution `Binomial(n, p)`: the number of successes in `n` independent trials,
/// each a success with probability `p`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Binomial {
    method: Method,
}

/// How a [`Binomial`] samples, with its constants precomputed.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Method {
    /// Always this count: `p = 0`, `p = 1` or `n = 0`.
    Constant(u32),
    /// Inversion; `flipped` samples `n − X` for `p > 1/2`.
    Binv(Binv, bool),
    /// Triangle, parallelogram, exponential tails; `flipped` as above.
    Btpe(Btpe, bool),
}

/// BINV's constants: `r = q^n`, `s = p/q`, `a = (n + 1)·s`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Binv {
    r: f64,
    s: f64,
    a: f64,
    n: u32,
}

/// BTPE's constants: `n`, `p ≤ 1/2`, the mode `m` and the triangle's half-width `p1`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Btpe {
    n: u32,
    p: f64,
    m: u32,
    p1: f64,
}

impl Binomial {
    /// The binomial distribution of `n` trials with success probability `p`.
    ///
    /// # Errors
    ///
    /// If `p` is outside `[0, 1]`.
    pub fn new(n: u32, p: Q16_16) -> Result<Self, SampleError> {
        if p < Q16_16::ZERO || p > Q16_16::ONE {
            return Err(SampleError::new("Binomial", "p must be in [0, 1]"));
        }
        if p == Q16_16::ZERO || n == 0 {
            return Ok(Self {
                method: Method::Constant(0),
            });
        }
        if p == Q16_16::ONE {
            return Ok(Self {
                method: Method::Constant(n),
            });
        }
        let p = f64_from_q16_16(p);
        // The distribution is symmetric under p -> 1 - p (exact for a Q16_16 p).
        let flipped = p > 0.5;
        let p = if flipped { 1.0 - p } else { p };
        let q = 1.0 - p;
        let n_f = f64::from(n);
        let np = n_f * p; // exact: a 32-bit n times a 17-bit p
        let method = if np < BINV_THRESHOLD {
            let s = p / q;
            Method::Binv(
                Binv {
                    r: libm::pow(q, n_f),
                    s,
                    a: (n_f + 1.0) * s,
                    n,
                },
                flipped,
            )
        } else {
            let npq = np * q;
            let p1 = libm::floor(2.195 * npq.sqrt() - 4.6 * q) + 0.5;
            // INVARIANT: 0 <= np + p <= n + 1 <= 2^32, so the floor is a u32 (n + 1 only if
            // p were 1, which is a constant above).
            let m = floor_to_i64(np + p)
                .and_then(|m| u32::try_from(m).ok())
                .unwrap_or(0);
            Method::Btpe(Btpe { n, p, m, p1 }, flipped)
        };
        Ok(Self { method })
    }

    /// Draws one count in `[0, n]`, reading `stream` from its current position (module docs: how
    /// many draws).
    pub fn sample(&self, stream: &mut RngStream) -> u32 {
        match self.method {
            Method::Constant(c) => c,
            Method::Binv(binv, flipped) => flip(binv.n, sample_binv(binv, stream), flipped),
            Method::Btpe(btpe, flipped) => flip(btpe.n, sample_btpe(btpe, stream), flipped),
        }
    }
}

/// `n − x` if `flipped`, else `x`; `x <= n` by construction.
fn flip(n: u32, x: u32, flipped: bool) -> u32 {
    if flipped { n - x } else { x }
}

/// BINV: sequential search of the inverse CDF from 0, one draw per attempt.
fn sample_binv(binv: Binv, stream: &mut RngStream) -> u32 {
    let cap = BINV_MAX_X.min(binv.n);
    'attempt: loop {
        let mut r = binv.r;
        let mut u = next_closed_open(stream);
        let mut x: u32 = 0;
        while u > r {
            u -= r;
            x += 1;
            if x > cap {
                continue 'attempt;
            }
            r *= binv.a / f64::from(x) - binv.s;
        }
        return x;
    }
}

/// `a·(1 + a/2)`: BTPE's exponential-tail rate from the slope at a tail's edge.
fn tail_rate(a: f64) -> f64 {
    a * (1.0 + 0.5 * a)
}

/// The Stirling-series correction of BTPE step 5.3.
fn stirling(a: f64) -> f64 {
    let a2 = a * a;
    (13860.0 - (462.0 - (132.0 - (99.0 - 140.0 / a2) / a2) / a2) / a2) / a / 166_320.0
}

/// `⌊x⌋` if it is a count in `[0, n]`.
fn count_in(x: f64, n: u32) -> Option<u32> {
    floor_to_i64(x)
        .and_then(|y| u32::try_from(y).ok())
        .filter(|&y| y <= n)
}

/// BTPE, steps 0 to 5, two draws per attempt. Same names as the paper.
#[allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "the paper's notation, kept so the port can be checked line by line"
)]
fn sample_btpe(btpe: Btpe, stream: &mut RngStream) -> u32 {
    // Step 0: constants as functions of n and p.
    let n = btpe.n;
    let n_f = f64::from(n);
    let np = n_f * btpe.p;
    let q = 1.0 - btpe.p;
    let npq = np * q;
    let f_m = np + btpe.p;
    let m = btpe.m;
    let m_f = f64::from(m);
    let p1 = btpe.p1; // half-width of the triangle; with height 1, also its area
    let x_m = m_f + 0.5; // tip of the triangle
    let x_l = x_m - p1; // left edge
    let x_r = x_m + p1; // right edge
    let c = 0.134 + 20.5 / (15.3 + m_f);
    let p2 = p1 * (1.0 + 2.0 * c); // p1 + the parallelograms' area
    let lambda_l = tail_rate((f_m - x_l) / (f_m - x_l * btpe.p));
    let lambda_r = tail_rate((x_r - f_m) / (x_r * q));
    let p3 = p2 + c / lambda_l;
    let p4 = p3 + c / lambda_r;

    loop {
        // Step 1: u selects the region; region 1 is a triangular variate directly.
        let u = p4 * next_closed_open(stream);
        let mut v = next_closed_open(stream);
        let y: u32;
        if u <= p1 {
            match count_in(x_m - p1 * v + u, n) {
                Some(found) => return found,
                None => continue,
            }
        }
        if u <= p2 {
            // Step 2: the parallelograms.
            let x = x_l + (u - p1) / c;
            v = v * c + 1.0 - (x - x_m).abs() / p1;
            if v > 1.0 {
                continue;
            }
            let Some(found) = count_in(x, n) else {
                continue;
            };
            y = found;
        } else if u <= p3 {
            // Step 3: the left exponential tail.
            let y_tmp = x_l + libm::log(v) / lambda_l;
            let Some(found) = count_in(y_tmp, n) else {
                continue;
            };
            y = found;
            v *= (u - p2) * lambda_l;
        } else {
            // Step 4: the right exponential tail.
            let Some(found) = count_in(x_r - libm::log(v) / lambda_r, n) else {
                continue;
            };
            y = found;
            v *= (u - p3) * lambda_r;
        }

        // Step 5.0: choose how to evaluate f(y).
        let k = y.abs_diff(m);
        let k_f = f64::from(k);
        if !(k > SQUEEZE_THRESHOLD && k_f < 0.5 * npq - 1.0) {
            // Step 5.1: f(y) by the recurrence from the mode.
            let s = btpe.p / q;
            let a = s * (n_f + 1.0);
            let mut f = 1.0;
            match m.cmp(&y) {
                Ordering::Less => {
                    for i in (m + 1)..=y {
                        f *= a / f64::from(i) - s;
                    }
                }
                Ordering::Greater => {
                    for i in (y + 1)..=m {
                        f /= a / f64::from(i) - s;
                    }
                }
                Ordering::Equal => {}
            }
            if v > f {
                continue;
            }
            return y;
        }

        // Step 5.2: squeeze ln(v) between bounds on ln(f(y)).
        let rho = (k_f / npq) * ((k_f * (k_f / 3.0 + 0.625) + 1.0 / 6.0) / npq + 0.5);
        let t = -0.5 * k_f * k_f / npq;
        let alpha = libm::log(v);
        if alpha < t - rho {
            return y;
        }
        if alpha > t + rho {
            continue;
        }

        // Step 5.3: the final test (GSL's signs, as in rand_distr: verified by one of the
        // algorithm's designers, according to the GSL authors).
        let x1 = f64::from(y) + 1.0;
        let f1 = m_f + 1.0;
        let z = f64::from(n - m) + 1.0;
        let w = f64::from(n - y) + 1.0;
        let y_sub_m = f64::from(y) - m_f; // exact: both are 32-bit integers
        let bound = x_m * libm::log(f1 / x1)
            + (f64::from(n - m) + 0.5) * libm::log(z / w)
            + y_sub_m * libm::log(w * btpe.p / (x1 * q))
            + stirling(f1)
            + stirling(z)
            - stirling(x1)
            - stirling(w);
        if alpha > bound {
            continue;
        }
        return y;
    }
}

#[cfg(test)]
mod tests {
    use super::{Binomial, Method};
    use rh_core::fixed::Q16_16;
    use rh_ids::{BranchSalt, PurposeId, RngKey, RngStream, SystemId};

    fn stream(entity: u64) -> RngStream {
        RngStream::new(&RngKey {
            world_seed: 3,
            branch_salt: BranchSalt::from_raw(0),
            system: SystemId::from_raw(1),
            purpose: PurposeId::of("binomial_unit_test"),
            entity,
            epoch: 0,
            t: 0,
        })
    }

    fn p(num: i64, den: i64) -> Q16_16 {
        Q16_16::from_ratio(num, den).unwrap()
    }

    #[test]
    fn parameters_outside_the_domain_are_refused() {
        assert!(Binomial::new(10, Q16_16::from_bits(-1)).is_err());
        assert!(Binomial::new(10, Q16_16::from_bits(65_537)).is_err());
        assert_eq!(
            Binomial::new(10, Q16_16::from_int(2))
                .unwrap_err()
                .to_string(),
            "Binomial: p must be in [0, 1]"
        );
    }

    #[test]
    fn degenerate_cases_are_constant_and_draw_nothing() {
        let mut s = stream(0);
        for (n, prob, want) in [
            (10, Q16_16::ZERO, 0),
            (10, Q16_16::ONE, 10),
            (0, p(1, 2), 0),
        ] {
            let b = Binomial::new(n, prob).unwrap();
            assert_eq!(b.method, Method::Constant(want));
            assert_eq!(b.sample(&mut s), want);
        }
        assert_eq!(s.position(), 0);
    }

    #[test]
    fn binv_and_btpe_are_chosen_by_n_p() {
        assert!(matches!(
            Binomial::new(19, p(1, 2)).unwrap().method,
            Method::Binv(_, false)
        ));
        assert!(matches!(
            Binomial::new(20, p(1, 2)).unwrap().method,
            Method::Btpe(_, false)
        ));
        assert!(matches!(
            Binomial::new(10, p(3, 4)).unwrap().method,
            Method::Binv(_, true)
        ));
        assert!(matches!(
            Binomial::new(1000, p(3, 4)).unwrap().method,
            Method::Btpe(_, true)
        ));
    }

    #[test]
    fn binv_takes_one_draw_and_btpe_an_even_number() {
        for entity in 0..200 {
            let mut s = stream(entity);
            let x = Binomial::new(30, p(1, 10)).unwrap().sample(&mut s);
            assert!(x <= 30);
            assert_eq!(s.position(), 1);
            let mut s = stream(entity);
            let x = Binomial::new(5000, p(3, 10)).unwrap().sample(&mut s);
            assert!(x <= 5000);
            assert!(s.position() >= 2 && s.position().is_multiple_of(2));
        }
    }

    #[test]
    fn samples_stay_in_range_at_the_extremes() {
        for entity in 0..300 {
            let mut s = stream(entity);
            for (n, prob) in [
                (1, p(1, 2)),
                (u32::MAX, Q16_16::EPSILON),
                (u32::MAX, Q16_16::from_bits(65_535)),
                (u32::MAX, p(1, 2)),
                (25, Q16_16::from_bits(65_535)),
            ] {
                assert!(Binomial::new(n, prob).unwrap().sample(&mut s) <= n);
            }
        }
    }
}
