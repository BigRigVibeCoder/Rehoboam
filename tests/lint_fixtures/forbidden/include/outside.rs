// Positive control for bin/lint_forbidden.py (BCK-002 A-27; SPR-008 T-00): a literal include! that
// resolves outside the repository is never followed and stays forbidden. Never compiled.
mod x4 {
    include!("../../../../../../../../../../../../../../../../etc/hostname");
}
