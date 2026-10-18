//! The world of the per-instant cost bench (`benches/instant.rs`; SPR-038 T-01, BCK-002 A-60).
//!
//! One stub system, [`SYSTEM`], owns the 19 columns M2 0.2.3 owns (`rh_sys_demography`'s
//! `OWNED`: nine `person.*`, ten `life_course.*`), so the core's ledger exists and is compacted and
//! checked as in `solo`, and its `invariants()` are M2's own. It is bound like M2:
//! - `Day`/`Vital`, **drain-only**: it drains the command [`COMMAND`], which nothing sends, so its
//!   inbox is always empty: the binding SPR-014 made fire at every day start;
//! - `Month`/`Vital`: it rewrites one `person.spouse` cell with its own value (so a held year has a
//!   change to check each month) and, in [`Mode::Death`] and [`Mode::Insert`], schedules one timer
//!   at the month's third day start;
//! - the timer pass (`Day`/`Vital` is its first `Vital` binding): a [`DEATH_TIMER`] closes one
//!   person's ledger row and life-course entry as M2's death does, and a [`BIRTH_TIMER`] inserts
//!   one row (`StepCtx::insert_person`) and appends its open life-course entry.
//!
//! The world holds `n` living persons (ids `l - n .. l`, every one with an open life-course entry,
//! in id order) and `l - n` dead ones (ids `0 .. l - n`, closed entries before the open ones), so
//! the life-course log has `l` rows, ascending by person as a real log appended in birth order is.
//!
//! The fixture uses only `rh-core`'s public API, so it builds on any head since SPR-014.
//!
//! REF: `CODEX/05_PROJECT/SPR-038_PerInstantCost.md` T-01 (the specs table's benches row);
//! `crates/rh-sys-demography/src/demography.rs` (the columns, `apply_death`'s writes).

use rh_core::access::{AccessSpec, ColumnWrite, CommandApply, Plane};
use rh_core::engine::Engine;
use rh_core::ownership::{Declaration, Source, SpecApply, SpecEntry, SpecTable, single_writer};
use rh_core::register::{RegistrationError, register};
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, SimError, SpecPurpose, StepCtx,
    SystemDescriptor, SystemModel, TimerSpec,
};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::EventRef;
use rh_ids::{BranchSalt, EntityRef, PersonId};
use rh_sys_demography::demography::{
    LC_BIRTH, LC_DEATH_CAUSE, LC_EXIT_KIND, LC_EXIT_TICK, LC_PERSON, OWNED, PERSON_ALIVE,
    PERSON_BIRTH, PERSON_DEATH_CAUSE, PERSON_DEATH_TICK, PERSON_ID, PERSON_SPOUSE,
};

/// The bench system's id.
pub const SYSTEM: &str = "B1.ledger";
/// The command its drain-only `Day`/`Vital` binding drains (nothing sends it).
pub const COMMAND: &str = "BenchTouch";
/// One day, in minutes.
pub const DAY: i64 = 1_440;
/// The closure id of the death timer.
pub const DEATH_TIMER: u32 = 1;
/// The closure id of the birth timer.
pub const BIRTH_TIMER: u32 = 2;
/// The day of the month the timers fire at, and the quiet instant measured (a day start that is
/// not a month start, with no timer due in [`Mode::Quiet`]).
pub const MEASURED_DAY: i64 = 2;
/// The days of the held year.
pub const YEAR_DAYS: i64 = 360;
const SEED: u64 = 38;
const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);

/// What the bench system's `Month`/`Vital` step schedules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Nothing: every day start that is not a month start is a quiet instant.
    Quiet,
    /// A death at day [`MEASURED_DAY`] of each month: the middle living person.
    Death,
    /// A birth at day [`MEASURED_DAY`] of each month: one new person, the next id.
    Insert,
}

enum Draw {
    Timers,
}

impl SpecPurpose for Draw {
    const ALL: &'static [Self] = &[Self::Timers];
    fn name(&self) -> &'static str {
        "bench_timers"
    }
}

const DAY_VITAL: PulseBinding = PulseBinding {
    pulse: Pulse::Day,
    phase: Phase::Vital,
};
const MONTH_VITAL: PulseBinding = PulseBinding {
    pulse: Pulse::Month,
    phase: Phase::Vital,
};

struct Ledger(Mode);

fn access() -> AccessSpec {
    AccessSpec {
        writes: OWNED
            .map(|column| ColumnWrite {
                column,
                plane: Plane::A,
            })
            .to_vec(),
        applies: vec![CommandApply::new(COMMAND, Pulse::Day, Phase::Vital)],
        drain_only: vec![DAY_VITAL],
        ..AccessSpec::default()
    }
}

