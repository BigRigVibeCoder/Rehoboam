// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `float` must catch `2e-3f64` (SPR-010 T-04).
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

/// A suffixed float literal: an annual death rate.
pub const DEATH_RATE: Rate = Rate::new(2e-3f64);
