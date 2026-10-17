// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `env` must catch an
// environment read through a renamed import (SPR-011 A-4). Never compiled; scanned as plane-A code
// by bin/check_all.sh. It holds exactly one forbidden API.

use std::env as e;

pub fn seed() -> Option<String> { e::var("EPOCHREALTIME").ok() }
