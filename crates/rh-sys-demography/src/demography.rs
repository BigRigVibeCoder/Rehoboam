//! The system `M2.demography`, rung-0 slice (SPR-012 T-07; `specs/systems/M2.demography.odd.toml`).
//!
//! Bound to Month/Vital. At each Month pulse instant `t_m`, for each living person in ascending
//! `person.id`: one keyed Bernoulli on the month's `Prob32` probability (S8, `death_time` k = 0);
//! on a hit, one keyed day draw (k = 1) and a death timer at that day start inside the month,
//! never the pulse instant (S9, S10). A due timer fires in Vital's timer pass and calls
//! [`apply_death`] (S12; SPR-011 ruling 6): alive flag, death tick and cause, the life-course exit,
//! `DEATH` with the deceased as its subject (role `target`), and `EXTINCTION` once when the ledger
//! has no living row left (ruling 17). A material input in a person's scope re-draws a pending
//! death at the new epoch (S11, [`SystemModel::resample_timer`]).
//!
//! M2 refuses `brass_alpha` and any β ≠ 1 at construction ([`DemographyError`]; ruling 7).

use core::fmt;

use rh_core::Phase;
use rh_core::access::{AccessSpec, ColumnWrite, CommandApply, Plane};
use rh_core::agenda::TimerEntry;
use rh_core::command::QueuedCommand;
use rh_core::fixed::Q16_16;
use rh_core::system::{
    AggregateSpec, DropReason, InvariantResult, ParamSchema, PulseBinding, ResampleCtx, SimError,
    SpecPurpose, StepCtx, SystemDescriptor, SystemModel, TimerSpec,
};
use rh_core::time::{Calendar, Pulse, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_events::{EventDraft, EventRef};
use rh_ids::{EntityKind, EntityRef, PersonId};

use crate::schedule::{DAY, Schedule, ScheduleError, death_day, resample_day};

/// The skeleton `lint_odd.py --rust M2.demography` generated (ADR-0033 §3), checked by
/// `bin/gen_skeletons.py --check`; private, as the smoke systems keep theirs.
mod skeleton {
    #![allow(
        dead_code,
        unreachable_pub,
        reason = "generated skeleton items (Params) the rung-0 slice does not read"
    )]
    use rh_core::{Fixed, Phase, Pulse};
    include!("../../rh-core/tests/generated/M2_demography.rs");
}

use skeleton::{DRAINS, RngPurpose, SCHEDULE, SPEC_VERSION, SYSTEM_ID};

impl SpecPurpose for RngPurpose {
    const ALL: &'static [Self] = &[Self::DeathTime];
    fn name(&self) -> &'static str {
        RngPurpose::name(*self)
    }
}

/// The ledger's columns M2 owns.
pub const PERSON_ID: &str = "person.id";
/// See [`PERSON_ID`].
pub const PERSON_ALIVE: &str = "person.alive";
/// See [`PERSON_ID`].
pub const PERSON_BIRTH: &str = "person.birth_tick";
/// See [`PERSON_ID`].
pub const PERSON_DEATH_TICK: &str = "person.death_tick";
/// See [`PERSON_ID`].
pub const PERSON_DEATH_CAUSE: &str = "person.death_cause";
/// See [`PERSON_ID`].
pub const PERSON_SPOUSE: &str = "person.spouse";
/// The life-course columns [`apply_death`] writes.
pub const LC_PERSON: &str = "life_course.person_id";
/// See [`LC_PERSON`].
pub const LC_EXIT_KIND: &str = "life_course.exit_kind";
/// See [`LC_PERSON`].
pub const LC_EXIT_TICK: &str = "life_course.exit_tick";
/// See [`LC_PERSON`].
pub const LC_DEATH_CAUSE: &str = "life_course.death_cause";
/// See [`LC_PERSON`].
pub const LC_BIRTH: &str = "life_course.birth_tick";

/// Every column M2 owns (`[[owns]]`), in spec order.
pub const OWNED: [&str; 19] = [
    "person.id",
    "person.birth_tick",
    "person.sex",
    "person.alive",
    "person.death_tick",
    "person.mother",
    "person.father",
    "person.spouse",
    "person.death_cause",
    "life_course.person_id",
    "life_course.origin",
    "life_course.mother",
    "life_course.father",
    "life_course.sex",
    "life_course.birth_tick",
    "life_course.entry_tick",
    "life_course.exit_kind",
    "life_course.exit_tick",
    "life_course.death_cause",
];

