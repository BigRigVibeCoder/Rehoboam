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
//! - **The envelope** (`specs/chronicle/envelope_v0.md`, SPR-020 T-02). A draft carries its
//!   place, magnitude and secrecy, its cast ([`RoleRow`]), its anchored snapshots ([`Snapshot`])
//!   and typed causal edges ([`CausalEdge`]); the engine stamps the [`Origin`]. The merge
//!   resolves every [`EventRef::Draft`] and coalesces edges to one per cause ([`coalesce`]);
//!   [`EventLog::append`] refuses what §5 forbids, and [`Event::digest`] (version 2) hashes every
//!   stored field but the admission flag.
//!
//! The phase is stored as its 0-based index (`rh_core::Phase::index`): this crate sits below
//! `rh-core` and does not name its types.
//!
//! The log is in memory; the World Stream writer (ADR-0030) and the chronicle drain it later.
//! [`EventLog::hash`] is a running fold of every event in id order that draining does not reset,
//! so event order is part of the hash contract and the drain cadence is not.
//!
//! REF: `docs/05_architecture/05.0_overview.md` §6; ADR-0007; ADR-0025 item 2;
//! `docs/09` V-DET-12; `CODEX/05_PROJECT/SPR-006_AgendaEventsInvariants.md` T-04, rulings 1 and 4;
//! `CODEX/05_PROJECT/SPR-020_EventEnvelope.md` T-02; REV-SPR-020 R3, R4, R8, R9, R21; REV-SPR-029 R20.
#![forbid(unsafe_code)]

mod coalesce;
mod envelope;
mod event_ref;

use core::fmt;

use rh_ids::{EntityRef, EventId, PlaceId};

pub use coalesce::{coalesce, coalesce_draft};
pub use envelope::{
    CausalEdge, EVIDENCE_ONE, Edge, EdgeKind, Origin, RoleId, RoleRow, Secrecy, Snapshot,
};
pub use event_ref::EventRef;

/// Domain tag of [`EventLog::hash`], of [`Event::digest`] and of the names they hash: version 2,
/// the event envelope (`envelope_v0.md` §6).
const TAG_EVENT_HASH: u64 = rh_ids::rng::tag("rehoboam/rh-events/log-hash/v2");

/// The most role rows an event may hold: the ordinal is a `u8` (envelope §1; R12 NIT-5).
pub const MAX_CAST: usize = 256;

/// An earlier draft of the same step: the `n`-th event the step emitted, counting from 0. A step
/// cannot know the `EventId` of an event it emits (ids are assigned at the merge), so it names it
/// by this reference; the merge replaces it with the id (`rh-core`).
///
/// Opaque (the Tester's GAP-1, SPR-014): a system gets one only from `StepCtx::emit`, never from
/// an integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DraftRef(usize);

impl DraftRef {
    /// The engine's constructor: `rh-core` numbers a step's drafts at `emit`, and `rh-snapshot`
    /// restores a queued command's draft from a keyframe. Not for systems.
    #[doc(hidden)]
    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        Self(index)
    }

    /// The draft's position among its step's emitted events, counting from 0.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

// `EventRef`'s text is shared with SPR-020 and kept identical (Architect, SPR-014): its methods
// live here, and the slot's opacity is documented on the type.
impl EventRef {
    /// The event's id, once it is known (always, after the merge, unless the slot is empty).
    #[must_use]
    pub const fn known(self) -> Option<EventId> {
        match self {
            Self::Known(id) => Some(id),
            Self::None | Self::Draft(_) => None,
        }
    }

    /// The draft this slot names, before the merge resolves it.
    #[must_use]
    pub const fn draft(self) -> Option<DraftRef> {
        match self {
            Self::Draft(d) => Some(d),
            Self::None | Self::Known(_) => None,
        }
    }

    /// Whether the slot is empty.
    #[must_use]
    pub const fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    /// The slot with a `Draft` replaced by the id `resolve` gives it (the merge, `rh-core`);
    /// `None` and `Known` are kept. A draft `resolve` does not know becomes `None`.
    #[must_use]
    pub fn resolved(self, resolve: impl FnOnce(DraftRef) -> Option<EventId>) -> Self {
        match self {
            Self::Draft(d) => resolve(d).map_or(Self::None, Self::Known),
            other => other,
        }
    }
}

impl From<DraftRef> for EventRef {
    fn from(d: DraftRef) -> Self {
        Self::Draft(d)
    }
}

impl From<EventId> for EventRef {
    fn from(id: EventId) -> Self {
        Self::Known(id)
    }
}