fn fail(message: String) -> SimError {
    SimError::System {
        system: SYSTEM,
        message,
    }
}

/// Closes the ledger row `row` at `t`, as M2's death does: `person.alive`, `death_tick` and
/// `death_cause`, then the open life-course entry's exit fields.
fn die(view: &mut SystemView<'_>, row: usize, t: i64) -> Result<(), SimError> {
    let person = view.read(PERSON_ID)?[row];
    view.write(PERSON_ALIVE)?[row] = 0;
    view.write(PERSON_DEATH_TICK)?[row] = t;
    view.write(PERSON_DEATH_CAUSE)?[row] = 1;
    let entry = view
        .read(LC_PERSON)?
        .iter()
        .zip(view.read(LC_EXIT_KIND)?)
        .position(|(&p, &k)| p == person && k == 0)
        .ok_or_else(|| fail(format!("person {person} has no open entry")))?;
    view.write(LC_EXIT_KIND)?[entry] = 1;
    view.write(LC_EXIT_TICK)?[entry] = t;
    view.write(LC_DEATH_CAUSE)?[entry] = 1;
    Ok(())
}

/// Inserts the next person, born at `t`: a ledger row through the core and an open entry.
fn birth(view: &mut SystemView<'_>, ctx: &mut StepCtx<'_>, t: i64) -> Result<(), SimError> {
    let next = view.read(LC_PERSON)?.iter().copied().max().unwrap_or(-1) + 1;
    let row: Vec<(&'static str, i64)> = OWNED
        .iter()
        .filter(|c| c.starts_with("person."))
        .map(|&c| (c, person_value(c, next, t)))
        .collect();
    ctx.insert_person(row);
    for &c in OWNED.iter().filter(|c| c.starts_with("life_course.")) {
        let value = entry_value(c, next, t);
        view.write(c)?.push(value);
    }
    Ok(())
}

/// A living person's value in `person.*` column `c`.
fn person_value(c: &str, id: i64, birth: i64) -> i64 {
    match c {
        PERSON_ID => id,
        PERSON_ALIVE => 1,
        PERSON_BIRTH => birth,
        PERSON_DEATH_TICK => i64::MAX,
        PERSON_SPOUSE | "person.mother" | "person.father" => -1,
        _ => 0,
    }
}

/// An open life-course entry's value in column `c`.
fn entry_value(c: &str, id: i64, birth: i64) -> i64 {
    match c {
        LC_PERSON => id,
        LC_BIRTH | "life_course.entry_tick" => birth,
        LC_EXIT_TICK => i64::MAX,
        "life_course.mother" | "life_course.father" => -1,
        _ => 0,
    }
}

impl SystemModel for Ledger {
    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: SYSTEM,
            fidelity: "F1",
            spec_version: "0.0.0",
        }
    }
    fn access(&self) -> AccessSpec {
        access()
    }
    fn schedule(&self) -> Vec<PulseBinding> {
        vec![MONTH_VITAL, DAY_VITAL]
    }
    fn rng_purposes(&self) -> Vec<&'static str> {
        Draw::names()
    }
    fn params_schema(&self) -> ParamSchema {
        ParamSchema::default()
    }
    fn step(
        &self,
        binding: PulseBinding,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        if binding == DAY_VITAL {
            // The drain: nothing sends the command, so there is nothing to take.
            // S16's positive control: a planted allocation in the quiet path.
            #[cfg(feature = "instant-alloc-control")]
            std::hint::black_box(Vec::<u64>::with_capacity(8));
            return match ctx.take_commands().first() {
                Some(q) => Err(fail(format!("an unexpected {} command", q.command.kind))),
                None => Ok(()),
            };
        }
        // A change to check each month: one cell rewritten with its own value.
        if let Some(&v) = view.read(PERSON_SPOUSE)?.first() {
            view.write(PERSON_SPOUSE)?[0] = v;
        }
        let kind = match self.0 {
            Mode::Quiet => return Ok(()),
            Mode::Death => DEATH_TIMER,
            Mode::Insert => BIRTH_TIMER,
        };
        let ids = view.read(PERSON_ID)?;
        let person = ids.get(ids.len() / 2).copied().unwrap_or(0);
        ctx.schedule(
            &Draw::Timers,
            TimerSpec {
                time: SimTime::from_minutes(ctx.now().minutes() + MEASURED_DAY * DAY),
                phase: Phase::Vital,
                entity: EntityRef::person(PersonId::from_raw(person.cast_unsigned())),
                kind,
                payload: 0,
                cause: EventRef::None,
            },
        )
    }
    fn fire(
        &self,
        timer: &rh_core::agenda::TimerEntry,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let t = ctx.now().minutes();
        match timer.timer.kind {
            DEATH_TIMER => {
                let person = timer.key.entity.raw().cast_signed();
                let row = view
                    .read(PERSON_ID)?
                    .binary_search(&person)
                    .map_err(|_| fail(format!("person {person} has no ledger row")))?;
                die(view, row, t)
            }
            BIRTH_TIMER => birth(view, ctx, t),
            kind => Err(fail(format!("an unknown timer kind {kind}"))),
        }
    }
    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult> {
        rh_sys_demography::demography::invariants(world)
    }
    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}

