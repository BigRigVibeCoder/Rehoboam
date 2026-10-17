//! Fixture: any other rh-stream file doing file I/O must be reported (SPR-013 planted control).
mod seal;

pub fn peek() -> Vec<u8> {
    std::fs::read("segment.parquet").unwrap_or_default()
}
