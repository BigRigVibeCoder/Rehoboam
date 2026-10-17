//! The person ledger and the life-course log (SPR-011 T-03, ruling 5; V-INV-01 subset, V-INV-04,
//! V-DET-01/02 with deaths).
//!
//! The stub world: `M2.mortal` owns the ledger's core columns and the life-course log and kills
//! people by a keyed draw each month in `Vital`; `T1.namer` owns another `person.*` column and
//! rewrites it each month in `Culture`, so compaction removes rows from a column the dying
//! system does not own; `X9.audit` reads `person.alive` in `Bookkeeping` and fails if it sees a
//! dead row, which proves the dead leave before any `Bookkeeping` step. Worlds vary in size and
//! seed (the DEF-5 lesson: one small world hides bugs), with closed historical life-course rows.
//! Inputs come from `SplitMix64` with fixed seeds, so every run checks the same cases.
//!
//! Refs: SPR-011 T-03; `crates/rh-core/src/ledger.rs`; `docs/09` V-INV-01, V-INV-04, V-DET-01,
//! V-DET-02.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken stub world must fail the test loudly"
)]

mod support;

use std::collections::BTreeSet;

use rh_core::access::Lag;
use rh_core::engine::Engine;
use rh_core::ledger::{LIFE_COURSE_EXIT_KIND, LIFE_COURSE_PERSON, OPEN, PERSON_ALIVE, PERSON_ID};
use rh_core::register::register;
use rh_core::system::{PulseBinding, SimError, SpecPurpose, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_ids::{BranchSalt, PersonId};
use support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const MONTH: i64 = 43_200;
/// One death in this many draws, each month: worlds empty within a few decades.
const DEATH_ODDS: u64 = 40;

enum Draw {
    Death,
}

impl SpecPurpose for Draw {
    const ALL: &'static [Self] = &[Self::Death];
    fn name(&self) -> &'static str {
        "death"
    }
}

/// `SplitMix64` (Steele, Lea and Flood 2014): the test's input generator.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

const LIFE: [&str; 8] = [
    "life_course.person_id",
    "life_course.origin",
    "life_course.sex",
    "life_course.birth_tick",
    "life_course.entry_tick",
    "life_course.exit_kind",
    "life_course.exit_tick",
    "life_course.death_cause",
];

fn col(v: &SystemView<'_>, c: &str) -> Result<Vec<i64>, SimError> {
    v.read(c).map(<[i64]>::to_vec)
}

/// M2's stand-in: each month, each living person dies with odds 1 / `DEATH_ODDS` (one keyed draw
/// per person, k = 0), through one function that writes the alive flag, the death tick and the
/// life-course exit, as `apply_death` does (ruling 6). Some ids misbehave on purpose (V-INV-04).
fn mortal(
    s: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let ids = col(v, PERSON_ID)?;
    let alive = col(v, PERSON_ALIVE)?;
    let persons = col(v, LIFE_COURSE_PERSON)?;
    let now = c.now().minutes();
    let mut dying = Vec::new();
    for (i, (&id, &a)) in ids.iter().zip(&alive).enumerate() {
        let draw = c.rng(&Draw::Death, PersonId::from_raw(id.cast_unsigned()), 0)?;
        if a == 1 && draw % DEATH_ODDS == 0 {
            dying.push((i, id));
        }
    }
    for &(i, id) in &dying {
        v.write(PERSON_ALIVE)?[i] = 0;
        v.write("person.death_tick")?[i] = now;
        let row = persons
            .iter()
            .zip(col(v, LIFE_COURSE_EXIT_KIND)?)
            .position(|(&p, k)| p == id && k == OPEN)
            .expect("an open entry for every living person");
        if s.id != "M2.forgetful" {
            v.write(LIFE_COURSE_EXIT_KIND)?[row] = 1;
            v.write("life_course.exit_tick")?[row] = now;
            v.write("life_course.death_cause")?[row] = 1;
        }
    }
    match s.id {
        // V-INV-04: a system that removes a ledger row itself, from a column it owns.
        "M2.remover" if !ids.is_empty() => {
            for c in ["person.id", "person.alive"] {
                v.write(c)?.pop();
            }
        }
        // A system that removes a life-course row.
        "M2.eraser" if !persons.is_empty() => {
            for c in LIFE {
                v.write(c)?.remove(0);
            }
        }
        // A system that writes a closed entry's exit again, or an entry field.
        "M2.rewriter" => {
            let kinds = col(v, LIFE_COURSE_EXIT_KIND)?;
            if let Some(row) = kinds.iter().position(|&k| k != OPEN) {
                v.write("life_course.exit_tick")?[row] += 1;
            }
        }
        "M2.reborn" if !persons.is_empty() => v.write("life_course.birth_tick")?[0] += 1,
        // A system that writes person.named, which T1.namer owns.
        "M2.trespasser" => {
            v.write("person.named")?;
        }
        _ => {}
    }
    Ok(())
}

