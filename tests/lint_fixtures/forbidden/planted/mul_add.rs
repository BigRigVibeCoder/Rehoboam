// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `mul-add` must catch it.
// Never compiled; scanned as plane-A code by bin/check_all.sh.

pub fn fma(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c)
}
