//! Fixture: rh-registry left FILE_IO_CRATES with BCK-002 A-51; a spec read in src/ is reported.

pub fn spec(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}
