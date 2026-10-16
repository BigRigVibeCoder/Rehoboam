// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `float` must catch `f32` (SPR-010 T-04).
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

/// A float threshold: famine when a store falls below a fraction of its need.
pub fn famine(store_ratio: f32) -> bool {
    store_ratio < THRESHOLD
}
