//! Integration and parity tests for M2 F1 partnering (SPR-015 T-04): the eligible-candidate scan
//! and its age bounds (N4), the `husband_pick` choice's CRN stability, `apply_marry` (N7) and the
//! households' marriage and death rules (H3, H4).
//!
//! The pure parts of the scan — [`pairing_candidates`], [`kin2`], [`fertility_band`] and
//! [`pick_husband`] — are tested directly on hand-built columns. The write paths — `apply_marry`
//! and `household_on_death` — run through the engine: a test-only `test.commander` system submits
//! a `Marry` and a `ScheduleDeath` command, which M2's Day/Vital drain applies.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken fixture must fail the test loudly"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, CommandSend, Lag, Plane};
use rh_core::command::{Arg, Command};
use rh_core::engine::Engine;
use rh_core::fixed::Q16_16;
use rh_core::register::register;
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, ResampleCtx, SimError, SpecPurpose,
    StepCtx, SystemDescriptor, SystemModel, TimerSpec,
};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{EventRef, RoleId};
use rh_ids::{BranchSalt, EntityRef, PersonId};
use rh_registry::ownership::{Ownership, SystemSpec};
use rh_sys_demography::Demography;
use rh_sys_demography::demography::{
    MAR_SOURCE_COMMAND, MAR_SOURCE_GENESIS, MARRIAGE, SEX_FEMALE, SEX_MALE, fertility_band, kin2,
    pairing_candidates, pick_husband,
};

mod spec_files;

const D360: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const YEAR: i64 = 360 * DAY;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The commander's spec: a test-only sender of `ScheduleDeath` and `Marry` to M2.
const COMMANDER_SPEC: &str = r#"
[system]
id = "test.commander"
[[owns]]
column = "test.plan"
entity = "test"
type = "i64"
plane = "A"
[[reads]]
column = "person.id"
owner = "M2.demography"
lag = "same_instant"
use = "identity"
[[commands]]
target = "M2.demography"
command = "ScheduleDeath"
timer = "DUE"
purpose = "test_due"
[[commands]]
target = "M2.demography"
command = "Marry"
timer = "DUE"
purpose = "test_due"
[[schedule]]
pulse = "Day"
phase = "Agenda"
[[schedule]]
pulse = "Day"
phase = "Vital"
[[rng]]
purpose = "test_due"
key = ["person", "tick"]
"#;

fn specs() -> rh_core::ownership::SpecTable {
    let mut ids = spec_files::registry_system_ids(&repo().join("specs/manifest/registry_v0.toml"))
        .expect("registry");
    ids.insert("test.commander".to_owned());
    let mut specs = spec_files::load_specs(&repo().join("specs/systems")).expect("specs");
    specs
        .push(SystemSpec::from_toml(Path::new("test.commander.odd.toml"), COMMANDER_SPEC).unwrap());
    Ownership::build(&specs, &ids).spec_table()
}

/// One founder: its id, birth tick and sex, in ascending id order.
fn tables(persons: &[(u64, i64, i64)]) -> WorldTables {
    let n = persons.len();
    let ids: Vec<i64> = persons.iter().map(|&(p, _, _)| p.cast_signed()).collect();
    let births: Vec<i64> = persons.iter().map(|&(_, b, _)| b).collect();
    let sexes: Vec<i64> = persons.iter().map(|&(_, _, s)| s).collect();
    let mut t = WorldTables::new();
    for (column, rows) in [
        ("person.id", ids.clone()),
        ("person.birth_tick", births.clone()),
        ("person.sex", sexes.clone()),
        ("person.alive", vec![1; n]),
        ("person.death_tick", vec![i64::MAX; n]),
        ("person.mother", vec![0; n]),
        ("person.father", vec![0; n]),
        ("person.spouse", vec![0; n]),
        ("person.death_cause", vec![0; n]),
        ("person.household", vec![0; n]),
        ("person.parity", vec![0; n]),
        ("person.last_birth_tick", vec![i64::MIN; n]),
        ("person.conceived_tick", vec![i64::MIN; n]),
        ("person.unborn_father", vec![0; n]),
        ("household.id", Vec::new()),
        ("household.head", Vec::new()),
        ("household.formed_tick", Vec::new()),
        ("household.dissolved_tick", Vec::new()),
        ("marriage.husband", Vec::new()),
        ("marriage.wife", Vec::new()),
        ("marriage.start_tick", Vec::new()),
        ("marriage.end_tick", Vec::new()),
        ("marriage.source", Vec::new()),
        ("life_course.person_id", ids),
        ("life_course.origin", vec![1; n]),
        ("life_course.mother", vec![0; n]),
        ("life_course.father", vec![0; n]),
        ("life_course.sex", sexes),
        ("life_course.birth_tick", births),
        ("life_course.entry_tick", vec![0; n]),
        ("life_course.exit_kind", vec![0; n]),
        ("life_course.exit_tick", vec![i64::MAX; n]),
        ("life_course.death_cause", vec![0; n]),
    ] {
        t.insert(column, rows);
    }
    t
}

