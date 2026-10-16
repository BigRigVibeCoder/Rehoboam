//! The single-writer and plane rules, as pure functions over data (SPR-005 ruling 3; `05.1` §2
//! rule 1; ADR-0024).
//!
//! One definition, two callers:
//! - the ownership lint (`rh_registry::ownership`, `rh-cli ownership`) builds the inputs from the
//!   spec files (`specs/systems/*.odd.toml`);
//! - registration ([`crate::register`]) runs the same functions on the `AccessSpec`s of the
//!   systems in code, with the specs handed in as data ([`SpecTable`]).
//!
//! `rh-core` reads no files; it sits below `rh-registry` (`rh-registry` depends on `rh-core`,
//! never the reverse; Architect ruling, 2026-10-16).
//!
//! | Function | Rule |
//! |---|---|
//! | [`single_writer`] | no column has two writers; a read names the writer; a read naming a declared system names a column it writes |
//! | [`plane_leak`] | a plane-A reader never reads a plane-B or plane-C column |
//! | [`check_access`] | one access declaration against the specs: writes only owned columns, reads only declared ones, no plane leak |
//! | [`system_plane`] | a system's plane from the planes of the columns it owns |
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §2 rule 1; ADR-0024; ADR-0033 §2;
//! `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` ruling 3; SPR-004 T-03.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// One broken ownership rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    /// The rule: `single-writer`, `undeclared-read`, `plane`, `registry`, `spec`, `system-id`,
    /// `rng-purpose`, `phase-order` or `schedule-cycle`.
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

/// The plane of a system from the planes of the columns it owns: `A` when it owns a plane-A
/// column or nothing, else the plane of its first column.
#[must_use]
pub fn system_plane<'a>(mut owned_planes: impl Iterator<Item = &'a str> + Clone) -> &'a str {
    if owned_planes.clone().next().is_none() || owned_planes.clone().any(|p| p == "A") {
        "A"
    } else {
        owned_planes.next().unwrap_or("A")
    }
}

/// Where a [`Declaration`] comes from: the messages name the right thing to fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// `specs/systems/*.odd.toml` (the lint).
    Spec,
    /// The `AccessSpec` a system registers in code.
    Code,
}

/// One system's access as the single-writer rule sees it: the columns it writes (with their
/// planes) and the columns it reads (with the owner it names).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// The system.
    pub system: String,
    /// `(column, plane)` for every column written (owned).
    pub writes: Vec<(String, String)>,
    /// `(column, named owner)` for every column read.
    pub reads: Vec<(String, String)>,
}

/// The single-writer rule over a set of declarations (`05.1` §2 rule 1):
///
/// - no column has two writers (the first declaration, in the given order, keeps it);
/// - a read names the system that writes the column;
/// - a read naming a declared system names a column that system writes.
///
/// A read whose named owner is not among the declarations is left to the caller (reported, not
/// refused). Returns the owner table and the violations.
#[must_use]
pub fn single_writer(
    declarations: &[Declaration],
    source: Source,
) -> (BTreeMap<String, Owner>, Vec<Violation>) {
    let mut owners: BTreeMap<String, Owner> = BTreeMap::new();
    let mut problems = Vec::new();
    for d in declarations {
        for (column, plane) in &d.writes {
            if let Some(first) = owners.get(column) {
                problems.push(Violation {
                    rule: "single-writer",
                    system: d.system.clone(),
                    message: match source {
                        Source::Spec => {
                            format!(
                                "owns {column}, which {} already owns (05.1 §2)",
                                first.system
                            )
                        }
                        Source::Code => format!(
                            "writes {column}, which {} also writes (05.1 §2)",
                            first.system
                        ),
                    },
                });
            } else {
                owners.insert(
                    column.clone(),
                    Owner {
                        system: d.system.clone(),
                        plane: plane.clone(),
                    },
                );
            }
        }
    }
    let declared: BTreeSet<&str> = declarations.iter().map(|d| d.system.as_str()).collect();
    for d in declarations {
        for (column, owner) in &d.reads {
            let message = match owners.get(column) {
                Some(o) if &o.system != owner => Some(format!(
                    "reads {column} from {owner}, but {} owns it",
                    o.system
                )),
                None if declared.contains(owner.as_str()) => Some(match source {
                    Source::Spec => {
                        format!("reads {column}, which {owner}'s spec does not declare in [[owns]]")
                    }
                    Source::Code => {
                        format!("reads {column}, which {owner} does not declare as a write")
                    }
                }),
                _ => None,
            };
            if let Some(message) = message {
                problems.push(Violation {
                    rule: "single-writer",
                    system: d.system.clone(),
                    message,
                });
            }
        }
    }
    (owners, problems)
}

/// The plane rule (ADR-0024): a plane-A reader never reads a plane-B or plane-C column.
#[must_use]
pub fn plane_leak(
    reader: &str,
    reader_plane: &str,
    column: &str,
    owner: &Owner,
) -> Option<Violation> {
    (reader_plane == "A" && (owner.plane == "B" || owner.plane == "C")).then(|| Violation {
        rule: "plane",
        system: reader.to_owned(),
        message: format!(
            "a plane-A system reads plane-{} column {column} (ADR-0024)",
            owner.plane
        ),
    })
}

/// What registration needs from one system's spec, as data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecEntry {
    /// `(column, plane)` for every `[[owns]]` entry.
    pub owns: Vec<(String, String)>,
    /// The columns of every `[[reads]]` entry.
    pub reads: Vec<String>,
    /// The `[[rng]] purpose` names, in spec order.
    pub rng: Vec<String>,
}

