//! `rh-sys-smoke`: the smoke world, the golden run's exercise of the whole engine (SPR-008 T-01).
//!
//! Plane A, kind `system`: the first system crate, under the same dependency rules as every other
//! (`bin/lint_arch.py`). Its seven systems are **dummies**: they model nothing, and exist so that a
//! 1,000-year run touches every part of the engine that determinism depends on (ruling 3):
//!
//! | System | Pulses and phases | Exercises |
//! |---|---|---|
//! | `X1.smoke_weather` | Year/Inputs, Hour/Environment | the Hour pulse; Q16.16 drift |
//! | `X2.smoke_harvest` | Season/Production, Month/Exchange | a `lagged` read; the `rh-sample` draw (Poisson, on `StepCtx::stream`) |
//! | `X3.smoke_vital` | Month/Vital | timers scheduled, fired, cancelled and re-sampled; `Settle` sent with a causal edge |
//! | `X4.smoke_estate` | Day/Activity, Day/Contagion | `Settle` applied in queue order; `INHERITANCE` caused by `X_DEATH` |
//! | `X5.smoke_polity` | Year/Polity, Month/Polity, Season/Military | two bindings of one system in one phase; `Settle` sent with no cause |
//! | `X6.smoke_chronicle` | Month/Agenda, Day/Culture, Day/Bookkeeping | a draft-local causal edge (`CENSUS_NOTE` after `CENSUS`) |
//! | `X7.smoke_market` | Day/Environment, Day/Activity | parallel levels (VER-SPR-008 DEF-2): it shares Environment with X1 and Activity with X4 every day, so those levels hold two jobs; it and X4 send each other commands (`Settle`, `Audit`) |
//!
//! Every phase of `05.1` §4.2 and every pulse is bound; reads are `same_instant` and `lagged`
//! (the other lags are BCK-002 A-28); every system has an invariant; every number is an integer or
//! Q16.16 bits, every draw is keyed. The material inputs that re-sample the timers come from
//! [`synthetic_inputs`]; the golden runner (`rh-golden`) journals each before submitting it.
//!
//! The dummy systems never enter the real Registry (ruling 2). Their specs are
//! `worlds/smoke/systems/*.odd.toml`, their ids are in `worlds/smoke/registry_overlay.toml`, and
//! each module `include!`s the skeleton `lint_odd.py --rust` generated from its spec
//! (`src/generated/`, kept identical by `bin/gen_skeletons.py --check`; ADR-0033 §3). The host
//! builds the spec table from those files (the crate has no I/O) and registers [`systems`] against
//! it, so registration compares this code with the specs (rule `spec`), as for real systems.
//!
//! REF: `CODEX/05_PROJECT/SPR-008_GoldenRunCI.md` T-01, rulings 1–3; `05.1` §2, §4; ADR-0025;
//! ADR-0033 §3.
#![forbid(unsafe_code)]

mod chronicle;
mod common;
mod estate;
mod harvest;
mod market;
mod polity;
mod vital;
mod weather;

use std::collections::BTreeSet;

use rh_core::engine::{Engine, MaterialInput};
use rh_core::ownership::SpecTable;
use rh_core::register::{RegistrationError, register};
use rh_core::system::SystemModel;
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::WorldTables;
use rh_ids::{BranchSalt, EntityRef};

pub use chronicle::Chronicle;
pub use common::{PERSONS, REGIONS, person, person_entity};
pub use estate::Estate;
pub use harvest::Harvest;
pub use market::Market;
pub use polity::Polity;
pub use vital::Vital;
pub use weather::Weather;

/// The smoke world's calendar: 365 days, four seasons.
pub const CALENDAR: Calendar = Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four);

/// The spec directory and Registry overlay of the smoke world, relative to the repository root.
pub const SPECS_DIR: &str = "worlds/smoke/systems";
/// See [`SPECS_DIR`].
pub const OVERLAY: &str = "worlds/smoke/registry_overlay.toml";

/// The seven dummy systems, unregistered.
#[must_use]
pub fn systems() -> Vec<Box<dyn SystemModel>> {
    vec![
        Box::new(Weather),
        Box::new(Harvest),
        Box::new(Vital),
        Box::new(Estate),
        Box::new(Polity),
        Box::new(Chronicle),
        Box::new(Market),
    ]
}

/// The initial tables (each spec's `[initialization]`).
#[must_use]
pub fn initial_tables() -> WorldTables {
    let mut t = WorldTables::new();
    for (column, rows) in [
        ("weather.temp", vec![common::degrees(10); REGIONS]),
        ("weather.hours", vec![0]),
        ("harvest.stock", vec![common::degrees(100); REGIONS]),
        ("harvest.last_yield", vec![0; REGIONS]),
        ("person.alive", vec![1; PERSONS]),
        ("person.pending", vec![0; PERSONS]),
        ("person.deaths", vec![0]),
        ("estate.heirs", vec![0; PERSONS]),
        ("estate.order", vec![0]),
        ("estate.gossip", vec![0]),
        ("polity.treasury", vec![0]),
        ("polity.decrees", vec![0]),
        ("chronicle.census", vec![0]),
        ("chronicle.days", vec![0]),
        ("chronicle.closed", vec![0]),
        ("market.price", vec![common::degrees(100); REGIONS]),
        ("market.ticks", vec![0]),
        ("market.audits", vec![0]),
    ] {
        t.insert(column, rows);
    }
    t
}

/// The smoke engine at instant 0: [`systems`] registered against `specs` (built by the host from
/// [`SPECS_DIR`] and [`OVERLAY`]), on [`initial_tables`], with `threads` workers.
///
/// # Errors
/// [`RegistrationError`] if the code and the specs disagree.
pub fn engine(specs: &SpecTable, seed: u64, threads: usize) -> Result<Engine, RegistrationError> {
    let registered = register(systems(), specs)?;
    let mut engine = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered,
        initial_tables(),
        SimTime::EPOCH,
    );
    engine.set_threads(threads);
    Ok(engine)
}

/// Years between two synthetic material inputs.
pub const INPUT_EVERY_YEARS: i64 = 7;

/// The synthetic material inputs of a run of `years` (ruling 3), as `(instant, entity)` pairs in
/// submission order: every [`INPUT_EVERY_YEARS`] years, on day 100 at 06:07 (between pulse
/// boundaries, so the input's instant is its own), eight persons chosen by the year, in entity
/// order. Each is one material input whose scope is that person: it cancels and re-samples the
/// person's pending death timers (ADR-0025 item 3). One pair is one journal row (SPR-007: a
/// journaled input has one entity in its scope, and each row becomes one engine input). A pure
/// function of `years`, and a run of fewer years gets a prefix of a longer run's inputs.
#[must_use]
pub fn synthetic_inputs(years: i64) -> Vec<(SimTime, EntityRef)> {
    (1..=years / INPUT_EVERY_YEARS)
        .flat_map(|k| {
            let year = k * INPUT_EVERY_YEARS;
            let minutes = year * CALENDAR.minutes_per_year() + 99 * 1_440 + 6 * 60 + 7;
            let first = usize::try_from(year).unwrap_or(0) % PERSONS;
            let scope: BTreeSet<EntityRef> = (0..8)
                .map(|i| person_entity((first + i * 5) % PERSONS))
                .collect();
            scope
                .into_iter()
                .map(move |e| (SimTime::from_minutes(minutes), e))
        })
        .collect()
}

/// The engine input of one synthetic input: a material input whose scope is `entity`.
#[must_use]
pub fn material(entity: EntityRef) -> MaterialInput {
    MaterialInput {
        scope: BTreeSet::from([entity]),
    }
}