/// The `test.commander` purpose.
enum Due {
    TestDue,
}

impl SpecPurpose for Due {
    const ALL: &'static [Self] = &[Self::TestDue];
    fn name(&self) -> &'static str {
        "test_due"
    }
}

/// A test-only commander: at its first Day/Agenda step it schedules a `DUE` timer for `tick`;
/// the timer's Vital fire submits the command. The plan column's first row is the one-shot flag.
struct Commander {
    tick: i64,
    kind: &'static str,
    a: u64,
    b: i64,
}

fn read(column: &'static str) -> ColumnRead {
    ColumnRead {
        column,
        owner: "M2.demography",
        lag: Lag::SameInstant,
    }
}

impl SystemModel for Commander {
    type Index = ();

    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: "test.commander",
            fidelity: "F0",
            spec_version: "0.0.0",
        }
    }

    fn access(&self) -> AccessSpec {
        AccessSpec {
            reads: vec![read("person.id")],
            writes: vec![ColumnWrite {
                column: "test.plan",
                plane: Plane::A,
            }],
            sends: vec![
                CommandSend {
                    command: "ScheduleDeath",
                    target: "M2.demography",
                },
                CommandSend {
                    command: "Marry",
                    target: "M2.demography",
                },
            ],
            ..AccessSpec::default()
        }
    }

    fn schedule(&self) -> Vec<PulseBinding> {
        vec![
            PulseBinding {
                pulse: Pulse::Day,
                phase: Phase::Agenda,
            },
            PulseBinding {
                pulse: Pulse::Day,
                phase: Phase::Vital,
            },
        ]
    }

    fn rng_purposes(&self) -> Vec<&'static str> {
        Due::names()
    }

    fn params_schema(&self) -> ParamSchema {
        ParamSchema::default()
    }

    fn step(
        &self,
        binding: PulseBinding,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        // One shot, scheduled at the first Agenda instant.
        if binding.phase != Phase::Agenda {
            return Ok(());
        }
        let active = view.read("test.plan")?.first().copied().unwrap_or(0);
        if active == 0 {
            return Ok(());
        }
        ctx.schedule(
            &Due::TestDue,
            TimerSpec {
                time: SimTime::from_minutes(self.tick),
                phase: Phase::Vital,
                entity: EntityRef::person(PersonId::from_raw(self.a)),
                kind: 1,
                payload: self.b,
                cause: EventRef::None,
            },
        )?;
        view.write("test.plan")?[0] = 0;
        Ok(())
    }

    fn fire(
        &self,
        _t: &rh_core::agenda::TimerEntry,
        _v: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        // ScheduleDeath is `{ victim, cause, culprit, cause_event }`; Marry is `{ a, b }`.
        let args = match self.kind {
            "Marry" => vec![Arg::Int(self.a.cast_signed()), Arg::Int(self.b)],
            "ScheduleDeath" => vec![
                Arg::Int(self.a.cast_signed()),
                Arg::Int(self.b),
                Arg::Int(0),
                Arg::Event(EventRef::None),
            ],
            other => {
                return Err(SimError::System {
                    system: "test.commander",
                    message: format!("unknown command {other}"),
                });
            }
        };
        c.submit(Command {
            kind: self.kind,
            person: PersonId::from_raw(self.a),
            args,
            cause: EventRef::None,
        })
    }

    fn resample_timer(
        &self,
        _t: &rh_core::agenda::TimerEntry,
        _ctx: &ResampleCtx,
    ) -> Result<Option<SimTime>, SimError> {
        Ok(None)
    }

    fn invariants(&self, _: &WorldTables) -> Vec<InvariantResult> {
        Vec::new()
    }

    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

