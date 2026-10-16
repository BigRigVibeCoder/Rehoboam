//! Undeclared-plane fixture: scanned as plane A (lint_arch reports the missing metadata).
pub fn now() -> std::time::SystemTime {
    std::time::SystemTime::now()
}
