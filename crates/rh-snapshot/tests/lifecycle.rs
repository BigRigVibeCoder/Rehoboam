//! SPR-014 on the plane hash and the keyframe (V-DET-05's harness), on stub systems:
//!
//! - **T-02:** a queued command with an `EventRef` payload field and a pending timer with a cause
//!   survive a keyframe round trip, on three world sizes; after the load the run continues exactly
//!   like the continuous one (`05.1` §2 rule 2: a keyframe stores a timer's `cause` as the same row
//!   bytes, `specs/chronicle/envelope_v0.md` §2).
//! - **T-05:** ledger rows inserted by the core (`StepCtx::insert_person`) round-trip the same way.
//! - **T-03:** the `dropped{…}` counter changes no hash: a run whose owner also counts a planted
//!   extra drop has the same monthly `state_hash` ladder (GOV-006 §3; the style of `tests/observer.rs`).
//!
//! Refs: `CODEX/05_PROJECT/SPR-014_LifecycleQueues.md` T-02, T-03, T-05; `docs/09` V-DET-05,
//! V-DET-21, V-INV-15.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests"
)]

mod support;

use rh_core::agenda::TimerEntry;
use rh_core::command::{Arg, Command};
use rh_core::crash::Provenance;
use rh_core::engine::Engine;
use rh_core::register::{Registered, register};
use rh_core::system::{DropReason, PulseBinding, SimError, SpecPurpose, StepCtx, TimerSpec};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{EventDraft, EventRef};
use rh_ids::rng::{h64_words, tag};
use rh_ids::{BranchSalt, EntityRef, PersonId};
use rh_snapshot::journal::{JournalPosition, chain_genesis};
use rh_snapshot::keyframe::{History, load_engine, write_keyframe};
use rh_snapshot::state::{AGENDA_TIMERS, ENGINE_COMMANDS, engine_tables};
use rh_snapshot::state_hash;
use support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const TAG: u64 = tag("rehoboam/rh-snapshot/tests/spr014/v1");
const SIZES: [usize; 3] = [3, 17, 64];
const SEEDS: [u64; 3] = [5, 77, 0x5EED];

enum Cp {
    Pick,
}

impl SpecPurpose for Cp {
    const ALL: &'static [Self] = &[Self::Pick];
    fn name(&self) -> &'static str {
        "pick"
    }
}

fn person(row: usize) -> PersonId {
    PersonId::from_raw(h64_words(TAG, &[row as u64]) >> 4)
}

/// `C1.cause` at Day/Agenda: emits `SENT`, submits `Note` with the draft as its cause and its
/// `source_event`, and schedules a timer three days on with the draft as its cause; it also
/// submits one `Note` a day with no source event. `D1.keep`
/// drains `Note` at Day/Agenda, in the same level, so every `Note` waits a day in the queue.
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
        c.schedule(
            &Cp::Pick,
            TimerSpec {
                time: SimTime::from_minutes(c.now().minutes() + 3 * DAY),
                phase: Phase::Agenda,
                entity: EntityRef::person(person(row)),
                kind: 1,
                payload: marker,
                cause: EventRef::Draft(sent),
            },
        )?;
    }
    // Each day one Note carries no source event: its field is hashed as -1 (the Tester's probe).
    c.submit(Command {
        kind: "Note",
        person: person(0),
        args: vec![Arg::Int(-day), Arg::Event(EventRef::None)],
        cause: EventRef::None,
    })?;
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
    v.write("cause.seen")?.extend([t.timer.payload, id]);
    Ok(())
}

fn keep_step(
    s: &Stub,
    b: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    keep(s, b, v, c, false)
}

/// The same drain with a planted extra drop: the flag is the step function, never state.
fn keep_step_planted(
    s: &Stub,
    b: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    keep(s, b, v, c, true)
}

