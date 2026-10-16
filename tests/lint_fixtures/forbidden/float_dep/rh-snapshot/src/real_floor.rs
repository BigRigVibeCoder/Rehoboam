//! The Tester's third snippet: float arithmetic on two REAL values, no float type named.

use rusqlite::types::ValueRef;

pub fn floor_of_max(a: ValueRef<'_>, b: ValueRef<'_>) -> Option<i64> {
    match (a, b) {
        (ValueRef::Real(x), ValueRef::Real(y)) => Some(x.max(y).floor() as i64),
        _ => None,
    }
}
