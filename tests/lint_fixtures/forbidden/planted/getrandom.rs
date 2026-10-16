// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `std-rng` must catch `getrandom`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn fill(buf: &mut [u8]) {
    let _ = getrandom::fill(buf);
}
