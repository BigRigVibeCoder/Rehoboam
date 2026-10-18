//! The golden-format case runner of the effect language (SPR-017 T-05): the Rust side of
//! `specs/registry/effect_ref.py`.
//!
//! It reads a golden-format file (`effect_language_golden_v0.toml`, or any file of its format),
//! builds the plane-A [`Env`] from its `[registry]` slice and fixture worlds from `[worlds.*]`
//! (tables, `[draws]`, the `fan` generator), runs each case by kind with its `bind`, `payload`,
//! `events`, `evaluations`, `extra_reads` and `extra_sends`, and renders the result in the file
//! header's exact text. `crates/rh-manifest/tests/effect_golden.rs` and `rh-cli effect eval` use it.
//!
//! Tool plane: it reads TOML, and reads a TOML float (a parameter bound, a cell) by its shortest
//! round-trip decimal, as `exact.rs` does; only exact fixed point reaches the evaluator.
//!
//! REF: `specs/registry/effect_language_v0.md` §7.4, §13; `specs/registry/effect_ref.py`
//! (`load_registry`, `load_world`, `fan_rows`, `run_case`).

use std::collections::{BTreeMap, BTreeSet};

use rh_registry::effect::env::{ColumnDef, CommandDef, ParamDecl, ReadDecl, SystemDecl};
use rh_registry::effect::host::{Draws, Host, Indexes, ReadOutcome, Reads};
use rh_registry::effect::succession::{CompiledLaw, LawText};
use rh_registry::effect::value::{fixed_from_bits, render_key, render_scalar};
use rh_registry::effect::{
    CheckOptions, ColumnId, CompileOptions, Diagnostics, DrawLedger, Env, EvalError, EvalOptions,
    Form, KindId, LANGUAGE_VERSION, Mutant, Output, Ty, Value, VerbStep, compile_with,
    render_actions,
};
use toml::Value as Toml;

use crate::exact::Decimal;

/// The case kinds of the golden format.
pub const CASE_KINDS: [&str; 5] = ["expr", "order", "candidates", "effects", "succession"];

/// The diagnostics a case may pin, by their golden names.
pub const DIAGNOSTICS: [&str; 4] = [
    "missing_reads",
    "dropped_actions",
    "neutral_reads",
    "truncated",
];

/// A loaded golden-format file.
#[derive(Clone, Debug)]
pub struct Golden {
    /// The environment of its `[registry]` slice.
    pub env: Env,
    /// The succession laws' programs, by law.
    pub laws: BTreeMap<String, [String; 3]>,
    worlds: BTreeMap<String, toml::Table>,
    /// The entity kinds whose rows are keyed by a column other than `id` (the life-course log by
    /// `person_id`).
    keys: BTreeMap<String, String>,
    /// The cases, in file order.
    pub cases: Vec<toml::Table>,
}

/// One case's outcome: the rendered result, and the diagnostics when it evaluated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseOutcome {
    /// `bool:true`, `line:[…]`, `error:E_…`, `invariant:E_…`, …
    pub text: String,
    /// The evaluator's diagnostics; `None` for a rejected program or an invariant.
    pub diags: Option<Diagnostics>,
}

impl CaseOutcome {
    /// A pinned diagnostic by its golden name, or −1 when the case did not evaluate.
    #[must_use]
    pub fn diagnostic(&self, name: &str) -> i128 {
        let Some(d) = self.diags else { return -1 };
        i128::from(match name {
            "missing_reads" => d.missing_reads,
            "dropped_actions" => d.dropped_actions,
            "neutral_reads" => d.neutral_reads,
            _ => d.truncated_relations,
        })
    }
}

fn s<'a>(t: &'a toml::Table, key: &str) -> Result<&'a str, String> {
    t.get(key)
        .and_then(Toml::as_str)
        .ok_or_else(|| format!("missing string `{key}`"))
}

fn table<'a>(t: &'a toml::Table, key: &str) -> Result<&'a toml::Table, String> {
    t.get(key)
        .and_then(Toml::as_table)
        .ok_or_else(|| format!("missing table `{key}`"))
}

