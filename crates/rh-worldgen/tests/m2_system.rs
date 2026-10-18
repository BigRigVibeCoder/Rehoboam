//! The system `M2.demography`, rung-0 slice (SPR-012 T-07), on worlds `rh-worldgen` builds.
//!
//! These tests live here because a system crate may depend only on the engine's own crates
//! (V-INSP-03): `rh-sys-demography` cannot dev-depend on `rh-worldgen`, while `rh-worldgen` may
//! dev-depend on the system it initialises.
//!
//! - Registration rule `spec` passes against the frozen spec; `brass_alpha` and β ≠ 1 are refused
//!   at construction with a named error (ruling 7).
//! - V-DET-12: every event's tick is its instant, ids strictly increase, and a sampled death sits
//!   in the agenda as a timer, on a day start inside its month (INV-M2-03), until it fires.
//! - V-INV-01 subset after every instant on random small worlds (size, seed and e₀ varied).
//! - EXTINCTION once, caused by the last DEATH (ruling 17); the run continues to its end.
//! - A material input re-draws a pending death at the new epoch, on a later day start or none.
//! - The 10,000-founder fixture (release only): q(15) inside the schedule's binomial 99.9% band
//!   for each of 32 seeds; V-DATA-04 subset over 1,000 years.
//!
//! Refs: SPR-012 T-07; `docs/09` V-DET-12, V-INV-01, V-DATA-04; ADR-0025.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken fixture must fail the test loudly"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rh_core::engine::{Engine, MaterialInput};
use rh_core::fixed::Q16_16;
use rh_core::ownership::SpecTable;
use rh_core::register::register;
use rh_core::system::SystemModel;
use rh_core::time::{Calendar, DaysPerYear, Pulse, SeasonsPerYear, SimTime};
use rh_events::Event;
use rh_ids::BranchSalt;
use rh_registry::ownership::Ownership;
use rh_registry::{DemographyParams, Founder, FounderRole, Sex, WorldSpec};
use rh_sys_demography::demography::{DEATH, EXTINCTION};
use rh_sys_demography::{Demography, DemographyError};
use spec_files::{load_specs, registry_system_ids, registry_table};

mod spec_files;

const D360: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const D365: Calendar = Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four);
const DAY: i64 = 1_440;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn specs() -> SpecTable {
    let ids =
        registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).expect("registry");
    let specs = load_specs(&repo().join("specs/systems")).expect("specs");
    let table = registry_table(&repo().join("specs/manifest/registry_v0.toml")).expect("registry");
    Ownership::build(&specs, &ids).spec_table_with(table)
}

fn world(seed: u64, calendar: Calendar, e0: i16, founders: Vec<Founder>) -> WorldSpec {
    WorldSpec {
        seed,
        calendar,
        horizon_years: 80,
        systems: Vec::new(),
        demography: Some(DemographyParams {
            e0: Q16_16::from_int(e0),
            brass_beta: Q16_16::ONE,
            tfr: Q16_16::from_bits(321126), // 4.9 in Q16_16
            elite_fertility_mult: Q16_16::ONE,
        }),
        founders,
    }
}

fn engine(spec: &WorldSpec, table: &SpecTable, threads: usize) -> Engine {
    let tables = rh_worldgen::genesis(spec, table).unwrap();
    let m2: Box<dyn SystemModel<Index = ()>> = Box::new(Demography::from_world(spec).unwrap().unwrap());
    let registered = register(vec![m2], table).unwrap_or_else(|e| panic!("{e}"));
    let mut e = Engine::new(
        spec.calendar,
        spec.seed,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    e.set_threads(threads);
    e
}

struct Mix(u64);
impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        u64::try_from((u128::from(self.next()) * u128::from(n)) >> 64).unwrap()
    }
}

fn random_world(rng: &mut Mix) -> WorldSpec {
    let n = 1 + rng.below(40);
    let founders = (0..n)
        .map(|_| Founder {
            role: FounderRole::Commoner,
            sex: None,
            age_years: (rng.below(2) == 0).then(|| u8::try_from(rng.below(95)).unwrap()),
        })
        .collect();
    let calendar = if rng.below(2) == 0 { D360 } else { D365 };
    let e0 = 20 + i16::try_from(rng.below(66)).unwrap();
    world(rng.next(), calendar, e0, founders)
}

