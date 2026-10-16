// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `rand-random` must catch `random_ratio`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn coin() -> bool {
    rand::random_ratio(1, 3)
}
