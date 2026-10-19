//! SPR-017 T-07: the host seam (`rh_registry::effect::seam`) on an `rh-core` stub world.
//!
//! The golden file's `dynasty` fixture is loaded into test-only owners of M2's, M7's, T1's, S6's
//! and S7's columns. A stub M7 evaluates the seven laws in its step through `SystemView` and gets
//! the golden lines; a planted `AccessSpec` without `life_course.sex` fails with
//! `SimError::UndeclaredRead`; `sortition` draws from the step's keyed stream; a stub T1 submits
//! `FLED`'s `Resign` actions with the verb's draft as their cause; and the host's indexes agree
//! with the scan defaults on random worlds. It lives here, beside the golden test, because a
//! dev-dependency of `rh-registry` on `rh-manifest` would form a cycle.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers fail loudly"
)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, CommandApply, CommandSend, Lag, Plane};
use rh_core::engine::Engine;
use rh_core::ownership::{
    Declaration, EmitDecl, Source, SpecApply, SpecEntry, SpecTable, single_writer,
};
use rh_core::register::register;
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, SimError, SpecPurpose, StepCtx,
    SystemDescriptor, SystemModel,
};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{EventDraft, RoleId};
use rh_ids::{BranchSalt, EntityRef, PersonId};
use rh_manifest::effect_cases::{Golden, splitmix64};
use rh_registry::effect::env::Env;
use rh_registry::effect::host::{Draws, Indexes, ReadOutcome, Reads};
use rh_registry::effect::seam::{StepDraws, SystemViewHost, to_command};
use rh_registry::effect::succession::{CompiledLaw, LawText};
use rh_registry::effect::{
    ColumnId, Diagnostics, EvalOptions, KindId, Output, ProgramKind, Value, compile,
};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const LAWS: [&str; 7] = [
    "primogeniture",
    "ultimogeniture",
    "seniority",
    "partition",
    "elective",
    "tanistry",
    "sortition",
];
/// The fixture's dead holder (Halvard) and his sovereign title.
const HOLDER: u64 = 7;
const TITLE: u64 = 100;

fn golden() -> Golden {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/registry/effect_language_golden_v0.toml"
    );
    Golden::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

/// M7's `[[rng]]` purpose the laws draw under.
#[derive(Clone, Copy, Debug)]
struct Sortition;

impl SpecPurpose for Sortition {
    const ALL: &'static [Self] = &[Self];
    fn name(&self) -> &'static str {
        "sortition"
    }
}

type StepFn = dyn Fn(PulseBinding, &mut SystemView<'_>, &mut StepCtx<'_>) -> Result<(), SimError>
    + Send
    + Sync;

/// A test system: its declarations as data and its step as a closure.
struct TestSystem {
    id: &'static str,
    access: AccessSpec,
    bindings: Vec<PulseBinding>,
    purposes: Vec<&'static str>,
    step: Box<StepFn>,
}

impl TestSystem {
    fn owner(id: &'static str, writes: Vec<&'static str>) -> Self {
        Self {
            id,
            access: AccessSpec {
                writes: writes
                    .into_iter()
                    .map(|column| ColumnWrite {
                        column,
                        plane: Plane::A,
                    })
                    .collect(),
                ..AccessSpec::default()
            },
            bindings: Vec::new(),
            purposes: Vec::new(),
            step: Box::new(|_, _, _| Ok(())),
        }
    }
}

