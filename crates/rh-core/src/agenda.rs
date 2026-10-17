//! The agenda: a binary-heap queue of timers keyed by `(time, phase, entity, seq)` (SPR-006 T-01,
//! T-02; `05.1` §4.3; ADR-0025).
//!
//! **Key.** [`AgendaKey`] orders by time, then phase ([`Phase`] order), then entity
//! ([`EntityRef`]: kind tag, then raw id), then `seq`. `seq` comes from one counter per timeline,
//! is assigned when a timer is inserted (the engine inserts only at the canonical merge, SPR-006
//! ruling 1) and is never reused, so every key is unique and the pop order is a pure function of
//! the keys: nothing depends on how `BinaryHeap` orders equal elements (ruling 2).
//!
//! **Cancellation** is by [`TimerId`] (the timer's `seq`) with lazy deletion: the id goes into a
//! tombstone set, the heap entry stays, and a pop discards a tombstoned entry and clears its
//! tombstone. A cancelled timer never fires.
//!
//! **State** ([`AgendaState`]) is a plain sorted dump of every `(key, timer)` pair still in the
//! heap (tombstoned ones included), the counter and the tombstones: what a keyframe stores
//! (ruling 7, SPR-007). [`Agenda::restore`] rebuilds an agenda that pops exactly as the original.
//!
//! The agenda itself does not know the current instant; the engine refuses a timer that is not
//! strictly after the current `(instant, phase)` (no back-dating, ADR-0025 item 2) before
//! inserting it.
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §4.1, §4.3; ADR-0025;
//! `CODEX/05_PROJECT/SPR-006_AgendaEventsInvariants.md` T-01, T-02, rulings 1, 2 and 7.

use core::cmp::Reverse;
use core::fmt;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use rh_events::EventRef;
use rh_ids::{EntityRef, PurposeId, SystemId};

use crate::phase::Phase;
use crate::time::SimTime;

/// A timer's identity: its `seq`, unique in the timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimerId(u64);

impl TimerId {
    /// Wraps a raw `seq` (restoring a dump, tests).
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw `seq`.
    #[must_use]
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for TimerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "timer#{}", self.0)
    }
}

/// The agenda key `(time, phase, entity, seq)`. The derived order is the pop order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgendaKey {
    /// The instant the timer fires at.
    pub time: SimTime,
    /// The phase it fires in.
    pub phase: Phase,
    /// The entity it is about.
    pub entity: EntityRef,
    /// The timeline-wide insertion counter.
    pub seq: u64,
}

impl AgendaKey {
    /// The timer's id.
    #[must_use]
    pub const fn id(&self) -> TimerId {
        TimerId(self.seq)
    }
}

/// What fires: the owning system, the RNG purpose its time was drawn under, and a closure id
/// with an integer argument ("a heap entry plus a closure ID", ADR-0025).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timer {
    /// The owning system's RNG `system_id` (name-derived, stable): only it can cancel the timer,
    /// and it handles the firing.
    pub owner: SystemId,
    /// The purpose the timer's time was drawn under; the re-sample key uses it (ADR-0025 item 3).
    pub purpose: PurposeId,
    /// The owner's closure id: which outcome this is.
    pub kind: u32,
    /// The owner's argument.
    pub payload: i64,
    /// The event that caused the outcome, if any (`05.1` §2 rule 2; EVO-001 R9): the merge
    /// resolves a draft of the scheduling step to its id, so in the agenda it is `Known` or
    /// `None`. A re-sample keeps it. Hash-neutral when `None` (R15): the timer row's tail
    /// `u8 1 ‖ u64 cause` is written only when present.
    pub cause: EventRef,
}

/// A timer with its key: what a system's `fire` and `resample_timer` receive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerEntry {
    /// The key.
    pub key: AgendaKey,
    /// The timer.
    pub timer: Timer,
}

impl TimerEntry {
    /// The timer's id.
    #[must_use]
    pub const fn id(&self) -> TimerId {
        self.key.id()
    }
}

/// Every entry still in the heap, the counter and the tombstones: the serialisable state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgendaState {
    /// `(key, timer)` pairs sorted by key, tombstoned ones included.
    pub entries: Vec<(AgendaKey, Timer)>,
    /// The next `seq`.
    pub next_seq: u64,
    /// Cancelled timers still in the heap, ascending.
    pub tombstones: Vec<TimerId>,
}

/// A refused agenda operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgendaError {
    /// The id is not a pending timer (never inserted, already fired, or already cancelled).
    NotPending(TimerId),
    /// A dump that cannot be an agenda's state.
    BadState(String),
}

