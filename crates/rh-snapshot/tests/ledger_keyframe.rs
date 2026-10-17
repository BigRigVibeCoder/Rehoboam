//! V-DET-05 with ledger compaction inside the window (SPR-011 T-03, ruling 5): a keyframe written
//! from a world whose people die, loaded and advanced, matches the continuous run month by month,
//! and the people who die after the cut leave the ledger in both.
//!
//! The world is the core's ledger test world in small: `M2.mortal` owns `person.id`,
//! `person.alive` and the life-course log and kills by a keyed draw each month in `Vital`;
//! `T1.namer` owns `person.named`, so compaction crosses owners.
//!
//! Refs: SPR-011 T-03; `docs/09` V-DET-05; `crates/rh-core/src/ledger.rs`.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken stub world must fail the test loudly"
)]

mod support;

use rh_core::access::Lag;
use rh_core::crash::Provenance;
use rh_core::engine::Engine;
use rh_core::register::{Registered, register};
use rh_core::system::{PulseBinding, SimError, SpecPurpose, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_ids::{BranchSalt, PersonId};
use rh_snapshot::journal::{JournalPosition, chain_genesis};
use rh_snapshot::keyframe::{History, load_engine, write_keyframe};
use rh_snapshot::state_hash;
use support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const MONTH: i64 = 43_200;

enum Draw {
    Death,
}

impl SpecPurpose for Draw {
    const ALL: &'static [Self] = &[Self::Death];
    fn name(&self) -> &'static str {
        "death"
    }
}

fn mortal(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let ids = v.read("person.id")?.to_vec();
    let persons = v.read("life_course.person_id")?.to_vec();
    for (i, &id) in ids.iter().enumerate() {
        if c.rng(&Draw::Death, PersonId::from_raw(id.cast_unsigned()), 0)? % 30 == 0 {
            v.write("person.alive")?[i] = 0;
            let kinds = v.read("life_course.exit_kind")?.to_vec();
            let row = (0..persons.len())
                .find(|&r| persons[r] == id && kinds[r] == 0)
                .expect("an open entry");
            v.write("life_course.exit_kind")?[row] = 1;
            v.write("life_course.exit_tick")?[row] = c.now().minutes();
            v.write("life_course.death_cause")?[row] = 1;
        }
    }
    Ok(())
}

fn namer(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let ids = v.read("person.id")?.to_vec();
    let t = c.now().minutes();
    for (slot, id) in v.write("person.named")?.iter_mut().zip(ids) {
        *slot = (id ^ t).rem_euclid(1_000);
    }
    Ok(())
}

fn registered() -> Registered {
    let mut m = Stub::new("M2.mortal").purposes_of::<Draw>();
    for c in [
        "person.id",
        "person.alive",
        "life_course.person_id",
        "life_course.exit_kind",
        "life_course.exit_tick",
        "life_course.death_cause",
    ] {
        m = m.writes(c);
    }
    let stubs = vec![
        m.at(Pulse::Month, Phase::Vital).stepping(mortal),
        Stub::new("T1.namer")
            .writes("person.named")
            .reads("person.id", "M2.mortal", Lag::SameInstant)
            .at(Pulse::Month, Phase::Culture)
            .stepping(namer),
    ];
    let specs = agreeing_specs(&stubs);
    register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap_or_else(|e| panic!("{e}"))
}

fn history(seed: u64) -> History {
    History {
        provenance: Provenance {
            build: "test-build".to_owned(),
            registry_version: "0.2.0".to_owned(),
            manifest_hash: "ledger".to_owned(),
        },
        world_seed: seed,
        branch_salt: 0,
    }
}

fn engine(n: i64, seed: u64) -> Engine {
    let ids: Vec<i64> = (1..=n).map(|i| i * 1_000_003 + 17).collect();
    let rows = ids.len();
    let mut t = WorldTables::new();
    t.insert("person.id", ids.clone());
    t.insert("person.alive", vec![1; rows]);
    t.insert("person.named", vec![0; rows]);
    t.insert("life_course.person_id", ids);
    t.insert("life_course.exit_kind", vec![0; rows]);
    t.insert("life_course.exit_tick", vec![i64::MAX; rows]);
    t.insert("life_course.death_cause", vec![0; rows]);
    let mut e = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered(),
        t,
        SimTime::EPOCH,
    );
    e.set_provenance(history(seed).provenance);
    e
}

fn monthly(e: &mut Engine, from: i64, to: i64) -> Vec<u64> {
    (from..=to)
        .map(|m| {
            e.advance_until(SimTime::from_minutes(m * MONTH), |_, _| {})
                .unwrap_or_else(|err| panic!("{err}"));
            state_hash(&e.snapshot())
        })
        .collect()
}

#[test]
fn v_det_05_a_keyframe_round_trips_across_ledger_compactions() {
    for (n, seed, cut) in [(60, 3, 7), (200, 11, 30), (17, 5, 1)] {
        let mut continuous = engine(n, seed);
        let _ = monthly(&mut continuous, 0, cut);
        let living_at_cut = continuous.tables().get("person.id").unwrap().len();
        let position = JournalPosition {
            timeline_id: 0,
            inputs: 0,
            head: chain_genesis(0),
        };
        let (bytes, _) = write_keyframe(&continuous, position).unwrap();
        let (mut loaded, _) = load_engine(&bytes, CALENDAR, registered(), &history(seed)).unwrap();
        let end = cut + 12 * 20;
        let expected = monthly(&mut continuous, cut + 1, end);
        let got = monthly(&mut loaded, cut + 1, end);
        assert_eq!(got, expected, "n {n}, cut at month {cut}");
        assert_eq!(loaded.snapshot(), continuous.snapshot());
        let living_at_end = loaded.tables().get("person.id").unwrap().len();
        assert!(
            living_at_end < living_at_cut,
            "n {n}: deaths after the cut ({living_at_cut} -> {living_at_end})"
        );
    }
}
