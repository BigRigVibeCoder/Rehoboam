//! Keyframes: the plane-A state written as Arrow IPC with zstd, and read back into an engine
//! (SPR-007 T-02, ruling 2; V-DET-05).
//!
//! # Format `rehoboam/rh-snapshot/keyframe/v1`
//!
//! A keyframe file is an envelope, then one Arrow IPC file. The envelope is the 8 bytes
//! [`KEYFRAME_MAGIC`] and the XXH3-128 (big-endian) of the Arrow bytes that follow; it is checked
//! before Arrow reads anything, because Arrow's IPC reader panics on some corrupt input instead of
//! failing (a bad buffer offset). The digest catches corruption and truncation, not forgery: the
//! state hash and the history check below are what bind a keyframe to its run.
//!
//! The Arrow IPC file is zstd level 3 (`05.2` §6: "Arrow IPC + zstd-3"), with a fixed schema of two
//! non-null columns, `table: Utf8` and `row: Binary`. Every plane-A table of the
//! [`EngineState`], world columns and engine tables alike, is stored as its rows in their
//! canonical encoding, the same bytes `state_hash` hashes ([`crate::hash`], [`crate::state`]); the
//! carried tables ([`crate::state::CARRIED_TABLES`], the undrained events) are stored too, so a load
//! restores them, though `state_hash` does not cover them. Rows go in
//! record batches of at most [`BATCH_ROWS`] rows. A world column's row is its `i64`, 8 bytes
//! little-endian.
//!
//! The header comes first, as the rows of the reserved table [`HEADER_TABLE`] (`:` is in no
//! `entity.column` name and no engine table name): each row is the UTF-8 line `key<TAB>value`,
//! in this order, every key once (`table` repeats, once per table):
//!
//! | Key | Value |
//! |---|---|
//! | `format` | the format, [`KEYFRAME_FORMAT`] |
//! | `hash_layout` | the hash layout, [`crate::hash::LAYOUT`] |
//! | `build`, `registry_version`, `manifest_hash` | the history tuple's strings |
//! | `world_seed`, `branch_salt` | decimal `u64` |
//! | `journal_timeline`, `journal_inputs`, `journal_head` | the [`JournalPosition`]: decimal, decimal, 32 hex digits |
//! | `state_hash` | 16 hex digits |
//! | `table` | `name<TAB>row count`, for every table in byte order of names |
//!
//! The table lines keep empty tables, which have no rows to carry their name. Arrow's own schema
//! metadata is not used: it is a hash map, which plane A does not name (GOV-003 §2).
//!
//! Keyframe bytes need not be reproducible (ruling 2); the state they restore must hash equal.
//! [`read_keyframe`] decodes, checks the stored hash against the decoded state, and returns it;
//! [`load_engine`] also checks the history tuple, restores the engine
//! ([`Engine::restore`], canonical form only) and checks the restored engine's hash once more.
//! A keyframe of a poisoned engine is refused.
//!
//! The keyframe's [`JournalPosition`] is the anchor that catches a journal truncated together with
//! a forged count and head (`crate::journal`, GAP-5).
//!
//! REF: `docs/05_architecture/05.2_world_model_and_data.md` §6; `05.1` §6 item 6; `docs/09`
//! V-DET-05; `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md` T-02, ruling 2.

use std::collections::BTreeMap;
use std::fmt;
use std::io::Cursor;
use std::sync::Arc;

use arrow_array::{Array, ArrayRef, BinaryArray, RecordBatch, StringArray};
use arrow_ipc::CompressionType;
use arrow_ipc::reader::FileReader;
use arrow_ipc::writer::{FileWriter, IpcWriteOptions};
use arrow_schema::{ArrowError, DataType, Field, Schema};
use rh_core::crash::Provenance;
use rh_core::engine::Engine;
use rh_core::register::Registered;
use rh_core::state::{EngineState, RestoreError};
use rh_core::time::Calendar;
use rh_core::world::WorldTables;
use rh_ids::BranchSalt;

use crate::hash::{LAYOUT, xxh3_128};
use crate::journal::JournalPosition;
use crate::state::{DecodeError, decode_engine_tables, engine_tables, state_hash};

