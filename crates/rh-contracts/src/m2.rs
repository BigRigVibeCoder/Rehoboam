//! `M2.demography`'s columns (`specs/systems/M2.demography.odd.toml` 0.2.3, `[[owns]]`), as typed
//! read accessors (SPR-014 T-06). No M2 logic: each accessor is one keyed lookup through the
//! caller's view, decoded to the spec's type.
//!
//! A row of `person.*` is a ledger row (sorted by `person.id`; find it with
//! [`rh_core::world::SystemView::person_row`]); a row of `life_course.*` is a log row. Every
//! accessor returns `Ok(None)` for a row the column does not have.

use rh_core::system::SimError;
use rh_core::time::SimTime;
use rh_core::world::SystemView;
use rh_ids::PersonId;

/// The owning system.
pub const OWNER: &str = "M2.demography";

/// A column's type, as the owner's spec declares it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnType {
    /// A `PersonId` (`u64` in the spec), 0 for none.
    Person,
    /// A `SimTime` in minutes (`i64` in the spec).
    Time,
    /// `bool`.
    Bool,
    /// An enum with these code names, code `i` the `i`-th.
    Enum(&'static [&'static str]),
}

/// `person.sex` and `life_course.sex` codes.
pub const SEX: &[&str] = &["male", "female"];
/// `person.death_cause` and `life_course.death_cause` codes.
pub const DEATH_CAUSE: &[&str] = &[
    "none",
    "natural",
    "murder",
    "battle",
    "execution",
    "accident",
];
/// `life_course.origin` codes.
pub const ORIGIN: &[&str] = &["born", "founder", "immigrant"];
/// `life_course.exit_kind` codes.
pub const EXIT_KIND: &[&str] = &["open", "death", "emigration"];

/// Every column this module reads, in the spec's `[[owns]]` order, with its type: what
/// `tests/drift.rs` compares with the spec.
pub const COLUMNS: [(&str, ColumnType); 19] = [
    ("person.id", ColumnType::Person),
    ("person.birth_tick", ColumnType::Time),
    ("person.sex", ColumnType::Enum(SEX)),
    ("person.alive", ColumnType::Bool),
    ("person.death_tick", ColumnType::Time),
    ("person.mother", ColumnType::Person),
    ("person.father", ColumnType::Person),
    ("person.spouse", ColumnType::Person),
    ("person.death_cause", ColumnType::Enum(DEATH_CAUSE)),
    ("life_course.person_id", ColumnType::Person),
    ("life_course.origin", ColumnType::Enum(ORIGIN)),
    ("life_course.mother", ColumnType::Person),
    ("life_course.father", ColumnType::Person),
    ("life_course.sex", ColumnType::Enum(SEX)),
    ("life_course.birth_tick", ColumnType::Time),
    ("life_course.entry_tick", ColumnType::Time),
    ("life_course.exit_kind", ColumnType::Enum(EXIT_KIND)),
    ("life_course.exit_tick", ColumnType::Time),
    ("life_course.death_cause", ColumnType::Enum(DEATH_CAUSE)),
];

/// A person's sex (`values = ["male", "female"]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sex {
    /// Code 0.
    Male,
    /// Code 1.
    Female,
}

/// How a person died (`values = ["none", "natural", "murder", "battle", "execution", "accident"]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeathCause {
    /// Code 0: alive, or not a death.
    None,
    /// Code 1.
    Natural,
    /// Code 2.
    Murder,
    /// Code 3.
    Battle,
    /// Code 4.
    Execution,
    /// Code 5.
    Accident,
}

/// How a person entered the log (`values = ["born", "founder", "immigrant"]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Origin {
    /// Code 0.
    Born,
    /// Code 1.
    Founder,
    /// Code 2.
    Immigrant,
}

/// How a person left the ledger (`values = ["open", "death", "emigration"]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExitKind {
    /// Code 0: still in the ledger.
    Open,
    /// Code 1.
    Death,
    /// Code 2.
    Emigration,
}

fn outside(column: &str, code: i64) -> SimError {
    SimError::System {
        system: "rh-contracts",
        message: format!("{column} holds code {code}, outside its spec's values ({OWNER})"),
    }
}

fn raw(view: &SystemView<'_>, column: &str, row: usize) -> Result<Option<i64>, SimError> {
    view.value(column, row)
}

fn person(view: &SystemView<'_>, column: &str, row: usize) -> Result<Option<PersonId>, SimError> {
    Ok(raw(view, column, row)?.map(|v| PersonId::from_raw(v.cast_unsigned())))
}

fn time(view: &SystemView<'_>, column: &str, row: usize) -> Result<Option<SimTime>, SimError> {
    Ok(raw(view, column, row)?.map(SimTime::from_minutes))
}

