//! `rh-events`: the event schema, causal edges and emission-order `EventId`s (SPR-006 T-04).
//!
//! Plane A, machinery. A system drafts an event ([`EventDraft`]) during its step; `rh-core`
//! merges every step's drafts at the end of each scheduler level in the canonical order (`05.1`
//! §2 rule 4) and appends them here. [`EventLog::append`] is the only way an event gets an
//! [`EventId`], and it enforces ADR-0025 item 2 and SPR-006 ruling 4:
//!
//! - **Emission order.** `EventId`s are allocated from one counter per timeline, in append order,
//!   so they strictly increase. Appends must come in non-decreasing `(tick, phase)` order, so the
//!   `EventId` order is the emission order `(tick, phase, seq)`; an append at an earlier
//!   `(tick, phase)` than the last one is refused ([`EventError::BackDated`]).
//! - **No back-dating.** The caller passes the current instant and phase; an event's tick is
//!   always that instant. An outcome in the future is a timer in the agenda, never an event.
//! - **Causal edges point back.** Every cause of an event is an `EventId` already allocated, so it
//!   is smaller than the new event's id ([`EventError::ForwardCause`]). A step names an event it
//!   emitted earlier in the same step by a [`DraftRef`]; the merge resolves it to that event's id,
//!   which is then smaller by construction.
//!
//! The phase is stored as its 0-based index (`rh_core::Phase::index`): this crate sits below
//! `rh-core` and does not name its types.
//!
//! The log is in memory; the World Stream writer (ADR-0030) and the chronicle drain it later.
//! [`EventLog::hash`] folds every event in id order, so event order is part of the hash contract.
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; ADR-0007; ADR-0025 item 2;
//! `docs/09` V-DET-12; `CODEX/05_PROJECT/SPR-006_AgendaEventsInvariants.md` T-04, rulings 1 and 4.
#![forbid(unsafe_code)]

use core::fmt;

use rh_ids::{EntityRef, EventId};

/// Domain tag of [`EventLog::hash`].
const TAG_EVENT_HASH: u64 = rh_ids::rng::tag("rehoboam/rh-events/log-hash/v1");

/// An earlier draft of the same step: the `n`-th event the step emitted, counting from 0. A step
/// cannot know the `EventId` of an event it emits (ids are assigned at the merge), so it names it
/// by this reference; the merge replaces it with the id (`rh-core`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DraftRef(pub usize);

/// An event as a system drafts it: no id, tick or phase yet (the merge assigns them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDraft {
    /// The event kind, as the emitter's spec declares it (`[[emits]] kind`).
    pub kind: &'static str,
    /// The entities the event is about, in the emitter's role order.
    pub subjects: Vec<EntityRef>,
    /// Causal edges: earlier events this one follows from.
    pub causes: Vec<EventId>,
    /// Causal edges to earlier drafts of the same step; resolved into `causes` before
    /// [`EventLog::append`], which refuses a draft that still has any.
    pub local_causes: Vec<DraftRef>,
    /// Kind-specific integer payload.
    pub payload: Vec<i64>,
}

impl EventDraft {
    /// A draft of `kind` with no subjects, causes or payload.
    #[must_use]
    pub fn new(kind: &'static str) -> Self {
        Self {
            kind,
            subjects: Vec::new(),
            causes: Vec::new(),
            local_causes: Vec::new(),
            payload: Vec::new(),
        }
    }

    /// Adds a subject.
    #[must_use]
    pub fn subject(mut self, entity: EntityRef) -> Self {
        self.subjects.push(entity);
        self
    }

    /// Adds a causal edge.
    #[must_use]
    pub fn caused_by(mut self, cause: EventId) -> Self {
        self.causes.push(cause);
        self
    }

    /// Adds a causal edge to an earlier draft of the same step.
    #[must_use]
    pub fn after(mut self, draft: DraftRef) -> Self {
        self.local_causes.push(draft);
        self
    }

    /// Sets the payload.
    #[must_use]
    pub fn with_payload(mut self, payload: Vec<i64>) -> Self {
        self.payload = payload;
        self
    }
}