impl SystemModel for TestSystem {
    type Index = ();

    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: self.id,
            fidelity: "F1",
            spec_version: "0.0.0",
        }
    }
    fn access(&self) -> AccessSpec {
        self.access.clone()
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        self.bindings.clone()
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        self.purposes.clone()
    }
    fn params_schema(&self) -> ParamSchema {
        ParamSchema::default()
    }
    fn step(
        &self,
        binding: PulseBinding,
        world: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        (self.step)(binding, world, ctx)
    }
    fn invariants(&self, _: &WorldTables) -> Vec<InvariantResult> {
        Vec::new()
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

/// The spec table the test systems agree with (rh-core's `agreeing_specs`).
fn spec_table(systems: &[TestSystem]) -> SpecTable {
    let specs: Vec<(Declaration, SpecEntry)> = systems
        .iter()
        .map(|s| {
            let declaration = s.access.declaration(s.id);
            let entry = SpecEntry {
                owns: declaration.writes.clone(),
                reads: s.access.read_columns(),
                rng: s.purposes.iter().map(|p| (*p).to_owned()).collect(),
                schedule: s
                    .bindings
                    .iter()
                    .map(|b| (b.pulse.name().to_owned(), b.phase.name().to_owned()))
                    .collect(),
                lags: s
                    .access
                    .reads
                    .iter()
                    .map(|r| (r.column.to_owned(), r.lag.name().to_owned()))
                    .collect(),
                commands: s
                    .access
                    .sends
                    .iter()
                    .map(|c| (c.target.to_owned(), c.command.to_owned()))
                    .collect(),
                emits: s.access.emits.iter().map(|k| (*k).to_owned()).collect(),
                envelope: s
                    .access
                    .emits
                    .iter()
                    .map(|&k| {
                        (
                            k.to_owned(),
                            EmitDecl {
                                roles: ["actor", "target", "witness", "instrument"]
                                    .map(str::to_owned)
                                    .to_vec(),
                                ..EmitDecl::default()
                            },
                        )
                    })
                    .collect(),
                applies: s
                    .access
                    .applies
                    .iter()
                    .map(|a| SpecApply {
                        command: a.command.to_owned(),
                        drained_by: (
                            a.drained_by.pulse.name().to_owned(),
                            a.drained_by.phase.name().to_owned(),
                        ),
                        key: "any".to_owned(),
                        key_field: "person".to_owned(),
                        ..SpecApply::default()
                    })
                    .collect(),
                from_manifest: Vec::new(),
                ..Default::default()
            };
            (declaration, entry)
        })
        .collect();
    let declarations: Vec<Declaration> = specs.iter().map(|(d, _)| d.clone()).collect();
    let (owners, _) = single_writer(&declarations, Source::Spec);
    SpecTable {
        owners,
        specs: specs.into_iter().map(|(d, e)| (d.system, e)).collect(),
        ..Default::default()
    }
}

fn engine(systems: Vec<TestSystem>, tables: WorldTables) -> Engine {
    let specs = spec_table(&systems);
    let registered = register(
        systems
            .into_iter()
            .map(|s| Box::new(s) as Box<dyn SystemModel<Index = ()>>)
            .collect(),
        &specs,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    Engine::new(
        CALENDAR,
        17,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    )
}

/// Runs the engine's first instant.
fn first_instant(mut engine: Engine) -> Result<Engine, SimError> {
    let t = engine.next_instant().unwrap();
    engine.advance_until(t, |_, _| {})?;
    Ok(engine)
}

/// A value as its stub-world cell.
fn cell(v: &Value) -> i64 {
    match v {
        Value::Bool(b) => i64::from(*b),
        Value::Int(i) => *i,
        Value::Q8(q) => i64::from(q.to_bits()),
        Value::Q16(q) => i64::from(q.to_bits()),
        Value::Enum(_, o) => i64::from(*o),
        Value::Ent(_, id) => id.cast_signed(),
        other => panic!("no cell for {other:?}"),
    }
}

/// The owners of the fixture's columns (those of registered systems), and their id columns.
const OWNERS: [&str; 5] = [
    "M2.demography",
    "M7.dynasty",
    "S6.artifacts",
    "S7.vows",
    "T1.minds",
];
/// The id column each keyed kind is read by, and its owner. The tenure log has none: its row index
/// is its id. The life-course log is read by person.
const ID_COLUMNS: [(&str, &str, &str); 7] = [
    ("person", "person.id", "M2.demography"),
    ("life_course", "life_course.person_id", "M2.demography"),
    ("title", "title.id", "M7.dynasty"),
    ("office", "office.id", "M7.dynasty"),
    ("claim", "claim.id", "M7.dynasty"),
    ("vow", "vow.id", "S7.vows"),
    ("artifact", "artifact.id", "S6.artifacts"),
];

/// The `dynasty` fixture as stub-world tables, and each owner's columns. The ledger's rules hold:
/// `person.*` aligned with an ascending `person.id`, and the life-course exit fields, with the
/// dead holder's entry closed, so the compaction at `Bookkeeping` leaves the open entries equal
/// to the ledger. `extra_office` adds an office row `(id, title, holder)`.
fn dynasty_tables(
    golden: &Golden,
    extra_office: Option<(u64, u64, u64)>,
) -> (WorldTables, BTreeMap<&'static str, Vec<&'static str>>) {
    let env = &golden.env;
    let world = golden
        .world(env, "dynasty", rh_registry::effect::Mutant::None)
        .unwrap();
    let mut tables = WorldTables::new();
    let mut owned: BTreeMap<&'static str, Vec<&'static str>> = BTreeMap::new();
    let mut put = |tables: &mut WorldTables, column: &str, owner: &str, rows: Vec<i64>| {
        let column = leak(column);
        tables.insert(column, rows);
        owned.entry(leak(owner)).or_default().push(column);
    };
    let alive_col = env.column_id("person.alive").unwrap();
    for kind in world.kinds() {
        let kind_name = env.kind_name(kind).to_owned();
        let mut rows: Vec<(u64, BTreeMap<ColumnId, Value>)> = world
            .rows(kind)
            .unwrap()
            .iter()
            .map(|(id, r)| (*id, r.clone()))
            .collect();
        if kind_name == "office"
            && let Some((id, title, holder)) = extra_office
        {
            let mut r = BTreeMap::new();
            r.insert(
                env.column_id("office.title").unwrap(),
                Value::Ent(env.kind("title").unwrap(), title),
            );
            r.insert(
                env.column_id("office.holder").unwrap(),
                Value::Ent(env.kind("person").unwrap(), holder),
            );
            rows.push((id, r));
        }
        for def in env.columns() {
            if def.entity != kind || !OWNERS.contains(&def.owner.as_str()) {
                continue;
            }
            let id = env.column_id(&def.name).unwrap();
            let col: Vec<i64> = rows
                .iter()
                .map(|(_, r)| r.get(&id).map_or(0, cell))
                .collect();
            put(&mut tables, &def.name, &def.owner, col);
        }
        if let Some((_, id_col, owner)) = ID_COLUMNS
            .iter()
            .find(|(k, _, _)| *k == kind_name)
            .filter(|(k, _, _)| *k != "life_course")
        {
            let ids = rows.iter().map(|(id, _)| id.cast_signed()).collect();
            put(&mut tables, id_col, owner, ids);
        }
        if kind_name == "life_course" {
            // Open for everyone in the ledger alive; closed (a death) for the compacted and the
            // dead holder of this instant.
            let living: Vec<u64> = world
                .rows(env.kind("person").unwrap())
                .unwrap()
                .iter()
                .filter(|(_, r)| r.get(&alive_col) == Some(&Value::Bool(true)))
                .map(|(id, _)| *id)
                .collect();
            let open: Vec<bool> = rows.iter().map(|(id, _)| living.contains(id)).collect();
            let kinds = open.iter().map(|&o| i64::from(!o)).collect();
            let ticks = open.iter().map(|&o| if o { i64::MAX } else { 0 }).collect();
            put(&mut tables, "life_course.exit_kind", "M2.demography", kinds);
            put(&mut tables, "life_course.exit_tick", "M2.demography", ticks);
        }
    }
    (tables, owned)
}

/// The owners of every fixture column, stepping nothing.
fn owners(owned: &BTreeMap<&'static str, Vec<&'static str>>, except: &str) -> Vec<TestSystem> {
    owned
        .iter()
        .filter(|(id, _)| **id != except)
        .map(|(id, cols)| TestSystem::owner(id, cols.clone()))
        .collect()
}

/// `system`'s golden reads, with each column's owner, plus the id columns of the kinds it reads
/// (the seam finds a row through its id column), less `drop`.
fn reads_of(env: &Env, system: &str, extra: &[&str], drop: &str, lag: Lag) -> Vec<ColumnRead> {
    let decl = &env.systems[system];
    let owner_of = |c: &str| -> Option<&'static str> {
        if let Some((_, _, o)) = ID_COLUMNS.iter().find(|(_, col, _)| *col == c) {
            return Some(o);
        }
        let def = env.column(env.column_id(c)?)?;
        OWNERS.iter().copied().find(|o| *o == def.owner)
    };
    decl.reads
        .keys()
        .map(String::as_str)
        .chain(extra.iter().copied())
        .filter(|c| *c != drop)
        .filter_map(|c| {
            Some(ColumnRead {
                column: leak(c),
                owner: owner_of(c).filter(|o| *o != system)?,
                lag,
            })
        })
        .collect()
}

/// What the stub M7 records in its step.
type Lines = Arc<Mutex<Vec<(String, Result<Vec<u64>, String>)>>>;

/// The stub M7: owns M7's columns, reads M7's declarations, evaluates every law for `(HOLDER,
/// title)` for each title, twice, through the seam; records each line.
fn law_engine(drop_read: &str, titles: Vec<u64>) -> (Engine, Lines) {
    let golden = golden();
    let (tables, owned) = dynasty_tables(&golden, None);
    let env = Arc::new(golden.env.clone());
    let laws: Vec<(String, CompiledLaw)> = LAWS
        .iter()
        .map(|&law| {
            let [c, e, o] = &golden.laws[law];
            let text = LawText {
                candidates: c,
                eligible: e,
                order: o,
            };
            (
                law.to_owned(),
                CompiledLaw::compile(&env, "M7.dynasty", text)
                    .unwrap_or_else(|(p, r)| panic!("{law}.{}: {r}", p.as_str())),
            )
        })
        .collect();
    let world = golden
        .world(&env, "dynasty", rh_registry::effect::Mutant::None)
        .unwrap();
    let params = world.params_of("M7.dynasty");
    let days = world.days_per_year();
    let lines: Lines = Arc::new(Mutex::new(Vec::new()));
    let mut systems = owners(&owned, "M7.dynasty");
    let reads = reads_of(
        &env,
        "M7.dynasty",
        &["person.id", "vow.id"],
        drop_read,
        Lag::SameInstant,
    );
    let mut m7 = TestSystem::owner("M7.dynasty", owned["M7.dynasty"].clone());
    m7.access.reads = reads;
    m7.purposes = vec!["sortition"];
    m7.bindings = vec![PulseBinding {
        pulse: Pulse::Year,
        phase: Phase::Polity,
    }];
    let out = Arc::clone(&lines);
    m7.step = Box::new(move |_, world, ctx| {
        let draws = StepDraws::<Sortition>::new(ctx, &env);
        let host = host(&env, world, draws, params.clone(), days);
        let person = env.kind("person").unwrap();
        let title_kind = env.kind("title").unwrap();
        let now = Value::Int(ctx.now().minutes());
        for &title in &titles {
            for (law, compiled) in &laws {
                for _ in 0..2 {
                    let bindings = [
                        Value::Ent(person, HOLDER),
                        Value::Ent(title_kind, title),
                        now.clone(),
                    ];
                    let mut diags = Diagnostics::default();
                    let line =
                        compiled.line(&env, &host, &bindings, &mut diags, EvalOptions::default());
                    if let Some(e) = host.take_error() {
                        return Err(e);
                    }
                    out.lock()
                        .unwrap()
                        .push((format!("{law}/{title}"), line.map_err(|e| format!("{e:?}"))));
                }
            }
        }
        Ok(())
    });
    systems.push(m7);
    (engine(systems, tables), lines)
}

/// The seam's host over a step, keyed as the fixture is.
fn host<'h, 'v, D: Draws>(
    env: &'h Env,
    world: &'h SystemView<'v>,
    draws: D,
    params: BTreeMap<String, Value>,
    days: i64,
) -> SystemViewHost<'h, 'v, D> {
    ID_COLUMNS.iter().fold(
        SystemViewHost::new(env, world, draws, params, days),
        |h, (kind, column, _)| h.with_key(kind, column),
    )
}

fn render(line: &[u64]) -> String {
    let ids: Vec<String> = line.iter().map(u64::to_string).collect();
    format!("line:[{}]", ids.join(", "))
}

/// V-WC-10: the six laws that draw nothing give the golden lines through `SystemView`, each
/// evaluation of a step the same.
#[test]
fn the_laws_through_system_view_give_the_golden_lines() {
    let golden = golden();
    let (engine, lines) = law_engine("", vec![TITLE]);
    first_instant(engine).unwrap();
    let lines = lines.lock().unwrap();
    assert_eq!(lines.len(), LAWS.len() * 2);
    for law in LAWS.iter().filter(|l| **l != "sortition") {
        let case = golden
            .cases
            .iter()
            .find(|c| c["id"].as_str() == Some(&format!("succession_{law}")))
            .unwrap();
        let got: Vec<_> = lines
            .iter()
            .filter(|(k, _)| k == &format!("{law}/{TITLE}"))
            .map(|(_, l)| l.clone().map(|l| render(&l)))
            .collect();
        assert_eq!(got.len(), 2);
        for g in got {
            assert_eq!(g.as_deref(), Ok(case["expect"].as_str().unwrap()), "{law}");
        }
    }
}

/// V-WC-10: a read the system's `AccessSpec` does not declare is the view's error, not a zero.
#[test]
fn an_undeclared_read_is_the_views_error() {
    let (engine, _) = law_engine("life_course.sex", vec![TITLE]);
    match first_instant(engine) {
        Err(SimError::UndeclaredRead { system, column }) => {
            assert_eq!((system, column.as_str()), ("M7.dynasty", "life_course.sex"));
        }
        Err(e) => panic!("another error: {e}"),
        Ok(_) => panic!("the undeclared read of life_course.sex was not refused"),
    }
}

/// §4.6, N6, minor 4: `sortition` draws from the step's keyed stream `(candidate, k = the title,
/// t)`. Two evaluations in one step give one line; each title's line is its candidates in
/// ascending draw order under its own `k`; the two titles' draws differ.
#[test]
fn sortition_draws_from_the_steps_stream() {
    let golden = golden();
    let (mut eng, lines) = law_engine("", vec![TITLE, 101]);
    // The expected order is computed from the same stream in a probe step of its own system,
    // under the same key: `H(seed, salt, system_id, epoch, purpose, entity, t, k)` names M7.
    eng = first_instant(eng).unwrap();
    drop(eng);
    let lines = lines.lock().unwrap();
    let of = |title: u64| -> Vec<Vec<u64>> {
        lines
            .iter()
            .filter(|(k, _)| k == &format!("sortition/{title}"))
            .map(|(_, l)| l.clone().unwrap())
            .collect()
    };
    let (a, b) = (of(TITLE), of(101));
    assert_eq!(a.len(), 2);
    assert_eq!(a[0], a[1], "two evaluations in one step");
    assert_eq!(b[0], b[1], "two evaluations in one step");
    // The eligible set is the golden case's; only the order comes from the stream.
    let case = golden
        .cases
        .iter()
        .find(|c| c["id"].as_str() == Some("succession_sortition"))
        .unwrap();
    let mut want: Vec<String> = case["expect"]
        .as_str()
        .unwrap()
        .trim_start_matches("line:[")
        .trim_end_matches(']')
        .split(", ")
        .map(str::to_owned)
        .collect();
    want.sort();
    let mut have: Vec<String> = a[0].iter().map(u64::to_string).collect();
    have.sort();
    assert_eq!(have, want, "sortition's candidates");
    let (keys_a, keys_b, crossed) = sortition_keys(&a[0], &b[0]);
    assert!(
        keys_a.windows(2).all(|w| w[0] <= w[1]),
        "title {TITLE} in draw order: {keys_a:?}"
    );
    assert!(
        keys_b.windows(2).all(|w| w[0] <= w[1]),
        "title 101 in draw order: {keys_b:?}"
    );
    // Independent per title: each of title 100's candidates draws otherwise under k = 101.
    assert!(!keys_b.is_empty());
    for ((ka, x), kb) in keys_a.iter().zip(&crossed) {
        assert_ne!(ka, kb, "candidate {x} drew the same under both titles");
    }
}

/// Draws with their candidates, of two lines, and the first line's under the second title.
type Keys = (Vec<(u64, u64)>, Vec<(u64, u64)>, Vec<u64>);

/// The draws M7's stream gives each candidate of the two lines (`k` = the title), in line order,
/// and the first line's candidates drawn under the second title's `k`, from a second run of the
/// same world whose M7 step only draws.
fn sortition_keys(a: &[u64], b: &[u64]) -> Keys {
    let golden = golden();
    let (tables, owned) = dynasty_tables(&golden, None);
    let mut systems = owners(&owned, "M7.dynasty");
    let mut m7 = TestSystem::owner("M7.dynasty", owned["M7.dynasty"].clone());
    m7.purposes = vec!["sortition"];
    m7.bindings = vec![PulseBinding {
        pulse: Pulse::Year,
        phase: Phase::Polity,
    }];
    let out: Arc<Mutex<Keys>> = Arc::default();
    let (sink, a, b) = (Arc::clone(&out), a.to_vec(), b.to_vec());
    m7.step = Box::new(move |_, _, ctx| {
        let key = |x: u64, title: u64| -> Result<(u64, u64), SimError> {
            Ok((ctx.rng(&Sortition, PersonId::from_raw(x), title)?, x))
        };
        let ka = a
            .iter()
            .map(|&x| key(x, TITLE))
            .collect::<Result<Vec<_>, _>>()?;
        let kb = b
            .iter()
            .map(|&x| key(x, 101))
            .collect::<Result<Vec<_>, _>>()?;
        let crossed = a
            .iter()
            .map(|&x| key(x, 101).map(|k| k.0))
            .collect::<Result<Vec<_>, _>>()?;
        *sink.lock().unwrap() = (ka, kb, crossed);
        Ok(())
    });
    systems.push(m7);
    first_instant(engine(systems, tables)).unwrap();
    out.lock().unwrap().clone()
}

/// §6, `05.1` §2 rule 2: a stub T1 evaluates `FLED`'s effects for Ragnar (16), who holds two
/// offices here, emits the event and submits each `Resign` through `StepCtx::submit` with the
/// event's draft as its cause; the stub M7 takes them in action order, each caused by FLED.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "the two stub systems and their world in one place"
)]
fn verb_actions_are_queued_commands_caused_by_the_verb() {
    let golden = golden();
    let (tables, owned) = dynasty_tables(&golden, Some((299, TITLE, 16)));
    let env = Arc::new(golden.env.clone());
    let text = golden
        .cases
        .iter()
        .find(|c| c["id"].as_str() == Some("effects_fled_resigns_each_office"))
        .unwrap()["text"]
        .as_str()
        .unwrap()
        .to_owned();
    let program = compile(ProgramKind::Effects, &text, &env, "T1.minds").unwrap();
    let mut systems: Vec<TestSystem> = owned
        .iter()
        .filter(|(id, _)| !["M7.dynasty", "T1.minds"].contains(id))
        .map(|(id, cols)| TestSystem::owner(id, cols.clone()))
        .collect();
    let mut t1 = TestSystem::owner("T1.minds", owned["T1.minds"].clone());
    t1.access.reads = reads_of(
        &env,
        "T1.minds",
        &["office.id", "person.id"],
        "",
        Lag::Lagged,
    );
    t1.access.sends = vec![CommandSend {
        command: "Resign",
        target: "M7.dynasty",
    }];
    t1.access.emits = vec!["FLED"];
    t1.bindings = vec![PulseBinding {
        pulse: Pulse::Year,
        phase: Phase::Polity,
    }];
    let t1_env = Arc::clone(&env);
    t1.step = Box::new(move |_, world, ctx| {
        let env = &t1_env;
        let actions = {
            let host = host(env, world, NoDraws, BTreeMap::new(), 360);
            let person = env.kind("person").unwrap();
            let bindings = [
                Value::Ent(person, 16),
                Value::Ent(person, 0),
                Value::Ent(env.kind("artifact").unwrap(), 0),
                Value::Event(rh_registry::effect::EventArg::Own),
                Value::Int(ctx.now().minutes()),
            ];
            let out = program
                .eval(
                    env,
                    &host,
                    &bindings,
                    &mut Diagnostics::default(),
                    EvalOptions::default(),
                )
                .unwrap();
            if let Some(e) = host.take_error() {
                return Err(e);
            }
            let Output::Effects(actions) = out else {
                panic!("not effects")
            };
            actions
        };
        let draft = ctx.emit(
            EventDraft::new("FLED").role(RoleId::ACTOR, EntityRef::person(PersonId::from_raw(16))),
        )?;
        let access = t1_access();
        for a in &actions {
            ctx.submit(to_command(env, &access, a, Some(draft)).unwrap())?;
        }
        Ok(())
    });
    let taken: Arc<Mutex<Vec<Taken>>> = Arc::default();
    let mut m7 = TestSystem::owner("M7.dynasty", owned["M7.dynasty"].clone());
    m7.access.applies = vec![CommandApply::new("Resign", Pulse::Year, Phase::Agenda)];
    m7.bindings = vec![PulseBinding {
        pulse: Pulse::Year,
        phase: Phase::Agenda,
    }];
    let sink = Arc::clone(&taken);
    m7.step = Box::new(move |_, _, ctx| {
        for q in ctx.take_commands() {
            sink.lock().unwrap().push((
                q.command.kind.to_owned(),
                q.command.person.to_raw(),
                q.command.args.iter().map(|a| a.int().unwrap()).collect(),
                q.cause.is_some(),
            ));
        }
        Ok(())
    });
    systems.push(t1);
    systems.push(m7);
    first_instant(engine(systems, tables)).unwrap();
    assert_eq!(
        *taken.lock().unwrap(),
        vec![
            ("Resign".to_owned(), 16, vec![200], true),
            ("Resign".to_owned(), 16, vec![299], true),
        ]
    );
}

