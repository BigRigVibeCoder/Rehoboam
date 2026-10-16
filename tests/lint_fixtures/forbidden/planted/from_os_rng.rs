// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `std-rng` must catch `from_os_rng`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn make() -> Generator {
    Generator::from_os_rng()
}
