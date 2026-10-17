//! The engine's observer interface: boundary notifications with no clock (SPR-009 T-03, ruling 4).
//!
//! Timing never happens inside plane A: `Instant` and `SystemTime` are banned here (GOV-003 §2).
//! Instead the engine tells an attached [`Observer`] where it is: the start and end of every
//! instant, phase, level and job, each [`Notice`] carrying the sim time, the phase and, for a job,
//! the system and its binding. The host (`rh-cli`, tool plane) reads its own clock when a notice
//! arrives and computes per-system times, the serial fraction and the vitals line (GOV-006 §1).
//!
//! **The observer cannot touch plane-A state.** A notice holds copies (sim time, phase, pulse,
//! counts) and `&'static str` system ids, never a reference into the tables, the agenda, the queues
//! or the log, and the observer is called through `&self`. So nothing it does can change a result:
//! a run with an observer attached hashes equal to one without (V-DET-07 style;
//! `rh-snapshot/tests/observer.rs`). Detaching it, or attaching another, changes nothing either.
//!
//! **Threads.** Job notices come from the thread that runs the job, so an observer is `Send +
//! Sync`; [`Notice::worker`] names the worker (job `k` of a level runs on worker `k % threads`).
//! Every other notice comes from the thread that called [`crate::engine::Engine::advance_until`].
//!
//! **A panicking observer** is a fatal engine error, reported, never unwound to the host (SPR-009
//! DEF-1). Every notice is delivered under the same `catch_unwind` discipline as system code: a
//! panic becomes `SimError::Observer`, naming the notice, which poisons the engine and leaves a crash
//! artifact whose system is `observer` (`crate::engine::OBSERVER`). A job notice's panic is that job's
//! error, so its columns go back to the tables with the others. `LevelMerge` is sent once every
//! column is back. A poisoned engine runs nothing more and refuses a keyframe, as for any fatal error.
//!
//! **Order.** For each instant: `InstantStart`; per phase `PhaseStart`, then per level of each pass
//! `LevelStart`, the level's `JobStart`/`JobEnd` pairs (interleaved across workers), `LevelMerge`
//! (every job has finished; the merge begins), `LevelEnd`; then `PhaseEnd`; and `InstantEnd` once the
//! instant has succeeded. A failing instant stops where it fails, with no closing notices: the
//! engine is poisoned and runs nothing more.
//!
//! REF: `CODEX/05_PROJECT/SPR-009_Benchmarks.md` T-03, ruling 4; GOV-006 §1, §3; GOV-004 law 2
//! (observability never changes history); `docs/09` V-DET-07.

use crate::phase::Phase;
use crate::system::PulseBinding;
use crate::time::SimTime;

/// The pass of a phase a level belongs to (`crate::engine`, instant step 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    /// Systems handling their due timers.
    Timers,
    /// The firing bindings.
    Bindings,
}

/// What one job does: its due timers, or one firing binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobWork {
    /// The system handles this many due timers, in key order.
    Timers(usize),
    /// The system steps at this binding.
    Binding(PulseBinding),
}

/// One job of a level: a system and its work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobRef {
    /// The system's Registry id.
    pub system: &'static str,
    /// What it does in this job.
    pub work: JobWork,
    /// The job's position in its level (the canonical order).
    pub index: usize,
}

/// Which boundary a notice marks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    /// An instant begins (before the lagged columns are recorded).
    InstantStart,
    /// A phase begins (before its inputs are applied or its timers popped).
    PhaseStart,
    /// A level begins: `jobs` jobs of `pass`, level `level` (counted from 0 within the pass).
    LevelStart {
        /// The pass.
        pass: Pass,
        /// The level within the pass.
        level: usize,
        /// The number of jobs.
        jobs: usize,
    },
    /// A job begins, on worker [`Notice::worker`].
    JobStart(JobRef),
    /// A job ends, on worker [`Notice::worker`].
    JobEnd(JobRef),
    /// Every job of the level has ended and its columns are back in the tables; the merge begins
    /// (it runs on one thread). Not sent when a job failed.
    LevelMerge,
    /// The level's merge has ended.
    LevelEnd,
    /// A phase ends (after `Bookkeeping`'s invariants, in that phase).
    PhaseEnd,
    /// The instant has succeeded.
    InstantEnd,
}

/// One notification: where the engine is. No clock, no state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Notice {
    /// The instant.
    pub at: SimTime,
    /// The phase; `None` for `InstantStart` and `InstantEnd`.
    pub phase: Option<Phase>,
    /// The worker that sends it: 0 except for job notices on several threads.
    pub worker: usize,
    /// The boundary.
    pub boundary: Boundary,
}

/// A host's observer. It sees notices only; see the module documentation.
pub trait Observer: Send + Sync {
    /// Receives one notice. It should not panic: a panic is caught and becomes a fatal
    /// `SimError::Observer` that poisons the engine (see the module documentation).
    fn notify(&self, notice: &Notice);
}
