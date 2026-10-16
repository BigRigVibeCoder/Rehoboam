//! Observer invariance (SPR-009 T-03, ruling 4; V-DET-07 style): a run with a host observer
//! attached hashes equal, month by month, to the same run without one, at one thread and at four,
//! with material inputs re-sampling timers on the way. The observer here does what a timing host
//! does: it takes a lock and records every notice, from every worker thread.
//!
//! It also pins the notice protocol the host relies on (`rh_core::observe`): instants, phases and
//! levels open and close in order, every job starts and ends inside its level, and the jobs a
//! level announces are the jobs that run.
//!
//! The observer cannot reach plane-A state: a notice holds copies and `&'static str` ids, and
//! `notify` takes `&self` (the type system enforces it; this test checks the result).
//!
//! REF: `CODEX/05_PROJECT/SPR-009_Benchmarks.md` T-03, ruling 4; `docs/09` V-DET-07; GOV-004 law 2.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests"
)]

mod life;
mod support;

use std::sync::{Arc, Mutex};

use life::{CALENDAR, DAY, entity, life_engine};
use rh_core::engine::{Engine, MaterialInput};
use rh_core::observe::{Boundary, Notice, Observer};
use rh_core::system::SimError;
use rh_core::time::SimTime;
use rh_snapshot::journal::{JournalPosition, chain_genesis};
use rh_snapshot::keyframe::{KeyframeError, write_keyframe};
use rh_snapshot::state_hash;
use std::sync::atomic::{AtomicUsize, Ordering};

const SEED: u64 = 21;
const YEAR: i64 = 365 * DAY;
const YEARS: i64 = 6;

/// Records every notice it receives.
#[derive(Default)]
struct Recorder(Mutex<Vec<Notice>>);

impl Observer for Recorder {
    fn notify(&self, notice: &Notice) {
        self.0.lock().unwrap().push(*notice);
    }
}

/// The life world at `threads`, with three material inputs.
fn world(threads: usize) -> Engine {
    let mut e = life_engine(SEED, 64, 3, false, threads);
    for (day, step) in [(40, 3), (500, 5), (1_200, 2)] {
        let scope = (0..64).step_by(step).map(entity).collect();
        e.submit_input(
            SimTime::from_minutes(day * DAY + 7),
            MaterialInput { scope },
        )
        .unwrap();
    }
    e
}

/// `state_hash` at every month boundary for `YEARS` years.
fn monthly(e: &mut Engine) -> Vec<(i64, u64)> {
    let mut out = Vec::new();
    while let Some(t) = e.next_instant().filter(|t| t.minutes() <= YEARS * YEAR) {
        e.advance_until(t, |_, _| {})
            .unwrap_or_else(|err| panic!("{err}"));
        if CALENDAR.is_boundary(rh_core::Pulse::Month, t) {
            out.push((t.minutes(), state_hash(&e.snapshot())));
        }
    }
    out
}

#[test]
fn v_det_07_style_an_attached_observer_changes_no_hash() {
    for threads in [1, 4] {
        let plain = monthly(&mut world(threads));
        let recorder = Arc::new(Recorder::default());
        let mut watched = world(threads);
        watched.set_observer(Some(recorder.clone()));
        let observed = monthly(&mut watched);
        // Month boundaries from the epoch to the end, both included.
        assert_eq!(plain.len(), usize::try_from(YEARS * 12 + 1).unwrap());
        assert_eq!(
            plain, observed,
            "the observer changed a hash at {threads} threads"
        );
        let notices = recorder.0.lock().unwrap().len();
        assert!(notices > 10_000, "the observer saw only {notices} notices");
        println!(
            "observer invariance: {} monthly hashes equal at {threads} threads, {notices} notices",
            plain.len()
        );
    }
}

