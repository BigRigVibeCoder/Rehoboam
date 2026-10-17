// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `network` must catch a
// socket reached through its parent module (REV-SPR-026 MIN-1; SPR-026). Never compiled; scanned
// as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

use std::os::unix::*;

pub fn serve(path: &str) -> bool {
    net::TcpListener::bind(path).is_ok()
}