/// A command as the owner took it: kind, person, arguments, and whether it has a cause.
type Taken = (String, u64, Vec<i64>, bool);

/// T1's declared sends, as its `AccessSpec` holds them (`to_command` takes the `'static` name).
fn t1_access() -> AccessSpec {
    AccessSpec {
        sends: vec![CommandSend {
            command: "Resign",
            target: "M7.dynasty",
        }],
        ..AccessSpec::default()
    }
}

/// A host draws nothing.
struct NoDraws;

impl Draws for NoDraws {
    fn draw(&self, _: &str, _: KindId, _: u64, _: i64) -> Option<u64> {
        None
    }
}

/// The scan defaults: `Reads` through the seam's host, `rows_where` by the trait's default scan,
/// `children` by a scan of the life-course log.
struct Scan<'a, H>(&'a H, &'a Env);

impl<H: Reads> Reads for Scan<'_, H> {
    fn read(&self, column: ColumnId, id: u64) -> ReadOutcome {
        self.0.read(column, id)
    }
    fn exists(&self, kind: KindId, id: u64) -> bool {
        self.0.exists(kind, id)
    }
    fn ids(&self, kind: KindId) -> Vec<u64> {
        self.0.ids(kind)
    }
}

impl<H: Reads> Indexes for Scan<'_, H> {
    fn children(&self, person: u64) -> Vec<u64> {
        let lc = self.1.kind("life_course").unwrap();
        let (m, f) = (
            self.1.column_id("life_course.mother").unwrap(),
            self.1.column_id("life_course.father").unwrap(),
        );
        self.ids(lc)
            .into_iter()
            .filter(|&c| {
                [m, f].iter().any(|&col| {
                    matches!(self.read(col, c), ReadOutcome::Row(Value::Ent(_, p)) if p == person && p != 0)
                })
            })
            .collect()
    }
}

