//! What the host turns into channel-A records after each instant (T-09): the drained events
//! (`Engine::drain_events`), the journal rows applied at the instant, and the life-course delta.
//!
//! - [`event_body`]: an `rh-events` event as an `EVENT` record (SPR-020 T-06; envelope v0 §1–§5):
//!   the verb's id, act classes and tellability from the dictionaries; the event's place,
//!   magnitude, secrecy and origin; its cast (`event_role`, already in ordinal order: `emit` sorts
//!   it), its snapshots (`event_snapshot`, the attribute by its dictionary index, in key order) and
//!   its typed edges (`causal_edge`, the motive by its dictionary index), each edge's
//!   `cause_told` from the writer's admission bitset ([`Admissions`], host memory, never plane A).
//!   An event the chronicle does not admit is no record ([`Admissions::admits`]).
//! - [`input_body`]: a journal row as an `INPUT` record, linked to the journal by its row digest.
//! - [`LifeTracker`]: the `LIFE_ENTRY` and `LIFE_EXIT` records of the life-course log
//!   (`life_course.*`), which is append-only (SPR-011 ruling 5): a row beyond the last one seen is
//!   an entry at its `entry_tick`; an open row that has closed is an exit at its `exit_tick`.
//!
//! REF: SPR-011 rulings 5, 11, 13; `specs/stream/channel_a_v0.sql`; `05.10` §4 (the fidelity
//! promise).

use std::collections::BTreeSet;

use rh_core::world::WorldTables;
use rh_snapshot::journal::{JournaledInput, row_digest, stream_fields};

use crate::StreamError;
use crate::record::{Body, Cause, Event, Input, LifeEntry, LifeExit, Role, Snapshot};
use crate::segment::Dictionaries;

/// The per-timeline admission bitset of the Chronicle Writer (`envelope_v0.md` §5): one bit per
/// `EventId`, set when the event was admitted. Host memory only; it is rebuilt from the sealed
/// `event` tables at resume ([`Admissions::from_told`]), so it needs no plane-A memory and no
/// read-back. In every P1a world every draft is admitted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Admissions {
    words: Vec<u64>,
}

impl Admissions {
    /// No event seen yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The bitset of a resumed timeline: `told` are the ids of the sealed `EVENT` records (every
    /// record is an admitted event; an id without one was refused).
    #[must_use]
    pub fn from_told(told: impl IntoIterator<Item = u64>) -> Self {
        let mut a = Self::new();
        for id in told {
            a.set(id);
        }
        a
    }

    fn set(&mut self, id: u64) {
        let (w, bit) = (usize::try_from(id / 64).unwrap_or(usize::MAX), id % 64);
        if self.words.len() <= w {
            self.words.resize(w + 1, 0);
        }
        self.words[w] |= 1 << bit;
    }

    /// Records the event's admission, and returns whether it gets a record.
    pub fn admits(&mut self, e: &rh_events::Event) -> bool {
        if e.admitted {
            self.set(e.id.to_raw());
        }
        e.admitted
    }

    /// Whether the event `id` was admitted (and so has an `EVENT` record).
    #[must_use]
    pub fn told(&self, id: u64) -> bool {
        usize::try_from(id / 64)
            .ok()
            .and_then(|w| self.words.get(w))
            .is_some_and(|w| w & (1 << (id % 64)) != 0)
    }
}