fn engine(persons: &[(u64, i64, i64)], commanders: Vec<Commander>) -> Engine {
    let table = specs();
    let m2 = Demography::new(
        Q16_16::from_int(30),
        None,
        Q16_16::ONE,
        Q16_16::from_bits(321_126), // 4.9
        Q16_16::ONE,
        D360,
    )
    .unwrap();
    let mut systems: Vec<Box<dyn SystemModel<Index = ()>>> =
        vec![Box::new(m2) as Box<dyn SystemModel<Index = ()>>];
    for c in commanders {
        systems.push(Box::new(c) as Box<dyn SystemModel<Index = ()>>);
    }
    let registered = register(systems, &table).unwrap_or_else(|e| panic!("{e}"));
    let mut t = tables(persons);
    t.insert("test.plan", vec![1, 0, 0]);
    Engine::new(
        D360,
        7,
        BranchSalt::from_raw(0),
        registered,
        t,
        SimTime::EPOCH,
    )
}

/// The ledger row of `id`, or `None` when absent.
fn row_of(t: &WorldTables, id: u64) -> Option<usize> {
    t.get("person.id")?
        .iter()
        .position(|v| v.cast_unsigned() == id)
}

fn col(t: &WorldTables, name: &str) -> Vec<i64> {
    t.get(name).cloned().unwrap_or_default()
}

// ----------------------------------------------------------------------------- pure scan tests

#[test]
fn fertility_band_pins_the_15_to_49_band() {
    let birth = |age: i64| -(age * YEAR);
    assert_eq!(fertility_band(0, birth(14), YEAR), None);
    assert_eq!(fertility_band(0, birth(15), YEAR), Some(0));
    assert_eq!(fertility_band(0, birth(49), YEAR), Some(6));
    assert_eq!(fertility_band(0, birth(50), YEAR), None);
}

#[test]
fn pairing_candidates_applies_age_bounds_and_every_exclusion() {
    // Woman (index 0) aged 25. Men: 17 (underage), 18, 30, 35 (out of gap), 30 but named, and 40
    // who is kin(w, 2).
    let births = [
        -(25 * YEAR),
        -(17 * YEAR),
        -(18 * YEAR),
        -(30 * YEAR),
        -(35 * YEAR),
        -(30 * YEAR),
        -(40 * YEAR),
    ];
    let ids = [100_i64, 101, 102, 103, 104, 105, 106];
    let alive = [1; 7];
    let sex = [
        SEX_FEMALE, SEX_MALE, SEX_MALE, SEX_MALE, SEX_MALE, SEX_MALE, SEX_MALE,
    ];
    let spouse = [0; 7];
    let named = [0, 0, 0, 0, 0, 1, 0];
    let vocation = [0; 7];
    let residence = [0; 7];
    let w_kin = BTreeSet::from([106_u64]);
    let matched = BTreeSet::new();

    // Preferred gap: the groom is within [woman - 15, woman + 5] years, i.e. [10, 30].
    let preferred = pairing_candidates(
        &ids, &births, &alive, &sex, &spouse, &named, &vocation, &residence, 0, 0, &w_kin,
        &matched, YEAR, true,
    );
    assert_eq!(
        preferred,
        vec![102, 103],
        "18 and 30 in gap; 35 out, 17 underage, named and kin excluded"
    );

    // Without the gap tier: 18, 30 and 35 all qualify.
    let relaxed = pairing_candidates(
        &ids, &births, &alive, &sex, &spouse, &named, &vocation, &residence, 0, 0, &w_kin,
        &matched, YEAR, false,
    );
    assert_eq!(relaxed, vec![102, 103, 104]);
}

