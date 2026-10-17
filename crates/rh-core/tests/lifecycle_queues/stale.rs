//! V-INV-15 on stub systems (SPR-014 T-03; `05.1` §2 rule 2, "The dead-person drop" and "Queue
//! age"; BCK-002 A-30, A-31; REV-EVO-001 R1, R3, R11, NIT-15).
//!
//! A world with a ledger: `E1.life` owns `person.id`, `person.alive` and the life-course columns,
//! drains `Kill` (`key = "living"`) and `Bury` (`key = "any"`) at Day/Vital, and holds natural
//! death timers. `F1.kill` sends at Day/Agenda and Day/Polity. The script, on five persons P0-P4:
//!
//! | When | What | Expected |
//! |---|---|---|
//! | day 0, Vital | E1 schedules natural deaths: P0 at day 2, P2 at day 3 | |
//! | day 1, Agenda | F1 sends Kill P0, Kill P1 twice, Kill P2 | queued to day 2 |
//! | day 2, Vital timer pass | P0 dies naturally | |
//! | day 2, Vital drain | Kill P0: P0 dead at the level's start | `dropped{E1.life, Kill, dead_key}` |
//! | | Kill P1 applied, the second Kill P1 finds P1 dead | `dropped{E1.life, Kill, dead}` |
//! | | Kill P2 applied | |
//! | day 2, Polity | F1 sends Bury P0 (dead, `key = "any"`) | |
//! | day 3, Vital timer pass | P2's natural timer fires for the dead P2 | `dropped{E1.life, timer.1, dead}` |
//! | day 3, Vital drain | Bury P0 applied | |
//!
//! Controls: an owner that skips its day-2 drain trips `queue-no-stale-command`, naming the
//! owner, the kind and the age; a non-idempotent death that rewrites a closed life-course row
//! trips the ledger's "exit fields are written once" (V-INV-01).

use rh_core::agenda::TimerEntry;
use rh_core::command::{Arg, Command};
use rh_core::engine::Engine;
use rh_core::register::register;
use rh_core::system::{
    DropKind, DropReason, PulseBinding, SimError, SpecPurpose, StepCtx, TimerSpec,
};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::EventRef;
use rh_ids::{BranchSalt, EntityRef, PersonId};

use super::delivery::person;
use crate::support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const PERSONS: usize = 5;

enum Natural {
    Natural,
}

impl SpecPurpose for Natural {
    const ALL: &'static [Self] = &[Self::Natural];
    fn name(&self) -> &'static str {
        "natural"
    }
}

/// The five persons in ascending id order (the ledger is sorted by `person.id`), so `P(i)` is
/// row `i`.
fn ids() -> Vec<u64> {
    let mut ids: Vec<u64> = (0..PERSONS).map(|r| person(r).to_raw()).collect();
    ids.sort_unstable();
    ids
}

fn p(i: usize) -> PersonId {
    PersonId::from_raw(ids()[i])
}

fn row(v: &SystemView<'_>, who: PersonId) -> Result<Option<usize>, SimError> {
    v.person_row(who)
}

fn living(v: &SystemView<'_>, who: PersonId) -> Result<bool, SimError> {
    Ok(row(v, who)?.is_some_and(|r| v.read("person.alive").is_ok_and(|a| a[r] == 1)))
}

/// The person's open life-course row.
fn open_row(v: &SystemView<'_>, who: PersonId) -> Result<Option<usize>, SimError> {
    let persons = v.read("life_course.person_id")?;
    let kinds = v.read("life_course.exit_kind")?;
    Ok(persons
        .iter()
        .zip(kinds)
        .position(|(&q, &k)| q.cast_unsigned() == who.to_raw() && k == 0))
}

