//! The differential golden of M2's draws (the Architect's ruling on the Tester's interim report of
//! SPR-012): the engine's death instants on fixture worlds equal an independent model's.
//!
//! `tests/golden/m2_draws_v1.txt` is written by `bin/gen_m2_draws_reference.py`, which models
//! `[algorithm.mortality_rung0]` S8–S12 on the keyed-RNG reference of `bin/gen_rng_golden.py` and the
//! schedule reference of `specs/demography/selftest_m2_rung0.py`, never on this crate. For each
//! world it lists the founders (id, birth tick), the material inputs in submission order (chosen to
//! re-sample pending deaths mid-month, at the timer's own instant, on the month's last day where no
//! day start is left, twice at one instant, and to bump the epoch of a person with no pending death)
//! and every death instant. This test builds the same ledger, registers `M2.demography` against the
//! frozen spec, submits the same inputs and requires the same deaths, so the wiring of every draw
//! (the monthly Bernoulli k = 0, the day k = 1, the re-sample's k = 0 and k = 1, the epochs) is
//! pinned.
//!
//! Controls (`bin/check_all.sh`): the features `mutant-resample-hit-k1` and `mutant-resample-day-k0`
//! each move one of `resample_timer`'s draws to the other k, and each must FAIL here;
//! `RH_M2_DRAWS_GOLDEN` names another golden file (one altered death line must FAIL).
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a malformed golden file or a broken engine must fail the test loudly"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use rh_core::engine::{Engine, MaterialInput};
use rh_core::fixed::Q16_16;
use rh_core::register::register;
use rh_core::system::SystemModel;
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::WorldTables;
use rh_ids::{BranchSalt, EntityRef, PersonId};
use rh_registry::ownership::Ownership;
use rh_sys_demography::Demography;
use rh_sys_demography::demography::DEATH;
use spec_files::{load_specs, registry_system_ids};

mod spec_files;

/// The committed golden file, or the one `RH_M2_DRAWS_GOLDEN` names (the altered-line control).
static GOLDEN: LazyLock<String> = LazyLock::new(load);

