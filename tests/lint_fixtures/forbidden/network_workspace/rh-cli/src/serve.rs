//! The serving module: the one place in the host allowed a socket (NETWORK_FILES).
pub fn serve(path: &str) -> bool {
    std::os::unix::net::UnixListener::bind(path).is_ok()
}
