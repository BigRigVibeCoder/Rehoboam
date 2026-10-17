// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `file-io` must catch std::fs
// renamed inside a nested group, `use std::{io, fs::{self as g}}` (DEF-1). Never compiled; scanned
// as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

use std::{io, fs::{self as g}};

pub fn seed() -> io::Result<Vec<u8>> { g::read("seed") }
