//! The event envelope v0 (`specs/chronicle/envelope_v0.md` §2, §5; SPR-020 T-02): roles,
//! anchored snapshots, typed causal edges and the event-level fields a draft sets.
//!
//! Names, not indexes: a snapshot's attribute and an edge's motive are the Registry **names**
//! (`&'static str` constants the skeleton emits), so the digest hashes the name and a Registry
//! addition that sorts earlier moves no hash (R4). A role is its code, the index in
//! `[event_roles].ids`, which the loader pins for the first four (§8).
//!
//! Fixed point: an edge's `weight` and `evidence` are `Q8_8` raw bits in an `i16` (this crate sits
//! below `rh-core::fixed` and stores the bits; `rh-core` builds them from its `Q8_8`). A snapshot's
//! value is an `i32` in the attribute's encoding (§4 rule 5). Nothing here is a float.

use core::fmt;

use rh_ids::{EntityRef, EventId};

use crate::DraftRef;

/// `Q8_8` raw bits of 1: the evidence of a deterministic edge (§5).
pub const EVIDENCE_ONE: i16 = 256;

/// A reference to an event in an event-typed slot (EVO-001; `05.1` §2 rule 2): one of the step's
/// own earlier drafts, an id read from state, or none.
///
/// It is opaque: it has no integer conversion and cannot be used as a draw key. The merge
/// replaces every [`EventRef::Draft`] with the id it assigns in the same merge (`05.1` §2 rule 4).
/// `None` is not an edge: a draft's causal edges hold `Draft` or `Known` only.
// SPR-014 T-02 owns this type's other uses (a timer's and a command's `cause`, payload fields);
// SPR-020 T-02 uses it in causal edges. The definition is SPR-014 T-02's, word for word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventRef {
    /// The step's own `n`-th draft.
    Draft(DraftRef),
    /// An event that already has its id.
    Known(EventId),
    /// No event.
    None,
}

/// A role code: the index in the Registry's `[event_roles].ids` (§1). The first four are pinned
/// by the loader (`Registry::problems`); a role is only ever appended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoleId(pub u8);

impl RoleId {
    /// `actor` (0): who acts.
    pub const ACTOR: Self = Self(0);
    /// `target` (1): who is acted on (the deceased of a `DEATH`: SPR-011 ruling 11).
    pub const TARGET: Self = Self(1);
    /// `witness` (2): present, neither acting nor acted on; the only role that may repeat.
    pub const WITNESS: Self = Self(2);
    /// `instrument` (3): the S6 artifact used (reserved until S6).
    pub const INSTRUMENT: Self = Self(3);
}

/// One row of an event's cast: who played which role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoleRow {
    /// The role code.
    pub role: RoleId,
    /// The entity.
    pub entity: EntityRef,
}

/// An anchored snapshot (§4): the value of a declared attribute the emitter read at emission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// The attribute's Registry name (`[attributes]`).
    pub attr: &'static str,
    /// The attribute's first argument: a cast member, or [`EntityRef::WORLD`] for the event itself.
    pub entity: EntityRef,
    /// The second argument of a two-argument attribute.
    pub partner: Option<EntityRef>,
    /// The value in the attribute's encoding (§4 rule 5).
    pub value: i32,
}

impl Snapshot {
    /// The canonical sort key of §4 rule 6 within one attribute name: `(entity, partner)`, a row
    /// without a partner first. The attribute's place is its `attr_id`, which the caller knows.
    #[must_use]
    pub fn key(&self) -> (EntityRef, Option<EntityRef>) {
        (self.entity, self.partner)
    }
}

/// The event's secrecy (`event.secrecy`; §2): the draft sets it, public by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
pub enum Secrecy {
    /// 0: anyone may know.
    #[default]
    Public = 0,
    /// 1: those present know.
    Witnessed = 1,
    /// 2: a faction knows.
    Faction = 2,
    /// 3: the actors alone know.
    Secret = 3,
}

impl Secrecy {
    /// Every secrecy, in code order.
    pub const ALL: [Self; 4] = [Self::Public, Self::Witnessed, Self::Faction, Self::Secret];

    /// The code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The secrecy with this code.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.code() == code)
    }
}

/// Where an event's causal chain starts (`event.origin`; §2): the engine stamps it from the job
/// that produced the draft. A system cannot set it. Code 2 is retired (ADR-0034).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
pub enum Origin {
    /// 0: the simulation.
    #[default]
    Sim = 0,
    /// 1: a player's journaled input (`PlayerAction`, `SpeechAct`).
    Player = 1,
    /// 3: an Oracle's journaled input (`OracleMove`, `OracleWhisper`, `OraclePierce`,
    /// `OracleOmen`, `OracleWard`).
    Oracle = 3,
    /// 4: the director's journaled input (`ForkOverride`).
    Director = 4,
}

