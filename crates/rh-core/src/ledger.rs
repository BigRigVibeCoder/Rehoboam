//! The person ledger and the life-course log, kept by the core (SPR-011 T-03, ruling 5).
//!
//! **The ledger** exists when `person.id` is a column of the world (its owner, `M2.demography`,
//! declares it); a world without `person.id` has no ledger, whatever other `person.*` columns it
//! has (the P0 smoke world). Every `person.*` column, whoever owns it, is one row per living person, aligned
//! with `person.id` and sorted ascending by it (as a `u64`; the column stores its bits as `i64`).
//! A death writes `person.alive = 0`; the dead row stays readable for the rest of the instant, and
//! the core removes it from **every** `person.*` column by a compaction pass at the start of
//! `Bookkeeping` of that instant, before the invariants and before any hash is taken. That is a
//! core-only exemption from single-writer ownership: compaction is a private act of the engine, and
//! no `SystemModel` can call it or remove a row itself. A step that changes the row count of a
//! `person.*` column is refused with [`SimError::Ledger`]; rows enter only at genesis (the host's
//! initial tables) until P1.3 adds a core append path for births.
//!
//! **The life-course log** (`life_course.*`) holds a row for everyone who ever entered. It is
//! append-only: a step may append rows, never remove or reorder them, never change a row's entry
//! fields, and write a row's exit fields (`exit_kind`, `exit_tick`, `death_cause`) only in the
//! step that closes it, from open (`exit_kind = 0`) to closed (not every field need change then);
//! a row appended open carries the open values in its exit fields, and one appended closed (a
//! back-filled ancestor, absent from the ledger: DEF-6) any; [`LifeCourseGuard`] refuses
//! anything else (DEF-3). The log is never
//! compacted (ADR-0007): kinship through the dead walks it (REV-P1a NEW-1).
//!
//! **Core invariants** (`Bookkeeping`, after compaction): every `person.*` column is as long as
//! `person.id`; `person.id` strictly increases; no row has `person.alive = 0`; every `life_course.*`
//! column is as long as `life_course.person_id`; and the open life-course entries are exactly the
//! ledger rows, one each, so the living count equals the open entries and a closed entry is absent
//! from the ledger.
//!
//! REF: `CODEX/05_PROJECT/SPR-011_WalkingSkeleton.md` T-03, ruling 5;
//! `specs/systems/M2.demography.odd.toml` (`[[owns]]` of `person.*` and `life_course.*`);
//! `docs/05_architecture/05.1_simulation_core.md` §5, §7, §9; ADR-0007; ADR-0026.

use std::collections::BTreeMap;

use crate::phase::Phase;
use crate::system::SimError;
use crate::world::{Column, ColumnId, WorldTables};

/// The prefix of the ledger's columns.
pub const PERSON_PREFIX: &str = "person.";
/// The ledger's key column.
pub const PERSON_ID: &str = "person.id";
/// The column whose 0 marks a row for compaction.
pub const PERSON_ALIVE: &str = "person.alive";
/// The prefix of the life-course log's columns.
pub const LIFE_COURSE_PREFIX: &str = "life_course.";
/// The life-course log's key column.
pub const LIFE_COURSE_PERSON: &str = "life_course.person_id";
/// The exit field that says whether an entry is open (0) or closed.
pub const LIFE_COURSE_EXIT_KIND: &str = "life_course.exit_kind";
/// Every exit field: written once, with `exit_kind`, when the person leaves.
pub const LIFE_COURSE_EXIT_FIELDS: [&str; 3] = [
    "life_course.death_cause",
    "life_course.exit_kind",
    "life_course.exit_tick",
];
/// `exit_kind` of an open entry.
pub const OPEN: i64 = 0;
/// Every exit field's value while the entry is open (`specs/systems/M2.demography.odd.toml`:
/// `exit_kind` open, `exit_tick` `i64::MAX`, `death_cause` none). An entry appended open carries
/// exactly these; only the step that closes an entry changes them (DEF-3).
pub const LIFE_COURSE_OPEN_VALUES: [(&str, i64); 3] = [
    ("life_course.death_cause", 0),
    ("life_course.exit_kind", OPEN),
    ("life_course.exit_tick", i64::MAX),
];

