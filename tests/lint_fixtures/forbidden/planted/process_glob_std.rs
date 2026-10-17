// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `process` must catch
// `process::Command` reached through a glob import of std, where no `std::process` path is written
// (VER-SPR-011-TEST GAP-3; BCK-002 A-49). Never compiled; scanned as plane-A code by
// bin/check_all.sh. It holds exactly one forbidden API.
use std::*;

pub fn uptime() {
    let _ = process::Command::new("uptime");
}
