//! T-05: the invariant framework and crash artifacts (SPR-006; GOV-004 §4; ruling 6).
//!
//! - **Injected violation.** `Q1.stock` draws a monthly loss for each of eight stocks; its
//!   invariant `no-negatives` fails at the first month a stock goes below zero, which depends on
//!   the seed. The run stops at that instant in `Bookkeeping` with a crash artifact; the test
//!   writes it to `artifacts/crash/<timeline>_<tick>.json` (not committed), reads it back, and
//!   replays it on a fresh engine built from the artifact's tuple, at 1 and 8 threads: the same
//!   violation recurs at the same instant.
//! - **Replay refusals.** Another seed or build is refused by the tuple check; a world without the
//!   violation reaches the instant without failing; a different failure is a mismatch.
//! - **Framework self-test.** Invariants run at every instant, in `Bookkeeping`, after that phase's
//!   systems: a flag a Bookkeeping system raises at one instant fails at exactly that instant. A
//!   step error (not an invariant) also leaves an artifact. A stopped engine is poisoned.
//! - **Inputs and the journal (DEF-2).** An artifact from a run that applied material inputs
//!   records their number as the journal position, and replay refuses it explicitly until SPR-007
//!   supplies the journal.
//! - **Panics (DEF-1).** A panicking step stops the run with `SimError::Panic` and an artifact that
//!   replays, at 1 and 4 threads.
//! - **Committed artifact.** `tests/golden/crash_stock_seed3.json` must equal the run's artifact and
//!   replay. `RH_CORE_CRASH_ARTIFACT` points the test at another artifact: `bin/check_all.sh`
//!   plants one with an altered instant, which must fail.
//!
//! Refs: SPR-006 T-05, ruling 6; GOV-004 §2, §4; `05.1` §9.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken stub world must fail the test loudly"
)]

mod support;

use std::path::PathBuf;

use rh_core::crash::{CRASH_DIR, CrashArtifact, Provenance};
use rh_core::engine::{Engine, MaterialInput, ReplayError};
use rh_core::register::register;
use rh_core::system::{InvariantResult, PulseBinding, SimError, SpecPurpose, StepCtx};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_ids::{BranchSalt, RegionId, TimelineId};
use support::{Stub, agreeing_specs};

const CALENDAR: Calendar = Calendar::new(DaysPerYear::D360, SeasonsPerYear::Two);
const ROWS: u16 = 8;

enum Lp {
    Loss,
}

impl SpecPurpose for Lp {
    const ALL: &'static [Self] = &[Self::Loss];
    fn name(&self) -> &'static str {
        "loss"
    }
}

fn stock_step(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let max_loss = v.read("stock.max_loss")?[0].cast_unsigned();
    let mut losses = Vec::new();
    for row in 0..ROWS {
        let d = c.rng(&Lp::Loss, RegionId::from_raw(row), 0)?;
        losses.push((d % max_loss).cast_signed());
    }
    for (s, loss) in v.write("stock.s")?.iter_mut().zip(losses) {
        *s -= loss;
    }
    Ok(())
}

fn no_negatives(_: &Stub, w: &WorldTables) -> Vec<InvariantResult> {
    let s = w.get("stock.s").map(Vec::as_slice).unwrap_or_default();
    let bad = s.iter().position(|&x| x < 0);
    vec![
        InvariantResult {
            name: "rows-present",
            holds: s.len() == usize::from(ROWS),
            detail: format!("{} rows", s.len()),
        },
        InvariantResult {
            name: "no-negatives",
            holds: bad.is_none(),
            detail: bad.map_or_else(String::new, |i| format!("stock.s[{i}] = {}", s[i])),
        },
    ]
}

fn provenance() -> Provenance {
    Provenance {
        build: "test-build".to_owned(),
        registry_version: "registry-test".to_owned(),
        manifest_hash: "manifest-test".to_owned(),
    }
}

/// The engine the tuple `(build, registry, manifest, seed)` describes; `max_loss` is the world.
fn stock_engine(seed: u64, max_loss: i64, threads: usize) -> Engine {
    let stubs = vec![
        Stub::new("Q1.stock")
            .purposes_of::<Lp>()
            .writes("stock.s")
            .writes("stock.max_loss")
            .at(Pulse::Month, Phase::Production)
            .stepping(stock_step)
            .checking(no_negatives),
        // An unrelated system on the Day pulse, so instants run between the failing ones.
        Stub::new("Q2.idle").at(Pulse::Day, Phase::Culture),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("stock.s", vec![200; usize::from(ROWS)]);
    tables.insert("stock.max_loss", vec![max_loss]);
    let mut e = Engine::new(
        CALENDAR,
        seed,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    );
    e.set_provenance(provenance());
    e.set_timeline(TimelineId::from_raw(0));
    e.set_threads(threads);
    e
}

fn ten_years() -> SimTime {
    SimTime::from_minutes(10 * CALENDAR.minutes_per_year())
}

fn crash_path(a: &CrashArtifact) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(CRASH_DIR)
        .join(a.file_name())
}

