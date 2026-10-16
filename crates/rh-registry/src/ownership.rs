//! Single-writer column ownership (SPR-004 T-03; `05.1` §2 rule 1; V-INV-04).
//!
//! The owner of every column is declared in the system specs (`specs/systems/*.odd.toml`,
//! `[[owns]]`, ADR-0033). This module builds the ownership table from them and checks a
//! system's access declaration against it:
//!
//! - `single-writer`: a system writes only columns its own spec owns, and no column has two
//!   owners;
//! - `undeclared-read`: a system reads only columns its spec lists in `[[reads]]`;
//! - `plane`: a plane-A system never reads a plane-B or plane-C column (ADR-0024);
//! - `registry`: every system with a spec exists in the Registry.
//!
//! [`AccessSpec`] is a stub of the `rh-core` type that SPR-005 builds (reads, writes; commands
//! and lags come later). Until systems exist in code, access declarations come from TOML files
//! (`[access]`, used by the positive controls) or are derived from the specs themselves.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// One owned column, from `[[owns]]` (other ODD fields are not needed here).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OwnedColumn {
    /// `entity.column`.
    pub column: String,
    /// The plane the column lives in: `A`, `B` or `C`.
    pub plane: String,
}

/// One declared read, from `[[reads]]`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeclaredRead {
    /// `entity.column`.
    pub column: String,
    /// The system the spec says owns it.
    pub owner: String,
}

#[derive(Debug, Deserialize)]
struct SpecHeader {
    id: String,
}

#[derive(Debug, Deserialize)]
struct SpecFile {
    system: SpecHeader,
    #[serde(default)]
    owns: Vec<OwnedColumn>,
    #[serde(default)]
    reads: Vec<DeclaredRead>,
}

/// The ownership-relevant part of one system spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSpec {
    /// The spec file.
    pub path: PathBuf,
    /// The system id, e.g. `M2.demography`.
    pub id: String,
    /// Columns it owns.
    pub owns: Vec<OwnedColumn>,
    /// Columns it declares it reads.
    pub reads: Vec<DeclaredRead>,
}

impl SystemSpec {
    /// Parses the ownership-relevant part of an ODD-R spec.
    ///
    /// # Errors
    /// The text is not TOML or lacks `[system].id`, or an `[[owns]]`/`[[reads]]` entry lacks
    /// its fields.
    pub fn from_toml(path: &Path, text: &str) -> Result<Self, String> {
        let f: SpecFile = toml::from_str(text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(SystemSpec {
            path: path.to_path_buf(),
            id: f.system.id,
            owns: f.owns,
            reads: f.reads,
        })
    }

    /// The plane of the system: `A` when it owns any plane-A column (or owns nothing).
    #[must_use]
    pub fn plane(&self) -> &str {
        if self.owns.is_empty() || self.owns.iter().any(|o| o.plane == "A") {
            "A"
        } else {
            self.owns[0].plane.as_str()
        }
    }
}

/// Loads every system spec in a directory (`*.odd.toml`, skipping `_`-prefixed templates), in
/// file-name order.
///
/// # Errors
/// The directory or a spec cannot be read or parsed.
pub fn load_specs(dir: &Path) -> Result<Vec<SystemSpec>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".odd.toml") && !n.starts_with('_'))
        })
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|p| {
            let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
            SystemSpec::from_toml(p, &text)
        })
        .collect()
}

/// The system ids of a Registry file: the keys of its `[systems]` table. Reads nothing else, so
/// this plane-A crate never parses a Registry number.
///
/// # Errors
/// The file cannot be read, is not TOML, or has no `[systems]` table.
pub fn registry_system_ids(path: &Path) -> Result<BTreeSet<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let table: toml::Table =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    match table.get("systems") {
        Some(toml::Value::Table(systems)) => Ok(systems.keys().cloned().collect()),
        _ => Err(format!("{}: no [systems] table", path.display())),
    }
}

/// The access a system declares: what it reads and what it writes. A stub of `rh-core`'s
/// `AccessSpec` (SPR-005 adds lifecycle commands, events and lags).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessSpec {
    /// The declaring system.
    pub system: String,
    /// Columns read.
    #[serde(default)]
    pub reads: Vec<String>,
    /// Columns written.
    #[serde(default)]
    pub writes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessFile {
    access: Vec<AccessSpec>,
}