fn code<T: Copy>(
    view: &SystemView<'_>,
    column: &str,
    row: usize,
    values: &[T],
) -> Result<Option<T>, SimError> {
    raw(view, column, row)?
        .map(|v| {
            usize::try_from(v)
                .ok()
                .and_then(|i| values.get(i).copied())
                .ok_or_else(|| outside(column, v))
        })
        .transpose()
}

const SEXES: [Sex; 2] = [Sex::Male, Sex::Female];
const CAUSES: [DeathCause; 6] = [
    DeathCause::None,
    DeathCause::Natural,
    DeathCause::Murder,
    DeathCause::Battle,
    DeathCause::Execution,
    DeathCause::Accident,
];
const ORIGINS: [Origin; 3] = [Origin::Born, Origin::Founder, Origin::Immigrant];
const EXITS: [ExitKind; 3] = [ExitKind::Open, ExitKind::Death, ExitKind::Emigration];

macro_rules! accessors {
    ($($(#[$doc:meta])* $name:ident: $column:literal => $ty:ty, |$v:ident, $c:ident, $r:ident| $body:expr;)*) => {
        $(
            $(#[$doc])*
            ///
            /// # Errors
            /// As [`SystemView::value`]; an enum code outside the spec's values.
            pub fn $name($v: &SystemView<'_>, $r: usize) -> Result<Option<$ty>, SimError> {
                let $c = $column;
                $body
            }
        )*
    };
}

accessors! {
    /// `person.id`: the ledger key.
    person_id: "person.id" => PersonId, |v, c, r| person(v, c, r);
    /// `person.birth_tick`.
    birth_tick: "person.birth_tick" => SimTime, |v, c, r| time(v, c, r);
    /// `person.sex`.
    sex: "person.sex" => Sex, |v, c, r| code(v, c, r, &SEXES);
    /// `person.alive`: true on every row but the dead of the current instant.
    alive: "person.alive" => bool, |v, c, r| Ok(raw(v, c, r)?.map(|a| a != 0));
    /// `person.death_tick`: `i64::MAX` while alive.
    death_tick: "person.death_tick" => SimTime, |v, c, r| time(v, c, r);
    /// `person.mother`, 0 for none.
    mother: "person.mother" => PersonId, |v, c, r| person(v, c, r);
    /// `person.father`, 0 for none.
    father: "person.father" => PersonId, |v, c, r| person(v, c, r);
    /// `person.spouse`, 0 for none.
    spouse: "person.spouse" => PersonId, |v, c, r| person(v, c, r);
    /// `person.death_cause`: none while alive.
    death_cause: "person.death_cause" => DeathCause, |v, c, r| code(v, c, r, &CAUSES);
    /// `life_course.person_id`.
    lc_person_id: "life_course.person_id" => PersonId, |v, c, r| person(v, c, r);
    /// `life_course.origin`.
    lc_origin: "life_course.origin" => Origin, |v, c, r| code(v, c, r, &ORIGINS);
    /// `life_course.mother`.
    lc_mother: "life_course.mother" => PersonId, |v, c, r| person(v, c, r);
    /// `life_course.father`.
    lc_father: "life_course.father" => PersonId, |v, c, r| person(v, c, r);
    /// `life_course.sex` (REV-P1a NEW-7).
    lc_sex: "life_course.sex" => Sex, |v, c, r| code(v, c, r, &SEXES);
    /// `life_course.birth_tick` (REV-P1a NEW-7).
    lc_birth_tick: "life_course.birth_tick" => SimTime, |v, c, r| time(v, c, r);
    /// `life_course.entry_tick`.
    lc_entry_tick: "life_course.entry_tick" => SimTime, |v, c, r| time(v, c, r);
    /// `life_course.exit_kind`.
    lc_exit_kind: "life_course.exit_kind" => ExitKind, |v, c, r| code(v, c, r, &EXITS);
    /// `life_course.exit_tick`: `i64::MAX` while open.
    lc_exit_tick: "life_course.exit_tick" => SimTime, |v, c, r| time(v, c, r);
    /// `life_course.death_cause`.
    lc_death_cause: "life_course.death_cause" => DeathCause, |v, c, r| code(v, c, r, &CAUSES);
}

#[cfg(test)]
mod tests {
    use super::{CAUSES, COLUMNS, DEATH_CAUSE, EXIT_KIND, EXITS, ORIGIN, ORIGINS, SEX, SEXES};

    #[test]
    fn the_enums_follow_their_code_lists() {
        assert_eq!(SEXES.len(), SEX.len());
        assert_eq!(CAUSES.len(), DEATH_CAUSE.len());
        assert_eq!(ORIGINS.len(), ORIGIN.len());
        assert_eq!(EXITS.len(), EXIT_KIND.len());
        let mut names: Vec<&str> = COLUMNS.iter().map(|(c, _)| *c).collect();
        names.dedup();
        assert_eq!(names.len(), COLUMNS.len(), "each column once");
    }
}