/// An emitted event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    /// Its id: the timeline's event counter at emission.
    pub id: EventId,
    /// The instant it was emitted at, in `i64` minutes (`SimTime`).
    pub tick: i64,
    /// The phase it was emitted in (`Phase::index`).
    pub phase: u8,
    /// The emitting system's Registry id.
    pub emitter: &'static str,
    /// The kind.
    pub kind: &'static str,
    /// The subjects.
    pub subjects: Vec<EntityRef>,
    /// The causal edges, all to smaller ids.
    pub causes: Vec<EventId>,
    /// The payload.
    pub payload: Vec<i64>,
}

impl Event {
    /// The emission-order key `(tick, phase, seq)`; `seq` is the id's raw value.
    #[must_use]
    pub fn order_key(&self) -> (i64, u8, u64) {
        (self.tick, self.phase, self.id.to_raw())
    }
}

/// A refused append.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventError {
    /// The append is at an earlier `(tick, phase)` than the last event (ADR-0025 item 2).
    BackDated {
        /// The kind.
        kind: &'static str,
        /// The `(tick, phase)` of the append.
        at: (i64, u8),
        /// The `(tick, phase)` of the last event.
        last: (i64, u8),
    },
    /// The draft still has draft-local causes: the caller resolves them first.
    Unresolved {
        /// The kind.
        kind: &'static str,
    },
    /// A cause is not an earlier event (SPR-006 ruling 4).
    ForwardCause {
        /// The kind.
        kind: &'static str,
        /// The offending cause.
        cause: EventId,
        /// The id the new event would get.
        id: EventId,
    },
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackDated { kind, at, last } => write!(
                f,
                "event {kind} at (tick {}, phase {}) is earlier than the last event at (tick {}, phase {}): no back-dating (ADR-0025)",
                at.0, at.1, last.0, last.1
            ),
            Self::Unresolved { kind } => {
                write!(f, "event {kind} still has unresolved draft-local causes")
            }
            Self::ForwardCause { kind, cause, id } => write!(
                f,
                "event {kind} (id {id}) names cause {cause}, which is not an earlier event"
            ),
        }
    }
}

impl std::error::Error for EventError {}

/// The event log of one timeline: the events in id order and the counter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventLog {
    events: Vec<Event>,
    next_seq: u64,
    last: Option<(i64, u8)>,
}

impl EventLog {
    /// An empty log; the first event gets id 0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The id the next event will get. Every allocated id is smaller.
    #[must_use]
    pub fn next_id(&self) -> EventId {
        EventId::from_raw(self.next_seq)
    }

    /// The `(tick, phase)` of the last event appended, if any (kept across [`EventLog::drain`]).
    #[must_use]
    pub fn last(&self) -> Option<(i64, u8)> {
        self.last
    }

    /// The events held, in id order.
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Hands the held events to a consumer (the stream writer). The counter and the last
    /// `(tick, phase)` stay, so ids never repeat and order is still enforced.
    pub fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Appends a draft emitted by `emitter` at instant `tick` in phase `phase`, and returns its id.
    ///
    /// # Errors
    /// [`EventError::BackDated`] if `(tick, phase)` is before the last event's;
    /// [`EventError::Unresolved`] if the draft still has draft-local causes;
    /// [`EventError::ForwardCause`] if a cause is not smaller than the new id. Nothing is appended
    /// and the counter does not move.
    pub fn append(
        &mut self,
        tick: i64,
        phase: u8,
        emitter: &'static str,
        draft: EventDraft,
    ) -> Result<EventId, EventError> {
        if let Some(last) = self.last
            && (tick, phase) < last
        {
            return Err(EventError::BackDated {
                kind: draft.kind,
                at: (tick, phase),
                last,
            });
        }
        if !draft.local_causes.is_empty() {
            return Err(EventError::Unresolved { kind: draft.kind });
        }
        let id = self.next_id();
        if let Some(&cause) = draft.causes.iter().find(|c| **c >= id) {
            return Err(EventError::ForwardCause {
                kind: draft.kind,
                cause,
                id,
            });
        }
        self.events.push(Event {
            id,
            tick,
            phase,
            emitter,
            kind: draft.kind,
            subjects: draft.subjects,
            causes: draft.causes,
            payload: draft.payload,
        });
        self.next_seq += 1;
        self.last = Some((tick, phase));
        Ok(id)
    }

