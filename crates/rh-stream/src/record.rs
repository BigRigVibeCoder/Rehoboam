//! The World Stream v0 records (`specs/stream/channel_a_v0.sql`, `channel_c_v0.sql`): the header
//! every record carries (R3), the six channel-A kinds and channel C's `MATCH`, and the order key
//! of an instant (R4).
//!
//! The field types are the DDL's: unsigned columns are unsigned here, an optional column is an
//! `Option`, and a child table (`event_role`, `causal_edge`, `sift_binding`, `sift_match_event`)
//! is a list on its record, so one record is one canonical line (`canonical_v0.md` §3). Nothing
//! is a float (R2).
//!
//! REF: `specs/stream/channel_a_v0.sql` R3–R5; `specs/stream/channel_c_v0.sql` R3–R6;
//! SPR-011 rulings 11–13 and 17.

/// The record header (R3): timeline, position in the timeline's channel, and instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Header {
    /// 0 = canonical; forks have their own ids.
    pub timeline_id: u32,
    /// The record's position in its timeline's channel, contiguous from 0 across segments.
    pub seq: u64,
    /// The instant (`SimTime` minutes).
    pub tick: i64,
}

/// A stream channel (`05.10` §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Channel {
    /// Channel A: the world (plane A).
    A,
    /// Channel C: narrative (plane C).
    C,
}

impl Channel {
    /// Its letter, as the directory and `segment.channel` spell it.
    #[must_use]
    pub const fn letter(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::C => "C",
        }
    }
}

/// A record kind: its code (the `record_kind` dictionary of its channel), name, R4 class rank
/// and channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// `INPUT` (1): a journaled push input at its applied tick.
    Input,
    /// `EVENT` (2): an admitted event (every draft in P1.0, `05.2` §4.0).
    Event,
    /// `LIFE_ENTRY` (3): a person enters the world.
    LifeEntry,
    /// `LIFE_EXIT` (4): a person leaves the world.
    LifeExit,
    /// `STEP_DIGEST` (5): once per Month pulse, after `Bookkeeping`.
    StepDigest,
    /// `KEYFRAME` (6): a pointer to a full snapshot.
    Keyframe,
    /// `MATCH` (channel C, 1): a completed pattern instance.
    Match,
}

impl Kind {
    /// The channel-A kinds, in code order.
    pub const CHANNEL_A: [Self; 6] = [
        Self::Input,
        Self::Event,
        Self::LifeEntry,
        Self::LifeExit,
        Self::StepDigest,
        Self::Keyframe,
    ];

    /// The kind code (`record_kind.kind`; channel C has its own code space).
    #[must_use]
    pub const fn code(self) -> u16 {
        match self {
            Self::Input | Self::Match => 1,
            Self::Event => 2,
            Self::LifeEntry => 3,
            Self::LifeExit => 4,
            Self::StepDigest => 5,
            Self::Keyframe => 6,
        }
    }

    /// The kind name (`record_kind.name`, the canonical line's first token).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Input => "INPUT",
            Self::Event => "EVENT",
            Self::LifeEntry => "LIFE_ENTRY",
            Self::LifeExit => "LIFE_EXIT",
            Self::StepDigest => "STEP_DIGEST",
            Self::Keyframe => "KEYFRAME",
            Self::Match => "MATCH",
        }
    }

    /// The R4 class rank (`record_kind.class`): `INPUT` 0, `EVENT` 1, LIFE 2, `STEP_DIGEST` 3,
    /// KEYFRAME 4; MATCH 0 in channel C.
    #[must_use]
    pub const fn class(self) -> u8 {
        match self {
            Self::Input | Self::Match => 0,
            Self::Event => 1,
            Self::LifeEntry | Self::LifeExit => 2,
            Self::StepDigest => 3,
            Self::Keyframe => 4,
        }
    }

    /// The channel the kind belongs to.
    #[must_use]
    pub const fn channel(self) -> Channel {
        match self {
            Self::Match => Channel::C,
            _ => Channel::A,
        }
    }

    /// The kind of a name, if it is one.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        [
            Self::Input,
            Self::Event,
            Self::LifeEntry,
            Self::LifeExit,
            Self::StepDigest,
            Self::Keyframe,
            Self::Match,
        ]
        .into_iter()
        .find(|k| k.name() == name)
    }
}

