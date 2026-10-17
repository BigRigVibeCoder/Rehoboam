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
//! The rules themselves (single writer, plane, the access check) are `rh_core::ownership`, which
//! registration in `rh-core` also runs (SPR-005 ruling 3); this module builds their inputs from the
//! spec files. [`AccessSpec`] here is the lint's file form of an access declaration (`[[access]]`,
//! used by the positive controls); systems in code declare `rh_core::access::AccessSpec`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rh_core::ownership::SpecEntry;
use serde::Deserialize;

// The single-writer and plane rules live in rh-core, which this crate depends on (SPR-005,
// Architect ruling on ruling 3); they are re-exported so the lint's API is unchanged.
pub use rh_core::ownership::{
    Declaration, Owner, Source, Violation, duplicates, plane_leak, single_writer, system_plane,
};

/// One owned column, from `[[owns]]` (other ODD fields are not needed here).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OwnedColumn {
    /// `entity.column`.
    pub column: String,
    /// The plane the column lives in: `A`, `B` or `C` (anything else is a violation).
    pub plane: String,
    /// The column's storage type, one of [`COLUMN_TYPES`].
    #[serde(rename = "type")]
    pub ty: String,
}

/// The planes a column may live in (ADR-0024).
pub const PLANES: [&str; 3] = ["A", "B", "C"];
/// The column types an ODD-R spec may declare (`specs/systems/lint_odd.py` `TYPES`).
pub const COLUMN_TYPES: [&str; 12] = [
    "i64",
    "u64",
    "i32",
    "u32",
    "u16",
    "u8",
    "bool",
    "enum",
    "q8_8",
    "q16_16",
    "f32_field",
    "event_ref",
];

/// One declared read, from `[[reads]]`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeclaredRead {
    /// `entity.column`.
    pub column: String,
    /// The system the spec says owns it.
    pub owner: String,
    /// When the read sees the column (`same_instant`, `start_of_day`, `previous_pulse`,
    /// `lagged`; `05.1` §2 rule 4). Registration compares it with the code's lag (SPR-005 G-a).
    #[serde(default)]
    pub lag: String,
}

/// One `[[schedule]]` binding (SPR-005 G-a).
#[derive(Debug, Deserialize)]
struct DeclaredBinding {
    pulse: String,
    phase: String,
}

#[derive(Debug, Deserialize)]
struct SpecHeader {
    id: String,
}

/// One keyed RNG purpose, from `[[rng]]` (SPR-005 T-06; ruling 2).
#[derive(Debug, Deserialize)]
struct DeclaredRng {
    purpose: String,
}

/// One lifecycle command submission, from `[[commands]]` (SPR-006).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeclaredCommand {
    /// The owning system the spec names.
    pub target: String,
    /// The command's Registry name.
    pub command: String,
}

/// One emitted event kind, from `[[emits]]` (SPR-006).
#[derive(Debug, Deserialize)]
struct DeclaredEmit {
    kind: String,
}

