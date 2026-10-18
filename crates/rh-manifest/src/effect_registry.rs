//! The Registry's effect-language sections and the real environment (SPR-017 T-06).
//!
//! `registry_v0.toml` carries the succession laws as programs (`[succession_laws.<law>]`), the
//! language's version, enums, reference kinds, keys and opaque columns (`[effect_language]`) and
//! the command signatures (`[commands]`). This module types those sections, builds the checker's
//! [`Env`] from the Registry and the system specs' TOML text ([`effect_env`]), and refuses, by
//! name, what the Registry cannot mean ([`registry_problems`], [`effect_problems`]).
//!
//! The specs are passed as text, so BCK-002 A-51's move of the spec loader moves only a call site.
//! An enum's ordinal is its declaration index (§3.1): a keyed Registry table's ids are taken in
//! source order ([`key_order`] of the Registry's text), never in `toml::Table`'s sorted order,
//! which is also why [`effect_env`] takes the text: [`Registry::to_toml`] cannot keep that order.
//!
//! REF: `specs/registry/effect_language_v0.md` §3.1, §4.5, §4.6, §6, §7.3, §7.4, §11.

use std::collections::{BTreeMap, BTreeSet};

use rh_registry::effect::env::{ColumnDef, CommandDef, ParamDecl, ReadDecl, SystemDecl};
use rh_registry::effect::succession::{CompiledLaw, LawText};
use rh_registry::effect::{
    Compiled, Env, Form, LANGUAGE_VERSION, ProgramKind, Ty, Value, compile_with, kind_options,
};
use serde::{Deserialize, Serialize};
use toml::Value as Toml;

use crate::effect_cases::cell_value;
use crate::exact::Decimal;
use crate::registry::{Number, Registry};

/// `[succession_laws]`: the law ids, in ordinal order (§11), and each law's programs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessionLaws {
    /// The law ids; `title.law` stores an index into this list.
    pub ids: Vec<String>,
    /// `[succession_laws.<law>]`, by law id.
    #[serde(flatten)]
    pub programs: BTreeMap<String, LawPrograms>,
}

/// `[succession_laws.<law>]`: a law's three programs (§7.2). Each is optional here, so a law
/// missing one is refused by name ([`registry_problems`]) rather than as a parse error.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LawPrograms {
    /// `candidates`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidates: Option<String>,
    /// `eligible`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eligible: Option<String>,
    /// `order`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
}

/// `[effect_language]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectLanguage {
    /// The language version the Registry's programs are written in (§11).
    pub version: String,
    /// Every enum a program can name, by name.
    #[serde(default)]
    pub enums: BTreeMap<String, EnumSource>,
    /// The entity kind of each reference column (`entity.column` to kind).
    #[serde(default)]
    pub refs: BTreeMap<String, String>,
    /// Tables keyed by a column other than their id: entity to its key column.
    #[serde(default)]
    pub keys: BTreeMap<String, String>,
    /// Event-id columns, which no program reads.
    #[serde(default)]
    pub opaque: Opaque,
}

/// Where an enum's values come from, and its named default (§3.1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnumSource {
    /// A Registry vocabulary: an id list or a keyed table, in source order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    /// Spec columns whose `[[owns]] values` must all be equal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,
    /// The value a missing read yields.
    pub default: String,
}

/// `[effect_language.opaque]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Opaque {
    /// The columns, as `entity.column`.
    #[serde(default)]
    pub columns: Vec<String>,
}

/// A `[commands]` signature (§6, §7.3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSig {
    /// The owning system: the one whose `[[applies]]` types it.
    pub owner: String,
    /// The owner's name of the queue-key field (an action names it `person`).
    pub person: String,
    /// An outcome command (`05.1` §2 rule 2): submitted only by its sender's timer.
    #[serde(default)]
    pub outcome: bool,
    /// The other parameters, in payload order.
    #[serde(default)]
    pub params: Vec<CommandParam>,
}

/// One parameter of a command.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandParam {
    /// Its payload field name.
    pub name: String,
    /// Its language type.
    #[serde(rename = "type")]
    pub ty: String,
}