/// S11's draws: k = 0 decides whether the re-drawn death happens, k = 1 picks its day. The two
/// `mutant-*` features exist only for the planted controls of `tests/draws_golden.rs` (the
/// Tester's surviving mutants, SPR-012): each moves one draw to the other k, and the differential
/// golden must FAIL. No build of a world enables them.
#[cfg(not(feature = "mutant-resample-hit-k1"))]
const K_HIT: u64 = 0;
#[cfg(feature = "mutant-resample-hit-k1")]
const K_HIT: u64 = 1;
/// See [`K_HIT`].
#[cfg(not(feature = "mutant-resample-day-k0"))]
const K_DAY: u64 = 1;
#[cfg(feature = "mutant-resample-day-k0")]
const K_DAY: u64 = 0;

/// The death timer's closure id (S10).
pub const DEATH_TIMER: u32 = 1;
/// `death_cause` natural (codes: none 0, natural 1, murder 2, battle 3, execution 4, accident 5).
pub const CAUSE_NATURAL: i64 = 1;
/// `life_course.exit_kind` death (open 0, death 1, emigration 2).
pub const EXIT_DEATH: i64 = 1;
/// The commands M2 applies in its `Day`/`Vital` drain (`[[applies]]`; SPR-014 T-05).
pub const SCHEDULE_DEATH: &str = "ScheduleDeath";
/// See [`SCHEDULE_DEATH`].
pub const MARRY: &str = "Marry";
/// The event kinds M2 emits.
pub const DEATH: &str = "DEATH";
/// See [`DEATH`].
pub const EXTINCTION: &str = "EXTINCTION";

/// Why M2 refused its parameters (ruling 7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DemographyError {
    /// `brass_alpha` set: rung 0 takes its level from e₀, and any α needs fixed-point `exp` (P1.3).
    AlphaRefused {
        /// The `Q16_16` bits given.
        bits: i32,
    },
    /// `brass_beta` other than exactly 1 (65536): Brass at β ≠ 1 needs fixed-point `exp`, `ln`.
    BetaNotOne {
        /// The `Q16_16` bits given.
        bits: i32,
    },
    /// The schedule refused e₀.
    Schedule(ScheduleError),
}

impl fmt::Display for DemographyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlphaRefused { bits } => write!(
                f,
                "M2.demography refuses brass_alpha = {bits} / 2^16 at rung 0: e0 sets the level, \
                 and any alpha needs fixed-point exp (P1.3; SPR-011 ruling 7)"
            ),
            Self::BetaNotOne { bits } => write!(
                f,
                "M2.demography refuses brass_beta = {bits} / 2^16 at rung 0: only beta = 1 \
                 (65536) runs until P1.3 (SPR-011 ruling 7)"
            ),
            Self::Schedule(e) => write!(f, "M2.demography: {e}"),
        }
    }
}

impl std::error::Error for DemographyError {}

/// The system `M2.demography`: its schedule, computed once at construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Demography {
    schedule: Schedule,
}

impl Demography {
    /// M2 for e₀ on `calendar`, refusing `brass_alpha` (any `Some`) and β ≠ 1 (ruling 7).
    ///
    /// # Errors
    /// [`DemographyError`], each naming the refused parameter.
    pub fn new(
        e0: Q16_16,
        brass_alpha: Option<Q16_16>,
        brass_beta: Q16_16,
        calendar: Calendar,
    ) -> Result<Self, DemographyError> {
        if let Some(alpha) = brass_alpha {
            return Err(DemographyError::AlphaRefused {
                bits: alpha.to_bits(),
            });
        }
        if brass_beta != Q16_16::ONE {
            return Err(DemographyError::BetaNotOne {
                bits: brass_beta.to_bits(),
            });
        }
        let schedule = Schedule::new(e0, calendar).map_err(DemographyError::Schedule)?;
        Ok(Self { schedule })
    }

    /// The schedule.
    #[must_use]
    pub const fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    #[allow(clippy::needless_pass_by_value, reason = "a map_err adapter")]
    fn sched_err(e: ScheduleError) -> SimError {
        fail(e.to_string())
    }

