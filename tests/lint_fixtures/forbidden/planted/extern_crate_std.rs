// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `std-rename` must catch
// `extern crate std as s;` (MIN-2). Never compiled; scanned as plane-A code by bin/check_all.sh.
// It holds exactly one forbidden API.

extern crate std as s;
