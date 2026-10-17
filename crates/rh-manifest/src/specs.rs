//! The host's spec loaders (BCK-002 A-51): read the system specs and a Registry's system ids
//! from files, and hand the text to `rh-registry`'s pure parsers. A tool crate, so plane A never
//! reads a spec file; `rh-cli ownership` and the golden runs call these at load.
//!
//! REF: `CODEX/05_PROJECT/BCK-002_ArchitectBacklog.md` A-51; `VER-SPR-012` MAJ-2.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rh_registry::ownership::{SystemSpec, parse_registry_system_ids, parse_specs};

/// Every system spec in `dir` (`*.odd.toml`, skipping `_`-prefixed templates), in file-name
/// order.
///
/// # Errors
/// The directory or a spec cannot be read or parsed.
pub fn load_specs(dir: &Path) -> Result<Vec<SystemSpec>, String> {
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        let wanted = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(rh_registry::ownership::is_spec_file);
        if wanted {
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            files.push((path, text));
        }
    }
    parse_specs(files)
}

/// The system ids of the Registry file at `path`: the keys of its `[systems]` table.
///
/// # Errors
/// The file cannot be read, is not TOML, or has no `[systems]` table.
pub fn registry_system_ids(path: &Path) -> Result<BTreeSet<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_registry_system_ids(&path.display().to_string(), &text)
}
