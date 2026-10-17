//! The engine view of GOV-006 §1: per-system times, the serial fraction and the vitals line, timed
//! by the host from the engine's clock-free notices (SPR-009 T-03, ruling 4).
//!
//! Timing never happens inside plane A. The engine sends notices at instant, phase, level and job
//! boundaries (`rh_core::observe`) and carries no clock; [`Timing`] is the host's observer, in the
//! tool plane, and reads the clock when a notice arrives. Attaching it changes no result:
//! `bin/check_all.sh` runs its N-thread golden run (1,000 years) with `--vitals` and compares that
//! ladder with the golden baseline and the 1-thread ladder byte for byte.
//!
//! - **Per-system time:** the wall time between each job's `JobStart` and `JobEnd`, summed per
//!   system (on several threads, jobs of one level overlap; each is its own system's time).
//! - **Serial fraction** (`docs/08` §9 bench 3; the gate there is P1a): the share of the run's wall
//!   time that is not inside a parallel level, a level with two or more jobs, between its
//!   `LevelStart` and `LevelMerge`. Merges, single-job levels and everything between levels are
//!   serial.
//! - **The vitals line** (GOV-006 §1, "tip, pace, ticks/s, plane hashes"): the last instant run, sim
//!   years per wall second, instants per wall second, the serial fraction, the three plane hashes,
//!   and the systems by share of job time.
//!
//! All arithmetic is in integer nanoseconds; shares are printed in tenths of a percent.
//!
//! REF: GOV-006 §1, §2; `docs/08` §9 bench 3; `CODEX/05_PROJECT/SPR-009_Benchmarks.md` T-03,
//! ruling 4.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    reason = "tool plane: the host reads the clock, the engine never does (SPR-009 ruling 4)"
)]

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rh_core::observe::{Boundary, Notice, Observer};
use rh_core::time::SimTime;

/// Minutes in a 365-day sim-year (the smoke calendar).
const MINUTES_PER_YEAR: u128 = 365 * 24 * 60;
const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// The clock `Timing` reads: wall time since some fixed start. Injected so the arithmetic can be
/// tested on scripted notices (VER-SPR-009-TEST GAP-13); [`Timing::default`] reads `Instant`.
type Clock = Box<dyn Fn() -> Duration + Send + Sync>;

