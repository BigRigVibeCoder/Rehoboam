//! The Tester's second snippet: a threshold decided by comparing two REAL values.

use rusqlite::types::ValueRef;

pub fn less(a: ValueRef<'_>, b: ValueRef<'_>) -> bool {
    matches!((a, b), (ValueRef::Real(x), ValueRef::Real(y)) if x < y)
}
