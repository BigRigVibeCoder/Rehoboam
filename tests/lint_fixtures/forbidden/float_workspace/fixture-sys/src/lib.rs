//! Plane-A system fixture: a famine threshold written in floats. The gate must report it.

/// Famine when the grain store falls below 40% of need: a float threshold (DET-3 forbids it).
pub fn famine(store: f32, need: f32) -> bool {
    store < need * 0.4
}
