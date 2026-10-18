//! SPR-006's "life world" (`crates/rh-core/tests/agenda_events.rs`), copied for rh-snapshot's
//! keyframe tests (SPR-007 T-02, V-DET-05): death timers, cures, inheritance commands, rumours and
//! censuses, so pending timers, queued commands, epochs and held events all exist. rh-core's tests
//! cannot use rh-snapshot (it depends on rh-core), so the world lives here as a copy, as
//! `../support/` is of rh-core's support.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken world must fail the test loudly"
)]

use rh_core::agenda::TimerEntry;
use rh_core::command::Command;
use rh_core::engine::Engine;
use rh_core::register::register;
use rh_core::system::{PulseBinding, ResampleCtx, SimError, SpecPurpose, StepCtx, TimerSpec};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{EdgeKind, EventDraft, EventRef, RoleId};
use rh_ids::rng::{h64_words, tag};
use rh_ids::{BranchSalt, EntityRef, PersonId};

use crate::support::{Stub, agreeing_specs};

pub const CALENDAR: Calendar = Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four);
pub const DAY: i64 = 1_440;
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

pub fn person(row: usize) -> PersonId {
    PersonId::from_raw(h64_words(TAG, &[row as u64]) >> 4)
}

pub fn entity(row: usize) -> EntityRef {
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
            c.emit(EventDraft::new("CURED").role(RoleId::TARGET, entity(row)))?;
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
            .role(RoleId::TARGET, entity(row))
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
        let mut e = EventDraft::new("INHERITANCE")
            .role(RoleId::TARGET, EntityRef::person(q.command.person));
        if let Some(cause) = q.cause {
            e = e.caused_by(EventRef::Known(cause), EdgeKind::Triggered);
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
            c.emit(EventDraft::new("RUMOR").role(RoleId::TARGET, entity(row)))?;
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

pub fn life_stubs(observer: bool) -> Vec<Stub> {
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

pub fn life_engine(seed: u64, rows: usize, rate: i64, observer: bool, threads: usize) -> Engine {
    life_engine_with(Vec::new(), seed, rows, rate, observer, threads)
}

/// The life world with `extra` systems registered beside its own.
pub fn life_engine_with(
    extra: Vec<Stub>,
    seed: u64,
    rows: usize,
    rate: i64,
    observer: bool,
    threads: usize,
) -> Engine {
    let mut stubs = life_stubs(observer);
    stubs.extend(extra);
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

/// The life world's registered systems (no observer).
pub fn life_registered() -> rh_core::register::Registered {
    let stubs = life_stubs(false);
    let specs = agreeing_specs(&stubs);
    register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap_or_else(|e| panic!("{e}"))
}