/// An event as a system drafts it (`envelope_v0.md` §2): no id, tick, phase, origin or admission
/// flag yet (the engine stamps the origin; the merge assigns the rest).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDraft {
    /// The event kind, as the emitter's spec declares it (`[[emits]] kind`).
    pub kind: &'static str,
    /// `event.place_id`: none until places exist (SPR-018).
    pub place: Option<PlaceId>,
    /// `event.magnitude`: 0 unless the verb's spec gives it a meaning.
    pub magnitude: i32,
    /// `event.secrecy`: public unless the spec says.
    pub secrecy: Secrecy,
    /// The cast: role rows (sorted and checked by `rh-core`'s `emit`).
    pub roles: Vec<RoleRow>,
    /// The anchored snapshots (sorted and checked by `emit`).
    pub snapshots: Vec<Snapshot>,
    /// The causal edges, to earlier drafts of the step or to known events; the merge resolves and
    /// coalesces them.
    pub causes: Vec<CausalEdge>,
    /// Kind-specific integer payload.
    pub payload: Vec<i64>,
}

impl EventDraft {
    /// A public draft of `kind` with no place, magnitude, cast, snapshot, cause or payload.
    #[must_use]
    pub fn new(kind: &'static str) -> Self {
        Self {
            kind,
            place: None,
            magnitude: 0,
            secrecy: Secrecy::Public,
            roles: Vec::new(),
            snapshots: Vec::new(),
            causes: Vec::new(),
            payload: Vec::new(),
        }
    }

    /// Sets the place.
    #[must_use]
    pub fn at(mut self, place: PlaceId) -> Self {
        self.place = Some(place);
        self
    }

    /// Sets the magnitude.
    #[must_use]
    pub fn magnitude(mut self, magnitude: i32) -> Self {
        self.magnitude = magnitude;
        self
    }

    /// Sets the secrecy.
    #[must_use]
    pub fn secrecy(mut self, secrecy: Secrecy) -> Self {
        self.secrecy = secrecy;
        self
    }

    /// Adds a role row.
    #[must_use]
    pub fn role(mut self, role: RoleId, entity: EntityRef) -> Self {
        self.roles.push(RoleRow { role, entity });
        self
    }

    /// Adds a witness (the one role that may repeat).
    #[must_use]
    pub fn witness(self, entity: EntityRef) -> Self {
        self.role(RoleId::WITNESS, entity)
    }

    /// Adds a one-argument snapshot of `attr` of `entity`.
    #[must_use]
    pub fn snapshot(mut self, attr: &'static str, entity: EntityRef, value: i32) -> Self {
        self.snapshots.push(Snapshot {
            attr,
            entity,
            partner: None,
            value,
        });
        self
    }

    /// Adds a two-argument snapshot of `attr` of `(entity, partner)`.
    #[must_use]
    pub fn relational_snapshot(
        mut self,
        attr: &'static str,
        entity: EntityRef,
        partner: EntityRef,
        value: i32,
    ) -> Self {
        self.snapshots.push(Snapshot {
            attr,
            entity,
            partner: Some(partner),
            value,
        });
        self
    }

    /// Adds a snapshot of `attr` of the event itself (stored as the world: §4 rule 3).
    #[must_use]
    pub fn event_snapshot(self, attr: &'static str, value: i32) -> Self {
        self.snapshot(attr, EntityRef::WORLD, value)
    }

    /// Adds a causal edge of a non-motivated kind: weight 0, and evidence 1 on a deterministic
    /// kind (the only kinds with a fixed evidence; the others take [`EventDraft::caused_by_with`]).
    #[must_use]
    pub fn caused_by(self, cause: EventRef, kind: EdgeKind) -> Self {
        let evidence = if kind.is_deterministic() {
            EVIDENCE_ONE
        } else {
            0
        };
        self.caused_by_with(cause, kind, evidence)
    }

    /// Adds a causal edge of a non-motivated kind with an explicit evidence (`Q8_8` raw bits).
    #[must_use]
    pub fn caused_by_with(mut self, cause: EventRef, kind: EdgeKind, evidence: i16) -> Self {
        self.causes.push(CausalEdge {
            cause,
            kind,
            motive: None,
            weight: 0,
            evidence,
        });
        self
    }

    /// Adds a `motivated_by` edge naming `motive` (a `[motive_edges]` name), with its weight and
    /// evidence as `Q8_8` raw bits.
    #[must_use]
    pub fn motivated_by(
        mut self,
        cause: EventRef,
        motive: &'static str,
        weight: i16,
        evidence: i16,
    ) -> Self {
        self.causes.push(CausalEdge {
            cause,
            kind: EdgeKind::MotivatedBy,
            motive: Some(motive),
            weight,
            evidence,
        });
        self
    }

    /// Adds a `motivated_by` edge whose contributor is not a named motive (an opinion term, a
    /// trait term).
    #[must_use]
    pub fn motivated_by_unnamed(mut self, cause: EventRef, weight: i16, evidence: i16) -> Self {
        self.causes.push(CausalEdge {
            cause,
            kind: EdgeKind::MotivatedBy,
            motive: None,
            weight,
            evidence,
        });
        self
    }

