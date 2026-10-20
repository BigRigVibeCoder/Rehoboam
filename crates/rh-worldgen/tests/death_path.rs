//! V-INV-03, subset: the single death path (SPR-014 T-05; `05.1` §2 rule 2; EVO-001 §4, R9), with
//! the real `M2.demography` on worlds `rh-worldgen` builds and two test-only senders, each holding
//! its outcome as its own timer under its own purpose and fired in `Vital`'s timer pass:
//!
//! - `X7.murder` stands in for M7: at a Month pulse it schedules `MURDER_DUE` for a day of that
//!   month (purpose `exit_murder`); its fire submits `ScheduleDeath { victim, murder, culprit,
//!   None }`; a material input re-draws the day among the month's remaining day starts;
//! - `X1.minds` stands in for T1: at Day/Agenda it emits `MURDER_ATTEMPT` and schedules `DEATH_DUE`
//!   for the next day start (purpose `outcome_delay`) with the draft as the timer's cause; its fire
//!   submits `ScheduleDeath` with that cause as `cause_event`; a re-sample keeps its own tick.
//!
//! Each death gives the same consequence set as an M2 hazard death (alive 0 and then compaction,
//! one closed life-course row, one `DEATH`), is applied in the `Vital` of its fire, and is never
//! queued across an `Inputs` phase (V-DET-21: no command is queued at any instant's end). A natural
//! death and a commanded death of one person in one slot apply once, the natural one first. An
//! input scoping the victim re-samples the sender's timer, which a probe shows pending at the input.
//!
//! Refs: SPR-014 T-05; `docs/09` V-INV-03, V-DET-21; REV-EVO-001 R9, R10, R17.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken fixture must fail the test loudly"
)]

use std::path::{Path, PathBuf};

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, CommandSend, Lag, Plane};
use rh_core::agenda::TimerEntry;
use rh_core::command::{Arg, Command};
use rh_core::engine::{Engine, MaterialInput};
use rh_core::fixed::Q16_16;
use rh_core::ownership::SpecTable;
use rh_core::register::{register, system_id};
use rh_core::system::{
    AggregateSpec, DropKind, DropReason, InvariantResult, ParamSchema, PulseBinding, ResampleCtx,
    SimError, SpecPurpose, StepCtx, SystemDescriptor, SystemModel, TimerSpec,
};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{EdgeKind, Event, EventDraft, EventRef, RoleId};
use rh_ids::{BranchSalt, EntityRef, PersonId};
use rh_registry::ownership::{Ownership, SystemSpec};
use rh_registry::{DemographyParams, Founder, FounderRole, WorldSpec};
use rh_sys_demography::Demography;
use rh_sys_demography::demography::DEATH;
use spec_files::{load_specs, registry_system_ids};

mod spec_files;

const D360: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const MONTH: i64 = 30 * DAY;
const CAUSE_MURDER: i64 = 2;
const CAUSE_NATURAL: i64 = 1;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const X7_SPEC: &str = r#"
[system]
id = "X7.murder"
[[owns]]
column = "x7.plan"
entity = "x7"
type = "i64"
plane = "A"
[[reads]]
column = "person.id"
owner = "M2.demography"
lag = "same_instant"
use = "identity"
[[reads]]
column = "person.alive"
owner = "M2.demography"
lag = "same_instant"
use = "identity"
[[commands]]
target = "M2.demography"
command = "ScheduleDeath"
timer = "MURDER_DUE"
purpose = "exit_murder"
[[schedule]]
pulse = "Month"
phase = "Polity"
[[schedule]]
pulse = "Day"
phase = "Vital"
[[rng]]
purpose = "exit_murder"
key = ["ruler", "month"]
"#;

const X1_SPEC: &str = r#"
[system]
id = "X1.minds"
[[owns]]
column = "x1.plan"
entity = "x1"
type = "i64"
plane = "A"
[[reads]]
column = "person.id"
owner = "M2.demography"
lag = "same_instant"
use = "identity"
[[reads]]
column = "person.alive"
owner = "M2.demography"
lag = "same_instant"
use = "identity"
[[commands]]
target = "M2.demography"
command = "ScheduleDeath"
timer = "DEATH_DUE"
purpose = "outcome_delay"
[[emits]]
kind = "MURDER_ATTEMPT"
roles = ["target"]
[[schedule]]
pulse = "Day"
phase = "Agenda"
[[schedule]]
pulse = "Day"
phase = "Vital"
[[rng]]
purpose = "outcome_delay"
key = ["person", "tick"]
"#;

