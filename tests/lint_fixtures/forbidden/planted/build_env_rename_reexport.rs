// Planted positive control for bin/lint_forbidden.py (BCK-002 A-50; SPR-014, the review's K2): rule
// `build-env` must catch a re-exported env renamed (use some_crate::env as e), a renamed build-environment macro.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

use some_crate::env as e;
pub fn home() -> &'static str { e!("HOME") }
