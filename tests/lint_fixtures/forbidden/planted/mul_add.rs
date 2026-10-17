// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `mul-add` must catch `mul_add`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn fma<T: Fused>(a: T, b: T, c: T) -> T {
    a.mul_add(b, c)
}