/// What the observer has seen so far. Times are readings of the clock.
#[derive(Default)]
struct State {
    first: Option<Duration>,
    last: Option<Duration>,
    first_at: Option<SimTime>,
    tip: Option<SimTime>,
    instants: u64,
    /// The running job on each worker: its system and when it started.
    running: Vec<Option<(&'static str, Duration)>>,
    per_system: BTreeMap<&'static str, Duration>,
    /// The open level: when it started and how many jobs it holds.
    level: Option<(Duration, usize)>,
    parallel: Duration,
}

/// The host's timing observer (see the module documentation).
pub struct Timing {
    clock: Clock,
    state: Mutex<State>,
}

impl Default for Timing {
    /// A timing observer on the wall clock.
    fn default() -> Self {
        let start = Instant::now();
        Self::with_clock(move || start.elapsed())
    }
}

impl Timing {
    /// A timing observer on the given clock (wall time since a fixed start).
    #[must_use]
    pub fn with_clock(clock: impl Fn() -> Duration + Send + Sync + 'static) -> Self {
        Self {
            clock: Box::new(clock),
            state: Mutex::new(State::default()),
        }
    }
}

impl Observer for Timing {
    fn notify(&self, notice: &Notice) {
        let now = (self.clock)();
        // A poisoned lock means an earlier notice panicked; keep timing with what is there.
        let mut s = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match notice.boundary {
            Boundary::InstantStart => {
                s.first.get_or_insert(now);
                s.first_at.get_or_insert(notice.at);
            }
            Boundary::InstantEnd => {
                s.last = Some(now);
                s.tip = Some(notice.at);
                s.instants += 1;
            }
            Boundary::LevelStart { jobs, .. } => s.level = Some((now, jobs)),
            Boundary::LevelMerge => {
                if let Some((start, jobs)) = s.level.take()
                    && jobs >= 2
                {
                    s.parallel += now.saturating_sub(start);
                }
            }
            Boundary::JobStart(job) => {
                if s.running.len() <= notice.worker {
                    s.running.resize(notice.worker + 1, None);
                }
                s.running[notice.worker] = Some((job.system, now));
            }
            Boundary::JobEnd(_) => {
                if let Some((system, start)) =
                    s.running.get_mut(notice.worker).and_then(Option::take)
                {
                    *s.per_system.entry(system).or_default() += now.saturating_sub(start);
                }
            }
            Boundary::PhaseStart | Boundary::PhaseEnd | Boundary::LevelEnd => {}
        }
    }
}

/// A finished run's vitals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vitals {
    /// The last instant run.
    pub tip: SimTime,
    /// Sim-minutes from the first instant to the tip.
    pub sim_minutes: u128,
    /// Instants run.
    pub instants: u64,
    /// Wall time from the first instant's start to the last instant's end.
    pub wall: Duration,
    /// Wall time inside parallel levels.
    pub parallel: Duration,
    /// Job time per system, in system order.
    pub per_system: Vec<(&'static str, Duration)>,
}

impl Timing {
    /// The vitals so far, or `None` before the first instant has ended.
    #[must_use]
    pub fn vitals(&self) -> Option<Vitals> {
        let s = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (first, last, tip, first_at) = (s.first?, s.last?, s.tip?, s.first_at?);
        Some(Vitals {
            tip,
            sim_minutes: u128::try_from(tip.minutes() - first_at.minutes()).unwrap_or(0),
            instants: s.instants,
            wall: last.saturating_sub(first),
            parallel: s.parallel,
            per_system: s.per_system.iter().map(|(k, v)| (*k, *v)).collect(),
        })
    }
}

/// `part / whole` in tenths of a percent, as `d.d%`.
fn percent(part: u128, whole: u128) -> String {
    let tenths = (part * 1000).checked_div(whole).unwrap_or(0);
    format!("{}.{}%", tenths / 10, tenths % 10)
}

/// `count / unit` per wall second, with one decimal, from the exact ratio (never truncated to
/// whole units first: DEF-6, a 1-year run printed a pace of 0.0).
fn per_second(count: u128, unit: u128, wall: Duration) -> String {
    let tenths = (count * NANOS_PER_SECOND * 10)
        .checked_div(unit * wall.as_nanos())
        .unwrap_or(0);
    format!("{}.{}", tenths / 10, tenths % 10)
}

impl Vitals {
    /// The serial fraction, in tenths of a percent of the wall time.
    #[must_use]
    pub fn serial_permille(&self) -> u128 {
        let wall = self.wall.as_nanos();
        (wall.saturating_sub(self.parallel.as_nanos()) * 1000)
            .checked_div(wall)
            .unwrap_or(0)
    }

