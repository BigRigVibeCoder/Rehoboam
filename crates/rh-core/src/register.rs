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
//! | `system-id` | no two ids hash to the same RNG `system_id`, nor to a reserved one ([`RESERVED_SYSTEM_IDS`]: `rh-worldgen`) | this module; SPR-011 ruling 10 |
//! | `single-writer`, `undeclared-read`, `plane` | each `AccessSpec` against the specs: the ownership lint's own check (`ownership::check_access`) | ruling 3 |
//! | `single-writer` | across the registered `AccessSpec`s: one writer per column, reads name the writer (`ownership::single_writer`) | ruling 3; T-04 |
//! | `plane` | a plane-A system reads no plane-B/C column a registered system writes (`ownership::plane_leak`) | ADR-0024 |
//! | `rng-purpose` | every purpose a system draws under is in its spec's `[[rng]]`, once | ruling 2; T-06 |
//! | `spec` | the bindings equal the spec's `[[schedule]]`, and each read's lag equals its `[[reads]].lag` | SPR-005 G-a |
//! | `phase-order`, `schedule-cycle` | the schedule derives (see [`crate::schedule`]) | T-05 |
//! | `command` | every command a system submits is in its spec's `[[commands]]` with the same owner; each command has one registered owner (`applies`); a submission names that owner, which must be registered; no system applies a command whose senders name another owner | SPR-006 T-03, ruling 5, G5 |
//! | `event-kind` | every event kind a system emits is in its spec's `[[emits]]`; each kind has one registered emitter | SPR-006 T-04 |
//!
//! Reported, not refused: a spec purpose the code never declares, a read whose owner is not
//! registered (its owner has no code yet), and a spec event kind the code never declares. A
//! command whose owner is not registered is refused (SPR-006 G5): nothing would ever apply it.
//!
//! The single-writer and plane rules are the functions of [`crate::ownership`], the ones the spec
//! lint (`rh-cli ownership`, through `rh_registry::ownership`) runs: where they meet, the lint
//! reads the specs and this checker reads the registered code (ruling 3).
//!
//! Registration sorts systems by id, so the result does not depend on the order they are passed
//! in.
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §2 rules 1, 2, 4, 6 and 8; ADR-0006;
//! ADR-0024; `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-04–T-06, rulings 2 and 3;
//! `CODEX/05_PROJECT/SPR-006_AgendaEventsInvariants.md` T-03, T-04, ruling 5.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ownership::{Source, SpecTable, Violation, check_access, plane_leak, single_writer};
use rh_ids::SystemId;

use crate::access::AccessSpec;
use crate::command::{ArgType, CommandDecl, KeyRule};
use crate::phase::Phase;
use crate::schedule::{self, Schedule, SchedulerInput};
use crate::system::{PulseBinding, SystemModel};
use crate::time::Pulse;
use rh_ids::EntityKind;

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

/// Ids no system may take: the keyed draws of world generation run under `rh-worldgen` (SPR-011
/// ruling 10), so a system whose RNG `system_id` equals it would share its streams. Registration
/// refuses such a system with rule `system-id`.
pub const RESERVED_SYSTEM_IDS: [&str; 1] = ["rh-worldgen"];

/// One registered system and what registration read from it.
pub(crate) struct Entry {
    pub(crate) model: Box<dyn SystemModel>,
    pub(crate) id: &'static str,
    pub(crate) system_id: SystemId,
    pub(crate) access: AccessSpec,
    pub(crate) bindings: Vec<PulseBinding>,
    pub(crate) purposes: Vec<&'static str>,
    /// The entity kind each purpose's `[[rng]] key` names, where it names one (BCK-002 A-50).
    pub(crate) purpose_kinds: BTreeMap<&'static str, EntityKind>,
    /// The rate and mode reads whose owner the world does not register, each with its neutral
    /// (`05.1` §2 rule 2, absent owners): the column enumerates as empty and a keyed lookup
    /// returns the neutral.
    pub(crate) dropped_reads: BTreeMap<&'static str, i64>,
}