/// `INPUT`: a journal row at its applied tick (`rh-snapshot` journal, IF-2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    /// The journal row's `seq`.
    pub journal_seq: u64,
    /// 0 push, the only class (ADR-0034).
    pub class: u8,
    /// The journal row's kind.
    pub input_kind: String,
    /// The journal row's scope name (`person`, `household`, …, `none`).
    pub scope: String,
    /// The scope's entity id, `None` for no scope.
    pub scope_id: Option<u64>,
    /// The journal row's producer.
    pub producer: String,
    /// The payload hash (16 bytes, XXH3-128 canonical form).
    pub payload_hash: [u8; 16],
    /// The journal row's digest: links the stream to the journal.
    pub row_digest: [u8; 16],
}

/// One `event_role` child row, its ordinal being its position in [`Event::roles`]: the cast is
/// ordered by `role`, then `(entity_kind, entity_id)` (`envelope_v0.md` §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Role {
    /// `rh-ids` `EntityKind`: 0 world, 1 person, 2 household, 3 settlement, 4 region; 5 artifact
    /// (reserved for S6).
    pub entity_kind: u8,
    /// The entity's raw id.
    pub entity_id: u64,
    /// The Registry `[event_roles]` index: 0 actor, 1 target, 2 witness, 3 instrument.
    pub role: u8,
}

impl Role {
    /// The ordinal order's key: role code, then entity.
    #[must_use]
    pub const fn order_key(&self) -> (u8, u8, u64) {
        (self.role, self.entity_kind, self.entity_id)
    }
}

/// One `event_snapshot` child row: an anchored attribute value (`envelope_v0.md` §4). Field order
/// is the key order (a `None` partner before any partner), then the value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Snapshot {
    /// The attribute's index in the Registry's `[attributes]` sorted by name.
    pub attr_id: u16,
    /// The first argument's entity kind (0 with id 0: the event itself).
    pub entity_kind: u8,
    /// The first argument's raw id.
    pub entity_id: u64,
    /// The partner's entity kind, for a relational attribute (both set or both `None`).
    pub partner_kind: Option<u8>,
    /// The partner's raw id.
    pub partner_id: Option<u64>,
    /// `q8_8` raw bits, bool 0/1, enum index, or an int.
    pub value: i32,
}

impl Snapshot {
    /// The row's key (`event_snapshot`: one row per key).
    #[must_use]
    pub const fn key(&self) -> (u16, u8, u64, Option<u8>, Option<u64>) {
        (
            self.attr_id,
            self.entity_kind,
            self.entity_id,
            self.partner_kind,
            self.partner_id,
        )
    }
}

/// One `causal_edge` child row of an event (the effect), typed (`envelope_v0.md` §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cause {
    /// The cause's event id (smaller than the effect's).
    pub cause_id: u64,
    /// `schema.sql`'s edge kind, 0-5 written (6-8 have no writer in P1a); SPR-006's untyped causes
    /// are 1 (triggered).
    pub kind: u8,
    /// The `[motive_edges]` index, set only on kind 3 (`motivated_by`).
    pub motive: Option<u16>,
    /// `q8_8` raw bits, 0 unless kind 3.
    pub weight: i16,
    /// `q8_8` raw bits in [0, 256]; 256 on a deterministic kind.
    pub evidence: i16,
    /// The cause has an `EVENT` record (true on every P1a edge).
    pub cause_told: bool,
}

impl Cause {
    /// A deterministic `triggered` edge, told: what P1a's untyped causes are.
    #[must_use]
    pub const fn triggered(cause_id: u64) -> Self {
        Self {
            cause_id,
            kind: 1,
            motive: None,
            weight: 0,
            evidence: 256,
            cause_told: true,
        }
    }
}

