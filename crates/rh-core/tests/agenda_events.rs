//! T-02, T-03 and T-04 on a stub "life world", and V-DET-12 (SPR-006).
//!
//! The world, on 48 persons (person ids are hashed, so `PersonId` order is not row order):
//! - `X1.vital` (Month, Vital) samples death times for the living as timers at intra-month
//!   minutes up to 60 days ahead (ADR-0025 item 1), cancels some pending ones (a cure), and on a
//!   firing timer marks the death, emits `DEATH` and submits `Inherit` to the estate, with the
//!   `DEATH` draft as the command's cause. Its fire handler refuses a timer whose person has no
//!   pending death: a cancelled timer that fired would stop the run;
//! - `X5.war` (Month, Military) submits `Inherit` for a few living persons, with no cause;
//! - `X2.estate` (Day, Agenda) owns and applies `Inherit`: it checks that its queue arrives in
//!   `(tick, phase, PersonId, seq)` order and emits `INHERITANCE`, caused by the death if any;
//! - `X3.rumor` and `X4.census` (Month, Agenda) share a scheduler level with the estate, so the
//!   three run on different threads; the census emits a `CENSUS_NOTE` caused by its own `CENSUS`
//!   draft (a draft-local causal edge); the rumor checks it never receives a command;
//! - `X9.observer` (Hour, Bookkeeping) does nothing: it stands for a watched run, which runs
//!   every hour instant.
//!
//! Tests:
//! - thread-count invariance at 1, 2, 4 and 8 threads (ruling 1): events, agenda state, tables and
//!   command queues identical;
//! - every mode: with and without the observer, the same events at the same ticks; death ticks
//!   fall between pulse boundaries (T-02);
//! - V-DET-12 over 16 random small worlds, instant by instant: every new event's tick is the
//!   current instant, `EventId`s strictly increase in `(tick, phase, seq)` order, causes point
//!   back, and every sampled outcome sits in the agenda as a future timer until it fires;
//! - material-input re-sampling: only the scope's timers are re-drawn, keyed by epoch; the input
//!   applies before a timer due at the same tick (T-02; ADR-0025 items 3 and 4);
//! - refusals: back-dated or unbound-phase timers, another system's timer, undeclared commands and
//!   events, and registration of a non-owner applier or a second emitter (T-03, T-04);
//! - the golden event log (`tests/golden/life_world_events.txt`): V-DET-12 is checked on the
//!   committed log, which the run must reproduce. `RH_CORE_LIFE_GOLDEN` points the test at another
//!   log: `bin/check_all.sh` plants a back-dated one that must fail.
//!
//! Refs: SPR-006 T-02–T-04, rulings 1–5; ADR-0025; `docs/09` V-DET-12; `05.1` §2 rule 2, §4.1.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken stub world must fail the test loudly"
)]

mod support;

/// SPR-014: lifecycle queues for P1a, on stub systems (`05.1` §2 rule 2, all six parts; rule 4's
/// merge order; EVO-001 §7). A module of the `agenda_events` test binary, as the plan names it.
///
/// - **V-DET-21, delivery** (T-01): a property test over random small worlds (three sizes, eight
///   seeds). A command reaches only its owner's `drained_by` binding, at that binding's first run
///   after the merge of the sender's level: within the instant to a later phase, at the next day
///   start when sent at or after the binding's run; in key order; equal at 1, 2, 4 and 8 threads.
///   A probe counts the same-instant deliveries and the cross-instant waits, and both are non-zero.
///   Every other job of the owner (another binding, its timer pass) refuses a non-empty inbox with
///   "delivered to a non-draining job", which the planted engine of EVO-001 option D1
///   (`--features mutant-deliver-every-job`) must trip.
/// - **V-DET-21, slots** (T-02): every `EventRef` an owner sees, in a command's cause, an
///   event-typed payload field and a timer's cause, is the id the sender's draft got at the merge,
///   at every thread count; an integer in an event slot, a field outside its type and a key person
///   that differs from the key field are refused at `submit` by name.
/// - **V-INV-15** (T-03): the dead-person drop at take, the owner's drops, the death timer for the
///   dead, `key = "any"`, the queue-age invariant and its control, the non-idempotent death.
/// - **V-INV-04** (T-04): a rate or mode read of an absent owner, served empty with its typed
///   neutral; a send to an absent owner, discarded and counted.
/// - **The row insert** (T-05): random worlds with two or three owners of `person.*` columns.
///
/// Refs: `CODEX/05_PROJECT/SPR-014_LifecycleQueues.md` T-01–T-05; `docs/09` V-DET-21, V-INV-15,
/// V-INV-04; REV-EVO-001 R1–R19.
mod lifecycle_queues {
    pub mod delivery;
    pub mod slots;
}

use std::path::PathBuf;

use rh_core::agenda::{AgendaState, TimerEntry};
use rh_core::command::Command;
use rh_core::engine::{Engine, InputError, MaterialInput};
use rh_core::register::{RegistrationError, register};
use rh_core::system::{PulseBinding, ResampleCtx, SimError, SpecPurpose, StepCtx, TimerSpec};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{Event, EventDraft};
use rh_ids::rng::{h64_words, tag};
use rh_ids::{BranchSalt, EntityRef, PersonId};
use support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const TAG: u64 = tag("rehoboam/rh-core/tests/agenda-events/v1");
/// Death timers land up to this many minutes ahead, so some are still pending a month later.
const HORIZON: u64 = 60 * 1_440;

/// The vital system's purposes.
enum Vp {
    DeathTime,
    Cure,
}

impl SpecPurpose for Vp {
    const ALL: &'static [Self] = &[Self::DeathTime, Self::Cure];
    fn name(&self) -> &'static str {
        match self {
            Self::DeathTime => "death_time",
            Self::Cure => "cure",
        }
    }
}

/// The war system's purpose.
enum Wp {
    Levy,
}

impl SpecPurpose for Wp {
    const ALL: &'static [Self] = &[Self::Levy];
    fn name(&self) -> &'static str {
        "levy"
    }
}

fn person(row: usize) -> PersonId {
    PersonId::from_raw(h64_words(TAG, &[row as u64]) >> 4)
}

fn entity(row: usize) -> EntityRef {
    EntityRef::person(person(row))
}

fn r64(row: usize) -> i64 {
    i64::try_from(row).unwrap()
}

fn row_of(payload: i64) -> usize {
    usize::try_from(payload).unwrap()
}

