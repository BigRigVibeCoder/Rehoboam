//! The keyframe loader's first line against a malformed Arrow body (DEF-3 layer 1, Architect
//! ruling on VER-SPR-007-TEST round 2): the IPC file's metadata is checked with `arrow-ipc`'s
//! verified flatbuffer accessors before `FileReader` sees the bytes.
//!
//! The envelope digest (`crate::keyframe`) stops accidental corruption, not a body written wrongly
//! under a correct digest (a buggy writer, another Arrow version). Arrow 60 trusts a record
//! batch's declared sizes: a forged uncompressed length makes zstd allocate it up front, and an
//! allocation failure aborts the process, which no `catch_unwind` can catch. So this module
//! refuses, with [`KeyframeError::Ipc`], every file that is not exactly what [`write_keyframe`]
//! writes:
//!
//! - the file magic, and a footer inside the file that verifies as a flatbuffer;
//! - metadata version V5, little-endian, no dictionaries, no custom metadata;
//! - the schema exactly `table: Utf8` and `row: Binary`, both non-null, no children, no
//!   dictionary encoding;
//! - every record-batch block inside the file, 8-aligned, ahead of the footer, its message a
//!   verified flatbuffer record batch of the block's body length, with at most [`BATCH_ROWS`]
//!   rows, two field nodes of that length with no nulls, six buffers, zstd buffer compression and
//!   no variadic buffers;
//! - every buffer inside its block's body, and every compressed buffer's declared uncompressed
//!   length within its bound: a validity buffer at most `rows / 8 + 64` bytes, an offsets buffer
//!   at most `4 (rows + 1)`, a values buffer at most [`MAX_VALUES_BYTES`]; and never more than
//!   [`ZSTD_MAX_RATIO`] times the compressed bytes, which no zstd frame can exceed.
//!
//! The hard ceiling on what one batch can make Arrow allocate is therefore
//! `2 (MAX_VALUES_BYTES + 4 (BATCH_ROWS + 1) + BATCH_ROWS / 8 + 64)` bytes, about 129 MiB.
//! [`write_keyframe`] splits batches so that every keyframe it writes is within these bounds.
//!
//! [`write_keyframe`]: crate::keyframe::write_keyframe
//! [`BATCH_ROWS`]: crate::keyframe::BATCH_ROWS
//!
//! REF: ADR-0007 (Arrow IPC with zstd for keyframes); VER-SPR-007-TEST round 2 DEF-3;
//! `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md` T-02.

use arrow_ipc::{
    BodyCompressionMethod, CompressionType, Endianness, MessageHeader, MetadataVersion, Type,
    root_as_footer, root_as_message,
};

use crate::keyframe::{BATCH_ROWS, KeyframeError};

/// The most bytes one values buffer (a batch's table names, or its rows) may declare.
pub const MAX_VALUES_BYTES: usize = 64 << 20;

/// The most a zstd frame can expand: a 4-byte RLE block decodes to at most 128 KiB.
pub const ZSTD_MAX_RATIO: usize = 32_768;

/// The Arrow IPC file magic, at both ends of the file.
const MAGIC: &[u8; 6] = b"ARROW1";
/// The continuation marker before every message's metadata.
const CONTINUATION: [u8; 4] = [0xFF; 4];
/// The 8-byte prefix of a compressed buffer: its uncompressed length, or -1 for "stored as is".
const PREFIX: usize = 8;
/// Buffers per batch: validity, offsets and values of `table`, then of `row`.
const BUFFERS: usize = 6;

fn refuse(message: impl Into<String>) -> KeyframeError {
    KeyframeError::Ipc(message.into())
}

/// `[start, start + len)` of `bytes`, if it lies inside.
fn span(bytes: &[u8], start: i64, len: i64) -> Option<&[u8]> {
    let start = usize::try_from(start).ok()?;
    let end = start.checked_add(usize::try_from(len).ok()?)?;
    bytes.get(start..end)
}

