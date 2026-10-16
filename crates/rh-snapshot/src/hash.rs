//! The plane hashes: chunked Merkle XXH3-64 with dirty tracking (SPR-007 T-01).
//!
//! `state_hash` (plane A), `observation_hash` (plane B) and `narrative_hash` (plane C) are
//! separate hashes over separate table sets (ADR-0024). Each is a four-level Merkle tree, so a
//! divergence can be narrowed to a table and a chunk of rows (the bisect tool, `05.1` §6 item 6).
//!
//! # Layout `rehoboam/rh-snapshot/hash/v1` (normative)
//!
//! `XXH3(b)` is XXH3-64 with seed 0 and the default secret. Every integer is little-endian;
//! `u32(x)` and `u64(x)` are its 4 and 8 bytes. `||` concatenates.
//!
//! 1. **Row.** A table defines its rows' canonical encoding `row(i)`, a byte string, and their
//!    key order. The stub `WorldTables` column (`rh-core`) is a table whose row `i` is the
//!    column's `i`-th `i64` as its 8 two's-complement bytes, in index order.
//! 2. **Chunk.** Rows `[c·R, min((c+1)·R, n))` of a table with `n` rows form chunk `c`, with
//!    `R = 1024` ([`CHUNK_ROWS`]). There are `⌈n / R⌉` chunks; an empty table has none.
//!    `chunk(c) = XXH3(D_CHUNK || u64(c) || u64(m) || u32(len(row)) || row ...)` over the chunk's
//!    `m` rows in order.
//! 3. **Table root.** `root = XXH3(D_TABLE || u32(len(name)) || name || u64(n) || u64(k) ||
//!    u64(chunk(0)) ... u64(chunk(k-1)))`, with `name` the table's UTF-8 name, `n` its row count and
//!    `k` its chunk count.
//! 4. **Plane hash.** `plane = XXH3(D_PLANE || u8(p) || u64(t) || u32(len(name)) || name ||
//!    u64(root) ...)` over the plane's `t` tables in strictly increasing byte order of their UTF-8
//!    names, with `p` the ASCII letter of the plane (`A`, `B` or `C`). A plane with no tables
//!    hashes `D_PLANE || u8(p) || u64(0)`.
//!
//! Each level hashes under its own domain tag, the ASCII bytes (no terminator) of
//! `D_CHUNK = "rehoboam/rh-snapshot/hash/v1/chunk"`, `D_TABLE = "rehoboam/rh-snapshot/hash/v1/table"`
//! and `D_PLANE = "rehoboam/rh-snapshot/hash/v1/plane"`. The tags have equal length, so none is a
//! prefix of another; every variable-length field is length-prefixed, so each level's input
//! decodes uniquely. Two planes with identical tables differ by the plane letter, and two tables
//! with identical rows by their names.
//!
//! **Version line.** The layout's version is the `v1` component of every tag and of [`LAYOUT`]. Any
//! change to these bytes is a new version: new tags (`.../hash/v2/...`), regenerated golden vectors,
//! a C3 change.
//!
//! `bin/gen_hash_golden.py` re-implements XXH3 and this layout independently, in the Python
//! standard library, and writes `tests/golden/rh_snapshot_hash_v1.txt`; `tests/golden.rs` must
//! reproduce it bit for bit. Changing any byte of the layout is a C3 change (`docs/13` §4.2).
//!
//! # Dirty tracking
//!
//! [`MerkleCache`] keeps every table's chunk hashes between hashes and recomputes only what is
//! marked dirty: a whole table when its version changes (the stub tables' column versions, see
//! [`rh_core::world::WorldTables`]), or single chunks through [`MerkleCache::mark_rows`] for a
//! writer that knows which rows it changed. A change of row count re-hashes the chunks from the
//! first one that changed size. Versions are unique within one `WorldTables` instance only, so the
//! cache also records the instance's identity and re-hashes in full when it sees another one (a
//! clone, a fork, a loaded keyframe; VER-SPR-007 DEF-1). Neither identity nor versions ever enter
//! a hash. [`MerkleCache::verify`] recomputes everything and compares, for tests and debug checks.
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §6 item 6; ADR-0024; ADR-0006;
//! `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md` T-01, Architect ruling 1.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use rh_core::world::WorldTables;
use twox_hash::{XxHash3_64, XxHash3_128};