    /// S8-S10 at the Month pulse instant.
    fn mortality(&self, view: &SystemView<'_>, ctx: &mut StepCtx<'_>) -> Result<(), SimError> {
        let t_m = ctx.now();
        let ids = view.read(PERSON_ID)?;
        let births = view.read(PERSON_BIRTH)?;
        let alive = view.read(PERSON_ALIVE)?;
        for ((&id, &birth), &a) in ids.iter().zip(births).zip(alive) {
            if a == 0 {
                continue;
            }
            let person = PersonId::from_raw(id.cast_unsigned());
            let entity = EntityRef::person(person);
            if !ctx.own_timers(entity).is_empty() {
                return Err(fail(format!(
                    "person {} has a death timer pending at the Month pulse {t_m}: INV-M2-03",
                    id.cast_unsigned()
                )));
            }
            let (q, end) = self.schedule.month_q(t_m, birth).map_err(Self::sched_err)?;
            if !ctx.chance(&RngPurpose::DeathTime, person, 0, q)? {
                continue;
            }
            let d1 = ctx.rng(&RngPurpose::DeathTime, person, 1)?;
            let tau = death_day(t_m.minutes(), end.minutes() - t_m.minutes(), d1);
            ctx.schedule(
                &RngPurpose::DeathTime,
                TimerSpec {
                    time: SimTime::from_minutes(tau),
                    phase: Phase::Vital,
                    entity,
                    kind: DEATH_TIMER,
                    payload: birth,
                    cause: rh_events::EventRef::None,
                },
            )?;
        }
        Ok(())
    }
}

fn fail(message: String) -> SimError {
    SimError::System {
        system: SYSTEM_ID,
        message,
    }
}

/// The ledger row of `person` (`person.id` is sorted ascending as `u64`).
fn ledger_row(view: &SystemView<'_>, person: u64) -> Result<Option<usize>, SimError> {
    Ok(view
        .read(PERSON_ID)?
        .binary_search_by_key(&person, |v| v.cast_unsigned())
        .ok())
}

/// What [`apply_death`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Death {
    /// The person died now.
    Applied,
    /// The person had no living ledger row (dead earlier, or absent): nothing was written. The
    /// caller drops the timer or the command and counts it (`05.1` §2 rule 2; V-INV-01: the exit
    /// fields are written at most once).
    AlreadyDead,
}

/// The single death path (S12; SPR-011 ruling 6; `05.1` §2 rule 2): the dead row (alive 0, death
/// tick, cause), the person's open life-course exit (written once), `DEATH` with the deceased as
/// subject, the cause code as payload and a causal edge from `cause_event` when there is one,
/// then `EXTINCTION` caused by that `DEATH` if no ledger row is alive. A timer fire and M2's
/// `Day`/`Vital` drain of `ScheduleDeath` both call it (SPR-014). Idempotent: a person with no
/// living ledger row is left as is ([`Death::AlreadyDead`]). The culprit's `actor` role on `DEATH`
/// is SPR-020's.
///
/// # Errors
/// [`SimError::System`] if a living person has no open life-course entry; any error of the view
/// or the context.
pub fn apply_death(
    view: &mut SystemView<'_>,
    ctx: &mut StepCtx<'_>,
    person: PersonId,
    cause: i64,
    cause_event: EventRef,
) -> Result<Death, SimError> {
    let now = ctx.now().minutes();
    let id = person.to_raw();
    let Some(row) =
        ledger_row(view, id)?.filter(|&r| view.read(PERSON_ALIVE).is_ok_and(|a| a[r] == 1))
    else {
        return Ok(Death::AlreadyDead);
    };
    let open = {
        let persons = view.read(LC_PERSON)?;
        let kinds = view.read(LC_EXIT_KIND)?;
        persons
            .iter()
            .zip(kinds)
            .rposition(|(&p, &k)| p.cast_unsigned() == id && k == 0)
            .ok_or_else(|| fail(format!("person {id} has no open life-course entry")))?
    };
    view.write(PERSON_ALIVE)?[row] = 0;
    view.write(PERSON_DEATH_TICK)?[row] = now;
    view.write(PERSON_DEATH_CAUSE)?[row] = cause;
    view.write(LC_EXIT_KIND)?[open] = EXIT_DEATH;
    view.write(LC_EXIT_TICK)?[open] = now;
    view.write(LC_DEATH_CAUSE)?[open] = cause;
    let mut draft = EventDraft::new(DEATH)
        .subject(EntityRef::person(person))
        .with_payload(vec![cause]);
    if let Some(cause) = cause_event.known() {
        draft = draft.caused_by(cause);
    }
    let death = ctx.emit(draft)?;
    if view.read(PERSON_ALIVE)?.iter().all(|&a| a == 0) {
        ctx.emit(EventDraft::new(EXTINCTION).after(death))?;
    }
    Ok(Death::Applied)
}

