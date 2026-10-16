// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `std-rng` must catch it.
// Never compiled; scanned as plane-A code by bin/check_all.sh.

use rand::rngs::StdRng;

pub fn make(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}
