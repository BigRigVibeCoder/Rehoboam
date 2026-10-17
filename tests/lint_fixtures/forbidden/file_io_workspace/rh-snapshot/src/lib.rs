//! Fixture: rh-snapshot is on FILE_IO_CRATES (the journal and keyframes) and may write files.

pub fn save(bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write("keyframe.bin", bytes)
}