fn kill(v: &mut SystemView<'_>, who: PersonId, now: i64) -> Result<(), SimError> {
    let (Some(r), Some(lc)) = (row(v, who)?, open_row(v, who)?) else {
        return Err(SimError::System {
            system: "E1.life",
            message: format!("kill of {who}, who has no living row"),
        });
    };
    v.write("person.alive")?[r] = 0;
    v.write("life_course.exit_kind")?[lc] = 1;
    v.write("life_course.exit_tick")?[lc] = now;
    v.write("life_course.death_cause")?[lc] = 2;
    v.write("life.kills")?[0] += 1;
    Ok(())
}

fn life_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let day = c.now().minutes() / DAY;
    if day == 0 {
        for (who, at) in [(p(0), 2), (p(2), 3)] {
            c.schedule(
                &Natural::Natural,
                TimerSpec {
                    time: SimTime::from_minutes(at * DAY),
                    phase: Phase::Vital,
                    entity: EntityRef::person(who),
                    kind: 1,
                    payload: 0,
                    cause: EventRef::None,
                },
            )?;
        }
    }
    if v.read("life.skip_day")?[0] == day {
        return Ok(()); // the planted skipped drain: the inbox stays queued
    }
    for q in c.take_commands() {
        match q.command.kind {
            "Kill" if living(v, q.command.person)? => kill(v, q.command.person, c.now().minutes())?,
            "Kill" => c.drop_command(&q, DropReason::Dead),
            "Bury" => v.write("life.buried")?[0] += 1,
            other => panic!("unexpected {other}"),
        }
    }
    Ok(())
}

fn life_fire(
    _: &Stub,
    t: &TimerEntry,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let who = PersonId::from_raw(t.key.entity.raw());
    if living(v, who)? {
        return kill(v, who, c.now().minutes());
    }
    if v.read("life.bad")?[0] == 1 {
        // The planted non-idempotent death: it rewrites the dead person's closed exit fields.
        let persons = v.read("life_course.person_id")?.to_vec();
        if let Some(lc) = persons
            .iter()
            .position(|&q| q.cast_unsigned() == who.to_raw())
        {
            v.write("life_course.exit_tick")?[lc] = c.now().minutes();
        }
        return Ok(());
    }
    c.drop_timer(t, DropReason::Dead);
    Ok(())
}

fn kill_step(
    _: &Stub,
    b: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let day = c.now().minutes() / DAY;
    let sends: Vec<(&'static str, usize)> = match (day, b.phase) {
        (1, Phase::Agenda) => vec![("Kill", 0), ("Kill", 1), ("Kill", 1), ("Kill", 2)],
        (2, Phase::Polity) => vec![("Bury", 0)],
        _ => Vec::new(),
    };
    for (kind, i) in sends {
        c.submit(Command {
            kind,
            person: p(i),
            args: vec![Arg::Int(p(i).to_raw().cast_signed())],
            cause: EventRef::None,
        })?;
    }
    Ok(())
}