/// The `EVENT` body of an emitted, admitted event; `admissions` gives each edge's `cause_told`.
///
/// # Errors
/// [`StreamError::Convert`] if the verb, a snapshot's attribute or an edge's motive is not in the
/// dictionaries, or a field does not fit its column.
pub fn event_body(
    e: &rh_events::Event,
    dictionaries: &Dictionaries,
    admissions: &Admissions,
) -> Result<Body, StreamError> {
    let bad = |m: String| StreamError::Convert(format!("event {} ({}): {m}", e.id, e.kind));
    let verb = dictionaries.verb(e.kind).ok_or_else(|| {
        bad(format!(
            "names no verb of the Registry {}",
            dictionaries.registry_version
        ))
    })?;
    let roles = e
        .roles
        .iter()
        .map(|r| Role {
            entity_kind: r.entity.kind().tag(),
            entity_id: r.entity.raw(),
            role: r.role.0,
        })
        .collect();
    let mut snapshots = e
        .snapshots
        .iter()
        .map(|s| {
            let attr_id = dictionaries
                .attributes
                .binary_search_by(|a| a.name.as_bytes().cmp(s.attr.as_bytes()))
                .ok()
                .and_then(|i| u16::try_from(i).ok())
                .ok_or_else(|| bad(format!("attribute {} is not in the dictionary", s.attr)))?;
            Ok(Snapshot {
                attr_id,
                entity_kind: s.entity.kind().tag(),
                entity_id: s.entity.raw(),
                partner_kind: s.partner.map(|p| p.kind().tag()),
                partner_id: s.partner.map(rh_ids::EntityRef::raw),
                value: s.value,
            })
        })
        .collect::<Result<Vec<_>, StreamError>>()?;
    snapshots.sort_by_key(Snapshot::key);
    let causes = e
        .causes
        .iter()
        .map(|c| {
            let motive = c
                .motive
                .map(|m| {
                    dictionaries
                        .motive_edges
                        .iter()
                        .position(|x| x == m)
                        .and_then(|i| u16::try_from(i).ok())
                        .ok_or_else(|| bad(format!("motive {m} is not in the dictionary")))
                })
                .transpose()?;
            Ok(Cause {
                cause_id: c.cause.to_raw(),
                kind: c.kind.code(),
                motive,
                weight: c.weight,
                evidence: c.evidence,
                cause_told: admissions.told(c.cause.to_raw()),
            })
        })
        .collect::<Result<Vec<_>, StreamError>>()?;
    Ok(Body::Event(Event {
        event_id: e.id.to_raw(),
        phase: e.phase,
        verb_id: verb.verb_id,
        verb: verb.name.clone(),
        act_classes: verb.act_classes,
        place_id: e.place.map(rh_ids::PlaceId::to_raw),
        magnitude: e.magnitude,
        tellability: verb.tellability,
        secrecy: e.secrecy.code(),
        origin: e.origin.code(),
        payload: e.payload.clone(),
        roles,
        snapshots,
        causes,
    }))
}

/// The `INPUT` body of a journal row (applied at the row's tick).
#[must_use]
pub fn input_body(row: &JournaledInput) -> Body {
    let (input_kind, scope, scope_id, producer) = stream_fields(row);
    Body::Input(Input {
        journal_seq: u64::from(row.seq),
        class: 0,
        input_kind,
        scope,
        scope_id,
        producer,
        payload_hash: row.payload_hash,
        row_digest: row_digest(row),
    })
}

/// The life-course log's columns the tracker reads (M2's `[[owns]]`).
const PERSON: &str = "life_course.person_id";
const ORIGIN: &str = "life_course.origin";
const MOTHER: &str = "life_course.mother";
const FATHER: &str = "life_course.father";
const SEX: &str = "life_course.sex";
const BIRTH: &str = "life_course.birth_tick";
const ENTRY: &str = "life_course.entry_tick";
const EXIT_KIND: &str = "life_course.exit_kind";
const EXIT_TICK: &str = "life_course.exit_tick";
const CAUSE: &str = "life_course.death_cause";

/// The life-course log's `exit_kind` of an open row (M2: open 0, death 1, emigration 2).
const OPEN: i64 = 0;

/// Follows the life-course log and yields its entries and exits as records (module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LifeTracker {
    /// The rows seen so far.
    seen: usize,
    /// The seen rows that were open when last observed.
    open: BTreeSet<usize>,
}

fn column<'a>(t: &'a WorldTables, name: &str) -> Result<&'a [i64], StreamError> {
    t.get(name)
        .map(Vec::as_slice)
        .ok_or_else(|| StreamError::Convert(format!("the world has no {name} column")))
}

fn small(name: &str, v: i64) -> Result<u8, StreamError> {
    u8::try_from(v).map_err(|_| StreamError::Convert(format!("{name} = {v} is not a small code")))
}