/// A random world of `n` persons: a life-course log with shuffled, non-contiguous person ids and
/// unknown parents, titles with unsorted ids, offices, claims and vows on random persons, and a
/// tenure log.
fn random_tables(n: usize, seed: u64) -> (WorldTables, Vec<&'static str>) {
    let mut s = seed;
    let mut next = move || {
        s = splitmix64(s);
        s
    };
    let pick = |r: u64, k: usize| usize::try_from(r % u64::try_from(k).unwrap()).unwrap();
    let mut ids: Vec<u64> = (0..n)
        .map(|i| 1000 + 7 * u64::try_from(i).unwrap())
        .collect();
    for i in (1..ids.len()).rev() {
        let j = pick(next(), i + 1);
        ids.swap(i, j);
    }
    let parent = |next: &mut dyn FnMut() -> u64, before: &[u64]| -> u64 {
        if before.is_empty() || next().is_multiple_of(5) {
            0
        } else {
            before[pick(next(), before.len())]
        }
    };
    let (mut mothers, mut fathers) = (Vec::new(), Vec::new());
    for i in 0..n {
        let before = &ids[..i];
        mothers.push(parent(&mut next, before));
        fathers.push(parent(&mut next, before));
    }
    let titles: Vec<u64> = (0..n / 4 + 2).map(|_| next() % 100_000 + 1).collect();
    let mut t = WorldTables::new();
    let mut cols = Vec::new();
    let mut put = |t: &mut WorldTables, c: &'static str, v: Vec<i64>| {
        t.insert(c, v);
        cols.push(c);
    };
    let as_i64 = |v: &[u64]| v.iter().map(|x| x.cast_signed()).collect::<Vec<_>>();
    put(&mut t, "life_course.person_id", as_i64(&ids));
    put(&mut t, "life_course.mother", as_i64(&mothers));
    put(&mut t, "life_course.father", as_i64(&fathers));
    let mut distinct = std::collections::BTreeSet::new();
    let title_ids: Vec<u64> = titles.into_iter().filter(|t| distinct.insert(*t)).collect();
    let some_person = |r: u64| ids[pick(r, ids.len())];
    let liege: Vec<u64> = title_ids
        .iter()
        .map(|_| title_ids[pick(next(), title_ids.len())])
        .collect();
    let holders: Vec<u64> = title_ids.iter().map(|_| some_person(next())).collect();
    put(&mut t, "title.id", as_i64(&title_ids));
    put(&mut t, "title.liege", as_i64(&liege));
    put(&mut t, "title.holder", as_i64(&holders));
    for (kind, a, b) in [
        ("office", "office.title", "office.holder"),
        ("claim", "claim.title", "claim.claimant"),
        ("tenure", "tenure.title", "tenure.holder"),
    ] {
        let rows = n / 2 + 3;
        let ta: Vec<u64> = (0..rows)
            .map(|_| title_ids[pick(next(), title_ids.len())])
            .collect();
        let pb: Vec<u64> = (0..rows).map(|_| some_person(next())).collect();
        put(&mut t, a, as_i64(&ta));
        put(&mut t, b, as_i64(&pb));
        if kind != "tenure" {
            let id_col = leak(&format!("{kind}.id"));
            let rid: Vec<u64> = (0..rows)
                .map(|i| 5_000_000 - 3 * u64::try_from(i).unwrap())
                .collect();
            put(&mut t, id_col, as_i64(&rid));
        }
    }
    let vowers: Vec<u64> = (0..=(n / 3)).map(|_| some_person(next())).collect();
    put(&mut t, "vow.vower", as_i64(&vowers));
    put(
        &mut t,
        "vow.id",
        (1..=vowers.len())
            .map(|i| i64::try_from(i).unwrap())
            .collect(),
    );
    (t, cols)
}

