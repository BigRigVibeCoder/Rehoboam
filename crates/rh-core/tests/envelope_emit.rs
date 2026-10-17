//! SPR-020 T-03: `StepCtx::emit` checks each draft against its kind's `[[emits]]` declaration
//! (`specs/chronicle/envelope_v0.md` §1, §2, §4; REV-SPR-020 R7, R10, R12 NIT-5).
//!
//! - **Refusals:** one planted draft per rule, each refused with its named `SimError`, which names
//!   the system, the kind and the offending name or code; a refused draft is never emitted.
//! - **Canonical order:** the cast and the snapshots come out sorted whatever order the draft gave,
//!   over three world sizes and eight seeds.
//! - **V-DATA-08 in miniature:** a stub in the shape of T1 snapshots `stress(actor)` from the column
//!   it owns before its own write for the act; the stored value is the emitter's pre-act value, not
//!   the column's value at the end of the step. A control emitter that snapshots after its write is
//!   caught by the same comparison.
//! - **Absent sources:** a snapshot read from a column whose owner is not registered is discarded
//!   and counted under `dropped{system, snapshot, absent_owner}`.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    reason = "test helpers: a broken stub world must fail the test loudly; one step plants every case"
)]

mod support;

use std::collections::BTreeMap;

use rh_core::access::Lag;
use rh_core::engine::Engine;
use rh_core::ownership::{AttributeDomain, EmitDecl, RegistryTable, SnapshotDecl, SpecTable};
use rh_core::register::register;
use rh_core::system::{PulseBinding, SimError, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{DraftRef, Event, EventDraft, EventRef, RoleId};
use rh_ids::rng::{h64_words, tag};
use rh_ids::{BranchSalt, EntityRef, PersonId};
use support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const MINDS: &str = "Z1.minds";
const STRESS: &str = "mind.stress";
const CASE: &str = "z.case";
const MOOD: &str = "mind.mood";
const BREAK: &str = "MENTAL_BREAK";
const OMEN: &str = "OMEN";
const MOODY: &str = "MOODY";
const RELIEF: i64 = 60 * 256;

fn person(row: usize) -> EntityRef {
    EntityRef::person(PersonId::from_raw(row as u64))
}

fn snapshot(attr: &str, of: &str, partner: Option<&str>, from: &str) -> SnapshotDecl {
    SnapshotDecl {
        attr: attr.to_owned(),
        of: of.to_owned(),
        partner: partner.map(str::to_owned),
        from: from.to_owned(),
        ty: from.starts_with("algorithm.").then(|| "q8_8".to_owned()),
    }
}

fn roles(names: &[&str]) -> Vec<String> {
    names.iter().map(|r| (*r).to_owned()).collect()
}

/// The planted case the step runs, from `z.case`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    /// A sound `MENTAL_BREAK`: the snapshot of the pre-act stress, then the write (V-DATA-08).
    Break,
    /// The control: the write, then the snapshot of the new value.
    BreakLate,
    /// A role the kind does not list (instrument).
    UndeclaredRole,
    /// Two actors.
    TwoActors,
    /// One witness twice.
    SameWitnessTwice,
    /// 257 witnesses.
    CastTooLarge,
    /// A snapshot of an attribute the kind does not declare.
    UndeclaredSnapshot,
    /// stress of the target, declared of the actor.
    SnapshotOfAnotherRole,
    /// stress outside `q8_8`.
    StressOutOfDomain,
    /// the weather enum past its values.
    WeatherOutOfDomain,
    /// The stress snapshot twice.
    DuplicateSnapshot,
    /// No stress snapshot although the actor is cast.
    MissingSnapshot,
    /// A motive the kind does not list.
    UndeclaredMotive,
    /// A break with no actor: the declared snapshot's role is unfilled, so it is absent.
    NoActor,
    /// A MOODY event with its mood snapshot, whose source's owner is absent.
    AbsentSource,
    /// The cast and snapshots in a shuffled order (seeded by the stress column).
    Shuffled,
}

const CASES: [Case; 16] = [
    Case::Break,
    Case::BreakLate,
    Case::UndeclaredRole,
    Case::TwoActors,
    Case::SameWitnessTwice,
    Case::CastTooLarge,
    Case::UndeclaredSnapshot,
    Case::SnapshotOfAnotherRole,
    Case::StressOutOfDomain,
    Case::WeatherOutOfDomain,
    Case::DuplicateSnapshot,
    Case::MissingSnapshot,
    Case::UndeclaredMotive,
    Case::NoActor,
    Case::AbsentSource,
    Case::Shuffled,
];

