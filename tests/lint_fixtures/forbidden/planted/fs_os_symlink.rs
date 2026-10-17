// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `file-io` must catch a
// platform fs module, std::os::unix::fs::symlink (MAJ-1). Never compiled; scanned as plane-A code
// by bin/check_all.sh. It holds exactly one forbidden API.

pub fn link(a: &str, b: &str) -> bool { std::os::unix::fs::symlink(a, b).is_ok() }
