//! The pace governor and run log (`specs/stream/control_v1.md`; SPR-027 T-03).
//!
//! Hosts the engine's pace loop (tool plane):
//! - speed bands (`live`, `1h`, `1d`, `1w`, `1mo`, `1y`, `10y`, `max`);
//! - the governor's cell, monotonic deadline arithmetic in integer nanoseconds, and the one
//!   atomic decision point before each instant;
//! - the request queue, reply slots, and snapshot publication through `arc-swap`;
//! - the run log `<run-dir>/run.log`.
//!
//! Plane A is untouched (docs/13 §3).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use arc_swap::ArcSwap;
use rh_protocol::control::{
    CONTROL_FORMAT, CONTROL_VERSION, ControlCommand, ControlConn, ControlHost, History, Reason,
    StepUntil, error, response,
};
use serde_json::{Map, Value, json};

use crate::run::Error;

/// Number of nanoseconds per second.
pub const NS_PER_S: i128 = 1_000_000_000;

/// Default controller connection cap (`control_v1.md` §1).
pub const DEFAULT_CONTROLLER_CAP: usize = 8;

/// Sim-minutes per hour.
pub const MINUTES_PER_HOUR: i64 = 60;
/// Sim-minutes per day.
pub const MINUTES_PER_DAY: i64 = 1_440;
/// Sim-minutes per week.
pub const MINUTES_PER_WEEK: i64 = 10_080;
/// Months per year.
pub const MONTHS_PER_YEAR: i64 = 12;
/// Decades multiplier.
pub const DECADE: i64 = 10;

/// Ceiling of `a / b` for non-negative `a` and positive `b`.
#[must_use]
pub fn ceil_div(a: i128, b: i128) -> i128 {
    if a <= 0 {
        0
    } else {
        (a - 1) / b + 1
    }
}

/// The band's rate in sim-minutes per wall-second (`control_v1.md` §3.1); `None` for `max`.
#[must_use]
pub fn rate(band: &str, minutes_per_year: i64) -> Option<i64> {
    match band {
        "live" => Some(1),
        "1h" => Some(MINUTES_PER_HOUR),
        "1d" => Some(MINUTES_PER_DAY),
        "1w" => Some(MINUTES_PER_WEEK),
        "1mo" => Some(minutes_per_year / MONTHS_PER_YEAR),
        "1y" => Some(minutes_per_year),
        "10y" => Some(DECADE * minutes_per_year),
        _ => None,
    }
}

/// The wall clock at or after which the instant at `tick` may run (`control_v1.md` §3.3).
#[must_use]
pub fn deadline_ns(anchor_wall_ns: i128, anchor_tick: i64, tick: i64, r: Option<i64>) -> i128 {
    match r {
        None => anchor_wall_ns,
        Some(rate) => {
            let num = i128::from(tick - anchor_tick) * NS_PER_S;
            let den = i128::from(rate);
            anchor_wall_ns + ceil_div(num, den)
        }
    }
}

/// Monotonic and realtime clocks for the governor.
pub trait GovernorClock: Send + Sync {
    /// Monotonic clock in nanoseconds.
    fn now_monotonic_ns(&self) -> i128;
    /// Realtime clock in integer nanoseconds since Unix epoch (for run log only).
    fn now_wall_ns(&self) -> u64;
    /// Sleep until monotonic clock reaches `deadline_ns`.
    fn sleep_until(&self, deadline_ns: i128);
}

/// Real clock backed by `Instant` and `SystemTime`.
pub struct RealClock {
    #[allow(
        clippy::disallowed_types,
        reason = "tool plane: RealClock monotonic anchor"
    )]
    start: std::time::Instant,
}

impl Default for RealClock {
    fn default() -> Self {
        Self::new()
    }
}

impl RealClock {
    /// Creates a fresh `RealClock`.
    #[must_use]
    #[allow(
        clippy::disallowed_types,
        clippy::disallowed_methods,
        reason = "tool plane: RealClock monotonic anchor"
    )]
    pub fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }
}

