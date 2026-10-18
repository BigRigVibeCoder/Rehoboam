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

/// `A1.send`'s Day/Vital step: with `send.bad`, seven malformed submissions first, each refused
/// with its own error (the step goes on); then, in both worlds, one good `Req` and one good `Key`.
fn send_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let p = person(1);
    let req = |args: Vec<Arg>, cause: EventRef| Command {
        kind: "Req",
        person: p,
        args,
        cause,
    };
    if v.read("send.bad")?[0] == 1 {
        let none = || Arg::Event(EventRef::None);
        let draft = EventRef::Draft(rh_events::DraftRef::from_index(9));
        let refused = [
            ("arity", c.submit(req(vec![Arg::Int(1)], EventRef::None))),
            (
                "u8 range",
                c.submit(req(vec![Arg::Int(300), none()], EventRef::None)),
            ),
            (
                "int in event",
                c.submit(req(vec![Arg::Int(1), Arg::Int(5)], EventRef::None)),
            ),
            (
                "event in int",
                c.submit(req(vec![none(), none()], EventRef::None)),
            ),
            (
                "unknown draft field",
                c.submit(req(vec![Arg::Int(1), Arg::Event(draft)], EventRef::None)),
            ),
            (
                "unknown draft cause",
                c.submit(req(vec![Arg::Int(1), none()], draft)),
            ),
            (
                "key mismatch",
                c.submit(Command {
                    kind: "Key",
                    person: p,
                    args: vec![Arg::Int(person(2).to_raw().cast_signed())],
                    cause: EventRef::None,
                }),
            ),
        ];
        for (what, result) in refused {
            let right = match (what, &result) {
                ("arity" | "u8 range" | "key mismatch", Err(SimError::Payload { .. }))
                | ("unknown draft field" | "unknown draft cause", Err(SimError::Event { .. })) => {
                    true
                }
                ("int in event", Err(SimError::EventSlot { field, .. })) => field == "ev",
                ("event in int", Err(SimError::EventSlot { field, .. })) => field == "n",
                _ => false,
            };
            assert!(right, "{what}: {result:?}");
        }
        assert!(matches!(
            c.submit(Command {
                kind: "Nope",
                person: p,
                args: Vec::new(),
                cause: EventRef::None,
            }),
            Err(SimError::UndeclaredCommand { .. })
        ));
    }
    c.submit(req(
        vec![Arg::Int(1), Arg::Event(EventRef::None)],
        EventRef::None,
    ))?;
    c.submit(Command {
        kind: "Key",
        person: p,
        args: vec![Arg::Int(p.to_raw().cast_signed())],
        cause: EventRef::None,
    })
}

fn own_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let taken = c.take_commands().len();
    v.write("own.n")?[0] += i64::try_from(taken).unwrap_or(i64::MAX);
    Ok(())
}