impl Origin {
    /// Every origin, in code order.
    pub const ALL: [Self; 4] = [Self::Sim, Self::Player, Self::Oracle, Self::Director];

    /// The code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The origin with this code (2 is retired and has none).
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|o| o.code() == code)
    }
}

/// A causal edge's kind (DATA-2; §5): `schema.sql`'s codes, 0–8.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EdgeKind {
    /// 0: an explicit unlocking reference (a verb with `references`).
    Contingent = 0,
    /// 1: the mechanical consequence of an earlier event.
    Triggered = 1,
    /// 2: a testimony hop (P1b); evidence is its fidelity.
    RelayedKnowledge = 2,
    /// 3: a utility decision's contributor, with an optional motive.
    MotivatedBy = 3,
    /// 4: an enabling condition that is itself an event.
    Enabled = 4,
    /// 5: a hazard this event's cause raised; evidence is the share.
    Probabilistic = 5,
    /// 6: reserved for an input event (no writer in P1a).
    Player = 6,
    /// 7: reserved for an input event (no writer in P1a).
    Oracle = 7,
    /// 8: reserved for an input event (no writer in P1a).
    Director = 8,
}

impl EdgeKind {
    /// Every kind, in code order.
    pub const ALL: [Self; 9] = [
        Self::Contingent,
        Self::Triggered,
        Self::RelayedKnowledge,
        Self::MotivatedBy,
        Self::Enabled,
        Self::Probabilistic,
        Self::Player,
        Self::Oracle,
        Self::Director,
    ];

    /// §5's coalescing precedence: richness first, then an explicit reference, a mechanical
    /// consequence, an enabling condition, a hazard share, a testimony hop. The three input kinds
    /// have no writer and never coalesce; they rank last so that [`crate::EventLog::append`]
    /// refuses them.
    pub const PRECEDENCE: [Self; 9] = [
        Self::MotivatedBy,
        Self::Contingent,
        Self::Triggered,
        Self::Enabled,
        Self::Probabilistic,
        Self::RelayedKnowledge,
        Self::Player,
        Self::Oracle,
        Self::Director,
    ];

    /// The code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The kind with this code.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.code() == code)
    }

    /// The `schema.sql` name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Contingent => "contingent",
            Self::Triggered => "triggered",
            Self::RelayedKnowledge => "relayed_knowledge",
            Self::MotivatedBy => "motivated_by",
            Self::Enabled => "enabled",
            Self::Probabilistic => "probabilistic",
            Self::Player => "player",
            Self::Oracle => "oracle",
            Self::Director => "director",
        }
    }

    /// Whether the kind's evidence is exactly 1 by rule (§5).
    #[must_use]
    pub const fn is_deterministic(self) -> bool {
        matches!(
            self,
            Self::Contingent | Self::Triggered | Self::Player | Self::Oracle | Self::Director
        )
    }

    /// Whether a system may write the kind in P1a (kinds 6–8 have no writer: §5).
    #[must_use]
    pub const fn has_writer(self) -> bool {
        !matches!(self, Self::Player | Self::Oracle | Self::Director)
    }

    /// The kind's rank in [`EdgeKind::PRECEDENCE`] (0 first).
    #[must_use]
    pub fn precedence(self) -> usize {
        Self::PRECEDENCE
            .iter()
            .position(|k| *k == self)
            .unwrap_or(Self::PRECEDENCE.len())
    }
}

impl fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A draft's causal edge: to an earlier draft of the step or to a known event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CausalEdge {
    /// The cause: `Draft` or `Known` ([`EventRef::None`] is refused at the merge).
    pub cause: EventRef,
    /// The kind.
    pub kind: EdgeKind,
    /// The `[motive_edges]` name, on a `motivated_by` edge only.
    pub motive: Option<&'static str>,
    /// `Q8_8` raw bits, signed: the contribution; 0 off `motivated_by`.
    pub weight: i16,
    /// `Q8_8` raw bits in [0, 256]: 256 on a deterministic kind.
    pub evidence: i16,
}

impl CausalEdge {
    /// The edge with its cause resolved to an id.
    #[must_use]
    pub const fn resolved(&self, cause: EventId) -> Edge {
        Edge {
            cause,
            kind: self.kind,
            motive: self.motive,
            weight: self.weight,
            evidence: self.evidence,
        }
    }
}

/// An emitted event's causal edge: the cause is an id smaller than the event's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    /// The cause.
    pub cause: EventId,
    /// The kind.
    pub kind: EdgeKind,
    /// The `[motive_edges]` name, on a `motivated_by` edge only.
    pub motive: Option<&'static str>,
    /// `Q8_8` raw bits, signed.
    pub weight: i16,
    /// `Q8_8` raw bits in [0, 256].
    pub evidence: i16,
}