fn keep(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
    planted: bool,
) -> Result<(), SimError> {
    let mut rows = Vec::new();
    for q in c.take_commands() {
        let id = q
            .command
            .event(1)
            .map_or(-1, |id| id.to_raw().cast_signed());
        rows.extend([q.command.int(0).unwrap_or(-1), id]);
        if planted {
            // A planted extra drop: counted, and nothing else changes.
            c.drop_command(&q, DropReason::Ended);
        }
    }
    v.write("keep.seen")?.extend(rows);
    Ok(())
}

fn slot_stubs() -> Vec<Stub> {
    slot_stubs_with(keep_step)
}

fn slot_stubs_with(keep: support::StepFn) -> Vec<Stub> {
    vec![
        Stub::new("C1.cause")
            .writes("cause.size")
            .writes("cause.seen")
            .purposes_of::<Cp>()
            .emits("SENT")
            .sends("Note", "D1.keep")
            .at(Pulse::Day, Phase::Agenda)
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
            .stepping(keep),
    ]
}

fn registered_of(stubs: Vec<Stub>) -> Registered {
    let specs = agreeing_specs(&stubs);
    register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap_or_else(|e| panic!("{e}"))
}

fn history(seed: u64) -> History {
    History {
        provenance: Provenance {
            build: "test-build".to_owned(),
            registry_version: "0.1.0".to_owned(),
            manifest_hash: "spr014".to_owned(),
        },
        world_seed: seed,
        branch_salt: 0,
    }
}

fn anchor() -> JournalPosition {
    JournalPosition {
        timeline_id: 0,
        inputs: 0,
        head: chain_genesis(0),
    }
}

fn slot_engine(seed: u64, size: usize, planted: bool) -> Engine {
    let mut tables = WorldTables::new();
    tables.insert("cause.size", vec![i64::try_from(size).unwrap()]);
    tables.insert("cause.seen", Vec::new());
    tables.insert("keep.seen", Vec::new());
    let mut e = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered_of(slot_stubs_with(if planted {
            keep_step_planted
        } else {
            keep_step
        })),
        tables,
        SimTime::EPOCH,
    );
    e.set_provenance(history(seed).provenance);
    e
}

/// The monthly `state_hash` ladder from where `e` stands to day `end`.
fn ladder(e: &mut Engine, end: i64) -> Vec<(i64, u64)> {
    let mut out = Vec::new();
    while let Some(t) = e.next_instant().filter(|t| t.minutes() < end * DAY) {
        e.advance_until(t, |_, _| {})
            .unwrap_or_else(|err| panic!("{err}"));
        drop(e.drain_events());
        if CALENDAR.is_boundary(Pulse::Month, t) || t.minutes() % (10 * DAY) == 0 {
            out.push((t.minutes(), state_hash(&e.snapshot())));
        }
    }
    out
}

/// T-02: a keyframe taken while a `Note` with a resolved `EventRef` field and one with an empty
/// field (hashed as -1) wait in their queue and timers with causes are pending loads to the same
/// state, and the loaded run continues exactly like the continuous one, on three world sizes and
/// three seeds.
#[test]
fn queued_event_slots_and_timer_causes_survive_a_keyframe() {
    for size in SIZES {
        for seed in SEEDS {
            let mut e = slot_engine(seed, size, false);
            e.advance_until(SimTime::from_minutes(7 * DAY), |_, _| {})
                .unwrap_or_else(|err| panic!("{err}"));
            let state = e.snapshot();
            // The probe: the cut really holds what the case claims.
            let queued = state.commands.queues.iter().flat_map(|(_, q)| q.iter());
            assert!(
                queued.clone().any(|q| q.command.event(1).is_some()),
                "{size} {seed}: no queued command with a resolved event field"
            );
            assert!(
                queued
                    .clone()
                    .any(|q| q.command.args.get(1) == Some(&Arg::Event(EventRef::None))),
                "{size} {seed}: no queued command with an empty event field"
            );
            assert!(
                state
                    .agenda
                    .entries
                    .iter()
                    .any(|(_, t)| t.cause.known().is_some()),
                "{size} {seed}: no pending timer with a cause"
            );
            let (bytes, _) = write_keyframe(&e, anchor()).unwrap();
            let (mut loaded, _) = load_engine(
                &bytes,
                CALENDAR,
                registered_of(slot_stubs()),
                &history(seed),
            )
            .unwrap_or_else(|err| panic!("{size} {seed}: {err}"));
            assert_eq!(loaded.snapshot(), state, "{size} {seed}");
            assert_eq!(ladder(&mut loaded, 60), ladder(&mut e, 60), "{size} {seed}");
        }
    }
    println!("T-02 keyframes: queued EventRef fields and timer causes round-trip in 9 worlds");
}

