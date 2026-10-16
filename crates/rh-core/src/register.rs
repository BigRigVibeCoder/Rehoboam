//! System registration: the access conflict checker, the RNG purpose check and the derived
//! schedule (SPR-005 T-04, T-05, T-06).
//!
//! [`register`] takes the systems and the spec table the host built from the specs
//! ([`SpecTable`], from `rh_registry::ownership::Ownership::spec_table`; `rh-core` has no I/O). It refuses the whole set, with
//! every violation, if any of these fails:
//!
//! | Rule | Check | Source |
//! |---|---|---|
//! | `registry` | each system id is registered once and has a spec | ADR-0033 |
//! | `system-id` | no two ids hash to the same RNG `system_id` | this module |
//! | `single-writer`, `undeclared-read`, `plane` | each `AccessSpec` against the specs: the ownership lint's own check (`ownership::check_access`) | ruling 3 |
//! | `single-writer` | across the registered `AccessSpec`s: one writer per column, reads name the writer (`ownership::single_writer`) | ruling 3; T-04 |
//! | `plane` | a plane-A system reads no plane-B/C column a registered system writes (`ownership::plane_leak`) | ADR-0024 |
//! | `rng-purpose` | every purpose a system draws under is in its spec's `[[rng]]`, once | ruling 2; T-06 |
//! | `phase-order`, `schedule-cycle` | the schedule derives (see [`crate::schedule`]) | T-05 |
//!
//! Reported, not refused: a spec purpose the code never declares, and a read whose owner is not
//! registered (its owner has no code yet).
//!
//! The single-writer and plane rules are the functions of [`crate::ownership`], the ones the spec
//! lint (`rh-cli ownership`, through `rh_registry::ownership`) runs: where they meet, the lint
//! reads the specs and this checker reads the registered code (ruling 3).
//!
//! Registration sorts systems by id, so the result does not depend on the order they are passed
//! in.
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §2 rules 1, 4 and 6; ADR-0006; ADR-0024;
//! `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-04–T-06, rulings 2 and 3.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ownership::{Source, SpecTable, Violation, check_access, plane_leak, single_writer};
use rh_ids::SystemId;

use crate::access::AccessSpec;
use crate::schedule::{self, Schedule, SchedulerInput};
use crate::system::{PulseBinding, SystemModel};

/// Domain tag of [`system_id`].
pub const TAG_SYSTEM_ID: u64 = rh_ids::rng::tag("rehoboam/rh-core/system/v1");

/// The RNG `system_id` of a Registry system id: the high 32 bits of
/// `h64_bytes(tag("rehoboam/rh-core/system/v1"), id)`.
///
/// Contract text since SPR-005 (`05.1` §6 item 2, the `system_id` row; Architect ruling): a
/// name-derived id never changes when another system is added, so adding a system never shifts
/// anyone else's draws; a collision is a registration error (`system-id`). Changing this layout,
/// or renaming a system, changes that system's draws: a C3 change. Golden values from the
/// independent reference (`bin/gen_rng_golden.py`, `system` lines) pin it
/// (`tests/system_id_golden.rs`).
#[must_use]
pub fn system_id(id: &str) -> SystemId {
    let h = rh_ids::rng::h64_bytes(TAG_SYSTEM_ID, id.as_bytes());
    SystemId::from_raw(u32::try_from(h >> 32).unwrap_or(0))
}

/// One registered system and what registration read from it.
pub(crate) struct Entry {
    pub(crate) model: Box<dyn SystemModel>,
    pub(crate) id: &'static str,
    pub(crate) system_id: SystemId,
    pub(crate) access: AccessSpec,
    pub(crate) bindings: Vec<PulseBinding>,
    pub(crate) purposes: Vec<&'static str>,
}

/// A registered, conflict-free set of systems with its derived schedule.
pub struct Registered {
    pub(crate) entries: Vec<Entry>,
    schedule: Schedule,
    reports: Vec<String>,
}

impl Registered {
    /// The system ids, in id order (the indices the schedule uses).
    #[must_use]
    pub fn ids(&self) -> Vec<&'static str> {
        self.entries.iter().map(|e| e.id).collect()
    }

    /// The derived schedule.
    #[must_use]
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Notes that are not violations (unused spec purposes, reads of unregistered owners).
    #[must_use]
    pub fn reports(&self) -> &[String] {
        &self.reports
    }

    /// The canonical order of one phase, as system ids.
    #[must_use]
    pub fn order(&self, phase: crate::phase::Phase) -> Vec<&'static str> {
        self.schedule
            .phase(phase)
            .order
            .iter()
            .map(|&i| self.entries[i].id)
            .collect()
    }

    /// The access declarations from code, in id order: what `rh-cli ownership` will count as
    /// "access declarations from code".
    #[must_use]
    pub fn accesses(&self) -> Vec<(&'static str, &AccessSpec)> {
        self.entries.iter().map(|e| (e.id, &e.access)).collect()
    }
}