fn vital_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let rows = v.read("vital.alive")?.len();
    let alive = v.read("vital.alive")?.to_vec();
    let mut pending = v.read("vital.pending")?.to_vec();
    let rate = v.read("vital.rate")?[0].cast_unsigned();
    for row in 0..rows {
        if alive[row] == 0 {
            continue;
        }
        if pending[row] == 0 {
            if c.rng(&Vp::DeathTime, person(row), 0)? % rate == 0 {
                let offset = 1 + c.rng(&Vp::DeathTime, person(row), 1)? % HORIZON;
                let time = c.now().checked_add_minutes(offset.cast_signed()).unwrap();
                c.schedule(
                    &Vp::DeathTime,
                    TimerSpec {
                        time,
                        phase: Phase::Vital,
                        entity: entity(row),
                        kind: 0,
                        payload: r64(row),
                        cause: rh_events::EventRef::None,
                    },
                )?;
                pending[row] = 1;
            }
        } else if c.rng(&Vp::Cure, person(row), 0)? % 5 == 0 {
            for t in c.own_timers(entity(row)) {
                c.cancel(t.id())?;
            }
            pending[row] = 0;
            c.emit(EventDraft::new("CURED").subject(entity(row)))?;
        }
    }
    v.write("vital.pending")?.copy_from_slice(&pending);
    Ok(())
}

fn vital_fire(
    s: &Stub,
    timer: &TimerEntry,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let row = row_of(timer.timer.payload);
    let fail = |message: String| SimError::System {
        system: s.id,
        message,
    };
    if c.now() != timer.key.time || c.phase() != timer.key.phase {
        return Err(fail(format!("{} fired off its own tick", timer.id())));
    }
    if v.read("vital.pending")?[row] != 1 || v.read("vital.alive")?[row] != 1 {
        return Err(fail(format!(
            "{} fired for row {row} with no pending death",
            timer.id()
        )));
    }
    v.write("vital.alive")?[row] = 0;
    v.write("vital.pending")?[row] = 0;
    let death = c.emit(
        EventDraft::new("DEATH")
            .subject(entity(row))
            .with_payload(vec![r64(row), timer.key.time.minutes()]),
    )?;
    c.submit(Command {
        kind: "Inherit",
        person: person(row),
        args: vec![rh_core::command::Arg::Int(r64(row))],
        cause: rh_events::EventRef::Draft(death),
    })
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "it must have the ResampleFn signature"
)]
fn vital_resample(
    _: &Stub,
    _: &TimerEntry,
    ctx: &ResampleCtx,
) -> Result<Option<SimTime>, SimError> {
    let offset = 1 + ctx.draw(0) % HORIZON;
    Ok(ctx.now().checked_add_minutes(offset.cast_signed()))
}

fn war_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let alive = v.read("vital.alive")?.to_vec();
    for (row, &a) in alive.iter().enumerate().rev() {
        if a == 1 && c.rng(&Wp::Levy, person(row), 0)? % 9 == 0 {
            c.submit(Command {
                kind: "Inherit",
                person: person(row),
                args: vec![rh_core::command::Arg::Int(r64(row))],
                cause: rh_events::EventRef::None,
            })?;
        }
    }
    Ok(())
}

fn estate_step(
    s: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let queue = c.take_commands();
    if !queue.windows(2).all(|w| w[0].key() < w[1].key()) {
        return Err(SimError::System {
            system: s.id,
            message: "the queue is not in (tick, phase, PersonId, seq) order".to_owned(),
        });
    }
    for q in queue {
        let row = row_of(q.command.int(0).unwrap());
        v.write("estate.heirs")?[row] += 1;
        let order = &mut v.write("estate.order")?[0];
        *order = order
            .wrapping_mul(31)
            .wrapping_add(q.command.person.to_raw().cast_signed());
        let mut e = EventDraft::new("INHERITANCE").subject(EntityRef::person(q.command.person));
        if let Some(cause) = q.cause {
            e = e.caused_by(cause);
        }
        c.emit(e)?;
    }
    Ok(())
}

fn rumor_step(
    s: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if !c.take_commands().is_empty() {
        return Err(SimError::System {
            system: s.id,
            message: "a non-owner received a command".to_owned(),
        });
    }
    let alive = v.read("vital.alive")?.to_vec();
    for (row, &a) in alive.iter().enumerate() {
        if a == 1 && row % 7 == usize::try_from(c.now().minutes().rem_euclid(7)).unwrap() {
            c.emit(EventDraft::new("RUMOR").subject(entity(row)))?;
        }
    }
    Ok(())
}

fn census_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let living: i64 = v.read("vital.alive")?.iter().sum();
    let census = c.emit(EventDraft::new("CENSUS").with_payload(vec![living]))?;
    c.emit(EventDraft::new("CENSUS_NOTE").after(census))?;
    Ok(())
}

fn life_stubs(observer: bool) -> Vec<Stub> {
    use rh_core::access::Lag::{SameInstant, StartOfDay};
    let mut stubs = vec![
        Stub::new("X1.vital")
            .purposes_of::<Vp>()
            .writes("vital.alive")
            .writes("vital.pending")
            .writes("vital.rate")
            .emits("DEATH")
            .emits("CURED")
            .sends("Inherit", "X2.estate")
            .at(Pulse::Month, Phase::Vital)
            .stepping(vital_step)
            .firing(vital_fire)
            .resampling(vital_resample),
        Stub::new("X2.estate")
            .writes("estate.heirs")
            .writes("estate.order")
            .emits("INHERITANCE")
            .at(Pulse::Day, Phase::Agenda)
            .applies("Inherit")
            .stepping(estate_step),
        Stub::new("X3.rumor")
            .reads("vital.alive", "X1.vital", SameInstant)
            .emits("RUMOR")
            .at(Pulse::Month, Phase::Agenda)
            .stepping(rumor_step),
        Stub::new("X4.census")
            .reads("vital.alive", "X1.vital", StartOfDay)
            .emits("CENSUS")
            .emits("CENSUS_NOTE")
            .at(Pulse::Month, Phase::Agenda)
            .stepping(census_step),
        Stub::new("X5.war")
            .purposes_of::<Wp>()
            .reads("vital.alive", "X1.vital", SameInstant)
            .sends("Inherit", "X2.estate")
            .at(Pulse::Month, Phase::Military)
            .stepping(war_step),
    ];
    if observer {
        stubs.push(Stub::new("X9.observer").at(Pulse::Hour, Phase::Bookkeeping));
    }
    stubs
}

fn life_engine(seed: u64, rows: usize, rate: i64, observer: bool, threads: usize) -> Engine {
    let stubs = life_stubs(observer);
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("vital.alive", vec![1; rows]);
    tables.insert("vital.pending", vec![0; rows]);
    tables.insert("vital.rate", vec![rate]);
    tables.insert("estate.heirs", vec![0; rows]);
    tables.insert("estate.order", vec![0]);
    let mut engine = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    engine.set_threads(threads);
    engine
}

