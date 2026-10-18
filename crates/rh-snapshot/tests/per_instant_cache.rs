//! The cached plane hash equals the full hash after every instant (SPR-038 S11, ruling T4).
//!
//! `rh-golden` hashes every stop in full, so a column the engine changes without a new version
//! would be invisible to every ladder; `MerkleCache::update_world` trusts versions. Here a plane-A
//! cache is kept across the instants of random worlds and compared with `PlaneDigest::of_world`
//! after every instant: positional compaction (SPR-038 T-02), the row insert, and steps writing
//! ledger and non-ledger columns all go through it.
//!
//! The world: `M2.mortal` owns `person.id`, `person.alive` and the life-course log, and at each day
//! start kills some persons by a keyed draw and inserts one now and then; `T1.namer` owns
//! `person.named` and writes it only at month starts, so on every other day compaction and the
//! insert change a column whose owner did not run (the case a missing version bump hides in); `S1.weather` writes a column of its
//! own every few days. Four sizes (1, 60, 600 and 2,500 founders: the last spans three hash
//! chunks, so a removed row in an early chunk is not re-hashed by the cache's resize rule alone)
//! and eight seeds from a fixed `SplitMix64`.
//!
//! Positive control: `--features rh-core/mutant-compact-keeps-version` (compaction puts columns
//! back with their old versions) must FAIL with "the cached plane hash differs from the full hash".
//!
//! REF: `CODEX/05_PROJECT/SPR-038_PerInstantCost.md` T-02 (S11, T4); VER-SPR-007 DEF-1.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken stub world must fail the test loudly"
)]

mod support;

use rh_core::access::Lag;
use rh_core::engine::Engine;
use rh_core::register::{Registered, register};
use rh_core::system::{PulseBinding, SimError, SpecPurpose, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_ids::{BranchSalt, PersonId};
use rh_snapshot::hash::{MerkleCache, Plane, PlaneDigest};
use support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
/// The days each world runs.
const DAYS: i64 = 75;
const SIZES: [usize; 4] = [1, 60, 600, 2_500];

enum Draw {
    Death,
    Birth,
}

impl SpecPurpose for Draw {
    const ALL: &'static [Self] = &[Self::Death, Self::Birth];
    fn name(&self) -> &'static str {
        match self {
            Self::Death => "death",
            Self::Birth => "birth",
        }
    }
}

const LIFE: [&str; 4] = [
    "life_course.person_id",
    "life_course.exit_kind",
    "life_course.exit_tick",
    "life_course.death_cause",
];

