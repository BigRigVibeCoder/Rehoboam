// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `file-clock` must catch
// `.modified(`, a file's timestamp read as a clock (BCK-002 A-44). Never compiled; scanned as
// plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn age(meta: &Metadata) -> u64 {
    elapsed(meta.modified())
}