/// Everything a run leaves: all events, the agenda's state, the tables, commands waiting. The
/// tables are compared whole: rh-core's tests stay hash-agnostic (SPR-007; the plane hash lives in
/// rh-snapshot).
#[derive(Debug, PartialEq, Eq)]
struct RunResult {
    events: Vec<Event>,
    agenda: AgendaState,
    tables: WorldTables,
    waiting: usize,
}

fn run(mut engine: Engine, years: i64) -> RunResult {
    let end = SimTime::from_minutes(years * CALENDAR.minutes_per_year() - 1);
    let mut events = Vec::new();
    while let Some(t) = engine.next_instant() {
        if t > end {
            break;
        }
        engine
            .advance_until(t, |_, _| {})
            .unwrap_or_else(|e| panic!("{e}"));
        events.extend(engine.drain_events());
    }
    RunResult {
        events,
        agenda: engine.agenda().state(),
        tables: engine.tables().clone(),
        waiting: engine.commands().len(),
    }
}

fn count(events: &[Event], kind: &str) -> usize {
    events.iter().filter(|e| e.kind == kind).count()
}

#[test]
fn ruling_1_thread_count_never_reaches_an_id() {
    let base = run(life_engine(7, 48, 4, false, 1), 3);
    for kind in [
        "DEATH",
        "CURED",
        "INHERITANCE",
        "RUMOR",
        "CENSUS",
        "CENSUS_NOTE",
    ] {
        assert!(count(&base.events, kind) > 3, "the world exercises {kind}");
    }
    // A death's inheritance cites it; a census note cites its census.
    let deaths: Vec<_> = base.events.iter().filter(|e| e.kind == "DEATH").collect();
    let cited = base
        .events
        .iter()
        .filter(|e| e.kind == "INHERITANCE" && !e.causes.is_empty())
        .count();
    assert_eq!(
        cited,
        deaths.len(),
        "every death passes its id through the command"
    );
    for e in base.events.iter().filter(|e| e.kind == "CENSUS_NOTE") {
        assert_eq!(e.causes, [rh_ids::EventId::from_raw(e.id.to_raw() - 1)]);
    }
    for threads in [2, 4, 8] {
        let other = run(life_engine(7, 48, 4, false, threads), 3);
        assert_eq!(other, base, "{threads} threads");
    }
}

#[test]
fn t02_timers_fire_at_their_own_tick_in_every_mode() {
    let headless = run(life_engine(11, 48, 3, false, 1), 2);
    let watched = run(life_engine(11, 48, 3, true, 4), 2);
    assert_eq!(
        watched.events, headless.events,
        "watched and headless runs agree"
    );
    let deaths: Vec<&Event> = headless
        .events
        .iter()
        .filter(|e| e.kind == "DEATH")
        .collect();
    assert!(deaths.len() > 10);
    for d in &deaths {
        assert_eq!(
            d.tick, d.payload[1],
            "a death is emitted at its timer's own tick"
        );
    }
    let off_boundary = deaths
        .iter()
        .filter(|d| !CALENDAR.is_boundary(Pulse::Hour, SimTime::from_minutes(d.tick)))
        .count();
    assert!(
        off_boundary * 2 > deaths.len(),
        "deaths fall between pulse boundaries, not at the month pulse"
    );
}

#[test]
fn v_det_12_no_back_dating_property() {
    for case in 0..16_u64 {
        let rows = 8 + usize::try_from(h64_words(TAG, &[case, 1]) % 40).unwrap();
        let rate = 2 + (h64_words(TAG, &[case, 2]) % 5).cast_signed();
        let threads = [1, 2, 4, 8][usize::try_from(case % 4).unwrap()];
        let mut engine = life_engine(case, rows, rate, case % 3 == 0, threads);
        let end = SimTime::from_minutes(CALENDAR.minutes_per_year());
        let mut last: Option<(i64, u8, u64)> = None;
        let mut seen = 0_usize;
        while let Some(t) = engine.next_instant() {
            if t > end {
                break;
            }
            engine
                .advance_until(t, |_, _| {})
                .unwrap_or_else(|e| panic!("{e}"));
            for e in engine.drain_events() {
                seen += 1;
                assert_eq!(
                    e.tick,
                    t.minutes(),
                    "case {case}: an event's tick is the current instant"
                );
                let key = e.order_key();
                assert!(
                    last.is_none_or(|l| l < key),
                    "case {case}: (tick, phase, seq) strictly increases"
                );
                assert!(
                    last.is_none_or(|l| l.2 < e.id.to_raw()),
                    "case {case}: EventIds strictly increase"
                );
                assert!(
                    e.causes.iter().all(|c| *c < e.id),
                    "case {case}: causes point back"
                );
                last = Some(key);
            }
            // Sampled outcomes sit in the agenda as future timers until they fire.
            let pending = engine.tables().get("vital.pending").unwrap();
            for (row, &p) in pending.iter().enumerate() {
                let timers = engine.agenda().pending_for(entity(row));
                assert_eq!(
                    timers.len(),
                    usize::try_from(p).unwrap(),
                    "case {case} row {row}"
                );
            }
            assert!(
                engine.agenda().peek().is_none_or(|k| k.time > t),
                "case {case}: no timer is left due"
            );
        }
        assert!(seen > 20, "case {case} emits events ({seen})");
    }
}

