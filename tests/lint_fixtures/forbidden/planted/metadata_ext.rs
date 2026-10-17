// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `file-clock` must catch a
// platform MetadataExt, a file's timestamps (DEF-2). Never compiled; scanned as plane-A code by
// bin/check_all.sh. It holds exactly one forbidden API.

use std::os::unix::fs::MetadataExt as _;
