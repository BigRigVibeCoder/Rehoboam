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
//! back-filled ancestor) any; [`LifeCourseGuard`] refuses anything else (DEF-3). The log is never
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

use crate::system::SimError;
use crate::world::{Column, WorldTables};

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
        let ledger = Self {
            person: names(PERSON_PREFIX),
            life_course: names(LIFE_COURSE_PREFIX),
        };
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
        Ok(Some(ledger))
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

    /// Removes every row with `person.alive = 0` from every `person.*` column (the core-only
    /// exemption from single-writer ownership; module docs). Returns how many rows left. A table
    /// with no dead row is left untouched, so its columns keep their versions.
    ///
    /// # Errors
    /// [`SimError::Invariant`] `ledger-aligned` if the columns are not aligned: compaction would
    /// remove the wrong rows.
    pub(crate) fn compact(&self, tables: &mut WorldTables) -> Result<usize, SimError> {
        self.check_aligned(tables)?;
        let alive = tables
            .get(PERSON_ALIVE)
            .map(Vec::as_slice)
            .unwrap_or_default();
        if alive.iter().all(|&a| a != 0) {
            return Ok(0);
        }
        let keep: Vec<bool> = alive.iter().map(|&a| a != 0).collect();
        let removed = keep.iter().filter(|&&k| !k).count();
        for name in &self.person {
            let Some(rows) = tables.take(name) else {
                continue;
            };
            let kept: Column = rows
                .into_iter()
                .zip(&keep)
                .filter_map(|(v, &k)| k.then_some(v))
                .collect();
            tables.insert(name, kept);
        }
        Ok(removed)
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
    pub(crate) fn guard<'c>(
        &self,
        tables: &WorldTables,
        owned: impl Iterator<Item = &'c str>,
    ) -> LifeCourseGuard {
        let mut guard = LifeCourseGuard::default();
        for column in owned {
            if self.is_person(column) {
                let len = tables.get(column).map_or(0, Vec::len);
                guard.person_rows.push((column.to_owned(), len));
            } else if self.is_life_course(column) {
                let rows = tables.get(column).cloned().unwrap_or_default();
                guard.life_course.push((column.to_owned(), rows));
            }
        }
        guard
    }
}

/// The ledger columns one step owns, as they stood before it (see [`Ledger::guard`]).
#[derive(Debug, Default)]
pub(crate) struct LifeCourseGuard {
    person_rows: Vec<(String, usize)>,
    life_course: Vec<(String, Column)>,
}

impl LifeCourseGuard {
    /// Whether the step owns no ledger column, so there is nothing to check.
    pub(crate) fn is_empty(&self) -> bool {
        self.person_rows.is_empty() && self.life_course.is_empty()
    }

    /// Checks the step's ledger columns after it ran.
    ///
    /// # Errors
    /// [`SimError::Ledger`] naming `system` and the column: a changed `person.*` row count, a
    /// removed life-course row, a changed entry field, or an exit field written twice.
    pub(crate) fn check(&self, system: &'static str, tables: &WorldTables) -> Result<(), SimError> {
        let refuse = |message: String| SimError::Ledger { system, message };
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
        let kinds_after = tables
            .get(LIFE_COURSE_EXIT_KIND)
            .map(Vec::as_slice)
            .unwrap_or_default();
        // A job that does not own exit_kind cannot change it: before is after.
        let kinds_before = self
            .life_course
            .iter()
            .find(|(c, _)| c == LIFE_COURSE_EXIT_KIND)
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
        Ok(())
    }
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
