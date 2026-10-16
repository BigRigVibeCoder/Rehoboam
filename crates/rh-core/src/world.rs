//! The world's tables and the per-system step view (SPR-005; stub storage).
//!
//! [`WorldTables`] is the plane-A store: named columns of `i64` rows in a `BTreeMap`, so every
//! iteration is in column-name order. It is the minimal store the scheduler and the V-DET-01/02
//! stub world need; the typed tables of `05.1` §5 (ledger, life-course log, pop aggregates)
//! replace the row type later without changing the access rules.
//!
//! [`SystemView`] is what a system's `step` sees. It enforces the system's [`AccessSpec`] at run
//! time:
//! - a write is allowed only to a declared written (owned) column;
//! - a read is allowed only from a declared read or an owned column;
//! - a lagged read (`start_of_day`, `previous_pulse`, `lagged`) sees the start-of-instant value,
//!   a `same_instant` read sees the current value.
//!
//! The written columns are moved out of the tables into the view for the step and moved back
//! afterwards. That is what lets systems of one scheduler level run on several threads with no
//! shared mutable state (T-05): each owns its columns, and reads of anyone else's are immutable.
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §2 rules 1 and 4, §5, §6 item 5;
//! `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-04, T-05.

use std::collections::BTreeMap;

use crate::access::AccessSpec;
use crate::system::SimError;

/// One column: rows of `i64` (stub storage; see the module documentation).
pub type Column = Vec<i64>;

/// Domain tag of the stub state hash.
const TAG_STATE_HASH: u64 = rh_ids::rng::tag("rehoboam/rh-core/state-hash/stub/v1");

/// The plane-A tables: columns by `entity.column` name, iterated in name order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldTables {
    columns: BTreeMap<String, Column>,
}

impl WorldTables {
    /// Empty tables.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces a column (world construction, before the first instant).
    pub fn insert(&mut self, column: &str, rows: Column) {
        self.columns.insert(column.to_owned(), rows);
    }

    /// A column, if present.
    #[must_use]
    pub fn get(&self, column: &str) -> Option<&Column> {
        self.columns.get(column)
    }

    /// Column names, in order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.columns.keys().map(String::as_str)
    }

    /// Removes a column, returning it (empty if absent).
    pub(crate) fn take(&mut self, column: &str) -> Column {
        self.columns.remove(column).unwrap_or_default()
    }

    /// A stub state hash: `SplitMix64` words over every column in name order (name bytes, row
    /// count, rows). Order-sensitive and total. The chunked Merkle xxh3 of `05.1` §6 item 6
    /// replaces it with the keyframe work (`rh-snapshot`).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let mut h = rh_ids::rng::h64_words(TAG_STATE_HASH, &[]);
        for (name, rows) in &self.columns {
            let name_hash = rh_ids::rng::h64_bytes(TAG_STATE_HASH, name.as_bytes());
            h = rh_ids::rng::h64_words(h, &[name_hash, rows.len() as u64]);
            for chunk in rows.chunks(64) {
                let words: Vec<u64> = chunk.iter().map(|v| v.cast_unsigned()).collect();
                h = rh_ids::rng::h64_words(h, &words);
            }
        }
        h
    }
}

/// What one system's `step` sees: its owned columns (mutable) and the rest (read-only).
pub struct SystemView<'a> {
    system: &'static str,
    access: &'a AccessSpec,
    owned: BTreeMap<&'static str, Column>,
    current: &'a WorldTables,
    start_of_instant: &'a WorldTables,
}

impl<'a> SystemView<'a> {
    /// A view for `system`, which owns `owned` (moved out of the tables by the caller).
    pub(crate) fn new(
        system: &'static str,
        access: &'a AccessSpec,
        owned: BTreeMap<&'static str, Column>,
        current: &'a WorldTables,
        start_of_instant: &'a WorldTables,
    ) -> Self {
        Self {
            system,
            access,
            owned,
            current,
            start_of_instant,
        }
    }

    /// Returns the owned columns to the caller.
    pub(crate) fn into_owned(self) -> BTreeMap<&'static str, Column> {
        self.owned
    }

    /// Reads a column the system declared it reads, or one it owns.
    ///
    /// # Errors
    /// [`SimError::UndeclaredRead`] for any other column; [`SimError::MissingColumn`] if a
    /// declared column does not exist in the world.
    pub fn read(&self, column: &str) -> Result<&[i64], SimError> {
        if let Some(rows) = self.owned.get(column) {
            return Ok(rows);
        }
        let Some(read) = self.access.read_of(column) else {
            return Err(SimError::UndeclaredRead {
                system: self.system,
                column: column.to_owned(),
            });
        };
        let tables = if read.lag.is_lagged() {
            self.start_of_instant
        } else {
            self.current
        };
        tables
            .get(column)
            .map(Vec::as_slice)
            .ok_or_else(|| SimError::MissingColumn {
                system: self.system,
                column: column.to_owned(),
            })
    }

    /// Mutable access to a column the system owns.
    ///
    /// # Errors
    /// [`SimError::UnownedWrite`] for any column the system does not declare as a write.
    pub fn write(&mut self, column: &str) -> Result<&mut Column, SimError> {
        self.owned
            .get_mut(column)
            .ok_or_else(|| SimError::UnownedWrite {
                system: self.system,
                column: column.to_owned(),
            })
    }
}
