// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `hash-collections` must catch `HashSet`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn count(s: &HashSet<u32>) -> usize {
    s.len()
}
