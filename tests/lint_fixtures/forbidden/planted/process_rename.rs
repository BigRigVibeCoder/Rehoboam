// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `process` must catch a
// renamed import of std::process (DEF-1). Never compiled; scanned as plane-A code by
// bin/check_all.sh. It holds exactly one forbidden API.

use std::process as p;

pub fn date() { let _ = p::Command::new("date").output(); }
