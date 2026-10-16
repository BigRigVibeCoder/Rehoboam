// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `lint-allow` must catch an allow
// of clippy::disallowed_methods (VER-SPR-003-TEST GAP-5). Never compiled; scanned as plane-A code by
// bin/check_all.sh. It holds exactly one forbidden API: the allow, which silences clippy's ban on the
// std `ln` below without a trace.

#[allow(clippy::disallowed_methods, reason = "a reason clippy accepts, but no one counts")]
pub fn log_density(x: Fraction) -> Fraction {
    x.ln()
}
