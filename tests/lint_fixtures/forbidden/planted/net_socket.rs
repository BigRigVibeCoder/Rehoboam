// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `network` must catch a
// socket opened by plane-A code (specs/stream/wire_v1.md §7; SPR-026). Never compiled; scanned as
// plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn serve(path: &str) -> bool {
    std::os::unix::net::UnixListener::bind(path).is_ok()
}
