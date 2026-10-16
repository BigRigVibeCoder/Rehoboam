// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `wall-clock` must catch `SystemTime`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn stamp() {
    let _t = SystemTime::now();
}
