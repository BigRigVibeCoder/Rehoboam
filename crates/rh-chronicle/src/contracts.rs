//! Interface contracts, event envelope definitions, and bounded ring buffer chronicle storage (SPEC-004).
//!
//! Plane A, machinery. Defines the `ColonyEvent` envelope schema, the `ChronicleWriter` trait,
//! and the `BoundedChronicleWriter` ring buffer for bounded history storage with zero heap allocation
//! in the simulation loop.
//!
//! REF: `specs/chronicle/envelope_v0.md`; `CODEX/05_PROJECT/SPR-020_EventEnvelope.md`;
//! `.spec/drafts/SPEC-004_chronicle_writer.md`.

use core::fmt;

use rh_events::{EdgeKind, Origin, Secrecy};
use rh_ids::{EventId, PersonId, PlaceId};

/// A fixed-size payload for colony events, guaranteeing zero heap allocation in simulation loop.
pub type EventPayload = [i64; 4];

/// Cinematic event envelope schema representing an event recorded in the colony chronicle.
///
/// Contains structured timestamps (tick, phase, year, day), causality markers (`event_id`, `cause_id`, `edge_kind`),
/// agent attribution (actor, target, origin, secrecy), and payload.
///
/// Designed as a compact, `Copy` value type with zero heap allocations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColonyEvent {
    /// Canonical event identifier assigned in emission order.
    pub event_id: EventId,
    /// Absolute simulation tick at which the event occurred.
    pub tick: u64,
    /// Scheduler phase index within the instant.
    pub phase: u8,
    /// Calendar year of the event.
    pub year: u32,
    /// Day of the year (0..=364).
    pub day: u16,
    /// Inciting causal event identifier, if caused by an earlier event.
    pub cause_id: Option<EventId>,
    /// Semantic nature of the causal link to `cause_id`.
    pub edge_kind: EdgeKind,
    /// Primary actor initiating or responsible for the event.
    pub actor: Option<PersonId>,
    /// Target entity acted upon or affected by the event.
    pub target: Option<PersonId>,
    /// Spatial location of the event, if bound to a place.
    pub place: Option<PlaceId>,
    /// Causal origin of the event (Simulation, Player, Oracle, Director).
    pub origin: Origin,
    /// Visibility and secrecy tier of the event.
    pub secrecy: Secrecy,
    /// Semantic event kind / verb code (e.g., "BIRTH", "DEATH", "`MENTAL_BREAK`").
    pub kind: &'static str,
    /// Scalar magnitude of the event (0 if unscaled).
    pub magnitude: i32,
    /// Integer payload values specific to the event kind.
    pub payload: EventPayload,
}

impl ColonyEvent {
    /// Creates a new `ColonyEvent` envelope with default attribution and no inciting cause.
    #[must_use]
    pub const fn new(
        event_id: EventId,
        tick: u64,
        phase: u8,
        year: u32,
        day: u16,
        kind: &'static str,
    ) -> Self {
        Self {
            event_id,
            tick,
            phase,
            year,
            day,
            cause_id: None,
            edge_kind: EdgeKind::Triggered,
            actor: None,
            target: None,
            place: None,
            origin: Origin::Sim,
            secrecy: Secrecy::Public,
            kind,
            magnitude: 0,
            payload: [0; 4],
        }
    }

    /// Sets the causal provenance edge (`cause_id` and `edge_kind`).
    #[must_use]
    pub const fn with_cause(mut self, cause_id: EventId, edge_kind: EdgeKind) -> Self {
        self.cause_id = Some(cause_id);
        self.edge_kind = edge_kind;
        self
    }

    /// Sets the actor responsible for this event.
    #[must_use]
    pub const fn with_actor(mut self, actor: PersonId) -> Self {
        self.actor = Some(actor);
        self
    }

    /// Sets the target entity acted upon.
    #[must_use]
    pub const fn with_target(mut self, target: PersonId) -> Self {
        self.target = Some(target);
        self
    }

    /// Sets the spatial location of the event.
    #[must_use]
    pub const fn with_place(mut self, place: PlaceId) -> Self {
        self.place = Some(place);
        self
    }

    /// Sets the origin and secrecy tier.
    #[must_use]
    pub const fn with_attribution(mut self, origin: Origin, secrecy: Secrecy) -> Self {
        self.origin = origin;
        self.secrecy = secrecy;
        self
    }

    /// Sets the scalar magnitude and payload.
    #[must_use]
    pub const fn with_payload(mut self, magnitude: i32, payload: EventPayload) -> Self {
        self.magnitude = magnitude;
        self.payload = payload;
        self
    }
}

impl Default for ColonyEvent {
    fn default() -> Self {
        Self {
            event_id: EventId::from_raw(0),
            tick: 0,
            phase: 0,
            year: 0,
            day: 0,
            cause_id: None,
            edge_kind: EdgeKind::Triggered,
            actor: None,
            target: None,
            place: None,
            origin: Origin::Sim,
            secrecy: Secrecy::Public,
            kind: "",
            magnitude: 0,
            payload: [0; 4],
        }
    }
}

/// Errors returned by chronicle writer operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChronicleError {
    /// Requested query limit exceeds writer capacity.
    InvalidLimit {
        /// Number of recent events requested.
        requested: usize,
        /// Maximum capacity of the chronicle ring buffer.
        capacity: usize,
    },
    /// Event timestamp is backdated relative to previously recorded events.
    InvalidTimestamp {
        /// Tick of the attempted event.
        event_tick: u64,
        /// Tick of the most recently recorded event.
        last_tick: u64,
    },
    /// Causal provenance violation: inciting cause does not strictly precede this event.
    CausalityViolation {
        /// Identifier of this event.
        event_id: EventId,
        /// Identifier of the referenced cause.
        cause_id: EventId,
    },
    /// Chronicle initialized with zero capacity.
    CapacityZero,
    /// Ring buffer overflow in non-overwriting mode.
    BufferOverflow,
}