fn specs() -> SpecTable {
    let mut ids =
        registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).expect("registry");
    ids.extend(["X7.murder".to_owned(), "X1.minds".to_owned()]);
    let mut specs = load_specs(&repo().join("specs/systems")).expect("specs");
    for (name, text) in [
        ("X7.murder.odd.toml", X7_SPEC),
        ("X1.minds.odd.toml", X1_SPEC),
    ] {
        specs.push(SystemSpec::from_toml(Path::new(name), text).unwrap());
    }
    Ownership::build(&specs, &ids).spec_table()
}

enum Murder {
    ExitMurder,
}

impl SpecPurpose for Murder {
    const ALL: &'static [Self] = &[Self::ExitMurder];
    fn name(&self) -> &'static str {
        "exit_murder"
    }
}

enum Delay {
    OutcomeDelay,
}

impl SpecPurpose for Delay {
    const ALL: &'static [Self] = &[Self::OutcomeDelay];
    fn name(&self) -> &'static str {
        "outcome_delay"
    }
}

/// A test-only sender: its id, its plan column, its bindings.
struct Sender {
    id: &'static str,
    plan: &'static str,
    bindings: Vec<PulseBinding>,
}

fn read(column: &'static str) -> ColumnRead {
    ColumnRead {
        column,
        owner: "M2.demography",
        lag: Lag::SameInstant,
    }
}

fn living(v: &SystemView<'_>, who: PersonId) -> Result<bool, SimError> {
    Ok(v.person_row(who)?
        .is_some_and(|r| v.read("person.alive").is_ok_and(|a| a[r] == 1)))
}