/// The core's name in errors.
const CORE: &str = "rh-core";

/// The ledger's columns, found when the engine is built: every `person.*` and `life_course.*`
/// column of the world, in name order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger {
    person: Vec<String>,
    life_course: Vec<String>,
    /// The `person.*` columns' slots, in name order (SPR-038 T-02).
    person_ids: Vec<ColumnId>,
    /// `person.alive`'s slot.
    alive: ColumnId,
}

impl Ledger {
    /// The ledger of `tables`: `None` when the world has no `person.id` column. Ruling 5: the
    /// ledger exists when `person.id` is registered; a world with other `person.*` columns and no
    /// `person.id` (the P0 smoke world's `person.alive`, `person.deaths`, `person.pending`) has no
    /// ledger, and the core neither compacts nor checks those columns.
    ///
    /// # Errors
    /// [`SimError::Ledger`] when the world has `person.id` but not `person.alive`,
    /// `life_course.person_id` or the life-course exit fields: its rows could not be compacted or
    /// checked, so the engine refuses to run rather than fail open.
    pub fn of(tables: &WorldTables) -> Result<Option<Self>, SimError> {
        if tables.get(PERSON_ID).is_none() {
            return Ok(None);
        }
        let names = |prefix: &str| -> Vec<String> {
            tables
                .names()
                .filter(|n| n.starts_with(prefix))
                .map(str::to_owned)
                .collect()
        };
        let person = names(PERSON_PREFIX);
        let person_ids = person.iter().filter_map(|c| tables.id(c)).collect();
        let required = [PERSON_ALIVE, LIFE_COURSE_PERSON]
            .into_iter()
            .chain(LIFE_COURSE_EXIT_FIELDS);
        for column in required {
            if tables.get(column).is_none() {
                return Err(error(format!(
                    "the world has person.id but no {column}: the ledger needs person.alive and \
                     the life-course log to compact and check its rows (SPR-011 ruling 5)"
                )));
            }
        }
        let Some(alive) = tables.id(PERSON_ALIVE) else {
            return Err(error(format!(
                "the world has person.id but no {PERSON_ALIVE}"
            )));
        };
        Ok(Some(Self {
            person,
            life_course: names(LIFE_COURSE_PREFIX),
            person_ids,
            alive,
        }))
    }

    /// Whether `column` is a ledger (`person.*`) column.
    #[must_use]
    pub fn is_person(&self, column: &str) -> bool {
        self.person.iter().any(|c| c == column)
    }

    /// Whether `column` is a life-course column.
    #[must_use]
    pub fn is_life_course(&self, column: &str) -> bool {
        self.life_course.iter().any(|c| c == column)
    }

    /// The `person.*` columns, in name order.
    #[must_use]
    pub fn person_columns(&self) -> &[String] {
        &self.person
    }

    /// The `person.*` columns with their slots, in name order (SPR-038 T-02).
    pub(crate) fn person_slots(&self) -> impl Iterator<Item = (&str, ColumnId)> {
        self.person
            .iter()
            .map(String::as_str)
            .zip(self.person_ids.iter().copied())
    }

