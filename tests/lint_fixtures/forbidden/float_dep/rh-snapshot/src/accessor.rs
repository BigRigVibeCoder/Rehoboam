//! The accessor: a nullable REAL read through `as_f64_or_null`, which the float rule's `_f64` suffix
//! pattern does not match on its own.

use rusqlite::types::ValueRef;

pub fn rank(v: ValueRef<'_>) -> Option<i64> {
    v.as_f64_or_null().ok().flatten().map(|x| x.round() as i64)
}