fn specs() -> SpecTable {
    let declaration: Declaration = access().declaration(SYSTEM);
    let entry = SpecEntry {
        owns: declaration.writes.clone(),
        rng: Draw::names().into_iter().map(str::to_owned).collect(),
        schedule: vec![
            ("Month".to_owned(), "Vital".to_owned()),
            ("Day".to_owned(), "Vital".to_owned()),
        ],
        applies: vec![SpecApply {
            command: COMMAND.to_owned(),
            drained_by: ("Day".to_owned(), "Vital".to_owned()),
            key: "any".to_owned(),
            key_field: "person".to_owned(),
            payload: Some(Vec::new()),
            outcome: false,
            payload_malformed: None,
        }],
        ..SpecEntry::default()
    };
    let (owners, _) = single_writer(std::slice::from_ref(&declaration), Source::Spec);
    SpecTable {
        owners,
        specs: [(SYSTEM.to_owned(), entry)].into_iter().collect(),
        ..SpecTable::default()
    }
}

/// The genesis tables: `n` living persons and `l - n` dead ones (module documentation).
///
/// # Panics
/// Never: `n <= l` is checked by [`engine`].
#[must_use]
pub fn tables(n: usize, l: usize) -> WorldTables {
    let dead = l - n;
    let id = |i: usize| i64::try_from(i).unwrap_or(i64::MAX);
    let birth = |i: usize| -(id(i % 20_000) + 1) * DAY;
    let mut t = WorldTables::new();
    for &c in OWNED.iter().filter(|c| c.starts_with("person.")) {
        t.insert(
            c,
            (dead..l)
                .map(|i| person_value(c, id(i), birth(i)))
                .collect(),
        );
    }
    for &c in OWNED.iter().filter(|c| c.starts_with("life_course.")) {
        let rows = (0..l).map(|i| {
            let open = entry_value(c, id(i), birth(i));
            if i >= dead {
                return open;
            }
            match c {
                LC_EXIT_KIND | LC_DEATH_CAUSE => 1,
                LC_EXIT_TICK => birth(i) / 2,
                _ => open,
            }
        });
        t.insert(c, rows.collect());
    }
    t
}

/// The bench engine at genesis: `n` living persons, a life-course log of `l` rows, the system in
/// `mode`. Its first instant is day 0, a month start.
///
/// # Errors
/// The registration error, as text, or `n > l`.
pub fn engine(mode: Mode, n: usize, l: usize) -> Result<Engine, String> {
    if n > l {
        return Err(format!(
            "{n} living persons need at least {n} log rows, not {l}"
        ));
    }
    let registered = register(vec![Box::new(Ledger(mode))], &specs())
        .map_err(|e: RegistrationError| e.to_string())?;
    Ok(Engine::new(
        CALENDAR,
        SEED,
        BranchSalt::from_raw(0),
        registered,
        tables(n, l),
        SimTime::EPOCH,
    ))
}

/// The last minute of day `day`: `advance_until` of it runs every instant through that day.
#[must_use]
pub const fn end_of_day(day: i64) -> SimTime {
    SimTime::from_minutes((day + 1) * DAY - 1)
}

/// The engine of a measured instant: built as [`engine`], then run through day
/// `MEASURED_DAY - 1`, so its next instant is day [`MEASURED_DAY`]'s start, the measured one.
///
/// # Errors
/// As [`engine`], or the run's error as text.
pub fn before_measured(mode: Mode, n: usize, l: usize) -> Result<Engine, String> {
    let mut e = engine(mode, n, l)?;
    e.advance_until(end_of_day(MEASURED_DAY - 1), |_, _| {})
        .map_err(|err| err.to_string())?;
    Ok(e)
}
