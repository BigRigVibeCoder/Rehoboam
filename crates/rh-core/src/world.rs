//! The world's tables and the per-system step view (SPR-005; stub storage).
//!
//! [`WorldTables`] is the plane-A store: named columns of `i64` rows in name-sorted slots, so every
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
/// every [`insert`](Self::insert), the only public way a column enters or returns to the tables (a
/// system's written columns come back by their slot after every step, which also takes a new
/// version). Equal versions mean the column has not been replaced since; `rh-snapshot`'s Merkle
/// cache uses that as its column-level dirty mark (SPR-007 T-01).
///
/// Versions are unique within one instance only: two clones that each replace a column can give
/// both the same version. So each instance also carries an **identity**, a number from one
/// process-wide counter, fresh at construction and at every clone; a cache that sees another
/// identity recomputes everything (VER-SPR-007 DEF-1).
///
/// Identity and versions are bookkeeping, not state. They never enter a hash, an equality or any
/// output: equality compares the columns only, and the plane hash never reads either. Their values
/// depend on how many tables the process has built, and nothing may depend on them but caching.
///
/// **Storage** (SPR-038 T-02): the columns are slots sorted by name, each `{name, version,
/// rows}`. The engine resolves a [`ColumnId`] (a slot's index) for every column it moves when it
/// is built, after which no column is added, so its moves are O(1) and allocate nothing. A column
/// moved out to a step is absent until it is put back, exactly as when the tables were a map:
/// [`get`](Self::get), [`names`](Self::names) and [`columns`](Self::columns) do not see it.
///
/// **Write marks** (SPR-038 T-03): each slot also carries the stamp of the last write to it, from
/// a counter of the tables: every [`insert`](Self::insert), every put-back of a column its step
/// wrote, and every core write (compaction, the row insert). Every path that puts rows into a slot
/// says whether it wrote them, so no write goes unmarked. The engine runs a check, the compaction or
/// a lagged copy only when a column it covers carries a mark newer than its last run. Marks are
/// bookkeeping like versions: never in a hash, an equality or any output.
///
/// DECISION (SPR-038 T-03; deviation 4, accepted by the Architect): the plan names "one engine
/// bitset per since-consumer". A write stamp per slot is the same relation: a consumer that last
/// ran at stamp `s` covers columns `C`, and its bitset would hold `{c in C : c written since s}`,
/// which is non-empty exactly when `max(mark(c) for c in C) > s`, because marks only grow and every
/// write takes a fresh one. One counter serves every consumer, so a consumer added later (SPR-015's
/// derived index) needs only its own `s`, and nothing is cleared or copied per instant. A skipped
/// binding writes nothing, so it moves no mark; a restored engine starts every consumer at 0 and
/// every column at a mark of at least 1 (each `insert`), so each runs at its first chance.
#[derive(Debug)]
pub struct WorldTables {
    slots: Vec<Slot>,
    next_version: u64,
    /// The last write mark given (0: none yet).
    stamp: u64,
    identity: u64,
}

/// One column's slot: its name, its version, its write mark, and its rows unless they are moved
/// out.
#[derive(Clone, Debug)]
struct Slot {
    name: String,
    version: u64,
    mark: u64,
    rows: Option<Column>,
}

/// A column's slot in one [`WorldTables`]: valid while no column is added to it, which after
/// `Engine::new` is always (SPR-038 T-02).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ColumnId(usize);

/// The process-wide source of [`WorldTables`] identities (bookkeeping only; see the type docs).
static NEXT_IDENTITY: AtomicU64 = AtomicU64::new(0);

fn fresh_identity() -> u64 {
    NEXT_IDENTITY.fetch_add(1, Ordering::Relaxed)
}

impl Default for WorldTables {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            next_version: 0,
            stamp: 0,
            identity: fresh_identity(),
        }
    }
}

/// A clone is another instance: it gets a fresh identity, so a cache never mistakes it for the
/// original after either changes.
impl Clone for WorldTables {
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            next_version: self.next_version,
            stamp: self.stamp,
            identity: fresh_identity(),
        }
    }
}

impl PartialEq for WorldTables {
    fn eq(&self, other: &Self) -> bool {
        let mut a = self.present();
        let mut b = other.present();
        loop {
            match (a.next(), b.next()) {
                (None, None) => return true,
                (Some(x), Some(y)) if x.0 == y.0 && x.1 == y.1 => {}
                _ => return false,
            }
        }
    }
}

impl Eq for WorldTables {}

impl WorldTables {
    /// Empty tables.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The present columns, `(name, rows, version)`, in name order.
    fn present(&self) -> impl Iterator<Item = (&str, &Column, u64)> {
        self.slots
            .iter()
            .filter_map(|s| s.rows.as_ref().map(|r| (s.name.as_str(), r, s.version)))
    }

    fn find(&self, column: &str) -> Result<usize, usize> {
        self.slots.binary_search_by(|s| s.name.as_str().cmp(column))
    }

    fn fresh_version(&mut self) -> u64 {
        let version = self.next_version;
        self.next_version += 1;
        version
    }

