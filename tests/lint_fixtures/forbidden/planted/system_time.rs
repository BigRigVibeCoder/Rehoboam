// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `wall-clock` must catch it.
// Never compiled; scanned as plane-A code by bin/check_all.sh.

pub fn stamp() {
    let _t = std::time::SystemTime::now();
}
