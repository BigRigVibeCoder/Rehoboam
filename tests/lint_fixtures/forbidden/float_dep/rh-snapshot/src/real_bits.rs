//! The Tester's first snippet: a REAL column's bits laundered into an integer.

use rusqlite::types::ValueRef;

pub fn bits(v: ValueRef<'_>) -> u64 {
    match v {
        ValueRef::Real(x) => x.to_bits(),
        _ => 0,
    }
}
