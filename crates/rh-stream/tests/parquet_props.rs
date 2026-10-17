//! Byte identity is a contract (the Architect's review of `parquet`, SPR-013): the writer
//! properties that reach a file are pinned and asserted here, and a golden segment's bytes are
//! pinned by their digest, so a `parquet` upgrade that changes the layout fails loudly.
//!
//! - Every file of a sealed segment has the fixed `created_by`, no key-value metadata, zstd at the
//!   fixed level on every column, no dictionary page or dictionary encoding, no statistics, no
//!   column index, no bloom filter, an offset index, one row group, Parquet 1.0 data pages.
//! - No column of any channel-A table has a float physical type (R2).
//! - The golden segment: the synthetic world of seed 42 with 7 persons over one year encodes to
//!   files whose XXH3-128 (over each file's name, length and bytes, in sealing order) is pinned.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers fail loudly"
)]

mod support;

use bytes::Bytes;
use parquet::basic::{Compression, Encoding, Type as Physical, ZstdLevel};
use parquet::file::reader::{FileReader, SerializedFileReader};
use rh_snapshot::hash::xxh3_128;
use rh_stream::canonical::hex16;
use rh_stream::parquet_io::{self, CREATED_BY, ZSTD};
use rh_stream::record::{Body, Header, Record};
use rh_stream::segment::{self, CHANNEL_A};
use support::{MPY, dictionaries, history, provenance};

/// The golden segment's records: the synthetic history of seed 42, 7 persons, one year, in R4
/// order with seq from 0 (and the exits linked to their events, as the writer does).
fn golden_records() -> Vec<Record> {
    let mut h = history(42, 7, 1);
    rh_stream::convert::link_exits(&mut h);
    let mut keyed: Vec<((i64, u8, u64, u64), Body)> = h
        .into_iter()
        .map(|(t, b)| {
            let (k1, k2) = b.order_k();
            ((t, b.kind().class(), k1, k2), b)
        })
        .collect();
    keyed.sort_by_key(|(k, _)| *k);
    keyed
        .into_iter()
        .enumerate()
        .map(|(i, (k, body))| Record {
            header: Header {
                timeline_id: 0,
                seq: u64::try_from(i).unwrap(),
                tick: k.0,
            },
            body,
        })
        .collect()
}

fn golden_files() -> Vec<(String, Vec<u8>)> {
    segment::encode(&golden_records(), 0, MPY, &provenance(), &dictionaries()).unwrap()
}

/// The pinned XXH3-128 of the golden segment's files (see the module docs). A change here is a
/// change of the segment bytes: a new `parquet`, a changed property or a changed format. It is
/// updated only together with the reason, in the PR that makes the change.
const GOLDEN_SEGMENT_BYTES: &str = "8f0799bae0b2c24ded75efec76e95a82";

fn files_digest(files: &[(String, Vec<u8>)]) -> String {
    let mut b = Vec::new();
    for (name, bytes) in files {
        b.extend_from_slice(&u32::try_from(name.len()).unwrap().to_le_bytes());
        b.extend_from_slice(name.as_bytes());
        b.extend_from_slice(&u64::try_from(bytes.len()).unwrap().to_le_bytes());
        b.extend_from_slice(bytes);
    }
    hex16(&xxh3_128(&b))
}

#[test]
fn the_golden_segments_bytes_are_pinned() {
    let files = golden_files();
    assert_eq!(
        files,
        golden_files(),
        "encoding is a function of the records"
    );
    let got = files_digest(&files);
    println!(
        "T-09 golden segment bytes: xxh3-128 {got} over {} files",
        files.len()
    );
    assert_eq!(
        got, GOLDEN_SEGMENT_BYTES,
        "the golden segment's bytes changed: a parquet upgrade, a writer property or the format"
    );
}

#[test]
fn every_file_carries_exactly_the_pinned_writer_properties() {
    let files = golden_files();
    assert_eq!(files.len(), CHANNEL_A.len());
    assert_eq!(files.last().unwrap().0, segment::SEGMENT_FILE);
    let level = ZstdLevel::try_new(ZSTD).unwrap();
    for (name, bytes) in &files {
        let r = SerializedFileReader::new(Bytes::copy_from_slice(bytes)).unwrap();
        let meta = r.metadata();
        let file = meta.file_metadata();
        assert_eq!(file.created_by(), Some(CREATED_BY), "{name}");
        assert!(
            file.key_value_metadata().is_none(),
            "{name}: key-value metadata"
        );
        assert_eq!(file.version(), 1, "{name}: Parquet 1.0");
        assert_eq!(meta.num_row_groups(), 1, "{name}");
        let group = meta.row_group(0);
        assert!(group.sorting_columns().is_none(), "{name}");
        for col in group.columns() {
            let path = col.column_path().string();
            // The file names the codec only; the level is the properties' (below).
            assert!(
                matches!(col.compression(), Compression::ZSTD(_)),
                "{name}.{path}: {:?}",
                col.compression()
            );
            assert!(
                col.dictionary_page_offset().is_none(),
                "{name}.{path}: dictionary"
            );
            assert!(col.statistics().is_none(), "{name}.{path}: statistics");
            assert!(
                col.bloom_filter_offset().is_none(),
                "{name}.{path}: bloom filter"
            );
            assert!(
                col.column_index_offset().is_none(),
                "{name}.{path}: column index"
            );
            assert!(
                col.offset_index_offset().is_some(),
                "{name}.{path}: offset index"
            );
            let encodings: Vec<Encoding> = col.encodings().collect();
            assert!(
                !encodings.contains(&Encoding::RLE_DICTIONARY)
                    && !encodings.contains(&Encoding::PLAIN_DICTIONARY),
                "{name}.{path}: {encodings:?}"
            );
        }
    }
    let props = parquet_io::properties().unwrap();
    let any = parquet::schema::types::ColumnPath::from("any");
    assert_eq!(props.compression(&any), Compression::ZSTD(level));
    assert!(!props.dictionary_enabled(&any));
    assert!(props.bloom_filter_properties(&any).is_none());
    assert_eq!(
        props.statistics_enabled(&any),
        parquet::file::properties::EnabledStatistics::None
    );
    assert!(props.key_value_metadata().is_none());
    assert_eq!(props.created_by(), CREATED_BY);
}

/// R2: no channel-A table has a column of a float physical type, list elements included.
#[test]
fn no_channel_a_column_is_a_float() {
    for (name, bytes) in golden_files() {
        let r = SerializedFileReader::new(Bytes::from(bytes)).unwrap();
        let schema = r.metadata().file_metadata().schema_descr_ptr();
        for col in schema.columns() {
            let t = col.physical_type();
            assert!(
                !matches!(t, Physical::FLOAT | Physical::DOUBLE),
                "{name}.{}: {t:?}",
                col.path().string()
            );
        }
    }
}