    /// The vitals line (GOV-006 §1). `date` renders the tip; `hashes` are the plane hashes A, B
    /// and C at the tip.
    #[must_use]
    pub fn line(&self, date: &str, hashes: [u64; 3]) -> String {
        let wall = self.wall.as_nanos();
        let job_total: u128 = self.per_system.iter().map(|(_, d)| d.as_nanos()).sum();
        let mut systems: Vec<&(&str, Duration)> = self.per_system.iter().collect();
        systems.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let top: Vec<String> = systems
            .iter()
            .map(|(s, d)| format!("{s} {}", percent(d.as_nanos(), job_total)))
            .collect();
        format!(
            "vitals: tip {date} | pace {} sim-years/s | {} ticks/s | serial {} of {} ms | \
             state {:016x} observation {:016x} narrative {:016x} | job time: {}",
            per_second(self.sim_minutes, MINUTES_PER_YEAR, self.wall),
            per_second(u128::from(self.instants), 1, self.wall),
            percent(wall.saturating_sub(self.parallel.as_nanos()), wall),
            self.wall.as_millis(),
            hashes[0],
            hashes[1],
            hashes[2],
            top.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shares_and_rates_are_integer_arithmetic() {
        assert_eq!(percent(1, 3), "33.3%");
        assert_eq!(percent(0, 0), "0.0%");
        assert_eq!(per_second(5, 1, Duration::from_secs(2)), "2.5");
        let v = Vitals {
            tip: SimTime::from_minutes(0),
            sim_minutes: 0,
            instants: 0,
            wall: Duration::from_secs(1),
            parallel: Duration::from_millis(250),
            per_system: vec![
                ("X1", Duration::from_millis(30)),
                ("X2", Duration::from_millis(10)),
            ],
        };
        assert_eq!(v.serial_permille(), 750);
        let line = v.line("0001-01-01", [1, 2, 3]);
        assert!(line.contains("serial 75.0% of 1000 ms"), "{line}");
        assert!(line.ends_with("job time: X1 75.0%, X2 25.0%"), "{line}");
    }

    use rh_core::Phase;
    use rh_core::observe::{JobRef, JobWork, Pass};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A timing observer on a scripted clock, and the means to set the clock (milliseconds).
    fn scripted() -> (Timing, Arc<AtomicU64>) {
        let ms = Arc::new(AtomicU64::new(0));
        let read = Arc::clone(&ms);
        let timing = Timing::with_clock(move || Duration::from_millis(read.load(Ordering::SeqCst)));
        (timing, ms)
    }

    fn job(system: &'static str, index: usize) -> JobRef {
        JobRef {
            system,
            work: JobWork::Timers(1),
            index,
        }
    }

    /// Where the scripted clock starts: far from zero, so a mistake about the origin (a wall time
    /// taken from the clock's zero, or `first` scaled) changes a result (VER-SPR-009-TEST GAP-22).
    const ORIGIN_MS: u64 = 3_600_000;

    /// Delivers `boundary` at clock time `ORIGIN_MS + ms`, instant `at` (minutes), from `worker`.
    fn send(t: &(Timing, Arc<AtomicU64>), ms: u64, at: i64, worker: usize, boundary: Boundary) {
        t.1.store(ORIGIN_MS + ms, Ordering::SeqCst);
        let phase = match boundary {
            Boundary::InstantStart | Boundary::InstantEnd => None,
            _ => Some(Phase::Activity),
        };
        t.0.notify(&Notice {
            at: SimTime::from_minutes(at),
            phase,
            worker,
            boundary,
        });
    }

    /// Half a 365-day sim-year, in minutes.
    const HALF_YEAR: i64 = 365 * 24 * 60 / 2;

    fn level(jobs: usize) -> Boundary {
        Boundary::LevelStart {
            pass: Pass::Bindings,
            level: 0,
            jobs,
        }
    }

    /// One instant at minute `at`, from clock `t0`: a one-job level (X1, 10 ms), then a two-job
    /// level of 40 ms. At one worker X2 runs 20 ms, then X3 20 ms. At two workers they overlap,
    /// interleaved: X2 on worker 0 for 30 ms, X3 on worker 1 for 20 ms, starting after X2 and
    /// ending before it. The instant ends at `t0 + 100`.
    fn instant(t: &(Timing, Arc<AtomicU64>), t0: u64, at: i64, workers: usize) {
        send(t, t0, at, 0, Boundary::InstantStart);
        send(t, t0 + 5, at, 0, level(1));
        send(t, t0 + 5, at, 0, Boundary::JobStart(job("X1", 0)));
        send(t, t0 + 15, at, 0, Boundary::JobEnd(job("X1", 0)));
        send(t, t0 + 15, at, 0, Boundary::LevelMerge);
        send(t, t0 + 20, at, 0, level(2));
        send(t, t0 + 20, at, 0, Boundary::JobStart(job("X2", 0)));
        if workers == 1 {
            send(t, t0 + 40, at, 0, Boundary::JobEnd(job("X2", 0)));
            send(t, t0 + 40, at, 0, Boundary::JobStart(job("X3", 1)));
            send(t, t0 + 60, at, 0, Boundary::JobEnd(job("X3", 1)));
        } else {
            send(t, t0 + 25, at, 1, Boundary::JobStart(job("X3", 1)));
            send(t, t0 + 45, at, 1, Boundary::JobEnd(job("X3", 1)));
            send(t, t0 + 50, at, 0, Boundary::JobEnd(job("X2", 0)));
        }
        send(t, t0 + 60, at, 0, Boundary::LevelMerge);
        send(t, t0 + 100, at, 0, Boundary::InstantEnd);
    }

    /// GAP-13: the arithmetic over scripted notices, at one worker and at two. Each mutant of
    /// VER-SPR-009-TEST fails one assertion: V1 (a one-job level counted as parallel), V2
    /// (per-system time scaled), V3 (the tip stuck), V4 (every `JobStart` on worker 0), V6
    /// (instants not counted), V7 (the pace scaled).
    #[test]
    fn timing_over_scripted_notices_at_one_and_two_workers() {
        for (workers, x2_ms) in [(1, 20), (2, 30)] {
            let t = scripted();
            instant(&t, 0, 0, workers);
            instant(&t, 100, HALF_YEAR, workers);
            let v = t.0.vitals().expect("two instants ended");
            assert_eq!(v.tip, SimTime::from_minutes(HALF_YEAR), "V3");
            assert_eq!(v.instants, 2, "V6");
            assert_eq!(v.wall, Duration::from_millis(200));
            // Only the two-job levels count, 40 ms each; the one-job levels are serial (V1).
            assert_eq!(
                v.parallel,
                Duration::from_millis(80),
                "V1 at {workers} worker(s)"
            );
            assert_eq!(v.serial_permille(), 600);
            // Per-system time is exact (V2). At two workers X3 starts on worker 1 while X2 runs
            // on worker 0; with every start filed under worker 0, X2's time would be lost (V4).
            assert_eq!(
                v.per_system,
                vec![
                    ("X1", Duration::from_millis(20)),
                    ("X2", Duration::from_millis(2 * x2_ms)),
                    ("X3", Duration::from_millis(40)),
                ],
                "V2/V4 at {workers} worker(s)"
            );
            // Half a sim-year in 0.2 s: 2.5 sim-years/s; 2 instants: 10.0 ticks/s (V7).
            let line = v.line("0000-07", [0, 0, 0]);
            assert!(
                line.contains("pace 2.5 sim-years/s | 10.0 ticks/s"),
                "{line}"
            );
            assert!(line.contains("serial 60.0% of 200 ms"), "{line}");
        }
    }

    /// DEF-6: a run shorter than a year still has its pace; nothing is truncated to whole years.
    #[test]
    fn the_pace_of_a_run_under_a_year_is_not_truncated() {
        let v = Vitals {
            tip: SimTime::from_minutes(0),
            // One year less a month, in 70 ms: 12.9 sim-years/s (VER-SPR-009-TEST DEF-6).
            sim_minutes: MINUTES_PER_YEAR * 334 / 365,
            instants: 1,
            wall: Duration::from_millis(71),
            parallel: Duration::ZERO,
            per_system: vec![],
        };
        let line = v.line("0000-12", [0, 0, 0]);
        assert!(line.contains("pace 12.8 sim-years/s"), "{line}");
        assert_eq!(
            per_second(
                MINUTES_PER_YEAR / 10,
                MINUTES_PER_YEAR,
                Duration::from_secs(1)
            ),
            "0.1"
        );
    }

    /// No vitals before an instant has ended; a job end without its start adds nothing.
    #[test]
    fn partial_sequences_give_no_vitals_and_no_stray_time() {
        let t = scripted();
        assert_eq!(t.0.vitals(), None);
        send(&t, 0, 0, 0, Boundary::InstantStart);
        assert_eq!(t.0.vitals(), None);
        send(&t, 5, 0, 3, Boundary::JobEnd(job("X9", 0)));
        send(&t, 10, 0, 0, Boundary::InstantEnd);
        let v = t.0.vitals().expect("one instant ended");
        assert!(v.per_system.is_empty());
        assert_eq!((v.instants, v.wall), (1, Duration::from_millis(10)));
    }
}
