//! A test helper: exempt when scanned as tests/, a finding when spliced into src/.
pub fn connect(path: &str) -> bool {
    std::os::unix::net::UnixStream::connect(path).is_ok()
}
