//! Test helper (BCK-002 A-51): read the system specs and a Registry's system ids from files, then
//! hand the text to `rh-registry`'s pure parsers. Plane-A `src/` never reads files; `tests/` may.
#![allow(dead_code, reason = "each test binary uses some of the helpers")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rh_registry::ownership::{SystemSpec, parse_registry_system_ids, parse_specs};

/// Every system spec in `dir` (`*.odd.toml`, no `_` templates), in file-name order.
///
/// # Errors
/// The directory or a spec cannot be read or parsed.
pub fn load_specs(dir: &Path) -> Result<Vec<SystemSpec>, String> {
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|x| x == "toml") {
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            files.push((path, text));
        }
    }
    parse_specs(files)
}

/// The system ids of the Registry file at `path`.
///
/// # Errors
/// The file cannot be read or has no `[systems]` table.
pub fn registry_system_ids(path: &Path) -> Result<BTreeSet<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_registry_system_ids(&path.display().to_string(), &text)
}