/// The layout's name and version.
pub const LAYOUT: &str = "rehoboam/rh-snapshot/hash/v1";

/// Rows per chunk (`R` in the layout).
pub const CHUNK_ROWS: usize = 1024;

const D_CHUNK: &[u8] = b"rehoboam/rh-snapshot/hash/v1/chunk";
const D_TABLE: &[u8] = b"rehoboam/rh-snapshot/hash/v1/table";
const D_PLANE: &[u8] = b"rehoboam/rh-snapshot/hash/v1/plane";

/// A state plane (ADR-0024).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Plane {
    /// Authoritative state: `state_hash`.
    A,
    /// Observation state: `observation_hash`.
    B,
    /// Narrative state: `narrative_hash`.
    C,
}

impl Plane {
    /// The plane's byte in the layout: its ASCII letter.
    #[must_use]
    pub const fn tag(self) -> u8 {
        match self {
            Self::A => b'A',
            Self::B => b'B',
            Self::C => b'C',
        }
    }
}

/// A table as the hash sees it: rows in key order, each with a canonical byte encoding.
pub trait HashRows {
    /// The number of rows.
    fn row_count(&self) -> usize;

    /// Appends row `row`'s canonical encoding to `out`. `row < row_count()`.
    fn encode_row(&self, row: usize, out: &mut Vec<u8>);
}

/// A stub `i64` column: row `i` is its `i`-th value, 8 little-endian bytes.
impl HashRows for [i64] {
    fn row_count(&self) -> usize {
        self.len()
    }

    fn encode_row(&self, row: usize, out: &mut Vec<u8>) {
        out.extend_from_slice(&self[row].to_le_bytes());
    }
}

/// Synthetic or encoded rows: row `i` is the `i`-th byte string.
impl HashRows for [Vec<u8>] {
    fn row_count(&self) -> usize {
        self.len()
    }

    fn encode_row(&self, row: usize, out: &mut Vec<u8>) {
        out.extend_from_slice(&self[row]);
    }
}

/// The hash of a byte string: XXH3-64, seed 0.
#[must_use]
pub fn xxh3(bytes: &[u8]) -> u64 {
    XxHash3_64::oneshot(bytes)
}

/// A row or name length. The layout limits both to `u32::MAX` bytes; nothing in the engine comes
/// near it (a longer one would saturate and could collide).
/// The 16-byte XXH3-128 of a byte string, seed 0, in its canonical form (high half first, each
/// half big-endian): the input journal's `payload_hash` (`05.9` IF-2).
#[must_use]
pub fn xxh3_128(bytes: &[u8]) -> [u8; 16] {
    XxHash3_128::oneshot(bytes).to_be_bytes()
}

