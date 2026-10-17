// Positive control for bin/lint_forbidden.py, rule file-io (DEF-1 test gap): after a grouped
// import, the path `fs::read` itself must be reported, on line 5, besides the import on line 4.
// Never compiled; scanned as plane-A code by bin/check_all.sh.
use std::{fs, io};
fn t() { let _ = fs::read("x"); }