/// T1's stand-in: rewrites `person.named` (which it owns) from `person.id` every month.
fn namer(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    _: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let ids = col(v, PERSON_ID)?;
    let named = v.write("person.named")?;
    for (slot, id) in named.iter_mut().zip(ids) {
        *slot = id.rem_euclid(7);
    }
    Ok(())
}

/// A `Bookkeeping` reader: the dead of this instant must already be gone.
fn audit(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if v.read(PERSON_ALIVE)?.contains(&0) {
        return Err(SimError::System {
            system: "X9.audit",
            message: format!("saw a dead row in Bookkeeping at {}", c.now()),
        });
    }
    Ok(())
}

fn mortal_stub(id: &'static str) -> Stub {
    let mut s = Stub::new(id).purposes_of::<Draw>();
    for c in [
        "person.id",
        "person.alive",
        "person.birth_tick",
        "person.death_tick",
    ]
    .into_iter()
    .chain(LIFE)
    {
        s = s.writes(c);
    }
    s.at(Pulse::Month, Phase::Vital).stepping(mortal)
}

fn systems(mortal: &'static str) -> Vec<Stub> {
    vec![
        mortal_stub(mortal),
        Stub::new("T1.namer")
            .writes("person.named")
            .reads("person.id", mortal, Lag::SameInstant)
            .at(Pulse::Month, Phase::Culture)
            .stepping(namer),
        Stub::new("X9.audit")
            .reads("person.alive", mortal, Lag::SameInstant)
            .at(Pulse::Month, Phase::Bookkeeping)
            .stepping(audit),
    ]
}

/// A world of `n` living founders with random sorted unique ids, and `closed` people who died
/// before the start: their life-course rows are closed and they have no ledger row.
fn world(n: usize, closed: usize, seed: u64) -> WorldTables {
    let mut rng = SplitMix64(seed);
    let mut ids = BTreeSet::new();
    while ids.len() < n + closed {
        ids.insert(rng.next_u64() >> 1 | 1);
    }
    let all: Vec<i64> = ids.into_iter().map(u64::cast_signed).collect();
    let mut dead_rows: Vec<usize> = Vec::new();
    while dead_rows.len() < closed {
        let i = usize::try_from(rng.next_u64() % (n + closed) as u64).unwrap();
        if !dead_rows.contains(&i) {
            dead_rows.push(i);
        }
    }
    let living: Vec<i64> = (0..all.len())
        .filter(|i| !dead_rows.contains(i))
        .map(|i| all[i])
        .collect();
    let mut t = WorldTables::new();
    let n_live = living.len();
    t.insert("person.id", living.clone());
    t.insert("person.alive", vec![1; n_live]);
    t.insert("person.birth_tick", vec![-MONTH * 12 * 20; n_live]);
    t.insert("person.death_tick", vec![i64::MAX; n_live]);
    t.insert("person.named", vec![0; n_live]);
    // Entry order: the founders in ascending id, as genesis appends them.
    let closed_row = |i: usize| dead_rows.contains(&i);
    let rows = all.len();
    t.insert("life_course.person_id", all.clone());
    t.insert("life_course.origin", vec![1; rows]);
    t.insert(
        "life_course.sex",
        (0..rows).map(|i| i64::from(i % 2 == 1)).collect(),
    );
    t.insert("life_course.birth_tick", vec![-MONTH * 12 * 20; rows]);
    t.insert("life_course.entry_tick", vec![0; rows]);
    let kind = |i| i64::from(closed_row(i));
    t.insert("life_course.exit_kind", (0..rows).map(kind).collect());
    let tick = |i| if closed_row(i) { -1 } else { i64::MAX };
    t.insert("life_course.exit_tick", (0..rows).map(tick).collect());
    t.insert("life_course.death_cause", (0..rows).map(kind).collect());
    t
}