impl GovernorClock for RealClock {
    #[allow(
        clippy::disallowed_types,
        clippy::disallowed_methods,
        reason = "tool plane: RealClock monotonic elapsed"
    )]
    fn now_monotonic_ns(&self) -> i128 {
        i128::try_from(self.start.elapsed().as_nanos()).unwrap_or(i128::MAX)
    }

    #[allow(
        clippy::disallowed_types,
        clippy::disallowed_methods,
        reason = "tool plane: wall clock for run log only"
    )]
    fn now_wall_ns(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
    }

    fn sleep_until(&self, deadline_ns: i128) {
        let now = self.now_monotonic_ns();
        if deadline_ns > now {
            let dur_ns = u64::try_from(deadline_ns - now).unwrap_or(u64::MAX);
            std::thread::sleep(Duration::from_nanos(dur_ns));
        }
    }
}

/// Simulated clock for testing (`control_v1.md` §9).
pub struct FakeClock {
    /// Monotonic time in nanoseconds.
    pub now_ns: AtomicI64,
}

impl FakeClock {
    /// Creates a `FakeClock` initialized to `start_ns`.
    #[must_use]
    pub const fn new(start_ns: i64) -> Self {
        Self {
            now_ns: AtomicI64::new(start_ns),
        }
    }

    /// Sets the current time directly.
    pub fn set(&self, ns: i64) {
        self.now_ns.store(ns, Ordering::SeqCst);
    }
}

impl GovernorClock for FakeClock {
    fn now_monotonic_ns(&self) -> i128 {
        i128::from(self.now_ns.load(Ordering::SeqCst))
    }

    fn now_wall_ns(&self) -> u64 {
        u64::try_from(self.now_ns.load(Ordering::SeqCst).max(0)).unwrap_or(u64::MAX)
    }

    fn sleep_until(&self, deadline_ns: i128) {
        let cur = self.now_ns.load(Ordering::SeqCst);
        let target = i64::try_from(deadline_ns).unwrap_or(i64::MAX);
        if target > cur {
            self.now_ns.store(target, Ordering::SeqCst);
        }
    }
}

/// The governor's cell as the engine applied it (`control_v1.md` §3.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    /// The band in force (`live`, `1h`, ..., `max`).
    pub band: String,
    /// The run state (`running`, `paused`, `finished`).
    pub state: String,
    /// The pending step target, if any.
    pub step_target: Option<i64>,
    /// The tick of the anchor.
    pub anchor_tick: i64,
    /// The monotonic clock nanoseconds when the anchor was set.
    pub anchor_wall_ns: i128,
}

impl Cell {
    /// The state and the rate the governor sleeps by: a pending step runs at `max`.
    #[must_use]
    pub fn effective(&self, minutes_per_year: i64) -> (&str, Option<i64>) {
        let rate = if self.step_target.is_some() {
            None
        } else {
            rate(&self.band, minutes_per_year)
        };
        (self.state.as_str(), rate)
    }
}

/// What the engine publishes at its decision point; what `control.status` reads (`control_v1.md` §3.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// Band token.
    pub band: String,
    /// State token.
    pub state: String,
    /// Step target, if any.
    pub step_target: Option<i64>,
    /// Tip tick (null before first instant).
    pub tip: Option<i64>,
    /// Next seq of channel A.
    pub next_seq: u64,
    /// Next tick on agenda.
    pub next_tick: Option<i64>,
    /// End reason if finished.
    pub end: Option<String>,
}

impl Snapshot {
    /// The pace object (`control_v1.md` §4).
    #[must_use]
    pub fn pace(&self) -> Value {
        json!({
            "band": self.band,
            "next_seq": self.next_seq,
            "state": self.state,
            "step": self.step_target.map(|t| json!({"until_tick": t})),
            "tick": self.tip,
        })
    }
}

struct PendingRequest {
    conn_number: usize,
    conn_pid: Option<u32>,
    rid: Value,
    cmd: ControlCommand,
    reply: Arc<Mutex<Option<Value>>>,
    cvar: Arc<Condvar>,
    applied_at_ns: Option<i128>,
    answered_seq: Option<usize>,
}

struct Inner {
    cell: Cell,
    pending: Vec<PendingRequest>,
    /// Connection map: `conn_number` -> `Option<pid>`.
    connections: std::collections::BTreeMap<usize, Option<u32>>,
    connections_opened: usize,
    answers_given: usize,
    end: Option<String>,
    run_log: Vec<String>,
    log_path: Option<PathBuf>,
}

impl Inner {
    fn cell_pace(&self, tip: Option<i64>, next_seq: u64) -> Value {
        json!({
            "band": self.cell.band,
            "next_seq": next_seq,
            "state": self.cell.state,
            "step": self.cell.step_target.map(|t| json!({"until_tick": t})),
            "tick": tip,
        })
    }
}

