//! Fixture: a test reading its golden-file override (exempt from the env rule).

#[test]
fn golden() { let _ = std::env::var_os("RH_GOLDEN"); }
