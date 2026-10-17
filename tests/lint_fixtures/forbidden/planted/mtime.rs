// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `file-clock` must catch a
// glob-imported MetadataExt accessor, `.mtime(` (DEF-2). Never compiled; scanned as plane-A code by
// bin/check_all.sh. It holds exactly one forbidden API.

pub fn age(m: &Meta) -> i64 { m.mtime() }