/// The exact decimal of a TOML number or a numeric string.
fn decimal(v: &Toml) -> Result<Decimal, String> {
    match v {
        Toml::Integer(i) => Ok(Decimal::from_int(i128::from(*i))),
        Toml::Float(f) => Decimal::parse_toml_float(&format!("{f}")).map_err(|e| e.to_string()),
        Toml::String(text) => Decimal::parse_python_float(text).map_err(|e| e.to_string()),
        other => Err(format!("{other} is not a number")),
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

fn parse_type(env: &mut Env, spec: &toml::Table) -> Result<Ty, String> {
    Ok(match s(spec, "type")? {
        "bool" => Ty::Bool,
        "int" => Ty::Int,
        "q8_8" => Ty::Q8,
        "q16_16" => Ty::Q16,
        "event" => Ty::Event,
        "enum" => {
            let name = s(spec, "enum")?;
            Ty::Enum(
                env.enum_id(name)
                    .ok_or_else(|| format!("unknown enum `{name}`"))?,
            )
        }
        "ref" => Ty::Ent(env.intern_kind(s(spec, "ref")?).map_err(|e| e.0)?),
        other => return Err(format!("unknown golden type `{other}`")),
    })
}

/// A table cell or a `neutral` literal as a value of a column's type (the reference's
/// `cell_value`).
///
/// # Errors
/// A value that is not of the column's type.
pub fn cell_value(env: &Env, ty: &Ty, raw: &Toml) -> Result<Value, String> {
    Ok(match ty {
        Ty::Bool => Value::Bool(match raw {
            Toml::Boolean(b) => *b,
            Toml::Integer(i) => *i != 0,
            Toml::String(t) => !t.is_empty(),
            other => return Err(format!("{other} is not a bool")),
        }),
        Ty::Q8 => fixed_from_bits(ty, scaled(&decimal(raw)?, 8)).ok_or("fixed")?,
        Ty::Q16 => fixed_from_bits(ty, scaled(&decimal(raw)?, 16)).ok_or("fixed")?,
        Ty::Enum(e) => {
            let def = env.enum_def(*e).ok_or("enum")?;
            let text = raw
                .as_str()
                .ok_or_else(|| format!("{raw} is not an enum value"))?;
            let i = def
                .values
                .iter()
                .position(|v| v == text)
                .ok_or_else(|| format!("`{text}` is not a {} value", def.name))?;
            Value::Enum(*e, u32::try_from(i).map_err(|_| "enum")?)
        }
        Ty::Int => Value::Int(
            raw.as_integer()
                .ok_or_else(|| format!("{raw} is not an integer"))?,
        ),
        Ty::Ent(k) => {
            let id = raw
                .as_integer()
                .ok_or_else(|| format!("{raw} is not an integer"))?;
            Value::Ent(
                *k,
                u64::try_from(id).map_err(|_| format!("{id} is not an entity id"))?,
            )
        }
        other => return Err(format!("no cell of type {}", env.type_name(other))),
    })
}

fn load_env(raw: &toml::Table) -> Result<Env, String> {
    let mut env = Env::new();
    for (name, def) in table(raw, "enums")? {
        let def = def.as_table().ok_or("enum")?;
        let values: Vec<String> = def
            .get("values")
            .and_then(Toml::as_array)
            .ok_or("enum values")?
            .iter()
            .map(|v| v.as_str().map(str::to_owned).ok_or("enum value"))
            .collect::<Result<_, _>>()?;
        env.add_enum(name, values, s(def, "default")?)
            .map_err(|e| e.0)?;
    }
    let entities = table(raw, "entities")?;
    for kind in entities
        .get("kinds")
        .and_then(Toml::as_array)
        .ok_or("entity kinds")?
    {
        env.intern_kind(kind.as_str().ok_or("kind")?)
            .map_err(|e| e.0)?;
    }
    for (name, c) in table(raw, "columns")? {
        let c = c.as_table().ok_or("column")?;
        let entity = env.intern_kind(s(c, "entity")?).map_err(|e| e.0)?;
        let key = match c.get("key").and_then(Toml::as_str) {
            Some(k) => env.intern_kind(k).map_err(|e| e.0)?,
            None => entity,
        };
        let ty = parse_type(&mut env, c)?;
        env.add_column(ColumnDef {
            name: name.clone(),
            entity,
            key,
            ty,
            plane_a: s(c, "plane")? == "A",
            owner: s(c, "owner")?.to_owned(),
        })
        .map_err(|e| e.0)?;
    }
    for (name, c) in table(raw, "commands")? {
        let c = c.as_table().ok_or("command")?;
        let mut params = Vec::new();
        for p in c
            .get("params")
            .and_then(Toml::as_array)
            .map_or(&[][..], Vec::as_slice)
        {
            let p = p.as_table().ok_or("command parameter")?;
            params.push((s(p, "name")?.to_owned(), parse_type(&mut env, p)?));
        }
        env.add_command(CommandDef {
            name: name.clone(),
            owner: s(c, "owner")?.to_owned(),
            key_field: s(c, "person")?.to_owned(),
            params,
        })
        .map_err(|e| e.0)?;
    }
    for (name, sys) in table(raw, "systems")? {
        let sys = sys.as_table().ok_or("system")?;
        let decl = system_decl(&env, sys)?;
        env.systems.insert(name.clone(), decl);
    }
    let registered = table(raw, "registered")?
        .get("systems")
        .and_then(Toml::as_array)
        .ok_or("registered")?;
    env.registered = registered
        .iter()
        .filter_map(Toml::as_str)
        .map(str::to_owned)
        .collect();
    Ok(env)
}

fn system_decl(env: &Env, sys: &toml::Table) -> Result<SystemDecl, String> {
    let mut decl = SystemDecl::default();
    let neutral = sys.get("neutral").and_then(Toml::as_table);
    for name in sys
        .get("reads")
        .and_then(Toml::as_array)
        .map_or(&[][..], Vec::as_slice)
    {
        let name = name.as_str().ok_or("read")?;
        let n = neutral.and_then(|t| t.get(name));
        decl.reads.insert(
            name.to_owned(),
            ReadDecl {
                neutral: neutral_value(env, name, n)?,
                identity: false,
            },
        );
    }
    for send in sys
        .get("sends")
        .and_then(Toml::as_array)
        .map_or(&[][..], Vec::as_slice)
    {
        let send = send.as_table().ok_or("send")?;
        decl.sends.insert(
            s(send, "command")?.to_owned(),
            s(send, "target")?.to_owned(),
        );
    }
    for purpose in sys
        .get("rng")
        .and_then(Toml::as_array)
        .map_or(&[][..], Vec::as_slice)
    {
        decl.rng.insert(purpose.as_str().ok_or("rng")?.to_owned());
    }
    if let Some(params) = sys.get("params").and_then(Toml::as_table) {
        for (name, bounds) in params {
            let pair = bounds.as_array().filter(|a| a.len() == 2).ok_or("bounds")?;
            let int = pair.iter().all(Toml::is_integer);
            let frac = if int { 0 } else { 16 };
            let lo = scaled(&decimal(&pair[0])?, frac);
            let hi = scaled(&decimal(&pair[1])?, frac);
            decl.params.insert(name.clone(), ParamDecl { int, lo, hi });
        }
    }
    Ok(decl)
}

fn neutral_value(env: &Env, column: &str, raw: Option<&Toml>) -> Result<Option<Value>, String> {
    let (Some(raw), Some(id)) = (raw, env.column_id(column)) else {
        return Ok(None);
    };
    let ty = env.column(id).map(|c| c.ty.clone()).ok_or("column")?;
    cell_value(env, &ty, raw).map(Some)
}

impl Golden {
    /// Loads a golden-format file's text: its version, the Registry slice, worlds and cases.
    ///
    /// # Errors
    /// A version other than the evaluator's (§11), a malformed slice, a duplicate case id or an
    /// unknown case kind (so no case is skipped silently).
    pub fn parse(text: &str) -> Result<Self, String> {
        let raw: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
        let version = raw.get("version").and_then(Toml::as_str).unwrap_or("");
        if version != LANGUAGE_VERSION {
            return Err(format!(
                "golden file version {version} != {LANGUAGE_VERSION}"
            ));
        }
        let registry = table(&raw, "registry")?;
        let env = load_env(registry)?;
        let keys = table(registry, "entities")?
            .get("keys")
            .and_then(Toml::as_table)
            .into_iter()
            .flatten()
            .map(|(k, v)| {
                v.as_str()
                    .map(|v| (k.clone(), v.to_owned()))
                    .ok_or("entity key")
            })
            .collect::<Result<_, _>>()?;
        let mut laws = BTreeMap::new();
        if let Some(programs) = registry.get("programs").and_then(Toml::as_table) {
            for (law, p) in programs
                .get("succession_laws")
                .and_then(Toml::as_table)
                .into_iter()
                .flatten()
            {
                let p = p.as_table().ok_or("law")?;
                laws.insert(
                    law.clone(),
                    [
                        s(p, "candidates")?.to_owned(),
                        s(p, "eligible")?.to_owned(),
                        s(p, "order")?.to_owned(),
                    ],
                );
            }
        }
        let worlds = table(&raw, "worlds")?
            .iter()
            .map(|(k, v)| v.as_table().cloned().map(|t| (k.clone(), t)).ok_or("world"))
            .collect::<Result<_, _>>()?;
        let mut cases = Vec::new();
        let mut seen = BTreeSet::new();
        for case in raw
            .get("case")
            .and_then(Toml::as_array)
            .map_or(&[][..], Vec::as_slice)
        {
            let case = case.as_table().ok_or("case")?.clone();
            let id = s(&case, "id")?.to_owned();
            if !seen.insert(id.clone()) {
                return Err(format!("duplicate case id {id}"));
            }
            let kind = s(&case, "kind")?;
            if !CASE_KINDS.contains(&kind) {
                return Err(format!("case {id}: unknown kind {kind}"));
            }
            cases.push(case);
        }
        Ok(Self {
            env,
            laws,
            worlds,
            keys,
            cases,
        })
    }

    /// Builds the fixture world `name`.
    ///
    /// # Errors
    /// An unknown world or a malformed table.
    pub fn world(&self, env: &Env, name: &str, mutant: Mutant) -> Result<FixtureWorld, String> {
        let raw = self
            .worlds
            .get(name)
            .ok_or_else(|| format!("unknown world {name}"))?;
        FixtureWorld::load(env, raw, &self.keys, mutant)
    }

    /// Runs one case.
    ///
    /// # Errors
    /// A crash: a malformed case, or an evaluation fault that is not a language invariant (the
    /// reference's crash).
    pub fn run_case(&self, case: &toml::Table, opts: EvalOptions) -> Result<CaseOutcome, String> {
        run_case(self, case, opts)
    }
}

/// A fixture snapshot: entity tables, parameters, draws and the calendar.
#[derive(Clone, Debug, Default)]
pub struct FixtureWorld {
    /// `now`.
    pub now: i64,
    days_per_year: i64,
    tables: BTreeMap<KindId, BTreeMap<u64, BTreeMap<ColumnId, Value>>>,
    params: BTreeMap<String, BTreeMap<String, Value>>,
    draws: BTreeMap<(String, KindId, u64, u64), u64>,
    children: BTreeMap<u64, Vec<u64>>,
    /// A test-only draw source for keys a generated world does not list (T-08).
    draw_seed: Option<u64>,
}

/// The rows of a `[worlds.<name>.fan]` regular tree (the reference's `fan_rows`): person 1 the
/// root, breadth-first ids, the father of `i >= 2` is `(i - 2) div fanout + 1`, every person male
/// with an unknown mother, one generation per level; with `dead_leaves` the last generation is
/// compacted (life-course rows only).
fn fan_rows(fan: &toml::Table) -> Result<Vec<(String, toml::Table)>, String> {
    let int = |k: &str, d: i64| {
        fan.get(k)
            .map_or(Ok(d), |v| v.as_integer().ok_or(format!("fan {k}")))
    };
    let fanout = int("fanout", 0)?;
    let depth = int("depth", 0)?;
    let generation = int("generation_days", 10_800)? * rh_core::time::MINUTES_PER_DAY;
    let dead_leaves = fan
        .get("dead_leaves")
        .and_then(Toml::as_bool)
        .unwrap_or(false);
    let mut rows = Vec::new();
    let (mut first, mut size) = (1_i64, 1_i64);
    for g in 0..=depth {
        for i in first..first + size {
            let father = if i == 1 { 0 } else { (i - 2) / fanout + 1 };
            let birth = -(depth - g) * generation;
            let leaf_dead = dead_leaves && g == depth;
            let mut log = toml::Table::new();
            for (k, v) in [
                ("id", Toml::Integer(100_000 + i)),
                ("person_id", Toml::Integer(i)),
                ("mother", Toml::Integer(0)),
                ("father", Toml::Integer(father)),
                ("birth_tick", Toml::Integer(birth)),
                ("sex", Toml::String("male".into())),
                (
                    "death_cause",
                    Toml::String(if leaf_dead { "natural" } else { "none" }.into()),
                ),
            ] {
                log.insert(k.into(), v);
            }
            rows.push(("life_course".to_owned(), log));
            if !leaf_dead {
                let mut person = toml::Table::new();
                for (k, v) in [
                    ("id", Toml::Integer(i)),
                    ("alive", Toml::Boolean(true)),
                    ("sex", Toml::String("male".into())),
                    ("birth_tick", Toml::Integer(birth)),
                    ("death_tick", Toml::Integer(0)),
                    ("death_cause", Toml::String("none".into())),
                    ("mother", Toml::Integer(0)),
                    ("father", Toml::Integer(father)),
                ] {
                    person.insert(k.into(), v);
                }
                rows.push(("person".to_owned(), person));
            }
        }
        first += size;
        size *= fanout;
    }
    Ok(rows)
}

/// Splits a draw key `purpose/kind:id/k`.
fn draw_key(env: &Env, key: &str) -> Result<(String, KindId, u64, u64), String> {
    let bad = || format!("draw key {key}");
    let mut parts = key.split('/');
    let (Some(purpose), Some(entity), Some(k), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(bad());
    };
    let (kind, id) = entity.split_once(':').ok_or_else(bad)?;
    let kind = env.kind(kind).ok_or_else(bad)?;
    let id: u64 = id.parse().map_err(|_| bad())?;
    let k: i64 = k.parse().map_err(|_| bad())?;
    Ok((purpose.to_owned(), kind, id, k.cast_unsigned()))
}

impl FixtureWorld {
    #[allow(
        clippy::too_many_lines,
        reason = "the reference's load_world in one place: tables, the fan, params, draws and the index"
    )]
    fn load(
        env: &Env,
        raw: &toml::Table,
        keys: &BTreeMap<String, String>,
        mutant: Mutant,
    ) -> Result<Self, String> {
        let int = |k: &str| {
            raw.get(k)
                .and_then(Toml::as_integer)
                .ok_or_else(|| format!("world `{k}`"))
        };
        let mut world = Self {
            now: int("now")?,
            days_per_year: int("days_per_year")?,
            ..Self::default()
        };
        let mut rows: Vec<(String, toml::Table)> = Vec::new();
        let generated = match raw.get("fan").and_then(Toml::as_table) {
            Some(fan) => fan_rows(fan)?,
            None => Vec::new(),
        };
        let generated_kinds: BTreeSet<String> = generated.iter().map(|(k, _)| k.clone()).collect();
        for (kind, list) in raw {
            if generated_kinds.contains(kind) {
                continue;
            }
            if let Some(list) = list.as_array() {
                for row in list {
                    rows.push((kind.clone(), row.as_table().ok_or("row")?.clone()));
                }
            }
        }
        rows.extend(generated);
        let by_row_id = cfg!(feature = "mutant-effect") && mutant == Mutant::LogByRowId;
        for (kind_name, row) in rows {
            let kind = env
                .kind(&kind_name)
                .ok_or_else(|| format!("unknown kind {kind_name}"))?;
            let mut cells = BTreeMap::new();
            for (col, raw_cell) in &row {
                if col == "id" {
                    continue;
                }
                let name = format!("{kind_name}.{col}");
                let id = env
                    .column_id(&name)
                    .ok_or_else(|| format!("unknown column {name}"))?;
                let ty = env.column(id).map(|c| c.ty.clone()).ok_or("column")?;
                cells.insert(id, cell_value(env, &ty, raw_cell)?);
            }
            let key_column = keys
                .get(&kind_name)
                .filter(|_| !(by_row_id && kind_name == "life_course"));
            let key = if let Some(key_column) = key_column {
                let name = format!("{kind_name}.{key_column}");
                let col = env
                    .column_id(&name)
                    .ok_or_else(|| format!("unknown key column {name}"))?;
                match cells.get(&col) {
                    Some(v) => u64::try_from(rh_registry::effect::value::scalar_int_pub(v))
                        .map_err(|_| "key")?,
                    None => return Err(format!("a {kind_name} row without {key_column}")),
                }
            } else {
                let id = row
                    .get("id")
                    .and_then(Toml::as_integer)
                    .ok_or_else(|| format!("a {kind_name} row without id"))?;
                u64::try_from(id).map_err(|_| "row id")?
            };
            world.tables.entry(kind).or_default().insert(key, cells);
        }
        if let Some(params) = raw.get("params").and_then(Toml::as_table) {
            for (system, values) in params {
                let decl = env
                    .systems
                    .get(system)
                    .ok_or_else(|| format!("params of unknown system {system}"))?;
                let mut out = BTreeMap::new();
                for (name, v) in values.as_table().ok_or("params")? {
                    let p = decl
                        .params
                        .get(name)
                        .ok_or_else(|| format!("unknown parameter {system}.{name}"))?;
                    let value = if p.int {
                        Value::Int(match v {
                            Toml::Integer(i) => *i,
                            Toml::String(t) => {
                                t.parse().map_err(|_| format!("parameter {name}"))?
                            }
                            _ => return Err(format!("parameter {name}")),
                        })
                    } else {
                        fixed_from_bits(&Ty::Q16, scaled(&decimal(v)?, 16)).ok_or("fixed")?
                    };
                    out.insert(name.clone(), value);
                }
                world.params.insert(system.clone(), out);
            }
        }
        if let Some(draws) = raw.get("draws").and_then(Toml::as_table) {
            for (key, v) in draws {
                let u: u64 = match v {
                    Toml::String(t) => t.parse().map_err(|_| format!("draw {key}"))?,
                    Toml::Integer(i) => u64::try_from(*i).map_err(|_| format!("draw {key}"))?,
                    _ => return Err(format!("draw {key}")),
                };
                world.draws.insert(draw_key(env, key)?, u);
            }
        }
        world.draw_seed = raw
            .get("draw_seed")
            .and_then(Toml::as_integer)
            .map(i64::cast_unsigned);
        if let (Some(lc), Some(mother), Some(father)) = (
            env.kind("life_course"),
            env.column_id("life_course.mother"),
            env.column_id("life_course.father"),
        ) {
            let mut index: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
            for (&pid, row) in world.tables.get(&lc).into_iter().flatten() {
                for col in [father, mother] {
                    if let Some(Value::Ent(_, parent)) = row.get(&col)
                        && *parent != 0
                    {
                        index.entry(*parent).or_default().insert(pid);
                    }
                }
            }
            world.children = index
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().collect()))
                .collect();
        }
        Ok(world)
    }

    /// The world as a [`Host`] over `env`.
    #[must_use]
    pub fn host<'a>(&'a self, env: &'a Env) -> FixtureHost<'a> {
        FixtureHost { env, world: self }
    }
}

