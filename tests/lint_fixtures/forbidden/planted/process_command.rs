// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `process` must catch
// `process::Command`, another program run from plane A (BCK-002 A-44). Never compiled; scanned as
// plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn uptime() -> Vec<u8> {
    std::process::Command::new("uptime").output().map(|o| o.stdout).unwrap_or_default()
}