/// The seam's indexes (`children`, `rows_where`, `tenure_rows`) equal the scan defaults on random
/// worlds of three sizes, eight seeds each.
#[test]
fn indexes_agree_with_the_scan() {
    let env = Arc::new(golden().env);
    for n in [12, 97, 600] {
        for seed in 1..=8u64 {
            let (tables, cols) = random_tables(n, seed * 0x9E37 + u64::try_from(n).unwrap());
            let mut probe = TestSystem::owner("X1.probe", cols);
            probe.bindings = vec![PulseBinding {
                pulse: Pulse::Year,
                phase: Phase::Polity,
            }];
            let checked = Arc::new(Mutex::new(0usize));
            let (env, sink) = (Arc::clone(&env), Arc::clone(&checked));
            probe.step = Box::new(move |_, world, _| {
                let indexed = host(&env, world, NoDraws, BTreeMap::new(), 360);
                let scan = Scan(&indexed, &env);
                let mut count = 0;
                let lc = env.kind("life_course").unwrap();
                for p in indexed.ids(lc).into_iter().chain([0, 1, u64::MAX]) {
                    assert_eq!(indexed.children(p), scan.children(p), "children of {p}");
                    count += usize::from(!indexed.children(p).is_empty());
                }
                for (kind, column) in [
                    ("title", "title.liege"),
                    ("title", "title.holder"),
                    ("office", "office.title"),
                    ("office", "office.holder"),
                    ("claim", "claim.title"),
                    ("claim", "claim.claimant"),
                    ("tenure", "tenure.title"),
                    ("tenure", "tenure.holder"),
                    ("vow", "vow.vower"),
                ] {
                    let k = env.kind(kind).unwrap();
                    let c = env.column_id(column).unwrap();
                    let targets: Vec<u64> = indexed
                        .ids(k)
                        .into_iter()
                        .filter_map(|id| match indexed.read(c, id) {
                            ReadOutcome::Row(Value::Ent(_, t)) => Some(t),
                            _ => None,
                        })
                        .chain([0, 2, u64::MAX])
                        .collect();
                    for target in targets {
                        assert_eq!(
                            indexed.rows_where(k, c, target),
                            scan.rows_where(k, c, target),
                            "{column} = {target}"
                        );
                        if kind == "tenure" && column == "tenure.title" {
                            assert_eq!(
                                indexed.tenure_rows(k, c, target),
                                scan.tenure_rows(k, c, target)
                            );
                        }
                        count += usize::from(!indexed.rows_where(k, c, target).is_empty());
                    }
                }
                if let Some(e) = indexed.take_error() {
                    return Err(e);
                }
                *sink.lock().unwrap() = count;
                Ok(())
            });
            first_instant(engine(vec![probe], tables)).unwrap();
            assert!(
                *checked.lock().unwrap() > 0,
                "n = {n}, seed {seed}: nothing was compared"
            );
        }
    }
}
