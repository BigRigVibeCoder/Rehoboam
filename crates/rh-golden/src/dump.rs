//! A whole plane-A state as text, so two builds' states can be compared table by table, chunk by
//! chunk and row by row (the bisect's last step, SPR-008 ruling 7).
//!
//! # Format `rehoboam-dump v1`
//!
//! ```text
//! rehoboam-dump v1
//! tick 1051199 date 0001-12
//! table person.alive i64 64       <- a world table: name, kind, row count; then one row per line
//! 1
//! 0
//! ...
//! table rh-core/agenda.meta bytes 1   <- an engine table: each row its encoded bytes, as hex
//! 0a00...
//! ```
//!
//! Tables come in name byte order: the world tables and the engine tables `state_hash` covers
//! (`rh_snapshot::state::CARRIED_TABLES` excepted), as `rh_snapshot::state::state_digest` takes
//! them, so a dump's digest equals the ladder's.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use rh_core::state::EngineState;
use rh_core::time::SimTime;
use rh_snapshot::hash::{Plane, PlaneDigest, TableDigest};
use rh_snapshot::state::{CARRIED_TABLES, engine_tables};

use crate::run::{Error, date};

/// The first line of a dump.
pub const MAGIC: &str = "rehoboam-dump v1";

/// One table's rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rows {
    /// A world table: one `i64` per row.
    Ints(Vec<i64>),
    /// An engine table: each row's encoded bytes.
    Bytes(Vec<Vec<u8>>),
}

impl Rows {
    /// The row count.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Ints(r) => r.len(),
            Self::Bytes(r) => r.len(),
        }
    }

    /// Whether there are no rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn digest(&self, name: &str) -> TableDigest {
        match self {
            Self::Ints(r) => TableDigest::of(name, r.as_slice()),
            Self::Bytes(r) => TableDigest::of(name, r.as_slice()),
        }
    }
}

/// A parsed dump.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dump {
    /// The instant.
    pub tick: i64,
    /// Its date.
    pub date: String,
    /// Every table, by name.
    pub tables: BTreeMap<String, Rows>,
}

impl Dump {
    /// The dump of `state` at `t`.
    #[must_use]
    pub fn of(state: &EngineState, t: SimTime) -> Self {
        let mut tables: BTreeMap<String, Rows> = state
            .tables
            .columns()
            .map(|(name, _, rows)| (name.to_owned(), Rows::Ints(rows.to_vec())))
            .collect();
        for (name, rows) in engine_tables(state) {
            if !CARRIED_TABLES.contains(&name) {
                tables.insert(name.to_owned(), Rows::Bytes(rows));
            }
        }
        Self {
            tick: t.minutes(),
            date: date(t),
            tables,
        }
    }

    /// The plane-A digest of the dumped tables: equal to `state_digest` of the state dumped.
    #[must_use]
    pub fn digest(&self) -> PlaneDigest {
        PlaneDigest::from_tables(
            Plane::A,
            self.tables
                .iter()
                .map(|(name, rows)| (name.clone(), rows.digest(name)))
                .collect(),
        )
    }

    /// The dump as its file text.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = format!("{MAGIC}\ntick {} date {}\n", self.tick, self.date);
        for (name, rows) in &self.tables {
            match rows {
                Rows::Ints(r) => {
                    let _ = writeln!(out, "table {name} i64 {}", r.len());
                    for v in r {
                        let _ = writeln!(out, "{v}");
                    }
                }
                Rows::Bytes(r) => {
                    let _ = writeln!(out, "table {name} bytes {}", r.len());
                    for row in r {
                        out.push_str(&hex(row));
                        out.push('\n');
                    }
                }
            }
        }
        out
    }

    /// Parses a dump.
    ///
    /// # Errors
    /// [`Error`] naming the first line that does not follow the format.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let bad = |n: usize, what: &str| Error(format!("dump line {n}: {what}"));
        let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l));
        if lines.next().map(|(_, l)| l) != Some(MAGIC) {
            return Err(bad(1, "expected the magic line"));
        }
        let (n, head) = lines.next().ok_or_else(|| bad(2, "no tick line"))?;
        let (tick, date) = match head.split(' ').collect::<Vec<_>>().as_slice() {
            ["tick", tick, "date", date] => (
                tick.parse().map_err(|_| bad(n, "bad tick"))?,
                (*date).to_owned(),
            ),
            _ => return Err(bad(n, "expected `tick <t> date <d>`")),
        };
        let mut tables = BTreeMap::new();
        while let Some((n, line)) = lines.next() {
            let (name, kind, count) = match line.split(' ').collect::<Vec<_>>().as_slice() {
                ["table", name, kind, count] => (
                    (*name).to_owned(),
                    (*kind).to_owned(),
                    count
                        .parse::<usize>()
                        .map_err(|_| bad(n, "bad row count"))?,
                ),
                _ => return Err(bad(n, "expected `table <name> <kind> <rows>`")),
            };
            let mut rows: Vec<&str> = Vec::with_capacity(count);
            for _ in 0..count {
                rows.push(
                    lines
                        .next()
                        .ok_or_else(|| bad(n, "the table ends early"))?
                        .1,
                );
            }
            let rows = match kind.as_str() {
                "i64" => Rows::Ints(
                    rows.iter()
                        .map(|r| r.parse().map_err(|_| bad(n, "a row is not an i64")))
                        .collect::<Result<_, _>>()?,
                ),
                "bytes" => Rows::Bytes(
                    rows.iter()
                        .map(|r| unhex(r).ok_or_else(|| bad(n, "a row is not hex")))
                        .collect::<Result<_, _>>()?,
                ),
                _ => return Err(bad(n, "a table is i64 or bytes")),
            };
            tables.insert(name, rows);
        }
        Ok(Self { tick, date, tables })
    }
}

/// Writes the dump of `state` at `t` to `path`.
///
/// # Errors
/// [`Error`] if the file cannot be written.
pub fn write(state: &EngineState, t: SimTime, path: &Path) -> Result<(), Error> {
    std::fs::write(path, Dump::of(state, t).render())
        .map_err(|e| Error(format!("dump {}: {e}", path.display())))
}

/// `bytes` as lowercase hex.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}
