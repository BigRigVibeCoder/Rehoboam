// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `float` must catch `f64` (SPR-010 T-04).
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

/// An escape with a reason that is not a declared float field: the float stays a violation.
pub fn hunger(stock: Stock) -> bool {
    stock.level() < LIMIT as f64 // LINT-ALLOW(forbidden-api): it is only a heuristic
}