    /// Removes every row with `person.alive = 0` from every `person.*` column (the core-only
    /// exemption from single-writer ownership; module docs). Returns how many rows left. A table
    /// with no dead row is left untouched, so its columns keep their versions.
    ///
    /// **Positional** (SPR-038 S4): one scan of `person.alive` finds the dead rows' positions (into
    /// `dead`, a buffer the engine keeps), then each `person.*` column loses exactly those rows by
    /// moving its later rows down, in place. The same rows stay, in the same order, as the
    /// collect it replaces; every column it changes takes a new version.
    ///
    /// # Errors
    /// [`SimError::Invariant`] `ledger-aligned` if the columns are not aligned: compaction would
    /// remove the wrong rows.
    pub(crate) fn compact(
        &self,
        tables: &mut WorldTables,
        dead: &mut Vec<usize>,
    ) -> Result<usize, SimError> {
        self.check_aligned(tables)?;
        dead.clear();
        if let Some(alive) = tables.get_at(self.alive) {
            dead.extend(
                alive
                    .iter()
                    .enumerate()
                    .filter(|&(_, &a)| a == 0)
                    .map(|(row, _)| row),
            );
        }
        if dead.is_empty() {
            return Ok(0);
        }
        for &id in &self.person_ids {
            let Some(mut rows) = tables.take_at(id) else {
                continue;
            };
            remove_rows(&mut rows, dead);
            #[cfg(not(feature = "mutant-compact-keeps-version"))]
            tables.put_at(id, rows);
            #[cfg(feature = "mutant-compact-keeps-version")]
            tables.put_at_keeping_version(id, rows);
        }
        Ok(dead.len())
    }

    fn check_aligned(&self, tables: &WorldTables) -> Result<(), SimError> {
        aligned(tables, &self.person, PERSON_ID, "ledger-aligned")?;
        aligned(
            tables,
            &self.life_course,
            LIFE_COURSE_PERSON,
            "life-course-aligned",
        )
    }

    /// The core's ledger invariants (module docs), after compaction.
    ///
    /// # Errors
    /// The first violated one, as [`SimError::Invariant`].
    pub(crate) fn check(&self, tables: &WorldTables) -> Result<(), SimError> {
        self.check_aligned(tables)?;
        let ids = column(tables, PERSON_ID);
        if let Some(w) = ids
            .windows(2)
            .find(|w| w[0].cast_unsigned() >= w[1].cast_unsigned())
        {
            return Err(violated(
                "ledger-sorted",
                format!(
                    "person.id is not strictly ascending: {} before {}",
                    w[0].cast_unsigned(),
                    w[1].cast_unsigned()
                ),
            ));
        }
        if let Some(i) = column(tables, PERSON_ALIVE).iter().position(|&a| a == 0) {
            return Err(violated(
                "ledger-no-dead-row",
                format!(
                    "person {} has alive = 0 after compaction",
                    ids[i].cast_unsigned()
                ),
            ));
        }
        let persons = column(tables, LIFE_COURSE_PERSON);
        let mut open: BTreeMap<u64, usize> = BTreeMap::new();
        let kinds = column(tables, LIFE_COURSE_EXIT_KIND);
        for (&p, &kind) in persons.iter().zip(kinds) {
            if kind == OPEN {
                *open.entry(p.cast_unsigned()).or_default() += 1;
            }
        }
        if let Some((p, n)) = open.iter().find(|&(_, &n)| n > 1) {
            return Err(violated(
                "life-course-one-open-entry",
                format!("person {p} has {n} open life-course entries"),
            ));
        }
        for &id in ids {
            if open.remove(&id.cast_unsigned()).is_none() {
                return Err(violated(
                    "life-course-one-open-entry",
                    format!(
                        "ledger row {} has no open life-course entry",
                        id.cast_unsigned()
                    ),
                ));
            }
        }
        if let Some(p) = open.keys().next() {
            return Err(violated(
                "life-course-closed-or-absent",
                format!(
                    "person {p} has an open life-course entry but no ledger row ({} open, {} living)",
                    open.len() + ids.len(),
                    ids.len()
                ),
            ));
        }
        Ok(())
    }

