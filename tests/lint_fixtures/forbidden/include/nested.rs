// Positive control for bin/lint_forbidden.py (BCK-002 A-27; SPR-008 T-00): an include! inside an
// included file is followed too, from that file's directory; its HashMap FAILS. Never compiled.
mod x2 {
    include!("skeletons/nested_skeleton.rs");
}