#[test]
fn t05_an_injected_violation_leaves_an_artifact_that_replays() {
    let mut ticks = Vec::new();
    for seed in [3_u64, 4, 5] {
        let mut engine = stock_engine(seed, 40, 1);
        let err = engine.advance_until(ten_years(), |_, _| {}).unwrap_err();
        let SimError::Invariant { system, name, .. } = &err else {
            panic!("expected a violated invariant, got {err}");
        };
        assert_eq!((*system, *name), ("Q1.stock", "no-negatives"));
        let a = engine.crash_artifact().expect("a crash artifact").clone();
        assert_eq!(
            (a.timeline, a.seed, a.branch_salt, a.journal_position),
            (0, seed, 0, 0)
        );
        assert_eq!(
            (
                a.build.as_str(),
                a.registry_version.as_str(),
                a.manifest_hash.as_str()
            ),
            ("test-build", "registry-test", "manifest-test")
        );
        assert_eq!(a.last_keyframe, None, "no keyframe until SPR-007");
        assert_eq!(a.phase, "Bookkeeping");
        assert_eq!(
            (a.system.as_str(), a.invariant.as_deref()),
            ("Q1.stock", Some("no-negatives"))
        );
        assert!(
            CALENDAR.is_boundary(Pulse::Month, SimTime::from_minutes(a.tick)),
            "it fails at the month the stock goes negative"
        );
        assert!(a.diagnostics.len() > 1 && a.diagnostics.len() <= 1_000);
        assert!(a.diagnostics.last().is_some_and(|d| d.contains("FATAL")));
        ticks.push(a.tick);

        // The host writes it where GOV-004 §4 says, and reads it back.
        let path = crash_path(&a);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, a.to_json()).unwrap();
        let read = CrashArtifact::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(read, a);

        // Replay from the tuple to the instant: the same violation, at 1 and 8 threads.
        for threads in [1, 8] {
            let mut fresh = stock_engine(read.seed, 40, threads);
            assert_eq!(
                fresh.replay(&read),
                Ok(()),
                "seed {seed}, {threads} threads"
            );
        }
    }
    ticks.sort_unstable();
    ticks.dedup();
    assert!(
        ticks.len() > 1,
        "the failing instant depends on the seed: {ticks:?}"
    );
}

#[test]
fn t05_replay_refuses_another_tuple_and_detects_a_different_outcome() {
    let mut engine = stock_engine(3, 40, 1);
    assert!(engine.advance_until(ten_years(), |_, _| {}).is_err());
    let a = engine.crash_artifact().unwrap().clone();
    assert_eq!(
        stock_engine(4, 40, 1).replay(&a),
        Err(ReplayError::Tuple("seed"))
    );
    let mut other_build = stock_engine(3, 40, 1);
    other_build.set_provenance(Provenance {
        build: "other".to_owned(),
        ..provenance()
    });
    assert_eq!(other_build.replay(&a), Err(ReplayError::Tuple("build")));
    assert_eq!(
        stock_engine(3, 1, 1).replay(&a),
        Err(ReplayError::NoFailure),
        "a world without the violation"
    );
    let mut altered = a.clone();
    altered.error.push('!');
    assert!(matches!(
        stock_engine(3, 40, 1).replay(&altered),
        Err(ReplayError::Mismatch { .. })
    ));
    // A stopped engine is poisoned: it returns the same error and runs nothing (SPR-005 G-d).
    let first = engine.poisoned().cloned();
    assert!(matches!(first, Some(SimError::Invariant { .. })));
    assert_eq!(engine.advance_until(ten_years(), |_, _| {}).err(), first);
    assert_eq!(engine.crash_artifact(), Some(&a));
}

fn raise_flag(
    _: &Stub,
    _: PulseBinding,
    v: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    let at = v.read("flag.at")?[0];
    if c.now().minutes() == at {
        v.write("flag.up")?[0] = 1;
    }
    v.write("flag.runs")?[0] += 1;
    Ok(())
}

fn flag_down(_: &Stub, w: &WorldTables) -> Vec<InvariantResult> {
    let up = w.get("flag.up").map_or(0, |c| c[0]);
    vec![InvariantResult {
        name: "flag-down",
        holds: up == 0,
        detail: "the flag is up".to_owned(),
    }]
}