#[test]
fn t02_material_input_resamples_only_its_scope_keyed_by_epoch() {
    let mid = SimTime::from_minutes(40 * DAY + 7);
    let mut engine = life_engine(5, 48, 2, false, 2);
    engine.advance_until(mid, |_, _| {}).unwrap();
    let before = engine.agenda().pending();
    assert!(before.len() >= 6, "timers are pending ({})", before.len());
    // The scope: the entities of the first two pending timers, and one at an exact due tick.
    let due_at = before[0].0.time; // the earliest pending timer: the input lands on its tick
    let scope_rows: Vec<EntityRef> = vec![before[0].0.entity, before[3].0.entity];
    let input = MaterialInput {
        scope: scope_rows.iter().copied().collect(),
    };
    assert!(
        matches!(
            engine.submit_input(SimTime::from_minutes(5), input.clone()),
            Err(InputError::PastInput { at, next: Some(next) }) if at == SimTime::from_minutes(5) && next > at
        ),
        "acting in the past is refused here (it forks)"
    );
    let mut twin = life_engine(5, 48, 2, false, 2);
    twin.advance_until(mid, |_, _| {}).unwrap();
    engine.submit_input(due_at, input.clone()).unwrap();
    engine.advance_until(due_at, |_, _| {}).unwrap();
    twin.advance_until(due_at, |_, _| {}).unwrap();

    for e in &scope_rows {
        assert_eq!(engine.epoch(*e), 1);
    }
    let after = engine.agenda().pending();
    let fired_now: Vec<_> = engine
        .events()
        .events()
        .iter()
        .filter(|e| e.tick == due_at.minutes() && e.kind == "DEATH")
        .map(|e| e.subjects[0])
        .collect();
    assert!(
        !fired_now.contains(&before[0].0.entity),
        "the input applies before the timer due at its own tick (ADR-0025 item 4)"
    );
    for (key, timer) in &before {
        let inside = input.scope.contains(&key.entity);
        let still = after.iter().any(|(k, _)| k == key);
        if inside {
            assert!(!still, "{key:?} is in scope: cancelled and re-drawn");
            let redrawn: Vec<_> = after
                .iter()
                .filter(|(k, t)| k.entity == key.entity && t == timer)
                .collect();
            assert_eq!(redrawn.len(), 1, "one re-drawn timer for {key:?}");
            assert!(redrawn[0].0.time > due_at && redrawn[0].0.seq > key.seq);
        } else if key.time > due_at {
            assert!(still, "{key:?} is outside the scope: untouched");
        }
    }
    // The twin, without the input, keeps the scope's original timers.
    assert!(
        twin.agenda()
            .pending()
            .iter()
            .any(|(k, _)| *k == before[3].0)
    );

    // A second input on the same entity: epoch 2, a different draw.
    let one = scope_rows[1];
    let first_redraw = engine.agenda().pending_for(one)[0].0.time;
    let next = due_at.checked_add_minutes(1).unwrap();
    engine
        .submit_input(
            next,
            MaterialInput {
                scope: [one].into_iter().collect(),
            },
        )
        .unwrap();
    engine.advance_until(next, |_, _| {}).unwrap();
    assert_eq!(engine.epoch(one), 2);
    let second_redraw = engine.agenda().pending_for(one)[0].0.time;
    assert_ne!(first_redraw, second_redraw, "the epoch keys the re-draw");
}

#[test]
fn material_inputs_replay_identically_at_any_thread_count() {
    let go = |threads| {
        let mut e = life_engine(9, 48, 2, false, threads);
        let at = SimTime::from_minutes(45 * DAY + 3);
        let scope = (0..48).step_by(5).map(entity).collect();
        e.submit_input(at, MaterialInput { scope }).unwrap();
        run(e, 1)
    };
    let one = go(1);
    for threads in [2, 4, 8] {
        assert_eq!(go(threads), one);
    }
}

// --- Refusals -----------------------------------------------------------------------------------

enum Np {
    N,
}

impl SpecPurpose for Np {
    const ALL: &'static [Self] = &[Self::N];
    fn name(&self) -> &'static str {
        "n"
    }
}

fn run_one(stubs: Vec<Stub>) -> Result<(), SimError> {
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("y.v", vec![0]);
    let mut e = Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    e.advance_until(SimTime::from_minutes(3 * DAY), |_, _| {})
}

fn spec_at(time: SimTime, phase: Phase) -> TimerSpec {
    TimerSpec {
        time,
        phase,
        entity: EntityRef::WORLD,
        kind: 0,
        payload: 0,
        cause: rh_events::EventRef::None,
    }
}

fn schedule_now(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    c.schedule(&Np::N, spec_at(c.now(), c.phase()))
}
fn schedule_earlier_phase(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    c.schedule(&Np::N, spec_at(c.now(), Phase::Inputs))
}
fn schedule_unbound(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    c.schedule(
        &Np::N,
        spec_at(c.now().checked_add_minutes(1).unwrap(), Phase::Culture),
    )
}
fn submit_undeclared(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    c.submit(Command {
        kind: "Smite",
        person: PersonId::from_raw(1),
        args: vec![],
        cause: rh_events::EventRef::None,
    })
}
fn emit_undeclared(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    c.emit(EventDraft::new("OMEN")).map(|_| ())
}
fn schedule_soon(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now() == SimTime::EPOCH {
        c.schedule(
            &Np::N,
            spec_at(SimTime::from_minutes(2 * DAY + 5), Phase::Vital),
        )?;
    }
    Ok(())
}
fn cancel_theirs(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now() == SimTime::from_minutes(DAY) {
        c.cancel(rh_core::agenda::TimerId::from_raw(0))?;
    }
    Ok(())
}

#[test]
fn t02_back_dated_and_unbound_timers_are_refused() {
    for (step, needle) in [
        (schedule_now as support::StepFn, "no back-dating"),
        (schedule_earlier_phase, "no back-dating"),
        (schedule_unbound, "would never fire"),
    ] {
        let err = run_one(vec![
            Stub::new("Y1.t")
                .purposes_of::<Np>()
                .at(Pulse::Day, Phase::Vital)
                .stepping(step),
        ])
        .unwrap_err();
        assert!(
            matches!(&err, SimError::Timer { message, .. } if message.contains(needle)),
            "{err}"
        );
    }
}

#[test]
fn t02_only_the_owner_cancels_a_timer() {
    let err = run_one(vec![
        Stub::new("Y1.owner")
            .purposes_of::<Np>()
            .at(Pulse::Day, Phase::Vital)
            .stepping(schedule_soon)
            .firing(|_, _, _, _| Ok(())),
        Stub::new("Y2.other")
            .at(Pulse::Day, Phase::Agenda)
            .stepping(cancel_theirs),
    ])
    .unwrap_err();
    assert!(
        matches!(&err, SimError::Timer { system: "Y2.other", message } if message.contains("another system")),
        "{err}"
    );
}

#[test]
fn t03_t04_undeclared_commands_and_events_are_refused_at_run_time() {
    let err = run_one(vec![
        Stub::new("Y1.s")
            .at(Pulse::Day, Phase::Vital)
            .stepping(submit_undeclared),
    ])
    .unwrap_err();
    assert_eq!(
        err,
        SimError::UndeclaredCommand {
            system: "Y1.s",
            command: "Smite"
        }
    );
    let err = run_one(vec![
        Stub::new("Y1.e")
            .at(Pulse::Day, Phase::Vital)
            .stepping(emit_undeclared),
    ])
    .unwrap_err();
    assert_eq!(
        err,
        SimError::UndeclaredEvent {
            system: "Y1.e",
            kind: "OMEN"
        }
    );
}

fn refused(stubs: Vec<Stub>, specs_from: &[Stub]) -> RegistrationError {
    let specs = agreeing_specs(specs_from);
    match register(stubs.into_iter().map(Stub::boxed).collect(), &specs) {
        Ok(_) => panic!("registration accepted"),
        Err(e) => e,
    }
}

fn messages(e: &RegistrationError, rule: &str) -> Vec<String> {
    e.violations
        .iter()
        .filter(|v| v.rule == rule)
        .map(|v| format!("{}: {}", v.system, v.message))
        .collect()
}

