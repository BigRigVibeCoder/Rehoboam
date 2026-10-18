//! The ledger guard's boundary for drain-only bindings (SPR-014; BCK-002 A-60 part, the
//! Architect's ruling of 2026-10-17).
//!
//! `E1.life` owns `person.id`, `person.alive` and the life-course columns, and drains `Touch` at
//! Day/Vital, a binding it declares drain-only. `F1.touch` sends one `Touch` at day 1's Agenda,
//! so `E1.life`'s day-2 drain has work and its other drains have none. A planted mode in
//! `life.plant` makes the drain misbehave:
//!
//! | Mode | What the drain does | Expected |
//! |---|---|---|
//! | 1 | with its inbox non-empty, writes an exit field of an open entry (no row count changes) | refused by name: the job has work, so the full guard runs |
//! | 2 | with its inbox empty, writes the same field | refused by name: the narrowing is gone (SPR-038 T-03), every write of a guarded column is value-checked from its pre-image |
//! | 3 | with its inbox empty, appends a copy of an open life-course row | fails the core's `life-course-one-open-entry` invariant: a legal append for the guard, a duplicate open entry for the ledger |
//! | 4 | with its inbox empty, inserts a newborn's ledger row and appends its open entry | passes: the append mode 3 makes is legal for the guard when the ledger agrees |
//!
//! Registration refuses a drain-only binding that drains none of the system's commands.

use rh_core::command::{Arg, Command};
use rh_core::engine::Engine;
use rh_core::register::register;
use rh_core::system::{PulseBinding, SimError, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::EventRef;

use super::delivery::person;
use crate::support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const PERSONS: usize = 5;
const LIFE: [&str; 4] = [
    "life_course.person_id",
    "life_course.exit_kind",
    "life_course.exit_tick",
    "life_course.death_cause",
];

fn ids() -> Vec<i64> {
    let mut ids: Vec<i64> = (0..PERSONS)
        .map(|r| person(r).to_raw().cast_signed())
        .collect();
    ids.sort_unstable();
    ids
}

fn life_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let inbox = c.take_commands();
    let day = c.now().minutes() / DAY;
    match (v.read("life.plant")?[0], inbox.is_empty(), day) {
        (1, false, _) | (2, true, 3) => v.write("life_course.death_cause")?[PERSONS - 1] = 3,
        (3, true, 3) => {
            for column in LIFE {
                let value = v.read(column)?[0];
                v.write(column)?.push(value);
            }
        }
        (4, true, 3) => {
            // A newborn: its ledger row through the core, its open entry appended with the open
            // values. Legal for the guard and for the ledger.
            let id = person(PERSONS).to_raw().cast_signed();
            c.insert_person(vec![("person.id", id), ("person.alive", 1)]);
            for (column, value) in LIFE.into_iter().zip([id, 0, i64::MAX, 0]) {
                v.write(column)?.push(value);
            }
        }
        _ => {}
    }
    v.write("life.touched")?[0] += i64::try_from(inbox.len()).unwrap_or(i64::MAX);
    Ok(())
}

fn touch_step(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now().minutes() / DAY == 1 {
        c.submit(Command {
            kind: "Touch",
            person: person(0),
            args: vec![Arg::Int(person(0).to_raw().cast_signed())],
            cause: EventRef::None,
        })?;
    }
    Ok(())
}

fn life() -> Stub {
    let mut stub = Stub::new("E1.life")
        .writes("person.id")
        .writes("person.alive");
    for column in LIFE {
        stub = stub.writes(column);
    }
    stub.writes("life.plant")
        .writes("life.touched")
        .at(Pulse::Day, Phase::Vital)
        .applies("Touch")
        .typed("Touch", &[("who", "PersonId")], "who", "any")
        .drain_only(Pulse::Day, Phase::Vital)
        .stepping(life_step)
}