fn put_u32(out: &mut Vec<u8>, x: usize) {
    let x = u32::try_from(x).unwrap_or(u32::MAX);
    out.extend_from_slice(&x.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, x: u64) {
    out.extend_from_slice(&x.to_le_bytes());
}

fn len64(x: usize) -> u64 {
    x as u64
}

/// The number of chunks of a table with `rows` rows.
#[must_use]
pub const fn chunk_count(rows: usize) -> usize {
    rows.div_ceil(CHUNK_ROWS)
}

/// The hash of chunk `chunk` of `table` (layout level 2). `buf` is scratch space.
#[must_use]
pub fn chunk_hash<T: HashRows + ?Sized>(table: &T, chunk: usize, buf: &mut Vec<u8>) -> u64 {
    let start = chunk * CHUNK_ROWS;
    let end = (start + CHUNK_ROWS).min(table.row_count());
    buf.clear();
    buf.extend_from_slice(D_CHUNK);
    put_u64(buf, len64(chunk));
    put_u64(buf, len64(end.saturating_sub(start)));
    let mut row = Vec::new();
    for i in start..end {
        row.clear();
        table.encode_row(i, &mut row);
        put_u32(buf, row.len());
        buf.extend_from_slice(&row);
    }
    xxh3(buf)
}

/// A table root from the table's name, row count and chunk hashes (layout level 3).
#[must_use]
pub fn table_root(name: &str, rows: usize, chunks: &[u64]) -> u64 {
    let mut buf = Vec::with_capacity(D_TABLE.len() + 20 + name.len() + 8 * chunks.len());
    buf.extend_from_slice(D_TABLE);
    put_u32(&mut buf, name.len());
    buf.extend_from_slice(name.as_bytes());
    put_u64(&mut buf, len64(rows));
    put_u64(&mut buf, len64(chunks.len()));
    for &c in chunks {
        put_u64(&mut buf, c);
    }
    xxh3(&buf)
}

/// A table's Merkle digest: its row count, chunk hashes and root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableDigest {
    /// The number of rows.
    pub rows: usize,
    /// The chunk hashes, in chunk order.
    pub chunks: Vec<u64>,
    /// The table root.
    pub root: u64,
}

impl TableDigest {
    /// The digest of the table `name`, every chunk hashed.
    #[must_use]
    pub fn of<T: HashRows + ?Sized>(name: &str, table: &T) -> Self {
        let rows = table.row_count();
        let mut buf = Vec::new();
        let chunks: Vec<u64> = (0..chunk_count(rows))
            .map(|c| chunk_hash(table, c, &mut buf))
            .collect();
        let root = table_root(name, rows, &chunks);
        Self { rows, chunks, root }
    }
}

/// A plane hash could not be computed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HashError {
    /// Table names were not in strictly increasing byte order (unsorted or duplicated).
    TableOrder {
        /// The name before.
        previous: String,
        /// The name out of order.
        name: String,
    },
}

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TableOrder { previous, name } => write!(
                f,
                "plane hash: table {name:?} does not follow {previous:?} in strictly increasing byte order"
            ),
        }
    }
}

impl std::error::Error for HashError {}

/// A plane hash from `(table name, table root)` pairs (layout level 4).
///
/// # Errors
/// [`HashError::TableOrder`] unless the names are in strictly increasing byte order.
pub fn plane_hash<'a>(
    plane: Plane,
    tables: impl IntoIterator<Item = (&'a str, u64)>,
) -> Result<u64, HashError> {
    let tables: Vec<(&str, u64)> = tables.into_iter().collect();
    for pair in tables.windows(2) {
        let (previous, name) = (pair[0].0, pair[1].0);
        if previous.as_bytes() >= name.as_bytes() {
            return Err(HashError::TableOrder {
                previous: previous.to_owned(),
                name: name.to_owned(),
            });
        }
    }
    Ok(encode_plane(plane, tables))
}

/// Layout level 4 over pairs already in strictly increasing name order.
fn encode_plane<'a>(plane: Plane, tables: impl IntoIterator<Item = (&'a str, u64)>) -> u64 {
    let mut body = Vec::new();
    let mut count = 0usize;
    for (name, root) in tables {
        count += 1;
        put_u32(&mut body, name.len());
        body.extend_from_slice(name.as_bytes());
        put_u64(&mut body, root);
    }
    let mut buf = Vec::with_capacity(D_PLANE.len() + 9 + body.len());
    buf.extend_from_slice(D_PLANE);
    buf.push(plane.tag());
    put_u64(&mut buf, len64(count));
    buf.extend_from_slice(&body);
    xxh3(&buf)
}

/// The hash of a plane with no tables (planes B and C until they gain tables).
#[must_use]
pub fn empty_plane_hash(plane: Plane) -> u64 {
    encode_plane(plane, [])
}

/// `state_hash`: the plane-A hash of the world tables, every chunk hashed.
#[must_use]
pub fn state_hash(tables: &WorldTables) -> u64 {
    PlaneDigest::of_world(tables).hash
}

