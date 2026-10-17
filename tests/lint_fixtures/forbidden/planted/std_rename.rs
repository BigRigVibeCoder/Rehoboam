// Planted positive control for bin/lint_forbidden.py (V-DET-19): rule `std-rename` must catch a
// renamed std, which would carry std::process past every path pattern (MIN-2). Never compiled;
// scanned as plane-A code by bin/check_all.sh. It holds exactly one forbidden API.

use std as s;
