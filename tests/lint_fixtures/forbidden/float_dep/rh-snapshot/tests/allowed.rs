//! Not reported: the float rules skip tests/ (lint_forbidden's DECISION), so a test may read a REAL
//! to prove the journal refuses one.

fn _probe(v: rusqlite::types::ValueRef<'_>) -> bool {
    matches!(v, rusqlite::types::ValueRef::Real(_))
}