    /// What a step must preserve of the ledger columns it owns: their row counts, and the
    /// life-course rows as they stood (see [`LifeCourseGuard`]).
    ///
    /// `full` is whether the job has work at this instant: a due timer, a non-empty inbox, or a
    /// binding that is not drain-only. A job without work keeps only its row counts, every
    /// ledger column's (BCK-002 A-60 part, SPR-014): this is a known narrowing. Its value checks
    /// (A-2's ids, the life-course fields, DEF-3, DEF-6) are skipped, since an empty drain writes
    /// nothing; A-60 revisits it with the guard's place in release builds.
    pub(crate) fn guard(
        &self,
        tables: &WorldTables,
        owned: impl Iterator<Item = &'static str>,
        phase: Phase,
        full: bool,
    ) -> LifeCourseGuard {
        let mut guard = LifeCourseGuard {
            phase,
            person_rows: Vec::new(),
            ids: None,
            alive_after_compaction: false,
            life_course: Vec::new(),
            life_course_rows: Vec::new(),
        };
        for column in owned {
            if self.is_person(column) {
                let len = tables.get(column).map_or(0, Vec::len);
                guard.person_rows.push((column, len));
                // A-2: only a job that writes person.id pays for keeping its ids.
                if full && column == PERSON_ID {
                    guard.ids = tables.get(column).cloned();
                }
                // G-1: compaction ran at the start of Bookkeeping; a death written after it would
                // survive the instant.
                if column == PERSON_ALIVE && phase == Phase::Bookkeeping {
                    guard.alive_after_compaction = true;
                }
            } else if self.is_life_course(column) {
                if full {
                    let rows = tables.get(column).cloned().unwrap_or_default();
                    guard.life_course.push((column, rows));
                } else {
                    let len = tables.get(column).map_or(0, Vec::len);
                    guard.life_course_rows.push((column, len));
                }
            }
        }
        guard
    }
}

