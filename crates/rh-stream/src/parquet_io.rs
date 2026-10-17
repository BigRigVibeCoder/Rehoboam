//! Tables to Parquet bytes and back, with the writer properties pinned (SPR-011 ruling 13;
//! the Architect's review of `parquet`, SPR-013).
//!
//! A segment table is a list of typed columns ([`ColumnSpec`]); [`write_table`] encodes one with
//! `parquet`'s low-level column writers (no Arrow), [`read_table`] decodes one and refuses a file
//! whose schema is not exactly the expected one. The column types are the DDL's (`DuckDB` names):
//! `UTINYINT`, `USMALLINT`, `UINTEGER`, `UBIGINT`, `INTEGER`, `BIGINT`, `BOOLEAN`, `VARCHAR`,
//! `BLOB`, `BIGINT[]`, `VARCHAR[]`, `SMALLINT`, and nullable `UTINYINT`, `USMALLINT`, `UINTEGER`,
//! `UBIGINT`, `BIGINT`, with the
//! Parquet logical types `DuckDB` reads back as those types. There is no float type (R2), so no
//! float column can be declared.
//!
//! **Byte identity is a contract** (V-STR-01). Every property that reaches the file is set
//! in [`properties`], never left to the library's defaults, and `tests/parquet_props.rs` asserts
//! them: a fixed `created_by`, dictionary encoding off, no statistics (so no column index and no
//! truncation), no bloom filter, no page checksum, no key-value metadata, zstd at a fixed level,
//! Parquet 1.0 data pages, fixed page and row-group limits, the offset index on, no content-defined
//! chunking. A golden segment's bytes are pinned by their digest (`tests/parquet_props.rs`), so a
//! library upgrade that changes the layout fails loudly; the canonical digest survives it.
//!
//! REF: SPR-011 ruling 13; `specs/stream/channel_a_v0.sql` R2, R7; `05.10` §5.

use std::sync::Arc;

use bytes::Bytes;
use parquet::basic::{Compression, ZstdLevel};
use parquet::data_type::{BoolType, ByteArray, ByteArrayType, Int32Type, Int64Type};
use parquet::file::properties::{EnabledStatistics, WriterProperties, WriterVersion};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::file::writer::SerializedFileWriter;
use parquet::record::Field;
use parquet::schema::parser::parse_message_type;

use crate::StreamError;

/// The writer's `created_by`: fixed, so that no library version string reaches the bytes.
pub const CREATED_BY: &str = "rehoboam rh-stream v0";
/// zstd's level for every segment file.
pub const ZSTD: i32 = 3;
/// Rows per data page at most.
pub const PAGE_ROWS: usize = 20_000;
/// Bytes per data page at most (before compression).
pub const PAGE_BYTES: usize = 1 << 20;
/// Rows per row group at most.
pub const ROW_GROUP_ROWS: usize = 1 << 20;
/// Values written per batch.
pub const WRITE_BATCH: usize = 1024;

/// A column type: the DDL's type and whether it is nullable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnType {
    /// `UTINYINT NOT NULL`.
    U8,
    /// `USMALLINT NOT NULL`.
    U16,
    /// `UINTEGER NOT NULL`.
    U32,
    /// `UBIGINT NOT NULL`.
    U64,
    /// `SMALLINT NOT NULL`.
    I16,
    /// `INTEGER NOT NULL`.
    I32,
    /// `BIGINT NOT NULL`.
    I64,
    /// `BOOLEAN NOT NULL`.
    Bool,
    /// `VARCHAR NOT NULL`.
    Str,
    /// `BLOB NOT NULL`.
    Blob,
    /// `BIGINT[] NOT NULL` (elements not null).
    ListI64,
    /// `VARCHAR[] NOT NULL` (elements not null).
    ListStr,
    /// `UTINYINT`, nullable.
    OptU8,
    /// `USMALLINT`, nullable.
    OptU16,
    /// `UINTEGER`, nullable.
    OptU32,
    /// `UBIGINT`, nullable.
    OptU64,
    /// `BIGINT`, nullable.
    OptI64,
}

