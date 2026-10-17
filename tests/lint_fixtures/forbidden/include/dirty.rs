// Positive control for bin/lint_forbidden.py (BCK-002 A-27; SPR-008 T-00): a literal, in-repository
// include! is followed, and the HashMap in the included skeleton FAILS at the skeleton's own line.
// Never compiled.
mod x1 {
    include!("skeletons/dirty_skeleton.rs");
}