/// The pace governor (`control_v1.md` §3.3).
pub struct Governor {
    clock: Arc<dyn GovernorClock>,
    minutes_per_year: i64,
    first_tick: i64,
    run_nonce: u64,
    timeline_id: u32,
    history: History,
    cap: usize,
    control_plant: Option<String>,
    published: ArcSwap<Snapshot>,
    inner: Mutex<Inner>,
    cvar: Condvar,
}

impl Governor {
    /// Creates a fresh `Governor` starting at `band` and `state`.
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "governor start configuration takes its inputs"
    )]
    pub fn new(
        clock: Arc<dyn GovernorClock>,
        minutes_per_year: i64,
        first_tick: i64,
        run_nonce: u64,
        timeline_id: u32,
        history: History,
        start_band: Option<&str>,
        start_paused: bool,
        is_resume: bool,
        resume_tick: Option<i64>,
        log_path: Option<PathBuf>,
        control_plant: Option<String>,
    ) -> Self {
        let band = start_band.unwrap_or("max").to_owned();
        let state = if start_paused {
            "paused".to_owned()
        } else {
            "running".to_owned()
        };
        let now_ns = clock.now_monotonic_ns();
        let initial_anchor_tick = if is_resume {
            resume_tick.unwrap_or(first_tick)
        } else {
            first_tick
        };
        let cell = Cell {
            band: band.clone(),
            state: state.clone(),
            step_target: None,
            anchor_tick: initial_anchor_tick,
            anchor_wall_ns: now_ns,
        };
        let initial_tip = if is_resume { resume_tick } else { None };
        let snapshot = Snapshot {
            band,
            state,
            step_target: None,
            tip: initial_tip,
            next_seq: 0,
            next_tick: Some(first_tick),
            end: None,
        };
        let mut inner = Inner {
            cell,
            pending: Vec::new(),
            connections: std::collections::BTreeMap::new(),
            connections_opened: 0,
            answers_given: 0,
            end: None,
            run_log: Vec::new(),
            log_path,
        };

        // Write initial host and pace log lines (control_v1.md §8.1)
        let wall_ns = clock.now_wall_ns();
        let tip_tok = initial_tip.map_or("~".to_owned(), |t| t.to_string());
        if is_resume {
            let l1 = format!("host {wall_ns} resume {tip_tok} {run_nonce}");
            append_log(inner.log_path.as_deref(), &mut inner.run_log, l1);
            let l2 = format!(
                "pace {wall_ns} {} {} {tip_tok} host.resume {run_nonce} conn=- pid=-",
                inner.cell.band, inner.cell.state
            );
            append_log(inner.log_path.as_deref(), &mut inner.run_log, l2);
        } else {
            let l1 = format!("host {wall_ns} start {tip_tok} {run_nonce}");
            append_log(inner.log_path.as_deref(), &mut inner.run_log, l1);
            let l2 = format!(
                "pace {wall_ns} {} {} {tip_tok} host.start {run_nonce} conn=- pid=-",
                inner.cell.band, inner.cell.state
            );
            append_log(inner.log_path.as_deref(), &mut inner.run_log, l2);
        }

        Self {
            clock,
            minutes_per_year,
            first_tick,
            run_nonce,
            timeline_id,
            history,
            cap: DEFAULT_CONTROLLER_CAP,
            control_plant,
            published: ArcSwap::from_pointee(snapshot),
            inner: Mutex::new(inner),
            cvar: Condvar::new(),
        }
    }

    /// The published snapshot read without lock.
    #[must_use]
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.published.load_full()
    }

    /// Returns the connection cap.
    #[must_use]
    pub const fn cap(&self) -> usize {
        self.cap
    }

    /// Sets connection cap.
    pub fn set_cap(&mut self, cap: usize) {
        self.cap = cap;
    }

    /// Current connection count.
    #[must_use]
    pub fn connection_count(&self) -> usize {
        self.inner
            .lock()
            .map_or(0, |inner| inner.connections.len())
    }

    /// Opens a controller connection. Returns `None` if cap exceeded.
    pub fn connect(&self, pid: Option<u32>) -> Option<ControlConn> {
        let mut inner = self.inner.lock().ok()?;
        inner.connections_opened += 1;
        let num = inner.connections_opened;
        let closed = inner.connections.len() >= self.cap;
        if !closed {
            inner.connections.insert(num, pid);
        }
        Some(ControlConn {
            number: num,
            closed,
        })
    }

    /// Closes a controller connection.
    pub fn close(&self, conn: &ControlConn) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.connections.remove(&conn.number);
        }
    }

    fn anchor(&self, inner: &mut Inner, tip: Option<i64>) {
        inner.cell.anchor_tick = tip.unwrap_or(self.first_tick);
        inner.cell.anchor_wall_ns = self.clock.now_monotonic_ns();
    }

    fn log_pace(
        &self,
        inner: &mut Inner,
        method: &str,
        conn_number: Option<usize>,
        conn_pid: Option<u32>,
        tip: Option<i64>,
    ) {
        let wall_ns = self.clock.now_wall_ns();
        let who = match conn_number {
            Some(n) => {
                let p = conn_pid.map_or("-".to_owned(), |pid| pid.to_string());
                format!("conn={n} pid={p}")
            }
            None => "conn=- pid=-".to_owned(),
        };
        let tip_tok = tip.map_or("~".to_owned(), |t| t.to_string());
        let line = format!(
            "pace {wall_ns} {} {} {tip_tok} {method} {} {who}",
            inner.cell.band, inner.cell.state, self.run_nonce
        );
        append_log(inner.log_path.as_deref(), &mut inner.run_log, line);
    }

    fn log_host(&self, inner: &mut Inner, event: &str, tip: Option<i64>) {
        let wall_ns = self.clock.now_wall_ns();
        let tip_tok = tip.map_or("~".to_owned(), |t| t.to_string());
        let line = format!("host {wall_ns} {event} {tip_tok} {}", self.run_nonce);
        append_log(inner.log_path.as_deref(), &mut inner.run_log, line);
    }

    fn changes_published(snap: &Snapshot, cmd: &ControlCommand) -> bool {
        match cmd {
            ControlCommand::Status => false,
            ControlCommand::Pace { band } => band != &snap.band,
            ControlCommand::Pause => snap.state != "paused",
            ControlCommand::Resume => snap.state != "running" || snap.step_target.is_some(),
            ControlCommand::Step { until } => {
                if snap.state != "running" || matches!(until, StepUntil::Instant) {
                    return true;
                }
                match until {
                    StepUntil::Tick(t) => snap.step_target != Some(*t),
                    StepUntil::Instant => true,
                }
            }
        }
    }

    fn status_from_snapshot(&self, snap: &Snapshot, num_conns: usize) -> Value {
        json!({
            "calendar": {"minutes_per_year": self.minutes_per_year},
            "control": {"format": CONTROL_FORMAT, "version": CONTROL_VERSION},
            "controllers": num_conns,
            "end": snap.end.as_ref().map(|r| json!({"reason": r})),
            "history": {
                "engine_build": self.history.engine_build,
                "manifest_hash": self.history.manifest_hash,
                "registry_version": self.history.registry_version,
                "world_seed": self.history.world_seed,
            },
            "pace": snap.pace(),
            "run_nonce": self.run_nonce,
            "timeline_id": self.timeline_id,
            "tip": {"next_seq": snap.next_seq, "tick": snap.tip},
        })
    }

    /// Applies queued commands, re-checks step reached, and publishes snapshot (`control_v1.md` §3.3 item 1).
    pub fn before_instant(&self, next_tick: Option<i64>, tip: Option<i64>, next_seq: u64) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };

        let mut deferred_status = Vec::new();
        let mut held_conns = BTreeSet::new();
        let mut to_apply = Vec::new();
        let mut kept = Vec::new();

        for p in inner.pending.drain(..) {
            if held_conns.contains(&p.conn_number) {
                kept.push(p);
                continue;
            }
            if matches!(p.cmd, ControlCommand::Status) {
                let conn_num = p.conn_number;
                deferred_status.push(p);
                held_conns.insert(conn_num);
                continue;
            }
            to_apply.push(p);
        }
        inner.pending = kept;

        for mut p in to_apply {
            let (before_state, before_rate) = {
                let (st, r) = inner.cell.effective(self.minutes_per_year);
                (st.to_owned(), r)
            };
            let reply = match self.apply_command(&mut inner, &p, next_tick, tip, next_seq) {
                Ok(result) => response(&Some(p.rid.clone()), result),
                Err(refusal) => refusal,
            };
            let (after_state, after_rate) = {
                let (st, r) = inner.cell.effective(self.minutes_per_year);
                (st.to_owned(), r)
            };
            let owed = self.control_plant.as_deref() == Some("paused-time-owed")
                && matches!(p.cmd, ControlCommand::Resume);
            if (before_state != after_state || before_rate != after_rate) && !owed {
                self.anchor(&mut inner, tip);
            }
            inner.answers_given += 1;
            p.applied_at_ns = Some(self.clock.now_monotonic_ns());
            p.answered_seq = Some(inner.answers_given);
            if let Ok(mut slot) = p.reply.lock() {
                *slot = Some(reply);
            }
            p.cvar.notify_one();
        }

        // Re-check pending step
        let step_reached = if let (Some(target), "running") =
            (inner.cell.step_target, inner.cell.state.as_str())
        {
            if self.control_plant.as_deref() == Some("step-overshoot") {
                tip.is_some_and(|t| t > target)
            } else {
                next_tick.is_none_or(|n| n > target)
            }
        } else {
            false
        };

        if step_reached {
            "paused".clone_into(&mut inner.cell.state);
            inner.cell.step_target = None;
            self.anchor(&mut inner, tip);
            self.log_pace(&mut inner, "step.reached", None, None, tip);
        }

        // Publish snapshot
        let snap = Snapshot {
            band: inner.cell.band.clone(),
            state: inner.cell.state.clone(),
            step_target: inner.cell.step_target,
            tip,
            next_seq,
            next_tick,
            end: inner.end.clone(),
        };
        self.published.store(Arc::new(snap));

        // Answer deferred status requests
        let snap_arc = self.published.load();
        let num_conns = inner.connections.len();
        for mut p in deferred_status {
            inner.answers_given += 1;
            p.applied_at_ns = Some(self.clock.now_monotonic_ns());
            p.answered_seq = Some(inner.answers_given);
            let rep =
                response(&Some(p.rid.clone()), self.status_from_snapshot(&snap_arc, num_conns));
            if let Ok(mut slot) = p.reply.lock() {
                *slot = Some(rep);
            }
            p.cvar.notify_one();
        }
    }

    fn apply_command(
        &self,
        inner: &mut Inner,
        p: &PendingRequest,
        next_tick: Option<i64>,
        tip: Option<i64>,
        next_seq: u64,
    ) -> Result<Value, Value> {
        match &p.cmd {
            ControlCommand::Status => unreachable!(),
            ControlCommand::Pace { band } => {
                let changed = band != &inner.cell.band;
                if changed {
                    band.clone_into(&mut inner.cell.band);
                    self.log_pace(inner, "control.pace", Some(p.conn_number), p.conn_pid, tip);
                }
                let mut pace = inner.cell_pace(tip, next_seq);
                if let Value::Object(ref mut m) = pace {
                    m.insert("changed".into(), Value::Bool(changed));
                }
                Ok(pace)
            }
            ControlCommand::Pause => {
                let changed = inner.cell.state == "running";
                if changed {
                    "paused".clone_into(&mut inner.cell.state);
                    inner.cell.step_target = None;
                    self.log_pace(inner, "control.pause", Some(p.conn_number), p.conn_pid, tip);
                }
                let mut pace = inner.cell_pace(tip, next_seq);
                if let Value::Object(ref mut m) = pace {
                    m.insert("changed".into(), Value::Bool(changed));
                }
                Ok(pace)
            }
            ControlCommand::Resume => {
                let changed = inner.cell.state == "paused" || inner.cell.step_target.is_some();
                if changed {
                    "running".clone_into(&mut inner.cell.state);
                    inner.cell.step_target = None;
                    self.log_pace(inner, "control.resume", Some(p.conn_number), p.conn_pid, tip);
                }
                let mut pace = inner.cell_pace(tip, next_seq);
                if let Value::Object(ref mut m) = pace {
                    m.insert("changed".into(), Value::Bool(changed));
                }
                Ok(pace)
            }
            ControlCommand::Step { until } => {
                let Some(nxt) = next_tick else {
                    let mut d = Map::new();
                    d.insert("end".into(), json!(inner.end.as_deref().unwrap_or("finished")));
                    return Err(error(&Some(p.rid.clone()), Reason::RunFinished, d));
                };
                let target = match until {
                    StepUntil::Instant => nxt,
                    StepUntil::Tick(t) => *t,
                };
                if (tip.is_none() && target < self.first_tick)
                    || tip.is_some_and(|t| target <= t)
                {
                    let mut d = Map::new();
                    d.insert("tip_tick".into(), json!(tip));
                    return Err(error(&Some(p.rid.clone()), Reason::PastTick, d));
                }
                let changed =
                    inner.cell.step_target != Some(target) || inner.cell.state != "running";
                if changed {
                    "running".clone_into(&mut inner.cell.state);
                    inner.cell.step_target = Some(target);
                    self.log_pace(
                        inner,
                        &format!("control.step:{target}"),
                        Some(p.conn_number),
                        p.conn_pid,
                        tip,
                    );
                    if nxt > target {
                        // Already passed: reached inside its own application (L2)
                        "paused".clone_into(&mut inner.cell.state);
                        inner.cell.step_target = None;
                        self.anchor(inner, tip);
                        self.log_pace(inner, "step.reached", None, None, tip);
                    }
                }
                let mut pace = inner.cell_pace(tip, next_seq);
                if let Value::Object(ref mut m) = pace {
                    m.insert("changed".into(), Value::Bool(changed));
                }
                Ok(pace)
            }
        }
    }

    /// The monotonic deadline in nanoseconds for the next instant, or `None` if paused/no deadline.
    #[must_use]
    pub fn deadline(&self, next_tick: Option<i64>) -> Option<i128> {
        let inner = self.inner.lock().ok()?;
        let (state, r) = inner.cell.effective(self.minutes_per_year);
        let tick = next_tick?;
        if state != "running" {
            return None;
        }
        Some(deadline_ns(inner.cell.anchor_wall_ns, inner.cell.anchor_tick, tick, r))
    }

    /// Waits for turn in the live run loop (`control_v1.md` §3.3 items 1-3).
    ///
    /// # Errors
    /// Returns error if interrupted or poisoned.
    pub fn wait_for_turn(
        &self,
        next_tick: i64,
        tip: Option<i64>,
        next_seq: u64,
    ) -> Result<(), Error> {
        loop {
            self.before_instant(Some(next_tick), tip, next_seq);
            let mut guard = self.inner.lock().map_err(|e| Error(e.to_string()))?;
            if !guard.pending.is_empty() {
                // M2: pending requests held behind status: next decision point at once!
                drop(guard);
                continue;
            }
            let snap = self.published.load();
            if snap.state == "finished" {
                return Err(Error("the run finished".to_owned()));
            }
            if snap.state == "paused" {
                guard = self.cvar.wait(guard).map_err(|e| Error(e.to_string()))?;
                drop(guard);
                continue;
            }
            // State is running.
            let (_, r) = guard.cell.effective(self.minutes_per_year);
            let dl = deadline_ns(guard.cell.anchor_wall_ns, guard.cell.anchor_tick, next_tick, r);
            let now = self.clock.now_monotonic_ns();
            if now >= dl {
                // Reached deadline: run instant at once (item 4, L4)
                return Ok(());
            }
            let diff = dl - now;
            let wait_ns = u64::try_from(diff).unwrap_or(u64::MAX);
            let wait_dur = Duration::from_nanos(wait_ns);
            let (new_guard, timeout_res) = self
                .cvar
                .wait_timeout(guard, wait_dur)
                .map_err(|e| Error(e.to_string()))?;
            if timeout_res.timed_out() {
                return Ok(());
            }
            drop(new_guard);
        }
    }

    /// End of an instant: updates tip and `next_seq` in snapshot.
    pub fn after_instant(&self, finished: bool, tip: Option<i64>, next_seq: u64) {
        if finished {
            self.finish("finished", tip, next_seq);
        } else {
            let snap = self.published.load();
            let new_snap = Snapshot {
                band: snap.band.clone(),
                state: snap.state.clone(),
                step_target: snap.step_target,
                tip,
                next_seq,
                next_tick: snap.next_tick,
                end: snap.end.clone(),
            };
            self.published.store(Arc::new(new_snap));
        }
    }

    /// Finishes the run in one critical section (`control_v1.md` §3.3 item 5, §8.2).
    pub fn finish(&self, reason: &str, tip: Option<i64>, next_seq: u64) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        if inner.cell.state == "finished" {
            return;
        }
        "finished".clone_into(&mut inner.cell.state);
        inner.cell.step_target = None;
        inner.end = Some(reason.to_owned());
        let event = if reason == "finished" {
            "finish"
        } else {
            "abort"
        };
        self.log_host(&mut inner, event, tip);
        let snap = Snapshot {
            band: inner.cell.band.clone(),
            state: "finished".to_owned(),
            step_target: None,
            tip,
            next_seq,
            next_tick: None,
            end: Some(reason.to_owned()),
        };
        self.published.store(Arc::new(snap));
        let num_conns = inner.connections.len();
        let snap_arc = self.published.load();
        let drained: Vec<_> = inner.pending.drain(..).collect();
        for mut p in drained {
            inner.answers_given += 1;
            p.applied_at_ns = Some(self.clock.now_monotonic_ns());
            p.answered_seq = Some(inner.answers_given);
            let rep = if matches!(p.cmd, ControlCommand::Status) {
                response(&Some(p.rid.clone()), self.status_from_snapshot(&snap_arc, num_conns))
            } else {
                let mut d = Map::new();
                d.insert("end".into(), json!(reason));
                error(&Some(p.rid.clone()), Reason::RunFinished, d)
            };
            if let Ok(mut slot) = p.reply.lock() {
                *slot = Some(rep);
            }
            p.cvar.notify_one();
        }
        self.cvar.notify_all();
    }

    /// Read the run log lines written so far.
    #[must_use]
    pub fn run_log(&self) -> Vec<String> {
        self.inner
            .lock()
            .map_or(Vec::new(), |inner| inner.run_log.clone())
    }

    /// Number of commands currently pending in the queue.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.inner.lock().map_or(0, |inner| inner.pending.len())
    }
}