/// The ids of each keyed top-level table of a Registry's text, in source order (the Architect's
/// keyed-table ruling): `toml::Table` sorts its keys, and an enum's ordinal is its index.
///
/// # Errors
/// Text that is not TOML.
pub fn key_order(text: &str) -> Result<BTreeMap<String, Vec<String>>, String> {
    use toml::de::{DeTable, DeValue};
    let table = DeTable::parse(text).map_err(|e| e.to_string())?;
    let mut out = BTreeMap::new();
    for (name, value) in table.get_ref() {
        let DeValue::Table(t) = value.get_ref() else {
            continue;
        };
        let mut keys: Vec<(usize, String)> = t
            .iter()
            .map(|(k, _)| (k.span().start, k.get_ref().to_string()))
            .collect();
        keys.sort();
        out.insert(
            name.get_ref().to_string(),
            keys.into_iter().map(|(_, k)| k).collect(),
        );
    }
    Ok(out)
}

/// The Registry's own effect-language problems, appended after [`Registry::problems`]'s by
/// [`Registry::from_toml`] so SPR-020's refusals keep their order. Those that need the specs are
/// [`effect_problems`]'s.
#[must_use]
pub fn registry_problems(reg: &Registry) -> Vec<String> {
    let mut out = Vec::new();
    let laws = &reg.succession_laws;
    for id in laws.programs.keys() {
        if !laws.ids.contains(id) {
            out.push(format!(
                "succession_laws.{id}: programs for an id that is not in succession_laws.ids"
            ));
        }
    }
    let Some(lang) = &reg.effect_language else {
        if !laws.programs.is_empty() {
            out.push(
                "succession_laws: programs need an [effect_language] version (§11)".to_owned(),
            );
        }
        for (vid, v) in &reg.verbs {
            if v.effects.is_some() {
                out.push(format!(
                    "verbs.{vid}: effects need an [effect_language] version (§11)"
                ));
            }
        }
        return out;
    };
    if lang.version != LANGUAGE_VERSION {
        out.push(format!(
            "effect_language.version '{}': this evaluator implements {LANGUAGE_VERSION} (§11)",
            lang.version
        ));
    }
    for id in &laws.ids {
        let Some(p) = laws.programs.get(id) else {
            out.push(format!("succession_laws.{id}: no programs (§7.3)"));
            continue;
        };
        for (name, text) in [
            ("candidates", &p.candidates),
            ("eligible", &p.eligible),
            ("order", &p.order),
        ] {
            if text.is_none() {
                out.push(format!("succession_laws.{id}: no {name} program (§7.3)"));
            }
        }
    }
    for (name, e) in &lang.enums {
        match (&e.registry, e.columns.is_empty()) {
            (Some(_), false) => out.push(format!(
                "effect_language.enums.{name}: names both a Registry list and columns"
            )),
            (None, true) => out.push(format!(
                "effect_language.enums.{name}: names neither a Registry list nor columns"
            )),
            (Some(list), true) => match registry_values(reg, &BTreeMap::new(), list) {
                None => out.push(format!(
                    "effect_language.enums.{name}: Registry section '{list}' does not exist"
                )),
                Some(values) if !values.contains(&e.default) => out.push(format!(
                    "effect_language.enums.{name}: default '{}' is not one of its values {values:?}",
                    e.default
                )),
                Some(_) => {}
            },
            (None, false) => {}
        }
    }
    for (cid, c) in &reg.commands {
        if !reg.systems.contains_key(&c.owner) {
            out.push(format!(
                "commands.{cid}: owner '{}' is not a Registry system",
                c.owner
            ));
        }
    }
    for (vid, v) in &reg.verbs {
        if v.effects.is_some() && !reg.systems.contains_key(&v.emitted_by) {
            out.push(format!(
                "verbs.{vid}: a verb with effects must be emitted by a Registry system, not '{}' (Q8)",
                v.emitted_by
            ));
        }
        if v.effects.is_none() && !v.payload.is_empty() {
            out.push(format!("verbs.{vid}: a payload without effects"));
        }
    }
    out
}

