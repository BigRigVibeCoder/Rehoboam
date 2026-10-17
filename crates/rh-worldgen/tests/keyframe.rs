//! V-INV-11 (SPR-012 T-06): founders' negative birth ticks round-trip through a keyframe.
//!
//! Genesis writes pre-epoch birth ticks (negative minutes). A keyframe of the engine built on
//! them, at genesis and after some months with deaths, loads back to the same state, and the
//! loaded engine advances exactly as the continuous one.
//!
//! Refs: SPR-012 T-06; `docs/09` V-INV-11; SPR-007 T-02 (keyframes).
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken fixture must fail the test loudly"
)]

use std::path::{Path, PathBuf};

use rh_core::crash::Provenance;
use rh_core::engine::Engine;
use rh_core::fixed::Q16_16;
use rh_core::ownership::SpecTable;
use rh_core::register::{Registered, register};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_ids::BranchSalt;
use rh_registry::ownership::Ownership;
use rh_registry::{DemographyParams, Founder, FounderRole, WorldSpec};
use rh_snapshot::journal::{JournalPosition, chain_genesis};
use rh_snapshot::keyframe::{History, load_engine, write_keyframe};
use rh_snapshot::state_hash;
use rh_sys_demography::Demography;
use spec_files::{load_specs, registry_system_ids};

mod spec_files;

const D360: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn specs() -> SpecTable {
    let ids =
        registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).expect("registry");
    let specs = load_specs(&repo().join("specs/systems")).expect("specs");
    Ownership::build(&specs, &ids).spec_table()
}

fn spec(seed: u64, n: usize) -> WorldSpec {
    WorldSpec {
        seed,
        calendar: D360,
        horizon_years: 80,
        systems: Vec::new(),
        demography: Some(DemographyParams {
            e0: Q16_16::from_int(25),
            brass_beta: Q16_16::ONE,
        }),
        founders: vec![
            Founder {
                role: FounderRole::Commoner,
                sex: None,
                age_years: None,
            };
            n
        ],
    }
}

fn registered(spec: &WorldSpec, table: &SpecTable) -> Registered {
    let m2 = Demography::from_world(spec).unwrap().unwrap();
    register(vec![Box::new(m2)], table).unwrap_or_else(|e| panic!("{e}"))
}

fn history(seed: u64) -> History {
    History {
        provenance: Provenance {
            build: "test-build".to_owned(),
            registry_version: "0.2.0".to_owned(),
            manifest_hash: "worldgen".to_owned(),
        },
        world_seed: seed,
        branch_salt: 0,
    }
}

fn engine(spec: &WorldSpec, table: &SpecTable) -> Engine {
    let tables = rh_worldgen::genesis(spec, table).unwrap();
    let mut e = Engine::new(
        D360,
        spec.seed,
        BranchSalt::from_raw(0),
        registered(spec, table),
        tables,
        SimTime::EPOCH,
    );
    e.set_provenance(history(spec.seed).provenance);
    e
}

fn monthly(e: &mut Engine, from: i64, to: i64) -> Vec<u64> {
    (from..=to)
        .map(|m| {
            e.advance_until(SimTime::from_minutes(m * 43_200), |_, _| {})
                .unwrap_or_else(|err| panic!("{err}"));
            state_hash(&e.snapshot())
        })
        .collect()
}

#[test]
fn v_inv_11_negative_birth_ticks_round_trip_through_a_keyframe() {
    let table = specs();
    let position = JournalPosition {
        timeline_id: 0,
        inputs: 0,
        head: chain_genesis(0),
    };
    let mut cases = 0;
    for (seed, n, cut) in [(1, 1, 0), (7, 40, 0), (19, 60, 30), (23, 12, 5)] {
        let spec = spec(seed, n);
        let mut continuous = engine(&spec, &table);
        if cut > 0 {
            let _ = monthly(&mut continuous, 0, cut);
        }
        let births = continuous
            .tables()
            .get("person.birth_tick")
            .unwrap()
            .clone();
        assert!(
            births.iter().all(|&b| b < 0),
            "founders are born before the epoch"
        );
        let (bytes, _) = write_keyframe(&continuous, position).unwrap();
        let (mut loaded, _) =
            load_engine(&bytes, D360, registered(&spec, &table), &history(seed)).unwrap();
        assert_eq!(loaded.snapshot(), continuous.snapshot(), "seed {seed}");
        assert_eq!(loaded.tables().get("person.birth_tick").unwrap(), &births);
        assert_eq!(
            loaded.tables().get("life_course.birth_tick"),
            continuous.tables().get("life_course.birth_tick")
        );
        let end = cut + 24;
        assert_eq!(
            monthly(&mut loaded, cut + 1, end),
            monthly(&mut continuous, cut + 1, end),
            "seed {seed}: the loaded engine advances as the continuous one"
        );
        cases += 1;
    }
    println!("V-INV-11: negative birth ticks round-trip through a keyframe in {cases} worlds");
}