#[test]
fn kin2_reaches_degree_two_and_excludes_self() {
    let mut parents: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
    let mut children: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    // w = 1. Parents 2 (father) and 3 (mother); sibling 4; child 5 (co-parent 6); grandchild 7;
    // grandparents 8 and 9 (the father's parents).
    parents.insert(1, (2, 3));
    parents.insert(4, (2, 3));
    parents.insert(5, (1, 6));
    parents.insert(7, (5, 10));
    parents.insert(2, (8, 9));
    for (parent, kid) in [(2, 1), (2, 4), (3, 1), (3, 4), (1, 5), (6, 5), (5, 7)] {
        children.entry(parent).or_default().push(kid);
    }
    let kin = kin2(1, &parents, &children);
    assert_eq!(kin, BTreeSet::from([2, 3, 4, 5, 6, 7, 8, 9]));
    assert!(!kin.contains(&1));
}

#[test]
fn husband_pick_is_crn_stable_under_non_winning_candidates() {
    // The draw is a pure function of the candidate, so the winner is fixed.
    let draws = |h: u64| -> Result<u64, SimError> {
        Ok(match h {
            10 => 5,
            11 | 13 => 3,
            12 => 7,
            _ => 9,
        })
    };
    let winner = pick_husband(&[10, 11, 12, 13], draws).unwrap().unwrap();
    assert_eq!(winner, 11, "least draw; 13 ties at 3 but 11 < 13");

    // Adding or removing non-winning candidates never changes the winner.
    assert_eq!(
        pick_husband(&[10, 11, 12, 13, 14], draws).unwrap(),
        Some(11)
    );
    assert_eq!(pick_husband(&[11, 13], draws).unwrap(), Some(11));
    assert_eq!(pick_husband(&[], draws).unwrap(), None);
}

// ----------------------------------------------------------------------------- engine tests

#[test]
fn apply_marry_sets_spouse_appends_a_row_and_emits_marriage() {
    // A (male, 30) and B (female, 25), unrelated and unmarried.
    let a = 101_u64;
    let b = 102_u64;
    let persons = [(a, -(30 * YEAR), SEX_MALE), (b, -(25 * YEAR), SEX_FEMALE)];
    let mut e = engine(
        &persons,
        vec![Commander {
            tick: DAY,
            kind: "Marry",
            a,
            b: b.cast_signed(),
        }],
    );
    e.advance_until(SimTime::from_minutes(4 * DAY), |_, _| {})
        .unwrap();
    let events = e.drain_events();
    let marriages: Vec<_> = events.iter().filter(|ev| ev.kind == MARRIAGE).collect();
    assert_eq!(marriages.len(), 1, "one MARRIAGE event");
    let marriage = marriages[0];
    assert_eq!(
        marriage
            .roles
            .iter()
            .find(|r| r.role == RoleId::ACTOR)
            .unwrap()
            .entity
            .raw(),
        a
    );
    assert_eq!(
        marriage
            .roles
            .iter()
            .find(|r| r.role == RoleId::TARGET)
            .unwrap()
            .entity
            .raw(),
        b
    );

    let t = e.tables();
    let (ra, rb) = (row_of(t, a).unwrap(), row_of(t, b).unwrap());
    assert_eq!(col(t, "person.spouse")[ra].cast_unsigned(), b);
    assert_eq!(col(t, "person.spouse")[rb].cast_unsigned(), a);
    // One marriage row, open, start at the drain tick, source = command.
    assert_eq!(col(t, "marriage.husband"), vec![a.cast_signed()]);
    assert_eq!(col(t, "marriage.wife"), vec![b.cast_signed()]);
    assert_eq!(col(t, "marriage.start_tick"), vec![marriage.tick]);
    assert_eq!(col(t, "marriage.end_tick"), vec![i64::MAX]);
    assert_eq!(col(t, "marriage.source"), vec![MAR_SOURCE_COMMAND]);
    // H3: a new household headed by the husband, both members.
    let hh = col(t, "person.household")[ra];
    assert_ne!(hh, 0);
    assert_eq!(col(t, "person.household")[rb], hh);
    assert_eq!(col(t, "household.id"), vec![hh]);
    assert_eq!(col(t, "household.head"), vec![a.cast_signed()]);
}

