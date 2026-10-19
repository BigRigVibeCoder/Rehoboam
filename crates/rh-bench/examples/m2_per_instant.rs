//! The per-instant wall time of real M2 alone, the setup of SPR-014's code review (finding K5):
//! release, M2 only, 3 sim-years (1,081 instants) on genesis worlds of 100, 1,000 and 10,000
//! founders. Ungated evidence for SPR-038's before/after record; it uses only APIs the merge base
//! has, so the same file runs on both.
//!
//! Run: `cargo run --release -p rh-bench --example m2_per_instant` (median of 3 per size).
//!
//! REF: `CODEX/05_PROJECT/SPR-038_PerInstantCost.md` T-04; the Architect's K5 note.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    reason = "a tool-plane example timing the engine from outside; plane A never reads this clock"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "an example: a broken setup stops loudly"
)]

use std::path::Path;
use std::time::Instant;

use rh_core::engine::Engine;
use rh_core::fixed::Q16_16;
use rh_core::register::register;
use rh_core::system::SystemModel;
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_ids::BranchSalt;
use rh_registry::ownership::Ownership;
use rh_registry::{DemographyParams, Founder, FounderRole, WorldSpec};
use rh_sys_demography::Demography;

const D360: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);

fn engine(n: usize) -> Engine {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ids =
        rh_manifest::specs::registry_system_ids(&repo.join("specs/manifest/registry_v0.toml"))
            .unwrap();
    let specs = rh_manifest::specs::load_specs(&repo.join("specs/systems")).unwrap();
    let table = Ownership::build(&specs, &ids).spec_table();
    let spec = WorldSpec {
        seed: 14,
        calendar: D360,
        horizon_years: 3,
        systems: Vec::new(),
        demography: Some(DemographyParams {
            e0: Q16_16::from_int(30),
            brass_beta: Q16_16::ONE,
            tfr: Q16_16::from_bits(321_126), // 4.9 in Q16_16
            elite_fertility_mult: Q16_16::ONE,
        }),
        founders: vec![
            Founder {
                role: FounderRole::Commoner,
                sex: None,
                age_years: None,
            };
            n
        ],
        households: Vec::new(),
    };
    let tables = rh_worldgen::genesis(&spec, &table).unwrap();
    let m2: Box<dyn SystemModel<Index = ()>> =
        Box::new(Demography::from_world(&spec).unwrap().unwrap());
    let registered = register(vec![m2], &table).unwrap();
    Engine::new(
        D360,
        spec.seed,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    )
}

fn main() {
    let end = SimTime::from_minutes(3 * D360.minutes_per_year());
    for n in [100, 1_000, 10_000] {
        let mut runs: Vec<(u128, u64)> = (0..3)
            .map(|_| {
                let mut e = engine(n);
                let mut instants = 0_u64;
                let start = Instant::now();
                e.advance_until(end, |_, _| instants += 1).unwrap();
                (start.elapsed().as_nanos(), instants)
            })
            .collect();
        runs.sort_unstable();
        let (ns, instants) = runs[1];
        let per = ns / u128::from(instants);
        println!(
            "m2_per_instant: {n} founders, {instants} instants over 3 sim-years: {}.{} us per instant (median of 3)",
            per / 1_000,
            per % 1_000 / 100
        );
    }
}
