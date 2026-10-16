// Negative control for bin/lint_forbidden.py (BCK-002 A-27; SPR-008 T-00): a literal, in-repository
// include! of a clean skeleton is followed, scanned and passes. Never compiled.
mod x0 {
    include!("skeletons/clean_skeleton.rs");
}