impl fmt::Display for AgendaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPending(id) => write!(f, "{id} is not a pending timer"),
            Self::BadState(why) => write!(f, "not an agenda state: {why}"),
        }
    }
}

impl std::error::Error for AgendaError {}

/// The agenda of one timeline.
#[derive(Clone, Debug, Default)]
pub struct Agenda {
    heap: BinaryHeap<Reverse<AgendaKey>>,
    entries: BTreeMap<u64, (AgendaKey, Timer)>,
    tombstones: BTreeSet<u64>,
    next_seq: u64,
    /// The pending (not tombstoned) timers of each entity, in key order: derived state, rebuilt by
    /// `restore`, never serialised.
    by_entity: BTreeMap<EntityRef, BTreeSet<AgendaKey>>,
}

impl Agenda {
    /// An empty agenda; the first timer gets `seq` 0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a timer at `(time, phase, entity)`, assigning the next `seq`, and returns its id.
    pub fn insert(
        &mut self,
        time: SimTime,
        phase: Phase,
        entity: EntityRef,
        timer: Timer,
    ) -> TimerId {
        let seq = self.next_seq;
        self.next_seq += 1;
        let key = AgendaKey {
            time,
            phase,
            entity,
            seq,
        };
        self.heap.push(Reverse(key));
        self.entries.insert(seq, (key, timer));
        self.by_entity.entry(entity).or_default().insert(key);
        TimerId(seq)
    }

    /// Cancels a pending timer (lazy deletion: a tombstone checked at pop).
    ///
    /// # Errors
    /// [`AgendaError::NotPending`] if `id` is not pending.
    pub fn cancel(&mut self, id: TimerId) -> Result<(AgendaKey, Timer), AgendaError> {
        match self.entries.get(&id.0) {
            Some(&entry) if !self.tombstones.contains(&id.0) => {
                self.tombstones.insert(id.0);
                self.unindex(&entry.0);
                self.normalize();
                Ok(entry)
            }
            _ => Err(AgendaError::NotPending(id)),
        }
    }

    /// The pending timer `id`, if it is pending.
    #[must_use]
    pub fn get(&self, id: TimerId) -> Option<(AgendaKey, Timer)> {
        if self.tombstones.contains(&id.0) {
            return None;
        }
        self.entries.get(&id.0).copied()
    }

    /// Whether `id` is pending.
    #[must_use]
    pub fn is_pending(&self, id: TimerId) -> bool {
        self.get(id).is_some()
    }

    /// The key of the next timer to fire, if any.
    #[must_use]
    pub fn peek(&self) -> Option<AgendaKey> {
        // INVARIANT: `normalize` runs after every mutation, so the top is never tombstoned.
        self.heap.peek().map(|r| r.0)
    }

    /// Pops the next pending timer, skipping cancelled ones.
    pub fn pop(&mut self) -> Option<(AgendaKey, Timer)> {
        let Reverse(key) = self.heap.pop()?;
        let entry = self.entries.remove(&key.seq);
        self.unindex(&key);
        self.normalize();
        entry
    }

    /// Pops every pending timer due at exactly `(time, phase)`, in key order.
    pub fn pop_due(&mut self, time: SimTime, phase: Phase) -> Vec<(AgendaKey, Timer)> {
        let mut due = Vec::new();
        while let Some(key) = self.peek() {
            if (key.time, key.phase) != (time, phase) {
                break;
            }
            if let Some(entry) = self.pop() {
                due.push(entry);
            }
        }
        due
    }

