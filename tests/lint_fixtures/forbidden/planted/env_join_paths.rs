// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `env` must catch the fully
// qualified std::env::join_paths, whose separator is the host's (DEF-5). Never compiled; scanned
// as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn joined(a: &[&str]) -> bool { ::std::env::join_paths(a).is_ok() }
