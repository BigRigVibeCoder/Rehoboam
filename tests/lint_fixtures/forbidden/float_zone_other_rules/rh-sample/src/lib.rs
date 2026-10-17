//! Fixture: the float zone using a fused multiply-add, which the exemption does not cover.

/// `a * b + c` fused: rounds differently across targets (V-DET-19 mul-add rule).
#[must_use]
pub(crate) fn affine(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c)
}
