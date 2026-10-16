// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `hash-collections` must catch it.
// Never compiled; scanned as plane-A code by bin/check_all.sh.

use std::collections::HashMap;

pub fn count(m: &HashMap<u32, u32>) -> usize {
    m.len()
}
