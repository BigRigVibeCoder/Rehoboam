// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `source-include` must catch `include!(`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

// DEF-4: code pasted in from a file outside the crate.
include!("../../../elsewhere/clock.rs");