/// A registered, conflict-free set of systems with its derived schedule.
pub struct Registered {
    pub(crate) entries: Vec<Entry>,
    schedule: Schedule,
    reports: Vec<String>,
    /// Every registered command: its owner, draining binding and typing (`05.1` §2 rule 2).
    pub(crate) commands: BTreeMap<&'static str, CommandDecl>,
    /// The declared `[[owns]] default` of the registered systems' columns (the row insert).
    pub(crate) defaults: BTreeMap<String, i64>,
}

impl Registered {
    /// The system ids, in id order (the indices the schedule uses).
    #[must_use]
    pub fn ids(&self) -> Vec<&'static str> {
        self.entries.iter().map(|e| e.id).collect()
    }

    /// The registered name equal to `name`, as the `&'static str` the engine's state uses: a
    /// system id, a command kind a system applies or sends, an event kind a system emits, or the
    /// core's own name (SPR-007 T-02: a keyframe's strings resolve through this, and an unknown
    /// name is refused).
    #[must_use]
    pub fn intern(&self, name: &str) -> Option<&'static str> {
        if name == crate::engine::CORE {
            return Some(crate::engine::CORE);
        }
        self.entries.iter().find_map(|e| {
            std::iter::once(e.id)
                .chain(e.access.applies.iter().map(|a| a.command))
                .chain(e.access.sends.iter().map(|c| c.command))
                .chain(e.access.emits.iter().copied())
                .find(|n| *n == name)
        })
    }

    /// The registered command `kind`: its owner, draining binding and typing.
    #[must_use]
    pub fn command(&self, kind: &str) -> Option<&CommandDecl> {
        self.commands.get(kind)
    }

    /// The rate and mode reads of `system` that registration dropped because their owner is
    /// absent, each with the neutral a keyed lookup returns (`05.1` §2 rule 2).
    #[must_use]
    pub fn dropped_reads(&self, system: &str) -> Vec<(&'static str, i64)> {
        self.entries
            .iter()
            .find(|e| e.id == system)
            .map(|e| e.dropped_reads.iter().map(|(c, n)| (*c, *n)).collect())
            .unwrap_or_default()
    }

    /// The columns some registered system reads with a lag, which the engine snapshots at the
    /// start of each instant (`05.1` §2 rule 4): a dropped read (absent owner) is not among them,
    /// since it creates no lag snapshot (rule 2; REV-P1a NEW-6).
    #[must_use]
    pub fn lagged_reads(&self) -> BTreeSet<&'static str> {
        self.entries
            .iter()
            .flat_map(|e| {
                e.access
                    .reads
                    .iter()
                    .filter(|r| r.lag.is_lagged() && !e.dropped_reads.contains_key(r.column))
                    .map(|r| r.column)
            })
            .collect()
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

    /// The canonical order of one phase's bindings, as `(system id, pulse)`.
    #[must_use]
    pub fn bindings(&self, phase: crate::phase::Phase) -> Vec<(&'static str, crate::time::Pulse)> {
        self.schedule
            .phase(phase)
            .order
            .iter()
            .map(|n| (self.entries[n.system].id, n.pulse))
            .collect()
    }

    /// The canonical order of one phase, as system ids (one per binding).
    #[must_use]
    pub fn order(&self, phase: crate::phase::Phase) -> Vec<&'static str> {
        self.schedule
            .phase(phase)
            .order
            .iter()
            .map(|n| self.entries[n.system].id)
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
                purpose_kinds: BTreeMap::new(),
                dropped_reads: BTreeMap::new(),
                model,
            }
        })
        .collect();
    entries.sort_by(|a, b| a.id.cmp(b.id));
    let mut violations = Vec::new();
    let mut reports = Vec::new();

    // Identity: one registration per id; distinct RNG system ids. A second registration of an id
    // is its own error and takes no further part, so it is never reported as conflicting with its
    // first registration (Tester DEF-2).
    let mut seen_ids: BTreeMap<SystemId, &str> = RESERVED_SYSTEM_IDS
        .iter()
        .map(|&id| (system_id(id), id))
        .collect();
    let mut kept: Vec<Entry> = Vec::with_capacity(entries.len());
    for e in entries {
        if kept.last().is_some_and(|k| k.id == e.id) {
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
        kept.push(e);
    }
    let mut entries = kept;

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
    check_reads(&mut entries, specs, &owners, &mut violations, &mut reports);

    // RNG purposes against the spec's [[rng]] table (ruling 2), and the kinds they key by (A-50).
    check_purposes(&entries, specs, &mut violations, &mut reports);
    purpose_kinds(&mut entries, specs, &mut reports);

    // Bindings and lags against the spec's [[schedule]] and [[reads]].lag (G-a).
    check_schedule_and_lags(&entries, specs, &mut violations);
    // Lifecycle commands and event kinds (SPR-006 ruling 5).
    let commands = check_commands(&entries, specs, &mut violations, &mut reports);
    check_events(&entries, specs, &mut violations, &mut reports);

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
    let violations = once_each(violations);
    match derived {
        Ok(schedule) if violations.is_empty() => {
            let defaults = entries
                .iter()
                .filter_map(|e| specs.specs.get(e.id))
                .flat_map(|s| s.defaults.iter().cloned())
                .collect();
            Ok(Registered {
                entries,
                schedule,
                reports,
                commands,
                defaults,
            })
        }
        Ok(_) => Err(RegistrationError {
            violations,
            reports,
        }),
        Err(schedule_problems) => {
            let mut violations = violations;
            violations.extend(schedule_problems);
            Err(RegistrationError {
                violations: once_each(violations),
                reports,
            })
        }
    }
}