impl AccessSpec {
    /// The access a spec implies: it writes what it owns and reads what it declares.
    #[must_use]
    pub fn from_spec(spec: &SystemSpec) -> Self {
        AccessSpec {
            system: spec.id.clone(),
            reads: spec.reads.iter().map(|r| r.column.clone()).collect(),
            writes: spec.owns.iter().map(|o| o.column.clone()).collect(),
        }
    }

    /// Parses `[[access]]` tables from TOML.
    ///
    /// # Errors
    /// Bad TOML or an unknown field.
    pub fn from_toml(text: &str) -> Result<Vec<Self>, String> {
        toml::from_str::<AccessFile>(text)
            .map(|f| f.access)
            .map_err(|e| e.to_string())
    }
}

/// One broken ownership rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    /// `single-writer`, `undeclared-read`, `plane` or `registry`.
    pub rule: &'static str,
    /// The system that breaks it.
    pub system: String,
    /// What it does.
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VIOLATION [{}] {}: {}",
            self.rule, self.system, self.message
        )
    }
}

/// The owner of one column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner {
    /// The owning system.
    pub system: String,
    /// The column's plane.
    pub plane: String,
}

/// Column ownership from the system specs, with the violations found while building it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ownership {
    /// Column to owner; the first spec (in file order) wins a contested column.
    pub owners: BTreeMap<String, Owner>,
    /// Specs by system id.
    pub specs: BTreeMap<String, SystemSpec>,
    /// Problems in the specs themselves (two owners, a system unknown to the Registry).
    pub problems: Vec<Violation>,
}

impl Ownership {
    /// Builds the table from the specs, checking them against each other and the Registry's
    /// system ids (from `rh-manifest`'s Registry, or [`registry_system_ids`]).
    #[must_use]
    pub fn build(specs: &[SystemSpec], system_ids: &BTreeSet<String>) -> Self {
        let mut owners: BTreeMap<String, Owner> = BTreeMap::new();
        let mut problems = Vec::new();
        let mut by_id = BTreeMap::new();
        for s in specs {
            if !system_ids.contains(&s.id) {
                problems.push(Violation {
                    rule: "registry",
                    system: s.id.clone(),
                    message: format!(
                        "{} declares a system that is not in the Registry",
                        s.path.display()
                    ),
                });
            }
            if by_id.insert(s.id.clone(), s.clone()).is_some() {
                problems.push(Violation {
                    rule: "single-writer",
                    system: s.id.clone(),
                    message: format!("{} is a second spec for this system", s.path.display()),
                });
            }
            for o in &s.owns {
                if let Some(first) = owners.get(&o.column) {
                    problems.push(Violation {
                        rule: "single-writer",
                        system: s.id.clone(),
                        message: format!(
                            "owns {}, which {} already owns (05.1 §2)",
                            o.column, first.system
                        ),
                    });
                } else {
                    owners.insert(
                        o.column.clone(),
                        Owner {
                            system: s.id.clone(),
                            plane: o.plane.clone(),
                        },
                    );
                }
            }
        }
        Ownership {
            owners,
            specs: by_id,
            problems,
        }
    }

    /// Checks one access declaration against the table and the system's own spec.
    #[must_use]
    pub fn check(&self, access: &AccessSpec) -> Vec<Violation> {
        let v = |rule, message: String| Violation {
            rule,
            system: access.system.clone(),
            message,
        };
        let Some(spec) = self.specs.get(&access.system) else {
            return vec![v(
                "registry",
                "has no system spec in specs/systems (ADR-0033)".to_owned(),
            )];
        };
        let mut out = Vec::new();
        for col in &access.writes {
            match self.owners.get(col) {
                Some(o) if o.system == access.system => {}
                Some(o) => out.push(v(
                    "single-writer",
                    format!(
                        "writes {col}, which {} owns; request the change with a lifecycle command (05.1 §2)",
                        o.system
                    ),
                )),
                None => out.push(v("single-writer", format!("writes {col}, which no system spec owns"))),
            }
        }
        let declared: BTreeSet<&str> = spec.reads.iter().map(|r| r.column.as_str()).collect();
        for col in &access.reads {
            let own = spec.owns.iter().any(|o| &o.column == col);
            if !own && !declared.contains(col.as_str()) {
                out.push(v(
                    "undeclared-read",
                    format!("reads {col}, which its spec does not list in [[reads]]"),
                ));
            }
            if spec.plane() == "A"
                && let Some(o) = self.owners.get(col)
                && (o.plane == "B" || o.plane == "C")
            {
                out.push(v(
                    "plane",
                    format!(
                        "a plane-A system reads plane-{} column {col} (ADR-0024)",
                        o.plane
                    ),
                ));
            }
        }
        out
    }