/// `EVENT` (`schema.sql` event, plus the header and the payload).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    /// The timeline's event counter (ruling 12).
    pub event_id: u64,
    /// The `05.1` §4.2 phase index of the emission.
    pub phase: u8,
    /// The verb's index in the Registry's `[verbs]` sorted by name (ruling 11).
    pub verb_id: u16,
    /// The verb's name.
    pub verb: String,
    /// The verb's act classes, bit i = `[act_classes].ids[i]`.
    pub act_classes: u64,
    /// `None` until places exist (P1.1a).
    pub place_id: Option<u32>,
    /// 0 in P1.0.
    pub magnitude: i32,
    /// The verb's Registry prior at emission.
    pub tellability: u8,
    /// 0 public, 1 witnessed, 2 faction, 3 secret.
    pub secrecy: u8,
    /// 0 sim, 1 player, 3 oracle, 4 director.
    pub origin: u8,
    /// The integer payload as emitted.
    pub payload: Vec<i64>,
    /// The cast, in ordinal order (role code, then entity).
    pub roles: Vec<Role>,
    /// The snapshots, strictly ascending by key.
    pub snapshots: Vec<Snapshot>,
    /// The causes, strictly ascending by `cause_id`.
    pub causes: Vec<Cause>,
}

/// `LIFE_ENTRY`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifeEntry {
    /// The person.
    pub person_id: u64,
    /// 0 born, 1 founder, 2 immigrant.
    pub origin: u8,
    /// 0 male, 1 female.
    pub sex: u8,
    /// 0 = none.
    pub mother_id: u64,
    /// 0 = none or unknown.
    pub father_id: u64,
    /// Negative for founders born before the epoch.
    pub birth_tick: i64,
}

/// `LIFE_EXIT`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifeExit {
    /// The person.
    pub person_id: u64,
    /// 0 death, 1 emigration off-map.
    pub exit_kind: u8,
    /// M2's enum: 0 none (emigration), 1 natural, 2 murder, 3 battle, 4 execution, 5 accident.
    pub death_cause: u8,
    /// The `DEATH` event, when one was admitted.
    pub event_id: Option<u64>,
}

/// `STEP_DIGEST`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepDigest {
    /// Ledger rows after the compaction of this instant.
    pub living: u64,
    /// `LIFE_ENTRY` records with origin born since the previous digest.
    pub births: u32,
    /// `LIFE_EXIT` records with `exit_kind` death since the previous digest.
    pub deaths: u32,
    /// Plane A's hash after `Bookkeeping`.
    pub state_hash: u64,
    /// Plane B's (in P1a the empty-plane constant: the engine hashes nothing in plane B).
    pub observation_hash: u64,
    /// Plane C's (in P1a the empty-plane constant).
    pub narrative_hash: u64,
}

/// `KEYFRAME`: a pointer to a full snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keyframe {
    /// `rehoboam/rh-snapshot/keyframe/v1`.
    pub keyframe_format: String,
    /// Relative to the run directory.
    pub keyframe_path: String,
    /// The hash the keyframe's header carries.
    pub state_hash: u64,
    /// Accepted inputs at the keyframe.
    pub journal_position: u64,
}

/// One `sift_binding` child row: role variable to entity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Binding {
    /// The pattern's role variable.
    pub role: String,
    /// `rh-ids` `EntityKind`.
    pub entity_kind: u8,
    /// The entity's raw id.
    pub entity_id: u64,
}

/// One `sift_match_event` child row: a clause and the event it bound.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClauseEvent {
    /// The clause's `bind` (`e1`).
    pub clause: String,
    /// The channel-A event id.
    pub event_id: u64,
}

/// `MATCH` (channel C).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    /// The content-addressed `MatchId` (`channel_c_v0.sql` R5).
    pub match_id: u64,
    /// The Registry `[pattern_packs]` id.
    pub pack: String,
    /// The pattern's `id`.
    pub pattern_id: String,
    /// The pattern's `version`.
    pub pattern_version: u16,
    /// 0 latest, 1 earliest, 2 all.
    pub selection: u8,
    /// 0 `clear_binding`, 1 consume, 2 keep.
    pub on_complete: u8,
    /// 1 complete (the only status in v0).
    pub status: u8,
    /// The first bound event's tick.
    pub opened_tick: i64,
    /// The completing event.
    pub closed_by_event_id: u64,
    /// Its channel-A seq.
    pub closed_by_seq: u64,
    /// The bindings, strictly ascending by role (bytewise).
    pub bindings: Vec<Binding>,
    /// The matched events in clause order.
    pub events: Vec<ClauseEvent>,
}