    /// The number of pending timers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len() - self.tombstones.len()
    }

    /// Whether no timer is pending.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The pending timers in key order.
    #[must_use]
    pub fn pending(&self) -> Vec<(AgendaKey, Timer)> {
        let mut out: Vec<(AgendaKey, Timer)> = self
            .entries
            .iter()
            .filter(|(seq, _)| !self.tombstones.contains(seq))
            .map(|(_, e)| *e)
            .collect();
        out.sort_unstable_by_key(|(k, _)| *k);
        out
    }

    /// The pending timers about `entity`, in key order.
    #[must_use]
    pub fn pending_for(&self, entity: EntityRef) -> Vec<(AgendaKey, Timer)> {
        self.by_entity
            .get(&entity)
            .into_iter()
            .flatten()
            .filter_map(|k| self.entries.get(&k.seq).copied())
            .collect()
    }

    /// The serialisable state (ruling 7).
    #[must_use]
    pub fn state(&self) -> AgendaState {
        let mut entries: Vec<(AgendaKey, Timer)> = self.entries.values().copied().collect();
        entries.sort_unstable_by_key(|(k, _)| *k);
        AgendaState {
            entries,
            next_seq: self.next_seq,
            tombstones: self.tombstones.iter().map(|&s| TimerId(s)).collect(),
        }
    }

    /// Rebuilds an agenda from its state, which must be in the canonical form [`Agenda::state`]
    /// produces, so that `restore(x).state() == x` and a keyframe's hash of `x` is the agenda's
    /// (SPR-006 GAP-6, ruled for SPR-007 T-02).
    ///
    /// # Errors
    /// [`AgendaError::BadState`] unless: the entries are in strictly increasing key order; every
    /// `seq` is below the counter and appears once; the tombstones are strictly increasing and
    /// each names an entry; and the first entry (the heap's top) is not tombstoned, since
    /// cancelling the top drops it at once.
    pub fn restore(state: &AgendaState) -> Result<Self, AgendaError> {
        let bad = |m: String| Err(AgendaError::BadState(m));
        if let Some(&[(a, _), (b, _)]) = state.entries.windows(2).find(|w| w[0].0 >= w[1].0) {
            return bad(format!(
                "entries are not in strictly increasing key order at seq {} then {}",
                a.seq, b.seq
            ));
        }
        if let Some(&[a, b]) = state.tombstones.windows(2).find(|w| w[0] >= w[1]) {
            return bad(format!(
                "tombstones are not strictly increasing at {a} then {b}"
            ));
        }
        if let (Some(&(top, _)), Some(_)) = (state.entries.first(), state.tombstones.first())
            && state.tombstones.binary_search(&top.id()).is_ok()
        {
            return bad(format!(
                "the first entry, seq {}, is tombstoned; state() never holds a cancelled top",
                top.seq
            ));
        }
        let mut agenda = Self {
            next_seq: state.next_seq,
            ..Self::default()
        };
        for &(key, timer) in &state.entries {
            if key.seq >= state.next_seq {
                return Err(AgendaError::BadState(format!(
                    "seq {} is not below the counter {}",
                    key.seq, state.next_seq
                )));
            }
            if agenda.entries.insert(key.seq, (key, timer)).is_some() {
                return Err(AgendaError::BadState(format!(
                    "seq {} appears twice",
                    key.seq
                )));
            }
            agenda.heap.push(Reverse(key));
            agenda.by_entity.entry(key.entity).or_default().insert(key);
        }
        for id in &state.tombstones {
            if !agenda.entries.contains_key(&id.0) || !agenda.tombstones.insert(id.0) {
                return Err(AgendaError::BadState(format!(
                    "tombstone {id} names no entry or appears twice"
                )));
            }
            if let Some(&(key, _)) = agenda.entries.get(&id.0) {
                agenda.unindex(&key);
            }
        }
        agenda.normalize();
        if agenda.state() != *state {
            return bad("the state is not in the canonical form state() produces".to_owned());
        }
        Ok(agenda)
    }

    /// Removes a key from the entity index.
    fn unindex(&mut self, key: &AgendaKey) {
        if let Some(keys) = self.by_entity.get_mut(&key.entity) {
            keys.remove(key);
            if keys.is_empty() {
                self.by_entity.remove(&key.entity);
            }
        }
    }

    /// Drops tombstoned entries from the top of the heap.
    fn normalize(&mut self) {
        while let Some(Reverse(key)) = self.heap.peek() {
            if !self.tombstones.remove(&key.seq) {
                break;
            }
            let seq = key.seq;
            self.heap.pop();
            self.entries.remove(&seq);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Agenda, AgendaError, Timer, TimerId};
    use crate::phase::Phase;
    use crate::time::SimTime;
    use rh_ids::{EntityKind, EntityRef, PurposeId, SystemId};

    fn timer(kind: u32) -> Timer {
        Timer {
            owner: SystemId::from_raw(1),
            purpose: PurposeId::from_raw(2),
            kind,
            payload: 0,
            cause: rh_events::EventRef::None,
        }
    }

    fn person(raw: u64) -> EntityRef {
        EntityRef::new(EntityKind::Person, raw)
    }

    #[test]
    fn pops_in_key_order_and_skips_cancelled() {
        let mut a = Agenda::new();
        let t = SimTime::from_minutes;
        let late = a.insert(t(20), Phase::Vital, person(1), timer(0));
        let agenda_phase = a.insert(t(10), Phase::Agenda, person(1), timer(1));
        let vital_p2 = a.insert(t(10), Phase::Vital, person(2), timer(2));
        let vital_p1 = a.insert(t(10), Phase::Vital, person(1), timer(3));
        let vital_p1_again = a.insert(t(10), Phase::Vital, person(1), timer(4));
        assert_eq!(a.cancel(vital_p2).map(|(k, _)| k.id()), Ok(vital_p2));
        assert_eq!(a.cancel(vital_p2), Err(AgendaError::NotPending(vital_p2)));
        assert_eq!(a.len(), 4);
        let order: Vec<TimerId> = std::iter::from_fn(|| a.pop())
            .map(|(k, _)| k.id())
            .collect();
        assert_eq!(order, [vital_p1, vital_p1_again, agenda_phase, late]);
        assert!(a.is_empty());
        assert_eq!(a.cancel(late), Err(AgendaError::NotPending(late)), "fired");
    }

    #[test]
    fn pop_due_takes_exactly_one_instant_and_phase() {
        let mut a = Agenda::new();
        let t = SimTime::from_minutes;
        a.insert(t(5), Phase::Vital, person(3), timer(0));
        a.insert(t(5), Phase::Vital, person(1), timer(1));
        a.insert(t(5), Phase::Agenda, person(1), timer(2));
        a.insert(t(6), Phase::Inputs, person(1), timer(3));
        assert!(a.pop_due(t(5), Phase::Inputs).is_empty());
        let due: Vec<u32> = a
            .pop_due(t(5), Phase::Vital)
            .iter()
            .map(|(_, x)| x.kind)
            .collect();
        assert_eq!(due, [1, 0]);
        assert_eq!(a.peek().map(|k| k.phase), Some(Phase::Agenda));
    }

    #[test]
    fn state_round_trips_with_tombstones() {
        let mut a = Agenda::new();
        let t = SimTime::from_minutes;
        for i in 0..6 {
            a.insert(
                t(100 - i),
                Phase::Vital,
                person(i.cast_unsigned()),
                timer(0),
            );
        }
        a.cancel(TimerId::from_raw(4))
            .unwrap_or_else(|e| panic!("{e}"));
        a.cancel(TimerId::from_raw(1))
            .unwrap_or_else(|e| panic!("{e}"));
        let state = a.state();
        assert!(state.entries.windows(2).all(|w| w[0].0 < w[1].0));
        let mut b = Agenda::restore(&state).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(b.state(), state);
        let mut a2 = a.clone();
        let pa: Vec<_> = std::iter::from_fn(|| a2.pop()).collect();
        let pb: Vec<_> = std::iter::from_fn(|| b.pop()).collect();
        assert_eq!(pa, pb);
        assert_eq!(pa.len(), 4);

        let mut bad = state.clone();
        bad.next_seq = 3;
        assert!(Agenda::restore(&bad).is_err());
        let mut bad = state.clone();
        bad.tombstones.push(TimerId::from_raw(99));
        assert!(Agenda::restore(&bad).is_err());
        // GAP-6: only the canonical form is accepted.
        let mut bad = state.clone();
        bad.entries.swap(0, 1);
        assert!(Agenda::restore(&bad).is_err(), "unsorted entries");
        let mut bad = state.clone();
        bad.entries.push(bad.entries[0]);
        assert!(Agenda::restore(&bad).is_err(), "a repeated entry");
        let mut bad = state.clone();
        bad.tombstones.reverse();
        assert!(Agenda::restore(&bad).is_err(), "unsorted tombstones");
        let mut bad = state.clone();
        bad.tombstones.push(bad.tombstones[0]);
        assert!(Agenda::restore(&bad).is_err(), "a repeated tombstone");
        let mut bad = state.clone();
        let top = bad.entries[0].0.id();
        bad.tombstones.insert(0, top);
        assert!(Agenda::restore(&bad).is_err(), "a tombstoned top");
    }

    /// GAP-6: `restore(x).state() == x` for agendas built by random inserts, cancels and pops.
    #[test]
    fn restore_of_state_is_the_identity_on_random_agendas() {
        let mut z: u64 = 0x5EED_AA06;
        let mut next = move || {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^ (x >> 31)
        };
        for _ in 0..200 {
            let mut a = Agenda::new();
            let mut ids = Vec::new();
            for _ in 0..(next() % 40) {
                match next() % 4 {
                    0 | 1 => ids.push(a.insert(
                        SimTime::from_minutes((next() % 50).cast_signed()),
                        Phase::ALL[usize::try_from(next() % 4).unwrap_or(0)],
                        person(next() % 5),
                        timer(u32::try_from(next() % 7).unwrap_or(0)),
                    )),
                    2 if !ids.is_empty() => {
                        let id = ids[usize::try_from(next()).unwrap_or(0) % ids.len()];
                        let _ = a.cancel(id);
                    }
                    _ => {
                        let _ = a.pop();
                    }
                }
            }
            let state = a.state();
            let b = Agenda::restore(&state).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(b.state(), state);
            assert_eq!(b.pending(), a.pending());
        }
    }
}