fn engine(stubs: Vec<Stub>, tables: WorldTables, seed: u64, threads: usize) -> Engine {
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut e = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    e.set_threads(threads);
    e
}

fn get<'t>(t: &'t WorldTables, c: &str) -> &'t [i64] {
    t.get(c).map(Vec::as_slice).unwrap_or_default()
}

/// V-INV-01 subset, checked here independently of the engine's own invariants.
fn check_ledger(t: &WorldTables, what: &str) {
    let ids = get(t, PERSON_ID);
    for c in t.names().filter(|c| c.starts_with("person.")) {
        assert_eq!(get(t, c).len(), ids.len(), "{what}: {c} is not aligned");
    }
    assert!(
        ids.windows(2)
            .all(|w| w[0].cast_unsigned() < w[1].cast_unsigned()),
        "{what}: person.id is not sorted"
    );
    let open: BTreeSet<i64> = get(t, LIFE_COURSE_PERSON)
        .iter()
        .zip(get(t, LIFE_COURSE_EXIT_KIND))
        .filter(|&(_, &k)| k == OPEN)
        .map(|(&p, _)| p)
        .collect();
    let living: BTreeSet<i64> = ids.iter().copied().collect();
    assert_eq!(
        living.len(),
        open.len(),
        "{what}: living count = open entries"
    );
    assert_eq!(
        living, open,
        "{what}: a closed entry is absent from the ledger"
    );
    assert!(
        !get(t, PERSON_ALIVE).contains(&0),
        "{what}: a dead row survived"
    );
}

#[test]
fn compaction_keeps_the_ledger_aligned_sorted_and_equal_to_the_open_entries() {
    let mut total_deaths = 0;
    for (k, &(n, closed)) in [(0, 0), (1, 0), (5, 2), (17, 3), (60, 10), (200, 25)]
        .iter()
        .enumerate()
    {
        for seed in [1, 7, 42] {
            let what = format!("n {n}, closed {closed}, seed {seed}");
            let mut e = engine(
                systems("M2.mortal"),
                world(n, closed, seed + k as u64),
                seed,
                1,
            );
            let rows = get(e.tables(), LIFE_COURSE_PERSON).len();
            for month in 1..=12 * 20 {
                e.advance_until(SimTime::from_minutes(MONTH * month), |_, _| {})
                    .unwrap_or_else(|err| panic!("{what}, month {month}: {err}"));
                check_ledger(e.tables(), &format!("{what}, month {month}"));
                // The log never shrinks and keeps every entry ever made.
                assert_eq!(get(e.tables(), LIFE_COURSE_PERSON).len(), rows, "{what}");
            }
            total_deaths += n - get(e.tables(), PERSON_ID).len();
        }
    }
    assert!(
        total_deaths > 300,
        "the worlds must actually die: {total_deaths} deaths"
    );
}

#[test]
fn a_death_in_vital_is_gone_before_bookkeeping_and_logged_with_its_instant() {
    let mut e = engine(systems("M2.mortal"), world(60, 0, 3), 3, 1);
    let before: Vec<i64> = get(e.tables(), PERSON_ID).to_vec();
    // Month pulses fire at 0, 43200, ...: advance one instant at a time until someone dies.
    let mut month = -1;
    while get(e.tables(), PERSON_ID).len() == before.len() {
        month += 1;
        // X9.audit fails the instant if it sees a dead row in Bookkeeping.
        e.advance_until(SimTime::from_minutes(MONTH * month), |_, _| {})
            .unwrap();
    }
    let t = e.tables();
    let gone: Vec<i64> = before
        .iter()
        .filter(|id| !get(t, PERSON_ID).contains(id))
        .copied()
        .collect();
    for id in gone {
        let row = get(t, LIFE_COURSE_PERSON)
            .iter()
            .position(|&p| p == id)
            .unwrap();
        assert_eq!(get(t, LIFE_COURSE_EXIT_KIND)[row], 1);
        assert_eq!(get(t, "life_course.exit_tick")[row], MONTH * month);
    }
    // person.named, owned by T1.namer, lost the same rows.
    assert_eq!(get(t, "person.named").len(), get(t, PERSON_ID).len());
}

