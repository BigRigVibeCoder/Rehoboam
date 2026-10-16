// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `std-rng` must catch `SmallRng`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn make(seed: u64) -> SmallRng {
    SeedableRng::seed_from_u64(seed)
}