fn fail_step(
    s: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    if c.now() >= SimTime::from_minutes(3 * 1_440) {
        return Err(SimError::System {
            system: s.id,
            message: "the stub step fails".to_owned(),
        });
    }
    Ok(())
}

fn flag_engine(at: i64) -> Engine {
    let stubs = vec![
        Stub::new("F1.flag")
            .writes("flag.up")
            .writes("flag.at")
            .writes("flag.runs")
            .at(Pulse::Hour, Phase::Bookkeeping)
            .stepping(raise_flag)
            .checking(flag_down),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut tables = WorldTables::new();
    tables.insert("flag.up", vec![0]);
    tables.insert("flag.at", vec![at]);
    tables.insert("flag.runs", vec![0]);
    Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        registered,
        tables,
        SimTime::EPOCH,
    )
}

#[test]
fn t05_invariants_run_every_instant_after_the_bookkeeping_systems() {
    let at = 5 * 1_440 + 7 * 60;
    let mut engine = flag_engine(at);
    let mut instants = 0;
    let err = engine
        .advance_until(ten_years(), |_, _| instants += 1)
        .unwrap_err();
    assert_eq!(
        err,
        SimError::Invariant {
            system: "F1.flag",
            name: "flag-down",
            detail: "the flag is up".to_owned()
        }
    );
    let a = engine.crash_artifact().unwrap();
    assert_eq!((a.tick, a.phase.as_str()), (at, "Bookkeeping"));
    assert_eq!(
        instants,
        5 * 24 + 7,
        "every earlier hour instant passed its check"
    );
    assert_eq!(engine.tables().get("flag.runs").unwrap()[0], 5 * 24 + 8);

    // A step error leaves an artifact too, with no invariant.
    let stubs = vec![
        Stub::new("F2.fail")
            .at(Pulse::Day, Phase::Polity)
            .stepping(fail_step),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap();
    let mut e = Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        registered,
        WorldTables::new(),
        SimTime::EPOCH,
    );
    assert!(e.advance_until(ten_years(), |_, _| {}).is_err());
    let a = e.crash_artifact().unwrap();
    assert_eq!(
        (
            a.tick,
            a.phase.as_str(),
            a.system.as_str(),
            a.invariant.as_deref()
        ),
        (3 * 1_440, "Polity", "F2.fail", None)
    );
    assert_eq!(a.error, "F2.fail: the stub step fails");
}

/// The committed crash artifact of the stock world at seed 3.
const CRASH_GOLDEN: &str = "tests/golden/crash_stock_seed3.json";

#[test]
fn t05_the_committed_crash_artifact_replays() {
    let mut engine = stock_engine(3, 40, 1);
    assert!(engine.advance_until(ten_years(), |_, _| {}).is_err());
    let produced = engine.crash_artifact().unwrap().to_json();
    let default = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CRASH_GOLDEN);
    if std::env::var_os("RH_CORE_WRITE_CRASH_GOLDEN").is_some() {
        std::fs::write(&default, &produced).unwrap();
    }
    let path = std::env::var_os("RH_CORE_CRASH_ARTIFACT").map_or(default.clone(), PathBuf::from);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let artifact =
        CrashArtifact::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    if let Err(e) = stock_engine(artifact.seed, 40, 1).replay(&artifact) {
        panic!("the crash artifact {} does not replay: {e}", path.display());
    }
    if path == default {
        assert_eq!(
            text, produced,
            "{CRASH_GOLDEN} differs from the run: a C3 change of the crash path"
        );
    }
    println!("T-05: crash artifact {} replays", artifact.file_name());
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "it must have the StepFn signature"
)]
fn panic_step(
    _: &Stub,
    _: PulseBinding,
    _: &mut SystemView<'_>,
    c: &mut StepCtx<'_>,
) -> Result<(), SimError> {
    assert!(c.now() < SimTime::from_minutes(2 * 1_440), "boom at day 2");
    Ok(())
}

/// Four independent systems in one level, so at 4 threads the panic happens on a worker.
fn panic_engine(threads: usize) -> Engine {
    let stubs = vec![
        Stub::new("P1.a").at(Pulse::Day, Phase::Polity),
        Stub::new("P2.boom")
            .at(Pulse::Day, Phase::Polity)
            .stepping(panic_step),
        Stub::new("P3.c").at(Pulse::Day, Phase::Polity),
        Stub::new("P4.d").at(Pulse::Day, Phase::Polity),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs).unwrap();
    let mut e = Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        registered,
        WorldTables::new(),
        SimTime::EPOCH,
    );
    e.set_threads(threads);
    e
}

