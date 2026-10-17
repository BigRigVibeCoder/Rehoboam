//! V-DET-14: draws keyed by the drawn entity's epoch (SPR-011 T-02; BCK-002 A-29; ADR-0025;
//! `05.1` §6 item 2).
//!
//! A stub system draws once a day for every person and every region (`rng` and `stream`), and
//! writes the draws into columns. Two runs of one world differ only by material inputs with random
//! scopes at random instants. Then:
//! - an entity outside every scope draws bit-equal in both runs, at every instant;
//! - an entity inside a scope draws as before up to the input's instant and differently from it
//!   on;
//! - every draw equals an independent `H` at the entity's epoch (the number of inputs whose scope
//!   held it, as of the draw's instant), so an entity at epoch 0 draws exactly as in P0.
//!
//! World size, seed, scopes and instants vary (the DEF-5 lesson), from `SplitMix64` with fixed
//! seeds, so every run checks the same cases.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken stub world must fail the test loudly"
)]

mod support;

use std::collections::BTreeSet;

use rh_core::engine::{Engine, MaterialInput};
use rh_core::register::{register, system_id};
use rh_core::system::{PulseBinding, SimError, SpecPurpose, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_ids::{BranchSalt, EntityRef, PersonId, PurposeId, RegionId, RngKey, h};
use support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const DAYS: i64 = 40;
const SYSTEM: &str = "X1.drawer";

enum Draw {
    Person,
    Region,
}

impl SpecPurpose for Draw {
    const ALL: &'static [Self] = &[Self::Person, Self::Region];
    fn name(&self) -> &'static str {
        match self {
            Self::Person => "person_draw",
            Self::Region => "region_draw",
        }
    }
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// Each day: `p.draw[i]` = draw 0 of person `i` by `rng`, `r.draw[i]` = draw 2 of region `i`'s
/// stream. The rows are the entities, by raw id.
fn draw(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let persons = v.write("p.draw")?.len();
    let mut people = Vec::with_capacity(persons);
    for i in 0..persons {
        people.push(c.rng(&Draw::Person, PersonId::from_raw(i as u64), 0)?);
    }
    let regions = v.write("r.draw")?.len();
    let mut places = Vec::with_capacity(regions);
    for i in 0..regions {
        let mut s = c.stream(&Draw::Region, RegionId::from_raw(u16::try_from(i).unwrap()))?;
        places.push(s.draw(2));
    }
    for (slot, d) in v.write("p.draw")?.iter_mut().zip(people) {
        *slot = d.cast_signed();
    }
    for (slot, d) in v.write("r.draw")?.iter_mut().zip(places) {
        *slot = d.cast_signed();
    }
    Ok(())
}

fn engine(persons: usize, regions: usize, seed: u64) -> Engine {
    let stubs = vec![
        Stub::new(SYSTEM)
            .purposes_of::<Draw>()
            .writes("p.draw")
            .writes("r.draw")
            .at(Pulse::Day, Phase::Vital)
            .stepping(draw),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut t = WorldTables::new();
    t.insert("p.draw", vec![0; persons]);
    t.insert("r.draw", vec![0; regions]);
    Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered,
        t,
        SimTime::EPOCH,
    )
}

/// The draws of every day: `(p.draw, r.draw)` after each day's instant.
fn run(e: &mut Engine) -> Vec<(Vec<i64>, Vec<i64>)> {
    (0..DAYS)
        .map(|d| {
            e.advance_until(SimTime::from_minutes(d * DAY), |_, _| {})
                .unwrap_or_else(|err| panic!("{err}"));
            let t = e.tables();
            (
                t.get("p.draw").unwrap().clone(),
                t.get("r.draw").unwrap().clone(),
            )
        })
        .collect()
}

/// The independent `H` of one draw (`05.1` §6 item 2), at the given epoch.
fn expected(seed: u64, purpose: &str, entity: u64, epoch: u32, day: i64, k: u64) -> i64 {
    h(
        &RngKey {
            world_seed: seed,
            branch_salt: BranchSalt::from_raw(0),
            system: system_id(SYSTEM),
            purpose: PurposeId::of(purpose),
            entity,
            epoch,
            t: day * DAY,
        },
        k,
    )
    .cast_signed()
}

#[test]
fn v_det_14_only_the_scoped_entities_draw_differently_and_only_from_the_input_on() {
    let mut rng = SplitMix64(0x5EED_0E0C);
    let mut checked_inside = 0;
    for (persons, regions) in [(1, 1), (7, 3), (40, 12), (150, 30)] {
        for seed in [1, 99, 0xDEAD_BEEF] {
            // 1 to 3 inputs, each scoping a random subset, at a random minute (a day start or not).
            let inputs: Vec<(i64, BTreeSet<EntityRef>)> = (0..=rng.below(3))
                .map(|_| {
                    let at = 1 + i64::try_from(rng.below(u64::try_from(DAYS * DAY - 2).unwrap()))
                        .unwrap();
                    let mut scope = BTreeSet::new();
                    for i in 0..persons {
                        if rng.below(3) == 0 {
                            scope.insert(EntityRef::person(PersonId::from_raw(i as u64)));
                        }
                    }
                    for i in 0..regions {
                        if rng.below(4) == 0 {
                            let r = RegionId::from_raw(u16::try_from(i).unwrap());
                            scope.insert(EntityRef::region(r));
                        }
                    }
                    (at, scope)
                })
                .collect();
            let base = run(&mut engine(persons, regions, seed));
            let mut touched = engine(persons, regions, seed);
            for (at, scope) in &inputs {
                touched
                    .submit_input(
                        SimTime::from_minutes(*at),
                        MaterialInput {
                            scope: scope.clone(),
                        },
                    )
                    .unwrap();
            }
            let got = run(&mut touched);
            // The epoch of an entity as of `day`'s draw: inputs at or before that instant.
            let epoch = |e: EntityRef, day: i64| -> u32 {
                let n = inputs
                    .iter()
                    .filter(|(at, scope)| *at <= day * DAY && scope.contains(&e))
                    .count();
                u32::try_from(n).unwrap()
            };
            for day in 0..DAYS {
                let (bp, br) = &base[usize::try_from(day).unwrap()];
                let (gp, gr) = &got[usize::try_from(day).unwrap()];
                for i in 0..persons {
                    let e = EntityRef::person(PersonId::from_raw(i as u64));
                    let n = epoch(e, day);
                    assert_eq!(bp[i], expected(seed, "person_draw", i as u64, 0, day, 0));
                    assert_eq!(gp[i], expected(seed, "person_draw", i as u64, n, day, 0));
                    assert_eq!(gp[i] == bp[i], n == 0, "person {i}, day {day}, epoch {n}");
                    checked_inside += usize::from(n > 0);
                }
                for i in 0..regions {
                    let e = EntityRef::region(RegionId::from_raw(u16::try_from(i).unwrap()));
                    let n = epoch(e, day);
                    assert_eq!(br[i], expected(seed, "region_draw", i as u64, 0, day, 2));
                    assert_eq!(gr[i], expected(seed, "region_draw", i as u64, n, day, 2));
                    assert_eq!(gr[i] == br[i], n == 0, "region {i}, day {day}, epoch {n}");
                }
            }
        }
    }
    assert!(
        checked_inside > 1_000,
        "scoped draws checked: {checked_inside}"
    );
}

#[test]
fn an_input_at_a_day_start_changes_that_days_draw() {
    // The input applies in Inputs, before the Vital draw of the same instant (ADR-0025).
    let mut base = engine(3, 1, 5);
    let base = run(&mut base);
    let mut touched = engine(3, 1, 5);
    let scope = BTreeSet::from([EntityRef::person(PersonId::from_raw(1))]);
    touched
        .submit_input(SimTime::from_minutes(10 * DAY), MaterialInput { scope })
        .unwrap();
    let got = run(&mut touched);
    assert_eq!(got[9], base[9]);
    assert_ne!(got[10].0[1], base[10].0[1]);
    assert_eq!(got[10].0[0], base[10].0[0]);
    assert_eq!(got[10].0[2], base[10].0[2]);
}