/// The actor of a break: the most stressed person (the lowest row on a tie).
fn actor_row(stress: &[i64]) -> usize {
    let max = stress.iter().copied().max().unwrap_or(0);
    stress.iter().position(|&s| s == max).unwrap_or(0)
}

fn minds_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let case = CASES[usize::try_from(v.read(CASE)?[0]).unwrap()];
    let stress = v.read(STRESS)?.to_vec();
    let a = actor_row(&stress);
    let t = (a + 1) % stress.len();
    let pre = i32::try_from(stress[a]).unwrap();
    let draft = || {
        EventDraft::new(BREAK)
            .role(RoleId::ACTOR, person(a))
            .role(RoleId::TARGET, person(t))
            .relational_snapshot("trust", person(a), person(t), -3)
    };
    let cause = c.emit(EventDraft::new(OMEN).event_snapshot("weather", 1))?;
    let with = |d: EventDraft| d.motivated_by(EventRef::Draft(cause), "GRIEF_FOR", 120, 30);
    match case {
        Case::Break => {
            c.emit(with(draft().snapshot("stress", person(a), pre)))?;
            v.write(STRESS)?[a] = stress[a] - RELIEF;
        }
        Case::BreakLate => {
            v.write(STRESS)?[a] = stress[a] - RELIEF;
            let after = i32::try_from(v.read(STRESS)?[a]).unwrap();
            c.emit(with(draft().snapshot("stress", person(a), after)))?;
        }
        Case::UndeclaredRole => {
            c.emit(draft().role(RoleId::INSTRUMENT, EntityRef::WORLD).snapshot(
                "stress",
                person(a),
                pre,
            ))?;
        }
        Case::TwoActors => {
            c.emit(
                draft()
                    .role(RoleId::ACTOR, person(t))
                    .snapshot("stress", person(a), pre),
            )?;
        }
        Case::SameWitnessTwice => {
            c.emit(draft().witness(person(t)).witness(person(t)).snapshot(
                "stress",
                person(a),
                pre,
            ))?;
        }
        Case::CastTooLarge => {
            let mut d = draft().snapshot("stress", person(a), pre);
            for k in 0..255 {
                d = d.witness(person(1_000 + k));
            }
            c.emit(d)?;
        }
        Case::UndeclaredSnapshot => {
            c.emit(draft().snapshot("stress", person(a), pre).snapshot(
                "holds_title",
                person(a),
                1,
            ))?;
        }
        Case::SnapshotOfAnotherRole => {
            c.emit(draft().snapshot("stress", person(t), pre))?;
        }
        Case::StressOutOfDomain => {
            c.emit(draft().snapshot("stress", person(a), 40_000))?;
        }
        Case::WeatherOutOfDomain => {
            c.emit(EventDraft::new(OMEN).event_snapshot("weather", 3))?;
        }
        Case::DuplicateSnapshot => {
            c.emit(
                draft()
                    .snapshot("stress", person(a), pre)
                    .snapshot("stress", person(a), pre),
            )?;
        }
        Case::MissingSnapshot => {
            c.emit(draft())?;
        }
        Case::UndeclaredMotive => {
            c.emit(draft().snapshot("stress", person(a), pre).motivated_by(
                EventRef::Draft(DraftRef(0)),
                "AMBITION_FOR",
                5,
                5,
            ))?;
        }
        Case::NoActor => {
            c.emit(EventDraft::new(BREAK).role(RoleId::TARGET, person(t)))?;
        }
        Case::AbsentSource => {
            c.emit(
                EventDraft::new(MOODY)
                    .role(RoleId::ACTOR, person(a))
                    .snapshot("mood_level", person(a), 7),
            )?;
        }
        Case::Shuffled => {
            // Witnesses, the target and the actor in an order drawn from the stress column; the
            // trust snapshot (actor, target) before the stress one.
            let mut d = EventDraft::new(BREAK);
            let n = stress.len();
            let mut rows: Vec<usize> = (0..n).filter(|&r| r != a && r != t).collect();
            rows.sort_by_key(|&r| {
                h64_words(tag("rehoboam/test/shuffle"), &[stress[r].cast_unsigned()])
            });
            for r in rows.iter().take(5) {
                d = d.witness(person(*r));
            }
            d = d
                .relational_snapshot("trust", person(a), person(t), -3)
                .role(RoleId::TARGET, person(t))
                .snapshot("stress", person(a), pre)
                .role(RoleId::ACTOR, person(a));
            c.emit(d)?;
        }
    }
    Ok(())
}