#[test]
fn detaching_the_observer_mid_run_changes_no_hash() {
    let plain = monthly(&mut world(2));
    let mut e = world(2);
    e.set_observer(Some(Arc::new(Recorder::default())));
    e.advance_until(SimTime::from_minutes(2 * YEAR), |_, _| {})
        .unwrap();
    e.set_observer(None);
    let rest = monthly(&mut e);
    assert_eq!(&plain[plain.len() - rest.len()..], rest.as_slice());
}

/// Checks the notice protocol of `rh_core::observe` over a whole run.
fn check_protocol(notices: &[Notice]) -> usize {
    let mut instants = 0;
    let (mut in_instant, mut in_phase, mut level_jobs) = (false, false, None::<(usize, usize)>);
    for n in notices {
        match n.boundary {
            Boundary::InstantStart => {
                assert!(!in_instant && n.phase.is_none());
                in_instant = true;
            }
            Boundary::PhaseStart => {
                assert!(in_instant && !in_phase);
                in_phase = true;
            }
            Boundary::LevelStart { jobs, .. } => {
                assert!(in_phase && level_jobs.is_none());
                level_jobs = Some((jobs, 0));
            }
            Boundary::JobStart(_) => assert!(level_jobs.is_some()),
            Boundary::JobEnd(_) => {
                let (want, ended) = level_jobs.as_mut().expect("a job outside a level");
                *ended += 1;
                assert!(*ended <= *want);
            }
            Boundary::LevelMerge => {
                let (want, ended) = level_jobs.expect("a merge outside a level");
                assert_eq!(want, ended, "the merge began before every job ended");
            }
            Boundary::LevelEnd => assert!(level_jobs.take().is_some()),
            Boundary::PhaseEnd => {
                assert!(in_phase && level_jobs.is_none());
                in_phase = false;
            }
            Boundary::InstantEnd => {
                assert!(in_instant && !in_phase);
                in_instant = false;
                instants += 1;
            }
        }
    }
    assert!(!in_instant);
    instants
}

#[test]
fn the_notices_follow_the_protocol_at_every_thread_count() {
    let mut counts = Vec::new();
    for threads in [1, 2, 4] {
        let recorder = Arc::new(Recorder::default());
        let mut e = world(threads);
        e.set_observer(Some(recorder.clone()));
        e.advance_until(SimTime::from_minutes(YEAR), |_, _| {})
            .unwrap();
        let notices = recorder.0.lock().unwrap();
        let instants = check_protocol(&notices);
        assert!(instants > 300, "{instants} instants in a year");
        // Every notice but a job's comes from worker 0; jobs spread only when there are threads.
        assert!(notices.iter().all(|n| n.worker < threads));
        assert!(notices.iter().all(|n| {
            matches!(n.boundary, Boundary::JobStart(_) | Boundary::JobEnd(_)) || n.worker == 0
        }));
        let mut kinds: Vec<String> = notices
            .iter()
            .map(|n| format!("{:?} {:?} {:?}", n.at, n.phase, n.boundary))
            .collect();
        kinds.sort();
        counts.push(kinds);
    }
    // The same notices (as a multiset) at every thread count; only their worker differs.
    assert_eq!(counts[0], counts[1]);
    assert_eq!(counts[0], counts[2]);
}

/// Panics at the `nth` notice of one kind (SPR-009 DEF-1).
struct Panicking {
    kind: &'static str,
    nth: usize,
    seen: AtomicUsize,
}

fn kind(b: &Boundary) -> &'static str {
    match b {
        Boundary::InstantStart => "InstantStart",
        Boundary::PhaseStart => "PhaseStart",
        Boundary::LevelStart { .. } => "LevelStart",
        Boundary::JobStart(_) => "JobStart",
        Boundary::JobEnd(_) => "JobEnd",
        Boundary::LevelMerge => "LevelMerge",
        Boundary::LevelEnd => "LevelEnd",
        Boundary::PhaseEnd => "PhaseEnd",
        Boundary::InstantEnd => "InstantEnd",
    }
}

