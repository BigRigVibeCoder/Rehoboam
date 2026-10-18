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
use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, CommandApply, Lag, Plane};
use rh_core::agenda::TimerEntry;
use rh_core::command::QueuedCommand;
use rh_core::fixed::Q16_16;
use rh_core::system::{
    AggregateSpec, DropReason, InvariantResult, ParamSchema, PulseBinding, ResampleCtx, SimError,
    SpecPurpose, StepCtx, SystemDescriptor, SystemModel, TimerSpec,
};
use rh_core::time::{Calendar, Pulse, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_events::{EdgeKind, EventDraft, EventRef};
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

use skeleton::{DRAINS, ROLE_ACTOR, ROLE_TARGET, ROLES, RngPurpose, SCHEDULE, SPEC_VERSION, SYSTEM_ID};

impl SpecPurpose for RngPurpose {
    const ALL: &'static [Self] = &[Self::DeathTime, Self::Conception, Self::BirthSex];
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
/// See [`PERSON_ID`].
pub const PERSON_HOUSEHOLD: &str = "person.household";
/// See [`PERSON_ID`].
pub const PERSON_PARITY: &str = "person.parity";
/// See [`PERSON_ID`].
pub const PERSON_LAST_BIRTH_TICK: &str = "person.last_birth_tick";
/// See [`PERSON_ID`].
pub const PERSON_CONCEIVED_TICK: &str = "person.conceived_tick";
/// See [`PERSON_ID`].
pub const PERSON_UNBORN_FATHER: &str = "person.unborn_father";
/// See [`PERSON_ID`].
pub const PERSON_SEX: &str = "person.sex";
/// See [`PERSON_ID`].
pub const PERSON_MOTHER: &str = "person.mother";
/// See [`PERSON_ID`].
pub const PERSON_FATHER: &str = "person.father";
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
pub const OWNED: [&str; 33] = [
    "person.id",
    "person.birth_tick",
    "person.sex",
    "person.alive",
    "person.death_tick",
    "person.mother",
    "person.father",
    "person.spouse",
    "person.death_cause",
    "person.household",
    "person.parity",
    "person.last_birth_tick",
    "person.conceived_tick",
    "person.unborn_father",
    "household.id",
    "household.head",
    "household.formed_tick",
    "household.dissolved_tick",
    "marriage.husband",
    "marriage.wife",
    "marriage.start_tick",
    "marriage.end_tick",
    "marriage.source",
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
/// See [`DEATH`].
pub const BIRTH: &str = "BIRTH";

/// The birth timer's closure id (F7).
pub const BIRTH_TIMER: u32 = 2;

/// Fertility bands: 7 five-year bands 15-19 to 45-49.
pub const BANDS: usize = 7;
/// Coale-Trussell natural fertility n(a) per band, in thousandths (F1).
pub const N_I: [u16; BANDS] = [411, 460, 431, 395, 322, 167, 24];
/// Gestation in calendar months (F2).
pub const GESTATION_MONTHS: i64 = 9;
/// Postpartum non-susceptible period in calendar months (F2).
pub const POSTPARTUM_MONTHS: i64 = 11;
/// Closed period G + P in Month pulses (F2).
pub const CLOSED_PERIOD: i64 = 20;
/// Reference TFR of the nuptiality schedule at M=1, in Q16_16 bits (F3).
pub const TFR1_BITS: i32 = 615_614;
/// Minimum fertile age in years (F0).
pub const MIN_AGE_YEARS: i64 = 15;
/// Maximum fertile age in years (F0).
pub const MAX_AGE_YEARS: i64 = 50;
/// Band width in years (F0).
pub const BAND_WIDTH_YEARS: i64 = 5;
/// SEX_M = rhe(2^32 * 105 / 205) ≈ 0.5122 (F2).
pub const SEX_M_BITS: u32 = 2_199_861_298;
/// Sex codes: male = 0, female = 1.
pub const SEX_MALE: i64 = 0;
/// See [`SEX_MALE`].
pub const SEX_FEMALE: i64 = 1;
/// life_course.origin codes.
pub const ORIGIN_FOUNDER: i64 = 0;
/// See [`ORIGIN_FOUNDER`].
pub const ORIGIN_BIRTH: i64 = 1;
/// life_course.exit_kind open.
pub const EXIT_OPEN: i64 = 0;
/// life_course.birth_order (parity, 1-based): founder marker.
pub const BIRTH_ORDER_NONE: i64 = 0;

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
    /// The ordinary conception table c_i (F4), one Prob32 per band.
    c: [rh_core::fixed::Prob32; BANDS],
    /// The elite conception table c_i^e (F4).
    c_e: [rh_core::fixed::Prob32; BANDS],
}

impl Demography {
    /// M2 for e₀ on `calendar`, refusing `brass_alpha` (any `Some`) and β ≠ 1 (ruling 7).
    /// `tfr` and `elite_fertility_mult` are Q16_16 values from the manifest or Registry defaults.
    ///
    /// Computes the conception tables `c_i` and `c_i^e` at construction (F4).
    ///
    /// # Errors
    /// [`DemographyError`], each naming the refused parameter.
    pub fn new(
        e0: Q16_16,
        brass_alpha: Option<Q16_16>,
        brass_beta: Q16_16,
        tfr: Q16_16,
        elite_fertility_mult: Q16_16,
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
        let m = conception_level(tfr);
        let m_e = elite_level(m, elite_fertility_mult);
        let c = conception_table(m);
        let c_e = conception_table(m_e);
        Ok(Self {
            schedule,
            c,
            c_e,
        })
    }

    /// The schedule.
    #[must_use]
    pub const fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// The ordinary conception table c_i (F4).
    #[must_use]
    pub const fn conception(&self) -> &[rh_core::fixed::Prob32; BANDS] {
        &self.c
    }

    /// The elite conception table c_i^e (F4).
    #[must_use]
    pub const fn conception_elite(&self) -> &[rh_core::fixed::Prob32; BANDS] {
        &self.c_e
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

    /// F5-F7 at the Month pulse instant: scan eligible women, draw conception, schedule birth timer.
    fn fertility(&self, view: &mut SystemView<'_>, ctx: &mut StepCtx<'_>) -> Result<(), SimError> {
        let t_m = ctx.now();
        let t_m_minutes = t_m.minutes();
        let year = self.schedule.calendar().minutes_per_year();

        // F6: read house_member.person (lagged to start of instant; M7's column).
        let house_members = view.read("house_member.person").unwrap_or(&[]);

        // The calendar's month-end for the birth date (9 months ahead).
        let mut tau_month = t_m;
        for _ in 0..GESTATION_MONTHS {
            tau_month = self.schedule.calendar().next_boundary(
                Pulse::Month,
                SimTime::from_minutes(tau_month.minutes() + 1),
            ).ok_or_else(|| fail("calendar overflow in fertility".to_owned()))?;
        }
        let birth_month_end = self.schedule.month_end(
            SimTime::from_minutes(tau_month.minutes()),
        ).map_err(Self::sched_err)?;
        let tau_month_start = tau_month;
        let days_in_birth_month = (birth_month_end.minutes() - tau_month_start.minutes()) / DAY;

        // First pass: read-only, collect conceptions
        struct Hit {
            idx: usize,
            woman_id: u64,
            father: i64,
            tau_minutes: i64,
        }
        let mut hits: Vec<Hit> = Vec::new();

        {
            let ids = view.read(PERSON_ID)?;
            let births = view.read(PERSON_BIRTH)?;
            let alive = view.read(PERSON_ALIVE)?;
            let sex = view.read(PERSON_SEX)?;
            let spouses = view.read(PERSON_SPOUSE)?;
            let conceived = view.read(PERSON_CONCEIVED_TICK)?;

            for idx in 0..ids.len() {
                if alive[idx] != 1 {
                    continue;
                }
                if sex[idx] != SEX_FEMALE {
                    continue;
                }
                let spouse_raw = spouses[idx].cast_unsigned();
                if spouse_raw == 0 {
                    continue;
                }
                if conceived[idx] != i64::MAX {
                    continue;
                }
                let birth = births[idx];
                let Some(band) = fertility_band(t_m_minutes, birth, year) else {
                    continue;
                };

                let woman_id = ids[idx].cast_unsigned();
                let is_elite = house_members
                    .binary_search_by(|&v| v.cast_unsigned().cmp(&woman_id))
                    .is_ok()
                    || house_members
                        .binary_search_by(|&v| v.cast_unsigned().cmp(&spouse_raw))
                        .is_ok();
                let table = if is_elite { &self.c_e } else { &self.c };
                let c_i = table[band];

                let woman = PersonId::from_raw(woman_id);
                if !ctx.chance(&RngPurpose::Conception, woman, 0, c_i)? {
                    continue;
                }
                let d1 = ctx.rng(&RngPurpose::Conception, woman, 1)?;
                let day_offset = 1 + ((u128::from(d1) * u128::from((days_in_birth_month - 1).cast_unsigned())) >> 64) as u64;
                let tau = tau_month_start.minutes() + DAY * day_offset.cast_signed();

                hits.push(Hit {
                    idx,
                    woman_id,
                    father: spouses[idx],
                    tau_minutes: tau,
                });
            }
        }

        // Second pass: write conception state and schedule timers
        for hit in &hits {
            view.write(PERSON_CONCEIVED_TICK)?[hit.idx] = t_m_minutes;
            view.write(PERSON_UNBORN_FATHER)?[hit.idx] = hit.father;

            ctx.schedule(
                &RngPurpose::Conception,
                TimerSpec {
                    time: SimTime::from_minutes(hit.tau_minutes),
                    phase: Phase::Vital,
                    entity: EntityRef::person(PersonId::from_raw(hit.woman_id)),
                    kind: BIRTH_TIMER,
                    payload: 0, // placeholder
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

/// F3: compute the conception level M = tfr / TFR1 in Q16_16.
#[must_use]
fn conception_level(tfr: Q16_16) -> Q16_16 {
    // m = rhe(T * 65536 / TFR1_BITS) where T = tfr bits
    let t = tfr.to_bits();
    let num = i64::from(t) * i64::from(Q16_16::ONE.to_bits());
    let m = if num >= 0 {
        (num + i64::from(TFR1_BITS) / 2) / i64::from(TFR1_BITS)
    } else {
        (num - i64::from(TFR1_BITS) / 2) / i64::from(TFR1_BITS)
    };
    Q16_16::from_bits(m as i32)
}

/// F3: compute elite level m_e = rhe(m * E / 65536).
#[must_use]
fn elite_level(m: Q16_16, elite_fertility_mult: Q16_16) -> Q16_16 {
    let num = i64::from(m.to_bits()) * i64::from(elite_fertility_mult.to_bits());
    // rhe(x / 65536): half to even
    let half = i64::from(Q16_16::ONE.to_bits()) / 2;
    let denom = i64::from(Q16_16::ONE.to_bits());
    let result = (num + half) / denom;
    Q16_16::from_bits(result as i32)
}

/// F4: compute the conception table c_i for 7 bands at level M (Q16_16).
#[must_use]
fn conception_table(level: Q16_16) -> [rh_core::fixed::Prob32; BANDS] {
    use rh_core::fixed::Prob32;
    let mut c = [Prob32::ZERO; BANDS];
    let one: u64 = 1 << 32;
    for i in 0..BANDS {
        let n_i = u64::from(N_I[i]);
        // b_i = m * N_i (births per woman-year in units of 2^-16 / 1000)
        let m_bits = level.to_bits().cast_unsigned();
        let b_i = (u128::from(m_bits) * u128::from(n_i)) >> 16;
        // den_i = 12 * 65536 * 1000 - 20 * b_i
        let den_i: i64 = {
            let base: i64 = 12 * 65536 * 1000;
            let sub: i64 = (20_u128 * b_i) as i64;
            base - sub
        };
        if den_i <= 0 {
            c[i] = Prob32::from_bits(one).unwrap_or(Prob32::ONE);
        } else {
            let num: u128 = u128::from(b_i as u64) * u128::from(one);
            let den: u128 = den_i.cast_unsigned().into();
            let val = (num + den / 2) / den;
            let clamped = if val > u128::from(one) { one } else { val as u64 };
            c[i] = Prob32::from_bits(clamped).unwrap_or(Prob32::ONE);
        }
    }
    c
}

/// F0: woman's band index i in 0..6, or None when outside [15, 50).
fn fertility_band(t: i64, birth: i64, year_minutes: i64) -> Option<usize> {
    let x = t - birth;
    if x < 0 {
        return None;
    }
    let age_years = x / year_minutes;
    if age_years < MIN_AGE_YEARS || age_years >= MAX_AGE_YEARS {
        return None;
    }
    let i = ((age_years - MIN_AGE_YEARS) / BAND_WIDTH_YEARS) as usize;
    if i < BANDS { Some(i) } else { None }
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
/// `target`, the cause code as payload and a causal edge from `cause_event` when there is one,
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
    // person.spouse is cleared on the spouse's death (0.2.3 [[owns]]; the Tester's DEF-2).
    let spouse = view.read(PERSON_SPOUSE)?[row].cast_unsigned();
    if let Some(survivor) = ledger_row(view, spouse)?.filter(|_| spouse != 0) {
        let spouses = view.write(PERSON_SPOUSE)?;
        if spouses[survivor].cast_unsigned() == id {
            spouses[survivor] = 0;
        }
    }
    let mut draft = EventDraft::new(DEATH)
        .role(ROLE_TARGET, EntityRef::person(person))
        .with_payload(vec![cause]);
    if !cause_event.is_none() {
        draft = draft.caused_by(cause_event, EdgeKind::Triggered);
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
        .unwrap_or(EventRef::None);
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

/// Helper: issue a child PersonId from mother and parity, rehashing against the current ledger.
fn issue_child_id(mother: PersonId, parity: u32, person_ids: &[i64]) -> PersonId {
    use rh_ids::person::{child_base, rehash, MAX_REHASH};
    let base = child_base(mother, parity);
    let mut id = base;
    for c in 0..=MAX_REHASH {
        let raw = id.to_raw();
        if person_ids.binary_search_by_key(&raw, |v| v.cast_unsigned()).is_err() {
            return id;
        }
        if c == MAX_REHASH {
            break;
        }
        id = rehash(base, c + 1);
    }
    // If all 64 rehashes collide, return the base anyway (the engine will handle the collision).
    // In practice this is astronomically unlikely.
    base
}

/// F9: apply_birth — insert the newborn into the ledger, append life_course row,
/// emit BIRTH event. Called from fire when a BIRTH_TIMER is due.
///
/// The mother may already be dead (death_tick = now), but her row must be present.
#[must_use]
pub fn apply_birth(
    view: &mut SystemView<'_>,
    ctx: &mut StepCtx<'_>,
    mother: PersonId,
) -> Result<(), SimError> {
    let now = ctx.now().minutes();
    let mother_id = mother.to_raw();
    let mother_row = match ledger_row(view, mother_id)? {
        Some(row) => row,
        None => {
            return Err(fail(format!(
                "mother {mother_id} has no ledger row at birth {now}"
            )));
        }
    };

    // Read mother state
    let parity = view.read(PERSON_PARITY)?[mother_row];
    // parity is the count BEFORE this birth; new_parity is 1-based
    let parity_count = parity.cast_unsigned();

    // F9: child id = H64(mother, parity) with rehash against the ledger
    let child_id = issue_child_id(mother, parity_count as u32, view.read(PERSON_ID)?);

    // Draw sex (F9: BirthSex, k=0, keyed by child's own PersonId)
    let child_person = child_id;
    let draw = ctx.rng(&RngPurpose::BirthSex, child_person, 0)?;
    let sex = if draw < u64::from(SEX_M_BITS) << 32 { SEX_MALE } else { SEX_FEMALE };

    // Get father from mother's unborn_father
    let father = view.read(PERSON_UNBORN_FATHER)?[mother_row];

    // Insert new row into all person.* owned columns
    {
        let ids = view.write(PERSON_ID)?;
        let pos = ids
            .binary_search_by_key(&child_id.to_raw(), |v| v.cast_unsigned())
            .unwrap_err();
        ids.insert(pos, child_id.to_raw().cast_signed());
        view.write(PERSON_BIRTH)? .insert(pos, now);
        view.write(PERSON_SEX)? .insert(pos, sex);
        view.write(PERSON_ALIVE)? .insert(pos, 1);
        view.write(PERSON_DEATH_TICK)? .insert(pos, i64::MAX);
        view.write(PERSON_MOTHER)? .insert(pos, mother_id.cast_signed());
        view.write(PERSON_FATHER)? .insert(pos, father);
        view.write(PERSON_SPOUSE)? .insert(pos, 0);
        view.write(PERSON_DEATH_CAUSE)? .insert(pos, 0);
        view.write(PERSON_HOUSEHOLD)? .insert(pos, 0);
        view.write(PERSON_PARITY)? .insert(pos, 0);
        view.write(PERSON_LAST_BIRTH_TICK)? .insert(pos, i64::MAX);
        view.write(PERSON_CONCEIVED_TICK)? .insert(pos, i64::MAX);
        view.write(PERSON_UNBORN_FATHER)? .insert(pos, 0);
    }

    // Append life_course row
    {
        let origin = ORIGIN_BIRTH;
        let _birth_order = parity + 1;
        let lc_id = child_id.to_raw().cast_signed();
        view.write(LC_PERSON)?.push(lc_id);
        view.write("life_course.origin")?.push(origin);
        view.write("life_course.mother")?.push(mother_id.cast_signed());
        view.write("life_course.father")?.push(father);
        view.write("life_course.sex")?.push(sex);
        view.write(LC_BIRTH)?.push(now);
        view.write("life_course.entry_tick")?.push(now);
        view.write(LC_EXIT_KIND)?.push(EXIT_OPEN);
        view.write(LC_EXIT_TICK)?.push(i64::MAX);
        view.write(LC_DEATH_CAUSE)?.push(0);
    }

    // Update mother: clear conception, increment parity, record last birth
    view.write(PERSON_CONCEIVED_TICK)?[mother_row] = i64::MAX;
    view.write(PERSON_PARITY)?[mother_row] = parity + 1;
    view.write(PERSON_LAST_BIRTH_TICK)?[mother_row] = now;

    // Emit BIRTH event
    let child_entity = EntityRef::person(child_person);
    ctx.emit(
        EventDraft::new(BIRTH)
            .role(ROLE_ACTOR, EntityRef::person(mother))
            .role(ROLE_TARGET, child_entity)
            .with_payload(vec![sex, parity + 1]),
    )?;

    Ok(())
}

impl SystemModel for Demography {
    type Index = ();
    
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
            emits: vec![DEATH, EXTINCTION, BIRTH],
            // M2 0.3.0's reads (F1): house_member.person for elite fertility
            reads: vec![ColumnRead {
                column: "house_member.person",
                owner: "M7.dynasty",
                lag: Lag::Lagged,
            }],
            ..AccessSpec::default()
        }
    }

    fn roles(&self) -> &'static [(&'static str, rh_events::RoleId)] {
        ROLES
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
            Pulse::Month => self.fertility(view, ctx).and_then(|()| self.mortality(view, ctx)),
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
        if timer.timer.kind == BIRTH_TIMER && entity.kind() == EntityKind::Person {
            // F9-F10: BIRTH_TIMER fires at a day start in the 9th month
            let at = timer.key.time;
            if at.minutes() % DAY != 0 {
                return Err(fail(format!(
                    "{} fires at {at}, not a day start: INV-M2-11",
                    timer.id()
                )));
            }
            let person = PersonId::from_raw(entity.raw());
            apply_birth(view, ctx, person)?;
            return Ok(());
        }
        if timer.timer.kind != DEATH_TIMER || entity.kind() != EntityKind::Person {
            return Err(fail(format!("{} is not a death or birth timer", timer.id())));
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
        if timer.timer.kind == BIRTH_TIMER {
            // F8: the birth timer's month is the 9th after conception.
            // Re-draw a day start in the same month strictly after t_in.
            // The birth is owed: if no day start remains, the birth fires immediately next day.
            let month_end = self.schedule.month_end(t_in).map_err(Self::sched_err)?;
            let first = (t_in.minutes().div_euclid(DAY) + 1) * DAY;
            let remaining = if first < month_end.minutes() {
                (month_end.minutes() - first) / DAY
            } else {
                0
            };
            if remaining == 0 {
                return Ok(Some(SimTime::from_minutes(first)));
            }
            // Pick a day offset from the remaining
            let d1 = ctx.draw(K_DAY);
            let offset = ((u128::from(d1) * u128::from(remaining.cast_unsigned())) >> 64) as u64;
            return Ok(Some(SimTime::from_minutes(first + DAY * offset.cast_signed())));
        }
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
///
/// O(N + L) when they hold (SPR-038 T-03): one pass over the log, the open and the closed persons
/// sorted (linear on a log appended in id order, the common case) and merged with the ledger's
/// ascending ids. When any does not hold, [`invariants_oracle`] computes the results, so the
/// results, names and texts are always the oracle's.
#[must_use]
pub fn invariants(world: &WorldTables) -> Vec<InvariantResult> {
    if all_hold(world) {
        return vec![
            result("INV-M2-01", None),
            result("INV-M2-02", None),
            result("INV-M2-04", None),
        ];
    }
    invariants_oracle(world)
}

/// Whether INV-M2-01, 02 and 04 all hold, in O(N + L). It answers `true` only when
/// [`invariants_oracle`] finds all three holding: it asks for a strictly ascending ledger, which the
/// core's `ledger-sorted` invariant guarantees whenever M2's run, and otherwise defers to the
/// oracle.
fn all_hold(world: &WorldTables) -> bool {
    let ids = col(world, PERSON_ID);
    let births = col(world, PERSON_BIRTH);
    if ids.len() != births.len()
        || ids
            .windows(2)
            .any(|w| w[0].cast_unsigned() >= w[1].cast_unsigned())
    {
        return false;
    }
    let mut open: Vec<(u64, i64)> = Vec::with_capacity(ids.len());
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
    if open.len() != ids.len() {
        return false;
    }
    open.sort_unstable();
    // INV-M2-02: the ledger, in table order, equals the sorted open rows.
    let equal = ids
        .iter()
        .zip(births)
        .zip(&open)
        .all(|((&i, &b), &o)| (i.cast_unsigned(), b) == o);
    if !equal {
        return false;
    }
    // INV-M2-02: no closed row's person is in the (strictly ascending) ledger: one merge.
    closed.sort_unstable();
    let mut j = 0;
    for &p in &closed {
        while j < ids.len() && ids[j].cast_unsigned() < p {
            j += 1;
        }
        if ids.get(j).is_some_and(|&i| i.cast_unsigned() == p) {
            return false;
        }
    }
    // INV-M2-04.
    col(world, PERSON_ALIVE)
        .iter()
        .zip(col(world, PERSON_DEATH_TICK))
        .all(|(&a, &d)| a == 1 && d == i64::MAX)
}

/// M2's invariants as SPR-011 wrote them: the oracle [`invariants`] defers to whenever one does not
/// hold, and the reference its O(N + L) path is tested against (SPR-038 T-03).
#[must_use]
pub fn invariants_oracle(world: &WorldTables) -> Vec<InvariantResult> {
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