/// The keyframe format.
pub const KEYFRAME_FORMAT: &str = "rehoboam/rh-snapshot/keyframe/v1";

/// The first 8 bytes of every keyframe file, before the digest of the Arrow bytes.
pub const KEYFRAME_MAGIC: [u8; 8] = *b"RHKF\0v1\n";

/// The envelope's length: the magic and the XXH3-128 digest.
const ENVELOPE: usize = 24;

/// The most rows in one record batch.
pub const BATCH_ROWS: usize = 65_536;

/// The zstd level (`05.2` §6).
pub const ZSTD_LEVEL: i32 = 3;

/// The prefix of engine table names ([`crate::state`]); every other table is a world column.
const ENGINE_PREFIX: &str = "rh-core/";

/// The reserved table that holds the header.
pub const HEADER_TABLE: &str = "rehoboam:keyframe";

/// The header keys, in their order (`table` lines follow them).
const HEADER_KEYS: [&str; 11] = [
    "format",
    "hash_layout",
    "build",
    "registry_version",
    "manifest_hash",
    "world_seed",
    "branch_salt",
    "journal_timeline",
    "journal_inputs",
    "journal_head",
    "state_hash",
];

/// The history tuple a keyframe belongs to: the engine build, Registry version and manifest hash
/// ([`Provenance`]), the world seed and the branch salt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History {
    /// Build, Registry version and manifest hash.
    pub provenance: Provenance,
    /// The world seed.
    pub world_seed: u64,
    /// The timeline's branch salt.
    pub branch_salt: u64,
}

/// What a keyframe's header says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyframeHeader {
    /// The history tuple.
    pub history: History,
    /// The journal position the state corresponds to.
    pub journal: JournalPosition,
    /// `state_hash` of the stored state.
    pub state_hash: u64,
}

/// A keyframe that cannot be written, read or restored.
#[derive(Debug)]
pub enum KeyframeError {
    /// Arrow failed (I/O, IPC, compression).
    Arrow(ArrowError),
    /// The file is not a keyframe of this format.
    Format(String),
    /// The engine is poisoned: its state is the failed instant's, not a history's.
    Poisoned(String),
    /// The keyframe belongs to another history.
    History(&'static str),
    /// The engine tables do not decode.
    Decode(DecodeError),
    /// The decoded state is not one the engine restores.
    Restore(RestoreError),
    /// The state's hash differs from the stored one.
    HashMismatch {
        /// The hash in the header.
        stored: u64,
        /// The hash of the decoded or restored state.
        computed: u64,
    },
}

impl fmt::Display for KeyframeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arrow(e) => write!(f, "keyframe: Arrow: {e}"),
            Self::Format(m) => write!(f, "keyframe: not a {KEYFRAME_FORMAT} file: {m}"),
            Self::Poisoned(e) => write!(f, "keyframe: the engine is poisoned ({e})"),
            Self::History(field) => write!(f, "keyframe: belongs to another history ({field})"),
            Self::Decode(e) => write!(f, "keyframe: {e}"),
            Self::Restore(e) => write!(f, "keyframe: {e}"),
            Self::HashMismatch { stored, computed } => write!(
                f,
                "keyframe: the state hashes to {computed:016x}, not the stored {stored:016x}"
            ),
        }
    }
}

impl std::error::Error for KeyframeError {}

impl From<ArrowError> for KeyframeError {
    fn from(e: ArrowError) -> Self {
        Self::Arrow(e)
    }
}

fn schema() -> Schema {
    Schema::new(vec![
        Field::new("table", DataType::Utf8, false),
        Field::new("row", DataType::Binary, false),
    ])
}

/// Every table of a state as `(name, rows)`, in byte order of names.
fn all_tables(state: &EngineState) -> BTreeMap<String, Vec<Vec<u8>>> {
    let mut tables: BTreeMap<String, Vec<Vec<u8>>> = state
        .tables
        .columns()
        .map(|(name, _, rows)| {
            (
                name.to_owned(),
                rows.iter().map(|v| v.to_le_bytes().to_vec()).collect(),
            )
        })
        .collect();
    for (name, rows) in engine_tables(state) {
        tables.insert(name.to_owned(), rows);
    }
    tables
}

