// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `rand-random` must catch `random_range`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn roll() -> u32 {
    rand::random_range(0..10)
}