    /// A `triggered` edge to an earlier draft of the same step.
    #[must_use]
    pub fn after(self, draft: DraftRef) -> Self {
        self.caused_by(EventRef::Draft(draft), EdgeKind::Triggered)
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
    /// `event.place_id`.
    pub place: Option<PlaceId>,
    /// `event.magnitude`.
    pub magnitude: i32,
    /// `event.secrecy`.
    pub secrecy: Secrecy,
    /// `event.origin`: the origin of the job that emitted it.
    pub origin: Origin,
    /// Whether the chronicle admits it (`05.2` §4.0; D38). Carried, never hashed: plane A never
    /// branches on it. Every P1a draft is admitted.
    pub admitted: bool,
    /// The cast, in canonical order (role, then entity).
    pub roles: Vec<RoleRow>,
    /// The snapshots, in canonical order.
    pub snapshots: Vec<Snapshot>,
    /// The causal edges, one per cause, by cause id ascending; all to smaller ids.
    pub causes: Vec<Edge>,
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
        /// The `(tick, phase)` of the refused append.
        at: (i64, u8),
        /// The `(tick, phase)` of the last event.
        last: (i64, u8),
    },
    /// A cause is still a [`EventRef::Draft`] (or [`EventRef::None`]): the merge resolves it first.
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
    /// A motive on an edge whose kind is not `motivated_by` (§5).
    MotiveOffMotivatedBy {
        /// The kind.
        kind: &'static str,
        /// The edge's cause.
        cause: EventId,
        /// The edge's kind.
        edge: EdgeKind,
        /// The motive.
        motive: &'static str,
    },
    /// A non-zero weight on an edge whose kind is not `motivated_by` (§5).
    WeightOffMotivatedBy {
        /// The kind.
        kind: &'static str,
        /// The edge's cause.
        cause: EventId,
        /// The edge's kind.
        edge: EdgeKind,
        /// The weight's raw bits.
        weight: i16,
    },
    /// An evidence other than 1 (256) on a deterministic kind, or outside [0, 1] (§5; R8).
    Evidence {
        /// The kind.
        kind: &'static str,
        /// The edge's cause.
        cause: EventId,
        /// The edge's kind.
        edge: EdgeKind,
        /// The evidence's raw bits.
        evidence: i16,
    },
    /// Two edges to one cause: the merge coalesces them first (§5; R9).
    DuplicateCause {
        /// The kind.
        kind: &'static str,
        /// The repeated cause.
        cause: EventId,
    },
    /// An edge of kind 6–8 (`player`, `oracle`, `director`), which has no writer in P1a (§5).
    NoWriter {
        /// The kind.
        kind: &'static str,
        /// The edge's cause.
        cause: EventId,
        /// The edge's kind.
        edge: EdgeKind,
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
                write!(f, "event {kind} still has an unresolved draft-local cause")
            }
            Self::ForwardCause { kind, cause, id } => write!(
                f,
                "event {kind} (id {id}) names cause {cause}, which is not an earlier event"
            ),
            Self::MotiveOffMotivatedBy {
                kind,
                cause,
                edge,
                motive,
            } => write!(
                f,
                "event {kind}: the {edge} edge to {cause} carries the motive {motive}; a motive is on motivated_by only"
            ),
            Self::WeightOffMotivatedBy {
                kind,
                cause,
                edge,
                weight,
            } => write!(
                f,
                "event {kind}: the {edge} edge to {cause} has weight {weight}; the weight is 0 off motivated_by"
            ),
            Self::Evidence {
                kind,
                cause,
                edge,
                evidence,
            } => write!(
                f,
                "event {kind}: the {edge} edge to {cause} has evidence {evidence}; it is 256 on a deterministic kind and in [0, 256] otherwise"
            ),
            Self::DuplicateCause { kind, cause } => write!(
                f,
                "event {kind} has two edges to cause {cause}; one edge per cause (coalesce first)"
            ),
            Self::NoWriter { kind, cause, edge } => write!(
                f,
                "event {kind}: the {edge} edge to {cause} is of an input kind, which has no writer in P1a"
            ),
        }
    }
}

impl std::error::Error for EventError {}

/// The edge rules of §5 that do not depend on the new id or on the other edges: a writer for the
/// kind, a motive and a weight only on `motivated_by`, evidence 1 on a deterministic kind and in
/// [0, 1] otherwise.
fn check_edge(kind: &'static str, e: &Edge) -> Result<(), EventError> {
    if !e.kind.has_writer() {
        return Err(EventError::NoWriter {
            kind,
            cause: e.cause,
            edge: e.kind,
        });
    }
    if e.kind != EdgeKind::MotivatedBy {
        if let Some(motive) = e.motive {
            return Err(EventError::MotiveOffMotivatedBy {
                kind,
                cause: e.cause,
                edge: e.kind,
                motive,
            });
        }
        if e.weight != 0 {
            return Err(EventError::WeightOffMotivatedBy {
                kind,
                cause: e.cause,
                edge: e.kind,
                weight: e.weight,
            });
        }
    }
    let evidence_ok = if e.kind.is_deterministic() {
        e.evidence == EVIDENCE_ONE
    } else {
        (0..=EVIDENCE_ONE).contains(&e.evidence)
    };
    if !evidence_ok {
        return Err(EventError::Evidence {
            kind,
            cause: e.cause,
            edge: e.kind,
            evidence: e.evidence,
        });
    }
    Ok(())
}