/// T-03: the `dropped{…}` counter is a diagnostic. A run whose owner counts a planted extra drop
/// on every command has the same monthly `state_hash` ladder and the same keyframe bytes, while
/// its counter differs.
#[test]
fn the_dropped_counter_changes_no_hash() {
    for seed in SEEDS {
        let mut plain = slot_engine(seed, 17, false);
        let mut planted = slot_engine(seed, 17, true);
        assert_eq!(ladder(&mut plain, 90), ladder(&mut planted, 90));
        assert!(plain.dropped().is_empty());
        assert!(
            !planted.dropped().is_empty(),
            "the planted drop was counted"
        );
        assert_eq!(
            write_keyframe(&plain, anchor()).unwrap().1.state_hash,
            write_keyframe(&planted, anchor()).unwrap().1.state_hash,
        );
    }
    println!("T-03 counter: a planted extra drop changes no state_hash in 3 worlds");
}

// --- T-05: inserted ledger rows through a keyframe --------------------------------------------

enum Ip {
    Born,
}

impl SpecPurpose for Ip {
    const ALL: &'static [Self] = &[Self::Born];
    fn name(&self) -> &'static str {
        "born"
    }
}

fn ledger_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let day = c.now().minutes() / DAY;
    for k in 0..2_u64 {
        let id = h64_words(TAG ^ 0xB0, &[day.cast_unsigned(), k]) >> 4;
        if c.rng(&Ip::Born, PersonId::from_raw(id), 0)? % 3 == 0 {
            c.insert_person(vec![("person.id", id.cast_signed()), ("person.alive", 1)]);
            v.write("life_course.person_id")?.push(id.cast_signed());
            v.write("life_course.exit_kind")?.push(0);
            v.write("life_course.exit_tick")?.push(i64::MAX);
            v.write("life_course.death_cause")?.push(0);
        }
    }
    Ok(())
}

fn ledger_stubs() -> Vec<Stub> {
    vec![
        Stub::new("K1.ledger")
            .writes("person.id")
            .writes("person.alive")
            .writes("life_course.person_id")
            .writes("life_course.exit_kind")
            .writes("life_course.exit_tick")
            .writes("life_course.death_cause")
            .purposes_of::<Ip>()
            .at(Pulse::Day, Phase::Vital)
            .stepping(ledger_step),
        Stub::new("K2.wealth")
            .writes("person.wealth")
            .default_of("person.wealth", 7),
    ]
}

fn ledger_engine(seed: u64, founders: usize) -> Engine {
    let mut ids: Vec<i64> = (0..founders)
        .map(|k| (h64_words(TAG ^ seed, &[k as u64]) >> 4).cast_signed())
        .collect();
    ids.sort_unstable_by_key(|v| v.cast_unsigned());
    let mut t = WorldTables::new();
    t.insert("person.id", ids.clone());
    t.insert("person.alive", vec![1; founders]);
    t.insert("person.wealth", vec![7; founders]);
    t.insert("life_course.person_id", ids);
    t.insert("life_course.exit_kind", vec![0; founders]);
    t.insert("life_course.exit_tick", vec![i64::MAX; founders]);
    t.insert("life_course.death_cause", vec![0; founders]);
    let mut e = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered_of(ledger_stubs()),
        t,
        SimTime::EPOCH,
    );
    e.set_provenance(history(seed).provenance);
    e
}