#[test]
fn a_head_death_widows_the_survivor_and_transfers_headship() {
    // A (male, 30) heads the household he shares with his wife B (female, 25).
    let a = 201_u64;
    let b = 202_u64;
    let persons = [(a, -(30 * YEAR), SEX_MALE), (b, -(25 * YEAR), SEX_FEMALE)];
    let table = specs();
    let mut t = tables(&persons);
    let hh = 9_001_i64;
    t.insert("person.spouse", vec![b.cast_signed(), a.cast_signed()]);
    t.insert("person.household", vec![hh, hh]);
    t.insert("household.id", vec![hh]);
    t.insert("household.head", vec![a.cast_signed()]);
    t.insert("household.formed_tick", vec![0]);
    t.insert("household.dissolved_tick", vec![i64::MAX]);
    t.insert("marriage.husband", vec![a.cast_signed()]);
    t.insert("marriage.wife", vec![b.cast_signed()]);
    t.insert("marriage.start_tick", vec![0]);
    t.insert("marriage.end_tick", vec![i64::MAX]);
    t.insert("marriage.source", vec![MAR_SOURCE_GENESIS]);
    t.insert("test.plan", vec![1, 0, 0]);

    let m2 = Demography::new(
        Q16_16::from_int(30),
        None,
        Q16_16::ONE,
        Q16_16::from_bits(321_126),
        Q16_16::ONE,
        D360,
    )
    .unwrap();
    let registered = register(
        vec![
            Box::new(m2) as Box<dyn SystemModel<Index = ()>>,
            Box::new(Commander {
                tick: DAY,
                kind: "ScheduleDeath",
                a,
                b: 1, // natural
            }) as Box<dyn SystemModel<Index = ()>>,
        ],
        &table,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let mut e = Engine::new(
        D360,
        7,
        BranchSalt::from_raw(0),
        registered,
        t,
        SimTime::EPOCH,
    );
    e.advance_until(SimTime::from_minutes(4 * DAY), |_, _| {})
        .unwrap();
    e.drain_events();

    let t = e.tables();
    // A died and its ledger row was compacted; B survives, widowed.
    assert!(row_of(t, a).is_none());
    let rb = row_of(t, b).unwrap();
    assert_eq!(col(t, "person.spouse")[rb], 0);
    // The marriage row is closed at the death instant.
    assert_eq!(col(t, "marriage.end_tick"), vec![DAY]);
    // H4: headship moved to the surviving spouse; the household stays open.
    assert_eq!(col(t, "household.head"), vec![b.cast_signed()]);
    assert_eq!(col(t, "household.dissolved_tick"), vec![i64::MAX]);
}

#[test]
fn the_last_members_death_dissolves_the_household() {
    // S (male, 40) alone in a one-person household.
    let s = 301_u64;
    let persons = [(s, -(40 * YEAR), SEX_MALE)];
    let table = specs();
    let mut t = tables(&persons);
    let hh = 9_101_i64;
    t.insert("person.household", vec![hh]);
    t.insert("household.id", vec![hh]);
    t.insert("household.head", vec![s.cast_signed()]);
    t.insert("household.formed_tick", vec![0]);
    t.insert("household.dissolved_tick", vec![i64::MAX]);
    t.insert("test.plan", vec![1, 0, 0]);

    let m2 = Demography::new(
        Q16_16::from_int(30),
        None,
        Q16_16::ONE,
        Q16_16::from_bits(321_126),
        Q16_16::ONE,
        D360,
    )
    .unwrap();
    let registered = register(
        vec![
            Box::new(m2) as Box<dyn SystemModel<Index = ()>>,
            Box::new(Commander {
                tick: DAY,
                kind: "ScheduleDeath",
                a: s,
                b: 1,
            }) as Box<dyn SystemModel<Index = ()>>,
        ],
        &table,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let mut e = Engine::new(
        D360,
        7,
        BranchSalt::from_raw(0),
        registered,
        t,
        SimTime::EPOCH,
    );
    e.advance_until(SimTime::from_minutes(4 * DAY), |_, _| {})
        .unwrap();
    e.drain_events();

    let t = e.tables();
    // S died (compacted), and its one-person household dissolved at the death instant.
    assert!(row_of(t, s).is_none());
    assert_eq!(col(t, "household.head"), vec![0]);
    assert_eq!(col(t, "household.dissolved_tick"), vec![DAY]);
}