/// The event log of one timeline: the events in id order and the counter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventLog {
    events: Vec<Event>,
    next_seq: u64,
    last: Option<(i64, u8)>,
    /// The running fold of every appended event's digest, in id order (kept across drains).
    running: u64,
}

/// The serialisable state of an event log (SPR-007 T-02): the events not yet drained, the
/// counter, the last `(tick, phase)` and the running hash.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventLogState {
    /// The held (undrained) events, in id order: the last ids before the counter.
    pub events: Vec<Event>,
    /// The id the next event gets.
    pub next_seq: u64,
    /// The `(tick, phase)` of the last event appended, if any.
    pub last: Option<(i64, u8)>,
    /// The running fold behind [`EventLog::hash`].
    pub running: u64,
}

impl EventLog {
    /// The serialisable state.
    #[must_use]
    pub fn state(&self) -> EventLogState {
        EventLogState {
            events: self.events.clone(),
            next_seq: self.next_seq,
            last: self.last,
            running: self.running,
        }
    }

    /// Rebuilds a log from a state in the form [`EventLog::state`] produces.
    ///
    /// # Errors
    /// A message unless the held events have the ids just below the counter, in order; their
    /// `(tick, phase)` never decreases; the last one is at `last`; every cause is a smaller id;
    /// and an empty log (counter 0) has no `last` and a zero running hash.
    pub fn from_state(state: &EventLogState) -> Result<Self, String> {
        let held = state.events.len() as u64;
        if held > state.next_seq {
            return Err(format!(
                "{held} events held but the counter is {}",
                state.next_seq
            ));
        }
        for (k, e) in state.events.iter().enumerate() {
            let expected = state.next_seq - held + k as u64;
            if e.id.to_raw() != expected {
                return Err(format!("event {} where id {expected} is expected", e.id));
            }
            if let Some(c) = e.causes.iter().find(|c| c.cause >= e.id) {
                return Err(format!("event {} names the later cause {}", e.id, c.cause));
            }
            if e.causes.windows(2).any(|w| w[0].cause >= w[1].cause) {
                return Err(format!("event {}'s causes are not ascending", e.id));
            }
        }
        if let Some(w) = state
            .events
            .windows(2)
            .find(|w| (w[0].tick, w[0].phase) > (w[1].tick, w[1].phase))
        {
            return Err(format!(
                "event {} is back-dated against {}",
                w[1].id, w[0].id
            ));
        }
        if let Some(e) = state.events.last()
            && state.last != Some((e.tick, e.phase))
        {
            return Err(format!(
                "the last (tick, phase) is not that of event {}",
                e.id
            ));
        }
        if state.next_seq == 0 && (state.last.is_some() || state.running != 0) {
            return Err("an empty log has no last event and no running hash".to_owned());
        }
        if state.next_seq > 0 && state.last.is_none() {
            return Err("a log with events has a last (tick, phase)".to_owned());
        }
        Ok(Self {
            events: state.events.clone(),
            next_seq: state.next_seq,
            last: state.last,
            running: state.running,
        })
    }

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

    /// Appends a draft emitted by `emitter` at instant `tick` in phase `phase` by a job of
    /// `origin`, admitted by the chronicle, and returns its id.
    ///
    /// # Errors
    /// As [`EventLog::append_admitted`].
    pub fn append(
        &mut self,
        tick: i64,
        phase: u8,
        emitter: &'static str,
        origin: Origin,
        draft: EventDraft,
    ) -> Result<EventId, EventError> {
        self.append_admitted(tick, phase, emitter, origin, true, draft)
    }

