// Planted positive control for bin/lint_forbidden.py (BCK-002 A-50; SPR-014, the review's K2): rule
// `build-env` must catch use std::option_env as oe; oe!("HOME"), a renamed build-environment macro.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

use std::option_env as oe;
pub fn home() -> Option<&'static str> { oe!("HOME") }