/// A Registry vocabulary's ids: an id list's `ids`, or a keyed table's keys in the order of
/// `order` (source order from [`key_order`]; sorted when `order` lacks the section).
fn registry_values(
    reg: &Registry,
    order: &BTreeMap<String, Vec<String>>,
    section: &str,
) -> Option<Vec<String>> {
    if section == "succession_laws" {
        return Some(reg.succession_laws.ids.clone());
    }
    if let Some(ids) = reg.id_list(section) {
        return Some(ids.clone());
    }
    match reg.sections.get(section)? {
        Toml::Table(t) => match t.get("ids") {
            Some(Toml::Array(a)) => a
                .iter()
                .map(|v| v.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>(),
            _ => Some(
                order
                    .get(section)
                    .cloned()
                    .unwrap_or_else(|| t.keys().cloned().collect()),
            ),
        },
        _ => None,
    }
}

/// One system spec's text, parsed: its id and its tables.
struct Spec {
    id: String,
    table: toml::Table,
}

fn parse_specs(specs: &[(String, String)]) -> Result<Vec<Spec>, Vec<String>> {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    for (name, text) in specs {
        match toml::from_str::<toml::Table>(text) {
            Ok(table) => {
                let id = table
                    .get("system")
                    .and_then(Toml::as_table)
                    .and_then(|s| s.get("id"))
                    .and_then(Toml::as_str);
                match id {
                    Some(id) => out.push(Spec {
                        id: id.to_owned(),
                        table,
                    }),
                    None => errors.push(format!("{name}: no [system] id")),
                }
            }
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

fn array<'a>(t: &'a toml::Table, key: &str) -> &'a [Toml] {
    t.get(key)
        .and_then(Toml::as_array)
        .map_or(&[], Vec::as_slice)
}

fn text<'a>(t: &'a toml::Table, key: &str) -> Option<&'a str> {
    t.get(key).and_then(Toml::as_str)
}

/// An `[[owns]]` entry's integer and id types.
const INT_TYPES: [&str; 8] = ["u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64"];

/// A Registry type name (`[commands]`, a verb payload) as a
/// language type: `int`, `bool`, `q8_8`, `q16_16`, `event`, `enum:<name>`, or an entity kind.
fn language_type(env: &mut Env, name: &str) -> Result<Ty, String> {
    Ok(match name {
        "int" => Ty::Int,
        "bool" => Ty::Bool,
        "q8_8" => Ty::Q8,
        "q16_16" => Ty::Q16,
        "event" => Ty::Event,
        _ => match name.strip_prefix("enum:") {
            Some(e) => Ty::Enum(
                env.enum_id(e)
                    .ok_or_else(|| format!("unknown enum '{e}'"))?,
            ),
            None if !name.is_empty()
                && name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_') =>
            {
                Ty::Ent(env.intern_kind(name).map_err(|e| e.0)?)
            }
            None => return Err(format!("unknown type '{name}'")),
        },
    })
}

/// The `values` an `[[owns]]` entry declares for each spec column.
fn owned_values(specs: &[Spec]) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    for spec in specs {
        for own in array(&spec.table, "owns") {
            let Some(own) = own.as_table() else { continue };
            let (Some(column), Some(values)) = (text(own, "column"), own.get("values")) else {
                continue;
            };
            let values: Vec<String> = values
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Toml::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            out.insert(column.to_owned(), values);
        }
    }
    out
}