#[derive(Debug, Deserialize)]
struct SpecFile {
    system: SpecHeader,
    #[serde(default)]
    owns: Vec<OwnedColumn>,
    #[serde(default)]
    reads: Vec<DeclaredRead>,
    #[serde(default)]
    rng: Vec<DeclaredRng>,
    #[serde(default)]
    schedule: Vec<DeclaredBinding>,
    #[serde(default)]
    commands: Vec<DeclaredCommand>,
    #[serde(default)]
    emits: Vec<DeclaredEmit>,
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
    /// Its keyed RNG purposes (`[[rng]] purpose`), in spec order. Registration in `rh-core`
    /// checks a system's purposes against them (SPR-005 T-06).
    pub rng: Vec<String>,
    /// Its `[[schedule]]` bindings as `(pulse, phase)` names, in spec order. Registration compares
    /// them with the code's bindings (SPR-005 G-a).
    pub schedule: Vec<(String, String)>,
    /// The lifecycle commands it submits (`[[commands]]`), in spec order (SPR-006).
    pub commands: Vec<DeclaredCommand>,
    /// The event kinds it emits (`[[emits]] kind`), in spec order (SPR-006).
    pub emits: Vec<String>,
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
            rng: f.rng.into_iter().map(|r| r.purpose).collect(),
            schedule: f.schedule.into_iter().map(|b| (b.pulse, b.phase)).collect(),
            commands: f.commands,
            emits: f.emits.into_iter().map(|e| e.kind).collect(),
        })
    }

    /// The plane of the system: `A` when it owns any plane-A column (or owns nothing).
    #[must_use]
    pub fn plane(&self) -> &str {
        system_plane(self.owns.iter().map(|o| o.plane.as_str()))
    }

    /// The declaration this spec implies for the single-writer rule: it writes what it owns and
    /// reads what it lists.
    #[must_use]
    pub fn declaration(&self) -> Declaration {
        Declaration {
            system: self.id.clone(),
            writes: self
                .owns
                .iter()
                .map(|o| (o.column.clone(), o.plane.clone()))
                .collect(),
            reads: self
                .reads
                .iter()
                .map(|r| (r.column.clone(), r.owner.clone()))
                .collect(),
        }
    }

    /// What `rh-core`'s registration needs from this spec (owned columns, declared reads, RNG
    /// purposes, commands and event kinds), as data: `rh-core` reads no files.
    #[must_use]
    pub fn entry(&self) -> SpecEntry {
        SpecEntry {
            owns: self
                .owns
                .iter()
                .map(|o| (o.column.clone(), o.plane.clone()))
                .collect(),
            reads: self.reads.iter().map(|r| r.column.clone()).collect(),
            rng: self.rng.clone(),
            schedule: self.schedule.clone(),
            lags: self
                .reads
                .iter()
                .map(|r| (r.column.clone(), r.lag.clone()))
                .collect(),
            commands: self
                .commands
                .iter()
                .map(|c| (c.target.clone(), c.command.clone()))
                .collect(),
            emits: self.emits.clone(),
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
                // The lint validates what it relies on itself (SPR-004 DEF-8): an unknown plane
                // would otherwise read as "not B or C" and pass the plane rule (fail-open).
                if !PLANES.contains(&o.plane.as_str()) {
                    problems.push(Violation {
                        rule: "plane",
                        system: s.id.clone(),
                        message: format!(
                            "owns {} in plane '{}'; a column needs plane A, B or C (ADR-0024)",
                            o.column, o.plane
                        ),
                    });
                }
                if !COLUMN_TYPES.contains(&o.ty.as_str()) {
                    problems.push(Violation {
                        rule: "spec",
                        system: s.id.clone(),
                        message: format!("owns {} with unknown type '{}'", o.column, o.ty),
                    });
                }
            }
        }
        // Two owners, and reads that name the wrong owner or a column the named owner's spec does
        // not declare (DEF-8): the shared single-writer rule. An owner without a spec yet is
        // reported by `unresolved_reads`, not refused.
        let declarations: Vec<Declaration> = specs.iter().map(SystemSpec::declaration).collect();
        let (owners, rule_problems) = single_writer(&declarations, Source::Spec);
        problems.extend(rule_problems);
        Ownership {
            owners,
            specs: by_id,
            problems,
        }
    }

    /// Checks one access declaration against the table and the system's own spec
    /// (`rh_core::ownership::check_access`, the rule registration runs too).
    #[must_use]
    pub fn check(&self, access: &AccessSpec) -> Vec<Violation> {
        let entry = self.specs.get(&access.system).map(SystemSpec::entry);
        rh_core::ownership::check_access(
            &self.owners,
            entry.as_ref(),
            &access.system,
            &access.reads,
            &access.writes,
        )
    }

    /// The table as data for `rh-core`'s registration (`rh_core::register::register`).
    #[must_use]
    pub fn spec_table(&self) -> rh_core::ownership::SpecTable {
        rh_core::ownership::SpecTable {
            owners: self.owners.clone(),
            specs: self
                .specs
                .iter()
                .map(|(id, s)| (id.clone(), s.entry()))
                .collect(),
        }
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
                ty: "bool".to_owned(),
            }],
            reads: reads
                .iter()
                .map(|c| DeclaredRead {
                    column: (*c).to_owned(),
                    owner: "M1.regions".to_owned(),
                    lag: "lagged".to_owned(),
                })
                .collect(),
            rng: vec![],
            schedule: vec![],
            commands: vec![],
            emits: vec![],
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

    /// DEF-8: the lint checks the spec fields it relies on, without `lint_odd.py`.
    #[test]
    fn spec_fields_the_lint_relies_on_are_validated() {
        let reg = registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).unwrap();
        let dir = repo().join("specs/systems");
        let read = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap();
        let m2 = read("M2.demography.odd.toml");
        let s7 = read("S7.vows.odd.toml");
        let cases = [
            (
                "M2.demography.odd.toml",
                m2.replacen("plane = \"A\"", "plane = \"D\"", 1),
                "plane",
                "in plane 'D'",
            ),
            (
                "M2.demography.odd.toml",
                m2.replacen("plane = \"A\"", "plane = \"b\"", 1),
                "plane",
                "in plane 'b'",
            ),
            (
                "M2.demography.odd.toml",
                m2.replacen("type = \"i64\"", "type = \"wibble\"", 1),
                "spec",
                "unknown type 'wibble'",
            ),
            (
                "S7.vows.odd.toml",
                s7.replacen("column = \"person.alive\"", "column = \"person.mood\"", 1),
                "single-writer",
                "reads person.mood, which M2.demography's spec does not declare",
            ),
            (
                "S7.vows.odd.toml",
                s7.replacen("owner = \"M2.demography\"", "owner = \"S6.artifacts\"", 1),
                "single-writer",
                "from S6.artifacts, but M2.demography owns it",
            ),
        ];
        for (file, text, rule, needle) in cases {
            let mut specs = load_specs(&dir).unwrap();
            let i = specs
                .iter()
                .position(|s| s.path.file_name().is_some_and(|n| n == file))
                .unwrap();
            specs[i] = SystemSpec::from_toml(&dir.join(file), &text).unwrap();
            let t = Ownership::build(&specs, &reg);
            assert!(
                t.problems
                    .iter()
                    .any(|v| v.rule == rule && v.message.contains(needle)),
                "{needle}: {:?}",
                t.problems
            );
        }
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
