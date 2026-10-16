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

/// What the observer has seen so far.
#[derive(Default)]
struct State {
    first: Option<Instant>,
    last: Option<Instant>,
    first_at: Option<SimTime>,
    tip: Option<SimTime>,
    instants: u64,
    /// The running job on each worker: its system and when it started.
    running: Vec<Option<(&'static str, Instant)>>,
    per_system: BTreeMap<&'static str, Duration>,
    /// The open level: when it started and how many jobs it holds.
    level: Option<(Instant, usize)>,
    parallel: Duration,
}

/// The host's timing observer (see the module documentation).
#[derive(Default)]
pub struct Timing {
    state: Mutex<State>,
}

impl Observer for Timing {
    fn notify(&self, notice: &Notice) {
        let now = Instant::now();
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
                    s.parallel += now - start;
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
                    *s.per_system.entry(system).or_default() += now - start;
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
            wall: last - first,
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

/// `count / wall` per second, with one decimal.
fn per_second(count: u128, wall: Duration) -> String {
    let tenths = (count * NANOS_PER_SECOND * 10)
        .checked_div(wall.as_nanos())
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
            per_second(self.sim_minutes / MINUTES_PER_YEAR, self.wall),
            per_second(u128::from(self.instants), self.wall),
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
        assert_eq!(per_second(5, Duration::from_secs(2)), "2.5");
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
}