#[test]
fn def1_a_panicking_step_poisons_the_engine_with_a_crash_artifact() {
    for threads in [1, 4] {
        let mut e = panic_engine(threads);
        let err = e.advance_until(ten_years(), |_, _| {}).unwrap_err();
        assert!(
            matches!(&err, SimError::Panic { system: "P2.boom", message } if message == "boom at day 2"),
            "{threads} threads: {err}"
        );
        assert_eq!(e.poisoned(), Some(&err));
        let a = e.crash_artifact().unwrap();
        assert_eq!(
            (
                a.tick,
                a.phase.as_str(),
                a.system.as_str(),
                a.invariant.as_deref()
            ),
            (2 * 1_440, "Polity", "P2.boom", None)
        );
        assert_eq!(a.error, "P2.boom panicked: boom at day 2");
        for replay_threads in [1, 4] {
            assert_eq!(
                panic_engine(replay_threads).replay(a),
                Ok(()),
                "the panic replays"
            );
        }
    }
}

#[test]
fn def2_an_artifact_after_material_inputs_records_them_and_replay_requires_the_journal() {
    let mut engine = stock_engine(3, 40, 1);
    let scope: std::collections::BTreeSet<_> = [rh_ids::EntityRef::region(RegionId::from_raw(2))]
        .into_iter()
        .collect();
    for at in [1_440, 2 * 1_440 + 5] {
        engine
            .submit_input(
                SimTime::from_minutes(at),
                MaterialInput {
                    scope: scope.clone(),
                },
            )
            .unwrap();
    }
    assert!(engine.advance_until(ten_years(), |_, _| {}).is_err());
    let a = engine.crash_artifact().unwrap().clone();
    assert_eq!(a.journal_position, 2, "two material inputs were applied");
    assert_eq!(
        stock_engine(3, 40, 1).replay(&a),
        Err(ReplayError::JournalRequired { position: 2 }),
        "no journal can be supplied before SPR-007: an explicit refusal, not NoFailure"
    );
    // Without inputs the position is 0 and the artifact replays (t05 tests above).
    let mut plain = stock_engine(3, 40, 1);
    assert!(plain.advance_until(ten_years(), |_, _| {}).is_err());
    assert_eq!(plain.crash_artifact().unwrap().journal_position, 0);
}

fn lag_engine(lag: rh_core::access::Lag, pulse: Pulse) -> Engine {
    let stubs = vec![
        Stub::new("L1.w")
            .writes("w.x")
            .at(Pulse::Day, Phase::Production),
        Stub::new("L2.r")
            .reads("w.x", "L1.w", lag)
            .at(pulse, Phase::Polity),
    ];
    let specs = agreeing_specs(&stubs);
    let registered = register(stubs.into_iter().map(Stub::boxed).collect(), &specs)
        .unwrap_or_else(|e| panic!("registration accepts every lag: {e}"));
    Engine::new(
        CALENDAR,
        1,
        BranchSalt::from_raw(0),
        registered,
        WorldTables::new(),
        SimTime::EPOCH,
    )
}

#[test]
fn a28_lags_p0_cannot_run_exactly_are_refused_by_the_engine() {
    use rh_core::access::Lag;
    for (lag, pulse, name) in [
        (Lag::PreviousPulse, Pulse::Day, "previous_pulse"),
        (Lag::PreviousPulse, Pulse::Month, "previous_pulse"),
        (Lag::StartOfDay, Pulse::Hour, "start_of_day"),
    ] {
        let mut e = lag_engine(lag, pulse);
        let expected = SimError::Unsupported {
            system: "L2.r",
            column: "w.x",
            lag: name,
        };
        assert_eq!(e.poisoned(), Some(&expected), "{name} on {pulse}");
        assert_eq!(
            e.advance_until(ten_years(), |_, _| {}).err(),
            Some(expected),
            "nothing runs"
        );
        assert!(e.crash_artifact().is_none(), "refused before any instant");
    }
    // The lags P0 runs exactly: lagged anywhere, start_of_day on a Day (or coarser) pulse.
    for (lag, pulse) in [
        (Lag::Lagged, Pulse::Hour),
        (Lag::StartOfDay, Pulse::Day),
        (Lag::StartOfDay, Pulse::Month),
        (Lag::SameInstant, Pulse::Hour),
    ] {
        let mut e = lag_engine(lag, pulse);
        assert_eq!(e.poisoned(), None, "{lag} on {pulse}");
        assert!(
            e.advance_until(SimTime::from_minutes(3 * 1_440), |_, _| {})
                .is_ok()
        );
    }
}
