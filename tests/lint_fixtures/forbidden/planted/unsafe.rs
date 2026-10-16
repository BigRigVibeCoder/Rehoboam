// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `unsafe` must catch it.
// Never compiled; scanned as plane-A code by bin/check_all.sh.

pub fn read(p: *const u8) -> u8 {
    unsafe { *p }
}