/// The fixture world as a [`Host`].
pub struct FixtureHost<'a> {
    env: &'a Env,
    world: &'a FixtureWorld,
}

impl Reads for FixtureHost<'_> {
    fn read(&self, column: ColumnId, id: u64) -> ReadOutcome {
        let Some(kind) = self.env.column(column).map(|c| c.entity) else {
            return ReadOutcome::Missing;
        };
        match self
            .world
            .tables
            .get(&kind)
            .and_then(|t| t.get(&id))
            .and_then(|r| r.get(&column))
        {
            Some(v) => ReadOutcome::Row(v.clone()),
            None => ReadOutcome::Missing,
        }
    }

    fn exists(&self, kind: KindId, id: u64) -> bool {
        self.world
            .tables
            .get(&kind)
            .is_some_and(|t| t.contains_key(&id))
    }

    fn ids(&self, kind: KindId) -> Vec<u64> {
        self.world
            .tables
            .get(&kind)
            .map(|t| t.keys().copied().collect())
            .unwrap_or_default()
    }
}

impl Indexes for FixtureHost<'_> {
    fn children(&self, person: u64) -> Vec<u64> {
        self.world
            .children
            .get(&person)
            .cloned()
            .unwrap_or_default()
    }
}

/// `SplitMix64`'s finaliser.
#[must_use]
pub fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The `draw_seed` fold of a draw key (T-08; the same fold as `effect_ref.py`'s): `SplitMix64` over
/// the seed, then each of the purpose's bytes, the kind's bytes, the id and `k`, in that order.
#[must_use]
pub fn seeded_draw(seed: u64, purpose: &str, kind: &str, id: u64, k: u64) -> u64 {
    let mut h = splitmix64(seed);
    for b in purpose
        .bytes()
        .chain(*b"/")
        .chain(kind.bytes())
        .chain(*b"/")
    {
        h = splitmix64(h ^ u64::from(b));
    }
    h = splitmix64(h ^ id);
    splitmix64(h ^ k)
}

impl Draws for FixtureHost<'_> {
    fn draw(&self, purpose: &str, kind: KindId, id: u64, k: u64) -> Option<u64> {
        if let Some(&u) = self.world.draws.get(&(purpose.to_owned(), kind, id, k)) {
            return Some(u);
        }
        self.world
            .draw_seed
            .map(|seed| seeded_draw(seed, purpose, self.env.kind_name(kind), id, k))
    }
}