/// T-05: rows the core inserted round-trip through a keyframe, and the loaded run continues like
/// the continuous one, on three world sizes and three seeds.
#[test]
fn inserted_ledger_rows_survive_a_keyframe() {
    for founders in SIZES {
        for seed in SEEDS {
            let mut e = ledger_engine(seed, founders);
            e.advance_until(SimTime::from_minutes(20 * DAY), |_, _| {})
                .unwrap_or_else(|err| panic!("{err}"));
            let rows = e.tables().get("person.id").unwrap().len();
            assert!(
                rows > founders,
                "{founders} {seed}: no row was inserted by day 20"
            );
            assert_eq!(e.tables().get("person.wealth").unwrap().len(), rows);
            let (bytes, _) = write_keyframe(&e, anchor()).unwrap();
            let (mut loaded, _) = load_engine(
                &bytes,
                CALENDAR,
                registered_of(ledger_stubs()),
                &history(seed),
            )
            .unwrap_or_else(|err| panic!("{founders} {seed}: {err}"));
            assert_eq!(loaded.snapshot(), e.snapshot());
            assert_eq!(
                ladder(&mut loaded, 80),
                ladder(&mut e, 80),
                "{founders} {seed}"
            );
        }
    }
    println!("T-05 keyframes: inserted ledger rows round-trip in 9 worlds");
}

/// A little-endian reader over one hashed row, for the encoding check below: it knows only the
/// layout text (`state.rs`'s table and `05.1` §2), never the encoder.
struct Row<'b> {
    bytes: &'b [u8],
    at: usize,
}

impl Row<'_> {
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let out: [u8; N] = self.bytes[self.at..self.at + N].try_into().unwrap();
        self.at += N;
        out
    }
    fn u8(&mut self) -> u8 {
        self.take::<1>()[0]
    }
    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take())
    }
    fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.take())
    }
    fn i64(&mut self) -> i64 {
        i64::from_le_bytes(self.take())
    }
    fn str(&mut self) -> String {
        let n = usize::try_from(self.u32()).unwrap();
        let s = String::from_utf8(self.bytes[self.at..self.at + n].to_vec()).unwrap();
        self.at += n;
        s
    }
    fn opt_u64(&mut self) -> Option<u64> {
        (self.u8() == 1).then(|| self.u64())
    }
    fn done(&self) -> bool {
        self.at == self.bytes.len()
    }
}

/// One `engine.commands` row: `str owner, i64 tick, u8 phase, u64 person, u64 seq, str sender,
/// opt(u64 cause), str kind, u32 n ‖ n × i64 args, opt(u64 draft)`; returns (args, cause, draft).
fn command_row(bytes: &[u8]) -> (Vec<i64>, Option<u64>, Option<u64>) {
    let mut r = Row { bytes, at: 0 };
    let _owner = r.str();
    let (_tick, _phase, _person, _seq) = (r.i64(), r.u8(), r.u64(), r.u64());
    let _sender = r.str();
    let cause = r.opt_u64();
    let _kind = r.str();
    let n = r.u32();
    let args = (0..n).map(|_| r.i64()).collect();
    let draft = r.opt_u64();
    assert!(r.done(), "trailing bytes in a command row");
    (args, cause, draft)
}

/// The fixed part of an `agenda.timers` row: `i64 time, u8 phase, u8 tag ‖ u64 entity, u64 seq,
/// u32 owner, u64 purpose, u32 kind, i64 payload, u8 tombstoned`.
const TIMER_ROW: usize = 8 + 1 + 9 + 8 + 4 + 8 + 4 + 8 + 1;

