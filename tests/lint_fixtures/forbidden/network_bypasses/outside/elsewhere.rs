//! A file outside the crate: never scanned on its own.
pub fn listen(path: &str) -> bool {
    std::os::unix::net::UnixListener::bind(path).is_ok()
}
