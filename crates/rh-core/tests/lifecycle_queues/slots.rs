//! V-DET-21, the slot part (SPR-014 T-02; `05.1` §2 rule 2, "Event-typed slots"; rule 4: the merge
//! takes a system's events before its timers and commands).
//!
//! `C1.cause` binds Day/Polity. Each day, for a keyed draw of persons, it emits `SENT` with a
//! unique marker as payload, submits `Note { marker: i64, source_event: EventRef }` to `D1.keep`
//! with the draft as the command's cause and as `source_event`, and schedules a timer for the next
//! day's Polity with the draft as its cause. `D1.keep` drains `Note` at Day/Agenda and records the
//! marker, the field's id and the cause's id; C1's fire records the marker and the timer's cause.
//! Every recorded id must be the id the `SENT` with that marker got at the merge.

use std::collections::BTreeMap;

use rh_core::agenda::TimerEntry;
use rh_core::command::{Arg, Command};
use rh_core::engine::Engine;
use rh_core::register::register;
use rh_core::system::{PulseBinding, SimError, SpecPurpose, StepCtx, TimerSpec};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{Event, EventDraft, EventRef};
use rh_ids::{BranchSalt, EntityRef};

use super::delivery::{SEEDS, SIZES, person};
use crate::support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;

enum Cp {
    Pick,
}

impl SpecPurpose for Cp {
    const ALL: &'static [Self] = &[Self::Pick];
    fn name(&self) -> &'static str {
        "pick"
    }
}

fn cause_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let n = usize::try_from(v.read("cause.size")?[0]).unwrap_or(0);
    let day = c.now().minutes() / DAY;
    for row in 0..n {
        if c.rng(&Cp::Pick, person(row), 0)? % 2 != 0 {
            continue;
        }
        let marker = day * 1_000 + i64::try_from(row).unwrap_or(0);
        let sent = c.emit(EventDraft::new("SENT").with_payload(vec![marker]))?;
        c.submit(Command {
            kind: "Note",
            person: person(row),
            args: vec![Arg::Int(marker), Arg::Event(EventRef::Draft(sent))],
            cause: EventRef::Draft(sent),
        })?;
        let next = SimTime::from_minutes(c.now().minutes() + DAY);
        c.schedule(
            &Cp::Pick,
            TimerSpec {
                time: next,
                phase: Phase::Polity,
                entity: EntityRef::person(person(row)),
                kind: 1,
                payload: marker,
                cause: EventRef::Draft(sent),
            },
        )?;
    }
    Ok(())
}

fn cause_fire(
    _: &Stub,
    t: &TimerEntry,
    v: &mut SystemView<'_>,
    _: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let id = t
        .timer
        .cause
        .known()
        .map_or(-1, |id| id.to_raw().cast_signed());
    let seen = v.write("cause.seen")?;
    seen.extend([t.timer.payload, id]);
    Ok(())
}

fn keep_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let id = |r: Option<rh_ids::EventId>| r.map_or(-1, |id| id.to_raw().cast_signed());
    let mut rows = Vec::new();
    for q in c.take_commands() {
        // The merge resolved every draft: the owner sees ids only.
        assert!(q.command.cause.draft().is_none());
        assert!(
            q.command.args[1]
                .event()
                .is_some_and(|r| r.draft().is_none())
        );
        rows.extend([
            q.command.int(0).unwrap_or(-1),
            id(q.command.event(1)),
            id(q.cause),
            id(q.command.cause.known()),
        ]);
    }
    v.write("keep.seen")?.extend(rows);
    Ok(())
}

fn stubs() -> Vec<Stub> {
    vec![
        Stub::new("C1.cause")
            .writes("cause.size")
            .writes("cause.seen")
            .purposes_of::<Cp>()
            .emits("SENT")
            .sends("Note", "D1.keep")
            .at(Pulse::Day, Phase::Polity)
            .stepping(cause_step)
            .firing(cause_fire),
        Stub::new("D1.keep")
            .writes("keep.seen")
            .at(Pulse::Day, Phase::Agenda)
            .applies("Note")
            .typed(
                "Note",
                &[("marker", "i64"), ("source_event", "EventRef")],
                "person",
                "any",
            )
            .stepping(keep_step),
    ]
}

fn engine(seed: u64, size: usize, threads: usize) -> Engine {
    let stubs = stubs();
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("cause.size", vec![i64::try_from(size).unwrap_or(0)]);
    tables.insert("cause.seen", Vec::new());
    tables.insert("keep.seen", Vec::new());
    let mut e = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    e.set_threads(threads);
    e
}

