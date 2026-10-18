//! The worldgen stub's contract (SPR-012 T-06; SPR-011 ruling 10).
//!
//! - V-DATA-05: every founder id is `founder(seed, k)` or its deterministic rehash, in ruling-10
//!   order; a forced collision resolves the same way on every run.
//! - The same `WorldSpec` gives bit-equal tables twice, and on several threads at once.
//! - `solo` gives Ada: female, aged 25 at tick 0.
//! - A write to a column whose owner does not initialise from the manifest is refused.
//! - Properties over random worlds (size and seed varied): ledger sorted by id, life-course rows
//!   aligned and open, ages inside their role's range, birth ticks before the epoch.
//!
//! Refs: SPR-012 T-06; `docs/09` V-DATA-05; ADR-0026 item 1.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken fixture must fail the test loudly"
)]

use std::path::{Path, PathBuf};

use rh_core::ownership::SpecTable;
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear};
use rh_core::world::WorldTables;
use rh_ids::person::{PersonIdTable, founder_base, rehash};
use rh_registry::ownership::Ownership;
use rh_registry::{Founder, FounderRole, Sex, WorldSpec};
use rh_worldgen::{GenesisError, SEX_FEMALE, founder_ids, genesis, genesis_with};
use spec_files::{load_specs, registry_system_ids, registry_table};

mod spec_files;

const D360: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const D365: Calendar = Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four);

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The real spec table: `specs/systems` against the Registry's system ids.
fn specs() -> SpecTable {
    let ids =
        registry_system_ids(&repo().join("specs/manifest/registry_v0.toml")).expect("registry");
    let specs = load_specs(&repo().join("specs/systems")).expect("specs");
    let table = registry_table(&repo().join("specs/manifest/registry_v0.toml")).expect("registry");
    Ownership::build(&specs, &ids).spec_table_with(table)
}

fn world(seed: u64, calendar: Calendar, founders: Vec<Founder>) -> WorldSpec {
    WorldSpec {
        seed,
        calendar,
        horizon_years: 80,
        systems: Vec::new(),
        demography: None,
        founders,
    }
}

/// `solo`'s compiled founders: Ada, a cast member, female, 25.
fn solo() -> WorldSpec {
    world(
        1,
        D360,
        vec![Founder {
            role: FounderRole::Cast,
            sex: Some(Sex::Female),
            age_years: Some(25),
        }],
    )
}

fn col<'t>(t: &'t WorldTables, name: &str) -> &'t [i64] {
    t.get(name).unwrap_or_else(|| panic!("no column {name}"))
}

/// A `SplitMix64` for the property cases.
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
    let n = 1 + rng.below(120);
    let founders = (0..n)
        .map(|_| Founder {
            role: [
                FounderRole::Cast,
                FounderRole::Child,
                FounderRole::Servant,
                FounderRole::Commoner,
            ][usize::try_from(rng.below(4)).unwrap()],
            sex: [None, Some(Sex::Female), Some(Sex::Male)][usize::try_from(rng.below(3)).unwrap()],
            age_years: (rng.below(2) == 0).then(|| u8::try_from(rng.below(90)).unwrap()),
        })
        .collect();
    let calendar = if rng.below(2) == 0 { D360 } else { D365 };
    world(rng.next(), calendar, founders)
}

#[test]
fn v_data_05_founder_ids_follow_ruling_10_order() {
    let mut rng = Mix(5);
    for _ in 0..40 {
        let spec = random_world(&mut rng);
        let ids = founder_ids(spec.seed, spec.founders.len(), &mut PersonIdTable::new()).unwrap();
        for (k, id) in ids.iter().enumerate() {
            assert_eq!(*id, founder_base(spec.seed, k as u64), "founder {k}");
        }
        let t = genesis(&spec, &specs()).unwrap();
        let mut sorted: Vec<i64> = ids.iter().map(|i| i.to_raw().cast_signed()).collect();
        sorted.sort_by_key(|v| v.cast_unsigned());
        assert_eq!(col(&t, "person.id"), sorted.as_slice());
        assert_eq!(col(&t, "life_course.person_id"), sorted.as_slice());
    }
    println!("V-DATA-05: 40 random worlds, every founder id is founder(seed, k) in genesis order");
}

