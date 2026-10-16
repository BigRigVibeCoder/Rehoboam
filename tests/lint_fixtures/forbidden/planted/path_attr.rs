// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `source-include` must catch `#[path`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

// DEF-4: a module whose source lives outside the crate, where the gate never looks.
#[path = "../../../elsewhere/clock.rs"]
mod clock;
