//! The host seam (Q9): what the evaluator reads and draws through.
//!
//! The evaluator holds no world. A host gives it three things:
//! - [`Reads`]: a column of an entity (a row's value, missing, or an absent owner's neutral), and
//!   whether a row exists. A life-course column is read with the person, never a row id (M7): the
//!   host finds the row through its index.
//! - [`Indexes`]: the derived indexes the prelude needs: children by parent over the life-course
//!   log, and the inverse lookups (`titles`, `offices`, `court`, `claims_*`, `vows_of`, a title's
//!   tenure rows), each with a scan default in ascending id.
//! - [`Draws`]: the keyed draw of a purpose, an entity and an index at the host's instant.
//!
//! Plus the calendar and the declaring system's parameter values ([`Host`]).
//!
//! REF: `specs/registry/effect_language_v0.md` §4.5, §4.6, §5.1, §12 Q9; SPR-017 T-07.

use super::env::{ColumnId, KindId};
use super::value::Value;

/// The three outcomes of a read (§4.5, NEW-4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadOutcome {
    /// The row's value.
    Row(Value),
    /// No such row (or `none`): the evaluator yields the type's zero and counts `missing_reads`.
    Missing,
    /// The owner is not registered: the coupling's neutral, counted as `neutral_reads`.
    Neutral(Value),
}

/// Column reads through the host's view.
pub trait Reads {
    /// Reads `column` of the entity `id`; for a life-course column, `id` is the person.
    fn read(&self, column: ColumnId, id: u64) -> ReadOutcome;
    /// Whether a row of `kind` with this id exists in the snapshot (`exists`, §3.1).
    fn exists(&self, kind: KindId, id: u64) -> bool;
    /// Every id of `kind`, ascending (the scan behind the inverse lookups).
    fn ids(&self, kind: KindId) -> Vec<u64>;
}

/// The derived indexes of §5.1.
pub trait Indexes: Reads {
    /// Every person whose mother or father is `person` in the life-course log, alive or dead,
    /// ascending and each once.
    fn children(&self, person: u64) -> Vec<u64>;

    /// The ids of `kind` whose entity column `column` references `target`, ascending. The
    /// default scans every row; a host with an index overrides it, and must agree with the scan.
    fn rows_where(&self, kind: KindId, column: ColumnId, target: u64) -> Vec<u64> {
        self.ids(kind)
            .into_iter()
            .filter(|&id| matches!(self.read(column, id), ReadOutcome::Row(Value::Ent(_, t)) if t == target))
            .collect()
    }

    /// A title's tenure rows in log order (ascending row id): `rows_where` over `tenure.title`.
    fn tenure_rows(&self, tenure: KindId, tenure_title: ColumnId, title: u64) -> Vec<u64> {
        self.rows_where(tenure, tenure_title, title)
    }
}

/// The keyed RNG (§4.6): `H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)`
/// with `t` the host's instant.
pub trait Draws {
    /// The 64-bit draw of `purpose` keyed on the entity `(kind, id)` and the index `k` (an `int`
    /// index as its two's-complement bits, an entity index as its id); `None` if the host has no
    /// draw for the key (a fixture world that lists its draws).
    fn draw(&self, purpose: &str, kind: KindId, id: u64, k: u64) -> Option<u64>;
}

/// Everything an evaluation reads.
pub trait Host: Indexes + Draws {
    /// The value of a Registry parameter of `system` (`int` or `q16_16`), if the world has it.
    fn param(&self, system: &str, name: &str) -> Option<Value>;
    /// `calendar.days_per_year`: the manifest's year.
    fn days_per_year(&self) -> i64;
}