    /// A hash of the held events in id order (every field) and of the counter: order-sensitive.
    /// A stub fold until the chunked Merkle hashes of `05.1` §6 item 6 (SPR-007).
    #[must_use]
    pub fn hash(&self) -> u64 {
        use rh_ids::rng::{h64_bytes, h64_words};
        let mut h = h64_words(TAG_EVENT_HASH, &[self.next_seq]);
        for e in &self.events {
            let mut words = vec![
                e.id.to_raw(),
                e.tick.cast_unsigned(),
                u64::from(e.phase),
                h64_bytes(TAG_EVENT_HASH, e.emitter.as_bytes()),
                h64_bytes(TAG_EVENT_HASH, e.kind.as_bytes()),
                e.subjects.len() as u64,
            ];
            for s in &e.subjects {
                words.push(u64::from(s.kind().tag()));
                words.push(s.raw());
            }
            words.push(e.causes.len() as u64);
            words.extend(e.causes.iter().map(|c| c.to_raw()));
            words.push(e.payload.len() as u64);
            words.extend(e.payload.iter().map(|v| v.cast_unsigned()));
            h = h64_words(h, &words);
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::{EventDraft, EventError, EventLog};
    use rh_ids::{EntityRef, EventId, PersonId};

    #[test]
    fn ids_follow_append_order_and_back_dating_is_refused() {
        let mut log = EventLog::new();
        let a = log.append(10, 4, "M2.demography", EventDraft::new("DEATH"));
        let b = log.append(10, 4, "M2.demography", EventDraft::new("DEATH"));
        let c = log.append(10, 9, "S7.vows", EventDraft::new("VOW_VOIDED"));
        assert_eq!(
            (a, b, c),
            (
                Ok(EventId::from_raw(0)),
                Ok(EventId::from_raw(1)),
                Ok(EventId::from_raw(2))
            )
        );
        let early = log.append(10, 4, "M2.demography", EventDraft::new("DEATH"));
        assert!(matches!(early, Err(EventError::BackDated { .. })));
        let past = log.append(9, 11, "M2.demography", EventDraft::new("DEATH"));
        assert!(matches!(past, Err(EventError::BackDated { .. })));
        assert_eq!(
            log.next_id(),
            EventId::from_raw(3),
            "a refusal allocates nothing"
        );
        let keys: Vec<_> = log.events().iter().map(super::Event::order_key).collect();
        assert!(keys.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn causes_point_back() {
        let mut log = EventLog::new();
        let d = log
            .append(0, 4, "M2.demography", EventDraft::new("DEATH"))
            .unwrap_or_else(|e| panic!("{e}"));
        let ok = log.append(
            0,
            9,
            "S7.vows",
            EventDraft::new("VOW_VOIDED")
                .caused_by(d)
                .subject(EntityRef::person(PersonId::from_raw(7))),
        );
        assert_eq!(ok, Ok(EventId::from_raw(1)));
        let self_cause = log.append(
            0,
            9,
            "S7.vows",
            EventDraft::new("VOW_VOIDED").caused_by(EventId::from_raw(2)),
        );
        assert!(matches!(self_cause, Err(EventError::ForwardCause { .. })));
        let local = log.append(
            0,
            9,
            "S7.vows",
            EventDraft::new("VOW_VOIDED").after(super::DraftRef(0)),
        );
        assert!(matches!(local, Err(EventError::Unresolved { .. })));
    }

    #[test]
    fn drain_keeps_the_counter_and_the_order_floor() {
        let mut log = EventLog::new();
        let h0 = log.hash();
        log.append(5, 1, "X", EventDraft::new("A").with_payload(vec![1]))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_ne!(log.hash(), h0);
        assert_eq!(log.drain().len(), 1);
        assert!(log.events().is_empty());
        assert_eq!(log.next_id(), EventId::from_raw(1));
        assert!(log.append(4, 1, "X", EventDraft::new("A")).is_err());
    }
}
