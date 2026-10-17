// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `file-io` must catch std::fs
// written with raw identifiers, `r#std::r#fs` (MIN-2). Never compiled; scanned as plane-A code by
// bin/check_all.sh. It holds exactly one forbidden API.

pub fn read(p: &str) -> String { r#std::r#fs::read_to_string(p).unwrap_or_default() }