impl SpecEntry {
    /// The plane of the system ([`system_plane`] over its owned columns).
    #[must_use]
    pub fn plane(&self) -> &str {
        system_plane(self.owns.iter().map(|(_, p)| p.as_str()))
    }
}

/// The specs as registration sees them: the column owners and each system's entry. The host
/// builds it (`rh_registry::ownership::Ownership::spec_table`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecTable {
    /// Column to owner, from the specs.
    pub owners: BTreeMap<String, Owner>,
    /// Spec entries by system id.
    pub specs: BTreeMap<String, SpecEntry>,
}

/// Checks one access declaration (`system` reads `reads`, writes `writes`) against the spec
/// owners and the system's own spec entry (`None`: the system has no spec):
///
/// - `registry`: the system has a spec (ADR-0033);
/// - `single-writer`: it writes only columns its spec owns;
/// - `undeclared-read`: it reads only columns it owns or lists in `[[reads]]`;
/// - `plane`: a plane-A system reads no plane-B/C column.
#[must_use]
pub fn check_access(
    owners: &BTreeMap<String, Owner>,
    spec: Option<&SpecEntry>,
    system: &str,
    reads: &[String],
    writes: &[String],
) -> Vec<Violation> {
    let v = |rule, message: String| Violation {
        rule,
        system: system.to_owned(),
        message,
    };
    let Some(spec) = spec else {
        return vec![v(
            "registry",
            "has no system spec in specs/systems (ADR-0033)".to_owned(),
        )];
    };
    let mut out = Vec::new();
    for col in writes {
        match owners.get(col) {
            Some(o) if o.system == system => {}
            Some(o) => out.push(v(
                "single-writer",
                format!(
                    "writes {col}, which {} owns; request the change with a lifecycle command (05.1 §2)",
                    o.system
                ),
            )),
            None => out.push(v(
                "single-writer",
                format!("writes {col}, which no system spec owns"),
            )),
        }
    }
    let declared: BTreeSet<&str> = spec.reads.iter().map(String::as_str).collect();
    for col in reads {
        let own = spec.owns.iter().any(|(c, _)| c == col);
        if !own && !declared.contains(col.as_str()) {
            out.push(v(
                "undeclared-read",
                format!("reads {col}, which its spec does not list in [[reads]]"),
            ));
        }
        if let Some(o) = owners.get(col)
            && let Some(leak) = plane_leak(system, spec.plane(), col, o)
        {
            out.push(leak);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        Declaration, Owner, Source, SpecEntry, check_access, plane_leak, single_writer,
        system_plane,
    };

    fn decl(system: &str, writes: &[(&str, &str)], reads: &[(&str, &str)]) -> Declaration {
        let pairs = |v: &[(&str, &str)]| -> Vec<(String, String)> {
            v.iter()
                .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
                .collect()
        };
        Declaration {
            system: system.to_owned(),
            writes: pairs(writes),
            reads: pairs(reads),
        }
    }

    #[test]
    fn two_writers_and_wrong_owners_are_found_with_source_specific_messages() {
        let d = [
            decl("A", &[("x.a", "A")], &[]),
            decl("B", &[("x.a", "A")], &[("x.a", "C"), ("x.b", "A")]),
            decl("C", &[], &[]),
        ];
        let (owners, spec) = single_writer(&d, Source::Spec);
        let (_, code) = single_writer(&d, Source::Code);
        assert_eq!(owners["x.a"].system, "A");
        let msgs = |v: &[super::Violation]| v.iter().map(|x| x.message.clone()).collect::<Vec<_>>();
        assert_eq!(
            msgs(&spec),
            [
                "owns x.a, which A already owns (05.1 §2)",
                "reads x.a from C, but A owns it",
                "reads x.b, which A's spec does not declare in [[owns]]",
            ]
        );
        assert_eq!(
            msgs(&code),
            [
                "writes x.a, which A also writes (05.1 §2)",
                "reads x.a from C, but A owns it",
                "reads x.b, which A does not declare as a write",
            ]
        );
    }

    #[test]
    fn the_access_check_and_the_plane_rule() {
        let (owners, _) = single_writer(
            &[
                decl("A", &[("x.a", "A")], &[]),
                decl("P", &[("pins.p", "B")], &[]),
            ],
            Source::Spec,
        );
        let entry = SpecEntry {
            owns: vec![("y.b".to_owned(), "A".to_owned())],
            reads: vec!["pins.p".to_owned()],
            rng: vec![],
        };
        let found = check_access(
            &owners,
            Some(&entry),
            "Y",
            &["pins.p".to_owned(), "x.a".to_owned()],
            &["y.b".to_owned(), "x.a".to_owned()],
        );
        let rules: Vec<&str> = found.iter().map(|v| v.rule).collect();
        // y.b: no spec in the table owns it; x.a: A owns it; pins.p: plane B; x.a read undeclared.
        assert_eq!(
            rules,
            ["single-writer", "single-writer", "plane", "undeclared-read"]
        );
        assert_eq!(
            check_access(&owners, None, "Z", &[], &[])[0].rule,
            "registry"
        );
        let b = Owner {
            system: "P".to_owned(),
            plane: "B".to_owned(),
        };
        assert!(plane_leak("Y", "A", "pins.p", &b).is_some());
        assert!(plane_leak("Y", "B", "pins.p", &b).is_none());
        assert_eq!(system_plane([].into_iter()), "A");
        assert_eq!(system_plane(["B", "A"].into_iter()), "A");
        assert_eq!(system_plane(["C"].into_iter()), "C");
    }
}