fn le_i32(bytes: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// Checks the Arrow IPC file `arrow` against the keyframe's fixed layout (module docs).
///
/// # Errors
/// [`KeyframeError::Ipc`] naming the first check that fails.
pub fn check(arrow: &[u8]) -> Result<(), KeyframeError> {
    let n = arrow.len();
    // n >= 16 makes `arrow[..6]`, `arrow[n - 6..]` and `n - 10` safe; `footer_start` below is
    // in [8, n - 10], so `arrow[footer_start..n - 10]` and `arrow[..footer_start]` are too.
    if n < 2 * MAGIC.len() + 4 || &arrow[..6] != MAGIC || &arrow[n - 6..] != MAGIC {
        return Err(refuse("not an Arrow IPC file"));
    }
    let footer_len = le_i32(arrow, n - 10).ok_or_else(|| refuse("no footer length"))?;
    let footer_start = usize::try_from(footer_len)
        .ok()
        .and_then(|len| (n - 10).checked_sub(len))
        .filter(|&start| start >= 8)
        .ok_or_else(|| refuse("the footer length is outside the file"))?;
    let footer = root_as_footer(&arrow[footer_start..n - 10])
        .map_err(|e| refuse(format!("the footer: {e}")))?;
    if footer.version() != MetadataVersion::V5 {
        return Err(refuse("metadata version is not V5"));
    }
    if footer.dictionaries().is_some_and(|d| !d.is_empty()) {
        return Err(refuse("dictionaries"));
    }
    if footer.custom_metadata().is_some() {
        return Err(refuse("footer custom metadata"));
    }
    check_schema(&footer.schema().ok_or_else(|| refuse("no schema"))?)?;
    let blocks = footer
        .recordBatches()
        .ok_or_else(|| refuse("no record batches"))?;
    for block in blocks {
        check_block(
            &arrow[..footer_start],
            block.offset(),
            block.metaDataLength(),
            block.bodyLength(),
        )?;
    }
    Ok(())
}

fn check_schema(schema: &arrow_ipc::Schema<'_>) -> Result<(), KeyframeError> {
    if schema.endianness() != Endianness::Little {
        return Err(refuse("big-endian schema"));
    }
    if schema.custom_metadata().is_some() || schema.features().is_some_and(|f| !f.is_empty()) {
        return Err(refuse("schema metadata or features"));
    }
    let fields = schema.fields().ok_or_else(|| refuse("no fields"))?;
    let expected = [("table", Type::Utf8), ("row", Type::Binary)];
    if fields.len() != expected.len() {
        return Err(refuse("the schema is not (table: Utf8, row: Binary)"));
    }
    for (field, (name, kind)) in fields.iter().zip(expected) {
        let plain = field.name() == Some(name)
            && !field.nullable()
            && field.type_type() == kind
            && field.dictionary().is_none()
            && field.children().is_none_or(|c| c.is_empty())
            && field.custom_metadata().is_none();
        if !plain {
            return Err(refuse(format!(
                "field {name} is not a plain non-null {name}"
            )));
        }
    }
    Ok(())
}

fn check_block(
    file: &[u8],
    offset: i64,
    meta_len: i32,
    body_len: i64,
) -> Result<(), KeyframeError> {
    let at = format!("the block at {offset}");
    // Signs first: a negative multiple of 8 passes the alignment test (DEF-3b), and a negative
    // body length would make the span below shorter than the metadata.
    if offset < 8 || meta_len <= 8 || body_len < 0 {
        return Err(refuse(format!(
            "{at}: offset, metadata length or body length out of range"
        )));
    }
    if offset % 8 != 0 || meta_len % 8 != 0 || body_len % 8 != 0 {
        return Err(refuse(format!("{at} is misaligned")));
    }
    let meta_len =
        usize::try_from(meta_len).map_err(|_| refuse(format!("{at}: metadata length")))?;
    let block = span(
        file,
        offset,
        i64::try_from(meta_len)
            .unwrap_or(i64::MAX)
            .saturating_add(body_len),
    )
    .ok_or_else(|| refuse(format!("{at} is outside the file")))?;
    let (meta, body) = block
        .split_at_checked(meta_len)
        .ok_or_else(|| refuse(format!("{at}: the metadata is outside the block")))?;
    // From here meta.len() == meta_len > 8, so `meta[..4]`, `meta.len() - 8` and
    // `meta[8..8 + fb_len]` (fb_len <= meta.len() - 8) are in bounds.
    if meta[..4] != CONTINUATION {
        return Err(refuse(format!("{at} has no continuation marker")));
    }
    let fb_len = le_i32(meta, 4)
        .and_then(|len| usize::try_from(len).ok())
        .filter(|&len| len <= meta.len() - 8)
        .ok_or_else(|| refuse(format!("{at}: the message length is outside the block")))?;
    let message = root_as_message(&meta[8..8 + fb_len])
        .map_err(|e| refuse(format!("{at}: the message: {e}")))?;
    if message.version() != MetadataVersion::V5
        || message.header_type() != MessageHeader::RecordBatch
        || message.bodyLength() != body_len
        || message.custom_metadata().is_some()
    {
        return Err(refuse(format!("{at} is not a V5 record batch of its body")));
    }
    let batch = message
        .header_as_record_batch()
        .ok_or_else(|| refuse(format!("{at} has no record batch")))?;
    let rows = usize::try_from(batch.length())
        .ok()
        .filter(|&rows| rows <= BATCH_ROWS)
        .ok_or_else(|| refuse(format!("{at} declares {} rows", batch.length())))?;
    let compressed = batch.compression().is_some_and(|c| {
        c.codec() == CompressionType::ZSTD && c.method() == BodyCompressionMethod::BUFFER
    });
    if !compressed || batch.variadicBufferCounts().is_some_and(|v| !v.is_empty()) {
        return Err(refuse(format!("{at} is not zstd buffer compression")));
    }
    let nodes = batch
        .nodes()
        .ok_or_else(|| refuse(format!("{at} has no nodes")))?;
    if nodes.len() != 2
        || nodes
            .iter()
            .any(|node| node.length() != batch.length() || node.null_count() != 0)
    {
        return Err(refuse(format!(
            "{at}: the field nodes are not two of {rows} rows"
        )));
    }
    let buffers = batch
        .buffers()
        .ok_or_else(|| refuse(format!("{at} has no buffers")))?;
    if buffers.len() != BUFFERS {
        return Err(refuse(format!("{at} has {} buffers", buffers.len())));
    }
    for (k, buffer) in buffers.iter().enumerate() {
        let bytes = span(body, buffer.offset(), buffer.length())
            .ok_or_else(|| refuse(format!("{at}: buffer {k} is outside the body")))?;
        let bound = match k % 3 {
            0 => rows / 8 + 64,
            1 => 4 * (rows + 1),
            _ => MAX_VALUES_BYTES,
        };
        check_buffer(bytes, bound).map_err(|m| refuse(format!("{at}: buffer {k} {m}")))?;
    }
    Ok(())
}

/// One compressed buffer: empty, or the 8-byte length prefix and data whose uncompressed length
/// is at most `bound` and at most what zstd can produce from it.
fn check_buffer(bytes: &[u8], bound: usize) -> Result<(), String> {
    if bytes.is_empty() {
        return Ok(());
    }
    let (prefix, data) = bytes
        .split_first_chunk::<PREFIX>()
        .ok_or("is shorter than its length prefix")?;
    let declared = i64::from_le_bytes(*prefix);
    let size = match declared {
        -1 => data.len(),
        d => usize::try_from(d).map_err(|_| format!("declares {d} bytes"))?,
    };
    let ratio = data.len().saturating_mul(ZSTD_MAX_RATIO);
    if size > bound || (declared != -1 && size > ratio) {
        return Err(format!(
            "declares {size} uncompressed bytes (bound {bound}, {} compressed)",
            data.len()
        ));
    }
    Ok(())
}
