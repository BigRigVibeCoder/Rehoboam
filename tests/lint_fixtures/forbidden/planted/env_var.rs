// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `env` must catch
// std::env::var_os, a clock reached through the environment (BCK-002 A-44; SPR-011 A-4). Never
// compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn now() -> bool { std::env::var_os("EPOCHREALTIME").is_some() }
