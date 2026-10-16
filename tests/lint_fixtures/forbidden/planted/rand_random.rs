// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `rand-random` must catch it.
// Never compiled; scanned as plane-A code by bin/check_all.sh.

pub fn roll() -> u8 {
    rand::random()
}
