//! Fixture: the float zone calling `std` transcendentals. The float allow must not excuse them.
#![allow(
    clippy::float_arithmetic,
    clippy::float_cmp,
    reason = "the float zone's crate-level allow, as in crates/rh-sample"
)]

/// A log-density written with `std`'s `ln` and `exp`, which vary by platform: clippy must reject both.
#[must_use]
pub fn log_density(x: f64) -> f64 {
    x.ln() - (-x).exp()
}