/// The ledger columns one step owns, as they stood before it (see [`Ledger::guard`]).
#[derive(Debug)]
pub(crate) struct LifeCourseGuard {
    phase: Phase,
    person_rows: Vec<(&'static str, usize)>,
    /// `person.id` before the step, kept only for a job with work that writes it (A-2).
    ids: Option<Column>,
    /// Whether the job writes `person.alive` in `Bookkeeping`, after the compaction (G-1).
    alive_after_compaction: bool,
    /// The life-course columns as they stood, for a job with work.
    life_course: Vec<(&'static str, Column)>,
    /// The life-course row counts, for a job without work: they must not change.
    life_course_rows: Vec<(&'static str, usize)>,
}

impl LifeCourseGuard {
    /// Whether the step owns no ledger column, so there is nothing to check.
    pub(crate) fn is_empty(&self) -> bool {
        self.person_rows.is_empty()
            && self.life_course.is_empty()
            && self.life_course_rows.is_empty()
    }

    /// A-2: an existing row's `person.id` never changes in a step; G-1: no death after the
    /// instant's compaction.
    fn check_ids_and_alive(
        &self,
        system: &'static str,
        tables: &WorldTables,
    ) -> Result<(), SimError> {
        let refuse = |message: String| SimError::Ledger { system, message };
        if let Some(before) = &self.ids {
            let after = tables.get(PERSON_ID).map(Vec::as_slice).unwrap_or_default();
            if let Some((row, (old, new))) = before
                .iter()
                .zip(after)
                .enumerate()
                .find(|(_, (old, new))| old != new)
            {
                return Err(refuse(format!(
                    "person.id row {row} changed from {} to {} in {}: an existing row's person.id \
                     never changes; rows leave only by rh-core's compaction and enter only at \
                     genesis (SPR-011 ruling 5; A-2)",
                    old.cast_unsigned(),
                    new.cast_unsigned(),
                    self.phase
                )));
            }
        }
        if self.alive_after_compaction
            && let Some(row) = tables
                .get(PERSON_ALIVE)
                .and_then(|alive| alive.iter().position(|&a| a == 0))
        {
            let id = tables
                .get(PERSON_ID)
                .and_then(|ids| ids.get(row))
                .map_or(0, |id| id.cast_unsigned());
            return Err(refuse(format!(
                "wrote person.alive = 0 for person {id} in {}, after the instant's compaction: \
                 a death is applied before Bookkeeping (SPR-011 ruling 5; G-1)",
                self.phase
            )));
        }
        Ok(())
    }

    /// Checks the step's ledger columns after it ran.
    ///
    /// # Errors
    /// [`SimError::Ledger`] naming `system` and the column: a changed `person.*` row count, a
    /// removed life-course row, a changed entry field, an exit field written twice, or an entry
    /// appended closed for a person in the ledger.
    pub(crate) fn check(&self, system: &'static str, tables: &WorldTables) -> Result<(), SimError> {
        let refuse = |message: String| SimError::Ledger { system, message };
        self.check_ids_and_alive(system, tables)?;
        for (column, before) in &self.person_rows {
            let after = tables.get(column).map_or(0, Vec::len);
            if after != *before {
                return Err(refuse(format!(
                    "{column} went from {before} to {after} rows: only rh-core's compaction \
                     removes ledger rows, and rows enter only at genesis until P1.3 (SPR-011 \
                     ruling 5)"
                )));
            }
        }
        for (column, before) in &self.life_course_rows {
            let after = tables.get(column).map_or(0, Vec::len);
            if after != *before {
                return Err(refuse(format!(
                    "{column} went from {before} to {after} rows in a step with no work (an \
                     empty drain): such a step writes no ledger row (BCK-002 A-60 part)"
                )));
            }
        }
        let kinds_after = tables
            .get(LIFE_COURSE_EXIT_KIND)
            .map(Vec::as_slice)
            .unwrap_or_default();
        // A job that does not own exit_kind cannot change it: before is after.
        let kinds_before = self
            .life_course
            .iter()
            .find(|(c, _)| *c == LIFE_COURSE_EXIT_KIND)
            .map_or(kinds_after, |(_, rows)| rows.as_slice());
        for (column, before) in &self.life_course {
            let after = tables.get(column).map(Vec::as_slice).unwrap_or_default();
            if after.len() < before.len() {
                return Err(refuse(format!(
                    "{column} went from {} to {} rows: the life-course log is append-only \
                     (SPR-011 ruling 5)",
                    before.len(),
                    after.len()
                )));
            }
            let open_value = LIFE_COURSE_OPEN_VALUES
                .iter()
                .find(|(c, _)| c == column)
                .map(|&(_, v)| v);
            for (row, (old, new)) in before.iter().zip(after).enumerate() {
                if old == new {
                    continue;
                }
                if open_value.is_none() {
                    return Err(refuse(format!(
                        "{column} row {row} changed from {old} to {new}: a life-course entry's \
                         fields are written once, at entry (SPR-011 ruling 5)"
                    )));
                }
                // DEF-3: exit fields are written only by the step that closes the entry.
                let was_open = kinds_before.get(row) == Some(&OPEN);
                let closes = was_open && kinds_after.get(row) != Some(&OPEN);
                if !was_open {
                    return Err(refuse(format!(
                        "{column} row {row} changed from {old} to {new} on a closed entry: exit \
                         fields are written once (SPR-011 ruling 5)"
                    )));
                }
                if !closes {
                    return Err(refuse(format!(
                        "{column} row {row} changed from {old} to {new} on an entry this step \
                         leaves open: exit fields are written only by the step that closes the \
                         entry (SPR-011 ruling 5; DEF-3)"
                    )));
                }
            }
            // DEF-3: an entry appended open carries the open value in every exit field; one
            // appended closed (a back-filled ancestor) may carry any.
            if let Some(open) = open_value {
                for (row, &value) in after.iter().enumerate().skip(before.len()) {
                    if kinds_after.get(row) == Some(&OPEN) && value != open {
                        return Err(refuse(format!(
                            "{column} row {row} was appended open with {value}, not the open \
                             value {open} (SPR-011 ruling 5; DEF-3)"
                        )));
                    }
                }
            }
        }
        self.check_closed_appends(system, tables, kinds_after)
    }

    /// DEF-6: an entry appended closed is a dead person's (a back-filled ancestor); one whose
    /// `person_id` is a ledger row, alive or dying this instant, is refused.
    fn check_closed_appends(
        &self,
        system: &'static str,
        tables: &WorldTables,
        kinds_after: &[i64],
    ) -> Result<(), SimError> {
        // A job that does not own life_course.person_id appends no entry (its length is fixed).
        let Some(before) = self
            .life_course
            .iter()
            .find(|(c, _)| *c == LIFE_COURSE_PERSON)
            .map(|(_, rows)| rows.len())
        else {
            return Ok(());
        };
        let ids = column(tables, PERSON_ID);
        let persons = column(tables, LIFE_COURSE_PERSON);
        for (row, &p) in persons.iter().enumerate().skip(before) {
            let closed = kinds_after.get(row).is_some_and(|&k| k != OPEN);
            // The ledger is sorted by id as a u64; a step may have unsorted it, so scan.
            if closed && ids.contains(&p) {
                return Err(SimError::Ledger {
                    system,
                    message: format!(
                        "life_course row {row} was appended closed for person {}, who has a \
                         ledger row: only a person absent from the ledger (a back-filled \
                         ancestor) is appended closed (SPR-011 ruling 5; DEF-6)",
                        p.cast_unsigned()
                    ),
                });
            }
        }
        Ok(())
    }
}

/// Removes the rows at `dead` (ascending, each below `rows.len()`) from `rows`, in place: one
/// `Vec::remove` for a single row, else each run between two removed rows moved down once.
fn remove_rows(rows: &mut Column, dead: &[usize]) {
    if let &[row] = dead {
        rows.remove(row);
        return;
    }
    let Some(&first) = dead.first() else {
        return;
    };
    let mut write = first;
    for (k, &row) in dead.iter().enumerate() {
        let next = dead.get(k + 1).copied().unwrap_or(rows.len());
        rows.copy_within(row + 1..next, write);
        write += next - row - 1;
    }
    rows.truncate(write);
}

fn column<'t>(tables: &'t WorldTables, name: &str) -> &'t [i64] {
    tables.get(name).map(Vec::as_slice).unwrap_or_default()
}

fn aligned(
    tables: &WorldTables,
    columns: &[String],
    key: &str,
    invariant: &'static str,
) -> Result<(), SimError> {
    let len = column(tables, key).len();
    match columns
        .iter()
        .find(|c| tables.get(c).map_or(0, Vec::len) != len)
    {
        Some(c) => Err(violated(
            invariant,
            format!(
                "{c} has {} rows, {key} has {len}",
                tables.get(c).map_or(0, Vec::len)
            ),
        )),
        None => Ok(()),
    }
}

fn violated(name: &'static str, detail: String) -> SimError {
    SimError::Invariant {
        system: CORE,
        name,
        detail,
    }
}

fn error(message: String) -> SimError {
    SimError::Ledger {
        system: CORE,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::remove_rows;

    /// SPR-038 S4: positional removal keeps exactly the rows the collect it replaces kept, in
    /// order, over random columns and dead sets (none, one, runs, the first and the last rows).
    #[test]
    fn positional_removal_keeps_what_the_collect_kept() {
        let mut x = 0x0038_0004_u64;
        let mut next = || {
            x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        for n in [0_usize, 1, 2, 60, 600, 3_000] {
            for _ in 0..40 {
                let rate = 1 + next() % 50;
                let rows: Vec<i64> = (0..n).map(|_| next().cast_signed()).collect();
                let keep: Vec<bool> = (0..n).map(|_| next() % rate != 0).collect();
                let dead: Vec<usize> = (0..n).filter(|&r| !keep[r]).collect();
                let expected: Vec<i64> = rows
                    .iter()
                    .zip(&keep)
                    .filter_map(|(&v, &k)| k.then_some(v))
                    .collect();
                let mut got = rows.clone();
                remove_rows(&mut got, &dead);
                assert_eq!(got, expected, "n {n}, {} dead", dead.len());
            }
        }
    }
}