impl ColumnType {
    /// The `DuckDB` type name the column reads back as.
    #[must_use]
    pub const fn duckdb(self) -> &'static str {
        match self {
            Self::U8 | Self::OptU8 => "UTINYINT",
            Self::U16 | Self::OptU16 => "USMALLINT",
            Self::I16 => "SMALLINT",
            Self::U32 | Self::OptU32 => "UINTEGER",
            Self::U64 | Self::OptU64 => "UBIGINT",
            Self::I32 => "INTEGER",
            Self::I64 | Self::OptI64 => "BIGINT",
            Self::Bool => "BOOLEAN",
            Self::Str => "VARCHAR",
            Self::Blob => "BLOB",
            Self::ListI64 => "BIGINT[]",
            Self::ListStr => "VARCHAR[]",
        }
    }

    /// The Parquet schema line of a column named `name`.
    fn schema(self, name: &str) -> String {
        let int = |rep: &str, phys: &str, logical: &str| {
            format!("{rep} {phys} {name} (INTEGER({logical}));")
        };
        match self {
            Self::U8 => int("required", "int32", "8,false"),
            Self::U16 => int("required", "int32", "16,false"),
            Self::U32 => int("required", "int32", "32,false"),
            Self::U64 => int("required", "int64", "64,false"),
            Self::OptU8 => int("optional", "int32", "8,false"),
            Self::OptU16 => int("optional", "int32", "16,false"),
            Self::OptU32 => int("optional", "int32", "32,false"),
            Self::I16 => int("required", "int32", "16,true"),
            Self::OptU64 => int("optional", "int64", "64,false"),
            Self::I32 => format!("required int32 {name};"),
            Self::I64 => format!("required int64 {name};"),
            Self::OptI64 => format!("optional int64 {name};"),
            Self::Bool => format!("required boolean {name};"),
            Self::Str => format!("required binary {name} (STRING);"),
            Self::Blob => format!("required binary {name};"),
            Self::ListI64 => format!(
                "required group {name} (LIST) {{ repeated group list {{ required int64 element; }} }}"
            ),
            Self::ListStr => format!(
                "required group {name} (LIST) {{ repeated group list {{ required binary element (STRING); }} }}"
            ),
        }
    }
}

/// One column of a table: its name and type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnSpec {
    /// The column name (the DDL's).
    pub name: &'static str,
    /// Its type.
    pub ty: ColumnType,
}

/// A column's values, one variant per [`ColumnType`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Column {
    /// `UTINYINT`.
    U8(Vec<u8>),
    /// `USMALLINT`.
    U16(Vec<u16>),
    /// `UINTEGER`.
    U32(Vec<u32>),
    /// `UBIGINT`.
    U64(Vec<u64>),
    /// `SMALLINT`.
    I16(Vec<i16>),
    /// `INTEGER`.
    I32(Vec<i32>),
    /// `BIGINT`.
    I64(Vec<i64>),
    /// `BOOLEAN`.
    Bool(Vec<bool>),
    /// `VARCHAR`.
    Str(Vec<String>),
    /// `BLOB`.
    Blob(Vec<Vec<u8>>),
    /// `BIGINT[]`.
    ListI64(Vec<Vec<i64>>),
    /// `VARCHAR[]`.
    ListStr(Vec<Vec<String>>),
    /// Nullable `UTINYINT`.
    OptU8(Vec<Option<u8>>),
    /// Nullable `USMALLINT`.
    OptU16(Vec<Option<u16>>),
    /// Nullable `UINTEGER`.
    OptU32(Vec<Option<u32>>),
    /// Nullable `UBIGINT`.
    OptU64(Vec<Option<u64>>),
    /// Nullable `BIGINT`.
    OptI64(Vec<Option<i64>>),
}

impl Column {
    /// An empty column of `ty`.
    #[must_use]
    pub const fn empty(ty: ColumnType) -> Self {
        match ty {
            ColumnType::U8 => Self::U8(Vec::new()),
            ColumnType::U16 => Self::U16(Vec::new()),
            ColumnType::U32 => Self::U32(Vec::new()),
            ColumnType::U64 => Self::U64(Vec::new()),
            ColumnType::I16 => Self::I16(Vec::new()),
            ColumnType::I32 => Self::I32(Vec::new()),
            ColumnType::I64 => Self::I64(Vec::new()),
            ColumnType::Bool => Self::Bool(Vec::new()),
            ColumnType::Str => Self::Str(Vec::new()),
            ColumnType::Blob => Self::Blob(Vec::new()),
            ColumnType::ListI64 => Self::ListI64(Vec::new()),
            ColumnType::ListStr => Self::ListStr(Vec::new()),
            ColumnType::OptU8 => Self::OptU8(Vec::new()),
            ColumnType::OptU16 => Self::OptU16(Vec::new()),
            ColumnType::OptU32 => Self::OptU32(Vec::new()),
            ColumnType::OptU64 => Self::OptU64(Vec::new()),
            ColumnType::OptI64 => Self::OptI64(Vec::new()),
        }
    }

