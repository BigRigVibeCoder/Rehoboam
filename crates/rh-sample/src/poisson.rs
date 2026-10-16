//! The Poisson distribution `Poisson(λ)`: PTRS for `λ ≥ 10`, inversion below.
//!
//! **PTRS** (transformed rejection with squeeze), written from the paper: W. Hörmann (1993), "The
//! transformed rejection method for generating Poisson random variables", *Insurance: Mathematics
//! and Economics* 12(1), 39–45, algorithm PTRS. `rand_distr`'s Poisson is Knuth's method plus a
//! rejection method, not PTRS (`docs/14`), so nothing here is ported. With `μ = λ`:
//!
//! ```text
//! setup:  b = 0.931 + 2.53·√μ    a = −0.059 + 0.02483·b
//!         α = 1.1239 + 1.1328/(b − 3.4)    v_r = 0.9277 − 3.6224/(b − 2)
//! 1. u = U(0,1) − 0.5,  v = U(0,1),  u_s = 0.5 − |u|,  k = ⌊(2a/u_s + b)·u + μ + 0.43⌋
//! 2. if u_s ≥ 0.07 and v ≤ v_r: return k                         (the squeeze)
//! 3. if k < 0, or u_s < 0.013 and v > u_s: go to 1               (outside the hat)
//! 4. if ln(v·α/(a/u_s² + b)) ≤ −μ + k·ln μ − ln k!: return k, else go to 1
//! ```
//!
//! `ln k!` is `libm::lgamma(k + 1)`. The paper recommends PTRS for `μ ≥ 10`.
//!
//! **Inversion** for `λ < 10`: one uniform `u`, then the sequential search of the CDF from 0 by
//! the recurrence `p(x) = p(x − 1)·λ/x` from `p(0) = e^−λ`, subtracting each `p(x)` from `u`, as
//! BINV does for the binomial. An attempt restarts if the count passes 110 (a > 30-standard-
//! deviation event for `λ < 10`, or the search stalling on rounding just below `u ≈ 1`).
//!
//! Parameter and result: `λ` is a `Q16_16` (so at most 32,768), the result a `u32` count.
//!
//! # Draws consumed
//!
//! A sample reads the stream sequentially from its current position:
//! - `λ = 0`: **no draw**.
//! - inversion (`λ < 10`): **one draw per attempt**, in practice one draw.
//! - PTRS (`λ ≥ 10`): **two draws per attempt** (`u`, then `v`), a variable number of attempts
//!   (Hörmann reports about 1.15 expected attempts at `μ = 10`, falling towards 1 as `μ` grows),
//!   deterministic given the stream.
//!
//! REF: SPR-003 T-02; ADR-0006 item 3; `05.1` §6 item 3; `docs/14` (`rand_distr`'s Poisson is not
//! PTRS).

use rh_core::fixed::Q16_16;
use rh_ids::RngStream;

use crate::convert::{f64_from_q16_16, floor_to_i64};
use crate::error::SampleError;
use crate::uniform::next_closed_open;

/// From this `λ` on, PTRS; below it, inversion (Hörmann 1993: PTRS for `μ ≥ 10`).
const PTRS_THRESHOLD: f64 = 10.0;
/// An inversion attempt restarts past this count.
const INVERSION_MAX_X: u32 = 110;

/// The Poisson distribution `Poisson(λ)`: the number of events of a process with rate `λ`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Poisson {
    method: Method,
}

/// How a [`Poisson`] samples, with its constants precomputed.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Method {
    /// `λ = 0`: always 0.
    Zero,
    /// Inversion from `p(0) = e^−λ`.
    Inversion { lambda: f64, p0: f64 },
    /// PTRS.
    Ptrs(Ptrs),
}

/// PTRS's constants (the paper's names; `alpha` is the paper's α, `v_r` its `v_r`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Ptrs {
    lambda: f64,
    ln_lambda: f64,
    a: f64,
    b: f64,
    alpha: f64,
    v_r: f64,
}

impl Poisson {
    /// The Poisson distribution with mean `lambda`.
    ///
    /// # Errors
    ///
    /// If `lambda` is negative.
    pub fn new(lambda: Q16_16) -> Result<Self, SampleError> {
        if lambda < Q16_16::ZERO {
            return Err(SampleError::new("Poisson", "lambda must not be negative"));
        }
        if lambda == Q16_16::ZERO {
            return Ok(Self {
                method: Method::Zero,
            });
        }
        let lambda = f64_from_q16_16(lambda);
        let method = if lambda < PTRS_THRESHOLD {
            Method::Inversion {
                lambda,
                p0: libm::exp(-lambda),
            }
        } else {
            let b = 0.931 + 2.53 * lambda.sqrt();
            Method::Ptrs(Ptrs {
                lambda,
                ln_lambda: libm::log(lambda),
                a: -0.059 + 0.024_83 * b,
                b,
                alpha: 1.1239 + 1.1328 / (b - 3.4),
                v_r: 0.9277 - 3.6224 / (b - 2.0),
            })
        };
        Ok(Self { method })
    }

