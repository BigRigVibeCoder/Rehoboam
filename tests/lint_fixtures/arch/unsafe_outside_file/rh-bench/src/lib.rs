//! Fixture: unsafe outside the listed file.
#![allow(unsafe_code, reason = "fixture")]
/// Reads a byte.
pub fn read(p: *const u8) -> u8 {
    unsafe { *p }
}
