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
//! **The guard and the gates** (SPR-038 T-03). The guard of a step's ledger columns works from
//! pre-images copied at the step's first `write()` of a guarded column (`person.id`, every
//! `life_course.*`), so every write is value-checked whatever the job's work: SPR-014's narrowing
//! (a job without work kept only its row counts) is gone, and a job that writes no guarded column
//! copies nothing. The engine compacts only after `person.alive` was written or a row inserted, and
//! runs [`Ledger::check`] only after a ledger column was written, inserted or compacted, since it
//! last ran; both are exact, since compaction of a table with no dead row changes nothing and the
//! check is a pure function of the tables. The check is O(N + L), its SPR-011 form kept as the test
//! oracle.
//!
//! REF: `CODEX/05_PROJECT/SPR-011_WalkingSkeleton.md` T-03, ruling 5;
//! `specs/systems/M2.demography.odd.toml` (`[[owns]]` of `person.*` and `life_course.*`);
//! `docs/05_architecture/05.1_simulation_core.md` §5, §7, §9; ADR-0007; ADR-0026;
//! `CODEX/05_PROJECT/SPR-038_PerInstantCost.md` T-02, T-03.

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
    /// Every ledger column's slot, `person.*` and `life_course.*` (SPR-038 T-03: the columns
    /// whose writes make the core check the ledger again).
    all_ids: Vec<ColumnId>,
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
        let life_course = names(LIFE_COURSE_PREFIX);
        let all_ids = person
            .iter()
            .chain(&life_course)
            .filter_map(|c| tables.id(c))
            .collect();
        Ok(Some(Self {
            person,
            life_course,
            person_ids,
            alive,
            all_ids,
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

    /// `person.alive`'s slot: a write to it makes the core compact again (SPR-038 T-03).
    pub(crate) const fn alive_slot(&self) -> ColumnId {
        self.alive
    }

    /// Every ledger column's slot (SPR-038 T-03).
    pub(crate) fn slots(&self) -> &[ColumnId] {
        &self.all_ids
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
            tables.put_at(id, rows, true);
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

    /// The core's ledger invariants (module docs), after compaction, in O(N + L) (SPR-038 T-03):
    /// the open entries' persons are collected and sorted (linear on a log appended in id order,
    /// the common case), then merged with the ledger's ascending ids. The results, names and texts
    /// are exactly those of [`Ledger::check_oracle`], the form it replaced, kept as the test
    /// oracle.
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
        let mut open: Vec<u64> = column(tables, LIFE_COURSE_PERSON)
            .iter()
            .zip(column(tables, LIFE_COURSE_EXIT_KIND))
            .filter(|&(_, &kind)| kind == OPEN)
            .map(|(&p, _)| p.cast_unsigned())
            .collect();
        open.sort_unstable();
        // The smallest person with two or more open entries.
        let mut run = 0;
        while run < open.len() {
            let n = open[run..].iter().take_while(|&&p| p == open[run]).count();
            if n > 1 {
                return Err(violated(
                    "life-course-one-open-entry",
                    format!("person {} has {n} open life-course entries", open[run]),
                ));
            }
            run += n;
        }
        // Each ledger row, ascending, against the open entries, ascending and now unique.
        let mut j = 0;
        let mut first_unmatched = None;
        for &id in ids {
            let id = id.cast_unsigned();
            while j < open.len() && open[j] < id {
                first_unmatched.get_or_insert(open[j]);
                j += 1;
            }
            if open.get(j) == Some(&id) {
                j += 1;
            } else {
                return Err(violated(
                    "life-course-one-open-entry",
                    format!("ledger row {id} has no open life-course entry"),
                ));
            }
        }
        if let Some(p) = first_unmatched.or_else(|| open.get(j).copied()) {
            return Err(violated(
                "life-course-closed-or-absent",
                format!(
                    "person {p} has an open life-course entry but no ledger row ({} open, {} living)",
                    open.len(),
                    ids.len()
                ),
            ));
        }
        Ok(())
    }

    /// The ledger invariants as SPR-011 wrote them (a map of open entries over the whole log): the
    /// oracle [`Ledger::check`] must agree with on every table (SPR-038 T-03).
    ///
    /// # Errors
    /// As [`Ledger::check`].
    #[cfg(test)]
    pub(crate) fn check_oracle(&self, tables: &WorldTables) -> Result<(), SimError> {
        use std::collections::BTreeMap;
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

    /// The guard of one step's ledger columns (`guarded`, one per owned ledger column, in write
    /// order), after the level put every column back.
    ///
    /// Every write is value-checked, whatever the job's work (SPR-038 S12; BCK-002 A-60): the
    /// pre-image of a guarded column (`person.id`, every `life_course.*`) is copied at the step's
    /// first `write()` of it, so a column the step did not write is unchanged and needs no check,
    /// and a job that writes no guarded column copies nothing. The row counts of `person.*`
    /// columns are checked for every job.
    ///
    /// # Errors
    /// [`SimError::Ledger`] naming `system` and the column: a changed `person.id` row (A-2), a
    /// death after the instant's compaction (G-1), a changed `person.*` row count, a removed
    /// life-course row, a changed entry field, an exit field written twice or on an entry the step
    /// leaves open, an entry appended open without the open values, or an entry appended closed for
    /// a person in the ledger (DEF-3, DEF-6).
    pub(crate) fn check_step(
        system: &'static str,
        phase: Phase,
        guarded: &[Guarded],
        tables: &WorldTables,
    ) -> Result<(), SimError> {
        let refuse = |message: String| SimError::Ledger { system, message };
        let pre = |name: &str| {
            guarded
                .iter()
                .find(|g| g.name == name)
                .and_then(|g| g.pre.as_deref())
        };
        // A-2: an existing row's person.id never changes in a step.
        if let Some(before) = pre(PERSON_ID) {
            let after = column(tables, PERSON_ID);
            if let Some((row, (old, new))) = before
                .iter()
                .zip(after)
                .enumerate()
                .find(|(_, (old, new))| old != new)
            {
                return Err(refuse(format!(
                    "person.id row {row} changed from {} to {} in {phase}: an existing row's \
                     person.id never changes; rows leave only by rh-core's compaction and enter \
                     only at genesis (SPR-011 ruling 5; A-2)",
                    old.cast_unsigned(),
                    new.cast_unsigned(),
                )));
            }
        }
        // G-1: compaction ran at the start of Bookkeeping; a death written after it would survive
        // the instant. An unwritten person.alive holds no 0 after compaction.
        let wrote_alive = guarded.iter().any(|g| g.name == PERSON_ALIVE && g.written);
        if wrote_alive
            && phase == Phase::Bookkeeping
            && let Some(row) = tables
                .get(PERSON_ALIVE)
                .and_then(|alive| alive.iter().position(|&a| a == 0))
        {
            let id = tables
                .get(PERSON_ID)
                .and_then(|ids| ids.get(row))
                .map_or(0, |id| id.cast_unsigned());
            return Err(refuse(format!(
                "wrote person.alive = 0 for person {id} in {phase}, after the instant's \
                 compaction: a death is applied before Bookkeeping (SPR-011 ruling 5; G-1)"
            )));
        }
        for g in guarded.iter().filter(|g| g.name.starts_with(PERSON_PREFIX)) {
            let after = tables.get(g.name).map_or(0, Vec::len);
            if after != g.rows_before {
                return Err(refuse(format!(
                    "{} went from {} to {after} rows: only rh-core's compaction removes ledger \
                     rows, and rows enter only at genesis until P1.3 (SPR-011 ruling 5)",
                    g.name, g.rows_before
                )));
            }
        }
        let kinds_after = column(tables, LIFE_COURSE_EXIT_KIND);
        // A step that did not write exit_kind left it as it was: before is after.
        let kinds_before = pre(LIFE_COURSE_EXIT_KIND).unwrap_or(kinds_after);
        for g in guarded {
            let Some(before) = g.pre.as_deref() else {
                continue;
            };
            if !g.name.starts_with(LIFE_COURSE_PREFIX) {
                continue;
            }
            check_entries(
                g.name,
                before,
                column(tables, g.name),
                kinds_before,
                kinds_after,
            )
            .map_err(refuse)?;
        }
        // DEF-6: only a step that wrote life_course.person_id appended an entry.
        let Some(before) = pre(LIFE_COURSE_PERSON).map(<[i64]>::len) else {
            return Ok(());
        };
        let ids = column(tables, PERSON_ID);
        let persons = column(tables, LIFE_COURSE_PERSON);
        for (row, &p) in persons.iter().enumerate().skip(before) {
            let closed = kinds_after.get(row).is_some_and(|&k| k != OPEN);
            // The ledger is sorted by id as a u64; a step may have unsorted it, so scan.
            if closed && ids.contains(&p) {
                return Err(refuse(format!(
                    "life_course row {row} was appended closed for person {}, who has a ledger \
                     row: only a person absent from the ledger (a back-filled ancestor) is \
                     appended closed (SPR-011 ruling 5; DEF-6)",
                    p.cast_unsigned()
                )));
            }
        }
        Ok(())
    }
}

/// What the ledger guard keeps of one owned ledger column across a step (SPR-038 T-03): its row
/// count before the step, whether the step wrote it, and its pre-image if it is guarded and was
/// written. The engine keeps one buffer of these per system.
#[derive(Debug)]
pub(crate) struct Guarded {
    pub(crate) name: &'static str,
    pub(crate) rows_before: usize,
    pub(crate) written: bool,
    pub(crate) pre: Option<Column>,
}

/// Whether `column` is one whose pre-image the ledger guard keeps: `person.id` (A-2) or any
/// life-course column.
pub(crate) fn guarded_column(column: &str) -> bool {
    column == PERSON_ID || column.starts_with(LIFE_COURSE_PREFIX)
}

/// One written life-course column against its pre-image: append-only, entry fields written once,
/// exit fields only by the step that closes the entry, and an entry appended open with the open
/// values (SPR-011 ruling 5; DEF-3). The message, unprefixed, on a violation.
fn check_entries(
    column: &str,
    before: &[i64],
    after: &[i64],
    kinds_before: &[i64],
    kinds_after: &[i64],
) -> Result<(), String> {
    if after.len() < before.len() {
        return Err(format!(
            "{column} went from {} to {} rows: the life-course log is append-only (SPR-011 \
             ruling 5)",
            before.len(),
            after.len()
        ));
    }
    let open_value = LIFE_COURSE_OPEN_VALUES
        .iter()
        .find(|(c, _)| *c == column)
        .map(|&(_, v)| v);
    for (row, (old, new)) in before.iter().zip(after).enumerate() {
        if old == new {
            continue;
        }
        if open_value.is_none() {
            return Err(format!(
                "{column} row {row} changed from {old} to {new}: a life-course entry's fields are \
                 written once, at entry (SPR-011 ruling 5)"
            ));
        }
        // DEF-3: exit fields are written only by the step that closes the entry.
        let was_open = kinds_before.get(row) == Some(&OPEN);
        let closes = was_open && kinds_after.get(row) != Some(&OPEN);
        if !was_open {
            return Err(format!(
                "{column} row {row} changed from {old} to {new} on a closed entry: exit fields \
                 are written once (SPR-011 ruling 5)"
            ));
        }
        if !closes {
            return Err(format!(
                "{column} row {row} changed from {old} to {new} on an entry this step leaves \
                 open: exit fields are written only by the step that closes the entry (SPR-011 \
                 ruling 5; DEF-3)"
            ));
        }
    }
    // DEF-3: an entry appended open carries the open value in every exit field; one appended
    // closed (a back-filled ancestor) may carry any.
    if let Some(open) = open_value {
        for (row, &value) in after.iter().enumerate().skip(before.len()) {
            if kinds_after.get(row) == Some(&OPEN) && value != open {
                return Err(format!(
                    "{column} row {row} was appended open with {value}, not the open value \
                     {open} (SPR-011 ruling 5; DEF-3)"
                ));
            }
        }
    }
    Ok(())
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

    /// SPR-038 T-03: the O(N + L) `Ledger::check` gives the oracle's result (the same error, name
    /// and text, or none) on random ledgers, valid and with one planted violation each, the log in
    /// id order or shuffled, over three sizes and eight seeds.
    #[test]
    fn the_merge_check_equals_the_oracle_on_random_ledgers() {
        use super::{LIFE_COURSE_EXIT_KIND, LIFE_COURSE_PERSON, Ledger, PERSON_ALIVE, PERSON_ID};
        use crate::world::WorldTables;
        let mut x = 0x0038_0303_u64;
        let mut next = move || {
            x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        let mut violated = 0;
        let mut compared = 0;
        for n in [1_usize, 60, 600] {
            for _ in 0..8 {
                for kind in 0..9 {
                    let mut ids: Vec<i64> = (0..n * 3).map(|_| next().cast_signed()).collect();
                    ids.sort_unstable_by_key(|v| v.cast_unsigned());
                    ids.dedup();
                    let mut log: Vec<(i64, i64)> = ids
                        .iter()
                        .enumerate()
                        .map(|(k, &p)| (p, i64::from(k % 3 != 0)))
                        .collect();
                    let mut living: Vec<i64> =
                        log.iter().filter(|e| e.1 == 0).map(|e| e.0).collect();
                    let mut alive = vec![1; living.len()];
                    if next() % 2 == 0 {
                        for i in (1..log.len()).rev() {
                            let j = usize::try_from(next() % (i as u64 + 1)).unwrap_or(0);
                            log.swap(i, j);
                        }
                    }
                    let m = living.len();
                    let pick =
                        |r: u64, len: usize| usize::try_from(r % len.max(1) as u64).unwrap_or(0);
                    match kind {
                        1 if m > 1 => living.swap(0, m - 1),
                        2 if m > 0 => alive[pick(next(), m)] = 0,
                        3 if m > 0 => log.push((living[pick(next(), m)], 0)),
                        4 if m > 0 => {
                            let p = living[pick(next(), m)];
                            log.iter_mut().filter(|e| e.0 == p).for_each(|e| e.1 = 1);
                        }
                        5 => log.push((next().cast_signed(), 0)),
                        6 if m > 0 => {
                            living.remove(pick(next(), m));
                            alive.pop();
                        }
                        7 => {
                            let p = next().cast_signed();
                            log.push((p, 0));
                            log.push((p, 0));
                        }
                        8 if m > 0 => {
                            living.push(next().cast_signed());
                            alive.push(1);
                        }
                        _ => {}
                    }
                    let mut t = WorldTables::new();
                    t.insert(PERSON_ID, living);
                    t.insert(PERSON_ALIVE, alive);
                    t.insert(LIFE_COURSE_PERSON, log.iter().map(|e| e.0).collect());
                    t.insert(LIFE_COURSE_EXIT_KIND, log.iter().map(|e| e.1).collect());
                    t.insert("life_course.exit_tick", vec![0; log.len()]);
                    t.insert("life_course.death_cause", vec![0; log.len()]);
                    let ledger = Ledger::of(&t).ok().flatten().unwrap_or_default();
                    let oracle = ledger.check_oracle(&t);
                    assert_eq!(ledger.check(&t), oracle, "n {n}, kind {kind}");
                    violated += usize::from(oracle.is_err());
                    compared += 1;
                }
            }
        }
        assert!(
            violated > 100,
            "planted violations seen: {violated} of {compared}"
        );
    }
}
