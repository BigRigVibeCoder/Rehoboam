//! Fixture: rh-stream's seal module is the one file FILE_IO_FILES allows file I/O (SPR-013).

pub fn seal(bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write("segment.parquet.tmp", bytes)
}