fn years(calendar: Calendar, n: i64) -> SimTime {
    SimTime::from_minutes(n * calendar.minutes_per_year())
}

#[test]
fn registration_against_the_frozen_spec_passes() {
    let spec = world(1, D360, 30, Vec::new());
    let m2: Box<dyn SystemModel<Index = ()>> = Box::new(Demography::from_world(&spec).unwrap().unwrap());
    let registered = register(vec![m2], &specs()).unwrap_or_else(|e| panic!("{e}"));
    let _ = registered;
}

#[test]
fn alpha_and_beta_other_than_one_are_refused_by_name() {
    let thirty = Q16_16::from_int(30);
    let e = Demography::new(thirty, Some(Q16_16::ZERO), Q16_16::ONE, Q16_16::from_bits(321126), Q16_16::ONE, D360).unwrap_err();
    assert_eq!(e, DemographyError::AlphaRefused { bits: 0 });
    assert!(e.to_string().contains("refuses brass_alpha"), "{e}");
    let beta = Q16_16::from_bits(65_537);
    let e = Demography::new(thirty, None, beta, Q16_16::from_bits(321126), Q16_16::ONE, D360).unwrap_err();
    assert_eq!(e, DemographyError::BetaNotOne { bits: 65_537 });
    assert!(e.to_string().contains("refuses brass_beta"), "{e}");
    let e = Demography::new(Q16_16::from_int(19), None, Q16_16::ONE, Q16_16::from_bits(321126), Q16_16::ONE, D360).unwrap_err();
    assert!(e.to_string().contains("outside the Registry bounds"), "{e}");
    assert!(Demography::new(thirty, None, Q16_16::ONE, Q16_16::from_bits(321126), Q16_16::ONE, D360).is_ok());
}

/// Runs `e` one instant at a time to `end`, checking V-DET-12, INV-M2-03 and V-INV-01 after each.
/// Returns every event.
fn run_checked(e: &mut Engine, calendar: Calendar, end: SimTime) -> Vec<Event> {
    let mut all: Vec<Event> = Vec::new();
    let mut pending: BTreeMap<u64, i64> = BTreeMap::new();
    while let Some(t) = e.next_instant() {
        if t > end {
            break;
        }
        e.advance_until(t, |_, _| {})
            .unwrap_or_else(|err| panic!("{t}: {err}"));
        let events = e.drain_events();
        for ev in &events {
            assert_eq!(ev.tick, t.minutes(), "V-DET-12: an event off its instant");
            if let Some(last) = all.last() {
                assert!(ev.id > last.id, "V-DET-12: event ids do not increase");
            }
            if ev.kind == DEATH {
                // SPR-020 T-03: a natural DEATH casts the deceased as target and no one else,
                // and carries no snapshot and no edge (envelope_v0.md §3).
                assert_eq!(ev.roles.len(), 1, "a natural DEATH's cast");
                assert_eq!(ev.roles[0].role, rh_events::RoleId::TARGET);
                assert!(ev.snapshots.is_empty() && ev.causes.is_empty());
                let person = ev.roles[0].entity.raw();
                assert_eq!(
                    pending.remove(&person),
                    Some(t.minutes()),
                    "V-DET-12: a death at {t} that was not a pending timer at that instant"
                );
            }
            all.push(ev.clone());
        }
        // Every pending death timer: on a day start inside its month, never a Month instant.
        for (key, _) in e.agenda().pending() {
            let at = key.time;
            assert_eq!(at.minutes() % DAY, 0, "INV-M2-03: {at} not a day start");
            assert!(
                !calendar.is_boundary(Pulse::Month, at),
                "INV-M2-03: {at} a Month instant"
            );
            let month = calendar.floor(Pulse::Month, at).unwrap();
            assert!(t >= month, "a timer pending beyond its month at {t}");
            pending.insert(key.entity.raw(), at.minutes());
        }
        for (system, inv) in e.invariants() {
            assert!(inv.holds, "{t}: {system} {}: {}", inv.name, inv.detail);
        }
    }
    all
}

