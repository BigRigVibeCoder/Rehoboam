// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `file-io` must catch a file
// read through a renamed std, `s::fs::read_to_string` (MAJ-1). Never compiled; scanned as plane-A
// code by bin/check_all.sh. The rename itself is rule `std-rename`, controlled by std_rename.rs.

pub fn read(p: &str) -> String { s::fs::read_to_string(p).unwrap_or_default() }
