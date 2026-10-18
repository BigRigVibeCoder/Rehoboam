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

use rh_core::ownership::{SpecApply, SpecEntry, SpecReadUse};
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
    /// An enum column's code names, by position (EVO-001).
    #[serde(default)]
    pub values: Option<Vec<String>>,
    /// The column's declared default, if any (`05.1` §2 rule 2: the ledger row insert).
    #[serde(default, deserialize_with = "literal")]
    pub default: Option<Literal>,
}

/// A TOML scalar from a spec (a `neutral` or a `default`), kept exactly as written: a float is
/// kept as its decimal text and turned into fixed-point bits without any float arithmetic
/// ([`Literal::typed`]), so this plane-A crate stays float-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    /// `true` or `false`.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A string: an enum code's name.
    Text(String),
    /// A float literal, as the shortest decimal text that reads back to it.
    Decimal(String),
}

fn literal<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Literal>, D::Error> {
    let v: Option<toml::Value> = Option::deserialize(d)?;
    Ok(v.and_then(|v| match v {
        toml::Value::Boolean(b) => Some(Literal::Bool(b)),
        toml::Value::Integer(i) => Some(Literal::Int(i)),
        toml::Value::String(s) => Some(Literal::Text(s)),
        f @ toml::Value::Float(_) => Some(Literal::Decimal(f.to_string())),
        _ => None,
    }))
}

impl Literal {
    /// The literal as a value of a column of type `ty` (with its enum `values`), as the column
    /// stores it: an enum code, 0 or 1, an integer, or a fixed-point value's raw bits (the exact
    /// decimal rounded half to even, ruling 9). `None` when it does not fit the type.
    #[must_use]
    pub fn typed(&self, ty: &str, values: Option<&[String]>) -> Option<i64> {
        match (self, ty) {
            (Self::Bool(b), "bool") => Some(i64::from(*b)),
            (Self::Text(s), "enum") => values?
                .iter()
                .position(|v| v == s)
                .and_then(|i| i64::try_from(i).ok()),
            (Self::Int(i), "q8_8") => i.checked_mul(1 << 8),
            (Self::Int(i), "q16_16") => i.checked_mul(1 << 16),
            (Self::Int(i), _) if ty != "bool" && ty != "enum" => Some(*i),
            (Self::Decimal(d), "q8_8") => decimal_bits(d, 8),
            (Self::Decimal(d), "q16_16") => decimal_bits(d, 16),
            _ => None,
        }
    }
}

/// The decimal `text` (`[-]digits[.digits]`) times `2^frac_bits`, rounded half to even, in
/// integer arithmetic.
fn decimal_bits(text: &str, frac_bits: u32) -> Option<i64> {
    let (negative, body) = text
        .strip_prefix('-')
        .map_or((false, text), |rest| (true, rest));
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    if int.is_empty() || !int.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let digits: i128 = format!("{int}{frac}").parse().ok()?;
    let scale = 10_i128.checked_pow(u32::try_from(frac.len()).ok()?)?;
    let n = digits.checked_mul(1_i128 << frac_bits)?;
    let (q, r) = (n / scale, n % scale);
    let q = if 2 * r > scale || (2 * r == scale && q % 2 == 1) {
        q + 1
    } else {
        q
    };
    i64::try_from(if negative { -q } else { q }).ok()
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
    /// `identity`, `rate` or `mode` (EVO-001 R4, R12).
    #[serde(default, rename = "use")]
    pub usage: String,
    /// A rate or mode read's neutral, as written (EVO-001 R12).
    #[serde(default, deserialize_with = "literal")]
    pub neutral: Option<Literal>,
}

/// One `[[schedule]]` binding (SPR-005 G-a).
#[derive(Debug, Deserialize)]
struct DeclaredBinding {
    pulse: String,
    phase: String,
    /// The entry's steps, as written (the review's K6: a drain-only binding has one, the drain).
    #[serde(default)]
    steps: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SpecHeader {
    id: String,
}

/// One keyed RNG purpose, from `[[rng]]` (SPR-005 T-06; ruling 2).
#[derive(Debug, Deserialize)]
struct DeclaredRng {
    purpose: String,
    #[serde(default)]
    key: Vec<String>,
}

/// One `[[applies]]` entry (EVO-001): the command a system owns, its typed payload, the binding
/// that drains it, its key rule and key field, and whether it carries a sampled outcome.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeclaredApply {
    /// The command.
    pub command: String,
    /// `"{ name: Type, ... }"`.
    #[serde(default)]
    pub payload: Option<String>,
    /// `"<Pulse>/<Phase>"`.
    #[serde(default)]
    pub drained_by: Option<String>,
    /// `living` or `any`.
    #[serde(default)]
    pub key: Option<String>,
    /// A `PersonId` payload field, or `person`.
    #[serde(default)]
    pub key_field: Option<String>,
    /// Whether it carries a sampled outcome.
    #[serde(default)]
    pub outcome: Option<bool>,
}