#[test]
fn v_data_05_a_forced_collision_rehashes_the_same_way_every_run() {
    let spec = random_world(&mut Mix(77));
    let base0 = founder_base(spec.seed, 0);
    let base2 = founder_base(spec.seed, 2);
    let run = || {
        // Founder 0's base is taken, and so is founder 2's base and its first rehash.
        let mut table = PersonIdTable::with_issued([base0, base2, rehash(base2, 1)]);
        let ids = founder_ids(spec.seed, spec.founders.len().max(3), &mut table).unwrap();
        let mut table = PersonIdTable::with_issued([base0, base2, rehash(base2, 1)]);
        (ids, genesis_with(&spec, &specs(), &mut table).unwrap())
    };
    let (ids, tables) = run();
    assert_eq!(ids[0], rehash(base0, 1));
    assert_eq!(ids[1], founder_base(spec.seed, 1));
    assert_eq!(ids[2], rehash(base2, 2));
    assert_eq!(run(), (ids, tables));
    println!("V-DATA-05: a forced collision resolves to the same rehash on every run");
}

#[test]
fn the_same_spec_gives_bit_equal_tables_twice_and_across_threads() {
    let mut rng = Mix(11);
    let specs = specs();
    for _ in 0..12 {
        let spec = random_world(&mut rng);
        let once = genesis(&spec, &specs).unwrap();
        assert_eq!(genesis(&spec, &specs).unwrap(), once);
        let threads: Vec<WorldTables> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..4)
                .map(|_| s.spawn(|| genesis(&spec, &specs).unwrap()))
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        assert!(threads.iter().all(|t| *t == once));
    }
}

#[test]
fn solo_gives_ada_female_aged_25_at_tick_0() {
    let t = genesis(&solo(), &specs()).unwrap();
    assert_eq!(col(&t, "person.id").len(), 1);
    assert_eq!(
        col(&t, "person.id")[0].cast_unsigned(),
        founder_base(1, 0).to_raw()
    );
    assert_eq!(col(&t, "person.sex"), [SEX_FEMALE]);
    let birth = col(&t, "person.birth_tick")[0];
    let year = D360.minutes_per_year();
    assert!(
        -birth >= 25 * year && -birth < 26 * year,
        "age at tick 0: {birth}"
    );
    assert_eq!(col(&t, "person.alive"), [1]);
    assert_eq!(col(&t, "person.death_tick"), [i64::MAX]);
    assert_eq!(col(&t, "life_course.origin"), [rh_worldgen::ORIGIN_FOUNDER]);
    assert_eq!(col(&t, "life_course.birth_tick"), [birth]);
    assert_eq!(col(&t, "life_course.exit_tick"), [i64::MAX]);
    println!(
        "T-06: solo gives Ada, female, aged {} minutes at tick 0",
        -birth
    );
}

