//! Fixture: rh-core holding a float threshold. The float zone's exemption must not reach it.

/// A famine threshold in floats (DET-3 forbids it outside rh-sample).
pub fn famine(store: f64) -> bool {
    store < 0.4
}