/// One lifecycle command submission, from `[[commands]]` (SPR-006).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeclaredCommand {
    /// The owning system the spec names.
    pub target: String,
    /// The command's Registry name.
    pub command: String,
    /// The sender's timer kind, for an outcome-carrying command (EVO-001 R9).
    #[serde(default)]
    pub timer: Option<String>,
    /// The purpose that timer is drawn under.
    #[serde(default)]
    pub purpose: Option<String>,
}

/// One emitted event kind, from `[[emits]]` (SPR-006).
#[derive(Debug, Deserialize)]
struct DeclaredEmit {
    kind: String,
}

/// `[initialization]`: only the manifest sections genesis initialises from (SPR-012 T-06).
#[derive(Debug, Default, Deserialize)]
struct DeclaredInitialization {
    #[serde(default)]
    from_manifest: Vec<String>,
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
    applies: Vec<DeclaredApply>,
    #[serde(default)]
    emits: Vec<DeclaredEmit>,
    #[serde(default)]
    initialization: DeclaredInitialization,
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
    /// Each `[[schedule]]` entry's `steps`, parallel to [`SystemSpec::schedule`] (the review's
    /// K6: what ties a code's drain-only binding to its spec).
    pub schedule_steps: Vec<Vec<String>>,
    /// The lifecycle commands it submits (`[[commands]]`), in spec order (SPR-006).
    pub commands: Vec<DeclaredCommand>,
    /// The lifecycle commands it owns (`[[applies]]`), in spec order (EVO-001; SPR-014).
    pub applies: Vec<DeclaredApply>,
    /// `(purpose, first key name)` of every `[[rng]]` entry (BCK-002 A-50).
    pub rng_keys: Vec<(String, String)>,
    /// The event kinds it emits (`[[emits]] kind`), in spec order (SPR-006).
    pub emits: Vec<String>,
    /// The manifest sections `[initialization] from_manifest` names, in spec order: genesis
    /// writes the system's columns from those sections only (SPR-011 ruling 10; SPR-012 T-06).
    pub from_manifest: Vec<String>,
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
            rng_keys: f
                .rng
                .iter()
                .map(|r| {
                    (
                        r.purpose.clone(),
                        r.key.first().cloned().unwrap_or_default(),
                    )
                })
                .collect(),
            rng: f.rng.into_iter().map(|r| r.purpose).collect(),
            schedule_steps: f.schedule.iter().map(|b| b.steps.clone()).collect(),
            schedule: f.schedule.into_iter().map(|b| (b.pulse, b.phase)).collect(),
            commands: f.commands,
            applies: f.applies,
            emits: f.emits.into_iter().map(|e| e.kind).collect(),
            from_manifest: f.initialization.from_manifest,
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
            from_manifest: self.from_manifest.clone(),
            applies: self.applies.iter().map(spec_apply).collect(),
            read_uses: Vec::new(),
            outcome_timers: self
                .commands
                .iter()
                .filter_map(|c| Some((c.command.clone(), c.timer.clone()?, c.purpose.clone()?)))
                .collect(),
            rng_keys: self.rng_keys.clone(),
            defaults: self
                .owns
                .iter()
                .filter_map(|o| {
                    let v = o.default.as_ref()?.typed(&o.ty, o.values.as_deref())?;
                    Some((o.column.clone(), v))
                })
                .collect(),
        }
    }

    /// [`SystemSpec::entry`], with each read's `use` and its neutral typed as the column
    /// (`columns`: every spec's `[[owns]]` by column; EVO-001 R12).
    #[must_use]
    pub fn entry_with(&self, columns: &BTreeMap<String, OwnedColumn>) -> SpecEntry {
        let mut entry = self.entry();
        entry.read_uses = self
            .reads
            .iter()
            .map(|r| SpecReadUse {
                column: r.column.clone(),
                usage: r.usage.clone(),
                neutral: r
                    .neutral
                    .as_ref()
                    .and_then(|n| match columns.get(&r.column) {
                        Some(o) => n.typed(&o.ty, o.values.as_deref()),
                        // An owner with no spec yet (M6's era state): the literal is typed by itself;
                        // a zero is zero in every type, and anything else is left untyped.
                        None => match n {
                            Literal::Bool(b) => Some(i64::from(*b)),
                            Literal::Int(i) => Some(*i),
                            Literal::Decimal(d)
                                if d.bytes().filter(u8::is_ascii_digit).all(|b| b == b'0') =>
                            {
                                Some(0)
                            }
                            _ => None,
                        },
                    }),
            })
            .collect();
        entry
    }
}

