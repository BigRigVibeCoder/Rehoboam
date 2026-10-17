//! The tick loop of the fixture host: a socket here would put a reader on the tick path.
pub fn tick(path: &str) -> f64 {
    let _l = std::os::unix::net::UnixListener::bind(path);
    0.5
}
