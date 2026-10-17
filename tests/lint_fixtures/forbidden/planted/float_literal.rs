// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `float` must catch `0.5` (SPR-010 T-04).
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

/// A float literal compared with a fixed-point value through an inferred float.
pub fn coin_flip(draw: Ratio) -> bool {
    draw.value() < 0.5
}
