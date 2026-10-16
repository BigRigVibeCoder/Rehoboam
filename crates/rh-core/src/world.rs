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
use std::sync::atomic::{AtomicU64, Ordering};

use crate::access::AccessSpec;
use crate::system::SimError;

/// One column: rows of `i64` (stub storage; see the module documentation).
pub type Column = Vec<i64>;

/// The plane-A tables: columns by `entity.column` name, iterated in name order.
///
/// Each column carries a **version**: a number from one counter of the tables, set afresh on
/// every [`insert`](Self::insert), the only way a column enters or returns to the tables (a
/// system's written columns come back through it after every step). Equal versions mean the
/// column has not been replaced since; `rh-snapshot`'s Merkle cache uses that as its column-level
/// dirty mark (SPR-007 T-01).
///
/// Versions are unique within one instance only: two clones that each replace a column can give
/// both the same version. So each instance also carries an **identity**, a number from one
/// process-wide counter, fresh at construction and at every clone; a cache that sees another
/// identity recomputes everything (VER-SPR-007 DEF-1).
///
/// Identity and versions are bookkeeping, not state. They never enter a hash, an equality or any
/// output: equality compares the columns only, and the plane hash never reads either. Their values
/// depend on how many tables the process has built, and nothing may depend on them but caching.
#[derive(Debug)]
pub struct WorldTables {
    columns: BTreeMap<String, (u64, Column)>,
    next_version: u64,
    identity: u64,
}

/// The process-wide source of [`WorldTables`] identities (bookkeeping only; see the type docs).
static NEXT_IDENTITY: AtomicU64 = AtomicU64::new(0);

fn fresh_identity() -> u64 {
    NEXT_IDENTITY.fetch_add(1, Ordering::Relaxed)
}

impl Default for WorldTables {
    fn default() -> Self {
        Self {
            columns: BTreeMap::new(),
            next_version: 0,
            identity: fresh_identity(),
        }
    }
}

/// A clone is another instance: it gets a fresh identity, so a cache never mistakes it for the
/// original after either changes.
impl Clone for WorldTables {
    fn clone(&self) -> Self {
        Self {
            columns: self.columns.clone(),
            next_version: self.next_version,
            identity: fresh_identity(),
        }
    }
}

impl PartialEq for WorldTables {
    fn eq(&self, other: &Self) -> bool {
        self.columns.len() == other.columns.len()
            && self
                .columns
                .iter()
                .zip(&other.columns)
                .all(|((a, (_, ra)), (b, (_, rb)))| a == b && ra == rb)
    }
}

impl Eq for WorldTables {}

impl WorldTables {
    /// Empty tables.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces a column (world construction, and the return of a system's written
    /// columns after its step). The column gets a new version.
    pub fn insert(&mut self, column: &str, rows: Column) {
        let version = self.next_version;
        self.next_version += 1;
        self.columns.insert(column.to_owned(), (version, rows));
    }

    /// A column, if present.
    #[must_use]
    pub fn get(&self, column: &str) -> Option<&Column> {
        self.columns.get(column).map(|(_, rows)| rows)
    }

    /// This instance's identity (bookkeeping for caches only; see the type documentation).
    #[must_use]
    pub fn identity(&self) -> u64 {
        self.identity
    }

    /// A column's version, if present (see the type documentation).
    #[must_use]
    pub fn version(&self, column: &str) -> Option<u64> {
        self.columns.get(column).map(|(v, _)| *v)
    }

    /// Column names, in order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.columns.keys().map(String::as_str)
    }

    /// Every column with its version, in name order.
    pub fn columns(&self) -> impl Iterator<Item = (&str, u64, &[i64])> {
        self.columns
            .iter()
            .map(|(name, (v, rows))| (name.as_str(), *v, rows.as_slice()))
    }

    /// Removes a column, returning it, or `None` if the world does not have it. Never creates
    /// one: columns are materialised only when the engine is built, from the owners' declared
    /// writes (SPR-005 G-c).
    pub(crate) fn take(&mut self, column: &str) -> Option<Column> {
        self.columns.remove(column).map(|(_, rows)| rows)
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