#[test]
fn v_det_12_and_v_inv_01_on_random_small_worlds() {
    let mut rng = Mix(12);
    let table = specs();
    let mut deaths = 0;
    let mut worlds = 0;
    for _ in 0..24 {
        let spec = random_world(&mut rng);
        let n = spec.founders.len();
        let mut e = engine(&spec, &table, 1);
        let events = run_checked(&mut e, spec.calendar, years(spec.calendar, 130));
        let died = events.iter().filter(|ev| ev.kind == DEATH).count();
        assert_eq!(died, n, "everyone dies by 130 years (l(100) = 0)");
        let ext: Vec<&Event> = events.iter().filter(|ev| ev.kind == EXTINCTION).collect();
        assert_eq!(ext.len(), 1, "EXTINCTION once");
        let last_death = events.iter().rev().find(|ev| ev.kind == DEATH).unwrap();
        assert_eq!(
            ext[0]
                .causes
                .iter()
                .map(|c| (c.cause, c.kind))
                .collect::<Vec<_>>(),
            vec![(last_death.id, rh_events::EdgeKind::Triggered)],
            "EXTINCTION caused by the last DEATH"
        );
        assert!(ext[0].roles.is_empty(), "EXTINCTION has no cast");
        assert_eq!(e.tables().get("person.id").unwrap().len(), 0);
        deaths += died;
        worlds += 1;
    }
    println!(
        "V-DET-12, V-INV-01: {worlds} random worlds, {deaths} deaths, every event at its instant, every death a pending day-start timer"
    );
}

#[test]
fn the_run_continues_after_extinction_and_is_thread_invariant() {
    let spec = world(
        3,
        D360,
        25,
        vec![
            Founder {
                role: FounderRole::Cast,
                sex: Some(Sex::Female),
                age_years: Some(80),
            };
            3
        ],
    );
    let table = specs();
    let run = |threads| {
        let mut e = engine(&spec, &table, threads);
        let mut hashes = Vec::new();
        e.advance_until(years(D360, 60), |t, tables| {
            hashes.push((t, tables.clone()));
        })
        .unwrap();
        (hashes, e.drain_events())
    };
    let one = run(1);
    assert_eq!(run(4), one);
    assert_eq!(one.1.iter().filter(|ev| ev.kind == EXTINCTION).count(), 1);
    let last = one.0.last().unwrap().0;
    assert!(
        last >= years(D360, 59),
        "the run stops at its end, not at extinction"
    );
}

#[test]
fn a_material_input_redraws_a_pending_death_at_the_new_epoch() {
    // Old founders at e0 = 20: deaths are frequent, so inputs land on pending timers.
    let founders = vec![
        Founder {
            role: FounderRole::Commoner,
            sex: None,
            age_years: Some(85),
        };
        30
    ];
    let spec = world(41, D365, 20, founders);
    let table = specs();
    let mut base = engine(&spec, &table, 1);
    // Run Month pulse by Month pulse until a death is pending at least two days out.
    let mut pending = Vec::new();
    while pending.is_empty() {
        let t = base.next_instant().unwrap();
        base.advance_until(t, |_, _| {}).unwrap();
        pending = base
            .agenda()
            .pending()
            .into_iter()
            .filter(|(k, _)| k.time.minutes() >= t.minutes() + 2 * DAY)
            .collect();
        assert!(t < years(D365, 20), "no pending death to re-sample");
    }
    let (key, _) = pending[0];
    let t_in = SimTime::from_minutes(key.time.minutes() - DAY / 2 - 3);
    let mut with_input = engine(&spec, &table, 1);
    with_input
        .submit_input(
            t_in,
            MaterialInput {
                scope: BTreeSet::from([key.entity]),
            },
        )
        .unwrap();
    with_input.advance_until(t_in, |_, _| {}).unwrap();
    assert_eq!(with_input.epoch(key.entity), 1);
    let redrawn: Vec<_> = with_input
        .agenda()
        .pending()
        .into_iter()
        .filter(|(k, _)| k.entity == key.entity)
        .collect();
    assert!(redrawn.len() <= 1);
    if let Some((k, _)) = redrawn.first() {
        assert_eq!(k.time.minutes() % DAY, 0, "a re-drawn death on a day start");
        assert!(k.time > t_in, "after the input");
        let end = D365.next_boundary(Pulse::Month, t_in).unwrap();
        assert!(k.time < end, "inside the pending month");
    }
    // Entities outside the scope keep their timers.
    let others = |e: &Engine| -> Vec<(i64, u64)> {
        e.agenda()
            .pending()
            .into_iter()
            .filter(|(k, _)| k.entity != key.entity)
            .map(|(k, _)| (k.time.minutes(), k.entity.raw()))
            .collect()
    };
    base.advance_until(t_in, |_, _| {}).unwrap();
    assert_eq!(others(&with_input), others(&base));
}