    fn fresh_mark(&mut self) -> u64 {
        self.stamp += 1;
        self.stamp
    }

    /// Adds or replaces a column (world construction, and the return of a system's written
    /// columns after its step). The column gets a new version.
    pub fn insert(&mut self, column: &str, rows: Column) {
        let version = self.fresh_version();
        let mark = self.fresh_mark();
        match self.find(column) {
            Ok(i) => {
                let slot = &mut self.slots[i];
                slot.version = version;
                slot.mark = mark;
                slot.rows = Some(rows);
            }
            Err(i) => self.slots.insert(
                i,
                Slot {
                    name: column.to_owned(),
                    version,
                    mark,
                    rows: Some(rows),
                },
            ),
        }
    }

    /// A column, if present.
    #[must_use]
    pub fn get(&self, column: &str) -> Option<&Column> {
        self.find(column)
            .ok()
            .and_then(|i| self.slots[i].rows.as_ref())
    }

    /// This instance's identity (bookkeeping for caches only; see the type documentation).
    #[must_use]
    pub fn identity(&self) -> u64 {
        self.identity
    }

    /// A column's version, if present (see the type documentation).
    #[must_use]
    pub fn version(&self, column: &str) -> Option<u64> {
        self.find(column)
            .ok()
            .map(|i| &self.slots[i])
            .and_then(|s| s.rows.as_ref().map(|_| s.version))
    }

    /// Column names, in order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.present().map(|(name, _, _)| name)
    }

    /// Every column with its version, in name order.
    pub fn columns(&self) -> impl Iterator<Item = (&str, u64, &[i64])> {
        self.present()
            .map(|(name, rows, v)| (name, v, rows.as_slice()))
    }

    /// The slot of `column`, present or moved out; `None` if the world does not have it. The
    /// engine resolves its ids once, when it is built (SPR-038 T-02).
    pub(crate) fn id(&self, column: &str) -> Option<ColumnId> {
        self.find(column).ok().map(ColumnId)
    }

    /// The column in slot `id`, if present (not moved out).
    pub(crate) fn get_at(&self, id: ColumnId) -> Option<&Column> {
        self.slots.get(id.0).and_then(|s| s.rows.as_ref())
    }

    /// Moves the column in slot `id` out, leaving it absent until [`put_at`](Self::put_at); `None`
    /// if it is already out. Never creates a column (SPR-005 G-c). O(1); allocates nothing.
    pub(crate) fn take_at(&mut self, id: ColumnId) -> Option<Column> {
        self.slots.get_mut(id.0).and_then(|s| s.rows.take())
    }

    /// Puts a column back into slot `id`, with a new version (every put-back takes one: SPR-038
    /// stage A) and, when `written`, a new write mark (T-03). O(1); allocates nothing.
    pub(crate) fn put_at(&mut self, id: ColumnId, rows: Column, written: bool) {
        let version = self.fresh_version();
        let mark = if written {
            Some(self.fresh_mark())
        } else {
            None
        };
        if let Some(slot) = self.slots.get_mut(id.0) {
            slot.version = version;
            slot.mark = mark.unwrap_or(slot.mark);
            slot.rows = Some(rows);
        }
    }

    /// Puts a written column back into slot `id` keeping the version it had: only the planted
    /// engine of `mutant-compact-keeps-version` does this (SPR-038 S11), and the per-instant cache
    /// test must then FAIL.
    #[cfg(feature = "mutant-compact-keeps-version")]
    pub(crate) fn put_at_keeping_version(&mut self, id: ColumnId, rows: Column) {
        let mark = self.fresh_mark();
        if let Some(slot) = self.slots.get_mut(id.0) {
            slot.mark = mark;
            slot.rows = Some(rows);
        }
    }

    /// The write mark of slot `id` (0 for none; see the type documentation).
    pub(crate) fn mark_at(&self, id: ColumnId) -> u64 {
        self.slots.get(id.0).map_or(0, |s| s.mark)
    }

    /// The newest write mark of any slot.
    pub(crate) const fn stamp(&self) -> u64 {
        self.stamp
    }

    /// Copies `rows` into slot `id`, reusing the slot's allocation (the engine's start-of-instant
    /// copy of a lagged column). The slot takes a new version.
    pub(crate) fn copy_at(&mut self, id: ColumnId, rows: &[i64]) {
        let version = self.fresh_version();
        if let Some(slot) = self.slots.get_mut(id.0) {
            slot.version = version;
            match &mut slot.rows {
                Some(dst) => {
                    dst.clear();
                    dst.extend_from_slice(rows);
                }
                None => slot.rows = Some(rows.to_vec()),
            }
        }
    }
}

/// One column a step owns, moved out of its slot for the step (SPR-038 T-02).
#[derive(Debug)]
pub(crate) struct OwnedColumn {
    pub(crate) name: &'static str,
    pub(crate) id: ColumnId,
    pub(crate) rows: Column,
    /// Its row count when it was moved out.
    pub(crate) rows_before: usize,
    /// Whether the step took it with [`SystemView::write`] (T-03): the put-back marks it.
    pub(crate) written: bool,
    /// Whether the ledger guard keeps its pre-image (`person.id`, every `life_course.*`).
    pub(crate) guarded: bool,
    /// Its rows as they stood before the step, copied at the step's first `write()` of a guarded
    /// column (SPR-038 S12): every write is value-checked, and a job that writes no guarded column
    /// copies nothing.
    pub(crate) pre: Option<Column>,
}