/// A plane's full Merkle digest: every table's digest and the plane hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaneDigest {
    /// The plane.
    pub plane: Plane,
    /// Every table's digest, by name (byte order equals `String` order).
    pub tables: BTreeMap<String, TableDigest>,
    /// The plane hash.
    pub hash: u64,
}

impl PlaneDigest {
    /// The digest of `tables`, given as `(name, digest)`; names must be distinct.
    #[must_use]
    pub fn from_tables(plane: Plane, tables: BTreeMap<String, TableDigest>) -> Self {
        let hash = plane_hash_of(plane, &tables);
        Self {
            plane,
            tables,
            hash,
        }
    }

    /// The plane-A digest of the world tables.
    #[must_use]
    pub fn of_world(world: &WorldTables) -> Self {
        let tables = world
            .columns()
            .map(|(name, _, rows)| (name.to_owned(), TableDigest::of(name, rows)))
            .collect();
        Self::from_tables(Plane::A, tables)
    }

    /// The first place where `self` and `other` differ, in table-name order, or `None` if the
    /// plane hashes agree.
    #[must_use]
    pub fn first_divergence(&self, other: &Self) -> Option<Divergence> {
        if self.hash == other.hash && self.plane == other.plane {
            return None;
        }
        let names: BTreeSet<&String> = self.tables.keys().chain(other.tables.keys()).collect();
        for name in names {
            match (self.tables.get(name), other.tables.get(name)) {
                (Some(a), Some(b)) if a.root == b.root => {}
                (Some(a), Some(b)) => {
                    let chunk = a
                        .chunks
                        .iter()
                        .zip(&b.chunks)
                        .position(|(x, y)| x != y)
                        .unwrap_or_else(|| a.chunks.len().min(b.chunks.len()));
                    return Some(Divergence::Chunk {
                        table: name.clone(),
                        chunk,
                        first_row: chunk * CHUNK_ROWS,
                    });
                }
                _ => {
                    return Some(Divergence::Table {
                        table: name.clone(),
                    });
                }
            }
        }
        Some(Divergence::Plane)
    }
}

fn plane_hash_of(plane: Plane, tables: &BTreeMap<String, TableDigest>) -> u64 {
    // `String`'s order is byte order and map keys are distinct, so the order rule holds.
    encode_plane(
        plane,
        tables.iter().map(|(name, d)| (name.as_str(), d.root)),
    )
}

/// Where two plane digests first differ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Divergence {
    /// A table present on one side only.
    Table {
        /// The table.
        table: String,
    },
    /// A chunk of a table present on both sides (or the first chunk past the shorter side).
    Chunk {
        /// The table.
        table: String,
        /// The first differing chunk.
        chunk: usize,
        /// Its first row.
        first_row: usize,
    },
    /// Equal tables under different planes.
    Plane,
}

/// One table's cached digest and dirty marks.
#[derive(Clone, Debug)]
struct Cached {
    version: Option<u64>,
    digest: TableDigest,
    dirty: BTreeSet<usize>,
}

/// A plane's Merkle hashes kept between hashes, recomputing only dirty chunks.
#[derive(Clone, Debug)]
pub struct MerkleCache {
    plane: Plane,
    tables: BTreeMap<String, Cached>,
    rehashed_chunks: u64,
    /// The identity of the world tables last hashed by [`MerkleCache::update_world`].
    world: Option<u64>,
}

impl MerkleCache {
    /// An empty cache for `plane`.
    #[must_use]
    pub fn new(plane: Plane) -> Self {
        Self {
            plane,
            tables: BTreeMap::new(),
            rehashed_chunks: 0,
            world: None,
        }
    }

    /// Marks the chunks holding `rows` of `table` dirty, for the next [`update`](Self::update).
    ///
    /// The range is clamped to the rows the cache holds for the table: rows past them are new,
    /// and [`update`](Self::update) hashes them when it sees the larger row count. An empty or
    /// fully out-of-range span marks nothing (VER-SPR-007 DEF-2: `0..usize::MAX` once inserted
    /// ~1.8e16 chunk indices).
    pub fn mark_rows(&mut self, table: &str, rows: std::ops::Range<usize>) {
        let Some(c) = self.tables.get_mut(table) else {
            return;
        };
        let end = rows.end.min(c.digest.rows);
        if rows.start < end {
            c.dirty
                .extend((rows.start / CHUNK_ROWS)..=((end - 1) / CHUNK_ROWS));
        }
    }