/// Each violation once, in first-seen order (Tester DEF-2). The lint's check and the shared rules
/// can find the same fact (a plane leak, a column declared twice); the report names it once.
fn once_each(violations: Vec<Violation>) -> Vec<Violation> {
    let mut seen = BTreeSet::new();
    violations
        .into_iter()
        .filter(|v| seen.insert(v.clone()))
        .collect()
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

/// G-a: a system's bindings equal its spec's `[[schedule]]` (as a set of `(pulse, phase)`), and
/// every read's lag equals the spec's `[[reads]].lag` for that column: the same principle as the
/// `AccessSpec` agreeing with the ownership lint. A read the spec does not list is
/// `undeclared-read`, reported by `check_access`.
fn check_schedule_and_lags(entries: &[Entry], specs: &SpecTable, violations: &mut Vec<Violation>) {
    for e in entries {
        let Some(spec) = specs.specs.get(e.id) else {
            continue; // `registry` already reported by check_access.
        };
        let code: BTreeSet<(&str, &str)> = e
            .bindings
            .iter()
            .map(|b| (b.pulse.name(), b.phase.name()))
            .collect();
        let declared: BTreeSet<(&str, &str)> = spec
            .schedule
            .iter()
            .map(|(p, f)| (p.as_str(), f.as_str()))
            .collect();
        if code != declared {
            let show = |set: &BTreeSet<(&str, &str)>| {
                let v: Vec<String> = set.iter().map(|(p, f)| format!("{p}/{f}")).collect();
                if v.is_empty() {
                    "nothing".to_owned()
                } else {
                    v.join(", ")
                }
            };
            violations.push(violation(
                e.id,
                "spec",
                format!(
                    "binds {} but its spec's [[schedule]] declares {}",
                    show(&code),
                    show(&declared)
                ),
            ));
        }
        for r in &e.access.reads {
            if let Some((_, lag)) = spec.lags.iter().find(|(c, _)| c == r.column)
                && lag != r.lag.name()
            {
                violations.push(violation(
                    e.id,
                    "spec",
                    format!(
                        "reads {} at lag {}, but its spec's [[reads]] declares '{lag}'",
                        r.column,
                        r.lag.name()
                    ),
                ));
            }
        }
    }
}

/// Each read against the registered writers: a plane-A reader of a plane-B or C column is
/// refused; a read whose owner is not registered goes to [`absent_owner_read`].
fn check_reads(
    entries: &mut [Entry],
    specs: &SpecTable,
    owners: &BTreeMap<String, crate::ownership::Owner>,
    violations: &mut Vec<Violation>,
    reports: &mut Vec<String>,
) {
    let registered: BTreeSet<&str> = entries.iter().map(|e| e.id).collect();
    for e in entries.iter_mut() {
        let spec = specs.specs.get(e.id);
        let reads = e.access.reads.clone();
        for r in &reads {
            match owners.get(r.column) {
                Some(o) => violations.extend(plane_leak(e.id, e.access.plane(), r.column, o)),
                None if !registered.contains(r.owner) => {
                    let Some(spec) = spec else {
                        continue; // `registry` already reported by check_access.
                    };
                    absent_owner_read(e, spec, r.column, r.owner, violations, reports);
                }
                None => {}
            }
        }
    }
}

/// A read of `column` whose owner is not registered (`05.1` §2 rule 2, absent owners; REV-P1a
/// NEW-4; REV-EVO-001 R4, R12): an identity read is refused under rule `absent-owner`; a rate or
/// mode read is dropped and reported, its neutral served by keyed lookups.
fn absent_owner_read(
    e: &mut Entry,
    spec: &crate::ownership::SpecEntry,
    column: &'static str,
    owner: &str,
    violations: &mut Vec<Violation>,
    reports: &mut Vec<String>,
) {
    let usage = spec.read_uses.iter().find(|u| u.column == column);
    match usage {
        Some(u) if u.droppable() => match u.neutral {
            Some(neutral) => {
                e.dropped_reads.insert(column, neutral);
                reports.push(format!(
                    "{}: reads {column} from {owner}, which is not registered: a {} read, dropped; its neutral is {neutral}",
                    e.id, u.usage
                ));
            }
            None => violations.push(violation(
                e.id,
                "absent-owner",
                format!(
                    "reads {column} as a {} read from {owner}, which is not registered, and its spec gives no typed neutral (05.1 §2 rule 2: the type's zero is never a fallback)",
                    u.usage
                ),
            )),
        },
        _ => violations.push(violation(
            e.id,
            "absent-owner",
            format!(
                "reads {column} as an identity read from {owner}, which is not registered: a world that registers {} registers {owner} (05.1 §2 rule 2)",
                e.id
            ),
        )),
    }
}

/// BCK-002 A-50: the entity kind each purpose's `[[rng]] key` names. `person` and the person
/// roles (`actor`, `target`, `ruler`, ...) key by a person, `region` by a region, `world` by the
/// world; a key name with no `EntityKind` yet (`title`, `house`, ...) is reported, and its draws
/// are not kind-checked until that kind exists (SPR-014's choice; M7, SPR-021).
fn purpose_kinds(entries: &mut [Entry], specs: &SpecTable, reports: &mut Vec<String>) {
    const PERSON_KEYS: [&str; 9] = [
        "person", "actor", "target", "ruler", "seeker", "thief", "victim", "person_a", "holder",
    ];
    for e in entries {
        let Some(spec) = specs.specs.get(e.id) else {
            continue;
        };
        for &p in &e.purposes {
            let Some((_, key)) = spec.rng_keys.iter().find(|(q, _)| q == p) else {
                continue;
            };
            let kind = match key.as_str() {
                k if PERSON_KEYS.contains(&k) => Some(EntityKind::Person),
                "region" => Some(EntityKind::Region),
                "world" => Some(EntityKind::World),
                _ => None,
            };
            match kind {
                Some(kind) => {
                    e.purpose_kinds.insert(p, kind);
                }
                None => reports.push(format!(
                    "{}: RNG purpose {p} keys by {key}, which has no EntityKind yet: its draws are not kind-checked (BCK-002 A-50)",
                    e.id
                )),
            }
        }
    }
}

/// The binding a spec's `drained_by` names, if its names parse.
fn spec_binding((pulse, phase): &(String, String)) -> Option<PulseBinding> {
    Some(PulseBinding {
        pulse: Pulse::from_name(pulse)?,
        phase: Phase::from_name(phase)?,
    })
}

fn show_binding(b: PulseBinding) -> String {
    format!("{}/{}", b.pulse.name(), b.phase.name())
}

/// SPR-006 T-03, ruling 5, and EVO-001 (SPR-014 T-01, T-04): lifecycle commands have one owner,
/// only it applies them, each is drained by the binding its owner's spec names (the code's
/// `DRAINS` equals the spec's `[[applies]] drained_by`, rule `spec`), and a send to an owner the
/// world does not register is reported (its submissions are discarded and counted), not refused.
/// Returns the registered commands with their typing.
fn check_commands(
    entries: &[Entry],
    specs: &SpecTable,
    violations: &mut Vec<Violation>,
    reports: &mut Vec<String>,
) -> BTreeMap<&'static str, CommandDecl> {
    let (owners, commands) = check_applies(entries, specs, violations);
    let registered: BTreeSet<&str> = entries.iter().map(|e| e.id).collect();
    for e in entries {
        let spec = specs.specs.get(e.id);
        for send in &e.access.sends {
            let in_spec = spec.is_some_and(|s| {
                s.commands
                    .iter()
                    .any(|(t, c)| t == send.target && c == send.command)
            });
            if spec.is_some() && !in_spec {
                violations.push(violation(
                    e.id,
                    "command",
                    format!(
                        "submits command {} to {}, which its spec's [[commands]] table does not declare",
                        send.command, send.target
                    ),
                ));
            }
            match owners.get(send.command) {
                Some(&owner) if owner != send.target => violations.push(violation(
                    e.id,
                    "command",
                    format!(
                        "submits command {} to {}, but {owner} applies it",
                        send.command, send.target
                    ),
                )),
                Some(_) => {}
                None if registered.contains(send.target) => violations.push(violation(
                    e.id,
                    "command",
                    format!(
                        "submits command {} to {}, which does not declare that it applies it",
                        send.command, send.target
                    ),
                )),
                None => reports.push(format!(
                    "{}: submits command {} to {}, which is not registered: each submission is discarded and counted under dropped{{{}, {}, absent_owner}} (05.1 §2 rule 2)",
                    e.id, send.command, send.target, e.id, send.command
                )),
            }
        }
    }
    commands
}