#[test]
fn t03_a_non_owner_is_refused_at_registration() {
    let sender = Stub::new("Z1.sender").sends("Inherit", "Z2.estate");
    let owner = Stub::new("Z2.estate")
        .at(Pulse::Day, Phase::Agenda)
        .applies("Inherit");
    let rogue = Stub::new("Z3.rogue")
        .at(Pulse::Day, Phase::Agenda)
        .applies("Inherit");
    // A rogue applier, with the sender's spec naming the estate as owner.
    let e = refused(
        vec![sender.clone(), rogue.clone()],
        &[sender.clone(), rogue.clone()],
    );
    assert_eq!(
        messages(&e, "command"),
        [
            "Z3.rogue: applies command Inherit, but Z1.sender's spec names Z2.estate as its owner; a non-owner never applies a command",
            "Z1.sender: submits command Inherit to Z2.estate, but Z3.rogue applies it",
        ]
    );
    // Two appliers.
    let e = refused(
        vec![sender.clone(), owner.clone(), rogue.clone()],
        &[sender.clone(), owner.clone(), rogue],
    );
    assert!(
        messages(&e, "command").contains(
            &"Z3.rogue: applies command Inherit, which Z2.estate also applies (05.1 §2 rule 2)"
                .to_owned()
        )
    );
    // A registered target that does not apply the command.
    let lazy = Stub::new("Z2.estate");
    let e = refused(vec![sender.clone(), lazy.clone()], &[sender.clone(), lazy]);
    assert_eq!(
        messages(&e, "command"),
        [
            "Z1.sender: submits command Inherit to Z2.estate, which does not declare that it applies it"
        ]
    );
    // A submission the spec does not declare.
    let spec_only = Stub::new("Z1.sender");
    let e = refused(
        vec![sender.clone(), owner.clone()],
        &[spec_only, owner.clone()],
    );
    assert_eq!(
        messages(&e, "command"),
        [
            "Z1.sender: submits command Inherit to Z2.estate, which its spec's [[commands]] table does not declare"
        ]
    );
    // The owner unregistered: reported, not refused (SPR-006 G5 withdrawn for sends; 05.1 §2
    // rule 2, absent owners): each submission is discarded and counted (SPR-014 T-04).
    let r = register(
        vec![sender.clone().boxed()],
        &agreeing_specs(std::slice::from_ref(&sender)),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        r.reports(),
        [
            "Z1.sender: submits command Inherit to Z2.estate, which is not registered: each submission is discarded and counted under dropped{Z1.sender, Inherit, absent_owner} (05.1 §2 rule 2)"
        ]
    );
}

#[test]
fn t04_one_emitter_per_event_kind_declared_in_its_spec() {
    let a = Stub::new("Z1.a").emits("OMEN");
    let b = Stub::new("Z2.b").emits("OMEN");
    let e = refused(vec![a.clone(), b.clone()], &[a.clone(), b]);
    assert_eq!(
        messages(&e, "event-kind"),
        ["Z2.b: emits OMEN, which Z1.a also emits"]
    );
    let e = refused(vec![a], &[Stub::new("Z1.a")]);
    assert_eq!(
        messages(&e, "event-kind"),
        ["Z1.a: emits OMEN, which its spec's [[emits]] table does not declare"]
    );
}

// --- The life world's event log: a golden, and V-DET-12 checked on it ------------------------

/// The committed event log of the life world (seed 7, 48 persons, two years, one thread).
const LIFE_GOLDEN: &str = "tests/golden/life_world_events.txt";

/// One event as a golden line: `id tick phase emitter kind s=[subjects] c=[causes] p=[payload]`.
fn event_line(e: &Event) -> String {
    let list = |v: Vec<String>| v.join(",");
    format!(
        "{} {} {} {} {} s=[{}] c=[{}] p=[{}]",
        e.id.to_raw(),
        e.tick,
        e.phase,
        e.emitter,
        e.kind,
        list(e.subjects.iter().map(ToString::to_string).collect()),
        list(e.causes.iter().map(|c| c.to_raw().to_string()).collect()),
        list(e.payload.iter().map(ToString::to_string).collect()),
    )
}

/// V-DET-12 on a log of golden lines: ids strictly increase, `(tick, phase)` never decreases (so the
/// id order is the emission order `(tick, phase, seq)`), and every cause is an earlier id.
fn check_v_det_12(lines: &[&str]) -> Result<(), String> {
    let mut last: Option<(u64, i64, u8)> = None;
    for (n, line) in lines.iter().enumerate() {
        let f: Vec<&str> = line.split(' ').collect();
        let bad = |why: &str| format!("line {}: {why}: {line}", n + 1);
        if f.len() != 8 {
            return Err(bad("not an event line"));
        }
        let id: u64 = f[0].parse().map_err(|_| bad("bad id"))?;
        let tick: i64 = f[1].parse().map_err(|_| bad("bad tick"))?;
        let phase: u8 = f[2].parse().map_err(|_| bad("bad phase"))?;
        if let Some((lid, ltick, lphase)) = last {
            if id <= lid {
                return Err(bad("EventIds do not strictly increase"));
            }
            if (tick, phase) < (ltick, lphase) {
                return Err(bad("an event is back-dated before the previous one"));
            }
        }
        let causes = f[6]
            .strip_prefix("c=[")
            .and_then(|c| c.strip_suffix(']'))
            .ok_or_else(|| bad("bad causes"))?;
        for c in causes.split(',').filter(|c| !c.is_empty()) {
            let c: u64 = c.parse().map_err(|_| bad("bad cause"))?;
            if c >= id {
                return Err(bad("a causal edge points forward"));
            }
        }
        last = Some((id, tick, phase));
    }
    Ok(())
}

#[test]
// LINT-ALLOW(forbidden-api): a test reads an environment override for its golden file; tests never link into the engine (SPR-011 A-4)
#[allow(
    clippy::disallowed_methods,
    reason = "a test reads an environment override for its golden file"
)]
fn v_det_12_golden_event_log() {
    let events = run(life_engine(7, 48, 4, false, 1), 2).events;
    let mut text = String::from(
        "# GENERATED by `RH_CORE_WRITE_LIFE_GOLDEN=1 cargo test -p rh-core --test agenda_events`;\n\
         # never edited by hand. The life world's event log (seed 7, 48 persons, two years): event\n\
         # order is part of the hash contract (SPR-006), so a change here is a C3 change.\n\
         # Format: <EventId> <tick> <phase index> <emitter> <kind> s=[subjects] c=[causes] p=[payload]\n",
    );
    for e in &events {
        text.push_str(&event_line(e));
        text.push('\n');
    }
    let default = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(LIFE_GOLDEN);
    let path = std::env::var_os("RH_CORE_LIFE_GOLDEN").map_or(default.clone(), PathBuf::from);
    if std::env::var_os("RH_CORE_WRITE_LIFE_GOLDEN").is_some() {
        std::fs::write(&default, &text).unwrap();
    }
    let committed =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lines: Vec<&str> = committed.lines().filter(|l| !l.starts_with('#')).collect();
    if let Err(why) = check_v_det_12(&lines) {
        panic!("V-DET-12 violated in {}: {why}", path.display());
    }
    let ours: Vec<&str> = text.lines().filter(|l| !l.starts_with('#')).collect();
    let first = lines.iter().zip(&ours).position(|(a, b)| a != b);
    assert_eq!(
        first,
        None,
        "{} differs at event {first:?}: a C3 change of event order",
        path.display()
    );
    assert_eq!(lines.len(), ours.len());
    println!(
        "V-DET-12: golden event log of {} events checked; 16 random worlds in v_det_12_no_back_dating_property",
        lines.len()
    );
}

