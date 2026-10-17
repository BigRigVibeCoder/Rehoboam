// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `env` must catch the fully
// qualified std::env::set_current_dir, which changes what every relative path names (DEF-5). Never
// compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn chdir(p: &str) -> bool { std::env::set_current_dir(p).is_ok() }
