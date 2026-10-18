//! Exact gating (SPR-038 T-03, S12): the gated engine equals the reference that runs every check,
//! compaction and lagged copy at every instant (`Gating::Off`), on random worlds, after every
//! instant; and every planted violation fails by name at the same instant with the same error in
//! both.
//!
//! The world, in three sizes (1, 60 and 600 founders) and eight seeds from a fixed `SplitMix64`:
//! - `M2.life` owns the ledger (`person.id`, `person.alive`, `person.mark`) and the life-course
//!   log. Its `Month`/`Vital` step kills persons by a keyed draw and inserts newborns; its
//!   drain-only `Day`/`Vital` binding applies the `Kill` commands `F1.sender` sends now and then
//!   (so most of its drains have no work); its `Day`/`Bookkeeping` binding writes nothing unless a
//!   violation is planted. Its invariant `INV-L-mark` says every `person.mark` is 0 or 1.
//! - `S1.weather` writes `weather.temp` on some days.
//! - `R1.reader` reads `weather.temp` and `person.alive` with a lag (start-of-instant copies) and
//!   writes their digest to `reader.seen` only when it changes.
//!
//! Many instants are quiet: no column is written, so the gated engine runs no ledger check, no
//! system invariant and no compaction there (`check_stats`).
//!
//! Positive control: `--features mutant-stale-lagged-snapshot` (a lagged column is never copied
//! again) must FAIL here, at the lagged reader ("the gated engine differs from `Gating::Off`").
//!
//! Run with `--features reference-gating-off` (the test target requires it).
//!
//! REF: `CODEX/05_PROJECT/SPR-038_PerInstantCost.md` T-03; `docs/05_architecture/05.1_simulation_core.md` §9.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken stub world must fail the test loudly"
)]

mod support;

use rh_core::access::Lag;
use rh_core::command::{Arg, Command};
use rh_core::engine::{CheckStats, Engine, Gating};
use rh_core::register::{Registered, register};
use rh_core::system::{InvariantResult, PulseBinding, SimError, SpecPurpose, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::EventRef;
use rh_ids::{BranchSalt, PersonId};
use support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Four);
const DAY: i64 = 1_440;
const DAYS: i64 = 100;
const SIZES: [usize; 3] = [1, 60, 600];
const LIFE: [&str; 4] = [
    "life_course.person_id",
    "life_course.exit_kind",
    "life_course.exit_tick",
    "life_course.death_cause",
];

enum Draw {
    Death,
    Birth,
    Send,
    Weather,
}

impl SpecPurpose for Draw {
    const ALL: &'static [Self] = &[Self::Death, Self::Birth, Self::Send, Self::Weather];
    fn name(&self) -> &'static str {
        match self {
            Self::Death => "death",
            Self::Birth => "birth",
            Self::Send => "send",
            Self::Weather => "weather",
        }
    }
}

/// `cfg.world`: `[death rate, planted kind, planted day]`.
const RATE: usize = 0;
const PLANT: usize = 1;
const PLANT_DAY: usize = 2;

fn day(c: &StepCtx<'_>) -> i64 {
    c.now().minutes() / DAY
}

/// Closes ledger row `row` and its open entry at `t`.
fn kill(v: &mut SystemView<'_>, row: usize, t: i64) -> Result<(), SimError> {
    let id = v.read("person.id")?[row];
    v.write("person.alive")?[row] = 0;
    let persons = v.read("life_course.person_id")?.to_vec();
    let kinds = v.read("life_course.exit_kind")?.to_vec();
    let entry = (0..persons.len())
        .find(|&r| persons[r] == id && kinds[r] == 0)
        .expect("an open entry");
    v.write("life_course.exit_kind")?[entry] = 1;
    v.write("life_course.exit_tick")?[entry] = t;
    v.write("life_course.death_cause")?[entry] = 1;
    Ok(())
}

fn append(v: &mut SystemView<'_>, entry: [i64; 4]) -> Result<(), SimError> {
    for (column, value) in LIFE.into_iter().zip(entry) {
        v.write(column)?.push(value);
    }
    Ok(())
}

fn next_id(v: &SystemView<'_>) -> Result<i64, SimError> {
    Ok(v.read("life_course.person_id")?
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        + 1)
}