fn mortal(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let t = c.now().minutes();
    let ids = v.read("person.id")?.to_vec();
    let rate = v.read("cfg.rate")?[0].cast_unsigned();
    for (i, &id) in ids.iter().enumerate() {
        if c.rng(&Draw::Death, PersonId::from_raw(id.cast_unsigned()), 0)? % rate != 0 {
            continue;
        }
        v.write("person.alive")?[i] = 0;
        let persons = v.read("life_course.person_id")?.to_vec();
        let kinds = v.read("life_course.exit_kind")?.to_vec();
        let row = (0..persons.len())
            .find(|&r| persons[r] == id && kinds[r] == 0)
            .expect("an open entry");
        v.write("life_course.exit_kind")?[row] = 1;
        v.write("life_course.exit_tick")?[row] = t;
        v.write("life_course.death_cause")?[row] = 1;
    }
    // A birth on about one day in three: the next id, inserted through the core.
    let next = v
        .read("life_course.person_id")?
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        + 1;
    if c.rng(&Draw::Birth, PersonId::from_raw(0), 0)? % 3 == 0 {
        c.insert_person(vec![("person.alive", 1), ("person.id", next)]);
        for (column, value) in LIFE.into_iter().zip([next, 0, i64::MAX, 0]) {
            v.write(column)?.push(value);
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
    let t = c.now().minutes();
    let ids = v.read("person.id")?.to_vec();
    for (slot, id) in v.write("person.named")?.iter_mut().zip(ids) {
        *slot = (id ^ t).rem_euclid(1_000);
    }
    Ok(())
}

fn weather(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let t = c.now().minutes();
    if (t / DAY) % 5 == 2 {
        v.write("weather.temp")?[0] = t % 97;
    }
    Ok(())
}

fn registered() -> Registered {
    let mut m = Stub::new("M2.mortal")
        .purposes_of::<Draw>()
        .writes("person.id")
        .writes("person.alive")
        .writes("cfg.rate");
    for c in LIFE {
        m = m.writes(c);
    }
    let stubs = vec![
        m.at(Pulse::Day, Phase::Vital).stepping(mortal),
        Stub::new("T1.namer")
            .writes("person.named")
            .reads("person.id", "M2.mortal", Lag::SameInstant)
            .at(Pulse::Month, Phase::Culture)
            .stepping(namer),
        Stub::new("S1.weather")
            .writes("weather.temp")
            .at(Pulse::Day, Phase::Environment)
            .stepping(weather),
    ];
    let specs = agreeing_specs(&stubs);
    register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap_or_else(|e| panic!("{e}"))
}

/// A fixed-seed `SplitMix64` for the worlds' shapes.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

fn engine(n: usize, seed: u64, rng: &mut SplitMix64) -> Engine {
    let mut ids: Vec<i64> = (0..n)
        .map(|_| (rng.next() % 1_000_000_000).cast_signed() + 1)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let rows = ids.len();
    let mut t = WorldTables::new();
    t.insert("person.id", ids.clone());
    t.insert("person.alive", vec![1; rows]);
    t.insert("person.named", vec![0; rows]);
    t.insert("life_course.person_id", ids);
    t.insert("life_course.exit_kind", vec![0; rows]);
    t.insert("life_course.exit_tick", vec![i64::MAX; rows]);
    t.insert("life_course.death_cause", vec![0; rows]);
    // About one death in `rate` persons a day: 2 to 9 deaths a day at the largest size.
    t.insert("cfg.rate", vec![(300 + rng.next() % 900).cast_signed()]);
    t.insert("weather.temp", vec![0]);
    Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered(),
        t,
        SimTime::EPOCH,
    )
}

#[test]
fn the_cached_plane_hash_equals_the_full_hash_after_every_instant() {
    let mut rng = SplitMix64(0x0038_C0DE);
    let mut instants = 0_u64;
    let mut compacted = 0_usize;
    for &n in &SIZES {
        for _ in 0..8 {
            let seed = rng.next();
            let mut e = engine(n, seed, &mut rng);
            let start = e.tables().get("person.id").unwrap().len();
            let mut cache = MerkleCache::new(Plane::A);
            let mut mismatch = None;
            e.advance_until(SimTime::from_minutes(DAYS * DAY - 1), |t, tables| {
                instants += 1;
                let cached = cache.update_world(tables);
                let full = PlaneDigest::of_world(tables).hash;
                if cached != full && mismatch.is_none() {
                    mismatch = Some(t);
                }
            })
            .unwrap_or_else(|err| panic!("n {n}, seed {seed}: {err}"));
            if let Some(t) = mismatch {
                panic!(
                    "the cached plane hash differs from the full hash at {t} (n {n}, seed {seed}): \
                     a column changed without a new version"
                );
            }
            let persons = e.tables().get("life_course.person_id").unwrap().len();
            let living = e.tables().get("person.id").unwrap().len();
            // Every entry beyond the living is a death: the path ran compaction.
            compacted += persons - living;
            assert!(persons >= start, "the log is append-only");
        }
    }
    assert_eq!(instants, 4 * 8 * u64::try_from(DAYS).unwrap());
    assert!(compacted > 1_000, "deaths compacted: {compacted}");
    println!(
        "S11: cached = full after each of {instants} instants, with {compacted} rows compacted"
    );
}
