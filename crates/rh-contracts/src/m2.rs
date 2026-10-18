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

/// A decoded value of any accessor, for code that walks [`ACCESSORS`] (the consumer contract
/// test): an enum is its code name, as the spec writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    /// A `PersonId`.
    Person(PersonId),
    /// A `SimTime`.
    Time(SimTime),
    /// A `bool`.
    Bool(bool),
    /// An enum code, by its name in the spec's `values`.
    Code(&'static str),
}

impl From<PersonId> for Value {
    fn from(p: PersonId) -> Self {
        Self::Person(p)
    }
}

impl From<SimTime> for Value {
    fn from(t: SimTime) -> Self {
        Self::Time(t)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

/// An accessor seen through [`Value`]: each entry of [`ACCESSORS`] calls the named accessor.
pub type Accessor = fn(&SystemView<'_>, usize) -> Result<Option<Value>, SimError>;

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

macro_rules! code_names {
    ($($ty:ident { $($variant:ident => $name:literal),* $(,)? })*) => {
        $(
            impl $ty {
                /// The code's name in the spec's `values`.
                #[must_use]
                pub const fn name(self) -> &'static str {
                    match self {
                        $(Self::$variant => $name,)*
                    }
                }
            }

            impl From<$ty> for Value {
                fn from(c: $ty) -> Self {
                    Self::Code(c.name())
                }
            }
        )*
    };
}

code_names! {
    Sex { Male => "male", Female => "female" }
    DeathCause {
        None => "none",
        Natural => "natural",
        Murder => "murder",
        Battle => "battle",
        Execution => "execution",
        Accident => "accident",
    }
    Origin { Born => "born", Founder => "founder", Immigrant => "immigrant" }
    ExitKind { Open => "open", Death => "death", Emigration => "emigration" }
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

fn boolean(view: &SystemView<'_>, column: &str, row: usize) -> Result<Option<bool>, SimError> {
    Ok(raw(view, column, row)?.map(|a| a != 0))
}

// One list makes the accessors, `COLUMNS` and `ACCESSORS`, so the table the drift test reads
// cannot name a column, a type or a decode the accessor does not use (the Tester's DEF-1).
macro_rules! decode {
    (fn person) => {
        PersonId
    };
    (fn time) => {
        SimTime
    };
    (fn boolean) => {
        bool
    };
    (fn enum $ty:ident) => {
        $ty
    };
    (call person $v:ident $c:ident $r:ident) => {
        person($v, $c, $r)
    };
    (call time $v:ident $c:ident $r:ident) => {
        time($v, $c, $r)
    };
    (call boolean $v:ident $c:ident $r:ident) => {
        boolean($v, $c, $r)
    };
    (call enum $ty:ident $codes:ident $v:ident $c:ident $r:ident) => {
        code($v, $c, $r, &$codes)
    };
    (type person) => {
        ColumnType::Person
    };
    (type time) => {
        ColumnType::Time
    };
    (type boolean) => {
        ColumnType::Bool
    };
    (type enum $ty:ident $codes:ident $names:ident) => {
        ColumnType::Enum($names)
    };
}

macro_rules! accessors {
    ($($(#[$doc:meta])* $name:ident: $column:literal => $kind:ident $(($ty:ident, $codes:ident, $names:ident))?;)*) => {
        $(
            $(#[$doc])*
            ///
            /// # Errors
            /// As [`SystemView::value`]; an enum code outside the spec's values.
            pub fn $name(view: &SystemView<'_>, row: usize) -> Result<Option<decode!(fn $kind $($ty)?)>, SimError> {
                let column = $column;
                decode!(call $kind $($ty $codes)? view column row)
            }
        )*

        /// Every column this module reads, in the spec's `[[owns]]` order, with its type: what
        /// `tests/drift.rs` compares with the spec. Made from the accessors' own list.
        pub const COLUMNS: &[(&str, ColumnType)] = &[$(($column, decode!(type $kind $($ty $codes $names)?)),)*];

        /// Every accessor with its column, in [`COLUMNS`]' order, seen through [`Value`]: what
        /// `tests/consumer.rs` calls on seeded rows.
        pub const ACCESSORS: &[(&str, Accessor)] = &[$(($column, |v, r| $name(v, r).map(|o| o.map(Value::from))),)*];
    };
}

accessors! {
    /// `person.id`: the ledger key.
    person_id: "person.id" => person;
    /// `person.birth_tick`.
    birth_tick: "person.birth_tick" => time;
    /// `person.sex`.
    sex: "person.sex" => enum(Sex, SEXES, SEX);
    /// `person.alive`: true on every row but the dead of the current instant.
    alive: "person.alive" => boolean;
    /// `person.death_tick`: `i64::MAX` while alive.
    death_tick: "person.death_tick" => time;
    /// `person.mother`, 0 for none.
    mother: "person.mother" => person;
    /// `person.father`, 0 for none.
    father: "person.father" => person;
    /// `person.spouse`, 0 for none.
    spouse: "person.spouse" => person;
    /// `person.death_cause`: none while alive.
    death_cause: "person.death_cause" => enum(DeathCause, CAUSES, DEATH_CAUSE);
    /// `life_course.person_id`.
    lc_person_id: "life_course.person_id" => person;
    /// `life_course.origin`.
    lc_origin: "life_course.origin" => enum(Origin, ORIGINS, ORIGIN);
    /// `life_course.mother`.
    lc_mother: "life_course.mother" => person;
    /// `life_course.father`.
    lc_father: "life_course.father" => person;
    /// `life_course.sex` (REV-P1a NEW-7).
    lc_sex: "life_course.sex" => enum(Sex, SEXES, SEX);
    /// `life_course.birth_tick` (REV-P1a NEW-7).
    lc_birth_tick: "life_course.birth_tick" => time;
    /// `life_course.entry_tick`.
    lc_entry_tick: "life_course.entry_tick" => time;
    /// `life_course.exit_kind`.
    lc_exit_kind: "life_course.exit_kind" => enum(ExitKind, EXITS, EXIT_KIND);
    /// `life_course.exit_tick`: `i64::MAX` while open.
    lc_exit_tick: "life_course.exit_tick" => time;
    /// `life_course.death_cause`.
    lc_death_cause: "life_course.death_cause" => enum(DeathCause, CAUSES, DEATH_CAUSE);
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
