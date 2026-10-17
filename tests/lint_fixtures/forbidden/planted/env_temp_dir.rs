// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `env` must catch the fully
// qualified std::env::temp_dir, a host directory or path list outside the history tuple (DEF-5). Never
// compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn host() -> Option<std::path::PathBuf> { Some(std::env::temp_dir()) }