    /// Appends a draft, as [`EventLog::append`], with the admission flag the merge computed
    /// (`05.2` §4.0; carried, never hashed).
    ///
    /// # Errors
    /// [`EventError::BackDated`] if `(tick, phase)` is before the last event's;
    /// [`EventError::Unresolved`] if a cause is not a known id; [`EventError::ForwardCause`] if a
    /// cause is not smaller than the new id; [`EventError::NoWriter`],
    /// [`EventError::MotiveOffMotivatedBy`], [`EventError::WeightOffMotivatedBy`] and
    /// [`EventError::Evidence`] for an edge that breaks §5; [`EventError::DuplicateCause`] for two
    /// edges to one cause. Nothing is appended and the counter does not move.
    pub fn append_admitted(
        &mut self,
        tick: i64,
        phase: u8,
        emitter: &'static str,
        origin: Origin,
        admitted: bool,
        draft: EventDraft,
    ) -> Result<EventId, EventError> {
        let kind = draft.kind;
        if let Some(last) = self.last
            && (tick, phase) < last
        {
            return Err(EventError::BackDated {
                kind,
                at: (tick, phase),
                last,
            });
        }
        let id = self.next_id();
        let mut causes = Vec::with_capacity(draft.causes.len());
        for c in &draft.causes {
            let EventRef::Known(cause) = c.cause else {
                return Err(EventError::Unresolved { kind });
            };
            causes.push(c.resolved(cause));
        }
        causes.sort_unstable_by_key(|e| e.cause);
        for e in &causes {
            if e.cause >= id {
                return Err(EventError::ForwardCause {
                    kind,
                    cause: e.cause,
                    id,
                });
            }
            check_edge(kind, e)?;
        }
        if let Some(w) = causes.windows(2).find(|w| w[0].cause == w[1].cause) {
            return Err(EventError::DuplicateCause {
                kind,
                cause: w[0].cause,
            });
        }
        let event = Event {
            id,
            tick,
            phase,
            emitter,
            kind,
            place: draft.place,
            magnitude: draft.magnitude,
            secrecy: draft.secrecy,
            origin,
            admitted,
            roles: draft.roles,
            snapshots: draft.snapshots,
            causes,
            payload: draft.payload,
        };
        self.running = rh_ids::rng::h64_words(TAG_EVENT_HASH, &[self.running, event.digest()]);
        self.events.push(event);
        self.next_seq += 1;
        self.last = Some((tick, phase));
        Ok(id)
    }

    /// The hash of every event ever appended, in id order, and of the counter: a running fold
    /// that [`EventLog::drain`] does not reset, so it is a function of the event sequence alone,
    /// whatever the drain cadence (SPR-006 G4). A stub until the chunked Merkle hashes of `05.1`
    /// §6 item 6 (SPR-007).
    #[must_use]
    pub fn hash(&self) -> u64 {
        rh_ids::rng::h64_words(TAG_EVENT_HASH, &[self.next_seq, self.running])
    }
}

/// `h(name)`: a Registry name, an emitter or a kind, hashed under the log's tag.
fn h_name(name: &str) -> u64 {
    rh_ids::rng::h64_bytes(TAG_EVENT_HASH, name.as_bytes())
}

/// A signed value sign-extended to a word.
fn sx(v: i64) -> u64 {
    v.cast_unsigned()
}

impl Event {
    /// A digest of every hashed field of the event, version 2 (`envelope_v0.md` §6): what the
    /// running log hash folds in. Words, in order:
    ///
    /// ```text
    /// id, tick, phase, h(emitter), h(kind),
    /// has_place (0|1), place (0 when absent), magnitude (i32 sign-extended), secrecy, origin,
    /// n_roles,     then per role row:  role, entity_kind_tag, entity_raw
    /// n_snapshots, then per snapshot:  h(attr name), entity_kind_tag, entity_raw, has_partner (0|1),
    ///                                  partner_kind_tag, partner_raw (0, 0 when absent), value (i32 sign-extended)
    /// n_causes,    then per edge:      cause_id, kind, has_motive (0|1), h(motive name) (0 when absent),
    ///                                  weight (i16 sign-extended), evidence
    /// n_payload,   then the payload words
    /// ```
    ///
    /// The admission flag is not hashed (`05.2` §4.0).
    #[must_use]
    pub fn digest(&self) -> u64 {
        // Sized exactly, so the digest never regrows its buffer (V-PERF-11).
        let n = 14
            + 3 * self.roles.len()
            + 7 * self.snapshots.len()
            + 6 * self.causes.len()
            + self.payload.len();
        let mut words = Vec::with_capacity(n);
        words.extend([
            self.id.to_raw(),
            sx(self.tick),
            u64::from(self.phase),
            h_name(self.emitter),
            h_name(self.kind),
            u64::from(self.place.is_some()),
            self.place.map_or(0, |p| u64::from(p.to_raw())),
            sx(i64::from(self.magnitude)),
            u64::from(self.secrecy.code()),
            u64::from(self.origin.code()),
            self.roles.len() as u64,
        ]);
        for r in &self.roles {
            words.extend([
                u64::from(r.role.0),
                u64::from(r.entity.kind().tag()),
                r.entity.raw(),
            ]);
        }
        words.push(self.snapshots.len() as u64);
        for s in &self.snapshots {
            words.extend([
                h_name(s.attr),
                u64::from(s.entity.kind().tag()),
                s.entity.raw(),
                u64::from(s.partner.is_some()),
                s.partner.map_or(0, |p| u64::from(p.kind().tag())),
                s.partner.map_or(0, EntityRef::raw),
                sx(i64::from(s.value)),
            ]);
        }
        words.push(self.causes.len() as u64);
        for e in &self.causes {
            words.extend([
                e.cause.to_raw(),
                u64::from(e.kind.code()),
                u64::from(e.motive.is_some()),
                e.motive.map_or(0, h_name),
                sx(i64::from(e.weight)),
                sx(i64::from(e.evidence)),
            ]);
        }
        words.push(self.payload.len() as u64);
        words.extend(self.payload.iter().map(|v| v.cast_unsigned()));
        rh_ids::rng::h64_words(TAG_EVENT_HASH, &words)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DraftRef, Edge, EdgeKind, EventDraft, EventError, EventLog, EventRef, Origin, coalesce,
    };
    use rh_ids::{EntityRef, EventId, PersonId};