/// Writes a keyframe of `engine` for `history` at journal position `journal`, and returns its
/// bytes and header.
///
/// # Errors
/// [`KeyframeError::Poisoned`] for a poisoned engine; [`KeyframeError::Format`] if the journal
/// position is another timeline's or a table name holds a tab or newline;
/// [`KeyframeError::Arrow`] if Arrow fails.
pub fn write_keyframe(
    engine: &Engine,
    history: &History,
    journal: JournalPosition,
) -> Result<(Vec<u8>, KeyframeHeader), KeyframeError> {
    if let Some(e) = engine.poisoned() {
        return Err(KeyframeError::Poisoned(e.to_string()));
    }
    let state = engine.snapshot();
    if journal.timeline_id != state.timeline.to_raw() {
        return Err(KeyframeError::Format(format!(
            "the journal position is timeline {}'s, the engine runs timeline {}",
            journal.timeline_id,
            state.timeline.to_raw()
        )));
    }
    let header = KeyframeHeader {
        history: history.clone(),
        journal,
        state_hash: state_hash(&state),
    };
    let tables = all_tables(&state);
    if let Some(bad) = tables.keys().find(|n| n.contains(['\t', '\n'])) {
        return Err(KeyframeError::Format(format!("table name {bad:?}")));
    }
    let schema = Arc::new(schema());
    let options = IpcWriteOptions::default()
        .try_with_compression(Some(CompressionType::ZSTD))?
        .try_with_compression_level(Some(ZSTD_LEVEL))?;
    let mut writer = FileWriter::try_new_with_options(Vec::new(), &schema, options)?;
    let header_rows = header_lines(&header, &tables);
    let all = std::iter::once((HEADER_TABLE, &header_rows))
        .chain(tables.iter().map(|(n, r)| (n.as_str(), r)));
    for (name, rows) in all {
        for chunk in rows.chunks(BATCH_ROWS) {
            let names: ArrayRef = Arc::new(StringArray::from(vec![name; chunk.len()]));
            let values: ArrayRef = Arc::new(BinaryArray::from_iter_values(chunk.iter()));
            writer.write(&RecordBatch::try_new(schema.clone(), vec![names, values])?)?;
        }
    }
    writer.finish()?;
    Ok((seal_keyframe(&writer.into_inner()?), header))
}

/// Wraps Arrow IPC bytes in the keyframe envelope: [`KEYFRAME_MAGIC`], then their XXH3-128.
#[must_use]
pub fn seal_keyframe(arrow: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(ENVELOPE + arrow.len());
    out.extend_from_slice(&KEYFRAME_MAGIC);
    out.extend_from_slice(&xxh3_128(arrow));
    out.extend_from_slice(arrow);
    out
}

/// The Arrow IPC bytes inside a keyframe file, once the envelope checks out.
///
/// # Errors
/// [`KeyframeError::Format`] if the magic is wrong or the digest does not match (corruption or
/// truncation).
pub fn unseal_keyframe(bytes: &[u8]) -> Result<&[u8], KeyframeError> {
    if bytes.len() < ENVELOPE || bytes[..8] != KEYFRAME_MAGIC {
        return Err(KeyframeError::Format("no keyframe envelope".to_owned()));
    }
    let arrow = &bytes[ENVELOPE..];
    if bytes[8..ENVELOPE] != xxh3_128(arrow) {
        return Err(KeyframeError::Format(
            "the file digest does not match (corrupt or truncated)".to_owned(),
        ));
    }
    Ok(arrow)
}