/// The `applies` half of [`check_commands`]: one owner per command, no non-owner, and each
/// drain equal to its spec's `drained_by` (rule `spec`); the owners and the typed declarations.
fn check_applies<'e>(
    entries: &'e [Entry],
    specs: &SpecTable,
    violations: &mut Vec<Violation>,
) -> (
    BTreeMap<&'e str, &'static str>,
    BTreeMap<&'static str, CommandDecl>,
) {
    // The registered owner of each command: the first system, in id order, that applies it.
    let mut owners: BTreeMap<&'e str, &'static str> = BTreeMap::new();
    let mut commands: BTreeMap<&'static str, CommandDecl> = BTreeMap::new();
    for e in entries {
        let spec = specs.specs.get(e.id);
        for a in &e.access.applies {
            let c = a.command;
            if let Some(first) = owners.get(c) {
                violations.push(violation(
                    e.id,
                    "command",
                    format!("applies command {c}, which {first} also applies (05.1 §2 rule 2)"),
                ));
                continue;
            }
            owners.insert(c, e.id);
            // Every spec that submits c must name e as its owner.
            for (sender, spec) in &specs.specs {
                for (target, command) in &spec.commands {
                    if command == c && target != e.id {
                        violations.push(violation(
                            e.id,
                            "command",
                            format!(
                                "applies command {c}, but {sender}'s spec names {target} as its owner; a non-owner never applies a command"
                            ),
                        ));
                    }
                }
            }
            let mut decl = CommandDecl::untyped(e.id, a.drained_by);
            if let Some(spec) = spec {
                match spec.applies.iter().find(|s| s.command == c) {
                    None => violations.push(violation(
                        e.id,
                        "spec",
                        format!(
                            "drains command {c} at {}, but its spec's [[applies]] does not declare {c}",
                            show_binding(a.drained_by)
                        ),
                    )),
                    Some(s) => {
                        let declared = spec_binding(&s.drained_by);
                        if declared != Some(a.drained_by) {
                            violations.push(violation(
                                e.id,
                                "spec",
                                format!(
                                    "drains command {c} at {}, but its spec's [[applies]] says drained_by = {}/{}",
                                    show_binding(a.drained_by),
                                    s.drained_by.0,
                                    s.drained_by.1
                                ),
                            ));
                        }
                        if !e.bindings.contains(&a.drained_by) {
                            violations.push(violation(
                                e.id,
                                "spec",
                                format!(
                                    "drains command {c} at {}, which is not one of its bindings",
                                    show_binding(a.drained_by)
                                ),
                            ));
                        }
                        decl = typed(decl, s, &specs.enum_codes, e.id, violations);
                    }
                }
            }
            commands.insert(c, decl);
        }
        if let Some(spec) = spec {
            for s in &spec.applies {
                if e.access.apply_of(&s.command).is_none() {
                    violations.push(violation(
                        e.id,
                        "spec",
                        format!(
                            "its spec's [[applies]] declares command {} drained_by = {}/{}, but its code drains nothing for it",
                            s.command, s.drained_by.0, s.drained_by.1
                        ),
                    ));
                }
            }
        }
    }
    (owners, commands)
}