/// The planted violation of kind `kind`, written at its day in `Vital` (kind 2 in `Bookkeeping`).
fn plant(v: &mut SystemView<'_>, kind: i64, t: i64) -> Result<(), SimError> {
    let living = v.read("person.id")?.to_vec();
    let first = living.first().copied().unwrap_or(0);
    match kind {
        3 => append(v, [first, 0, i64::MAX, 0]),
        4 => {
            let id = next_id(v)?;
            append(v, [id, 0, i64::MAX, 0])
        }
        5 => {
            v.write("life_course.person_id")?[0] += 1;
            Ok(())
        }
        6 => {
            let kinds = v.read("life_course.exit_kind")?.to_vec();
            let closed = kinds.iter().position(|&k| k != 0).expect("a closed entry");
            v.write("life_course.exit_tick")?[closed] = t;
            Ok(())
        }
        7 => {
            let id = next_id(v)?;
            append(v, [id, 0, 5, 0])
        }
        8 => append(v, [first, 1, t, 1]),
        9 => {
            v.write("person.id")?[0] += 1;
            Ok(())
        }
        10 => {
            v.write("person.mark")?[0] = 7;
            Ok(())
        }
        11 => {
            v.write("person.mark")?.push(0);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn life(
    _: &Stub,
    binding: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let now = c.now().minutes();
    let cfg = v.read("cfg.world")?.to_vec();
    let planted = (cfg[PLANT_DAY] == day(c)).then_some(cfg[PLANT]);
    match (binding.pulse, binding.phase) {
        (Pulse::Day, Phase::Vital) => {
            for q in c.take_commands() {
                let target = q.command.person.to_raw().cast_signed();
                let row = v.read("person.id")?.binary_search(&target).ok();
                match row {
                    Some(r) if v.read("person.alive")?[r] == 1 => kill(v, r, now)?,
                    _ => c.drop_command(&q, rh_core::system::DropReason::Dead),
                }
            }
            if let Some(kind) = planted.filter(|&k| k != 2) {
                plant(v, kind, now)?;
            }
        }
        (Pulse::Day, Phase::Bookkeeping) => {
            if planted == Some(2) && !v.read("person.alive")?.is_empty() {
                v.write("person.alive")?[0] = 0;
            }
        }
        _ => {
            let rate = cfg[RATE].cast_unsigned();
            let ids = v.read("person.id")?.to_vec();
            for (r, &id) in ids.iter().enumerate() {
                if c.rng(&Draw::Death, PersonId::from_raw(id.cast_unsigned()), 0)? % rate == 0 {
                    kill(v, r, now)?;
                }
            }
            let births =
                usize::try_from(c.rng(&Draw::Birth, PersonId::from_raw(0), 0)? % 4).unwrap();
            for id in (next_id(v)?..).take(births) {
                c.insert_person(vec![
                    ("person.alive", 1),
                    ("person.id", id),
                    ("person.mark", id & 1),
                ]);
                append(v, [id, 0, i64::MAX, 0])?;
            }
        }
    }
    Ok(())
}

fn marks(_: &Stub, w: &WorldTables) -> Vec<InvariantResult> {
    let bad = w
        .get("person.mark")
        .and_then(|m| m.iter().position(|&x| x != 0 && x != 1));
    vec![InvariantResult {
        name: "INV-L-mark",
        holds: bad.is_none(),
        detail: bad.map_or_else(String::new, |r| format!("row {r} is neither 0 nor 1")),
    }]
}

fn sender(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let ids = v.read("person.id")?;
    let draw = c.rng(&Draw::Send, PersonId::from_raw(0), 0)?;
    if ids.is_empty() || draw % 7 != 0 {
        return Ok(());
    }
    let target = ids[usize::try_from((draw >> 8) % ids.len() as u64).unwrap()];
    c.submit(Command {
        kind: "Kill",
        person: PersonId::from_raw(target.cast_unsigned()),
        args: vec![Arg::Int(target)],
        cause: EventRef::None,
    })
}

fn weather(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let draw = c.rng(&Draw::Weather, PersonId::from_raw(0), 0)?;
    if draw % 4 == 0 {
        v.write("weather.temp")?[0] = (draw >> 8).cast_signed() % 40;
    }
    Ok(())
}

fn reader(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    _: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let temp = v.read("weather.temp")?[0];
    let alive: i64 = v.read("person.alive")?.iter().sum();
    let seen = temp * 1_000_003 + alive;
    if v.read("reader.seen")?[0] != seen {
        v.write("reader.seen")?[0] = seen;
    }
    Ok(())
}

fn registered() -> Registered {
    let mut life_stub = Stub::new("M2.life")
        .purposes_of::<Draw>()
        .writes("person.id")
        .writes("person.alive")
        .writes("person.mark")
        .writes("cfg.world");
    for column in LIFE {
        life_stub = life_stub.writes(column);
    }
    let stubs = vec![
        life_stub
            .at(Pulse::Day, Phase::Vital)
            .applies("Kill")
            .typed("Kill", &[("who", "PersonId")], "who", "any")
            .drain_only(Pulse::Day, Phase::Vital)
            .at(Pulse::Month, Phase::Vital)
            .at(Pulse::Day, Phase::Bookkeeping)
            .stepping(life)
            .checking(marks),
        Stub::new("F1.sender")
            .purposes_of::<Draw>()
            .sends("Kill", "M2.life")
            .reads("person.id", "M2.life", Lag::Lagged)
            .at(Pulse::Day, Phase::Agenda)
            .stepping(sender),
        Stub::new("S1.weather")
            .purposes_of::<Draw>()
            .writes("weather.temp")
            .at(Pulse::Day, Phase::Environment)
            .stepping(weather),
        Stub::new("R1.reader")
            .writes("reader.seen")
            .reads("weather.temp", "S1.weather", Lag::Lagged)
            .reads("person.alive", "M2.life", Lag::Lagged)
            .at(Pulse::Day, Phase::Culture)
            .stepping(reader),
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

/// Genesis: `n` founders alive and `n / 3 + 1` dead ancestors (closed entries), ids ascending.
fn tables(n: usize, plant: (i64, i64), rng: &mut SplitMix64) -> WorldTables {
    let dead = n / 3 + 1;
    let mut ids: Vec<i64> = (0..n + dead)
        .map(|_| (rng.next() % 1_000_000_000).cast_signed() + 1)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let (ancestors, living) = ids.split_at(dead.min(ids.len()));
    let mut living = living.to_vec();
    if plant.0 == 1 && living.len() > 1 {
        let last = living.len() - 1;
        living.swap(0, last);
    }
    let rows = living.len();
    let mut t = WorldTables::new();
    t.insert("person.id", living.clone());
    t.insert("person.alive", vec![1; rows]);
    t.insert("person.mark", living.iter().map(|id| id & 1).collect());
    let mut log: Vec<[i64; 4]> = ancestors.iter().map(|&a| [a, 1, -DAY, 1]).collect();
    log.extend(living.iter().map(|&p| [p, 0, i64::MAX, 0]));
    for (k, column) in LIFE.into_iter().enumerate() {
        t.insert(column, log.iter().map(|e| e[k]).collect());
    }
    t.insert(
        "cfg.world",
        vec![(40 + rng.next() % 200).cast_signed(), plant.0, plant.1],
    );
    t.insert("weather.temp", vec![0]);
    t.insert("reader.seen", vec![-1]);
    t
}

fn engine(tables: WorldTables, seed: u64, gating: Gating) -> Engine {
    let mut e = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered(),
        tables,
        SimTime::EPOCH,
    );
    e.set_gating(gating);
    e
}

fn delta(a: CheckStats, b: CheckStats) -> CheckStats {
    CheckStats {
        compactions: b.compactions - a.compactions,
        compacted_rows: b.compacted_rows - a.compacted_rows,
        ledger_checks: b.ledger_checks - a.ledger_checks,
        invariant_runs: b.invariant_runs - a.invariant_runs,
        snapshot_copies: b.snapshot_copies - a.snapshot_copies,
    }
}

#[test]
fn the_gated_engine_equals_gating_off_after_every_instant() {
    let mut rng = SplitMix64(0x0038_0003);
    let (mut instants, mut quiet, mut changed) = (0_u64, 0_u64, 0_u64);
    let mut totals = (CheckStats::default(), CheckStats::default());
    for &n in &SIZES {
        for _ in 0..8 {
            let seed = rng.next();
            let t0 = tables(n, (0, -1), &mut rng);
            let mut gated = engine(t0.clone(), seed, Gating::Exact);
            let mut off = engine(t0, seed, Gating::Off);
            let end = SimTime::from_minutes(DAYS * DAY - 1);
            while let Some(t) = gated.next_instant().filter(|&t| t <= end) {
                assert_eq!(off.next_instant(), Some(t), "n {n}, seed {seed}");
                let before = gated.tables().clone();
                let stats = gated.check_stats();
                gated
                    .advance_until(t, |_, _| {})
                    .unwrap_or_else(|e| panic!("n {n}, seed {seed}, gated at {t}: {e}"));
                off.advance_until(t, |_, _| {})
                    .unwrap_or_else(|e| panic!("n {n}, seed {seed}, off at {t}: {e}"));
                assert!(
                    gated.snapshot() == off.snapshot(),
                    "the gated engine differs from Gating::Off at {t} (n {n}, seed {seed})"
                );
                let d = delta(stats, gated.check_stats());
                instants += 1;
                if gated.tables() == &before {
                    // Nothing changed: unless something was written without a change (the reader
                    // never does that), no check ran.
                    if d.invariant_runs == 0 {
                        quiet += 1;
                        assert_eq!(d.ledger_checks, 0, "a ledger check at a quiet {t}");
                        assert_eq!(d.compactions, 0, "a compaction at a quiet {t}");
                    }
                } else {
                    changed += 1;
                    assert_eq!(
                        d.invariant_runs, 1,
                        "no check after a change at {t} (n {n})"
                    );
                }
            }
            totals.0 = sum(totals.0, gated.check_stats());
            totals.1 = sum(totals.1, off.check_stats());
        }
    }
    let (g, o) = totals;
    assert!(quiet > instants / 5, "{quiet} quiet of {instants} instants");
    assert!(
        changed > instants / 5,
        "{changed} changed of {instants} instants"
    );
    assert_eq!(
        o.invariant_runs, instants,
        "Gating::Off checks every instant"
    );
    assert!(g.invariant_runs < o.invariant_runs && g.ledger_checks < o.ledger_checks);
    assert!(g.compacted_rows == o.compacted_rows && g.compacted_rows > 100);
    assert!(g.snapshot_copies < o.snapshot_copies);
    println!(
        "SPR-038 T-03: gated = Gating::Off after each of {instants} instants ({quiet} quiet); \
         gated ran {} invariant runs, {} ledger checks, {} compactions, {} copies; Off {}, {}, {}, {}",
        g.invariant_runs,
        g.ledger_checks,
        g.compactions,
        g.snapshot_copies,
        o.invariant_runs,
        o.ledger_checks,
        o.compactions,
        o.snapshot_copies
    );
}

fn sum(a: CheckStats, b: CheckStats) -> CheckStats {
    CheckStats {
        compactions: a.compactions + b.compactions,
        compacted_rows: a.compacted_rows + b.compacted_rows,
        ledger_checks: a.ledger_checks + b.ledger_checks,
        invariant_runs: a.invariant_runs + b.invariant_runs,
        snapshot_copies: a.snapshot_copies + b.snapshot_copies,
    }
}

/// Runs `tables` to day `DAYS` gated and under `Gating::Off`: both fail, at the same instant with
/// the same error, whose text is returned.
fn fails_alike(t0: &WorldTables, seed: u64, what: &str) -> String {
    let end = SimTime::from_minutes(DAYS * DAY - 1);
    let mut gated = engine(t0.clone(), seed, Gating::Exact);
    let mut off = engine(t0.clone(), seed, Gating::Off);
    let g = gated
        .advance_until(end, |_, _| {})
        .expect_err(&format!("{what}: the gated run must fail"));
    let o = off
        .advance_until(end, |_, _| {})
        .expect_err(&format!("{what}: the Gating::Off run must fail"));
    assert_eq!(g, o, "{what}: the same error");
    let (ga, oa) = (
        gated.crash_artifact().unwrap(),
        off.crash_artifact().unwrap(),
    );
    assert_eq!(
        (ga.tick, &ga.phase),
        (oa.tick, &oa.phase),
        "{what}: the same instant"
    );
    g.to_string()
}

#[test]
fn every_planted_violation_fails_by_name_at_the_same_instant_gated_and_off() {
    let cases: [(i64, &str, &str); 11] = [
        (
            1,
            "unsorted ids",
            "invariant ledger-sorted of rh-core violated",
        ),
        (2, "a dead row left", "after the instant's compaction"),
        (
            3,
            "two open entries",
            "invariant life-course-one-open-entry of rh-core violated",
        ),
        (
            4,
            "an open entry without a row",
            "invariant life-course-closed-or-absent of rh-core violated",
        ),
        (
            5,
            "an entry field changed",
            "a life-course entry's fields are written once",
        ),
        (
            6,
            "an exit field written twice",
            "on a closed entry: exit fields are written once",
        ),
        (
            7,
            "an open append with a non-open value",
            "not the open value",
        ),
        (8, "a closed append for a living person", "DEF-6"),
        (
            9,
            "an id changed",
            "an existing row's person.id never changes",
        ),
        (
            10,
            "a system invariant broken",
            "invariant INV-L-mark of M2.life violated",
        ),
        (11, "a ledger row count changed", "person.mark went from"),
    ];
    let mut rng = SplitMix64(0x0038_0013);
    for (kind, what, expected) in cases {
        for n in [30_usize, 60] {
            for _ in 0..4 {
                let seed = rng.next();
                let plant_day = i64::try_from(rng.next() % 40).unwrap() + 3;
                let t0 = tables(n, (kind, plant_day), &mut rng);
                let text = fails_alike(&t0, seed, what);
                assert!(text.contains(expected), "{what} (n {n}): {text}");
            }
        }
        println!("SPR-038 T-03: planted {what}: refused alike, gated and Off");
    }
}