fn engine(skip_day: i64, bad: bool) -> Engine {
    let stubs = vec![
        Stub::new("E1.life")
            .writes("person.id")
            .writes("person.alive")
            .writes("life_course.person_id")
            .writes("life_course.exit_kind")
            .writes("life_course.exit_tick")
            .writes("life_course.death_cause")
            .writes("life.kills")
            .writes("life.buried")
            .writes("life.skip_day")
            .writes("life.bad")
            .purposes_of::<Natural>()
            .at(Pulse::Day, Phase::Vital)
            .applies("Kill")
            .applies("Bury")
            .typed("Kill", &[("victim", "PersonId")], "victim", "living")
            .typed("Bury", &[("deceased", "PersonId")], "deceased", "any")
            .stepping(life_step)
            .firing(life_fire),
        Stub::new("F1.kill")
            .sends("Kill", "E1.life")
            .sends("Bury", "E1.life")
            .at(Pulse::Day, Phase::Polity)
            .at(Pulse::Day, Phase::Agenda)
            .stepping(kill_step),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let ids: Vec<i64> = ids().into_iter().map(u64::cast_signed).collect();
    let mut tables = WorldTables::new();
    tables.insert("person.id", ids.clone());
    tables.insert("person.alive", vec![1; PERSONS]);
    tables.insert("life_course.person_id", ids);
    tables.insert("life_course.exit_kind", vec![0; PERSONS]);
    tables.insert("life_course.exit_tick", vec![i64::MAX; PERSONS]);
    tables.insert("life_course.death_cause", vec![0; PERSONS]);
    tables.insert("life.kills", vec![0]);
    tables.insert("life.buried", vec![0]);
    tables.insert("life.skip_day", vec![skip_day]);
    tables.insert("life.bad", vec![i64::from(bad)]);
    Engine::new(
        CALENDAR,
        7,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    )
}

fn through_day(e: &mut Engine, day: i64) -> Result<(), SimError> {
    e.advance_until(SimTime::from_minutes((day + 1) * DAY - 1), |_, _| {})
}

/// V-INV-15: each drop is counted under `dropped{system, kind, reason}` and the dropped command or
/// timer is never applied; a `key = "any"` command for the dead is applied.
#[test]
fn v_inv_15_drops_are_counted_and_never_applied() {
    let mut e = engine(-1, false);
    through_day(&mut e, 4).unwrap_or_else(|err| panic!("{err}"));
    let dropped: Vec<(&str, String, &str, u64)> = e
        .dropped()
        .iter()
        .map(|((s, k, r), n)| (*s, k.to_string(), r.name(), *n))
        .collect();
    assert_eq!(
        dropped,
        [
            ("E1.life", "Kill".to_owned(), "dead_key", 1),
            ("E1.life", "Kill".to_owned(), "dead", 1),
            ("E1.life", "timer.1".to_owned(), "dead", 1),
        ]
    );
    let t = e.tables();
    // P0 died naturally; P1 and P2 by Kill (the dropped Kill P0 and the second Kill P1 never ran).
    assert_eq!(
        t.get("life.kills").unwrap()[0],
        3,
        "P0's natural death, then P1 and P2"
    );
    assert_eq!(
        t.get("life.buried").unwrap()[0],
        1,
        "Bury of the dead P0 (key = any) applied"
    );
    let left: Vec<u64> = t
        .get("person.id")
        .unwrap()
        .iter()
        .map(|v| v.cast_unsigned())
        .collect();
    assert_eq!(left, [ids()[3], ids()[4]]);
    assert_eq!(e.commands().len(), 0);
    assert_eq!(
        e.dropped()
            .get(&("E1.life", DropKind::Command("Kill"), DropReason::DeadKey)),
        Some(&1)
    );
}

/// Positive control: an owner that skips one drain trips `queue-no-stale-command`, naming the
/// owner, the kind and the age (`05.1` §2 rule 2, queue age; §9).
#[test]
fn a_skipped_drain_trips_queue_no_stale_command() {
    let mut e = engine(2, false);
    let err = through_day(&mut e, 3).expect_err("a skipped drain must be fatal");
    let text = err.to_string();
    assert!(
        text.starts_with(
            "invariant queue-no-stale-command of rh-core violated: E1.life's Kill command seq "
        ) && text.contains("from F1.kill has waited 1440 minutes at "),
        "{text}"
    );
    assert!(
        e.crash_artifact()
            .is_some_and(|a| a.invariant.as_deref() == Some("queue-no-stale-command"))
    );
}

/// Positive control: a death applied twice to one person (a non-idempotent `apply_death`) trips the
/// ledger's life-course rule, V-INV-01's "exit fields are written once".
#[test]
fn a_non_idempotent_death_trips_exit_fields_written_once() {
    let mut e = engine(-1, true);
    let err = through_day(&mut e, 4).expect_err("a second death must be refused");
    let text = err.to_string();
    assert!(
        text.starts_with("E1.life: ledger: life_course.exit_tick row ")
            && text.ends_with("on a closed entry: exit fields are written once (SPR-011 ruling 5)"),
        "{text}"
    );
}