    /// Its type.
    #[must_use]
    pub const fn ty(&self) -> ColumnType {
        match self {
            Self::U8(_) => ColumnType::U8,
            Self::U16(_) => ColumnType::U16,
            Self::U32(_) => ColumnType::U32,
            Self::U64(_) => ColumnType::U64,
            Self::I16(_) => ColumnType::I16,
            Self::I32(_) => ColumnType::I32,
            Self::I64(_) => ColumnType::I64,
            Self::Bool(_) => ColumnType::Bool,
            Self::Str(_) => ColumnType::Str,
            Self::Blob(_) => ColumnType::Blob,
            Self::ListI64(_) => ColumnType::ListI64,
            Self::ListStr(_) => ColumnType::ListStr,
            Self::OptU8(_) => ColumnType::OptU8,
            Self::OptU16(_) => ColumnType::OptU16,
            Self::OptU32(_) => ColumnType::OptU32,
            Self::OptU64(_) => ColumnType::OptU64,
            Self::OptI64(_) => ColumnType::OptI64,
        }
    }

    /// Its row count.
    #[must_use]
    pub const fn len(&self) -> usize {
        match self {
            Self::U8(v) => v.len(),
            Self::U16(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::U64(v) => v.len(),
            Self::I16(v) => v.len(),
            Self::I32(v) => v.len(),
            Self::I64(v) => v.len(),
            Self::Bool(v) => v.len(),
            Self::Str(v) => v.len(),
            Self::Blob(v) => v.len(),
            Self::ListI64(v) => v.len(),
            Self::ListStr(v) => v.len(),
            Self::OptU8(v) => v.len(),
            Self::OptU16(v) => v.len(),
            Self::OptU32(v) => v.len(),
            Self::OptU64(v) => v.len(),
            Self::OptI64(v) => v.len(),
        }
    }

    /// Whether it has no rows.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The pinned writer properties (module docs). The one place they are set.
///
/// # Errors
/// [`StreamError::Parquet`] if the library refuses the pinned zstd level (never a silent fallback
/// to its default: that would change the bytes; REV-SPR-013 NIT).
pub fn properties() -> Result<WriterProperties, StreamError> {
    let zstd = ZstdLevel::try_new(ZSTD).map_err(pq("zstd level"))?;
    Ok(WriterProperties::builder()
        .set_writer_version(WriterVersion::PARQUET_1_0)
        .set_created_by(CREATED_BY.to_owned())
        .set_key_value_metadata(None)
        .set_sorting_columns(None)
        .set_compression(Compression::ZSTD(zstd))
        .set_dictionary_enabled(false)
        .set_statistics_enabled(EnabledStatistics::None)
        .set_write_page_header_statistics(false)
        .set_statistics_truncate_length(None)
        .set_column_index_truncate_length(None)
        .set_bloom_filter_enabled(false)
        .set_offset_index_disabled(false)
        .set_write_path_in_schema(true)
        .set_content_defined_chunking(None)
        .set_data_page_row_count_limit(PAGE_ROWS)
        .set_data_page_size_limit(PAGE_BYTES)
        .set_dictionary_page_size_limit(PAGE_BYTES)
        .set_write_batch_size(WRITE_BATCH)
        .set_max_row_group_row_count(Some(ROW_GROUP_ROWS))
        .set_max_row_group_bytes(None)
        .set_write_row_group_number_distinct_values(false)
        .build())
}

fn pq(context: &str) -> impl Fn(parquet::errors::ParquetError) -> StreamError + '_ {
    move |e| StreamError::Parquet(format!("{context}: {e}"))
}

/// The Parquet message type of a table.
fn message(name: &str, spec: &[ColumnSpec]) -> String {
    let fields: Vec<String> = spec.iter().map(|c| c.ty.schema(c.name)).collect();
    format!("message {name} {{ {} }}", fields.join(" "))
}

/// Encodes one table: `columns[i]` holds the values of `spec[i]`, all of one length.
///
/// # Errors
/// [`StreamError::Table`] if the columns do not match the spec; [`StreamError::Parquet`] if the
/// library fails.
pub fn write_table(
    name: &str,
    spec: &[ColumnSpec],
    columns: &[Column],
) -> Result<Vec<u8>, StreamError> {
    if spec.len() != columns.len() {
        return Err(StreamError::Table(format!(
            "{name}: {} columns for a spec of {}",
            columns.len(),
            spec.len()
        )));
    }
    let rows = columns.first().map_or(0, Column::len);
    for (s, c) in spec.iter().zip(columns) {
        if s.ty != c.ty() || c.len() != rows {
            return Err(StreamError::Table(format!(
                "{name}.{}: a {:?} column of {} rows, expected {:?} of {rows}",
                s.name,
                c.ty(),
                c.len(),
                s.ty
            )));
        }
    }
    let schema = Arc::new(parse_message_type(&message(name, spec)).map_err(pq(name))?);
    let mut out = Vec::new();
    let mut writer =
        SerializedFileWriter::new(&mut out, schema, Arc::new(properties()?)).map_err(pq(name))?;
    let mut group = writer.next_row_group().map_err(pq(name))?;
    for c in columns {
        let Some(mut col) = group.next_column().map_err(pq(name))? else {
            return Err(StreamError::Table(format!(
                "{name}: the writer ran out of columns"
            )));
        };
        write_column(&mut col, c).map_err(pq(name))?;
        col.close().map_err(pq(name))?;
    }
    group.close().map_err(pq(name))?;
    writer.close().map_err(pq(name))?;
    Ok(out)
}

type PqResult<T> = Result<T, parquet::errors::ParquetError>;

fn bytes_of(v: &[u8]) -> ByteArray {
    ByteArray::from(v.to_vec())
}

fn write_column(
    col: &mut parquet::file::writer::SerializedColumnWriter<'_>,
    c: &Column,
) -> PqResult<()> {
    fn opt<T: Copy>(v: &[Option<T>]) -> (Vec<T>, Vec<i16>) {
        let values = v.iter().filter_map(|x| *x).collect();
        let defs = v.iter().map(|x| i16::from(x.is_some())).collect();
        (values, defs)
    }
    fn levels<T>(v: &[Vec<T>]) -> (Vec<i16>, Vec<i16>) {
        let (mut defs, mut reps) = (Vec::new(), Vec::new());
        for list in v {
            if list.is_empty() {
                defs.push(0);
                reps.push(0);
            }
            for i in 0..list.len() {
                defs.push(1);
                reps.push(i16::from(i > 0));
            }
        }
        (defs, reps)
    }
    match c {
        Column::U8(v) => {
            let x: Vec<i32> = v.iter().map(|&a| i32::from(a)).collect();
            col.typed::<Int32Type>().write_batch(&x, None, None)?;
        }
        Column::U16(v) => {
            let x: Vec<i32> = v.iter().map(|&a| i32::from(a)).collect();
            col.typed::<Int32Type>().write_batch(&x, None, None)?;
        }
        Column::U32(v) => {
            let x: Vec<i32> = v.iter().map(|&a| a.cast_signed()).collect();
            col.typed::<Int32Type>().write_batch(&x, None, None)?;
        }
        Column::U64(v) => {
            let x: Vec<i64> = v.iter().map(|&a| a.cast_signed()).collect();
            col.typed::<Int64Type>().write_batch(&x, None, None)?;
        }
        Column::I16(v) => {
            let x: Vec<i32> = v.iter().map(|&a| i32::from(a)).collect();
            col.typed::<Int32Type>().write_batch(&x, None, None)?;
        }
        Column::I32(v) => {
            col.typed::<Int32Type>().write_batch(v, None, None)?;
        }
        Column::I64(v) => {
            col.typed::<Int64Type>().write_batch(v, None, None)?;
        }
        Column::Bool(v) => {
            col.typed::<BoolType>().write_batch(v, None, None)?;
        }
        Column::Str(v) => {
            let x: Vec<ByteArray> = v.iter().map(|s| bytes_of(s.as_bytes())).collect();
            col.typed::<ByteArrayType>().write_batch(&x, None, None)?;
        }
        Column::Blob(v) => {
            let x: Vec<ByteArray> = v.iter().map(|b| bytes_of(b)).collect();
            col.typed::<ByteArrayType>().write_batch(&x, None, None)?;
        }
        Column::ListI64(v) => {
            let (defs, reps) = levels(v);
            let x: Vec<i64> = v.iter().flatten().copied().collect();
            col.typed::<Int64Type>()
                .write_batch(&x, Some(&defs), Some(&reps))?;
        }
        Column::ListStr(v) => {
            let (defs, reps) = levels(v);
            let x: Vec<ByteArray> = v.iter().flatten().map(|s| bytes_of(s.as_bytes())).collect();
            col.typed::<ByteArrayType>()
                .write_batch(&x, Some(&defs), Some(&reps))?;
        }
        Column::OptU8(v) => {
            let (values, defs) = opt(v);
            let x: Vec<i32> = values.iter().map(|&a| i32::from(a)).collect();
            col.typed::<Int32Type>()
                .write_batch(&x, Some(&defs), None)?;
        }
        Column::OptU16(v) => {
            let (values, defs) = opt(v);
            let x: Vec<i32> = values.iter().map(|&a| i32::from(a)).collect();
            col.typed::<Int32Type>()
                .write_batch(&x, Some(&defs), None)?;
        }
        Column::OptU32(v) => {
            let (values, defs) = opt(v);
            let x: Vec<i32> = values.iter().map(|&a| a.cast_signed()).collect();
            col.typed::<Int32Type>()
                .write_batch(&x, Some(&defs), None)?;
        }
        Column::OptU64(v) => {
            let (values, defs) = opt(v);
            let x: Vec<i64> = values.iter().map(|&a| a.cast_signed()).collect();
            col.typed::<Int64Type>()
                .write_batch(&x, Some(&defs), None)?;
        }
        Column::OptI64(v) => {
            let (values, defs) = opt(v);
            col.typed::<Int64Type>()
                .write_batch(&values, Some(&defs), None)?;
        }
    }
    Ok(())
}

/// Decodes one table written by [`write_table`] with `spec`, refusing a file whose schema is not
/// exactly the one `spec` gives (a column added, removed, renamed, retyped or reordered).
///
/// # Errors
/// [`StreamError::Table`] for a schema or value mismatch; [`StreamError::Parquet`] if the library
/// cannot read the bytes.
pub fn read_table(
    name: &str,
    spec: &[ColumnSpec],
    bytes: &[u8],
) -> Result<Vec<Column>, StreamError> {
    let reader = SerializedFileReader::new(Bytes::copy_from_slice(bytes)).map_err(pq(name))?;
    let expected = parse_message_type(&message(name, spec)).map_err(pq(name))?;
    let actual = reader.metadata().file_metadata().schema();
    if *actual != expected {
        return Err(StreamError::Table(format!(
            "{name}: the file's schema is not the segment format's"
        )));
    }
    let mut out: Vec<Column> = spec.iter().map(|c| Column::empty(c.ty)).collect();
    for row in reader.get_row_iter(None).map_err(pq(name))? {
        let row = row.map_err(pq(name))?;
        let fields: Vec<&Field> = row.get_column_iter().map(|(_, f)| f).collect();
        if fields.len() != spec.len() {
            return Err(StreamError::Table(format!(
                "{name}: a row of {} fields",
                fields.len()
            )));
        }
        for ((col, s), f) in out.iter_mut().zip(spec).zip(fields) {
            push(col, f).ok_or_else(|| {
                StreamError::Table(format!(
                    "{name}.{}: a value that is not a {:?}",
                    s.name, s.ty
                ))
            })?;
        }
    }
    Ok(out)
}

fn push(col: &mut Column, f: &Field) -> Option<()> {
    match (col, f) {
        (Column::U8(v), Field::UByte(x)) => v.push(*x),
        (Column::U16(v), Field::UShort(x)) => v.push(*x),
        (Column::U32(v), Field::UInt(x)) => v.push(*x),
        (Column::U64(v), Field::ULong(x)) => v.push(*x),
        (Column::I16(v), Field::Short(x)) => v.push(*x),
        (Column::I32(v), Field::Int(x)) => v.push(*x),
        (Column::I64(v), Field::Long(x)) => v.push(*x),
        (Column::Bool(v), Field::Bool(x)) => v.push(*x),
        (Column::Str(v), Field::Str(x)) => v.push(x.clone()),
        (Column::Blob(v), Field::Bytes(x)) => v.push(x.data().to_vec()),
        (Column::OptU8(v), Field::UByte(x)) => v.push(Some(*x)),
        (Column::OptU8(v), Field::Null) => v.push(None),
        (Column::OptU16(v), Field::UShort(x)) => v.push(Some(*x)),
        (Column::OptU16(v), Field::Null) => v.push(None),
        (Column::OptU32(v), Field::UInt(x)) => v.push(Some(*x)),
        (Column::OptU32(v), Field::Null) => v.push(None),
        (Column::OptU64(v), Field::ULong(x)) => v.push(Some(*x)),
        (Column::OptU64(v), Field::Null) => v.push(None),
        (Column::OptI64(v), Field::Long(x)) => v.push(Some(*x)),
        (Column::OptI64(v), Field::Null) => v.push(None),
        (Column::ListI64(v), Field::ListInternal(list)) => {
            let items = list
                .elements()
                .iter()
                .map(|e| match e {
                    Field::Long(x) => Some(*x),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?;
            v.push(items);
        }
        (Column::ListStr(v), Field::ListInternal(list)) => {
            let items = list
                .elements()
                .iter()
                .map(|e| match e {
                    Field::Str(x) => Some(x.clone()),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?;
            v.push(items);
        }
        _ => return None,
    }
    Some(())
}