fn run(mut e: Engine) -> (Vec<Event>, WorldTables) {
    e.advance_until(SimTime::from_minutes(20 * DAY - 1), |_, _| {})
        .unwrap_or_else(|err| panic!("{err}"));
    (e.drain_events(), e.tables().clone())
}

/// V-DET-21, slots: the id a command's cause, its `EventRef` field and a timer's cause carry is
/// the id of the sender's draft, at 1, 2, 4 and 8 threads, on three world sizes and eight seeds.
#[test]
fn v_det_21_event_slots_carry_the_merged_id() {
    let mut checked = (0, 0);
    for size in SIZES {
        for seed in SEEDS {
            let base = run(engine(seed, size, 1));
            for threads in [2, 4, 8] {
                assert_eq!(
                    base,
                    run(engine(seed, size, threads)),
                    "{size} {seed} {threads}"
                );
            }
            let (events, tables) = base;
            let id_of: BTreeMap<i64, i64> = events
                .iter()
                .filter(|e| e.kind == "SENT")
                .map(|e| (e.payload[0], e.id.to_raw().cast_signed()))
                .collect();
            for r in tables.get("keep.seen").unwrap().chunks(4) {
                let want = id_of[&r[0]];
                assert_eq!(r[1..], [want, want, want], "command for marker {}", r[0]);
                checked.0 += 1;
            }
            for r in tables.get("cause.seen").unwrap().chunks(2) {
                assert_eq!(r[1], id_of[&r[0]], "timer for marker {}", r[0]);
                checked.1 += 1;
            }
        }
    }
    assert!(checked.0 > 0 && checked.1 > 0, "{checked:?}");
}

// --- Refusals at submit (05.1 §2 rule 2; SPR-014 T-02) ----------------------------------------

fn bad_args(_: &Stub, row: i64) -> Vec<Arg> {
    let who = person(0).to_raw().cast_signed();
    match row {
        0 => vec![Arg::Int(who), Arg::Int(1), Arg::Int(77)],
        1 => vec![Arg::Int(who), Arg::Int(2), Arg::Event(EventRef::None)],
        2 => vec![
            Arg::Int(person(1).to_raw().cast_signed()),
            Arg::Int(1),
            Arg::Event(EventRef::None),
        ],
        3 => vec![
            Arg::Event(EventRef::None),
            Arg::Int(1),
            Arg::Event(EventRef::None),
        ],
        _ => vec![Arg::Int(who)],
    }
}

fn bad_step(
    s: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let case = v.read("bad.case")?[0];
    c.submit(Command {
        kind: "Audit",
        person: person(0),
        args: bad_args(s, case),
        cause: EventRef::None,
    })
}

fn refusal(case: i64) -> String {
    let stubs = vec![
        Stub::new("G1.bad")
            .writes("bad.case")
            .sends("Audit", "H1.audit")
            .at(Pulse::Day, Phase::Polity)
            .stepping(bad_step),
        Stub::new("H1.audit")
            .at(Pulse::Day, Phase::Agenda)
            .applies("Audit")
            .typed(
                "Audit",
                &[
                    ("who", "PersonId"),
                    ("flag", "bool"),
                    ("source_event", "EventRef"),
                ],
                "who",
                "any",
            ),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("bad.case", vec![case]);
    let mut e = Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    e.advance_until(SimTime::EPOCH, |_, _| {})
        .expect_err("refused")
        .to_string()
}

#[test]
fn an_integer_in_an_event_slot_and_a_payload_outside_its_types_are_refused_by_name() {
    assert_eq!(
        refusal(0),
        "G1.bad submitted command Audit with an integer in its event-typed field source_event, or an event slot in an integer field (05.1 §2 rule 2: an EventRef slot takes a DraftRef or an EventId, never an integer)"
    );
    assert_eq!(
        refusal(3),
        "G1.bad submitted command Audit with an integer in its event-typed field who, or an event slot in an integer field (05.1 §2 rule 2: an EventRef slot takes a DraftRef or an EventId, never an integer)"
    );
    assert_eq!(
        refusal(1),
        "G1.bad submitted command Audit: field flag = 2 is outside its type Bool"
    );
    assert!(
        refusal(2).starts_with("G1.bad submitted command Audit: its key person ")
            && refusal(2).ends_with("differs from its key field who (05.1 §2 rule 2)"),
        "{}",
        refusal(2)
    );
    assert_eq!(
        refusal(4),
        "G1.bad submitted command Audit: 1 fields, but H1.audit's [[applies]] payload has 3"
    );
}