impl Observer for Panicking {
    fn notify(&self, notice: &Notice) {
        if kind(&notice.boundary) == self.kind
            && self.seen.fetch_add(1, Ordering::SeqCst) + 1 == self.nth
        {
            panic!("observer boom at {}", self.kind);
        }
    }
}

const KINDS: [&str; 9] = [
    "InstantStart",
    "PhaseStart",
    "LevelStart",
    "JobStart",
    "JobEnd",
    "LevelMerge",
    "LevelEnd",
    "PhaseEnd",
    "InstantEnd",
];

/// The tables' shape: every column's name and length.
fn shape(e: &Engine) -> Vec<(String, usize)> {
    e.tables()
        .columns()
        .map(|(n, _, rows)| (n.to_owned(), rows.len()))
        .collect()
}

#[test]
fn def_1_a_panicking_observer_poisons_the_engine_at_every_notice_kind() {
    for threads in [1, 4] {
        for k in KINDS {
            let mut e = world(threads);
            let before = shape(&e);
            e.set_observer(Some(Arc::new(Panicking {
                kind: k,
                nth: 200,
                seen: AtomicUsize::new(0),
            })));
            let err = e
                .advance_until(SimTime::from_minutes(YEARS * YEAR), |_, _| {})
                .expect_err(&format!(
                    "a panic at {k} must stop the run at {threads} threads"
                ));
            assert!(
                matches!(&err, SimError::Observer { notice, message } if notice.starts_with(k) && message == &format!("observer boom at {k}")),
                "{k} at {threads} threads: {err}"
            );
            let artifact = e.crash_artifact().expect("a crash artifact");
            assert_eq!(artifact.system, rh_core::engine::OBSERVER);
            assert!(artifact.error.contains(k), "{}", artifact.error);
            assert_eq!(e.poisoned(), Some(&err));
            // Poisoned: it runs nothing more and repeats the error.
            assert_eq!(
                e.advance_until(SimTime::from_minutes(YEARS * YEAR), |_, _| {}),
                Err(err)
            );
            // No column was lost to the unwind (the defect): every column is back, full length.
            assert_eq!(shape(&e), before, "{k} at {threads} threads lost a column");
            // The poison path refuses a keyframe of this state.
            let position = JournalPosition {
                timeline_id: 0,
                inputs: 3,
                head: chain_genesis(0),
            };
            assert!(matches!(
                write_keyframe(&e, position),
                Err(KeyframeError::Poisoned(_))
            ));
        }
    }
}

#[test]
fn def_1_the_failing_instant_is_the_same_at_one_and_four_threads() {
    for k in ["JobStart", "JobEnd", "LevelMerge"] {
        // The tick and the error's text: the text names the notice, never the worker (review 8).
        let outcomes: Vec<(i64, String)> = [1, 4]
            .into_iter()
            .map(|threads| {
                let mut e = world(threads);
                e.set_observer(Some(Arc::new(Panicking {
                    kind: k,
                    nth: 500,
                    seen: AtomicUsize::new(0),
                })));
                let err = e
                    .advance_until(SimTime::from_minutes(YEARS * YEAR), |_, _| {})
                    .expect_err("the observer panics");
                (
                    e.crash_artifact().expect("an artifact").tick,
                    err.to_string(),
                )
            })
            .collect();
        assert_eq!(outcomes[0], outcomes[1], "{k}");
        assert!(!outcomes[0].1.contains("worker"), "{}", outcomes[0].1);
    }
}

