// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `thread-rng` must catch `ThreadRng`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn roll(r: &mut ThreadRng) -> u32 {
    r.next_u32()
}