impl LifeTracker {
    /// A tracker that has seen nothing: its first [`LifeTracker::observe`] yields every row as an
    /// entry (genesis: the founders at tick 0).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A tracker that has seen every row of `tables` already (a run resumed from a keyframe:
    /// those records are in the sealed segments).
    ///
    /// # Errors
    /// [`StreamError::Convert`] if the world has no life-course log.
    pub fn resumed(tables: &WorldTables) -> Result<Self, StreamError> {
        let kinds = column(tables, EXIT_KIND)?;
        Ok(Self {
            seen: kinds.len(),
            open: (0..kinds.len()).filter(|&i| kinds[i] == OPEN).collect(),
        })
    }

    /// The `(tick, body)` of every entry and exit since the last call, entries first.
    ///
    /// # Errors
    /// [`StreamError::Convert`] if a column is missing, the log shrank, or a value is outside its
    /// code (the log is append-only; ruling 5).
    pub fn observe(&mut self, tables: &WorldTables) -> Result<Vec<(i64, Body)>, StreamError> {
        let person = column(tables, PERSON)?;
        let kinds = column(tables, EXIT_KIND)?;
        if person.len() < self.seen {
            return Err(StreamError::Convert(format!(
                "the life-course log shrank from {} to {} rows: it is append-only (ruling 5)",
                self.seen,
                person.len()
            )));
        }
        let mut out = Vec::new();
        let new = self.seen..person.len();
        if !new.is_empty() {
            let (origin, mother, father, sex, birth, entry) = (
                column(tables, ORIGIN)?,
                column(tables, MOTHER)?,
                column(tables, FATHER)?,
                column(tables, SEX)?,
                column(tables, BIRTH)?,
                column(tables, ENTRY)?,
            );
            for i in new.clone() {
                out.push((
                    entry[i],
                    Body::LifeEntry(LifeEntry {
                        person_id: person[i].cast_unsigned(),
                        origin: small("origin", origin[i])?,
                        sex: small("sex", sex[i])?,
                        mother_id: mother[i].cast_unsigned(),
                        father_id: father[i].cast_unsigned(),
                        birth_tick: birth[i],
                    }),
                ));
            }
        }
        let candidates: Vec<usize> = self.open.iter().copied().chain(new.clone()).collect();
        let closed: Vec<usize> = candidates
            .into_iter()
            .filter(|&i| kinds[i] != OPEN)
            .collect();
        if !closed.is_empty() {
            let (tick, cause) = (column(tables, EXIT_TICK)?, column(tables, CAUSE)?);
            for &i in &closed {
                // M2's codes (death 1, emigration 2) to channel A's (death 0, emigration 1).
                let exit_kind = match kinds[i] {
                    1 => 0,
                    2 => 1,
                    k => {
                        return Err(StreamError::Convert(format!(
                            "life_course.exit_kind = {k} is not open, death or emigration"
                        )));
                    }
                };
                out.push((
                    tick[i],
                    Body::LifeExit(LifeExit {
                        person_id: person[i].cast_unsigned(),
                        exit_kind,
                        death_cause: small("death_cause", cause[i])?,
                        event_id: None,
                    }),
                ));
            }
        }
        for i in new {
            if kinds[i] == OPEN {
                self.open.insert(i);
            }
        }
        for i in closed {
            self.open.remove(&i);
        }
        self.seen = person.len();
        Ok(out)
    }
}

/// Links each death's `LIFE_EXIT` to its `DEATH` event (DATA-5 admits every death at rungs 0–2,
/// ruling 11): the `EVENT` of verb `DEATH` at the same tick whose target is the person. A death
/// without one keeps `event_id` empty.
pub fn link_exits(records: &mut [(i64, Body)]) {
    let deaths: Vec<(i64, u64, u64)> = records
        .iter()
        .filter_map(|(t, b)| match b {
            Body::Event(e) if e.verb == "DEATH" => e
                .roles
                .iter()
                .find(|r| r.role == rh_events::RoleId::TARGET.0)
                .map(|r| (*t, r.entity_id, e.event_id)),
            _ => None,
        })
        .collect();
    for (t, b) in records.iter_mut() {
        if let Body::LifeExit(x) = b
            && x.exit_kind == 0
            && x.event_id.is_none()
        {
            x.event_id = deaths
                .iter()
                .find(|(dt, p, _)| dt == t && *p == x.person_id)
                .map(|(_, _, id)| *id);
        }
    }
}