    /// Draws one count, reading `stream` from its current position (module docs: how many draws).
    pub fn sample(&self, stream: &mut RngStream) -> u32 {
        match self.method {
            Method::Zero => 0,
            Method::Inversion { lambda, p0 } => sample_inversion(lambda, p0, stream),
            Method::Ptrs(ptrs) => sample_ptrs(&ptrs, stream),
        }
    }
}

/// Sequential search of the inverse CDF from 0, one draw per attempt.
fn sample_inversion(lambda: f64, p0: f64, stream: &mut RngStream) -> u32 {
    'attempt: loop {
        let mut p = p0;
        let mut u = next_closed_open(stream);
        let mut x: u32 = 0;
        while u > p {
            u -= p;
            x += 1;
            if x > INVERSION_MAX_X {
                continue 'attempt;
            }
            p *= lambda / f64::from(x);
        }
        return x;
    }
}

/// PTRS, steps 1 to 4 (module docs), two draws per attempt.
fn sample_ptrs(c: &Ptrs, stream: &mut RngStream) -> u32 {
    loop {
        // Step 1.
        let u = next_closed_open(stream) - 0.5;
        let v = next_closed_open(stream);
        let us = 0.5 - u.abs();
        // u_s = 0 (u = −0.5) gives −∞, which floor_to_i64 refuses: the k < 0 branch of step 3.
        let k = floor_to_i64((2.0 * c.a / us + c.b) * u + c.lambda + 0.43);
        // Step 2: the squeeze.
        if us >= 0.07
            && v <= c.v_r
            && let Some(k) = k.and_then(|k| u32::try_from(k).ok())
        {
            return k;
        }
        // Step 3: outside the hat.
        let Some(k) = k.and_then(|k| u32::try_from(k).ok()) else {
            continue;
        };
        if us < 0.013 && v > us {
            continue;
        }
        // Step 4: the exact test against ln p(k).
        let k_f = f64::from(k);
        let lhs = libm::log(v * c.alpha / (c.a / (us * us) + c.b));
        let rhs = -c.lambda + k_f * c.ln_lambda - libm::lgamma(k_f + 1.0);
        if lhs <= rhs {
            return k;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Method, Poisson};
    use rh_core::fixed::Q16_16;
    use rh_ids::{BranchSalt, PurposeId, RngKey, RngStream, SystemId};

    fn stream(entity: u64) -> RngStream {
        RngStream::new(&RngKey {
            world_seed: 3,
            branch_salt: BranchSalt::from_raw(0),
            system: SystemId::from_raw(1),
            purpose: PurposeId::of("poisson_unit_test"),
            entity,
            epoch: 0,
            t: 0,
        })
    }

    #[test]
    fn negative_lambda_is_refused() {
        let err = Poisson::new(Q16_16::from_bits(-1)).unwrap_err();
        assert_eq!(err.to_string(), "Poisson: lambda must not be negative");
    }

    #[test]
    fn zero_is_constant_and_draws_nothing() {
        let mut s = stream(0);
        assert_eq!(Poisson::new(Q16_16::ZERO).unwrap().sample(&mut s), 0);
        assert_eq!(s.position(), 0);
    }

    #[test]
    fn the_method_switches_at_ten() {
        let below = Q16_16::from_bits((10 << 16) - 1);
        assert!(matches!(
            Poisson::new(below).unwrap().method,
            Method::Inversion { .. }
        ));
        assert!(matches!(
            Poisson::new(Q16_16::from_int(10)).unwrap().method,
            Method::Ptrs(_)
        ));
    }

    #[test]
    fn inversion_takes_one_draw_and_ptrs_an_even_number() {
        for entity in 0..200 {
            let mut s = stream(entity);
            let _ = Poisson::new(Q16_16::from_int(3)).unwrap().sample(&mut s);
            assert_eq!(s.position(), 1);
            let mut s = stream(entity);
            let _ = Poisson::new(Q16_16::from_int(40)).unwrap().sample(&mut s);
            assert!(s.position() >= 2 && s.position().is_multiple_of(2));
        }
    }

    #[test]
    fn extremes_sample_without_panicking() {
        for entity in 0..300 {
            let mut s = stream(entity);
            let _ = Poisson::new(Q16_16::EPSILON).unwrap().sample(&mut s);
            let x = Poisson::new(Q16_16::MAX).unwrap().sample(&mut s);
            assert!((30_000..36_000).contains(&x), "{x}");
        }
    }
}
