//! The engine's plane-A state as plain data: what a keyframe holds and `state_hash` covers
//! (SPR-007 T-02; Architect rulings 2026-10-16).
//!
//! [`EngineState`] is everything a run carries from one instant to the next besides the code:
//! - the world tables;
//! - the agenda (pending timers, tombstones, the counter);
//! - the lifecycle command queues;
//! - the per-entity epochs;
//! - the queued material inputs;
//! - the event log's held events (carried output: kept by a keyframe, outside `state_hash`), counter,
//!   last `(tick, phase)` and running hash;
//! - the cursor (the next instant to consider), the number of material inputs accepted (the
//!   journal position, VER-SPR-007-TEST GAP-10), and the
//!   timeline.
//!
//! Bookkeeping that never changes a result is left out: the thread count, the diagnostic records,
//! the derived indexes, and the tables' instance identity and column versions.
//!
//! [`crate::engine::Engine::snapshot`] produces it; [`crate::engine::Engine::restore`] rebuilds an
//! engine from it. Every part must be in canonical form, the one `snapshot` produces, and is
//! refused otherwise with a [`RestoreError`] (SPR-006 GAP-6), so that
//! `restore(x).snapshot() == x` and a keyframe's hash of `x` is the restored engine's.
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §6 item 6; ADR-0024;
//! `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md` T-02, ruling 2.

use core::fmt;

use rh_events::EventLogState;
use rh_ids::{EntityRef, TimelineId};

use crate::agenda::{AgendaError, AgendaKey, AgendaState, Timer};
use crate::command::CommandQueuesState;
use crate::engine::MaterialInput;
use crate::time::SimTime;
use crate::world::WorldTables;

/// The engine's plane-A state (see the module documentation).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineState {
    /// The next instant the engine considers; `None` once time has run out.
    pub cursor: Option<SimTime>,
    /// The timeline.
    pub timeline: TimelineId,
    /// The number of material inputs accepted, applied or still queued: the journal position
    /// (VER-SPR-007-TEST GAP-10). At least the number queued in `inputs`.
    pub accepted_inputs: u64,
    /// The world tables.
    pub tables: WorldTables,
    /// The agenda.
    pub agenda: AgendaState,
    /// The lifecycle command queues.
    pub commands: CommandQueuesState,
    /// The epoch of every entity a material input has touched, in entity order (each at least 1).
    pub epochs: Vec<(EntityRef, u32)>,
    /// The queued material inputs: instants strictly increasing, each with its inputs in
    /// submission order (never an empty list).
    pub inputs: Vec<(SimTime, Vec<MaterialInput>)>,
    /// The event log.
    pub events: EventLogState,
}

/// A state the engine refuses to restore.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestoreError {
    /// The agenda is not canonical.
    Agenda(AgendaError),
    /// The command queues are not canonical, or name an unknown owner, sender or kind.
    Commands(String),
    /// The epochs are not canonical.
    Epochs(String),
    /// The queued inputs are not canonical.
    Inputs(String),
    /// The event log is not canonical, or names an unknown emitter or kind.
    Events(String),
    /// The tables lack a column a registered system writes.
    Tables(String),
    /// A timer names an owner that is not registered.
    TimerOwner(String),
    /// A pending timer is dated before the cursor (VER-SPR-007-TEST GAP-9): it should have fired.
    TimerTime(String),
}

impl fmt::Display for RestoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Agenda(e) => write!(f, "restore: agenda: {e}"),
            Self::Commands(m) => write!(f, "restore: command queues: {m}"),
            Self::Epochs(m) => write!(f, "restore: epochs: {m}"),
            Self::Inputs(m) => write!(f, "restore: queued inputs: {m}"),
            Self::Events(m) => write!(f, "restore: event log: {m}"),
            Self::Tables(m) => write!(f, "restore: tables: {m}"),
            Self::TimerTime(m) => write!(f, "restore: agenda timers: {m}"),
            Self::TimerOwner(m) => write!(f, "restore: agenda: {m}"),
        }
    }
}

impl std::error::Error for RestoreError {}

/// Checks the canonical form of the epochs: entities strictly increasing, every epoch at least 1.
pub(crate) fn check_epochs(epochs: &[(EntityRef, u32)]) -> Result<(), RestoreError> {
    if let Some(w) = epochs.windows(2).find(|w| w[0].0 >= w[1].0) {
        return Err(RestoreError::Epochs(format!(
            "entities are not strictly increasing: {} then {}",
            w[0].0, w[1].0
        )));
    }
    if let Some((e, _)) = epochs.iter().find(|(_, n)| *n == 0) {
        return Err(RestoreError::Epochs(format!(
            "{e} has epoch 0; only entities a material input touched are listed"
        )));
    }
    Ok(())
}

/// Checks the canonical form of the queued inputs: instants strictly increasing, none before the
/// cursor, no empty list.
/// A pending timer must not be dated before the cursor: one that was due has fired (or, cancelled,
/// was dropped when its instant ran), so an earlier entry is a forged or corrupt state
/// (VER-SPR-007-TEST GAP-9), refused as queued inputs are. `entries` are in key order, so the
/// first is the earliest.
pub(crate) fn check_timers(
    entries: &[(AgendaKey, Timer)],
    cursor: Option<SimTime>,
) -> Result<(), RestoreError> {
    match (entries.first(), cursor) {
        (Some((key, _)), Some(c)) if key.time < c => Err(RestoreError::TimerTime(format!(
            "a timer at {} is before the cursor {c}",
            key.time
        ))),
        (Some((key, _)), None) => Err(RestoreError::TimerTime(format!(
            "a timer at {} after time has run out",
            key.time
        ))),
        _ => Ok(()),
    }
}

pub(crate) fn check_inputs(
    inputs: &[(SimTime, Vec<MaterialInput>)],
    cursor: Option<SimTime>,
    accepted: u64,
) -> Result<(), RestoreError> {
    let queued: usize = inputs.iter().map(|(_, list)| list.len()).sum();
    if u64::try_from(queued).unwrap_or(u64::MAX) > accepted {
        return Err(RestoreError::Inputs(format!(
            "{queued} inputs are queued but only {accepted} were accepted"
        )));
    }
    if let Some(w) = inputs.windows(2).find(|w| w[0].0 >= w[1].0) {
        return Err(RestoreError::Inputs(format!(
            "instants are not strictly increasing: {} then {}",
            w[0].0, w[1].0
        )));
    }
    if let Some((t, _)) = inputs.iter().find(|(_, list)| list.is_empty()) {
        return Err(RestoreError::Inputs(format!("an empty list at {t}")));
    }
    match (inputs.first(), cursor) {
        (Some((t, _)), Some(c)) if *t < c => Err(RestoreError::Inputs(format!(
            "an input at {t} is before the cursor {c}"
        ))),
        (Some((t, _)), None) => Err(RestoreError::Inputs(format!(
            "an input at {t} after time has run out"
        ))),
        _ => Ok(()),
    }
}