    const SIM: Origin = Origin::Sim;

    fn known(raw: u64) -> EventRef {
        EventRef::Known(EventId::from_raw(raw))
    }

    #[test]
    fn ids_follow_append_order_and_back_dating_is_refused() {
        let mut log = EventLog::new();
        let a = log.append(10, 4, "M2.demography", SIM, EventDraft::new("DEATH"));
        let b = log.append(10, 4, "M2.demography", SIM, EventDraft::new("DEATH"));
        let c = log.append(10, 9, "S7.vows", SIM, EventDraft::new("VOW_VOIDED"));
        assert_eq!(
            (a, b, c),
            (
                Ok(EventId::from_raw(0)),
                Ok(EventId::from_raw(1)),
                Ok(EventId::from_raw(2))
            )
        );
        let early = log.append(10, 4, "M2.demography", SIM, EventDraft::new("DEATH"));
        assert!(matches!(early, Err(EventError::BackDated { .. })));
        let past = log.append(9, 11, "M2.demography", SIM, EventDraft::new("DEATH"));
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
            .append(0, 4, "M2.demography", SIM, EventDraft::new("DEATH"))
            .unwrap_or_else(|e| panic!("{e}"));
        let ok = log.append(
            0,
            9,
            "S7.vows",
            SIM,
            EventDraft::new("VOW_VOIDED")
                .caused_by(EventRef::Known(d), EdgeKind::Triggered)
                .role(
                    super::RoleId::ACTOR,
                    EntityRef::person(PersonId::from_raw(7)),
                ),
        );
        assert_eq!(ok, Ok(EventId::from_raw(1)));
        let self_cause = log.append(
            0,
            9,
            "S7.vows",
            SIM,
            EventDraft::new("VOW_VOIDED").caused_by(known(2), EdgeKind::Triggered),
        );
        assert!(matches!(self_cause, Err(EventError::ForwardCause { .. })));
        let local = log.append(
            0,
            9,
            "S7.vows",
            SIM,
            EventDraft::new("VOW_VOIDED").after(DraftRef::from_index(0)),
        );
        assert!(matches!(local, Err(EventError::Unresolved { .. })));
        let none = log.append(
            0,
            9,
            "S7.vows",
            SIM,
            EventDraft::new("VOW_VOIDED").caused_by(EventRef::None, EdgeKind::Triggered),
        );
        assert!(matches!(none, Err(EventError::Unresolved { .. })));
    }

    /// SPR-020 T-02: each `append` refusal of §5, planted, is refused by its named error, and a
    /// refusal touches nothing (the counter, the last instant, the hash and the held events).
    #[test]
    fn every_edge_refusal_is_named_and_touches_nothing() {
        let mut log = EventLog::new();
        for _ in 0..3 {
            log.append(0, 1, "X", SIM, EventDraft::new("A"))
                .unwrap_or_else(|e| panic!("{e}"));
        }
        let before = log.clone();
        let d = || EventDraft::new("B");
        let mut motive_on_triggered = d().caused_by(known(0), EdgeKind::Triggered);
        motive_on_triggered.causes[0].motive = Some("GRIEF_FOR");
        let mut weight_on_triggered = d().caused_by(known(0), EdgeKind::Triggered);
        weight_on_triggered.causes[0].weight = 5;
        let cases: Vec<(EventDraft, &str)> = vec![
            (d().after(DraftRef::from_index(0)), "Unresolved"),
            (d().caused_by(known(3), EdgeKind::Triggered), "ForwardCause"),
            (motive_on_triggered, "MotiveOffMotivatedBy"),
            (weight_on_triggered, "WeightOffMotivatedBy"),
            (
                d().caused_by_with(known(0), EdgeKind::Triggered, 255),
                "Evidence",
            ),
            (
                d().caused_by_with(known(0), EdgeKind::Contingent, 0),
                "Evidence",
            ),
            (
                d().caused_by_with(known(0), EdgeKind::Enabled, 257),
                "Evidence",
            ),
            (d().motivated_by(known(0), "GRIEF_FOR", 10, -1), "Evidence"),
            (
                d().caused_by(known(0), EdgeKind::Triggered)
                    .caused_by(known(0), EdgeKind::Contingent),
                "DuplicateCause",
            ),
            (d().caused_by(known(0), EdgeKind::Player), "NoWriter"),
            (d().caused_by(known(0), EdgeKind::Oracle), "NoWriter"),
            (d().caused_by(known(0), EdgeKind::Director), "NoWriter"),
        ];
        for (draft, name) in cases {
            let got = log.append(1, 1, "X", SIM, draft);
            let err = got.err().unwrap_or_else(|| panic!("{name}: accepted"));
            assert!(format!("{err:?}").starts_with(name), "{name}: got {err:?}");
            assert_eq!(log, before, "{name}: a refused append touched the log");
        }
        let ok = log.append(
            1,
            1,
            "X",
            SIM,
            d().caused_by(known(2), EdgeKind::Triggered)
                .motivated_by(known(0), "GRIEF_FOR", -7, 0)
                .caused_by_with(known(1), EdgeKind::Probabilistic, 0),
        );
        assert_eq!(ok, Ok(EventId::from_raw(3)));
        let cause_ids: Vec<u64> = log.events()[3]
            .causes
            .iter()
            .map(|e| e.cause.to_raw())
            .collect();
        assert_eq!(cause_ids, [0, 1, 2], "edges are sorted by cause id");
    }

