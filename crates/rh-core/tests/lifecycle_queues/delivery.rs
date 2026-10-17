//! V-DET-21, the delivery part (SPR-014 T-01; `05.1` §2 rule 2, "Key and delivery" and
//! "Draining"; rule 4's merge order).
//!
//! The world: `A1.send` binds Day in Vital, Polity, Agenda and Culture and, in each step, submits
//! `Req` to `B1.own` for a keyed draw of persons, with the phase it was sent in as its argument.
//! `B1.own` drains `Req` at Day/Agenda only (`drained_by`); it also binds Day/Activity and
//! schedules a timer each day for the next day's Agenda, and both of those jobs refuse a non-empty
//! inbox. A1 and B1 share Agenda's level (no edge between them), so a command sent at Agenda merges
//! after B1's binding has run and waits for the next day start; one sent at Vital or Polity is
//! drained in the same instant.

use rh_core::agenda::TimerEntry;
use rh_core::command::{Arg, Command};
use rh_core::engine::Engine;
use rh_core::register::register;
use rh_core::system::{PulseBinding, SimError, SpecPurpose, StepCtx, TimerSpec};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::EventRef;
use rh_ids::rng::{h64_words, tag};
use rh_ids::{BranchSalt, EntityRef, PersonId};

use crate::support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const TAG: u64 = tag("rehoboam/rh-core/tests/spr014-delivery/v1");
/// The property's floor: three world sizes and eight seeds (SPR-014, carried rules).
pub const SIZES: [usize; 3] = [3, 17, 64];
/// See [`SIZES`].
pub const SEEDS: [u64; 8] = [1, 2, 3, 5, 8, 13, 21, 34];

/// The sender's purpose.
pub enum Pick {
    Pick,
}

impl SpecPurpose for Pick {
    const ALL: &'static [Self] = &[Self::Pick];
    fn name(&self) -> &'static str {
        "pick"
    }
}

/// The owner's purpose (its daily timer).
pub enum Tick {
    Tick,
}

impl SpecPurpose for Tick {
    const ALL: &'static [Self] = &[Self::Tick];
    fn name(&self) -> &'static str {
        "tick"
    }
}

pub fn person(row: usize) -> PersonId {
    PersonId::from_raw(h64_words(TAG, &[row as u64]) >> 4)
}

fn fail(system: &'static str, message: String) -> SimError {
    SimError::System { system, message }
}

fn send_step(
    _: &Stub,
    b: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let size = usize::try_from(v.read("send.size")?[0]).unwrap_or(0);
    let phase = i64::from(b.phase.index());
    for row in 0..size {
        if c.rng(&Pick::Pick, person(row), u64::from(b.phase.index()))? % 3 == 0 {
            c.submit(Command {
                kind: "Req",
                person: person(row),
                args: vec![Arg::Int(phase)],
                cause: EventRef::None,
            })?;
        }
    }
    Ok(())
}

/// What a non-draining job of the owner does: it must see no inbox (`05.1` §2 rule 2).
fn refuse_inbox(c: &mut StepCtx<'_>) -> Result<(), SimError> {
    let inbox = c.take_commands();
    if inbox.is_empty() {
        Ok(())
    } else {
        Err(fail(
            "B1.own",
            format!(
                "delivered to a non-draining job: {} Req command(s) at ({}, {})",
                inbox.len(),
                c.now(),
                c.phase()
            ),
        ))
    }
}