fn minds(read_mood: bool) -> Stub {
    let break_decl = EmitDecl {
        roles: roles(&["actor", "target", "witness"]),
        motives: roles(&["GRIEF_FOR", "RETALIATION_FOR"]),
        snapshots: vec![
            snapshot("stress", "actor", None, STRESS),
            snapshot("trust", "actor", Some("target"), "algorithm.trust"),
        ],
    };
    let omen_decl = EmitDecl {
        snapshots: vec![snapshot("weather", "event", None, "algorithm.sky")],
        ..EmitDecl::default()
    };
    let moody_decl = EmitDecl {
        roles: roles(&["actor"]),
        snapshots: vec![snapshot("mood_level", "actor", None, MOOD)],
        ..EmitDecl::default()
    };
    let stub = Stub::new(MINDS)
        .writes(STRESS)
        .writes(CASE)
        .emits(BREAK)
        .emits(OMEN)
        .emits(MOODY)
        .declares(BREAK, break_decl)
        .declares(OMEN, omen_decl)
        .declares(MOODY, moody_decl)
        .at(Pulse::Day, Phase::Agenda)
        .stepping(minds_step);
    if read_mood {
        stub.reads(MOOD, "Z9.absent", Lag::SameInstant)
    } else {
        stub
    }
}

fn registry() -> RegistryTable {
    RegistryTable {
        event_roles: roles(&["actor", "target", "witness", "instrument"]),
        motives: roles(&["RETALIATION_FOR", "AMBITION_FOR", "GRIEF_FOR"]),
        attributes: [
            ("holds_title", AttributeDomain::Bool),
            ("mood_level", AttributeDomain::Q8_8),
            ("stress", AttributeDomain::Q8_8),
            ("trust", AttributeDomain::Q8_8),
            ("weather", AttributeDomain::Enum(3)),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect(),
    }
}

/// The stress column of a world of `rows` persons and `seed`.
fn stress_of(rows: usize, seed: u64) -> Vec<i64> {
    (0..rows as u64)
        .map(|r| {
            i64::try_from(h64_words(tag("rehoboam/test/stress"), &[seed, r]) % (100 * 256)).unwrap()
        })
        .collect()
}

struct Ran {
    result: Result<(), SimError>,
    events: Vec<Event>,
    stress_after: Vec<i64>,
    dropped: BTreeMap<(&'static str, String, String), u64>,
}

fn run(case: Case, rows: usize, seed: u64) -> Ran {
    let stubs = vec![minds(case == Case::AbsentSource)];
    let mut specs: SpecTable = agreeing_specs(&stubs);
    specs.registry = registry();
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert(STRESS, stress_of(rows, seed));
    tables.insert(MOOD, vec![0; rows]);
    let index = CASES.iter().position(|c| *c == case).unwrap();
    tables.insert(CASE, vec![i64::try_from(index).unwrap()]);
    let mut e = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    let result = e.advance_until(SimTime::from_minutes(DAY / 2), |_, _| {});
    let dropped = e.dropped().clone();
    let stress_after = e.tables().get(STRESS).cloned().unwrap_or_default();
    Ran {
        result,
        events: e.drain_events(),
        stress_after,
        dropped,
    }
}

/// Each planted draft is refused by its named error, which names the system and the kind; the
/// refused draft is never emitted.
#[test]
fn every_planted_draft_is_refused_by_name() {
    let expect: [(Case, &str, &str); 12] = [
        (Case::UndeclaredRole, "UndeclaredRole", "with role 3"),
        (Case::TwoActors, "DuplicateRole", "of role 0"),
        (Case::SameWitnessTwice, "DuplicateRole", "of role 2"),
        (Case::CastTooLarge, "CastTooLarge", "a cast of 257 rows"),
        (
            Case::UndeclaredSnapshot,
            "UndeclaredSnapshot",
            "of holds_title",
        ),
        (
            Case::SnapshotOfAnotherRole,
            "SnapshotNotOfTheCast",
            "of stress",
        ),
        (
            Case::StressOutOfDomain,
            "SnapshotOutOfDomain",
            "stress = 40000",
        ),
        (
            Case::WeatherOutOfDomain,
            "SnapshotOutOfDomain",
            "weather = 3",
        ),
        (Case::DuplicateSnapshot, "DuplicateSnapshot", "of stress"),
        (Case::MissingSnapshot, "MissingSnapshot", "snapshot stress"),
        (
            Case::UndeclaredMotive,
            "UndeclaredMotive",
            "motive AMBITION_FOR",
        ),
        // The control of the rule below: an absent source is not a missing snapshot.
        (Case::NoActor, "", ""),
    ];
    for (case, variant, detail) in expect {
        let ran = run(case, 12, 3);
        if variant.is_empty() {
            assert_eq!(ran.result, Ok(()), "{case:?}");
            continue;
        }
        let err = ran.result.expect_err(&format!("{case:?}: accepted"));
        let (name, text) = (format!("{err:?}"), err.to_string());
        assert!(name.starts_with(variant), "{case:?}: {name}");
        assert!(text.contains(MINDS), "{case:?}: the system: {text}");
        assert!(text.contains(variant), "{case:?}: the error's name: {text}");
        assert!(
            text.contains(detail),
            "{case:?}: the offending item: {text}"
        );
        let kind = if case == Case::WeatherOutOfDomain {
            OMEN
        } else {
            BREAK
        };
        assert!(text.contains(kind), "{case:?}: the kind: {text}");
    }
    // A break whose actor is unfilled carries no stress snapshot (its role is unfilled).
    let ran = run(Case::NoActor, 12, 3);
    let b = ran.events.iter().find(|e| e.kind == BREAK).unwrap();
    assert!(b.snapshots.is_empty());
}

/// The cast and the snapshots are sorted to their canonical order (§1, §4 rule 6), so the event
/// does not depend on the order the draft gave: role code, then entity; attribute name (the
/// `attr_id` order), then entity, then partner.
#[test]
fn the_cast_and_the_snapshots_come_out_in_canonical_order() {
    for rows in [8, 40, 300] {
        for seed in 1..=8 {
            let ran = run(Case::Shuffled, rows, seed);
            assert_eq!(ran.result, Ok(()), "rows {rows} seed {seed}");
            let b = ran.events.iter().find(|e| e.kind == BREAK).unwrap();
            let mut sorted = b.roles.clone();
            sorted.sort();
            assert_eq!(b.roles, sorted, "rows {rows} seed {seed}");
            assert_eq!(b.roles[0].role, RoleId::ACTOR);
            assert_eq!(b.roles[1].role, RoleId::TARGET);
            assert_eq!(b.roles.len(), 7);
            let attrs: Vec<&str> = b.snapshots.iter().map(|s| s.attr).collect();
            assert_eq!(attrs, ["stress", "trust"], "rows {rows} seed {seed}");
        }
    }
}

/// V-DATA-08 in miniature: the stored stress is the emitter's pre-act value (the oracle: the
/// world's stress before the step), not the column at the end of the step; the late control is
/// caught by the same comparison. Three world sizes and eight seeds.
#[test]
fn v_data_08_the_snapshot_is_the_pre_act_value() {
    let mut caught = 0;
    for rows in [3, 40, 300] {
        for seed in 1..=8 {
            let before = stress_of(rows, seed);
            let a = actor_row(&before);
            for case in [Case::Break, Case::BreakLate] {
                let ran = run(case, rows, seed);
                assert_eq!(ran.result, Ok(()), "{case:?} rows {rows} seed {seed}");
                let b = ran.events.iter().find(|e| e.kind == BREAK).unwrap();
                let stored = i64::from(
                    b.snapshots
                        .iter()
                        .find(|s| s.attr == "stress")
                        .unwrap()
                        .value,
                );
                assert_eq!(ran.stress_after[a], before[a] - RELIEF, "the act wrote");
                assert_ne!(stored, ran.stress_after[a] + 1, "sanity");
                let pre_act = stored == before[a];
                if case == Case::Break {
                    assert!(
                        pre_act,
                        "rows {rows} seed {seed}: {stored} vs {}",
                        before[a]
                    );
                    assert_ne!(stored, ran.stress_after[a], "not the end-of-step column");
                } else {
                    assert!(!pre_act, "the late control must be caught");
                    caught += 1;
                }
                // The motive edge coalesced to one, kept with its motive, weight and margin.
                let edge = b.causes[0];
                assert_eq!(
                    (edge.motive, edge.weight, edge.evidence),
                    (Some("GRIEF_FOR"), 120, 30)
                );
            }
        }
    }
    assert_eq!(caught, 24);
    println!("V-DATA-08 miniature: 24 pre-act snapshots equal the oracle; 24 late controls caught");
}

/// §4 rule 4: a snapshot whose source's owner is not registered is discarded silently and
/// counted; the event is emitted without it.
#[test]
fn an_absent_source_is_discarded_and_counted() {
    let ran = run(Case::AbsentSource, 12, 5);
    assert_eq!(ran.result, Ok(()));
    let m = ran.events.iter().find(|e| e.kind == MOODY).unwrap();
    assert!(m.snapshots.is_empty(), "{:?}", m.snapshots);
    assert_eq!(
        ran.dropped,
        [(
            (MINDS, "MOODY.mood_level".to_owned(), "Z9.absent".to_owned()),
            1
        )]
        .into_iter()
        .collect()
    );
}