/// DEF-2 (VER-SPR-009-TEST): the content of every notice, checked against what the run itself
/// says, independently of the engine's notify code:
/// - every notice's `at` is the instant `advance_until` reports to its callback;
/// - instant notices carry no phase; phases open and close in `Phase::ALL` order, and every notice
///   between a phase's start and end carries that phase;
/// - a job notice carries its level's instant and phase, and comes from worker
///   `index % min(threads, jobs)`; every other notice from worker 0.
fn check_contents(notices: &[Notice], instants: &[SimTime], threads: usize) {
    let mut instant = instants.iter();
    let (mut at, mut phases, mut phase) = (None::<SimTime>, Vec::new(), None::<rh_core::Phase>);
    let mut level_jobs = 0;
    for n in notices {
        match n.boundary {
            Boundary::InstantStart => {
                at = instant.next().copied();
                phases.clear();
                assert_eq!(n.phase, None);
            }
            Boundary::InstantEnd => {
                assert_eq!(n.phase, None);
                assert_eq!(phases, rh_core::Phase::ALL.to_vec(), "phases at {at:?}");
            }
            Boundary::PhaseStart => {
                phase = n.phase;
                phases.push(n.phase.expect("PhaseStart carries its phase"));
            }
            Boundary::LevelStart { jobs, .. } => level_jobs = jobs,
            _ => {}
        }
        assert_eq!(Some(n.at), at, "{n:?}: not the engine's instant");
        if !matches!(n.boundary, Boundary::InstantStart | Boundary::InstantEnd) {
            assert_eq!(n.phase, phase, "{n:?}: not the current phase");
        }
        match n.boundary {
            Boundary::JobStart(job) | Boundary::JobEnd(job) => {
                assert_eq!(n.worker, job.index % threads.min(level_jobs), "{n:?}");
            }
            _ => assert_eq!(n.worker, 0, "{n:?}: an engine-thread notice off worker 0"),
        }
    }
    assert_eq!(instant.next(), None, "instants without notices");
}

#[test]
fn def_2_every_notice_carries_the_engine_s_instant_phase_and_worker() {
    for threads in [1, 2, 4] {
        let recorder = Arc::new(Recorder::default());
        let mut e = world(threads);
        e.set_observer(Some(recorder.clone()));
        let mut instants = Vec::new();
        e.advance_until(SimTime::from_minutes(YEAR), |t, _| instants.push(t))
            .unwrap();
        let notices = recorder.0.lock().unwrap();
        assert!(instants.len() > 300);
        assert!(
            notices.iter().any(|n| n.worker > 0) || threads == 1,
            "jobs spread over workers at {threads} threads"
        );
        check_contents(&notices, &instants, threads);
    }
}

/// Panics at the `nth` notice of one kind, remembering that notice's instant (OBS-1).
struct PanickingAt {
    kind: &'static str,
    nth: usize,
    seen: AtomicUsize,
    at: std::sync::atomic::AtomicI64,
}

impl Observer for PanickingAt {
    fn notify(&self, notice: &Notice) {
        if kind(&notice.boundary) == self.kind
            && self.seen.fetch_add(1, Ordering::SeqCst) + 1 == self.nth
        {
            self.at.store(notice.at.minutes(), Ordering::SeqCst);
            panic!("observer boom at {}", self.kind);
        }
    }
}

/// VER-SPR-009-TEST OBS-1: whichever notice the observer fails on, the crash artifact's tick is
/// the instant of that notice, `InstantStart` included (the engine records its instant before the
/// first notice).
#[test]
fn obs_1_the_artifact_names_the_failing_instant_at_every_notice_kind() {
    for threads in [1, 4] {
        for k in KINDS {
            let observer = Arc::new(PanickingAt {
                kind: k,
                nth: 300,
                seen: AtomicUsize::new(0),
                at: std::sync::atomic::AtomicI64::new(-1),
            });
            let mut e = world(threads);
            e.set_observer(Some(observer.clone()));
            let _err = e.advance_until(SimTime::from_minutes(YEARS * YEAR), |_, _| {});
            let tick = e.crash_artifact().expect("an artifact").tick;
            assert_eq!(
                tick,
                observer.at.load(Ordering::SeqCst),
                "{k} at {threads} threads"
            );
        }
    }
}
