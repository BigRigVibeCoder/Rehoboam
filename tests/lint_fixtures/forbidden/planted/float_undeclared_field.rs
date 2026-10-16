// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `float` must catch `f32` (SPR-010 T-04).
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

/// An f32_field escape naming a column no system spec declares: the float stays a violation.
pub struct Region {
    pub fertility: f32, // LINT-ALLOW(forbidden-api): f32_field region.fertility
}
