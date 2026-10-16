// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `wall-clock` must catch `time::Instant`.
// Never compiled; scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

// DEF-4: a path spaced around `::`, inside a macro body.
macro_rules! stamp {
    () => {
        std :: time :: Instant :: now()
    };
}