// LINT-ALLOW(forbidden-api): a test reads an environment override for its golden file; tests never link into the engine (SPR-011 A-4)
#[allow(
    clippy::disallowed_methods,
    reason = "a test reads an environment override for its golden file"
)]
fn load() -> String {
    match std::env::var_os("RH_M2_DRAWS_GOLDEN") {
        Some(path) => std::fs::read_to_string(path).expect("the override file is readable"),
        None => include_str!("golden/m2_draws_v1.txt").to_owned(),
    }
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One fixture world as the golden file states it.
#[derive(Default)]
struct World {
    name: String,
    seed: u64,
    days: u32,
    e0_bits: i32,
    years: i64,
    persons: Vec<(u64, i64)>,
    inputs: Vec<(i64, u64)>,
    deaths: Vec<(i64, u64)>,
    resamples: usize,
}

fn num<T: std::str::FromStr>(s: &str) -> T {
    s.parse().unwrap_or_else(|_| panic!("not a number: {s}"))
}

fn worlds() -> Vec<World> {
    let mut out: Vec<World> = Vec::new();
    for line in GOLDEN.lines().filter(|l| !l.starts_with('#')) {
        let w: Vec<&str> = line.split(' ').collect();
        match w.as_slice() {
            ["world", name, seed, days, e0, years] => out.push(World {
                name: (*name).to_owned(),
                seed: num(seed),
                days: num(days),
                e0_bits: num(e0),
                years: num(years),
                ..World::default()
            }),
            ["person", id, birth] => out.last_mut().unwrap().persons.push((num(id), num(birth))),
            ["input", t, p] => out.last_mut().unwrap().inputs.push((num(t), num(p))),
            ["death", t, p] => out.last_mut().unwrap().deaths.push((num(t), num(p))),
            ["resample", ..] => out.last_mut().unwrap().resamples += 1,
            _ => panic!("unexpected golden line `{line}`"),
        }
    }
    out
}

/// M2's columns for founders `persons` (ascending id), as genesis writes them.
fn tables(persons: &[(u64, i64)]) -> WorldTables {
    let n = persons.len();
    let ids: Vec<i64> = persons.iter().map(|&(p, _)| p.cast_signed()).collect();
    let births: Vec<i64> = persons.iter().map(|&(_, b)| b).collect();
    let mut t = WorldTables::new();
    for (column, rows) in [
        ("person.id", ids.clone()),
        ("person.birth_tick", births.clone()),
        ("person.sex", vec![1; n]),
        ("person.alive", vec![1; n]),
        ("person.death_tick", vec![i64::MAX; n]),
        ("person.mother", vec![0; n]),
        ("person.father", vec![0; n]),
        ("person.spouse", vec![0; n]),
        ("person.death_cause", vec![0; n]),
        ("life_course.person_id", ids),
        ("life_course.origin", vec![1; n]),
        ("life_course.mother", vec![0; n]),
        ("life_course.father", vec![0; n]),
        ("life_course.sex", vec![1; n]),
        ("life_course.birth_tick", births),
        ("life_course.entry_tick", vec![0; n]),
        ("life_course.exit_kind", vec![0; n]),
        ("life_course.exit_tick", vec![i64::MAX; n]),
        ("life_course.death_cause", vec![0; n]),
    ] {
        t.insert(column, rows);
    }
    t
}

fn run(w: &World) -> Vec<(i64, u64)> {
    let ids = registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).unwrap();
    let specs = Ownership::build(&load_specs(&repo().join("specs/systems")).unwrap(), &ids);
    let calendar = match w.days {
        360 => Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four),
        365 => Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four),
        d => panic!("{d} days a year"),
    };
    let m2 = Demography::new(Q16_16::from_bits(w.e0_bits), None, Q16_16::ONE, calendar).unwrap();
    let registered = register(
        vec![Box::new(m2) as Box<dyn SystemModel>],
        &specs.spec_table(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let mut persons = w.persons.clone();
    persons.sort_unstable();
    let mut e = Engine::new(
        calendar,
        w.seed,
        BranchSalt::from_raw(0),
        registered,
        tables(&persons),
        SimTime::EPOCH,
    );
    for &(t, p) in &w.inputs {
        let scope = BTreeSet::from([EntityRef::person(PersonId::from_raw(p))]);
        e.submit_input(SimTime::from_minutes(t), MaterialInput { scope })
            .unwrap();
    }
    let end = SimTime::from_minutes(w.years * calendar.minutes_per_year() - 1);
    e.advance_until(end, |_, _| {})
        .unwrap_or_else(|err| panic!("{}: {err}", w.name));
    let mut deaths: Vec<(i64, u64)> = e
        .drain_events()
        .into_iter()
        .filter(|ev| ev.kind == DEATH)
        .map(|ev| (ev.tick, ev.subjects[0].raw()))
        .collect();
    deaths.sort_unstable();
    deaths
}

#[test]
fn the_engines_death_instants_equal_the_independent_model() {
    let worlds = worlds();
    assert!(worlds.len() >= 6, "only {} worlds", worlds.len());
    let (mut deaths, mut inputs, mut resamples) = (0, 0, 0);
    for w in &worlds {
        let got = run(w);
        if let Some(i) = got.iter().zip(&w.deaths).position(|(a, b)| a != b) {
            panic!(
                "{}: death {i} differs: engine {:?}, model {:?}",
                w.name, got[i], w.deaths[i]
            );
        }
        assert_eq!(got.len(), w.deaths.len(), "{}: death count", w.name);
        deaths += got.len();
        inputs += w.inputs.len();
        resamples += w.resamples;
    }
    println!(
        "M2 draws: {deaths} death instants in {} worlds equal the independent model's, with {inputs} inputs and {resamples} re-samples",
        worlds.len()
    );
}