/// Builds the checker's environment (§7.4) from the Registry, its text (for the source order of
/// its keyed tables) and the system specs' TOML text, given as `(name, text)` pairs (the name is
/// for messages only).
///
/// Columns: every `[[owns]]` entry of a known type, and nothing else (a column is in the
/// environment only when a spec owns it, G4). An id column is an `int`, or a reference of the kind
/// `[effect_language.refs]` gives it; an `event_ref` column and an `[effect_language.opaque]` one
/// are left out, since no program reads an event (G3). An enum column whose enum `[effect_language.enums]` does not
/// name is left out too. Systems: each spec's `[[reads]]` (an identity read needs no `neutral`),
/// `[[commands]]` and `[[rng]]`, with the Registry's parameters at their exact bounds. Every spec
/// system is registered.
///
/// # Errors
/// Every problem found, by name: a spec that does not parse, an enum whose columns disagree or
/// whose default is not a value, a reference to an unknown column, a type the language lacks.
#[allow(
    clippy::too_many_lines,
    reason = "one pass per §7.4 section, in the order the checker needs them"
)]
pub fn effect_env(
    reg: &Registry,
    registry_text: &str,
    specs: &[(String, String)],
) -> Result<Env, Vec<String>> {
    let order = key_order(registry_text).map_err(|e| vec![e])?;
    let specs = parse_specs(specs)?;
    let lang = reg.effect_language.clone().unwrap_or_default();
    let mut env = Env::new();
    let mut errors = Vec::new();
    let owned = owned_values(&specs);
    // Enums first: the columns and commands name them.
    let mut column_enum: BTreeMap<&str, &str> = BTreeMap::new();
    let mut registry_enum: BTreeMap<&str, &str> = BTreeMap::new();
    for (name, src) in &lang.enums {
        let values = if let Some(list) = &src.registry {
            registry_enum.insert(list.as_str(), name.as_str());
            registry_values(reg, &order, list)
        } else {
            let mut first: Option<(&String, &Vec<String>)> = None;
            for column in &src.columns {
                column_enum.insert(column.as_str(), name.as_str());
                match (owned.get(column), first) {
                    (None, _) => errors.push(format!(
                        "effect_language.enums.{name}: column '{column}' is not an enum column of a spec"
                    )),
                    (Some(v), None) => first = Some((column, v)),
                    (Some(v), Some((c0, v0))) if v != v0 => errors.push(format!(
                        "effect_language.enums.{name}: columns '{c0}' and '{column}' declare different values ({v0:?}, {v:?})"
                    )),
                    (Some(_), Some(_)) => {}
                }
            }
            first.map(|(_, v)| v.clone())
        };
        let Some(values) = values else {
            if src.registry.is_some() || src.columns.is_empty() {
                errors.push(format!("effect_language.enums.{name}: no values"));
            }
            continue;
        };
        if let Err(e) = env.add_enum(name, values, &src.default) {
            errors.push(format!("effect_language.enums.{name}: {}", e.0));
        }
    }
    // The key column of a log table gives its read key.
    let mut key_kind: BTreeMap<String, String> = BTreeMap::new();
    for (entity, column) in &lang.keys {
        let full = format!("{entity}.{column}");
        match lang.refs.get(&full) {
            Some(kind) => {
                key_kind.insert(entity.clone(), kind.clone());
            }
            None => errors.push(format!(
                "effect_language.keys.{entity}: '{full}' is not a reference column of that entity in [effect_language.refs]"
            )),
        }
    }
    let declared: BTreeSet<&str> = specs
        .iter()
        .flat_map(|s| array(&s.table, "owns"))
        .filter_map(|o| o.as_table().and_then(|o| text(o, "entity")))
        .collect();
    for (column, kind) in &lang.refs {
        if !declared.contains(kind.as_str()) {
            errors.push(format!(
                "effect_language.refs.{column}: kind '{kind}' is not an entity of a spec"
            ));
        }
    }
    let opaque: BTreeSet<&str> = lang.opaque.columns.iter().map(String::as_str).collect();
    let mut seen_refs: BTreeSet<&str> = BTreeSet::new();
    let mut seen_opaque: BTreeSet<&str> = BTreeSet::new();
    for spec in &specs {
        for own in array(&spec.table, "owns") {
            let Some(own) = own.as_table() else { continue };
            let (Some(column), Some(entity), Some(ty)) =
                (text(own, "column"), text(own, "entity"), text(own, "type"))
            else {
                errors.push(format!(
                    "{}: an [[owns]] entry without column, entity or type",
                    spec.id
                ));
                continue;
            };
            let ty = if opaque.contains(column) {
                seen_opaque.insert(column);
                None
            } else if let Some(kind) = lang.refs.get(column) {
                seen_refs.insert(column);
                if ty == "u64" {
                    match env.intern_kind(kind) {
                        Ok(k) => Some(Ty::Ent(k)),
                        Err(e) => {
                            errors.push(e.0);
                            None
                        }
                    }
                } else {
                    errors.push(format!(
                        "effect_language.refs.{column}: a reference column must be u64, not {ty}"
                    ));
                    None
                }
            } else {
                match ty {
                    t if INT_TYPES.contains(&t) => Some(Ty::Int),
                    "bool" => Some(Ty::Bool),
                    "q8_8" => Some(Ty::Q8),
                    "q16_16" => Some(Ty::Q16),
                    "enum" => {
                        let name = text(own, "registry")
                            .and_then(|r| registry_enum.get(r).copied())
                            .or_else(|| column_enum.get(column).copied());
                        name.and_then(|n| env.enum_id(n)).map(Ty::Enum)
                    }
                    _ => None,
                }
            };
            let Some(ty) = ty else { continue };
            let added = (|| {
                let entity_kind = env.intern_kind(entity)?;
                let key = match key_kind.get(entity) {
                    Some(k) => env.intern_kind(k)?,
                    None => entity_kind,
                };
                env.add_column(ColumnDef {
                    name: column.to_owned(),
                    entity: entity_kind,
                    key,
                    ty,
                    plane_a: text(own, "plane") == Some("A"),
                    owner: spec.id.clone(),
                })
            })();
            if let Err(e) = added {
                errors.push(format!("{}: {}", spec.id, e.0));
            }
        }
    }
    for column in lang.refs.keys() {
        if !seen_refs.contains(column.as_str()) {
            errors.push(format!(
                "effect_language.refs.{column}: not a column of a spec"
            ));
        }
    }
    for column in &lang.opaque.columns {
        if !seen_opaque.contains(column.as_str()) {
            errors.push(format!(
                "effect_language.opaque: '{column}' is not a column of a spec"
            ));
        }
    }
    for (name, c) in &reg.commands {
        let mut params = Vec::new();
        for p in &c.params {
            match language_type(&mut env, &p.ty) {
                Ok(ty) => params.push((p.name.clone(), ty)),
                Err(e) => errors.push(format!("commands.{name}.{}: {e}", p.name)),
            }
        }
        let added = env.add_command(CommandDef {
            name: name.clone(),
            owner: c.owner.clone(),
            key_field: c.person.clone(),
            params,
        });
        if let Err(e) = added {
            errors.push(format!("commands.{name}: {}", e.0));
        }
    }
    for spec in &specs {
        match system_decl(reg, &env, spec) {
            Ok(decl) => {
                env.systems.insert(spec.id.clone(), decl);
            }
            Err(mut e) => errors.append(&mut e),
        }
    }
    env.registered = specs.iter().map(|s| s.id.clone()).collect();
    if errors.is_empty() {
        Ok(env)
    } else {
        Err(errors)
    }
}