fn own_step(
    _: &Stub,
    b: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if b.phase == Phase::Activity {
        refuse_inbox(c)?;
        // The daily timer, so the owner's Agenda timer pass runs too (it must drain nothing).
        let next = c
            .now()
            .checked_add_minutes(DAY)
            .ok_or_else(|| fail("B1.own", "time overflow".to_owned()))?;
        return c.schedule(
            &Tick::Tick,
            TimerSpec {
                time: next,
                phase: Phase::Agenda,
                entity: EntityRef::WORLD,
                kind: 1,
                payload: 0,
                cause: EventRef::None,
            },
        );
    }
    let now = c.now();
    let mut last = None;
    let (mut same, mut waits, mut log) = (0, 0, v.read("own.log")?[0].cast_unsigned());
    for q in c.take_commands() {
        let key = q.key();
        if last.is_some_and(|l| l >= key) {
            return Err(fail(
                "B1.own",
                format!("inbox out of key order at seq {}", q.seq),
            ));
        }
        last = Some(key);
        if q.command.kind != "Req" {
            return Err(fail("B1.own", format!("unexpected {}", q.command.kind)));
        }
        let sent = q.command.int(0).unwrap_or(-1);
        if sent != i64::from(q.phase.index()) {
            return Err(fail("B1.own", format!("seq {} lost its argument", q.seq)));
        }
        // The first run of the drainer (Day/Agenda) after the sender's merge.
        let expected = if q.phase < Phase::Agenda {
            q.tick
        } else {
            SimTime::from_minutes(q.tick.minutes() + DAY)
        };
        if now != expected {
            return Err(fail(
                "B1.own",
                format!(
                    "seq {} sent at ({}, {}) delivered at {now}, not at {expected}",
                    q.seq, q.tick, q.phase
                ),
            ));
        }
        if q.tick == now {
            same += 1;
        } else {
            waits += 1;
        }
        log = h64_words(
            TAG,
            &[
                log,
                now.minutes().cast_unsigned(),
                q.seq,
                q.command.person.to_raw(),
            ],
        );
    }
    v.write("own.same")?[0] += same;
    v.write("own.wait")?[0] += waits;
    v.write("own.log")?[0] = log.cast_signed();
    Ok(())
}

fn own_fire(
    _: &Stub,
    _: &TimerEntry,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    refuse_inbox(c)
}

pub fn stubs() -> Vec<Stub> {
    vec![
        Stub::new("A1.send")
            .writes("send.size")
            .purposes_of::<Pick>()
            .sends("Req", "B1.own")
            .at(Pulse::Day, Phase::Vital)
            .at(Pulse::Day, Phase::Polity)
            .at(Pulse::Day, Phase::Agenda)
            .at(Pulse::Day, Phase::Culture)
            .stepping(send_step),
        Stub::new("B1.own")
            .writes("own.log")
            .writes("own.same")
            .writes("own.wait")
            .purposes_of::<Tick>()
            .at(Pulse::Day, Phase::Activity)
            .at(Pulse::Day, Phase::Agenda)
            .applies_at("Req", Pulse::Day, Phase::Agenda)
            .typed("Req", &[("phase", "u8")], "person", "any")
            .stepping(own_step)
            .firing(own_fire),
    ]
}

pub fn engine(seed: u64, size: usize, threads: usize) -> Engine {
    let stubs = stubs();
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("send.size", vec![i64::try_from(size).unwrap_or(0)]);
    tables.insert("own.log", vec![0]);
    tables.insert("own.same", vec![0]);
    tables.insert("own.wait", vec![0]);
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

/// Runs `days` days; the tables and the commands still queued at the end.
pub fn run(mut engine: Engine, days: i64) -> Result<(WorldTables, usize), SimError> {
    engine.advance_until(SimTime::from_minutes(days * DAY - 1), |_, _| {})?;
    Ok((engine.tables().clone(), engine.commands().len()))
}

/// V-DET-21, delivery: every command reaches its drainer at the first run after its merge, in key
/// order, and no other job of the owner; the result is equal at 1, 2, 4 and 8 threads; the probe
/// sees both same-instant deliveries and cross-instant waits.
#[test]
fn v_det_21_delivery_to_the_one_draining_binding() {
    let (mut same, mut waits) = (0, 0);
    for size in SIZES {
        for seed in SEEDS {
            let base = run(engine(seed, size, 1), 40).unwrap_or_else(|e| panic!("{e}"));
            for threads in [2, 4, 8] {
                let other = run(engine(seed, size, threads), 40).unwrap_or_else(|e| panic!("{e}"));
                assert_eq!(base, other, "size {size} seed {seed} at {threads} threads");
            }
            same += base.0.get("own.same").map_or(0, |c| c[0]);
            waits += base.0.get("own.wait").map_or(0, |c| c[0]);
        }
    }
    assert!(same > 0, "the probe saw no same-instant delivery");
    assert!(waits > 0, "the probe saw no cross-instant wait");
    println!(
        "V-DET-21 delivery: {same} same-instant deliveries and {waits} cross-instant waits over 24 worlds, only to the drainer, equal at 1, 2, 4 and 8 threads"
    );
}