fn header_lines(header: &KeyframeHeader, tables: &BTreeMap<String, Vec<Vec<u8>>>) -> Vec<Vec<u8>> {
    let h = &header.history;
    let values = [
        KEYFRAME_FORMAT.to_owned(),
        LAYOUT.to_owned(),
        h.provenance.build.clone(),
        h.provenance.registry_version.clone(),
        h.provenance.manifest_hash.clone(),
        h.world_seed.to_string(),
        h.branch_salt.to_string(),
        header.journal.timeline_id.to_string(),
        header.journal.inputs.to_string(),
        hex(&header.journal.head),
        format!("{:016x}", header.state_hash),
    ];
    HEADER_KEYS
        .iter()
        .zip(values)
        .map(|(k, v)| format!("{k}\t{v}"))
        .chain(
            tables
                .iter()
                .map(|(name, rows)| format!("table\t{name}\t{}", rows.len())),
        )
        .map(String::into_bytes)
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        // Writing to a String cannot fail.
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn unhex16(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32 {
        return None;
    }
    let mut out = [0; 16];
    for (k, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(2 * k..2 * k + 2)?, 16).ok()?;
    }
    Some(out)
}

/// Reads a keyframe into its state and header, names resolved through `registered`, and checks
/// that the state hashes to the stored hash.
///
/// # Errors
/// [`KeyframeError::Arrow`], [`KeyframeError::Format`], [`KeyframeError::Decode`] or
/// [`KeyframeError::HashMismatch`].
pub fn read_keyframe(
    bytes: &[u8],
    registered: &Registered,
) -> Result<(EngineState, KeyframeHeader), KeyframeError> {
    let reader = FileReader::try_new(Cursor::new(unseal_keyframe(bytes)?), None)?;
    let mut lines: Vec<Vec<u8>> = Vec::new();
    let mut tables: BTreeMap<String, Vec<Vec<u8>>> = BTreeMap::new();
    for batch in reader {
        let batch = batch?;
        let (names, rows) = columns(&batch)?;
        for k in 0..batch.num_rows() {
            let row = rows.value(k).to_vec();
            if names.value(k) == HEADER_TABLE {
                lines.push(row);
            } else {
                tables
                    .entry(names.value(k).to_owned())
                    .or_default()
                    .push(row);
            }
        }
    }
    let (header, expected) = parse_header(&lines)?;
    for (name, count) in &expected {
        let got = tables.get(name).map_or(0, Vec::len);
        if got != *count {
            return Err(KeyframeError::Format(format!(
                "table {name} has {got} rows, the header says {count}"
            )));
        }
        tables.entry(name.clone()).or_default();
    }
    if let Some(extra) = tables.keys().find(|k| !expected.contains_key(*k)) {
        return Err(KeyframeError::Format(format!(
            "table {extra} is not in the header"
        )));
    }
    let (engine, world): (BTreeMap<_, _>, BTreeMap<_, _>) = tables
        .into_iter()
        .partition(|(name, _)| name.starts_with(ENGINE_PREFIX));
    let mut tables_out = WorldTables::new();
    for (name, rows) in world {
        let column = rows
            .iter()
            .map(|r| {
                <[u8; 8]>::try_from(r.as_slice())
                    .map(i64::from_le_bytes)
                    .map_err(|_| KeyframeError::Format(format!("a row of {name} is not 8 bytes")))
            })
            .collect::<Result<Vec<i64>, _>>()?;
        tables_out.insert(&name, column);
    }
    let state =
        decode_engine_tables(&engine, registered, tables_out).map_err(KeyframeError::Decode)?;
    let computed = state_hash(&state);
    if computed != header.state_hash {
        return Err(KeyframeError::HashMismatch {
            stored: header.state_hash,
            computed,
        });
    }
    Ok((state, header))
}