/// One spec's declarations (§4.5, §4.6, §4.7).
fn system_decl(reg: &Registry, env: &Env, spec: &Spec) -> Result<SystemDecl, Vec<String>> {
    let mut decl = SystemDecl::default();
    let mut errors = Vec::new();
    for read in array(&spec.table, "reads") {
        let Some(read) = read.as_table() else {
            continue;
        };
        let Some(column) = text(read, "column") else {
            errors.push(format!("{}: a [[reads]] entry without a column", spec.id));
            continue;
        };
        let identity = text(read, "use") == Some("identity");
        let neutral = match (read.get("neutral"), env.column_id(column)) {
            (Some(raw), Some(id)) => {
                let ty = env.column(id).map(|c| c.ty.clone());
                match ty.map(|ty| cell_value(env, &ty, raw)) {
                    Some(Ok(v)) => Some(v),
                    Some(Err(e)) => {
                        errors.push(format!("{}: [[reads]] {column}: neutral {e}", spec.id));
                        None
                    }
                    None => None,
                }
            }
            _ => None,
        };
        decl.reads
            .insert(column.to_owned(), ReadDecl { neutral, identity });
    }
    for send in array(&spec.table, "commands") {
        let Some(send) = send.as_table() else {
            continue;
        };
        if let (Some(command), Some(target)) = (text(send, "command"), text(send, "target")) {
            decl.sends.insert(command.to_owned(), target.to_owned());
        }
    }
    for rng in array(&spec.table, "rng") {
        if let Some(purpose) = rng.as_table().and_then(|r| text(r, "purpose")) {
            decl.rng.insert(purpose.to_owned());
        }
    }
    if let Some(sys) = reg.systems.get(&spec.id) {
        for (name, bounds) in &sys.params {
            let int = matches!((bounds.lo, bounds.hi), (Number::Int(_), Number::Int(_)));
            let frac = if int { 0 } else { 16 };
            match sys.bound_literals.get(name) {
                Some((Ok(lo), Ok(hi))) => {
                    decl.params.insert(
                        name.clone(),
                        ParamDecl {
                            int,
                            lo: scaled(lo, frac),
                            hi: scaled(hi, frac),
                        },
                    );
                }
                _ => errors.push(format!(
                    "systems.{}.params.{name}: no exact bounds (load the Registry from text)",
                    spec.id
                )),
            }
        }
    }
    if errors.is_empty() {
        Ok(decl)
    } else {
        Err(errors)
    }
}

/// `value · 2^frac` rounded half to even, saturated to `i128`.
fn scaled(d: &Decimal, frac: u32) -> i128 {
    d.to_fixed(frac).unwrap_or(if d.is_negative() {
        i128::MIN
    } else {
        i128::MAX
    })
}

/// The Registry's programs, compiled against an environment.
pub struct Programs {
    /// The succession laws, by id, in `ids` order.
    pub laws: Vec<(String, CompiledLaw)>,
    /// The verb effects programs, by verb id.
    pub verbs: BTreeMap<String, Compiled>,
}

impl Programs {
    /// The number of programs: three per law, one per verb.
    #[must_use]
    pub fn count(&self) -> usize {
        self.laws.len() * 3 + self.verbs.len()
    }
}

