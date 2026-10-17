//! V-PERF-11: heap allocations per steady-state tick of the smoke world (SPR-009 T-05, ruling 5).
//!
//! This target installs the counting allocator ([`rh_bench::alloc_count::Counting`]) as its
//! `#[global_allocator]`: the only one in the workspace besides the arena of the instruction-count
//! targets (`bin/lint_arch.py`, global-allocator), in the only target that enables `count-alloc`.
//!
//! 1. **Self-tests.** The counter is checked on two patterns whose counts are known exactly, so a
//!    broken counter can never report a passing budget.
//! 2. **The window.** The smoke world ([`rh_bench::smoke`]) runs [`WARM_UP_YEARS`] to reach a steady
//!    state, then [`MEASURED_YEARS`] more one instant (tick) at a time, at 1 and 4 threads. Per
//!    thread count it reports the ticks, the allocations (`alloc`, `alloc_zeroed`), the growths
//!    (reallocations to a larger size, counted separately: V-PERF-11), the shrinks and the most
//!    allocations in one tick.
//! 3. **The sites.** At 1 thread an observer attributes every allocation of the window to the
//!    stretch of the instant it happened in: a system's step (`step <system>`), the level's dispatch
//!    and the threads it spawns, the merge, or the engine between levels, phases and instants. The
//!    observer's own allocations are excluded. Driving these counts to 0 is BCK-002 A-32.
//!
//! 4. **The rungs** (SPR-013 T-13): `solo`, `household` and `hamlet`, each at 1 thread over the same
//!    window on its own calendar, as `rh-cli run` builds them, with no inputs: one budget line per
//!    rung (`rung <world> …`).
//!
//! The P0 engine allocates per tick (per-level buffers, scoped threads), so the budget is the
//! measured count, and any increase fails (ruling 5). `bin/perf_gate.py alloc-gate` runs this
//! target and compares its output with `baselines/allocations_v1.txt`.
//!
//! Run: `cargo bench -p rh-bench --features count-alloc --bench alloc_budget -- --out FILE`.
//! `RH_ALLOC_CONTROL=1` wraps `X1.smoke_weather` so it also allocates in its step: the positive
//! control, which must exceed the budget.
//!
//! REF: `docs/09` V-PERF-11; `CODEX/05_PROJECT/SPR-009_Benchmarks.md` T-05, ruling 5; BCK-002 A-32.
#![allow(missing_docs, reason = "a bench binary")]

use std::collections::BTreeMap;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use rh_bench::alloc_count::{Counting, Snapshot, snapshot};
use rh_bench::{rungs, smoke};
use rh_core::engine::Engine;
use rh_core::observe::{Boundary, Notice, Observer};
use rh_core::time::SimTime;

#[global_allocator]
static COUNTING: Counting = Counting;

/// Years run before the window, to reach a steady state.
const WARM_UP_YEARS: i64 = 2;
/// Years measured, one instant at a time.
const MEASURED_YEARS: i64 = 1;
/// The thread counts measured (the scoped threads of a parallel level allocate: BCK-002 A-32).
const THREADS: [usize; 2] = [1, 4];
const MINUTES_PER_YEAR: i64 = 365 * 24 * 60;
const FORMAT: &str = "rehoboam/rh-bench/allocations/v1";

/// One buffer: allocated, grown twice past its capacity with `reserve_exact`, shrunk once, freed.
/// Explicit calls, so the counts do not depend on `Vec`'s growth policy.
const KNOWN: Snapshot = Snapshot {
    allocations: 1,
    growths: 2,
    shrinks: 1,
    deallocations: 1,
};

fn self_test() -> Snapshot {
    let before = snapshot();
    let mut v: Vec<u64> = Vec::with_capacity(4);
    v.extend_from_slice(black_box(&[1; 4]));
    v.reserve_exact(4);
    v.extend_from_slice(black_box(&[2; 4]));
    v.reserve_exact(8);
    v.truncate(2);
    v.shrink_to_fit();
    drop(black_box(v));
    snapshot().since(before)
}