impl fmt::Display for ChronicleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit { requested, capacity } => {
                write!(
                    f,
                    "chronicle query limit {requested} exceeds capacity {capacity}"
                )
            }
            Self::InvalidTimestamp {
                event_tick,
                last_tick,
            } => {
                write!(
                    f,
                    "chronicle event tick {event_tick} backdated after {last_tick}"
                )
            }
            Self::CausalityViolation {
                event_id,
                cause_id,
            } => {
                write!(
                    f,
                    "chronicle causality violation: cause {cause_id:?} does not precede event {event_id:?}"
                )
            }
            Self::CapacityZero => write!(f, "chronicle capacity must be greater than zero"),
            Self::BufferOverflow => write!(f, "chronicle buffer overflow in strict mode"),
        }
    }
}

impl core::error::Error for ChronicleError {}

/// Abstract contract for recording colony events and querying recent history.
pub trait ChronicleWriter {
    /// Appends an event envelope into the chronicle history.
    ///
    /// # Errors
    /// Returns `ChronicleError::InvalidTimestamp` if the event is backdated.
    /// Returns `ChronicleError::CausalityViolation` if `event.cause_id >= event.event_id`.
    fn record_event(&mut self, event: &ColonyEvent) -> Result<(), ChronicleError>;

    /// Returns a contiguous slice of the most recent events, ordered from oldest to newest.
    ///
    /// If fewer events have been recorded than `limit`, returns all recorded events.
    #[must_use]
    fn query_recent(&self, limit: usize) -> &[ColonyEvent];
}

/// A bounded ring buffer chronicle writer with zero heap allocations during simulation recording.
///
/// Uses a mirrored internal buffer of `2 * capacity` elements. When writing at index `i`,
/// both `buffer[i]` and `buffer[i + capacity]` are updated. This guarantees that any sub-window
/// of up to `capacity` consecutive elements is available as a single contiguous slice `&[ColonyEvent]`,
/// completely eliminating heap allocations and copying overhead on queries.
#[derive(Debug, Clone)]
pub struct BoundedChronicleWriter {
    buffer: Vec<ColonyEvent>,
    capacity: usize,
    write_pos: usize,
    count: usize,
    total_recorded: u64,
    last_tick: u64,
}

impl BoundedChronicleWriter {
    /// Pre-allocates a bounded chronicle writer with the specified maximum capacity.
    ///
    /// Allocates memory once at creation. Subsequent calls to `record_event` and `query_recent`
    /// perform zero heap allocations.
    ///
    /// # Errors
    /// Returns `ChronicleError::CapacityZero` if `capacity == 0`.
    pub fn new(capacity: usize) -> Result<Self, ChronicleError> {
        if capacity == 0 {
            return Err(ChronicleError::CapacityZero);
        }

        let total_slots = capacity.saturating_mul(2);
        let mut buffer = Vec::with_capacity(total_slots);
        buffer.resize(total_slots, ColonyEvent::default());

        Ok(Self {
            buffer,
            capacity,
            write_pos: 0,
            count: 0,
            total_recorded: 0,
            last_tick: 0,
        })
    }

    /// Returns the maximum event capacity of this chronicle writer.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the current number of events stored in the buffer (`<= capacity`).
    #[must_use]
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Returns true if no events have been recorded yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns the cumulative number of events recorded over the lifetime of the writer.
    #[must_use]
    pub const fn total_recorded(&self) -> u64 {
        self.total_recorded
    }
}

impl ChronicleWriter for BoundedChronicleWriter {
    fn record_event(&mut self, event: &ColonyEvent) -> Result<(), ChronicleError> {
        // Enforce monotonicity of simulation tick
        if self.total_recorded > 0 && event.tick < self.last_tick {
            return Err(ChronicleError::InvalidTimestamp {
                event_tick: event.tick,
                last_tick: self.last_tick,
            });
        }

        // Enforce causality invariant: cause_id must strictly precede event_id
        if let Some(cause) = event.cause_id
            && cause.to_raw() >= event.event_id.to_raw()
        {
            return Err(ChronicleError::CausalityViolation {
                event_id: event.event_id,
                cause_id: cause,
            });
        }

        let pos = self.write_pos;
        let mirror_pos = pos + self.capacity;

        // Mirrored write for contiguous zero-copy query slices
        self.buffer[pos] = *event;
        self.buffer[mirror_pos] = *event;

        self.write_pos = (pos + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
        self.total_recorded += 1;
        self.last_tick = event.tick;

        Ok(())
    }

    fn query_recent(&self, limit: usize) -> &[ColonyEvent] {
        let n = limit.min(self.count);
        if n == 0 {
            return &[];
        }

        // Calculate end index in mirrored space
        let end_idx = if self.write_pos == 0 {
            (2 * self.capacity) - 1
        } else {
            (self.write_pos + self.capacity) - 1
        };

        let start_idx = (end_idx + 1) - n;
        &self.buffer[start_idx..=end_idx]
    }
}

/// Type alias for `BoundedChronicleWriter` representing ring buffer chronicle storage.
pub type RingBufferChronicle = BoundedChronicleWriter;
