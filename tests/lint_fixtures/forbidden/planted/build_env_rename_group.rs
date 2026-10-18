// Planted positive control for bin/lint_forbidden.py (BCK-002 A-50; SPR-014, the review's K2): rule
// `build-env` must catch use core::{option_env as x}; x!("HOME"), a renamed build-environment macro.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

use core::{option_env as x};
pub fn home() -> Option<&'static str> { x!("HOME") }