impl Host for FixtureHost<'_> {
    fn param(&self, system: &str, name: &str) -> Option<Value> {
        self.world.params.get(system)?.get(name).cloned()
    }

    fn days_per_year(&self) -> i64 {
        self.world.days_per_year
    }
}

/// A case binding: `kind:id`, an integer or a boolean, with its type.
fn parse_binding(env: &mut Env, raw: &Toml) -> Result<(Ty, Value), String> {
    Ok(match raw {
        Toml::Boolean(b) => (Ty::Bool, Value::Bool(*b)),
        Toml::Integer(i) => (Ty::Int, Value::Int(*i)),
        Toml::String(text) => {
            let (kind, id) = text
                .split_once(':')
                .ok_or_else(|| format!("binding {text}"))?;
            let kind = env.intern_kind(kind).map_err(|e| e.0)?;
            let id: u64 = id.parse().map_err(|_| format!("binding {text}"))?;
            (Ty::Ent(kind), Value::Ent(kind, id))
        }
        other => return Err(format!("binding {other}")),
    })
}

fn result_type(text: &str) -> Result<Ty, String> {
    Ok(match text {
        "bool" => Ty::Bool,
        "int" => Ty::Int,
        "q8_8" => Ty::Q8,
        "q16_16" => Ty::Q16,
        other => return Err(format!("result type {other}")),
    })
}