/// A zero-filled buffer (`vec![0; n]`), which `Vec` takes from `alloc_zeroed`, then freed: an
/// uncounted `alloc_zeroed` would under-count engine code that zero-fills (VER-SPR-009-TEST).
const KNOWN_ZEROED: Snapshot = Snapshot {
    allocations: 1,
    growths: 0,
    shrinks: 0,
    deallocations: 1,
};

fn zeroed_test() -> Snapshot {
    let before = snapshot();
    let z: Vec<u64> = vec![0; black_box(64)];
    drop(black_box(z));
    snapshot().since(before)
}

/// A stretch of an instant: (phase, what, system).
type Site = (&'static str, &'static str, &'static str);

/// Attributes allocations to the stretch between two notices (1 thread only: on several, workers'
/// allocations interleave). Its own allocations (a new map entry) are excluded: the count it
/// attributes runs from the end of one notice to the start of the next.
#[derive(Default)]
struct Sites {
    inner: Mutex<SitesState>,
}

#[derive(Default)]
struct SitesState {
    last: Snapshot,
    current: Site,
    counts: BTreeMap<Site, (u64, u64)>,
}

fn stretch(notice: &Notice) -> Site {
    let phase = notice.phase.map_or("-", rh_core::Phase::name);
    match notice.boundary {
        Boundary::InstantStart => ("-", "instant setup and inputs", ""),
        Boundary::PhaseStart => (phase, "phase start (timers due)", ""),
        Boundary::LevelStart { .. } | Boundary::JobEnd(_) => (phase, "level dispatch", ""),
        Boundary::JobStart(job) => (phase, "step", job.system),
        Boundary::LevelMerge => (phase, "merge", ""),
        Boundary::LevelEnd => (phase, "between levels", ""),
        Boundary::PhaseEnd => (phase, "after the phase", ""),
        Boundary::InstantEnd => ("-", "between instants", ""),
    }
}

impl Observer for Sites {
    fn notify(&self, notice: &Notice) {
        let now = snapshot();
        let mut s = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let d = now.since(s.last);
        let site = s.current;
        let entry = s.counts.entry(site).or_default();
        entry.0 += d.allocations;
        entry.1 += d.growths;
        s.current = stretch(notice);
        s.last = snapshot();
    }
}

impl Sites {
    /// Starts the window: every count to zero, the stretch counted from now.
    fn reset(&self) {
        let mut s = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for v in s.counts.values_mut() {
            *v = (0, 0);
        }
        s.last = snapshot();
    }

    fn report(&self) -> Vec<String> {
        let s = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        s.counts
            .iter()
            .filter(|(_, (a, g))| *a + *g > 0)
            .map(|((phase, what, system), (a, g))| {
                let label = if system.is_empty() {
                    (*what).to_owned()
                } else {
                    format!("{what} {system}")
                };
                format!("site {phase} | {label} | allocations {a} growths {g}")
            })
            .collect()
    }
}

/// One thread count's window.
struct Window {
    ticks: u64,
    total: Snapshot,
    max_per_tick: u64,
}

fn advance(engine: &mut Engine, t: SimTime) -> Result<(), String> {
    engine
        .advance_until(t, |_, _| {})
        .map_err(|e| e.to_string())
}

/// Runs the warm-up, then the window one instant at a time, on a calendar of `minutes_per_year`.
fn measure(
    engine: &mut Engine,
    sites: Option<&Sites>,
    minutes_per_year: i64,
) -> Result<Window, String> {
    advance(
        engine,
        SimTime::from_minutes(WARM_UP_YEARS * minutes_per_year - 1),
    )?;
    if let Some(s) = sites {
        s.reset();
    }
    let end = SimTime::from_minutes((WARM_UP_YEARS + MEASURED_YEARS) * minutes_per_year - 1);
    let (mut ticks, mut max_per_tick) = (0, 0);
    let start = snapshot();
    while let Some(t) = engine.next_instant().filter(|t| *t <= end) {
        let before = snapshot();
        advance(engine, t)?;
        max_per_tick = max_per_tick.max(snapshot().since(before).allocations);
        ticks += 1;
    }
    Ok(Window {
        ticks,
        total: snapshot().since(start),
        max_per_tick,
    })
}

fn report(repo: &Path, control: bool) -> Result<String, String> {
    let mut lines = vec![
        format!(
            "# {FORMAT}: heap allocations per steady-state tick of the smoke world (SPR-009 T-05)."
        ),
        "# Written by `python -I bin/perf_gate.py alloc-record`, never by hand (SPR-009 ruling 3)."
            .to_owned(),
        format!("format {FORMAT}"),
        format!(
            "window smoke seed {} warm-up {WARM_UP_YEARS} years, measured {MEASURED_YEARS} year(s), no inputs",
            smoke::SEED
        ),
    ];
    if control {
        lines.push(format!(
            "control {} zero-fills {} bytes after each step",
            smoke::CONTROL_SYSTEM,
            smoke::CONTROL_BYTES
        ));
    }
    let mut site_lines = Vec::new();
    for threads in THREADS {
        let mut engine = smoke::engine(repo, threads, control)?;
        let sites = (threads == 1).then(|| Arc::new(Sites::default()));
        if let Some(s) = &sites {
            engine.set_observer(Some(s.clone()));
        }
        let w = measure(&mut engine, sites.as_deref(), MINUTES_PER_YEAR)?;
        lines.push(format!(
            "budget threads {threads} ticks {} allocations {} growths {} shrinks {} max_per_tick {}",
            w.ticks, w.total.allocations, w.total.growths, w.total.shrinks, w.max_per_tick
        ));
        if let Some(s) = sites {
            site_lines = s.report();
        }
    }
    // SPR-013 T-13: each rung of the ladder, at 1 thread (one system: every level holds one job),
    // over the same window on its own calendar; its budget is its measured count.
    lines.push(format!(
        "window-rungs {} warm-up {WARM_UP_YEARS} years, measured {MEASURED_YEARS} year(s), no inputs, manifest seeds",
        rungs::RUNGS.join(",")
    ));
    for world in rungs::RUNGS {
        let (mut engine, calendar, seed) = rungs::engine(repo, world, 1)?;
        let w = measure(&mut engine, None, calendar.minutes_per_year())?;
        lines.push(format!(
            "rung {world} seed {seed} threads 1 ticks {} allocations {} growths {} shrinks {} max_per_tick {}",
            w.ticks, w.total.allocations, w.total.growths, w.total.shrinks, w.max_per_tick
        ));
    }
    lines.extend(site_lines);
    Ok(lines.join("\n") + "\n")
}

#[allow(
    clippy::disallowed_methods,
    reason = "tool plane: command-line arguments and environment overrides (SPR-011 A-4)"
)]
fn main() -> ExitCode {
    let got = self_test();
    let zeroed = zeroed_test();
    if got != KNOWN || zeroed != KNOWN_ZEROED {
        println!(
            "counter self-test FAILED: expected {KNOWN:?} and {KNOWN_ZEROED:?} (zeroed), counted {got:?} and {zeroed:?}"
        );
        return ExitCode::FAILURE;
    }
    println!("counter self-test: {got:?} and {zeroed:?} (zeroed) as expected");
    let args: Vec<String> = std::env::args().collect();
    let out = args
        .windows(2)
        .find(|w| w[0] == "--out")
        .map(|w| PathBuf::from(&w[1]));
    let Some(out) = out else {
        return ExitCode::SUCCESS;
    };
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let control = std::env::var("RH_ALLOC_CONTROL").is_ok_and(|v| v == "1");
    match report(&repo, control)
        .and_then(|text| std::fs::write(&out, text).map_err(|e| format!("{}: {e}", out.display())))
    {
        Ok(()) => {
            println!("allocations written to {}", out.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("allocation budget run FAILED: {e}");
            ExitCode::FAILURE
        }
    }
}
