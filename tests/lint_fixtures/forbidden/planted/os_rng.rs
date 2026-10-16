// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `std-rng` must catch `OsRng`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn roll() -> u32 {
    OsRng.next_u32()
}