#[test]
fn v_det_01_02_a_world_with_deaths_is_identical_across_threads_and_repeats() {
    for (n, seed) in [(60, 11), (200, 5)] {
        let run = |threads: usize| {
            let mut e = engine(systems("M2.mortal"), world(n, 4, seed), seed, threads);
            let mut states = Vec::new();
            for month in 1..=12 * 30 {
                e.advance_until(SimTime::from_minutes(MONTH * month), |_, _| {})
                    .unwrap();
                states.push(e.tables().clone());
            }
            states
        };
        let reference = run(1);
        assert!(
            get(reference.last().unwrap(), PERSON_ID).len() < n,
            "people die in {n}"
        );
        for threads in [1, 2, 4, 8] {
            assert!(run(threads) == reference, "n {n}: {threads} threads differ");
        }
    }
}

fn first_error(mortal: &'static str) -> SimError {
    let mut e = engine(systems(mortal), world(30, 3, 9), 9, 1);
    e.advance_until(SimTime::from_minutes(MONTH * 12 * 40), |_, _| {})
        .expect_err("the misbehaving system must be refused")
}

#[test]
fn v_inv_04_a_step_that_removes_a_ledger_row_is_refused_by_name() {
    match first_error("M2.remover") {
        SimError::Ledger { system, message } => {
            assert_eq!(system, "M2.remover");
            assert!(
                message.starts_with("person.id went from 30 to 29 rows: only rh-core's compaction removes ledger rows"),
                "{message}"
            );
        }
        other => panic!("refused for the wrong reason: {other}"),
    }
}

#[test]
fn v_inv_04_a_step_that_writes_a_person_column_it_does_not_own_is_refused() {
    assert_eq!(
        first_error("M2.trespasser"),
        SimError::UnownedWrite {
            system: "M2.trespasser",
            column: "person.named".to_owned()
        }
    );
}

#[test]
fn the_life_course_log_is_append_only_and_exits_are_written_once() {
    for (mortal, expected) in [
        (
            "M2.eraser",
            "life_course.person_id went from 33 to 32 rows: the life-course log is append-only",
        ),
        (
            "M2.rewriter",
            "life_course.exit_tick row 10 changed from -1 to 0 on a closed entry: exit fields are written once",
        ),
        (
            "M2.reborn",
            "life_course.birth_tick row 0 changed from -10368000 to -10367999: a life-course entry's fields are written once, at entry",
        ),
    ] {
        match first_error(mortal) {
            SimError::Ledger { system, message } => {
                assert_eq!(system, mortal);
                assert!(message.starts_with(expected), "{mortal}: {message}");
            }
            other => panic!("{mortal} refused for the wrong reason: {other}"),
        }
    }
}

#[test]
fn a_death_whose_life_course_entry_stays_open_breaks_the_invariant() {
    match first_error("M2.forgetful") {
        SimError::Invariant { system, name, .. } => {
            assert_eq!((system, name), ("rh-core", "life-course-closed-or-absent"));
        }
        other => panic!("wrong error: {other}"),
    }
}

#[test]
fn a_world_with_ledger_columns_but_no_person_id_never_runs() {
    let mut tables = world(5, 0, 1);
    let mut t = WorldTables::new();
    for name in tables.names().map(str::to_owned).collect::<Vec<_>>() {
        if name != PERSON_ID {
            t.insert(&name, tables.get(&name).unwrap().clone());
        }
    }
    tables = t;
    let stubs = vec![
        Stub::new("T1.namer")
            .writes("person.named")
            .at(Pulse::Month, Phase::Culture),
    ];
    let mut e = engine(stubs, tables, 1, 1);
    match e.advance_until(SimTime::from_minutes(MONTH), |_, _| {}) {
        Err(SimError::Ledger { system, message }) => {
            assert_eq!(system, "rh-core");
            assert!(message.contains("but no person.id"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}