    /// Brings one table's digest up to date and returns its root.
    ///
    /// `version` is the table's version when it has one (`None` means the caller tracks rows
    /// with [`mark_rows`](Self::mark_rows) alone): a new version re-hashes every chunk. A changed
    /// row count re-hashes from the first chunk whose size changed. Otherwise only the chunks
    /// marked dirty are re-hashed.
    pub fn update<T: HashRows + ?Sized>(
        &mut self,
        name: &str,
        version: Option<u64>,
        table: &T,
    ) -> u64 {
        let rows = table.row_count();
        let mut buf = Vec::new();
        let Some(c) = self.tables.get_mut(name) else {
            let digest = TableDigest::of(name, table);
            self.rehashed_chunks += len64(digest.chunks.len());
            let root = digest.root;
            self.tables.insert(
                name.to_owned(),
                Cached {
                    version,
                    digest,
                    dirty: BTreeSet::new(),
                },
            );
            return root;
        };
        let n = chunk_count(rows);
        if c.version != version {
            c.dirty.extend(0..n);
            c.version = version;
        }
        let resized = c.digest.rows != rows;
        if resized {
            // Chunks before the old last chunk keep their size; the rest change or appear.
            let first_changed = c.digest.rows.min(rows) / CHUNK_ROWS;
            c.dirty.extend(first_changed..n);
            c.digest.chunks.resize(n, 0);
            c.digest.rows = rows;
        }
        let dirty = std::mem::take(&mut c.dirty);
        let mut touched = false;
        for k in dirty.into_iter().filter(|&k| k < n) {
            c.digest.chunks[k] = chunk_hash(table, k, &mut buf);
            self.rehashed_chunks += 1;
            touched = true;
        }
        if touched || resized {
            c.digest.root = table_root(name, rows, &c.digest.chunks);
        }
        c.digest.root
    }

    /// Drops every table not in `keep` (tables removed from the plane).
    pub fn retain(&mut self, keep: &BTreeSet<&str>) {
        self.tables.retain(|name, _| keep.contains(name.as_str()));
    }

    /// Brings every column of the world tables up to date, drops removed ones, and returns
    /// `state_hash`. Only plane-A caches may hash the world.
    ///
    /// Column versions are unique within one `WorldTables` instance only, so tables of another
    /// identity (a clone, a fork, a loaded keyframe) are re-hashed in full (VER-SPR-007 DEF-1).
    pub fn update_world(&mut self, world: &WorldTables) -> u64 {
        if self.world != Some(world.identity()) {
            self.tables.clear();
            self.world = Some(world.identity());
        }
        let mut names = BTreeSet::new();
        for (name, version, rows) in world.columns() {
            self.update(name, Some(version), rows);
            names.insert(name);
        }
        self.retain(&names);
        self.hash()
    }

    /// The plane hash over the cached tables (call [`update`](Self::update) first).
    #[must_use]
    pub fn hash(&self) -> u64 {
        self.digest().hash
    }

    /// The plane digest over the cached tables.
    #[must_use]
    pub fn digest(&self) -> PlaneDigest {
        PlaneDigest::from_tables(
            self.plane,
            self.tables
                .iter()
                .map(|(k, c)| (k.clone(), c.digest.clone()))
                .collect(),
        )
    }

    /// Chunks hashed since the cache was made (for dirty-tracking tests and metrics).
    #[must_use]
    pub fn rehashed_chunks(&self) -> u64 {
        self.rehashed_chunks
    }

    /// Whether the cached digest of every world column equals a full recomputation.
    #[must_use]
    pub fn verify(&self, world: &WorldTables) -> bool {
        self.digest() == PlaneDigest::of_world(world)
    }
}