impl SystemModel for Sender {
    type Index = ();

    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: self.id,
            fidelity: "F0",
            spec_version: "0.0.0",
        }
    }
    fn access(&self) -> AccessSpec {
        AccessSpec {
            reads: vec![read("person.id"), read("person.alive")],
            writes: vec![ColumnWrite {
                column: self.plan,
                plane: Plane::A,
            }],
            sends: vec![CommandSend {
                command: "ScheduleDeath",
                target: "M2.demography",
            }],
            emits: if self.id == "X1.minds" {
                vec!["MURDER_ATTEMPT"]
            } else {
                Vec::new()
            },
            ..AccessSpec::default()
        }
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        self.bindings.clone()
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        if self.id == "X7.murder" {
            Murder::names()
        } else {
            Delay::names()
        }
    }
    fn params_schema(&self) -> ParamSchema {
        ParamSchema::default()
    }
    fn roles(&self) -> &'static [(&'static str, RoleId)] {
        static ROLES: [(&str, RoleId); 1] = [("target", RoleId::TARGET)];
        if self.id == "X1.minds" { &ROLES } else { &[] }
    }
    fn step(
        &self,
        b: PulseBinding,
        v: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        // plan: [active, the victim's id bits, the instant it acts at, the outcome's instant].
        let plan = v.read(self.plan)?.to_vec();
        if plan[0] == 0 || c.now().minutes() != plan[2] || b.phase == Phase::Vital {
            return Ok(());
        }
        let who = PersonId::from_raw(plan[1].cast_unsigned());
        let (cause, purpose_is_murder) = if self.id == "X1.minds" {
            let draft = c.emit(
                EventDraft::new("MURDER_ATTEMPT").role(RoleId::TARGET, EntityRef::person(who)),
            )?;
            (EventRef::Draft(draft), false)
        } else {
            (EventRef::None, true)
        };
        let spec = TimerSpec {
            time: SimTime::from_minutes(plan[3]),
            phase: Phase::Vital,
            entity: EntityRef::person(who),
            kind: 1,
            payload: 77, // the culprit's id, carried by the timer (R10)
            cause,
        };
        if purpose_is_murder {
            c.schedule(&Murder::ExitMurder, spec)
        } else {
            c.schedule(&Delay::OutcomeDelay, spec)
        }
    }
    fn fire(
        &self,
        t: &TimerEntry,
        v: &mut SystemView<'_>,
        c: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let who = PersonId::from_raw(t.key.entity.raw());
        // A sender's fire checks the key person is alive before it submits (05.1 §2 rule 2).
        if !living(v, who)? {
            c.drop_timer(t, DropReason::Dead);
            return Ok(());
        }
        c.submit(Command {
            kind: "ScheduleDeath",
            person: who,
            args: vec![
                Arg::Int(who.to_raw().cast_signed()),
                Arg::Int(CAUSE_MURDER),
                Arg::Int(t.timer.payload),
                Arg::Event(t.timer.cause),
            ],
            cause: EventRef::None,
        })
    }
    fn resample_timer(
        &self,
        t: &TimerEntry,
        ctx: &ResampleCtx,
    ) -> Result<Option<SimTime>, SimError> {
        if self.id == "X1.minds" {
            return Ok(Some(t.key.time)); // a fixed delay re-samples to its own tick (R9)
        }
        // R17: uniform over the month's day starts at or after the input (the input's own day
        // when its day start is after it).
        let t_in = ctx.now().minutes();
        let month_end = (t_in / MONTH + 1) * MONTH;
        let first = (t_in + DAY - 1) / DAY * DAY;
        let first = if first == t_in { first + DAY } else { first };
        let days = (month_end - first) / DAY;
        let pick = u64::try_from(days.max(1)).unwrap();
        let k = i64::try_from((u128::from(ctx.draw(0)) * u128::from(pick)) >> 64).unwrap();
        Ok(Some(SimTime::from_minutes(first + k * DAY)))
    }
    fn invariants(&self, _: &WorldTables) -> Vec<InvariantResult> {
        Vec::new()
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

fn senders() -> Vec<Box<dyn SystemModel<Index = ()>>> {
    vec![
        Box::new(Sender {
            id: "X7.murder",
            plan: "x7.plan",
            bindings: vec![
                PulseBinding {
                    pulse: Pulse::Month,
                    phase: Phase::Polity,
                },
                PulseBinding {
                    pulse: Pulse::Day,
                    phase: Phase::Vital,
                },
            ],
        }),
        Box::new(Sender {
            id: "X1.minds",
            plan: "x1.plan",
            bindings: vec![
                PulseBinding {
                    pulse: Pulse::Day,
                    phase: Phase::Agenda,
                },
                PulseBinding {
                    pulse: Pulse::Day,
                    phase: Phase::Vital,
                },
            ],
        }),
    ]
}

/// No plan.
const IDLE: [i64; 4] = [0; 4];

/// A plan: kill `who` (its id), acting at `act`, the death due at `due`.
fn plan(who: u64, act: i64, due: i64) -> [i64; 4] {
    [1, who.cast_signed(), act, due]
}

/// The founders' ids of the world `engine` builds, sorted.
fn founder_ids(seed: u64, founders: usize, age: u8, e0: i16) -> Vec<u64> {
    ids(&engine(seed, founders, age, e0, IDLE, IDLE))
}

/// The world: `founders` founders aged `age`; the senders' plans.
fn engine(seed: u64, founders: usize, age: u8, e0: i16, x7: [i64; 4], x1: [i64; 4]) -> Engine {
    engine_with(seed, (founders, age, e0), x7, x1, |_| {})
}

/// [`engine`], with `edit` applied to the genesis tables.
fn engine_with(
    seed: u64,
    (founders, age, e0): (usize, u8, i16),
    x7: [i64; 4],
    x1: [i64; 4],
    edit: impl FnOnce(&mut WorldTables),
) -> Engine {
    let spec = WorldSpec {
        seed,
        calendar: D360,
        horizon_years: 5,
        systems: Vec::new(),
        demography: Some(DemographyParams {
            e0: Q16_16::from_int(e0),
            brass_beta: Q16_16::ONE,
            tfr: Q16_16::from_bits(321_126), // 4.9 in Q16_16
            elite_fertility_mult: Q16_16::ONE,
            density_capacity: 10_000_000,
            density_mortality_slope: Q16_16::ONE,
        }),
        founders: (0..founders)
            .map(|_| Founder {
                role: FounderRole::Commoner,
                sex: None,
                age_years: Some(age),
            })
            .collect(),
        households: Vec::new(),
    };
    let table = specs();
    let mut tables = rh_worldgen::genesis(&spec, &table).unwrap();
    tables.insert("x7.plan", x7.to_vec());
    tables.insert("x1.plan", x1.to_vec());
    edit(&mut tables);
    let mut systems: Vec<Box<dyn SystemModel<Index = ()>>> =
        vec![Box::new(Demography::from_world(&spec).unwrap().unwrap())];
    systems.extend(senders());
    let registered = register(systems, &table).unwrap_or_else(|e| panic!("{e}"));
    Engine::new(
        D360,
        seed,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    )
}

/// Runs to `end`, checking after every instant that no command is left queued: a commanded death
/// exists as a command only between a `Vital` timer pass and that `Vital`'s drain (V-DET-21).
fn run(e: &mut Engine, end: i64) -> Vec<Event> {
    let mut events = Vec::new();
    while let Some(t) = e.next_instant().filter(|t| t.minutes() <= end) {
        e.advance_until(t, |_, _| {})
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(e.commands().len(), 0, "a command queued at the end of {t}");
        events.extend(e.drain_events());
    }
    events
}

fn deaths_of(events: &[Event], who: u64) -> Vec<&Event> {
    events
        .iter()
        .filter(|ev| ev.kind == DEATH && ev.roles.iter().any(|s| s.entity.raw() == who))
        .collect()
}

fn ids(e: &Engine) -> Vec<u64> {
    e.tables()
        .get("person.id")
        .unwrap()
        .iter()
        .map(|v| v.cast_unsigned())
        .collect()
}

/// The life-course row of `who`: (exit kind, exit tick, death cause).
fn exit(e: &Engine, who: u64) -> (i64, i64, i64) {
    let t = e.tables();
    let row = t
        .get("life_course.person_id")
        .unwrap()
        .iter()
        .position(|&p| p.cast_unsigned() == who)
        .unwrap();
    (
        t.get("life_course.exit_kind").unwrap()[row],
        t.get("life_course.exit_tick").unwrap()[row],
        t.get("life_course.death_cause").unwrap()[row],
    )
}

/// V-INV-03 subset: X7's and X1's deaths take the single path, each in the `Vital` of its fire;
/// X1's `DEATH` is caused by its `MURDER_ATTEMPT`.
#[test]
fn v_inv_03_commanded_deaths_take_the_single_death_path() {
    // Young founders at e0 = 85: no natural death in two months for these seeds (checked below).
    for seed in [3, 11, 29] {
        let x7_due = MONTH + 9 * DAY;
        let x1_due = MONTH + 15 * DAY;
        let before = founder_ids(seed, 8, 20, 85);
        let mut e = engine(
            seed,
            8,
            20,
            85,
            plan(before[0], MONTH, x7_due),
            plan(before[1], x1_due - DAY, x1_due),
        );
        let events = run(&mut e, 2 * MONTH);
        let after = ids(&e);
        let (v7, v1) = (before[0], before[1]);
        assert_eq!(
            after.len(),
            before.len() - 2,
            "seed {seed}: exactly the two victims died"
        );
        assert!(
            !after.contains(&v7) && !after.contains(&v1),
            "both rows compacted"
        );
        for (who, due) in [(v7, x7_due), (v1, x1_due)] {
            assert_eq!(
                exit(&e, who),
                (1, due, CAUSE_MURDER),
                "seed {seed}: one closed row"
            );
            let d = deaths_of(&events, who);
            assert_eq!(d.len(), 1, "one DEATH");
            assert_eq!(
                (d[0].tick, d[0].phase),
                (due, Phase::Vital.index()),
                "in its Vital"
            );
            assert_eq!(d[0].payload, [CAUSE_MURDER]);
        }
        let attempt = events
            .iter()
            .find(|ev| ev.kind == "MURDER_ATTEMPT")
            .unwrap();
        assert_eq!(
            deaths_of(&events, v1)[0]
                .causes
                .iter()
                .map(|e| (e.cause, e.kind))
                .collect::<Vec<_>>(),
            [(attempt.id, EdgeKind::Triggered)],
            "DEATH <- MURDER_ATTEMPT"
        );
        assert!(
            deaths_of(&events, v7)[0].causes.is_empty(),
            "a hazard draw has no cause"
        );
        assert!(e.dropped().is_empty());
    }
    println!("V-INV-03: commanded deaths take the single death path in 3 worlds");
}

/// A natural death and a commanded death of one person in one slot apply once, the natural one
/// first (EVO-001 §4): the sender's fire, after M2's in the timer pass, finds the person dead.
#[test]
fn a_natural_and_a_commanded_death_in_one_slot_apply_once_the_natural_first() {
    // Founders aged 99 at e0 = 20: some draw a natural death in month 0. Find one, and its day,
    // with a run that commands nothing, then aim X1 at that very slot.
    let mut probe = engine(5, 100, 99, 20, IDLE, IDLE);
    probe.advance_until(SimTime::EPOCH, |_, _| {}).unwrap();
    let (key, timer) = probe
        .agenda()
        .pending()
        .into_iter()
        .find(|(k, t)| t.owner == system_id("M2.demography") && k.time.minutes() > DAY)
        .expect("a natural death timer in month 0");
    let (who, tau) = (key.entity.raw(), key.time.minutes());
    assert_eq!(timer.kind, 1, "M2's DEATH_TIMER");
    let mut e = engine(5, 100, 99, 20, IDLE, plan(who, tau - DAY, tau));
    let events = run(&mut e, MONTH);
    assert_eq!(
        exit(&e, who),
        (1, tau, CAUSE_NATURAL),
        "the natural death, once"
    );
    assert_eq!(deaths_of(&events, who).len(), 1);
    assert_eq!(
        e.dropped()
            .get(&("X1.minds", DropKind::Timer(1), DropReason::Dead)),
        Some(&1),
        "the commanded death's timer found the person dead"
    );
}

/// An input that scopes the victim re-samples the sender's timer (R9, R10): the probe shows the
/// timer pending at the input; it is re-inserted (a new `seq`) with its culprit and cause kept.
#[test]
fn an_input_on_the_victim_resamples_the_senders_timer() {
    let x7_due = MONTH + 20 * DAY;
    let who = founder_ids(3, 8, 20, 85)[0];
    let mut e = engine(3, 8, 20, 85, plan(who, MONTH, x7_due), IDLE);
    let victim = EntityRef::person(PersonId::from_raw(who));
    e.advance_until(SimTime::from_minutes(MONTH + 5 * DAY), |_, _| {})
        .unwrap();
    let before = e.agenda().pending_for(victim);
    assert_eq!(
        before.len(),
        1,
        "the probe: the murder timer is pending at the input"
    );
    assert_eq!(before[0].1.owner, system_id("X7.murder"));
    let at = SimTime::from_minutes(MONTH + 6 * DAY + 7);
    e.submit_input(
        at,
        MaterialInput {
            scope: [victim].into_iter().collect(),
        },
    )
    .unwrap();
    e.advance_until(at, |_, _| {}).unwrap();
    let after = e.agenda().pending_for(victim);
    assert_eq!(after.len(), 1, "re-drawn, not dropped");
    assert_ne!(after[0].0.seq, before[0].0.seq, "cancelled and re-inserted");
    assert!(after[0].0.time > at, "a later day start");
    assert_eq!(after[0].1.payload, 77, "the culprit kept");
    assert_eq!(after[0].1.purpose, before[0].1.purpose);
    let events = run(&mut e, 2 * MONTH);
    assert_eq!(deaths_of(&events, victim.raw()).len(), 1);
}

/// Seeds `a` and `b` married to each other (`person.spouse` both ways).
fn marry(t: &mut WorldTables, a: u64, b: u64) {
    let ids: Vec<u64> = t
        .get("person.id")
        .unwrap()
        .iter()
        .map(|v| v.cast_unsigned())
        .collect();
    let row = |p: u64| ids.iter().position(|&q| q == p).unwrap();
    let (ra, rb) = (row(a), row(b));
    let mut spouse = t.get("person.spouse").unwrap().clone();
    spouse[ra] = b.cast_signed();
    spouse[rb] = a.cast_signed();
    t.insert("person.spouse", spouse);
}

/// `who`'s `person.spouse`, if `who` has a ledger row.
fn spouse_of(e: &Engine, who: u64) -> Option<u64> {
    let row = ids(e).iter().position(|&q| q == who)?;
    Some(e.tables().get("person.spouse").unwrap()[row].cast_unsigned())
}

/// The Tester's DEF-2: `person.spouse` is cleared on the spouse's death (0.2.3 `[[owns]]`), when
/// the spouse dies by a command and when by M2's own timer; the survivor's row keeps 0 after the
/// dead row is compacted.
#[test]
fn a_death_clears_the_survivors_spouse_by_command_and_by_timer() {
    // By command: X1 kills B; A was married to B.
    let before = founder_ids(3, 8, 20, 85);
    let (a, b) = (before[0], before[1]);
    let x1_due = MONTH + 15 * DAY;
    let mut e = engine_with(3, (8, 20, 85), IDLE, plan(b, x1_due - DAY, x1_due), |t| {
        marry(t, a, b);
    });
    assert_eq!(spouse_of(&e, a), Some(b), "seeded married");
    let events = run(&mut e, 2 * MONTH);
    assert_eq!(deaths_of(&events, b).len(), 1, "B died by the command");
    assert_eq!(spouse_of(&e, b), None, "B's row compacted");
    assert_eq!(spouse_of(&e, a), Some(0), "A's spouse cleared");
    // By timer: B draws a natural death in month 0; A has no timer that month.
    let mut probe = engine(5, 100, 99, 20, IDLE, IDLE);
    probe.advance_until(SimTime::EPOCH, |_, _| {}).unwrap();
    let pending: Vec<u64> = probe
        .agenda()
        .pending()
        .into_iter()
        .filter(|(_, t)| t.owner == system_id("M2.demography"))
        .map(|(k, _)| k.entity.raw())
        .collect();
    let b = pending[0];
    let a = ids(&probe)
        .into_iter()
        .find(|p| !pending.contains(p))
        .expect("a founder with no death in month 0");
    let mut e = engine_with(5, (100, 99, 20), IDLE, IDLE, |t| marry(t, a, b));
    let events = run(&mut e, MONTH - 1);
    assert_eq!(deaths_of(&events, b).len(), 1, "B died by M2's timer");
    assert_eq!(spouse_of(&e, b), None, "B's row compacted");
    assert_eq!(spouse_of(&e, a), Some(0), "A's spouse cleared");
    println!("DEF-2: a death clears the survivor's spouse, by command and by timer");
}

/// The Tester's probe: two `ScheduleDeath`s for one victim in one slot (X7's and X1's) apply
/// once; M2 drops the second as `dead`, and there is one `DEATH` and one closed row.
#[test]
fn two_commanded_deaths_of_one_person_in_one_slot_apply_once() {
    for seed in [3, 11] {
        let v = founder_ids(seed, 8, 20, 85)[0];
        let due = MONTH + 12 * DAY;
        let mut e = engine(
            seed,
            8,
            20,
            85,
            plan(v, MONTH, due),
            plan(v, due - DAY, due),
        );
        let events = run(&mut e, 2 * MONTH);
        assert_eq!(deaths_of(&events, v).len(), 1, "seed {seed}: one DEATH");
        assert_eq!(
            exit(&e, v),
            (1, due, CAUSE_MURDER),
            "seed {seed}: one closed row"
        );
        assert_eq!(
            e.dropped().get(&(
                "M2.demography",
                DropKind::Command("ScheduleDeath"),
                DropReason::Dead
            )),
            Some(&1),
            "seed {seed}: the second command found the victim dead"
        );
    }
    println!("probes: a double commanded death applies once");
}

/// The Tester's probe: a person killed by command keeps M2's natural death timer; when it fires,
/// M2 drops it as `dead` and nothing else happens.
#[test]
fn a_natural_timer_after_a_commanded_death_is_dropped_as_dead() {
    let mut probe = engine(5, 100, 99, 20, IDLE, IDLE);
    probe.advance_until(SimTime::EPOCH, |_, _| {}).unwrap();
    let (key, _) = probe
        .agenda()
        .pending()
        .into_iter()
        .find(|(k, t)| t.owner == system_id("M2.demography") && k.time.minutes() > 3 * DAY)
        .expect("a natural death timer after day 3 of month 0");
    let (who, tau) = (key.entity.raw(), key.time.minutes());
    let due = 2 * DAY;
    let mut e = engine(5, 100, 99, 20, IDLE, plan(who, due - DAY, due));
    let events = run(&mut e, MONTH - 1);
    assert_eq!(
        deaths_of(&events, who).len(),
        1,
        "one DEATH, the commanded one"
    );
    assert_eq!(exit(&e, who), (1, due, CAUSE_MURDER));
    assert!(tau > due, "the natural timer was later");
    assert_eq!(
        e.dropped()
            .get(&("M2.demography", DropKind::Timer(1), DropReason::Dead)),
        Some(&1),
        "M2's natural timer fired for the dead and was dropped"
    );
    println!("probes: a natural timer after a commanded death is dropped as dead");
}

/// A scenario's world, built afresh for each thread count.
type MakeEngine = Box<dyn Fn() -> Engine>;

/// The review's K7: every death-path scenario of this file is thread-invariant. Each runs at 1, 2,
/// 4 and 8 threads with equal `EngineState` and equal `dropped{…}` counters: the commanded deaths
/// of V-INV-03, a natural and a commanded death in one slot, two commanded deaths of one victim,
/// and the survivor's spouse cleared (DEF-2).
#[test]
fn every_death_path_scenario_is_thread_invariant() {
    let ids = founder_ids(3, 8, 20, 85);
    let v = [ids[0], ids[1]];
    let mut probe = engine(5, 100, 99, 20, IDLE, IDLE);
    probe.advance_until(SimTime::EPOCH, |_, _| {}).unwrap();
    let (key, _) = probe
        .agenda()
        .pending()
        .into_iter()
        .find(|(k, t)| t.owner == system_id("M2.demography") && k.time.minutes() > DAY)
        .expect("a natural death timer in month 0");
    let (natural, tau) = (key.entity.raw(), key.time.minutes());
    let x7_due = MONTH + 9 * DAY;
    let x1_due = MONTH + 15 * DAY;
    let scenarios: Vec<(&str, MakeEngine, i64)> = vec![
        (
            "V-INV-03",
            Box::new(move || {
                engine(
                    3,
                    8,
                    20,
                    85,
                    plan(v[0], MONTH, x7_due),
                    plan(v[1], x1_due - DAY, x1_due),
                )
            }),
            2 * MONTH,
        ),
        (
            "same slot",
            Box::new(move || engine(5, 100, 99, 20, IDLE, plan(natural, tau - DAY, tau))),
            MONTH,
        ),
        (
            "double command",
            Box::new(move || {
                let due = MONTH + 12 * DAY;
                engine(
                    3,
                    8,
                    20,
                    85,
                    plan(v[0], MONTH, due),
                    plan(v[0], due - DAY, due),
                )
            }),
            2 * MONTH,
        ),
        (
            "DEF-2",
            Box::new(move || {
                engine_with(
                    3,
                    (8, 20, 85),
                    IDLE,
                    plan(v[1], x1_due - DAY, x1_due),
                    |t| {
                        marry(t, v[0], v[1]);
                    },
                )
            }),
            2 * MONTH,
        ),
    ];
    for (name, make, end) in &scenarios {
        let at = |threads: usize| {
            let mut e = make();
            e.set_threads(threads);
            drop(run(&mut e, *end));
            (e.snapshot(), e.dropped().clone())
        };
        let (state, dropped) = at(1);
        for threads in [2, 4, 8] {
            let (s, d) = at(threads);
            assert!(s == state, "{name}: EngineState at {threads} threads");
            assert_eq!(d, dropped, "{name}: dropped at {threads} threads");
        }
    }
    println!("K7: four death-path scenarios equal at 1, 2, 4 and 8 threads");
}