fn refusal_world(bad: bool) -> Engine {
    let stubs = vec![
        Stub::new("A1.send")
            .writes("send.bad")
            .sends("Req", "B1.own")
            .sends("Key", "B1.own")
            .at(Pulse::Day, Phase::Vital)
            .stepping(send_step),
        Stub::new("B1.own")
            .writes("own.n")
            .at(Pulse::Day, Phase::Agenda)
            .applies("Req")
            .typed("Req", &[("n", "u8"), ("ev", "EventRef")], "person", "any")
            .applies("Key")
            .typed("Key", &[("who", "PersonId")], "who", "any")
            .stepping(own_step),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("send.bad", vec![i64::from(bad)]);
    tables.insert("own.n", vec![0]);
    Engine::new(
        CALENDAR,
        11,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    )
}

/// The Tester's P-B3: refused submissions touch nothing. A world whose sender tries seven
/// malformed commands and an undeclared kind each day ends with the same tables (but its flag),
/// queues, event counter and drop counters as one that never tried.
#[test]
fn refused_submissions_touch_nothing() {
    let (mut bad, mut good) = (refusal_world(true), refusal_world(false));
    through_day(&mut bad, 3);
    through_day(&mut good, 3);
    let (mut tb, mut tg) = (bad.tables().clone(), good.tables().clone());
    tb.insert("send.bad", vec![0]);
    tg.insert("send.bad", vec![0]);
    assert_eq!(tb, tg, "tables");
    assert_eq!(bad.commands().state(), good.commands().state(), "queues");
    assert_eq!(bad.events().next_id(), good.events().next_id(), "event ids");
    assert_eq!(bad.dropped(), good.dropped(), "drop counters");
    assert_eq!(
        good.tables().get("own.n").map(|t| t[0]),
        Some(8),
        "two good commands a day, drained in the same day's Agenda (after Vital), days 0-3"
    );
    println!("probes: refused submissions touch nothing");
}

/// `A1.send` submits one `Req` on 1 February (day 31 of a 365-day year).
fn february_send(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now().minutes() != 31 * DAY {
        return Ok(());
    }
    c.submit(Command {
        kind: "Req",
        person: person(1),
        args: Vec::new(),
        cause: EventRef::None,
    })
}

/// `B1.own` drains at Month/Vital, but skips 1 March (day 59).
fn skip_march(
    s: &Stub,
    b: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now().minutes() == 59 * DAY {
        return Ok(());
    }
    own_step(s, b, v, c)
}

/// The Tester's P-C8: queue age at a `Month` drain in a 365-day calendar. A `Req` sent on 1
/// February and not drained on 1 March has waited the month it was queued in (28 days), so the
/// skipped drain trips `queue-no-stale-command` at that instant.
#[test]
fn a_skipped_month_drain_in_a_365_day_year_trips_the_queue_age() {
    let stubs = vec![
        Stub::new("A1.send")
            .writes("send.n")
            .sends("Req", "B1.own")
            .at(Pulse::Day, Phase::Agenda)
            .stepping(february_send),
        Stub::new("B1.own")
            .writes("own.n")
            .at(Pulse::Month, Phase::Vital)
            .applies("Req")
            .stepping(skip_march),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("send.n", vec![0]);
    tables.insert("own.n", vec![0]);
    let calendar = Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four);
    let mut e = Engine::new(
        calendar,
        5,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    let err = e
        .advance_until(SimTime::from_minutes(100 * DAY), |_, _| {})
        .expect_err("a skipped Month drain must trip the queue age");
    let text = err.to_string();
    println!("probes: {text}");
    assert!(
        text.starts_with(
            "invariant queue-no-stale-command of rh-core violated: B1.own's Req command"
        ) && text.contains(&format!("has waited {} minutes", 28 * DAY)),
        "{text}"
    );
}

fn untyped_send(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    c.submit(Command {
        kind: "Note",
        person: person(1),
        args: vec![Arg::Int(1), Arg::Event(EventRef::None)],
        cause: EventRef::None,
    })
}

/// The review's K1: an untyped command (its owner's `[[applies]]` declares no payload) with an
/// event field is refused at `submit`, naming the sender, the kind and the field position, never
/// queued to fail at the next keyframe.
#[test]
fn an_event_field_in_an_untyped_command_is_refused_at_submit() {
    let stubs = vec![
        Stub::new("A1.send")
            .sends("Note", "B1.own")
            .at(Pulse::Day, Phase::Agenda)
            .stepping(untyped_send),
        Stub::new("B1.own")
            .writes("own.n")
            .at(Pulse::Day, Phase::Vital)
            .applies("Note")
            .stepping(own_step),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("own.n", vec![0]);
    let mut e = Engine::new(
        CALENDAR,
        3,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    let err = e
        .advance_until(SimTime::from_minutes(DAY - 1), |_, _| {})
        .expect_err("an event slot in an untyped command must be refused");
    assert_eq!(
        err.to_string(),
        "A1.send submitted command Note: field 1 is an event slot, but B1.own's [[applies]] \
         declares no payload: an untyped command carries integers only"
    );
    assert_eq!(e.commands().len(), 0, "nothing queued");
}

/// `F1.doom` at day 0's Agenda: schedules its outcome timer, in `Vital` (where it is also bound,
/// so the timer can fire), at the minute `doom.at` holds.
fn doom_schedule(
    _: &Stub,
    b: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now().minutes() != 0 || b.phase != Phase::Agenda {
        return Ok(());
    }
    c.schedule(
        &Due::Due,
        TimerSpec {
            time: SimTime::from_minutes(v.read("doom.at")?[0]),
            phase: Phase::Vital,
            entity: EntityRef::person(person(1)),
            kind: 1,
            payload: 0,
            cause: EventRef::None,
        },
    )
}

/// The outcome timer's fire submits the outcome-carrying `Doom` (R9: only a sender's timer does).
fn doom_fire(
    _: &Stub,
    _: &TimerEntry,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    c.submit(Command {
        kind: "Doom",
        person: person(1),
        args: Vec::new(),
        cause: EventRef::None,
    })
}

fn doom_world(at: i64) -> Engine {
    let stubs = vec![
        Stub::new("F1.doom")
            .writes("doom.at")
            .purposes_of::<Due>()
            .sends("Doom", "E1.own")
            .at(Pulse::Day, Phase::Agenda)
            .at(Pulse::Day, Phase::Vital)
            .stepping(doom_schedule)
            .firing(doom_fire),
        Stub::new("E1.own")
            .writes("own.n")
            .at(Pulse::Day, Phase::Vital)
            .applies("Doom")
            .outcome("Doom")
            .stepping(own_step),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("doom.at", vec![at]);
    tables.insert("own.n", vec![0]);
    Engine::new(
        CALENDAR,
        3,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    )
}

/// The review's K3 (R9, 05.1 §2 rule 2): an outcome-carrying command is applied in the instant it
/// is submitted. Fired at a day start, Day/Vital's drain runs after the timer pass and applies it;
/// fired at 12:00 (no Day pulse) the merge refuses it, naming the sender and the command.
#[test]
fn an_outcome_command_whose_drain_does_not_follow_is_refused_at_the_merge() {
    let mut day_start = doom_world(DAY);
    through_day(&mut day_start, 1);
    assert_eq!(day_start.tables().get("own.n").map(|t| t[0]), Some(1));
    assert_eq!(day_start.commands().len(), 0, "applied in its instant");
    let err = doom_world(DAY + 720)
        .advance_until(SimTime::from_minutes(2 * DAY), |_, _| {})
        .expect_err("an outcome submitted where its drain does not follow must be refused");
    let text = err.to_string();
    assert!(
        text.starts_with("F1.doom")
            && text.contains("submitted outcome-carrying command Doom at (t=2160min, Vital)")
            && text.contains("its drain Day/Vital does not run later in this instant"),
        "{text}"
    );
    println!("probes: an outcome command is refused unless its drain follows in the instant");
}
