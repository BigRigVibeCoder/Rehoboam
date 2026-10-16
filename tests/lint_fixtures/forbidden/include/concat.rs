// Positive control for bin/lint_forbidden.py (BCK-002 A-27; SPR-008 T-00): a non-literal include!
// argument is never followed and stays forbidden, even when it would name a clean file. Never
// compiled.
mod x3 {
    include!(concat!("skeletons/", "clean_skeleton.rs"));
}
