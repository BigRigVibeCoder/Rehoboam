//! A test may open a socket to drive the host.
fn connect(path: &str) -> bool {
    std::os::unix::net::UnixStream::connect(path).is_ok()
}