    /// Columns read by some spec whose owner has no spec yet: reported, not violations.
    #[must_use]
    pub fn unresolved_reads(&self) -> Vec<String> {
        let mut out = Vec::new();
        for s in self.specs.values() {
            for r in &s.reads {
                if !self.owners.contains_key(&r.column) {
                    out.push(format!(
                        "{}: reads {} from {}, which has no spec yet",
                        s.id, r.column, r.owner
                    ));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn table() -> Ownership {
        let reg = registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).unwrap();
        let specs = load_specs(&repo().join("specs/systems")).unwrap();
        Ownership::build(&specs, &reg)
    }

    fn access(system: &str, reads: &[&str], writes: &[&str]) -> AccessSpec {
        AccessSpec {
            system: system.to_owned(),
            reads: reads.iter().map(ToString::to_string).collect(),
            writes: writes.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn the_specs_agree_with_themselves() {
        let t = table();
        assert_eq!(t.problems, vec![]);
        assert_eq!(t.owners["person.alive"].system, "M2.demography");
        assert_eq!(t.owners["encounter_log.day_index"].system, "S5.daily_life");
        for spec in t.specs.values() {
            assert_eq!(t.check(&AccessSpec::from_spec(spec)), vec![], "{}", spec.id);
        }
    }

    /// T-03 acceptance: a system writing a column it does not own is rejected.
    #[test]
    fn writing_an_unowned_column_is_rejected() {
        let t = table();
        let found = t.check(&access(
            "S5.daily_life",
            &[],
            &["encounter_log.day_index", "person.alive"],
        ));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, "single-writer");
        assert!(
            found[0]
                .message
                .starts_with("writes person.alive, which M2.demography owns")
        );
        let nobody = t.check(&access("M2.demography", &[], &["person.mood"]));
        assert_eq!(
            nobody[0].message,
            "writes person.mood, which no system spec owns"
        );
    }

    #[test]
    fn undeclared_reads_and_unknown_systems_are_rejected() {
        let t = table();
        let found = t.check(&access("M2.demography", &["person.residence"], &[]));
        assert_eq!(found[0].rule, "undeclared-read");
        assert_eq!(t.check(&access("M9.ghost", &[], &[]))[0].rule, "registry");
    }

    #[test]
    fn two_owners_and_plane_leaks_are_found() {
        let reg = registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).unwrap();
        let spec = |id: &str, col: &str, plane: &str, reads: &[&str]| SystemSpec {
            path: PathBuf::from(format!("{id}.odd.toml")),
            id: id.to_owned(),
            owns: vec![OwnedColumn {
                column: col.to_owned(),
                plane: plane.to_owned(),
            }],
            reads: reads
                .iter()
                .map(|c| DeclaredRead {
                    column: (*c).to_owned(),
                    owner: "x".to_owned(),
                })
                .collect(),
        };
        let t = Ownership::build(
            &[
                spec("M1.regions", "region.pins", "B", &[]),
                spec("M2.demography", "person.alive", "A", &["region.pins"]),
                spec("M3.agriculture", "person.alive", "A", &[]),
            ],
            &reg,
        );
        assert_eq!(t.problems.len(), 1);
        assert!(
            t.problems[0]
                .message
                .contains("which M2.demography already owns")
        );
        let leak = t.check(&access("M2.demography", &["region.pins"], &[]));
        assert_eq!(leak[0].rule, "plane");
    }

    #[test]
    fn access_files_parse() {
        let a = AccessSpec::from_toml(
            "[[access]]\nsystem = \"S5.daily_life\"\nwrites = [\"person.alive\"]\n",
        )
        .unwrap();
        assert_eq!(a[0].writes, vec!["person.alive".to_owned()]);
        assert!(AccessSpec::from_toml("[[access]]\nsystem = \"x\"\nowns = []\n").is_err());
    }
}