/// A system that schedules a timer and keeps the trait's default `fire` (D7).
struct NoFire;

impl rh_core::system::SystemModel for NoFire {
    fn descriptor(&self) -> rh_core::system::SystemDescriptor {
        rh_core::system::SystemDescriptor {
            id: "Y9.nofire",
            fidelity: "F1",
            spec_version: "0.0.0",
        }
    }
    fn access(&self) -> rh_core::access::AccessSpec {
        rh_core::access::AccessSpec::default()
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        vec![PulseBinding {
            pulse: Pulse::Day,
            phase: Phase::Vital,
        }]
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        Np::names()
    }
    fn params_schema(&self) -> rh_core::system::ParamSchema {
        rh_core::system::ParamSchema::default()
    }
    fn step(
        &self,
        _: PulseBinding,
        _: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        if c.now() == SimTime::EPOCH {
            c.schedule(
                &Np::N,
                spec_at(SimTime::from_minutes(DAY / 2), Phase::Vital),
            )?;
        }
        Ok(())
    }
    fn invariants(&self, _: &WorldTables) -> Vec<rh_core::system::InvariantResult> {
        Vec::new()
    }
    fn declared_aggregates(&self) -> Vec<rh_core::system::AggregateSpec> {
        Vec::new()
    }
}

#[test]
fn d7_the_default_fire_is_a_sim_error_that_poisons_the_engine() {
    let stub = Stub::new("Y9.nofire")
        .purposes_of::<Np>()
        .at(Pulse::Day, Phase::Vital);
    let specs = agreeing_specs(&[stub]);
    let registered = register(vec![Box::new(NoFire)], &specs).unwrap_or_else(|e| panic!("{e}"));
    let mut e = Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        registered,
        WorldTables::new(),
        SimTime::EPOCH,
    );
    let err = e
        .advance_until(SimTime::from_minutes(3 * DAY), |_, _| {})
        .unwrap_err();
    assert!(
        matches!(&err, SimError::System { system: "Y9.nofire", message } if message.contains("does not implement SystemModel::fire")),
        "{err}"
    );
    assert_eq!(e.poisoned(), Some(&err));
    let a = e.crash_artifact().unwrap();
    assert_eq!((a.tick, a.phase.as_str()), (DAY / 2, "Vital"));
}

fn schedule_twins(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now() == SimTime::EPOCH {
        for kind in [rh_ids::EntityKind::Person, rh_ids::EntityKind::Household] {
            c.schedule(
                &Np::N,
                TimerSpec {
                    time: SimTime::from_minutes(300 * DAY),
                    phase: Phase::Vital,
                    entity: EntityRef::new(kind, 5),
                    kind: 0,
                    payload: 0,
                    cause: rh_events::EventRef::None,
                },
            )?;
        }
    }
    Ok(())
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "it must have the ResampleFn signature"
)]
fn redraw_far(_: &Stub, _: &TimerEntry, ctx: &ResampleCtx) -> Result<Option<SimTime>, SimError> {
    Ok(ctx
        .now()
        .checked_add_minutes(1 + (ctx.draw(0) % (100 * 1_440)).cast_signed()))
}

#[test]
fn g3_resample_draws_key_on_the_entity_kind_too() {
    let stubs = vec![
        Stub::new("Y1.twins")
            .purposes_of::<Np>()
            .at(Pulse::Day, Phase::Vital)
            .stepping(schedule_twins)
            .resampling(redraw_far),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap();
    let mut e = Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        registered,
        WorldTables::new(),
        SimTime::EPOCH,
    );
    let person = EntityRef::new(rh_ids::EntityKind::Person, 5);
    let household = EntityRef::new(rh_ids::EntityKind::Household, 5);
    let at = SimTime::from_minutes(DAY + 3);
    e.submit_input(
        at,
        MaterialInput {
            scope: [person, household].into_iter().collect(),
        },
    )
    .unwrap();
    e.advance_until(at, |_, _| {}).unwrap();
    assert_eq!((e.epoch(person), e.epoch(household)), (1, 1));
    let p = e.agenda().pending_for(person)[0].0.time;
    let h = e.agenda().pending_for(household)[0].0.time;
    assert_ne!(
        p, h,
        "same purpose, raw id, epoch and instant, different kinds: different draws"
    );
    assert_eq!(
        rh_core::system::resample_entity_id(person),
        h64_words(rh_core::system::TAG_RESAMPLE_ENTITY, &[1, 5]),
        "the entity_id layout"
    );
}

#[test]
fn g4_the_event_hash_does_not_depend_on_the_drain_cadence() {
    let month = CALENDAR.minutes_per_year() / 12;
    let end = SimTime::from_minutes(2 * CALENDAR.minutes_per_year() - 1);
    let hash_with = |drain_every: Option<i64>| -> u64 {
        let mut e = life_engine(7, 48, 4, false, 2);
        if let Some(step) = drain_every {
            let mut t = step - 1;
            while t < end.minutes() {
                e.advance_until(SimTime::from_minutes(t), |_, _| {})
                    .unwrap();
                e.drain_events();
                t += step;
            }
        }
        e.advance_until(end, |_, _| {}).unwrap();
        e.events().hash()
    };
    let never = hash_with(None);
    assert_eq!(hash_with(Some(month)), never, "drained monthly");
    assert_eq!(hash_with(Some(12 * month)), never, "drained yearly");
}

/// G1/G2: two timers of one owner and one of another, all due at the same (t, Vital).
const SLOT: i64 = DAY + 600;

fn schedule_slot(
    s: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now() == SimTime::EPOCH {
        let n = if s.id == "Y1.a" { 2 } else { 1 };
        for k in 0..n {
            c.schedule(
                &Np::N,
                TimerSpec {
                    time: SimTime::from_minutes(SLOT),
                    phase: Phase::Vital,
                    entity: EntityRef::new(rh_ids::EntityKind::Person, 7),
                    kind: k,
                    payload: 0,
                    cause: rh_events::EventRef::None,
                },
            )?;
        }
    }
    Ok(())
}