/// One timer row: (payload, cause, length); the cause tail `u8 1 ‖ u64` follows the tombstone
/// byte, and only when there is a cause (R15).
fn timer_row(bytes: &[u8]) -> (i64, Option<u64>, usize) {
    let mut r = Row {
        bytes,
        at: TIMER_ROW - 9,
    };
    let payload = r.i64();
    assert_eq!(r.u8(), 0, "not tombstoned");
    let cause = if r.done() {
        None
    } else {
        assert_eq!(r.u8(), 1, "the cause tail's tag");
        Some(r.u64())
    };
    assert!(r.done(), "trailing bytes in a timer row");
    (payload, cause, bytes.len())
}

fn encoding_send(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now().minutes() != 0 {
        return Ok(());
    }
    // The first event of the run: its id is 0, which must stay 0 (never -1, never "none").
    let sent = c.emit(EventDraft::new("SENT").with_payload(vec![1]))?;
    let (p1, p2) = (PersonId::from_raw(11), PersonId::from_raw(12));
    for (who, marker, source) in [(p1, 7, EventRef::Draft(sent)), (p2, 8, EventRef::None)] {
        c.submit(Command {
            kind: "Note",
            person: who,
            args: vec![Arg::Int(marker), Arg::Event(source)],
            cause: source,
        })?;
        c.schedule(
            &Cp::Pick,
            TimerSpec {
                time: SimTime::from_minutes(3 * DAY),
                phase: Phase::Agenda,
                entity: EntityRef::person(who),
                kind: 1,
                payload: marker * 10,
                cause: source,
            },
        )?;
    }
    Ok(())
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "it must have the StepFn signature"
)]
fn encoding_keep(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    drop(c.take_commands());
    Ok(())
}

/// The Tester's encoding probe (R15; `enc_check.py` ported): the hashed rows, decoded from the
/// layout alone. Event id 0 is stored as 0 in an args field, a command cause and a timer cause; an
/// empty event field is -1 and an absent cause adds no byte (a timer row of 51 bytes, 60 with a
/// cause).
#[test]
fn hashed_rows_encode_event_zero_none_and_the_cause_tail_as_the_layout_says() {
    let stubs = vec![
        Stub::new("C1.cause")
            .writes("cause.n")
            .purposes_of::<Cp>()
            .emits("SENT")
            .sends("Note", "D1.keep")
            .at(Pulse::Day, Phase::Agenda)
            .stepping(encoding_send),
        Stub::new("D1.keep")
            .writes("keep.n")
            .at(Pulse::Day, Phase::Agenda)
            .applies("Note")
            .typed(
                "Note",
                &[("marker", "i64"), ("source_event", "EventRef")],
                "person",
                "any",
            )
            .stepping(encoding_keep),
    ];
    let mut tables = WorldTables::new();
    tables.insert("cause.n", vec![0]);
    tables.insert("keep.n", vec![0]);
    let mut e = Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        registered_of(stubs),
        tables,
        SimTime::EPOCH,
    );
    e.advance_until(SimTime::from_minutes(DAY - 1), |_, _| {})
        .unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(
        e.commands().len(),
        2,
        "both commands queued to the next day"
    );
    let rows = engine_tables(&e.snapshot());
    let of = |name: &str| rows.iter().find(|(n, _)| *n == name).unwrap().1.clone();
    let mut commands: Vec<_> = of(ENGINE_COMMANDS).iter().map(|b| command_row(b)).collect();
    commands.sort_unstable();
    assert_eq!(
        commands,
        [(vec![7, 0], Some(0), Some(0)), (vec![8, -1], None, None)],
        "Known(0) is 0, None is -1, an absent cause and draft are absent"
    );
    let mut timers: Vec<_> = of(AGENDA_TIMERS).iter().map(|b| timer_row(b)).collect();
    timers.sort_unstable();
    assert_eq!(
        timers,
        [(70, Some(0), TIMER_ROW + 9), (80, None, TIMER_ROW)],
        "the cause tail u8 1 ‖ u64 only with a cause"
    );
    println!("encoding: event id 0, None as -1 and the cause tail decode from the layout alone");
}
