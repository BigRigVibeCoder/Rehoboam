//! The tick loop: a socket spliced in from another file (R19 probes B, C, D).
mod serve;
#[path = "../tests/socket.rs"]
mod sock;
include!("../tests/socket.rs");
#[path = "../../outside/elsewhere.rs"]
mod elsewhere;

pub fn tick() -> bool {
    serve::serve("x") && sock::connect("y") && connect("z") && elsewhere::listen("w")
}