/// One `[[applies]]` entry as registration takes it.
fn spec_apply(a: &DeclaredApply) -> SpecApply {
    let drained_by = a
        .drained_by
        .as_deref()
        .and_then(|d| d.split_once('/'))
        .map(|(p, f)| (p.to_owned(), f.to_owned()))
        .unwrap_or_default();
    SpecApply {
        command: a.command.clone(),
        drained_by,
        key: a.key.clone().unwrap_or_else(|| "any".to_owned()),
        key_field: a.key_field.clone().unwrap_or_else(|| "person".to_owned()),
        payload: a.payload.as_deref().and_then(payload_types),
        outcome: a.outcome.unwrap_or(false),
    }
}

/// The `(name, type)` fields of a typed payload `"{ name: Type, ... }"`, or `None` when it is not
/// typed (`lint_odd.py` refuses that on a frozen spec).
fn payload_types(text: &str) -> Option<Vec<(String, String)>> {
    let inner = text.trim().strip_prefix('{')?.strip_suffix('}')?;
    inner
        .split(',')
        .filter(|f| !f.trim().is_empty())
        .map(|f| {
            let (name, ty) = f.split_once(':')?;
            Some((name.trim().to_owned(), ty.trim().to_owned()))
        })
        .collect()
}

/// Whether a spec file name is a system spec: `*.odd.toml`, not a `_`-prefixed template.
#[must_use]
pub fn is_spec_file(name: &str) -> bool {
    name.ends_with(".odd.toml") && !name.starts_with('_')
}

/// Parses the system specs among `files` (`(path, text)` pairs, as a host read them): the files
/// [`is_spec_file`] accepts, in path order. A pure parser (BCK-002 A-51): reading the files is the
/// host's (`rh_manifest::specs::load_specs`) or a test helper's, never plane A's.
///
/// # Errors
/// A spec cannot be parsed.
pub fn parse_specs(files: Vec<(PathBuf, String)>) -> Result<Vec<SystemSpec>, String> {
    let mut files: Vec<(PathBuf, String)> = files
        .into_iter()
        .filter(|(p, _)| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(is_spec_file)
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
        .iter()
        .map(|(p, text)| SystemSpec::from_toml(p, text))
        .collect()
}

/// The system ids of a Registry's text: the keys of its `[systems]` table. Reads nothing else, so
/// this plane-A crate never parses a Registry number. `name` is for messages only. A pure parser
/// (BCK-002 A-51).
///
/// # Errors
/// The text is not TOML, or has no `[systems]` table.
pub fn parse_registry_system_ids(name: &str, text: &str) -> Result<BTreeSet<String>, String> {
    let table: toml::Table = toml::from_str(text).map_err(|e| format!("{name}: {e}"))?;
    match table.get("systems") {
        Some(toml::Value::Table(systems)) => Ok(systems.keys().cloned().collect()),
        _ => Err(format!("{name}: no [systems] table")),
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
    /// system ids (from `rh-manifest`'s Registry, or [`parse_registry_system_ids`]).
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
        let columns: BTreeMap<String, OwnedColumn> = self
            .specs
            .values()
            .flat_map(|s| s.owns.iter().map(|o| (o.column.clone(), o.clone())))
            .collect();
        rh_core::ownership::SpecTable {
            owners: self.owners.clone(),
            specs: self
                .specs
                .iter()
                .map(|(id, s)| (id.clone(), s.entry_with(&columns)))
                .collect(),
            enum_codes: columns
                .iter()
                .filter_map(|(c, o)| Some((c.clone(), o.values.as_ref()?.len())))
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