fn engine(plant: i64) -> Engine {
    let stubs = vec![
        life(),
        Stub::new("F1.touch")
            .sends("Touch", "E1.life")
            .at(Pulse::Day, Phase::Agenda)
            .stepping(touch_step),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let ids = ids();
    let mut tables = WorldTables::new();
    tables.insert("person.id", ids.clone());
    tables.insert("person.alive", vec![1; PERSONS]);
    tables.insert("life_course.person_id", ids);
    tables.insert("life_course.exit_kind", vec![0; PERSONS]);
    tables.insert("life_course.exit_tick", vec![i64::MAX; PERSONS]);
    tables.insert("life_course.death_cause", vec![0; PERSONS]);
    tables.insert("life.plant", vec![plant]);
    tables.insert("life.touched", vec![0]);
    Engine::new(
        CALENDAR,
        7,
        rh_ids::BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    )
}

fn through_day(e: &mut Engine, day: i64) -> Result<(), SimError> {
    e.advance_until(SimTime::from_minutes((day + 1) * DAY - 1), |_, _| {})
}

#[test]
fn a_drain_with_work_that_writes_an_open_exit_field_is_refused_by_name() {
    let mut clean = engine(0);
    through_day(&mut clean, 4).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(clean.tables().get("life.touched").map(|t| t[0]), Some(1));
    let err = through_day(&mut engine(1), 4)
        .expect_err("an exit field written on an open entry must be refused");
    let text = err.to_string();
    assert!(
        text == "E1.life: ledger: life_course.death_cause row 4 changed from 0 to 3 on an entry \
                 this step leaves open: exit fields are written only by the step that closes the \
                 entry (SPR-011 ruling 5; DEF-3)",
        "{text}"
    );
    println!("A-60 part guard: a drain with work is fully guarded; refused by name");
}

/// The narrowing is gone (SPR-038 T-03, S12): a drain-only job with an empty inbox that writes an
/// exit field of an open entry is refused by name, as a job with work is: the guard's pre-image is
/// taken at the first `write()` of a guarded column, whatever the job's work.
#[test]
fn a_drain_without_work_that_writes_is_refused_by_name() {
    let err = through_day(&mut engine(2), 4)
        .expect_err("an exit field written on an open entry must be refused");
    let text = err.to_string();
    assert!(
        text == "E1.life: ledger: life_course.death_cause row 4 changed from 0 to 3 on an entry \
                 this step leaves open: exit fields are written only by the step that closes the \
                 entry (SPR-011 ruling 5; DEF-3)",
        "{text}"
    );
    // An appended entry is a legal write for the guard (an open entry with the open values); the
    // duplicate open entry it makes fails the core's ledger invariant at the instant's Bookkeeping.
    let err = through_day(&mut engine(3), 4).expect_err("an appended row must be refused");
    let text = err.to_string();
    assert!(
        text.starts_with("invariant life-course-one-open-entry of rh-core violated: person ")
            && text.ends_with(" has 2 open life-course entries"),
        "{text}"
    );
    // The same append is otherwise legal for the guard: with its ledger row inserted, the drain
    // without work passes, and the entry and the row are both there.
    let mut legal = engine(4);
    through_day(&mut legal, 4).unwrap_or_else(|e| panic!("a legal append was refused: {e}"));
    assert_eq!(
        legal.tables().get("life_course.person_id").map(Vec::len),
        Some(PERSONS + 1)
    );
    assert_eq!(
        legal.tables().get("person.id").map(Vec::len),
        Some(PERSONS + 1)
    );
    println!("SPR-038 T-03: a drain without work is value-checked; refused by name");
}

#[test]
fn a_drain_only_binding_that_drains_nothing_is_refused_by_name() {
    let stubs = vec![
        life()
            .at(Pulse::Day, Phase::Agenda)
            .drain_only(Pulse::Day, Phase::Agenda),
        Stub::new("F1.touch")
            .sends("Touch", "E1.life")
            .at(Pulse::Day, Phase::Polity)
            .stepping(touch_step),
    ];
    let specs = agreeing_specs(&stubs);
    let err = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .err()
        .expect("a drain-only binding with no drain must be refused");
    let text = err.to_string();
    assert!(
        text.contains(
            "VIOLATION [spec] E1.life: declares Day/Agenda drain-only, but no command it applies is \
             drained there"
        ),
        "{text}"
    );
}
