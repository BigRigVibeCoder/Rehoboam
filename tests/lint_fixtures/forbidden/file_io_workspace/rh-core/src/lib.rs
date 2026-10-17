//! Fixture: plane-A engine code reading a file (the file-io rule must report it).

pub fn seed() -> String {
    std::fs::read_to_string("seed.txt").unwrap_or_default()
}