/// The fixture of T-07: `n` founders aged 0 at e₀ = 30 on the 360-day calendar.
fn newborns(seed: u64, n: usize) -> WorldSpec {
    world(
        seed,
        D360,
        30,
        vec![
            Founder {
                role: FounderRole::Commoner,
                sex: None,
                age_years: Some(0),
            };
            n
        ],
    )
}

/// The 10,000-founder q(15) check of T-07 (deviation 1 and MAJ-1 of the Architect's review,
/// SPR-012).
///
/// **The null hypothesis.** The engine's deaths follow the fixed-point schedule. Founders "aged 0"
/// are born at a keyed offset inside the year before tick 0 (ruling 10), so founder `i` is `x_i`
/// in [0, 1) years old at genesis and can only die from there: under the null, its probability of
/// dying before age 15 is `p_i = 1 - l(15) / l(x_i)`, not the plain `1 - l(15)` (which ignores the
/// infancy already survived). The deaths by age 15 of one seed are then Poisson-binomial with mean
/// `sum p_i` and variance `sum p_i (1 - p_i)`.
///
/// **Two checks, both at z = 3.290527 (two-sided 99.9 %).**
/// - Per seed: each of the 32 seeds' counts within the band. Under the null the family-wise false
///   alarm rate is about 1 - 0.999^32 = 3.2 %.
/// - Pooled: the sum of the 32 counts against the sum of the 32 means, with the summed variance
///   (the seeds are independent). It has the power the per-seed check lacks: an engine at
///   e0 = 30.25 instead of 30 passes every seed but sits at a pooled z of about -5.7 (the review's
///   mutant runs). The pooled z is printed in the gate's pass line.
/// - **The pooled check is one-sided in practice** (VER-SPR-012-TEST GAP-9; BCK-002 A-53). On these
///   fixed seeds the correct engine's pooled z is about -2.0, one realisation of the null. An error
///   that kills fewer people than the schedule (z moves down) fails once it shifts the pooled count
///   by about 1.3 sd; one that kills more (z moves up) needs about 5.3 sd. So the check catches a
///   too-high e0 (or a too-low hazard) at small sizes and a too-low e0 only at large ones. That is
///   accepted rather than buying symmetry with more seeds: M2's fixed-point schedule is bit-equal
///   to the exact reference (`schedule_golden`), and the differential golden of M2's draws
///   (`draws_golden`) pins every death instant, so this check guards the wiring of the
///   Bernoulli to the schedule, which either direction of error breaks at a size the band sees.
///
/// **Fixed seeds.** The 32 world seeds are 1000 to 1031, written here, never drawn: the test is
/// one fixed experiment whose outcome (and false-alarm odds) the review can reproduce, and a
/// failure names a seed that replays.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release only: 32 x 10,000 founders; bin/check_all.sh runs it with --release"
)]
#[allow(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    reason = "test-side band arithmetic on the schedule's exact integers; never plane-A state"
)]
fn q15_of_10000_newborns_is_inside_the_binomial_band_for_32_seeds() {
    const N: usize = 10_000;
    const Z: f64 = 3.290_527;
    let table = specs();
    let year = D360.minutes_per_year();
    let m2 = Demography::new(Q16_16::from_int(30), None, Q16_16::ONE, Q16_16::from_bits(321126), Q16_16::ONE, D360).unwrap();
    let l15 = m2.schedule().survivors_at(15 * year).unwrap() as f64;
    let results: Vec<(u64, usize, f64, f64)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..32_u64)
            .map(|seed| {
                let (table, m2) = (&table, &m2);
                s.spawn(move || {
                    let spec = newborns(1_000 + seed, N);
                    let mut e = engine(&spec, table, 1);
                    let (mut mean, mut var) = (0.0, 0.0);
                    for &b in e.tables().get("person.birth_tick").unwrap() {
                        let lx = m2.schedule().survivors_at(-b).unwrap() as f64;
                        let p = 1.0 - l15 / lx;
                        mean += p;
                        var += p * (1.0 - p);
                    }
                    e.advance_until(SimTime::from_minutes(16 * year), |_, _| {})
                        .unwrap();
                    let t = e.tables();
                    let births = t.get("life_course.birth_tick").unwrap();
                    let exits = t.get("life_course.exit_tick").unwrap();
                    let dead = births
                        .iter()
                        .zip(exits)
                        .filter(|&(&b, &x)| x != i64::MAX && x - b < 15 * year)
                        .count();
                    (seed, dead, mean, var.sqrt())
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut worst: f64 = 0.0;
    let (mut dead_all, mut mean_all, mut var_all) = (0_usize, 0.0, 0.0);
    for &(seed, dead, mean, sd) in &results {
        let z = (dead as f64 - mean) / sd;
        worst = worst.max(z.abs());
        assert!(
            z.abs() <= Z,
            "seed {seed}: {dead} of {N} dead by 15, expected {mean:.1} +- {:.1}",
            Z * sd
        );
        dead_all += dead;
        mean_all += mean;
        var_all += sd * sd;
    }
    let pooled = (dead_all as f64 - mean_all) / var_all.sqrt();
    assert!(
        pooled.abs() <= Z,
        "pooled over 32 seeds: {dead_all} dead by 15, expected {mean_all:.1} +- {:.1} (z = {pooled:.2})",
        Z * var_all.sqrt()
    );
    println!(
        "T-07 q(15): 32 seeds x {N} founders aged 0 (born in the year before tick 0), each observed q(15) inside the schedule's binomial 99.9 % band (worst |z| = {worst:.2}); pooled z = {pooled:.2}, inside +-3.29"
    );
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release only: 10,000 founders over 1,000 years; bin/check_all.sh runs it with --release"
)]
fn v_data_04_every_person_has_one_founder_entry_and_a_death_iff_absent() {
    const N: usize = 10_000;
    let table = specs();
    let spec = newborns(7, N);
    let mut e = engine(&spec, &table, 2);
    let mut emitted: BTreeSet<u64> = BTreeSet::new();
    let end = years(D360, 1_000);
    let step = years(D360, 50);
    let mut at = SimTime::EPOCH;
    while at < end {
        at = SimTime::from_minutes((at.minutes() + step.minutes()).min(end.minutes()));
        e.advance_until(at, |_, _| {}).unwrap();
        for ev in e.drain_events() {
            emitted.extend(ev.roles.iter().map(|r| r.entity.raw()));
        }
        let t = e.tables();
        let ledger: BTreeSet<u64> = t
            .get("person.id")
            .unwrap()
            .iter()
            .map(|v| v.cast_unsigned())
            .collect();
        let lc = t.get("life_course.person_id").unwrap();
        let origin = t.get("life_course.origin").unwrap();
        let kind = t.get("life_course.exit_kind").unwrap();
        let mut entries: BTreeMap<u64, usize> = BTreeMap::new();
        for (i, &p) in lc.iter().enumerate() {
            assert_eq!(origin[i], 1, "founder provenance");
            *entries.entry(p.cast_unsigned()).or_default() += 1;
            let dead = kind[i] == 1;
            assert_eq!(
                dead,
                !ledger.contains(&p.cast_unsigned()),
                "death record iff absent"
            );
        }
        for p in &emitted {
            assert_eq!(
                entries.get(p),
                Some(&1),
                "person {p}: exactly one life-course entry"
            );
        }
        assert_eq!(entries.len(), N);
    }
    assert!(e.tables().get("person.id").unwrap().is_empty());
    println!(
        "V-DATA-04: {} persons emitted over 1,000 years, each with one founder entry and a death record iff absent from the ledger",
        emitted.len()
    );
}
