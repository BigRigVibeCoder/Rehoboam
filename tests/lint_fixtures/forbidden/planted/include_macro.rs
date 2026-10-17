// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `source-include` must catch `include!(`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

// DEF-4: code pasted in from a file outside the crate. Since BCK-002 A-27 a literal in-repository
// include! is followed instead; this one names no file, so it stays a violation.
include!("../../../elsewhere/clock.rs");
