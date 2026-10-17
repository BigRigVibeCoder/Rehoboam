//! What the host turns into channel-A records after each instant (T-09): the drained events
//! (`Engine::drain_events`), the journal rows applied at the instant, and the life-course delta.
//!
//! - [`event_body`]: an `rh-events` event as an `EVENT` record, with the verb's id, act classes
//!   and tellability from the dictionaries, its subjects as roles (ruling 11: the deceased of a
//!   `DEATH` is its `target`; a natural death has no actor; `EXTINCTION` has no role), and its
//!   untyped causes as deterministic, told `triggered` edges (kind 1, no motive, weight 0,
//!   evidence 1) in ascending order. The cast is in ordinal order (role code, then entity;
//!   `envelope_v0.md` §1). M2's events declare no snapshot, so none is written. P1.0 admits every
//!   draft (`05.2` §4.0), so every event is a record.
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
use crate::record::{Body, Cause, Event, Input, LifeEntry, LifeExit, Role};
use crate::segment::Dictionaries;

/// `causal_edge.kind` of SPR-006's untyped causes (ruling 11).
pub const TRIGGERED: u8 = 1;
/// `event_role.role` of a target (Registry `[event_roles]` index 1).
pub const ROLE_TARGET: u8 = 1;

/// The role of each subject of a P1.0 verb, in the emitter's subject order (ruling 11). A verb
/// not listed has no role mapping yet and is refused rather than guessed.
#[must_use]
pub fn roles_of(verb: &str) -> Option<&'static [u8]> {
    match verb {
        "DEATH" => Some(&[ROLE_TARGET]),
        "EXTINCTION" => Some(&[]),
        _ => None,
    }
}

/// The `EVENT` body of an emitted event.
///
/// # Errors
/// [`StreamError::Convert`] if the verb is not in the dictionaries, has no P1.0 role mapping, or
/// the event has another number of subjects.
pub fn event_body(e: &rh_events::Event, dictionaries: &Dictionaries) -> Result<Body, StreamError> {
    let verb = dictionaries.verb(e.kind).ok_or_else(|| {
        StreamError::Convert(format!(
            "event {} of kind {} names no verb of the Registry {}",
            e.id, e.kind, dictionaries.registry_version
        ))
    })?;
    let roles = roles_of(e.kind).ok_or_else(|| {
        StreamError::Convert(format!(
            "verb {} has no role mapping in P1.0 (ruling 11)",
            e.kind
        ))
    })?;
    if roles.len() != e.subjects.len() {
        return Err(StreamError::Convert(format!(
            "event {} ({}): {} subjects, the verb's roles are {}",
            e.id,
            e.kind,
            e.subjects.len(),
            roles.len()
        )));
    }
    let causes: BTreeSet<u64> = e.causes.iter().map(|c| c.to_raw()).collect();
    let mut cast: Vec<Role> = e
        .subjects
        .iter()
        .zip(roles)
        .map(|(s, &role)| Role {
            entity_kind: s.kind() as u8,
            entity_id: s.raw(),
            role,
        })
        .collect();
    cast.sort_by_key(Role::order_key);
    Ok(Body::Event(Event {
        event_id: e.id.to_raw(),
        phase: e.phase,
        verb_id: verb.verb_id,
        verb: verb.name.clone(),
        act_classes: verb.act_classes,
        place_id: None,
        magnitude: 0,
        tellability: verb.tellability,
        secrecy: 0,
        origin: 0,
        payload: e.payload.clone(),
        roles: cast,
        snapshots: Vec::new(),
        causes: causes.into_iter().map(Cause::triggered).collect(),
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
                .find(|r| r.role == ROLE_TARGET)
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