#[test]
fn a_column_whose_owner_does_not_initialise_from_the_manifest_is_refused() {
    // The owner of the ledger initialises from no manifest section.
    let mut table = specs();
    table
        .specs
        .get_mut("M2.demography")
        .unwrap()
        .from_manifest
        .clear();
    let Err(e) = genesis(&solo(), &table) else {
        panic!("genesis wrote columns whose owner does not initialise from the manifest")
    };
    assert!(
        matches!(
            e,
            GenesisError::NotFromManifest {
                section: "cast",
                ..
            }
        ),
        "{e}"
    );
    assert!(
        e.to_string()
            .contains("does not initialise from the manifest section cast")
    );
    // A neighbour: the owner initialises from cast but not from households, and a household
    // child is a founder.
    let mut table = specs();
    table
        .specs
        .get_mut("M2.demography")
        .unwrap()
        .from_manifest
        .retain(|s| s != "households");
    let mut spec = solo();
    spec.founders.push(Founder {
        role: FounderRole::Child,
        sex: None,
        age_years: None,
    });
    let Err(e) = genesis(&spec, &table) else {
        panic!("a household child was written without households in from_manifest")
    };
    assert!(
        matches!(
            e,
            GenesisError::NotFromManifest {
                section: "households",
                ..
            }
        ),
        "{e}"
    );
    // A column another system owns, which does not initialise from the manifest.
    let mut table = specs();
    table.owners.get_mut("person.spouse").unwrap().system = "T1.minds".to_owned();
    table
        .specs
        .get_mut("T1.minds")
        .unwrap()
        .from_manifest
        .clear();
    let Err(e) = genesis(&solo(), &table) else {
        panic!("genesis wrote a column owned by a system that does not initialise from it")
    };
    assert!(
        matches!(&e, GenesisError::NotFromManifest { column: "person.spouse", owner, .. } if owner == "T1.minds"),
        "{e}"
    );
    println!("T-06: writes to columns whose owner does not initialise from the manifest refused");
}

#[test]
fn random_worlds_have_sorted_aligned_rows_and_ages_in_range() {
    let mut rng = Mix(2026);
    let specs = specs();
    for case in 0..60 {
        let spec = random_world(&mut rng);
        let t = genesis(&spec, &specs).unwrap();
        let n = spec.founders.len();
        for c in t.names() {
            assert_eq!(t.get(c).unwrap().len(), n, "case {case}: {c}");
        }
        let ids = col(&t, "person.id");
        assert!(
            ids.windows(2)
                .all(|w| w[0].cast_unsigned() < w[1].cast_unsigned())
        );
        let year = spec.calendar.minutes_per_year();
        let order = founder_ids(spec.seed, n, &mut PersonIdTable::new()).unwrap();
        for (k, f) in spec.founders.iter().enumerate() {
            let row = ids
                .iter()
                .position(|&i| i == order[k].to_raw().cast_signed())
                .unwrap();
            let birth = col(&t, "person.birth_tick")[row];
            assert!(birth <= 0, "case {case}: born after the epoch");
            let age = -birth / year;
            let (lo, hi) = match (f.age_years, f.role) {
                (Some(a), _) => (i64::from(a), i64::from(a) + 1),
                (None, FounderRole::Child) => (0, 18),
                (None, FounderRole::Servant) => (15, 60),
                (None, _) => (0, 60),
            };
            assert!(
                (lo..hi).contains(&age),
                "case {case} founder {k}: age {age}"
            );
            if let Some(s) = f.sex {
                assert_eq!(col(&t, "person.sex")[row] == SEX_FEMALE, s == Sex::Female);
            }
            assert_eq!(col(&t, "life_course.birth_tick")[row], birth);
        }
        for c in [
            "life_course.exit_kind",
            "life_course.entry_tick",
            "person.mother",
        ] {
            assert!(col(&t, c).iter().all(|&v| v == 0), "case {case}: {c}");
        }
    }
    println!(
        "T-06: 60 random worlds (1 to 120 founders, random seeds, both calendars) laid out correctly"
    );
}

#[test]
fn drawn_sex_is_fair_and_ages_cover_their_range() {
    let n = 4_000;
    let spec = world(
        9,
        D360,
        vec![
            Founder {
                role: FounderRole::Commoner,
                sex: None,
                age_years: None,
            };
            n
        ],
    );
    let t = genesis(&spec, &specs()).unwrap();
    let female = col(&t, "person.sex")
        .iter()
        .filter(|&&s| s == SEX_FEMALE)
        .count();
    // A fair coin: 4,000 draws, within 4.5 standard deviations (sd = 31.6).
    assert!(female.abs_diff(n / 2) < 142, "{female} of {n}");
    let year = D360.minutes_per_year();
    let ages: std::collections::BTreeSet<i64> = col(&t, "person.birth_tick")
        .iter()
        .map(|b| -b / year)
        .collect();
    assert_eq!(ages, (0..60).collect());
}