/// What one system's `step` sees: its owned columns (mutable) and the rest (read-only).
pub struct SystemView<'a> {
    system: &'static str,
    access: &'a AccessSpec,
    /// The owned columns, in write order: a buffer the engine reuses (SPR-038 T-02).
    owned: Vec<OwnedColumn>,
    current: &'a WorldTables,
    start_of_instant: &'a WorldTables,
    /// The rate and mode reads registration dropped (their owner is absent), with their neutrals
    /// (`05.1` §2 rule 2): each enumerates as an empty column, and [`SystemView::value`] returns
    /// its neutral for any row.
    dropped: &'a BTreeMap<&'static str, i64>,
}

/// No dropped reads: the default for views built outside an engine level.
static NO_DROPPED: BTreeMap<&'static str, i64> = BTreeMap::new();

impl<'a> SystemView<'a> {
    /// A view for `system`, which owns `owned` (moved out of the tables by the caller).
    pub(crate) fn new(
        system: &'static str,
        access: &'a AccessSpec,
        owned: Vec<OwnedColumn>,
        current: &'a WorldTables,
        start_of_instant: &'a WorldTables,
    ) -> Self {
        Self {
            system,
            access,
            owned,
            current,
            start_of_instant,
            dropped: &NO_DROPPED,
        }
    }

    /// The view with the system's dropped reads (absent owners, `05.1` §2 rule 2).
    pub(crate) fn with_dropped(mut self, dropped: &'a BTreeMap<&'static str, i64>) -> Self {
        self.dropped = dropped;
        self
    }

    /// Returns the owned columns to the caller.
    pub(crate) fn into_owned(self) -> Vec<OwnedColumn> {
        self.owned
    }

    /// The owned column `column`, if the system owns it: a linear scan of its writes.
    fn owned(&self, column: &str) -> Option<&OwnedColumn> {
        self.owned.iter().find(|o| o.name == column)
    }

    /// Reads a column the system declared it reads, or one it owns.
    ///
    /// # Errors
    /// [`SimError::UndeclaredRead`] for any other column; [`SimError::MissingColumn`] if a
    /// declared column does not exist in the world.
    pub fn read(&self, column: &str) -> Result<&[i64], SimError> {
        if let Some(o) = self.owned(column) {
            return Ok(&o.rows);
        }
        let Some(read) = self.access.read_of(column) else {
            return Err(SimError::UndeclaredRead {
                system: self.system,
                column: column.to_owned(),
            });
        };
        if self.dropped.contains_key(column) {
            return Ok(&[]);
        }
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

    /// A keyed lookup: row `row` of `column`, through the same checks as [`SystemView::read`].
    /// `None` for a row the column does not have. For a rate or mode read that registration
    /// dropped because its owner is absent, every row gives the read's declared neutral, never
    /// the type's zero (`05.1` §2 rule 2; `rh-contracts` accessors call this).
    ///
    /// # Errors
    /// As [`SystemView::read`].
    pub fn value(&self, column: &str, row: usize) -> Result<Option<i64>, SimError> {
        if self.owned(column).is_none()
            && self.access.read_of(column).is_some()
            && let Some(&neutral) = self.dropped.get(column)
        {
            return Ok(Some(neutral));
        }
        Ok(self.read(column)?.get(row).copied())
    }

    /// The ledger row of `person`: its position in `person.id`, which is sorted ascending as a
    /// `u64` (SPR-011 ruling 5). The system must own or read `person.id`.
    ///
    /// # Errors
    /// As [`SystemView::read`] of `person.id`.
    pub fn person_row(&self, person: rh_ids::PersonId) -> Result<Option<usize>, SimError> {
        Ok(self
            .read(crate::ledger::PERSON_ID)?
            .binary_search_by_key(&person.to_raw(), |v| v.cast_unsigned())
            .ok())
    }

    /// Mutable access to a column the system owns. Taking it marks the column written: the core
    /// then compacts, checks and copies after it as after any write (SPR-038 T-03), so a step
    /// takes it only to write.
    ///
    /// # Errors
    /// [`SimError::UnownedWrite`] for any column the system does not declare as a write.
    pub fn write(&mut self, column: &str) -> Result<&mut Column, SimError> {
        self.owned
            .iter_mut()
            .find(|o| o.name == column)
            .map(|o| {
                if !o.written {
                    o.written = true;
                    // The pre-image is taken before the step can change a row (S12); the planted
                    // engine of `mutant-late-pre-image` takes it after the step instead.
                    if o.guarded && !cfg!(feature = "mutant-late-pre-image") {
                        o.pre = Some(o.rows.clone());
                    }
                }
                &mut o.rows
            })
            .ok_or_else(|| SimError::UnownedWrite {
                system: self.system,
                column: column.to_owned(),
            })
    }
}
