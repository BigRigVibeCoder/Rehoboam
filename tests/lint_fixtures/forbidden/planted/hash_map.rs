// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `hash-collections` must catch `HashMap`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn count(m: &HashMap<u32, u32>) -> usize {
    m.len()
}