/// The typing of one registered command from its owner's `[[applies]]` entry.
fn typed(
    mut decl: CommandDecl,
    s: &crate::ownership::SpecApply,
    enum_codes: &BTreeMap<String, usize>,
    owner: &str,
    violations: &mut Vec<Violation>,
) -> CommandDecl {
    decl.key = if s.key == "living" {
        KeyRule::Living
    } else {
        KeyRule::Any
    };
    decl.outcome = s.outcome;
    if let Some(fields) = &s.payload {
        let mut typed = Vec::with_capacity(fields.len());
        for (name, ty) in fields {
            match ArgType::parse(ty, enum_codes) {
                Some(t) => typed.push((name.clone(), t)),
                None => violations.push(violation(
                    owner,
                    "spec",
                    format!(
                        "command {}'s payload field {name} has type {ty}, outside the payload vocabulary",
                        s.command
                    ),
                )),
            }
        }
        if s.key_field != "person" {
            match typed
                .iter()
                .position(|(n, t)| *n == s.key_field && *t == ArgType::Person)
            {
                Some(i) => decl.key_field = Some(i),
                None => violations.push(violation(
                    owner,
                    "spec",
                    format!(
                        "command {}'s key_field {} is not a PersonId field of its payload",
                        s.command, s.key_field
                    ),
                )),
            }
        }
        decl.payload = Some(typed);
    }
    decl
}

/// SPR-006 T-04: every emitted kind is declared in the spec, and each kind has one emitter.
fn check_events(
    entries: &[Entry],
    specs: &SpecTable,
    violations: &mut Vec<Violation>,
    reports: &mut Vec<String>,
) {
    let mut emitters: BTreeMap<&str, &str> = BTreeMap::new();
    for e in entries {
        let spec = specs.specs.get(e.id);
        for &kind in &e.access.emits {
            if let Some(first) = emitters.get(kind) {
                violations.push(violation(
                    e.id,
                    "event-kind",
                    format!("emits {kind}, which {first} also emits"),
                ));
            } else {
                emitters.insert(kind, e.id);
            }
            if let Some(spec) = spec
                && !spec.emits.iter().any(|k| k == kind)
            {
                violations.push(violation(
                    e.id,
                    "event-kind",
                    format!("emits {kind}, which its spec's [[emits]] table does not declare"),
                ));
            }
        }
        for kind in spec.map(|s| s.emits.as_slice()).unwrap_or_default() {
            if !e.access.emits_kind(kind) {
                reports.push(format!(
                    "{}: spec event kind {kind} has no use in code",
                    e.id
                ));
            }
        }
    }
}
