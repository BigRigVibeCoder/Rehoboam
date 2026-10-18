//! The Tester's probes on SPR-014's queue rules, as tests (the Tester's interim on 921d372, item
//! 3): the dead-person drop only where there is a ledger, a key person with no ledger row, and the
//! re-sample's entity-kind check (BCK-002 A-50).
//!
//! `E1.life` drains `Kill` (`key = "living"`) at Day/Vital and counts what it applies;
//! `F1.kill` sends at Day/Agenda: `Kill` of person 0 on day 1, and `Kill` of a person with no
//! ledger row on day 2.

use std::collections::BTreeSet;

use rh_core::agenda::TimerEntry;
use rh_core::command::{Arg, Command};
use rh_core::engine::{Engine, MaterialInput};
use rh_core::register::register;
use rh_core::system::{
    DropKind, DropReason, PulseBinding, ResampleCtx, SimError, SpecPurpose, StepCtx, TimerSpec,
};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::EventRef;
use rh_ids::{BranchSalt, EntityRef, PersonId, RegionId};

use super::delivery::person;
use crate::support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;

/// A person `delivery::person` never makes for a small row: no ledger row has it.
fn stranger() -> PersonId {
    PersonId::from_raw(0x00AB_CDEF)
}

fn life_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let applied = c.take_commands().len();
    v.write("life.applied")?[0] += i64::try_from(applied).unwrap_or(i64::MAX);
    Ok(())
}

fn kill_step(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let victim = match c.now().minutes() / DAY {
        1 => person(0),
        2 => stranger(),
        _ => return Ok(()),
    };
    c.submit(Command {
        kind: "Kill",
        person: victim,
        args: vec![Arg::Int(victim.to_raw().cast_signed())],
        cause: EventRef::None,
    })
}

/// The two systems; with `ledger`, `E1.life` also owns the ledger's columns.
fn engine(ledger: bool) -> Engine {
    let mut life = Stub::new("E1.life").writes("life.applied");
    if ledger {
        for column in [
            "person.id",
            "person.alive",
            "life_course.person_id",
            "life_course.exit_kind",
            "life_course.exit_tick",
            "life_course.death_cause",
        ] {
            life = life.writes(column);
        }
    }
    let stubs = vec![
        life.at(Pulse::Day, Phase::Vital)
            .applies("Kill")
            .typed("Kill", &[("victim", "PersonId")], "victim", "living")
            .stepping(life_step),
        Stub::new("F1.kill")
            .sends("Kill", "E1.life")
            .at(Pulse::Day, Phase::Agenda)
            .stepping(kill_step),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("life.applied", vec![0]);
    if ledger {
        let mut ids: Vec<i64> = (0..3).map(|r| person(r).to_raw().cast_signed()).collect();
        ids.sort_unstable();
        tables.insert("person.id", ids.clone());
        tables.insert("person.alive", vec![1; 3]);
        tables.insert("life_course.person_id", ids);
        tables.insert("life_course.exit_kind", vec![0; 3]);
        tables.insert("life_course.exit_tick", vec![i64::MAX; 3]);
        tables.insert("life_course.death_cause", vec![0; 3]);
    }
    Engine::new(
        CALENDAR,
        7,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    )
}

fn through_day(e: &mut Engine, day: i64) {
    e.advance_until(SimTime::from_minutes((day + 1) * DAY - 1), |_, _| {})
        .unwrap_or_else(|err| panic!("{err}"));
}

/// `key = "living"` drops nothing in a world with no ledger (`person.id` unregistered): both
/// `Kill`s, the stranger's included, reach the owner.
#[test]
fn key_living_drops_nothing_without_a_ledger() {
    let mut e = engine(false);
    through_day(&mut e, 3);
    assert_eq!(e.tables().get("life.applied").map(|t| t[0]), Some(2));
    assert!(e.dropped().is_empty(), "{:?}", e.dropped());
    println!("probes: key = living without a ledger delivers every command");
}

/// In a ledger world, a `key = "living"` command whose person has no ledger row is dropped at the
/// take (`dead_key`), never delivered; the living person's is delivered.
#[test]
fn a_key_person_with_no_ledger_row_is_dropped_as_dead_key() {
    let mut e = engine(true);
    through_day(&mut e, 3);
    assert_eq!(
        e.tables().get("life.applied").map(|t| t[0]),
        Some(1),
        "only person 0's Kill"
    );
    assert_eq!(
        e.dropped()
            .get(&("E1.life", DropKind::Command("Kill"), DropReason::DeadKey)),
        Some(&1)
    );
    assert_eq!(e.dropped().len(), 1);
}

enum Due {
    Due,
}

impl SpecPurpose for Due {
    const ALL: &'static [Self] = &[Self::Due];
    fn name(&self) -> &'static str {
        "due"
    }
}

/// Schedules one timer under `due` (keyed by person) a month on, for the entity `timer.region`
/// picks: a region when it is 1, else person 0.
fn schedule_due(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now().minutes() != 0 {
        return Ok(());
    }
    let entity = if v.read("timer.region")?[0] == 1 {
        EntityRef::region(RegionId::from_raw(4))
    } else {
        EntityRef::person(person(0))
    };
    c.schedule(
        &Due::Due,
        TimerSpec {
            time: SimTime::from_minutes(30 * DAY),
            phase: Phase::Vital,
            entity,
            kind: 1,
            payload: 0,
            cause: EventRef::None,
        },
    )
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "it must have the ResampleFn signature"
)]
fn redraw(_: &Stub, _: &TimerEntry, ctx: &ResampleCtx) -> Result<Option<SimTime>, SimError> {
    Ok(ctx.now().checked_add_minutes(DAY))
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "it must have the FireFn signature"
)]
fn fire_nothing(
    _: &Stub,
    _: &TimerEntry,
    _: &mut SystemView<'_>,
    _: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    Ok(())
}

/// The re-sample checks the timer's entity kind against its purpose's `[[rng]] key` (A-50): a
/// person-keyed timer held for a region is refused by name when an input re-samples it; one held
/// for a person is re-drawn.
#[test]
fn the_resample_checks_the_draw_kind() {
    let run = |region: bool| {
        let stubs = vec![
            Stub::new("R1.timer")
                .writes("timer.region")
                .purposes_of::<Due>()
                .rng_key("due", "person")
                .at(Pulse::Day, Phase::Vital)
                .stepping(schedule_due)
                .firing(fire_nothing)
                .resampling(redraw),
        ];
        let specs = agreeing_specs(&stubs);
        let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
            .unwrap_or_else(|e| panic!("{e}"));
        let mut tables = WorldTables::new();
        tables.insert("timer.region", vec![i64::from(region)]);
        let mut e = Engine::new(
            CALENDAR,
            7,
            BranchSalt::from_raw(0),
            registered,
            tables,
            SimTime::EPOCH,
        );
        through_day(&mut e, 0);
        let entity = if region {
            EntityRef::region(RegionId::from_raw(4))
        } else {
            EntityRef::person(person(0))
        };
        let scope: BTreeSet<EntityRef> = [entity].into_iter().collect();
        e.submit_input(SimTime::from_minutes(2 * DAY), MaterialInput { scope })
            .unwrap_or_else(|err| panic!("{err}"));
        e.advance_until(SimTime::from_minutes(3 * DAY), |_, _| {})
    };
    run(false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        run(true).unwrap_err().to_string(),
        "R1.timer drew under purpose due, whose [[rng]] key is a person, with a region (BCK-002 A-50)"
    );
    println!("probes: the re-sample's draw kind is checked");
}
