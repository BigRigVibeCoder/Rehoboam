// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `file-io` must catch
// `std::fs` in plane-A code outside the crates allowed file I/O (BCK-002 A-44). Never compiled;
// scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn seed() -> String {
    std::fs::read_to_string("seed.txt").unwrap_or_default()
}
