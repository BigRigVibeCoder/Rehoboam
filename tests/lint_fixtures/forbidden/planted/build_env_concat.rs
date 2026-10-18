// Planted positive control for bin/lint_forbidden.py (BCK-002 A-50; SPR-014 T-07): rule
// `build-env` must catch env!(concat!("CARGO_PKG_", "NAME")), the build machine's environment read into the engine build.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

pub fn built() -> &'static str { env!(concat!("CARGO_PKG_", "NAME")) }