/// A record's body: one of the kinds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    /// `INPUT`.
    Input(Input),
    /// `EVENT`.
    Event(Event),
    /// `LIFE_ENTRY`.
    LifeEntry(LifeEntry),
    /// `LIFE_EXIT`.
    LifeExit(LifeExit),
    /// `STEP_DIGEST`.
    StepDigest(StepDigest),
    /// `KEYFRAME`.
    Keyframe(Keyframe),
    /// `MATCH`.
    Match(Match),
}

impl Body {
    /// Its kind.
    #[must_use]
    pub const fn kind(&self) -> Kind {
        match self {
            Self::Input(_) => Kind::Input,
            Self::Event(_) => Kind::Event,
            Self::LifeEntry(_) => Kind::LifeEntry,
            Self::LifeExit(_) => Kind::LifeExit,
            Self::StepDigest(_) => Kind::StepDigest,
            Self::Keyframe(_) => Kind::Keyframe,
            Self::Match(_) => Kind::Match,
        }
    }

    /// The order key's `(k1, k2)` within its class at one tick (channel A, R4): `journal_seq`,
    /// `event_id`, `(person_id, 0 entry | 1 exit)`, 0, 0. Channel C orders by its own key
    /// ([`crate::canonical`] does not need it; `rh-sifter` sorts its matches).
    #[must_use]
    pub const fn order_k(&self) -> (u64, u64) {
        match self {
            Self::Input(i) => (i.journal_seq, 0),
            Self::Event(e) => (e.event_id, 0),
            Self::LifeEntry(e) => (e.person_id, 0),
            Self::LifeExit(x) => (x.person_id, 1),
            Self::StepDigest(_) | Self::Keyframe(_) | Self::Match(_) => (0, 0),
        }
    }
}

/// One record: its header and its body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// The header.
    pub header: Header,
    /// The body.
    pub body: Body,
}

impl Record {
    /// Its kind.
    #[must_use]
    pub const fn kind(&self) -> Kind {
        self.body.kind()
    }

    /// The channel-A order key (R4): `(tick, class, k1, k2)`. Within a timeline's channel A, `seq`
    /// follows it.
    #[must_use]
    pub const fn order_key(&self) -> (i64, u8, u64, u64) {
        let (k1, k2) = self.body.order_k();
        (self.header.tick, self.kind().class(), k1, k2)
    }

    /// The heap bytes this record owns beyond its own `size_of` (its strings' and vectors'
    /// capacities, nested ones included): with `size_of::<Record>()`, its memory as allocated, the
    /// measure of `LiveEdge::queued_bytes` (GAP-1). The allocator's own overhead is not counted.
    #[must_use]
    pub fn heap_bytes(&self) -> usize {
        fn vec<T>(v: &Vec<T>) -> usize {
            v.capacity() * std::mem::size_of::<T>()
        }
        match &self.body {
            Body::Input(i) => i.input_kind.capacity() + i.scope.capacity() + i.producer.capacity(),
            Body::Event(e) => {
                e.verb.capacity()
                    + vec(&e.payload)
                    + vec(&e.roles)
                    + vec(&e.snapshots)
                    + vec(&e.causes)
            }
            Body::LifeEntry(_) | Body::LifeExit(_) | Body::StepDigest(_) => 0,
            Body::Keyframe(k) => k.keyframe_format.capacity() + k.keyframe_path.capacity(),
            Body::Match(m) => {
                m.pack.capacity()
                    + m.pattern_id.capacity()
                    + vec(&m.bindings)
                    + m.bindings.iter().map(|b| b.role.capacity()).sum::<usize>()
                    + vec(&m.events)
                    + m.events.iter().map(|c| c.clause.capacity()).sum::<usize>()
            }
        }
    }
}

/// The memory of `records` as allocated: the vector's capacity in records, and each record's own
/// heap ([`Record::heap_bytes`]).
#[must_use]
pub fn allocated(records: &Vec<Record>) -> usize {
    records.capacity() * std::mem::size_of::<Record>()
        + records.iter().map(Record::heap_bytes).sum::<usize>()
}

/// The sim-year of `tick` on a calendar of `minutes_per_year` minutes (R5): `floor(tick / mpy)`.
#[must_use]
pub const fn sim_year(tick: i64, minutes_per_year: i64) -> i64 {
    tick.div_euclid(minutes_per_year)
}