impl ControlHost for Governor {
    fn submit(&self, conn: &ControlConn, id: &Value, cmd: ControlCommand) -> Value {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_e) => return error(&Some(id.clone()), Reason::InvalidRequest, Map::new()),
        };

        let inflight = inner.pending.iter().any(|p| p.conn_number == conn.number);

        // 1. Status with nothing inflight
        if matches!(cmd, ControlCommand::Status) && !inflight {
            let snap = self.published.load();
            return response(
                &Some(id.clone()),
                self.status_from_snapshot(&snap, inner.connections.len()),
            );
        }

        // 2. Finished run
        if inner.cell.state == "finished" {
            if matches!(cmd, ControlCommand::Status) {
                let snap = self.published.load();
                return response(
                    &Some(id.clone()),
                    self.status_from_snapshot(&snap, inner.connections.len()),
                );
            }
            let mut d = Map::new();
            d.insert("end".into(), json!(inner.end.as_deref().unwrap_or("finished")));
            return error(&Some(id.clone()), Reason::RunFinished, d);
        }

        // 3. No-change command when queue is empty and nothing inflight
        if inner.pending.is_empty() && !inflight {
            let snap = self.published.load();
            if !Self::changes_published(&snap, &cmd) {
                let mut pace_obj = snap.pace();
                if let Value::Object(ref mut m) = pace_obj {
                    m.insert("changed".into(), Value::Bool(false));
                }
                return response(&Some(id.clone()), pace_obj);
            }
        }

        // 4. Queue and wait
        let reply_slot = Arc::new(Mutex::new(None));
        let cvar = Arc::new(Condvar::new());
        let conn_pid = inner.connections.get(&conn.number).copied().flatten();
        inner.pending.push(PendingRequest {
            conn_number: conn.number,
            conn_pid,
            rid: id.clone(),
            cmd,
            reply: Arc::clone(&reply_slot),
            cvar: Arc::clone(&cvar),
            applied_at_ns: None,
            answered_seq: None,
        });
        self.cvar.notify_one();
        drop(inner);

        let Ok(mut rep) = reply_slot.lock() else {
            return error(&Some(id.clone()), Reason::InvalidRequest, Map::new());
        };
        while rep.is_none() {
            let Ok(new_rep) = cvar.wait(rep) else {
                return error(&Some(id.clone()), Reason::InvalidRequest, Map::new());
            };
            rep = new_rep;
        }
        rep.take()
            .unwrap_or_else(|| error(&Some(id.clone()), Reason::InvalidRequest, Map::new()))
    }
}

fn append_log(path: Option<&Path>, lines_buf: &mut Vec<String>, line: String) {
    if let Some(p) = path {
        use std::io::Write as _;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            let _ = writeln!(f, "{line}");
        }
    }
    lines_buf.push(line);
}