/// Whether `person` has a living ledger row.
fn living(view: &SystemView<'_>, person: u64) -> Result<bool, SimError> {
    Ok(ledger_row(view, person)?.is_some_and(|r| view.read(PERSON_ALIVE).is_ok_and(|a| a[r] == 1)))
}

/// The `Day`/`Vital` drain (M2 0.2.3; `05.1` §2 rule 2; SPR-014 T-05): every queued
/// `ScheduleDeath` and `Marry`, in key order. `rh-core` has already dropped each one whose key
/// person was dead when the level started (`key = "living"`); what this drain finds dead, it drops
/// with reason `dead`.
fn drain(view: &mut SystemView<'_>, ctx: &mut StepCtx<'_>) -> Result<(), SimError> {
    for q in ctx.take_commands() {
        match q.command.kind {
            SCHEDULE_DEATH => schedule_death(view, ctx, &q)?,
            MARRY => marry(view, ctx, &q)?,
            other => return Err(fail(format!("M2 does not apply command {other}"))),
        }
    }
    Ok(())
}

/// `ScheduleDeath { victim, cause, culprit, cause_event }` through [`apply_death`].
fn schedule_death(
    view: &mut SystemView<'_>,
    ctx: &mut StepCtx<'_>,
    q: &QueuedCommand,
) -> Result<(), SimError> {
    let victim = q
        .command
        .int(0)
        .ok_or_else(|| fail("ScheduleDeath without a victim".to_owned()))?;
    let cause = q
        .command
        .int(1)
        .ok_or_else(|| fail("ScheduleDeath without a cause".to_owned()))?;
    let cause_event = q
        .command
        .args
        .get(3)
        .and_then(|a| a.event())
        .unwrap_or_default();
    let victim = PersonId::from_raw(victim.cast_unsigned());
    if apply_death(view, ctx, victim, cause, cause_event)? == Death::AlreadyDead {
        ctx.drop_command(q, DropReason::Dead);
    }
    Ok(())
}

/// `Marry { a, b }`, as frozen 0.2.x states it: `person.spouse` written both ways; a dead party
/// drops it. Households and the marriage rows (N7) are SPR-015's.
fn marry(
    view: &mut SystemView<'_>,
    ctx: &mut StepCtx<'_>,
    q: &QueuedCommand,
) -> Result<(), SimError> {
    let a = q
        .command
        .int(0)
        .ok_or_else(|| fail("Marry without a".to_owned()))?;
    let b = q
        .command
        .int(1)
        .ok_or_else(|| fail("Marry without b".to_owned()))?;
    let (a, b) = (a.cast_unsigned(), b.cast_unsigned());
    if !living(view, a)? || !living(view, b)? {
        ctx.drop_command(q, DropReason::Dead);
        return Ok(());
    }
    let (Some(ra), Some(rb)) = (ledger_row(view, a)?, ledger_row(view, b)?) else {
        ctx.drop_command(q, DropReason::Dead);
        return Ok(());
    };
    let spouse = view.write(PERSON_SPOUSE)?;
    spouse[ra] = b.cast_signed();
    spouse[rb] = a.cast_signed();
    Ok(())
}

