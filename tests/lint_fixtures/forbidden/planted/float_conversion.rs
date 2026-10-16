// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `float` must catch `to_f64` (SPR-010 T-04).
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

/// A conversion that yields a float without naming its type.
pub fn share(part: Count) -> Ratio {
    Ratio::from_raw(part.to_f64())
}
