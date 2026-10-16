// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `rand-random` must catch `random_iter`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn rolls() -> usize {
    rand::random_iter::<u8>().take(3).count()
}
