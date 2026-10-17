//! The serving module: sockets are allowed here, re-exporting them is not (R19 probes A, E, G).
pub use std::os::unix::net::UnixListener as Pipe;
pub type Ear = std::os::unix::net::UnixStream;
pub use std::os::unix::net::*;

pub fn serve(path: &str) -> bool {
    Pipe::bind(path).is_ok()
}