impl SystemModel for Demography {
    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: SYSTEM_ID,
            fidelity: "F1",
            spec_version: SPEC_VERSION,
        }
    }

    fn access(&self) -> AccessSpec {
        AccessSpec {
            writes: OWNED
                .map(|column| ColumnWrite {
                    column,
                    plane: Plane::A,
                })
                .to_vec(),
            applies: CommandApply::from_drains(DRAINS),
            emits: vec![DEATH, EXTINCTION],
            ..AccessSpec::default()
        }
    }

    fn schedule(&self) -> Vec<PulseBinding> {
        PulseBinding::from_schedule(SCHEDULE)
    }

    fn rng_purposes(&self) -> Vec<&'static str> {
        <RngPurpose as SpecPurpose>::names()
    }

    fn params_schema(&self) -> ParamSchema {
        ParamSchema {
            names: vec![
                "e0",
                "brass_alpha",
                "brass_beta",
                "tfr",
                "eps_f",
                "eps_m",
                "elite_fertility_mult",
            ],
        }
    }

    fn step(
        &self,
        binding: PulseBinding,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        match binding.pulse {
            Pulse::Day => drain(view, ctx),
            _ => self.mortality(view, ctx),
        }
    }

    fn fire(
        &self,
        timer: &TimerEntry,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let entity = timer.key.entity;
        if timer.timer.kind != DEATH_TIMER || entity.kind() != EntityKind::Person {
            return Err(fail(format!("{} is not a death timer", timer.id())));
        }
        let at = timer.key.time;
        if self.schedule.calendar().is_boundary(Pulse::Month, at) || at.minutes() % DAY != 0 {
            return Err(fail(format!(
                "{} fires at {at}, not a day start inside a month: INV-M2-03",
                timer.id()
            )));
        }
        // A death timer for a person already dead (a natural death drawn before a commanded one
        // applied earlier in the instant) is dropped and counted (05.1 §2 rule 2; R3 point 4).
        let person = PersonId::from_raw(entity.raw());
        if apply_death(view, ctx, person, CAUSE_NATURAL, EventRef::None)? == Death::AlreadyDead {
            ctx.drop_timer(timer, DropReason::Dead);
        }
        Ok(())
    }

    fn resample_timer(
        &self,
        timer: &TimerEntry,
        ctx: &ResampleCtx,
    ) -> Result<Option<SimTime>, SimError> {
        let t_in = ctx.now();
        let birth = timer.timer.payload;
        let end = self.schedule.month_end(t_in).map_err(Self::sched_err)?;
        let q = self
            .schedule
            .interval_q(t_in.minutes() - birth, end.minutes() - birth)
            .map_err(Self::sched_err)?;
        if !q.hits(ctx.draw(K_HIT)) {
            return Ok(None);
        }
        Ok(resample_day(t_in.minutes(), end.minutes(), ctx.draw(K_DAY)).map(SimTime::from_minutes))
    }

    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult> {
        invariants(world)
    }

    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

fn col<'t>(world: &'t WorldTables, name: &str) -> &'t [i64] {
    world.get(name).map(Vec::as_slice).unwrap_or_default()
}

fn result(name: &'static str, problem: Option<String>) -> InvariantResult {
    InvariantResult {
        name,
        holds: problem.is_none(),
        detail: problem.unwrap_or_default(),
    }
}

/// M2's invariants after `Bookkeeping`'s compaction: INV-M2-01 (the living count equals the open
/// life-course rows), INV-M2-02 (each ledger row has one open life-course row with its id and
/// birth tick; no closed row's person is in the ledger) and INV-M2-04 (every row alive with
/// `death_tick = i64::MAX`). INV-M2-03 needs the agenda and is checked by V-DET-12's tests.
#[must_use]
pub fn invariants(world: &WorldTables) -> Vec<InvariantResult> {
    let ids = col(world, PERSON_ID);
    let births = col(world, PERSON_BIRTH);
    let mut open: Vec<(u64, i64)> = Vec::new();
    let mut closed: Vec<u64> = Vec::new();
    for ((&p, &k), &b) in col(world, LC_PERSON)
        .iter()
        .zip(col(world, LC_EXIT_KIND))
        .zip(col(world, LC_BIRTH))
    {
        if k == 0 {
            open.push((p.cast_unsigned(), b));
        } else {
            closed.push(p.cast_unsigned());
        }
    }
    open.sort_unstable();
    let conservation = (open.len() != ids.len()).then(|| {
        format!(
            "{} living ledger rows but {} open life-course rows",
            ids.len(),
            open.len()
        )
    });
    let ledger: Vec<(u64, i64)> = ids
        .iter()
        .zip(births)
        .map(|(&i, &b)| (i.cast_unsigned(), b))
        .collect();
    let consistency = if ledger == open {
        closed
            .iter()
            .find(|&&p| ledger.binary_search_by_key(&p, |&(i, _)| i).is_ok())
            .map(|p| format!("person {p} has a closed life-course row and a ledger row"))
    } else {
        let row = ledger
            .iter()
            .zip(&open)
            .position(|(a, b)| a != b)
            .unwrap_or(ledger.len().min(open.len()));
        Some(format!(
            "ledger row {row} {:?} does not match the open life-course rows {:?}",
            ledger.get(row),
            open.get(row)
        ))
    };
    let agree = col(world, PERSON_ALIVE)
        .iter()
        .zip(col(world, PERSON_DEATH_TICK))
        .position(|(&a, &d)| a != 1 || d != i64::MAX)
        .map(|r| {
            format!("ledger row {r} is not alive with death_tick = i64::MAX after Bookkeeping")
        });
    vec![
        result("INV-M2-01", conservation),
        result("INV-M2-02", consistency),
        result("INV-M2-04", agree),
    ]
}
