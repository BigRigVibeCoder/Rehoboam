//! `AccessSpec`: what a system reads and writes (SPR-005 T-04; `05.1` §2).
//!
//! A system declares, in code, the columns it reads (with the owner it expects and the lag it
//! reads at) and the columns it writes (which must be columns it owns, with their plane).
//! Registration ([`crate::register`]) checks the declarations of all registered systems against
//! each other and against the system specs, with the same single-writer rule as the ownership
//! lint (`rh_registry::ownership`; SPR-005 ruling 3). At run time the step view
//! ([`crate::world::SystemView`]) refuses any read or write the declaration does not list.
//!
//! Lifecycle commands and event types are declared in the specs (`[[commands]]`, `[[emits]]`);
//! they join `AccessSpec` when the queues that carry them exist (`05.1` §2 rule 2), not before
//! (ADR-0033 §5).
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §2 rules 1 and 4; ADR-0024;
//! `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-04, ruling 3.

use core::fmt;

use rh_registry::ownership::Declaration;

/// The plane a column lives in (ADR-0024).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Plane {
    /// Authoritative state.
    A,
    /// Observation (pins, facts ledger, hydrated views).
    B,
    /// Narrative (sift matches, significance, visibility).
    C,
}

impl Plane {
    /// The letter used in the specs (`[[owns]] plane`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }
}

/// When a read sees the column: the `[[reads]] lag` of the specs (`lint_odd.py`).
///
/// `SameInstant` reads see every write made earlier in the same instant, so the scheduler runs
/// the writer first; a writer in a later phase is a registration error. The three lagged kinds
/// read the value the column had at the start of the instant (`05.1` §2 rule 4: "the reader sees
/// the value from the previous instant") and impose no order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lag {
    /// `same_instant`: after the writer, within the instant.
    SameInstant,
    /// `start_of_day`: the value at the start of the instant.
    StartOfDay,
    /// `previous_pulse`: the value at the start of the instant.
    PreviousPulse,
    /// `lagged`: the value at the start of the instant.
    Lagged,
}

impl Lag {
    /// Every lag, in spec order.
    pub const ALL: [Self; 4] = [
        Self::SameInstant,
        Self::StartOfDay,
        Self::PreviousPulse,
        Self::Lagged,
    ];

    /// The spec spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SameInstant => "same_instant",
            Self::StartOfDay => "start_of_day",
            Self::PreviousPulse => "previous_pulse",
            Self::Lagged => "lagged",
        }
    }

    /// The lag with this spec spelling, if any.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.name() == name)
    }

    /// Whether the read sees the start-of-instant value (any lag but `SameInstant`).
    #[must_use]
    pub const fn is_lagged(self) -> bool {
        !matches!(self, Self::SameInstant)
    }
}

/// A declared read: `entity.column`, the system expected to own it, and the lag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnRead {
    /// `entity.column`.
    pub column: &'static str,
    /// The owning system, as the reader's spec names it.
    pub owner: &'static str,
    /// When the read sees the column.
    pub lag: Lag,
}

/// A declared write: an owned column and its plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnWrite {
    /// `entity.column`.
    pub column: &'static str,
    /// The column's plane.
    pub plane: Plane,
}

/// The access one system declares (`05.1` §2: `fn access(&self) -> AccessSpec`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccessSpec {
    /// Columns read, with owner and lag.
    pub reads: Vec<ColumnRead>,
    /// Columns written: owned columns only.
    pub writes: Vec<ColumnWrite>,
}

impl AccessSpec {
    /// The plane of the declaring system, by the rule the lint uses
    /// (`rh_registry::ownership::system_plane`).
    #[must_use]
    pub fn plane(&self) -> &'static str {
        rh_registry::ownership::system_plane(self.writes.iter().map(|w| w.plane.name()))
    }

    /// The declared read of `column`, if any.
    #[must_use]
    pub fn read_of(&self, column: &str) -> Option<&ColumnRead> {
        self.reads.iter().find(|r| r.column == column)
    }

    /// Whether `column` is declared as a write.
    #[must_use]
    pub fn writes_column(&self, column: &str) -> bool {
        self.writes.iter().any(|w| w.column == column)
    }

    /// The view of this declaration the shared single-writer rule takes.
    #[must_use]
    pub fn declaration(&self, system: &str) -> Declaration {
        Declaration {
            system: system.to_owned(),
            writes: self
                .writes
                .iter()
                .map(|w| (w.column.to_owned(), w.plane.name().to_owned()))
                .collect(),
            reads: self
                .reads
                .iter()
                .map(|r| (r.column.to_owned(), r.owner.to_owned()))
                .collect(),
        }
    }

    /// The view of this declaration the spec lint's `Ownership::check` takes.
    #[must_use]
    pub fn lint_access(&self, system: &str) -> rh_registry::ownership::AccessSpec {
        rh_registry::ownership::AccessSpec {
            system: system.to_owned(),
            reads: self.reads.iter().map(|r| r.column.to_owned()).collect(),
            writes: self.writes.iter().map(|w| w.column.to_owned()).collect(),
        }
    }
}

impl fmt::Display for Lag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