/// A refused registration: every violation found, and the reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistrationError {
    /// Every violation, in id order then rule order.
    pub violations: Vec<Violation>,
    /// The notes that are not violations.
    pub reports: Vec<String>,
}

impl fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for v in &self.violations {
            writeln!(f, "{v}")?;
        }
        write!(
            f,
            "registration refused: {} violations",
            self.violations.len()
        )
    }
}

impl std::error::Error for RegistrationError {}

fn violation(system: &str, rule: &'static str, message: String) -> Violation {
    Violation {
        rule,
        system: system.to_owned(),
        message,
    }
}

/// Registers `systems` against the specs' table.
///
/// # Errors
/// [`RegistrationError`] with every violation of the rules in the module table.
pub fn register(
    systems: Vec<Box<dyn SystemModel>>,
    specs: &SpecTable,
) -> Result<Registered, RegistrationError> {
    let mut entries: Vec<Entry> = systems
        .into_iter()
        .map(|model| {
            let d = model.descriptor();
            Entry {
                id: d.id,
                system_id: system_id(d.id),
                access: model.access(),
                bindings: model.schedule(),
                purposes: model.rng_purposes(),
                model,
            }
        })
        .collect();
    entries.sort_by(|a, b| a.id.cmp(b.id));
    let mut violations = Vec::new();
    let mut reports = Vec::new();

    // Identity: one registration per id; distinct RNG system ids.
    let mut seen_ids: BTreeMap<SystemId, &str> = BTreeMap::new();
    for (i, e) in entries.iter().enumerate() {
        if i > 0 && entries[i - 1].id == e.id {
            violations.push(violation(
                e.id,
                "registry",
                "is registered twice".to_owned(),
            ));
            continue;
        }
        if let Some(other) = seen_ids.insert(e.system_id, e.id) {
            violations.push(violation(
                e.id,
                "system-id",
                format!("hashes to the RNG system_id of {other} ({})", e.system_id),
            ));
        }
    }

    // Each declaration against the specs: the lint's own check.
    for e in &entries {
        violations.extend(check_access(
            &specs.owners,
            specs.specs.get(e.id),
            e.id,
            &e.access.read_columns(),
            &e.access.write_columns(),
        ));
    }

    // The declarations against each other: the shared single-writer and plane rules.
    let declarations: Vec<_> = entries.iter().map(|e| e.access.declaration(e.id)).collect();
    let (owners, conflicts) = single_writer(&declarations, Source::Code);
    violations.extend(conflicts);
    let registered: BTreeSet<&str> = entries.iter().map(|e| e.id).collect();
    for e in &entries {
        for r in &e.access.reads {
            match owners.get(r.column) {
                Some(o) => violations.extend(plane_leak(e.id, e.access.plane(), r.column, o)),
                None if !registered.contains(r.owner) => reports.push(format!(
                    "{}: reads {} from {}, which is not registered",
                    e.id, r.column, r.owner
                )),
                None => {}
            }
        }
    }

    // RNG purposes against the spec's [[rng]] table (ruling 2).
    check_purposes(&entries, specs, &mut violations, &mut reports);

    // The schedule (T-05), only when the declarations themselves are sound.
    let inputs: Vec<SchedulerInput<'_>> = entries
        .iter()
        .map(|e| SchedulerInput {
            id: e.id,
            access: &e.access,
            bindings: &e.bindings,
        })
        .collect();
    let derived = schedule::derive(&inputs);
    drop(inputs);
    match derived {
        Ok(schedule) if violations.is_empty() => Ok(Registered {
            entries,
            schedule,
            reports,
        }),
        Ok(_) => Err(RegistrationError {
            violations,
            reports,
        }),
        Err(schedule_problems) => {
            violations.extend(schedule_problems);
            Err(RegistrationError {
                violations,
                reports,
            })
        }
    }
}

/// T-06: every purpose a system draws under is in its spec's `[[rng]]` table, once; a spec
/// purpose with no code use is reported.
fn check_purposes(
    entries: &[Entry],
    specs: &SpecTable,
    violations: &mut Vec<Violation>,
    reports: &mut Vec<String>,
) {
    for e in entries {
        let Some(spec) = specs.specs.get(e.id) else {
            continue; // `registry` already reported by check_access.
        };
        let mut declared = BTreeSet::new();
        for &p in &e.purposes {
            if !declared.insert(p) {
                violations.push(violation(
                    e.id,
                    "rng-purpose",
                    format!("declares RNG purpose {p} twice"),
                ));
            } else if !spec.rng.iter().any(|s| s == p) {
                violations.push(violation(
                    e.id,
                    "rng-purpose",
                    format!(
                        "draws under RNG purpose {p}, which its spec's [[rng]] table does not declare (ruling 2)"
                    ),
                ));
            }
        }
        for s in &spec.rng {
            if !declared.contains(s.as_str()) {
                reports.push(format!("{}: spec RNG purpose {s} has no use in code", e.id));
            }
        }
    }
}