/// Emits one event per timer, recording how many of its own timers it still sees pending for the
/// entity, and tries to cancel each: a sibling due in the same slot is already out of the agenda.
fn fire_slot(
    _: &Stub,
    timer: &TimerEntry,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let visible = c.own_timers(timer.key.entity);
    for t in &visible {
        c.cancel(t.id())?;
    }
    c.emit(
        EventDraft::new(
            if timer.timer.owner == rh_core::register::system_id("Y1.a") {
                "A_FIRED"
            } else {
                "B_FIRED"
            },
        )
        .with_payload(vec![
            i64::from(timer.timer.kind),
            i64::try_from(visible.len()).unwrap(),
        ]),
    )
    .map(|_| ())
}

#[test]
fn g1_g2_timers_in_one_slot_fire_together_in_canonical_then_key_order() {
    for threads in [1, 4] {
        let stubs = vec![
            Stub::new("Y1.a")
                .purposes_of::<Np>()
                .emits("A_FIRED")
                .at(Pulse::Day, Phase::Vital)
                .stepping(schedule_slot)
                .firing(fire_slot),
            Stub::new("Y2.b")
                .purposes_of::<Np>()
                .emits("B_FIRED")
                .at(Pulse::Day, Phase::Vital)
                .stepping(schedule_slot)
                .firing(fire_slot),
        ];
        let specs = agreeing_specs(&stubs);
        let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap();
        let mut e = Engine::new(
            CALENDAR,
            1,
            BranchSalt::from_raw(0),
            registered,
            WorldTables::new(),
            SimTime::EPOCH,
        );
        e.set_threads(threads);
        e.advance_until(SimTime::from_minutes(2 * DAY), |_, _| {})
            .unwrap();
        let fired: Vec<(i64, u8, &str, Vec<i64>)> = e
            .events()
            .events()
            .iter()
            .map(|ev| (ev.tick, ev.phase, ev.kind, ev.payload.clone()))
            .collect();
        let vital = Phase::Vital.index();
        assert_eq!(
            fired,
            [
                // Owner A first (canonical order), its timers in key order; none sees a sibling.
                (SLOT, vital, "A_FIRED", vec![0, 0]),
                (SLOT, vital, "A_FIRED", vec![1, 0]),
                (SLOT, vital, "B_FIRED", vec![0, 0]),
            ],
            "{threads} threads"
        );
    }
}

/// DEF-3: the re-drawn time a material input produces, computed independently of `ResampleCtx`
/// from `rh_ids`' keyed hash with the full layout of `05.1` §6 item 2:
/// `H(world_seed, branch_salt, system_id, epoch, purpose, entity_id, t_input, k = 0)`, where
/// `system_id` is the high 32 bits of `h64_bytes(tag("rehoboam/rh-core/system/v1"), id)` and the
/// re-sample `entity_id` is `h64_words(tag("rehoboam/rh-core/resample-entity/v1"), [kind, raw])`.
fn expected_redraw(seed: u64, entity: EntityRef, epoch: u32, t_input: i64) -> i64 {
    let system = rh_ids::rng::h64_bytes(tag("rehoboam/rh-core/system/v1"), b"Y1.twins") >> 32;
    let key = rh_ids::RngKey {
        world_seed: seed,
        branch_salt: BranchSalt::from_raw(0),
        system: rh_ids::SystemId::from_raw(u32::try_from(system).unwrap()),
        purpose: rh_ids::PurposeId::of("n"),
        entity: h64_words(
            tag("rehoboam/rh-core/resample-entity/v1"),
            &[u64::from(entity.kind().tag()), entity.raw()],
        ),
        epoch,
        t: t_input,
    };
    // `redraw_far`: one minute after the input, plus the draw modulo 100 days.
    t_input + 1 + (rh_ids::h(&key, 0) % (100 * 1_440)).cast_signed()
}

#[test]
fn def3_redraws_equal_the_independent_epoch_keyed_hash() {
    let stubs = vec![
        Stub::new("Y1.twins")
            .purposes_of::<Np>()
            .at(Pulse::Day, Phase::Vital)
            .stepping(schedule_twins)
            .resampling(redraw_far),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap();
    let seed = 0x5eed;
    let mut e = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered,
        WorldTables::new(),
        SimTime::EPOCH,
    );
    let twins = [
        EntityRef::new(rh_ids::EntityKind::Person, 5),
        EntityRef::new(rh_ids::EntityKind::Household, 5),
    ];
    for (epoch, at) in [(1_u32, DAY + 3), (2, 2 * DAY + 11)] {
        let t = SimTime::from_minutes(at);
        e.submit_input(
            t,
            MaterialInput {
                scope: twins.into_iter().collect(),
            },
        )
        .unwrap();
        e.advance_until(t, |_, _| {}).unwrap();
        for entity in twins {
            assert_eq!(e.epoch(entity), epoch);
            let pending = e.agenda().pending_for(entity);
            assert_eq!(pending.len(), 1);
            assert_eq!(
                pending[0].0.time.minutes(),
                expected_redraw(seed, entity, epoch, at),
                "{entity} at epoch {epoch}"
            );
        }
    }
}

/// DEF-5: a timer due at a Day boundary in Vital, whose `fire` writes `t.v`, and a reader bound
/// to (Day, Vital) that reads `t.v` at `same_instant`. D1 (the timer pass runs before the binding
/// pass) means the reader sees the write of the same instant.
const D1_TICK: i64 = 3 * DAY;

fn d1_schedule(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now() == SimTime::EPOCH {
        c.schedule(
            &Np::N,
            spec_at(SimTime::from_minutes(D1_TICK), Phase::Vital),
        )?;
    }
    Ok(())
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "it must have the FireFn signature"
)]
fn d1_fire(
    _: &Stub,
    _: &TimerEntry,
    v: &mut SystemView<'_>,
    _: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    v.write("t.v")?[0] = 1;
    Ok(())
}

fn d1_read(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now().minutes() == D1_TICK {
        let seen = v.read("t.v")?[0];
        v.write("r.seen")?[0] = seen;
    }
    Ok(())
}

#[test]
fn def5_a_timer_fires_before_any_binding_of_its_phase() {
    use rh_core::access::Lag::SameInstant;
    for threads in [1, 4] {
        let stubs = vec![
            Stub::new("D1.timer")
                .purposes_of::<Np>()
                .writes("t.v")
                .at(Pulse::Day, Phase::Vital)
                .stepping(d1_schedule)
                .firing(d1_fire),
            Stub::new("D2.reader")
                .reads("t.v", "D1.timer", SameInstant)
                .writes("r.seen")
                .at(Pulse::Day, Phase::Vital)
                .stepping(d1_read),
        ];
        let specs = agreeing_specs(&stubs);
        let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap();
        let mut tables = WorldTables::new();
        tables.insert("t.v", vec![0]);
        tables.insert("r.seen", vec![-1]);
        let mut e = Engine::new(
            CALENDAR,
            1,
            BranchSalt::from_raw(0),
            registered,
            tables,
            SimTime::EPOCH,
        );
        e.set_threads(threads);
        e.advance_until(SimTime::from_minutes(D1_TICK), |_, _| {})
            .unwrap();
        assert_eq!(
            e.tables().get("r.seen").unwrap()[0],
            1,
            "{threads} threads: the binding at ({D1_TICK}, Vital) sees the timer's write (D1)"
        );
    }
}