/// Parses the header lines into the header and the expected table row counts.
fn parse_header(
    lines: &[Vec<u8>],
) -> Result<(KeyframeHeader, BTreeMap<String, usize>), KeyframeError> {
    let bad = |m: String| KeyframeError::Format(m);
    let text: Vec<&str> = lines
        .iter()
        .map(|l| std::str::from_utf8(l).map_err(|_| bad("a header line is not UTF-8".to_owned())))
        .collect::<Result<_, _>>()?;
    if text.len() < HEADER_KEYS.len() {
        return Err(bad(format!("{} header lines", text.len())));
    }
    let mut values = Vec::new();
    for (key, line) in HEADER_KEYS.iter().zip(&text) {
        match line.split_once('\t') {
            Some((k, v)) if k == *key => values.push(v),
            _ => return Err(bad(format!("header line {line:?} where {key} is expected"))),
        }
    }
    let num = |k: usize| -> Result<u64, KeyframeError> {
        values[k]
            .parse()
            .map_err(|_| bad(format!("{} is not a decimal u64", HEADER_KEYS[k])))
    };
    if values[0] != KEYFRAME_FORMAT {
        return Err(bad(format!("format {:?}", values[0])));
    }
    if values[1] != LAYOUT {
        return Err(bad(format!("hash layout {:?}", values[1])));
    }
    let header = KeyframeHeader {
        history: History {
            provenance: Provenance {
                build: values[2].to_owned(),
                registry_version: values[3].to_owned(),
                manifest_hash: values[4].to_owned(),
            },
            world_seed: num(5)?,
            branch_salt: num(6)?,
        },
        journal: JournalPosition {
            timeline_id: u32::try_from(num(7)?).map_err(|_| bad("journal timeline".to_owned()))?,
            inputs: u32::try_from(num(8)?).map_err(|_| bad("journal inputs".to_owned()))?,
            head: unhex16(values[9]).ok_or_else(|| bad("journal head".to_owned()))?,
        },
        state_hash: u64::from_str_radix(values[10], 16)
            .map_err(|_| bad("state hash".to_owned()))?,
    };
    let mut expected = BTreeMap::new();
    for line in &text[HEADER_KEYS.len()..] {
        let parsed = line
            .strip_prefix("table\t")
            .and_then(|rest| rest.split_once('\t'))
            .and_then(|(name, count)| Some((name, count.parse::<usize>().ok()?)));
        let Some((name, count)) = parsed else {
            return Err(bad(format!(
                "header line {line:?} where a table line is expected"
            )));
        };
        if expected.insert(name.to_owned(), count).is_some() {
            return Err(bad(format!("table {name} listed twice")));
        }
    }
    Ok((header, expected))
}

fn columns(batch: &RecordBatch) -> Result<(&StringArray, &BinaryArray), KeyframeError> {
    let bad = || KeyframeError::Format("the batch is not (table: Utf8, row: Binary)".to_owned());
    if batch.num_columns() != 2 {
        return Err(bad());
    }
    let names = batch
        .column(0)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(bad)?;
    let rows = batch
        .column(1)
        .as_any()
        .downcast_ref::<BinaryArray>()
        .ok_or_else(bad)?;
    if names.null_count() > 0 || rows.null_count() > 0 {
        return Err(bad());
    }
    Ok((names, rows))
}

/// Loads a keyframe into a running engine: reads it, checks that it belongs to `expected`,
/// restores the engine for `registered` ([`Engine::restore`]) and checks that the restored engine
/// hashes to the stored hash.
///
/// # Errors
/// Every [`read_keyframe`] error; [`KeyframeError::History`] naming the first differing field;
/// [`KeyframeError::Restore`]; [`KeyframeError::HashMismatch`] for the restored engine.
pub fn load_engine(
    bytes: &[u8],
    calendar: Calendar,
    registered: Registered,
    expected: &History,
) -> Result<(Engine, KeyframeHeader), KeyframeError> {
    let (state, header) = read_keyframe(bytes, &registered)?;
    let h = &header.history;
    let checks: [(&'static str, bool); 5] = [
        ("build", h.provenance.build == expected.provenance.build),
        (
            "registry_version",
            h.provenance.registry_version == expected.provenance.registry_version,
        ),
        (
            "manifest_hash",
            h.provenance.manifest_hash == expected.provenance.manifest_hash,
        ),
        ("world_seed", h.world_seed == expected.world_seed),
        ("branch_salt", h.branch_salt == expected.branch_salt),
    ];
    if let Some((field, _)) = checks.iter().find(|(_, ok)| !ok) {
        return Err(KeyframeError::History(field));
    }
    let engine = Engine::restore(
        calendar,
        h.world_seed,
        BranchSalt::from_raw(h.branch_salt),
        registered,
        h.provenance.clone(),
        state,
    )
    .map_err(KeyframeError::Restore)?;
    let computed = state_hash(&engine.snapshot());
    if computed != header.state_hash {
        return Err(KeyframeError::HashMismatch {
            stored: header.state_hash,
            computed,
        });
    }
    Ok((engine, header))
}
