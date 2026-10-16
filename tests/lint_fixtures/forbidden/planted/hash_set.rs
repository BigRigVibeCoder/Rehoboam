// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `hash-collections` must catch it.
// Never compiled; scanned as plane-A code by bin/check_all.sh.

use std::collections::HashSet;

pub fn count(s: &HashSet<u32>) -> usize {
    s.len()
}