fn set_binding(
    scope: &mut Vec<(String, Ty)>,
    values: &mut Vec<Value>,
    name: &str,
    ty: Ty,
    v: Value,
) {
    if let Some(i) = scope.iter().position(|(n, _)| n == name) {
        scope[i].1 = ty;
        values[i] = v;
    } else {
        scope.push((name.to_owned(), ty));
        values.push(v);
    }
}

enum Fault {
    Rejected(String),
    Invariant(String),
    Crash(String),
}

impl From<EvalError> for Fault {
    fn from(e: EvalError) -> Self {
        match e.invariant() {
            Some(code) => Self::Invariant(code.to_owned()),
            None => Self::Crash(e.to_string()),
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the reference's prepare and run_case in one place"
)]
fn run_case(golden: &Golden, case: &toml::Table, opts: EvalOptions) -> Result<CaseOutcome, String> {
    let id = s(case, "id")?;
    let kind = s(case, "kind")?;
    let system = s(case, "system")?.to_owned();
    let mut env = golden.env.clone();
    // extra_reads (column -> neutral) and extra_sends give the case a variant of its system.
    let extra_reads = case.get("extra_reads").and_then(Toml::as_table);
    let extra_sends = case.get("extra_sends").and_then(Toml::as_array);
    if extra_reads.is_some() || extra_sends.is_some() {
        let mut decl = env
            .systems
            .get(&system)
            .cloned()
            .ok_or_else(|| format!("{id}: unknown system {system}"))?;
        for (column, raw) in extra_reads.into_iter().flatten() {
            let neutral = neutral_value(&env, column, Some(raw))?;
            decl.reads.insert(
                column.clone(),
                ReadDecl {
                    neutral,
                    identity: false,
                },
            );
        }
        for send in extra_sends.into_iter().flatten() {
            let send = send.as_table().ok_or("send")?;
            decl.sends.insert(
                s(send, "command")?.to_owned(),
                s(send, "target")?.to_owned(),
            );
        }
        env.systems.insert(system.clone(), decl);
    }
    let world_name = case
        .get("world")
        .and_then(Toml::as_str)
        .unwrap_or("dynasty");
    let world = golden.world(&env, world_name, opts.mutant)?;
    let mut scope: Vec<(String, Ty)> = vec![("now".into(), Ty::Int)];
    let mut values: Vec<Value> = vec![Value::Int(world.now)];
    if kind == "effects" {
        set_binding(
            &mut scope,
            &mut values,
            "event",
            Ty::Event,
            Value::Event(rh_registry::effect::EventArg::Own),
        );
    }
    for (name, raw) in case
        .get("bind")
        .and_then(Toml::as_table)
        .into_iter()
        .flatten()
    {
        let (ty, v) = parse_binding(&mut env, raw)?;
        set_binding(&mut scope, &mut values, name, ty, v);
    }
    let payload = case.get("payload").and_then(Toml::as_table);
    for (name, raw) in payload.into_iter().flatten() {
        let (ty, v) = parse_binding(&mut env, raw)?;
        set_binding(&mut scope, &mut values, name, ty, v);
    }
    let compile_opts = CompileOptions {
        scope: scope.clone(),
        loop_vars: Vec::new(),
        expected: match case.get("result").and_then(Toml::as_str) {
            Some(r) => Some(result_type(r)?),
            None => None,
        },
        check: CheckOptions {
            allow_key: kind == "order",
            verb_program: kind == "effects",
            outcome_payload: payload.is_some_and(|p| !p.is_empty()),
        },
    };
    let host = world.host(&env);
    let evaluations = case
        .get("evaluations")
        .and_then(Toml::as_integer)
        .unwrap_or(1)
        .max(1);
    let mut diags = Diagnostics::default();
    let per_step = cfg!(feature = "mutant-effect") && opts.mutant == Mutant::LedgerPerStep;
    let mut step_ledger = DrawLedger::new();
    let mut texts = Vec::new();
    for _ in 0..evaluations {
        let ledger = per_step.then_some(&mut step_ledger);
        match evaluate(
            golden,
            &env,
            &host,
            case,
            kind,
            &system,
            &compile_opts,
            &values,
            &mut diags,
            opts,
            ledger,
        ) {
            Ok(text) => texts.push(text),
            Err(Fault::Rejected(code)) => {
                return Ok(CaseOutcome {
                    text: format!("error:{code}"),
                    diags: None,
                });
            }
            Err(Fault::Invariant(code)) => {
                return Ok(CaseOutcome {
                    text: format!("invariant:{code}"),
                    diags: None,
                });
            }
            Err(Fault::Crash(why)) => return Err(format!("{id}: crashed: {why}")),
        }
    }
    let text = if texts.windows(2).all(|w| w[0] == w[1]) {
        texts.swap_remove(0)
    } else {
        "error:NOT_IDEMPOTENT".into()
    };
    Ok(CaseOutcome {
        text,
        diags: Some(diags),
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "the case runner's one dispatch point; every argument is a distinct input"
)]
fn evaluate(
    golden: &Golden,
    env: &Env,
    host: &FixtureHost<'_>,
    case: &toml::Table,
    kind: &str,
    system: &str,
    opts: &CompileOptions,
    values: &[Value],
    diags: &mut Diagnostics,
    eval: EvalOptions,
    step: Option<&mut DrawLedger>,
) -> Result<String, Fault> {
    let text = s(case, "text").map_err(Fault::Crash)?;
    let rejected = |r: rh_registry::effect::Rejection| Fault::Rejected(r.code.as_str().to_owned());
    let mut fresh = DrawLedger::new();
    let has_step = step.is_some();
    let ledger = match step {
        Some(l) => l,
        None => &mut fresh,
    };
    match kind {
        "expr" | "order" => {
            let program = compile_with(Form::Expr, text, env, system, opts).map_err(rejected)?;
            let Output::Value(v) = program.eval_in(env, host, values, diags, ledger, eval)? else {
                return Err(Fault::Crash("an expression yields a value".into()));
            };
            Ok(if kind == "order" {
                format!("key:{}", render_key(env, &v))
            } else {
                render_scalar(env, &v)
            })
        }
        "candidates" => {
            let program =
                compile_with(Form::Candidates, text, env, system, opts).map_err(rejected)?;
            let Output::Relation(ids) = program.eval_in(env, host, values, diags, ledger, eval)?
            else {
                return Err(Fault::Crash("candidates yields a relation".into()));
            };
            Ok(format!("relation:[{}]", join(&ids)))
        }
        "effects" => {
            let program = compile_with(Form::Effects, text, env, system, opts).map_err(rejected)?;
            let Some(events) = case
                .get("events")
                .and_then(Toml::as_array)
                .filter(|e| !e.is_empty())
            else {
                let Output::Effects(actions) =
                    program.eval_in(env, host, values, diags, ledger, eval)?
                else {
                    return Err(Fault::Crash("effects yields actions".into()));
                };
                return Ok(render_actions(env, &actions));
            };
            // r4-2: several events of one verb at one instant share the system's step ledger.
            let mut step = VerbStep::new();
            let mut bound = values.to_vec();
            let mut actions = Vec::new();
            let mut env_for_kinds = env.clone();
            for event in events {
                for (name, raw) in event.as_table().into_iter().flatten() {
                    let (_, v) = parse_binding(&mut env_for_kinds, raw).map_err(Fault::Crash)?;
                    if let Some(i) = opts.scope.iter().position(|(n, _)| n == name) {
                        bound[i] = v;
                    }
                }
                actions.extend(step.eval(&program, env, host, &bound, eval)?);
            }
            add(diags, &step.diags);
            Ok(render_actions(env, &actions))
        }
        "succession" => {
            let law = s(case, "law").map_err(Fault::Crash)?;
            let texts = golden
                .laws
                .get(law)
                .ok_or_else(|| Fault::Crash(format!("unknown law {law}")))?;
            let law_text = LawText {
                candidates: &texts[0],
                eligible: &texts[1],
                order: &texts[2],
            };
            let compiled = CompiledLaw::compile_in(env, system, law_text, &opts.scope)
                .map_err(|(_, r)| rejected(r))?;
            let line =
                compiled.line_in(env, host, values, diags, eval, has_step.then_some(ledger))?;
            Ok(format!("line:[{}]", join(&line)))
        }
        other => Err(Fault::Crash(format!("unknown kind {other}"))),
    }
}

fn add(into: &mut Diagnostics, d: &Diagnostics) {
    into.missing_reads += d.missing_reads;
    into.neutral_reads += d.neutral_reads;
    into.truncated_relations += d.truncated_relations;
    into.dropped_actions += d.dropped_actions;
}

fn join(ids: &[u64]) -> String {
    ids.iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Checks one case against its expected text and pinned diagnostics; `Err` carries the report
/// line (`FAIL <id>: …`).
///
/// # Errors
/// A mismatch or a crash, as a `FAIL` line.
pub fn check_case(golden: &Golden, case: &toml::Table, opts: EvalOptions) -> Result<(), String> {
    let id = s(case, "id")?;
    let outcome = match golden.run_case(case, opts) {
        Ok(o) => o,
        Err(why) => return Err(format!("FAIL {id}: {why}")),
    };
    let expected = s(case, "expect")?;
    let mut ok = outcome.text == expected;
    for name in DIAGNOSTICS {
        if let Some(want) = case.get(name).and_then(Toml::as_integer) {
            ok &= i128::from(want) == outcome.diagnostic(name);
        }
    }
    if ok {
        Ok(())
    } else {
        Err(format!(
            "FAIL {id}: expected {expected} got {} diagnostics {:?}",
            outcome.text, outcome.diags
        ))
    }
}