/// The system that owns `title.law`, which declares the laws' reads and draws (§7.2).
fn law_system(env: &Env) -> String {
    env.column_id("title.law")
        .and_then(|c| env.column(c))
        .map_or_else(|| "M7.dynasty".to_owned(), |c| c.owner.clone())
}

/// Compiles every Registry program (the laws, the verbs' `effects`) against `env`.
///
/// # Errors
/// Every program that does not compile (naming the law or verb, the program and the §10 code), a
/// constant `death_cause.none` sent to `ScheduleDeath` (Q13), and two verb programs of one system
/// that share a draw site (§4.6). Two laws may share a site: a title has one law.
pub fn compile_programs(reg: &Registry, env: &Env) -> Result<Programs, Vec<String>> {
    let mut errors = Vec::new();
    let mut laws = Vec::new();
    let system = law_system(env);
    for id in &reg.succession_laws.ids {
        let Some(p) = reg.succession_laws.programs.get(id) else {
            continue;
        };
        let (Some(candidates), Some(eligible), Some(order)) =
            (&p.candidates, &p.eligible, &p.order)
        else {
            continue;
        };
        let text = LawText {
            candidates,
            eligible,
            order,
        };
        match CompiledLaw::compile(env, &system, text) {
            Ok(law) => laws.push((id.clone(), law)),
            Err((program, r)) => errors.push(format!(
                "succession_laws.{id}.{}: {} {}",
                program.as_str(),
                r.code,
                r.message
            )),
        }
    }
    let mut verbs = BTreeMap::new();
    for (vid, v) in &reg.verbs {
        let Some(effects) = &v.effects else { continue };
        match compile_verb(env, &v.emitted_by, effects, &v.payload) {
            Ok(c) => {
                verbs.insert(vid.clone(), c);
            }
            Err(e) => errors.push(format!("verbs.{vid}.effects: {e}")),
        }
    }
    let death_none = env.enum_id("death_cause").and_then(|e| {
        let i = env.enum_def(e)?.values.iter().position(|v| v == "none")?;
        Some(Value::Enum(e, u32::try_from(i).ok()?))
    });
    let schedule_death = env.command_id("ScheduleDeath");
    let mut sites: BTreeMap<(&str, &str), &str> = BTreeMap::new();
    for (vid, c) in &verbs {
        for (command, param, value) in c.constant_enum_args() {
            if Some(command) == schedule_death
                && param == "cause"
                && Some(&value) == death_none.as_ref()
            {
                errors.push(format!(
                    "verbs.{vid}.effects: sends the constant death_cause.none to ScheduleDeath, which M2 refuses (Q13)"
                ));
            }
        }
        for site in c.draw_sites() {
            if let Some(other) = sites.insert((c.system(), site.as_str()), vid.as_str()) {
                errors.push(format!(
                    "verbs.{other} and verbs.{vid} ({}): E_RNG_REUSE, both draw at {site} (§4.6)",
                    c.system()
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(Programs { laws, verbs })
    } else {
        Err(errors)
    }
}

/// A verb's effects program, with its payload fields bound after §7.1's variables.
fn compile_verb(
    env: &Env,
    system: &str,
    text: &str,
    payload: &BTreeMap<String, String>,
) -> Result<Compiled, String> {
    let mut opts = kind_options(ProgramKind::Effects, env).map_err(|r| r.to_string())?;
    let mut env_types = env.clone();
    for (name, ty) in payload {
        opts.scope
            .push((name.clone(), language_type(&mut env_types, ty)?));
    }
    if env_types.kinds().len() != env.kinds().len() {
        return Err("a payload names an entity kind no column has".to_owned());
    }
    opts.check.outcome_payload = !payload.is_empty();
    compile_with(Form::Effects, text, env, system, &opts)
        .map_err(|r| format!("{} {}", r.code, r.message))
}

/// Every effect-language problem of a Registry against the system specs: the environment's
/// ([`effect_env`]) and the programs' ([`compile_programs`]). Run after [`Registry::from_toml`]
/// accepted the Registry (which holds [`registry_problems`]).
#[must_use]
pub fn effect_problems(
    reg: &Registry,
    registry_text: &str,
    specs: &[(String, String)],
) -> Vec<String> {
    match effect_env(reg, registry_text, specs) {
        Err(e) => e,
        Ok(env) => compile_programs(reg, &env).err().unwrap_or_default(),
    }
}