    #[test]
    fn drain_keeps_the_counter_and_the_order_floor() {
        let mut log = EventLog::new();
        let h0 = log.hash();
        log.append(5, 1, "X", SIM, EventDraft::new("A").with_payload(vec![1]))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_ne!(log.hash(), h0);
        let h1 = log.hash();
        assert_eq!(log.drain().len(), 1);
        assert!(log.events().is_empty());
        assert_eq!(log.hash(), h1, "draining does not change the hash");
        assert_eq!(log.next_id(), EventId::from_raw(1));
        assert!(log.append(4, 1, "X", SIM, EventDraft::new("A")).is_err());
    }

    #[test]
    fn the_hash_ignores_the_drain_cadence_and_the_admission_flag() {
        let fill = |drain_every: Option<u64>, admit_odd: bool| -> u64 {
            let mut log = EventLog::new();
            for i in 0..240_i64 {
                log.append_admitted(
                    i / 4,
                    u8::try_from(i % 4).unwrap_or(0),
                    "X",
                    SIM,
                    !admit_odd || i % 2 == 0,
                    EventDraft::new("A").with_payload(vec![i]),
                )
                .unwrap_or_else(|e| panic!("{e}"));
                if drain_every.is_some_and(|n| (i + 1).cast_unsigned() % n == 0) {
                    log.drain();
                }
            }
            log.hash()
        };
        let never = fill(None, false);
        assert_eq!(fill(Some(20), false), never, "monthly");
        assert_eq!(fill(Some(240), false), never, "yearly");
        assert_eq!(fill(Some(1), false), never, "every event");
        assert_eq!(fill(None, true), never, "the admission flag is not hashed");
    }