// --- Engine state: snapshot and restore (SPR-007 T-02) ----------------------------------------

fn life_registered() -> rh_core::register::Registered {
    let stubs = life_stubs(false);
    let specs = agreeing_specs(&stubs);
    register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap_or_else(|e| panic!("{e}"))
}

fn restore(state: rh_core::state::EngineState) -> Result<Engine, rh_core::state::RestoreError> {
    Engine::restore(
        CALENDAR,
        13,
        BranchSalt::from_raw(0),
        life_registered(),
        rh_core::crash::Provenance::default(),
        state,
    )
}

/// A life world with inputs queued for later, run to `end` without draining its events.
fn life_run_to(end: SimTime) -> Engine {
    let mut e = life_engine(13, 48, 3, false, 1);
    for (day, step) in [(45, 5), (400, 7), (600, 3)] {
        let scope = (0..48).step_by(step).map(entity).collect();
        e.submit_input(
            SimTime::from_minutes(day * DAY + 3),
            MaterialInput { scope },
        )
        .unwrap();
    }
    e.advance_until(end, |_, _| {})
        .unwrap_or_else(|e| panic!("{e}"));
    e
}

/// The split point: day 50, when timers are pending, events held and two inputs still queued.
const SPLIT: SimTime = SimTime::from_minutes(50 * DAY + 1);
/// The end of both runs: two years.
const TWO_YEARS: SimTime = SimTime::from_minutes(2 * 365 * DAY - 1);

#[test]
fn t02_restore_of_a_snapshot_continues_exactly_like_the_unbroken_run() {
    let unbroken = life_run_to(TWO_YEARS).snapshot();
    let at_one = life_run_to(SPLIT).snapshot();
    // The snapshot carries every kind of engine-held state.
    assert!(!at_one.agenda.entries.is_empty(), "pending timers");
    assert!(!at_one.events.events.is_empty(), "undrained events");
    assert!(!at_one.epochs.is_empty(), "epochs");
    assert!(!at_one.inputs.is_empty(), "inputs queued for later");
    assert!(at_one.accepted_inputs > 0);
    let mut restored = restore(at_one.clone()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(restored.snapshot(), at_one, "restore(x).snapshot() == x");
    restored
        .advance_until(TWO_YEARS, |_, _| {})
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(restored.snapshot(), unbroken);
}

#[test]
fn t02_restore_refuses_every_non_canonical_or_unknown_part() {
    use rh_core::state::{EngineState, RestoreError as R};
    type Edit = fn(&mut EngineState);
    type Want = fn(&R) -> bool;
    let base = life_run_to(SPLIT).snapshot();
    let cases: [(&str, Edit, Want); 13] = [
        (
            "unsorted agenda",
            |s| s.agenda.entries.swap(0, 1),
            |e| matches!(e, R::Agenda(_)),
        ),
        (
            "unknown timer owner",
            |s| s.agenda.entries[0].1.owner = rh_ids::SystemId::from_raw(7),
            |e| matches!(e, R::TimerOwner(_)),
        ),
        (
            "unsorted epochs",
            |s| s.epochs.reverse(),
            |e| matches!(e, R::Epochs(_)),
        ),
        (
            "epoch 0",
            |s| s.epochs[0].1 = 0,
            |e| matches!(e, R::Epochs(_)),
        ),
        (
            "empty input list",
            |s| s.inputs[0].1.clear(),
            |e| matches!(e, R::Inputs(_)),
        ),
        (
            "input before the cursor",
            |s| s.inputs[0].0 = SimTime::EPOCH,
            |e| matches!(e, R::Inputs(_)),
        ),
        (
            "event counter",
            |s| s.events.next_seq += 1,
            |e| matches!(e, R::Events(_)),
        ),
        (
            "undeclared event kind",
            |s| s.events.events[0].kind = "WEDDING",
            |e| matches!(e, R::Events(_)),
        ),
        (
            "unsorted events",
            |s| s.events.events.swap(0, 1),
            |e| matches!(e, R::Events(_)),
        ),
        (
            "missing columns",
            |s| s.tables = WorldTables::new(),
            |e| matches!(e, R::Tables(_)),
        ),
        (
            "unsorted inputs",
            |s| s.inputs.reverse(),
            |e| matches!(e, R::Inputs(_)),
        ),
        (
            // GAP-11: more inputs queued than the engine ever accepted.
            "queued inputs beyond the accepted count",
            |s| s.accepted_inputs = 0,
            |e| matches!(e, R::Inputs(_)),
        ),
        (
            // GAP-9: the earliest pending timer moved to just before the cursor.
            "timer before the cursor",
            |s| {
                let cursor = s.cursor.unwrap();
                s.agenda.entries[0].0.time = SimTime::from_minutes(cursor.minutes() - 1);
            },
            |e| matches!(e, R::TimerTime(_)),
        ),
    ];
    for (name, edit, want) in cases {
        let mut s = base.clone();
        edit(&mut s);
        match restore(s) {
            Err(e) => assert!(want(&e), "{name}: refused for another reason: {e}"),
            Ok(_) => panic!("{name}: accepted"),
        }
    }
    // The other side of GAP-9's boundary. A keyframe is taken between advances, so no phase of
    // the cursor instant has run: a timer at the cursor is pending in every phase, and restores.
    let cursor = base.cursor.unwrap();
    assert!(
        base.agenda.entries[1].0.time > cursor,
        "the second timer is later"
    );
    for phase in rh_core::Phase::ALL {
        let mut at_cursor = base.clone();
        at_cursor.agenda.entries[0].0.time = cursor;
        at_cursor.agenda.entries[0].0.phase = phase;
        assert!(at_cursor.agenda.entries.windows(2).all(|w| w[0].0 < w[1].0));
        restore(at_cursor).unwrap_or_else(|e| panic!("a timer at the cursor in {phase:?}: {e}"));
    }
    // A far-future timer is legal too.
    let mut far = base.clone();
    let last = far.agenda.entries.len() - 1;
    far.agenda.entries[last].0.time = SimTime::from_minutes(cursor.minutes() + 1_000 * 525_600);
    restore(far).unwrap_or_else(|e| panic!("a far-future timer: {e}"));
}
