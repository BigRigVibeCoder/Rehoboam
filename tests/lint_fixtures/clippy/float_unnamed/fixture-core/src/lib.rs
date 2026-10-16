//! Plane-A fixture: float arithmetic on values that are never named as floats here. Clippy must reject it.

/// A famine threshold computed in floats that the text gate cannot see: `ext::draw()` returns `f64`.
#[must_use]
pub fn famine() -> bool {
    let store = ext::draw();
    let need = ext::draw();
    store * store < need
}