    /// The R20 known answer (envelope §5 as amended): `GRIEF_FOR` +100, `FEAR_OF` +100 and
    /// `RETALIATION_FOR` −150 to one cause give weight +50 and motive `FEAR_OF` (index 4 beats
    /// `GRIEF_FOR`'s 6). The old rule (largest |weight|) would have named `RETALIATION_FOR`.
    #[test]
    fn coalescing_known_answers() {
        const MOTIVES: [&str; 9] = [
            "RETALIATION_FOR",
            "JEALOUSY_OF",
            "VOW_OF",
            "AMBITION_FOR",
            "FEAR_OF",
            "LOYALTY_TO",
            "GRIEF_FOR",
            "GREED_FOR",
            "CORRUPTION_BY",
        ];
        let idx = |m: &str| MOTIVES.iter().position(|x| *x == m);
        let c = EventId::from_raw(4);
        let m = |motive: Option<&'static str>, weight: i16, evidence: i16| Edge {
            cause: c,
            kind: EdgeKind::MotivatedBy,
            motive,
            weight,
            evidence,
        };
        let one = |edges: Vec<Edge>| {
            let out = coalesce("X", edges, idx).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(out.len(), 1);
            out[0]
        };
        let r20 = one(vec![
            m(Some("GRIEF_FOR"), 100, 10),
            m(Some("FEAR_OF"), 100, 20),
            m(Some("RETALIATION_FOR"), -150, 30),
        ]);
        assert_eq!(
            (r20.kind, r20.motive, r20.weight, r20.evidence),
            (EdgeKind::MotivatedBy, Some("FEAR_OF"), 50, 30)
        );
        // R21: a contingent reference and a RETALIATION_FOR motive to one harm keep motivated_by
        // with its motive, weight and margin.
        let r21 = one(vec![
            Edge {
                cause: c,
                kind: EdgeKind::Contingent,
                motive: None,
                weight: 0,
                evidence: 256,
            },
            m(Some("RETALIATION_FOR"), 90, 40),
        ]);
        assert_eq!(
            (r21.kind, r21.motive, r21.weight, r21.evidence),
            (EdgeKind::MotivatedBy, Some("RETALIATION_FOR"), 90, 40)
        );
        // Saturated once, in any order: 32767 + 32767 - 32768 is 32766 exactly (a step-by-step
        // saturation gives -1 in this order), and 32767 + 32767 + 5 saturates to i16::MAX.
        for order in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
            for (w, want) in [
                ([32767_i16, 32767, -32768], 32766),
                ([32767, 32767, 5], i16::MAX),
            ] {
                let e = one(order
                    .iter()
                    .map(|&k| m(Some("GREED_FOR"), w[k], 0))
                    .collect());
                assert_eq!(e.weight, want, "{w:?} in order {order:?}");
            }
        }
        assert_eq!(
            one(vec![
                m(Some("GREED_FOR"), -32768, 0),
                m(Some("FEAR_OF"), -1, 0)
            ])
            .weight,
            i16::MIN
        );
        // A zero sum names nothing; a negative one the most negative; an unnamed winner none.
        assert_eq!(
            one(vec![m(Some("GRIEF_FOR"), 5, 0), m(Some("FEAR_OF"), -5, 0)]).motive,
            None
        );
        assert_eq!(
            one(vec![m(Some("GRIEF_FOR"), -9, 0), m(Some("FEAR_OF"), 2, 0)]).motive,
            Some("GRIEF_FOR")
        );
        assert_eq!(
            one(vec![m(None, 9, 0), m(Some("FEAR_OF"), 2, 0)]).motive,
            None
        );
        assert_eq!(
            one(vec![m(None, 9, 0), m(Some("CORRUPTION_BY"), 9, 0)]).motive,
            Some("CORRUPTION_BY"),
            "an unnamed contributor counts as the highest index"
        );
    }

    /// Mixed deterministic kinds keep the precedence's first; a bad contributor is refused.
    #[test]
    fn coalescing_mixed_kinds_and_bad_contributors() {
        let idx = |m: &str| ["FEAR_OF"].iter().position(|x| *x == m);
        let c = EventId::from_raw(4);
        let m = |motive: Option<&'static str>, weight: i16, evidence: i16| Edge {
            cause: c,
            kind: EdgeKind::MotivatedBy,
            motive,
            weight,
            evidence,
        };
        let one = |edges: Vec<Edge>| {
            let out = coalesce("X", edges, idx).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(out.len(), 1);
            out[0]
        };
        // Triggered and enabled to one cause: triggered, evidence 1.
        let te = one(vec![
            Edge {
                cause: c,
                kind: EdgeKind::Enabled,
                motive: None,
                weight: 0,
                evidence: 17,
            },
            Edge {
                cause: c,
                kind: EdgeKind::Triggered,
                motive: None,
                weight: 0,
                evidence: 256,
            },
        ]);
        assert_eq!((te.kind, te.evidence), (EdgeKind::Triggered, 256));
        // A contributor that breaks an edge rule is refused, not absorbed.
        let bad = Edge {
            cause: c,
            kind: EdgeKind::Triggered,
            motive: None,
            weight: 3,
            evidence: 256,
        };
        assert!(matches!(
            coalesce("X", vec![bad, m(Some("FEAR_OF"), 1, 0)], idx),
            Err(EventError::WeightOffMotivatedBy { .. })
        ));
    }

    /// SPR-007 T-02: the state round-trips (before and after a drain), and only the form
    /// `state()` produces restores.
    #[test]
    fn state_round_trips_and_only_the_canonical_form_restores() {
        use super::EventLogState;
        let mut log = EventLog::new();
        assert_eq!(EventLog::from_state(&log.state()), Ok(log.clone()));
        for i in 0..6_i64 {
            let d = EventDraft::new("A").with_payload(vec![i]);
            let d = if i > 0 {
                d.caused_by(known(0), EdgeKind::Triggered)
            } else {
                d
            };
            log.append(i / 2, u8::try_from(i % 2).unwrap_or(0), "X", SIM, d)
                .unwrap_or_else(|e| panic!("{e}"));
        }
        let state = log.state();
        assert_eq!(EventLog::from_state(&state), Ok(log.clone()));
        let mut drained = log.clone();
        drained.drain();
        assert_eq!(EventLog::from_state(&drained.state()), Ok(drained.clone()));
        let refused = |f: fn(&mut EventLogState)| {
            let mut s = state.clone();
            f(&mut s);
            EventLog::from_state(&s).is_err()
        };
        assert!(
            refused(|s| s.next_seq += 1),
            "ids not just below the counter"
        );
        assert!(refused(|s| s.events.swap(1, 2)), "ids out of order");
        assert!(
            refused(|s| {
                s.events.remove(2);
            }),
            "a gap"
        );
        assert!(
            refused(|s| s.last = Some((0, 0))),
            "last is not the last event's"
        );
        assert!(refused(|s| s.events[4].tick = -1), "back-dated");
        assert!(
            refused(|s| s.events[1].causes[0].cause = EventId::from_raw(3)),
            "a later cause"
        );
        assert!(
            refused(|s| {
                let e = s.events[3].causes[0];
                s.events[3].causes.push(e);
            }),
            "a repeated cause"
        );
        assert!(refused(|s| s.last = None), "events but no last");
        let empty = EventLogState {
            running: 1,
            ..EventLogState::default()
        };
        assert!(
            EventLog::from_state(&empty).is_err(),
            "an empty log with a running hash"
        );
    }
}
