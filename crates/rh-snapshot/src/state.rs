//! `state_hash` over all plane-A state: the world tables and the engine-held tables (SPR-007 T-02;
//! Architect ruling 2026-10-16: one plane-A hash, covering what the engine holds outside
//! `WorldTables`).
//!
//! # Engine tables (layout `rehoboam/rh-snapshot/hash/v1`, normative)
//!
//! An [`EngineState`] is hashed as one plane-A table set: each world column (a table whose row `i`
//! is its `i`-th `i64`, see [`crate::hash`]) and the eight engine tables below, all under the
//! plane-hash rule (strictly increasing byte order of names). Engine table names start with
//! `rh-core/`, which no `entity.column` name can, so they never collide with a world column.
//!
//! Row encodings use little-endian integers; `str` is `u32 len || UTF-8`; `opt(x)` is `u8 0`, or
//! `u8 1 || x`; an entity is `u8 kind tag || u64 raw` (`rh_ids::EntityKind::tag`); a phase is
//! `u8 index` (`Phase::index`); a time is its `i64` minutes; `list(x)` is `u32 count || x...`.
//!
//! | Table | One row per | Row |
//! |---|---|---|
//! | `rh-core/agenda.meta` | (one row) | `u64 next_seq` |
//! | `rh-core/agenda.timers` | heap entry, in key order | `i64 time, u8 phase, entity, u64 seq, u32 owner, u64 purpose, u32 kind, i64 payload, u8 tombstoned` |
//! | `rh-core/engine.commands` | queued command: owners in name order, each queue in key order | `str owner, i64 tick, u8 phase, u64 person, u64 seq, str sender, opt(u64 cause), str kind, list(i64 args), opt(u64 draft)` |
//! | `rh-core/engine.epochs` | entity a material input touched, in entity order | `entity, u32 epoch` |
//! | `rh-core/engine.inputs` | queued material input: instants in order, inputs in submission order | `i64 time, u32 index, list(entity scope)` |
//! | `rh-core/engine.meta` | (one row) | `opt(i64 cursor), u32 timeline, u64 applied_inputs, u64 command next_seq` |
//! | `rh-core/events.held` | undrained event, in id order | `u64 id, i64 tick, u8 phase, str emitter, str kind, list(entity subjects), list(u64 causes), list(i64 payload)` |
//! | `rh-core/events.meta` | (one row) | `u64 next_seq, opt(i64 tick || u8 phase) last, u64 running` |
//!
//! `bin/gen_hash_golden.py` encodes a synthetic engine state the same way, independently, and
//! `tests/golden.rs` checks every table root and the plane hash. The keyframe (T-02) stores these
//! rows and decodes them with [`decode_engine_tables`].
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §6 item 6; ADR-0024;
//! `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md` T-02.

use std::collections::BTreeMap;
use std::fmt;

use rh_core::agenda::{AgendaKey, AgendaState, Timer, TimerId};
use rh_core::command::{Command, CommandQueuesState, QueuedCommand};
use rh_core::engine::MaterialInput;
use rh_core::phase::Phase;
use rh_core::register::Registered;
use rh_core::state::EngineState;
use rh_core::time::SimTime;
use rh_core::world::WorldTables;
use rh_events::{DraftRef, Event, EventLogState};
use rh_ids::{EntityKind, EntityRef, EventId, PersonId, PurposeId, SystemId, TimelineId};

use crate::hash::{Plane, PlaneDigest, TableDigest};

/// The agenda's counter.
pub const AGENDA_META: &str = "rh-core/agenda.meta";
/// The agenda's heap entries.
pub const AGENDA_TIMERS: &str = "rh-core/agenda.timers";
/// The lifecycle command queues.
pub const ENGINE_COMMANDS: &str = "rh-core/engine.commands";
/// The per-entity epochs.
pub const ENGINE_EPOCHS: &str = "rh-core/engine.epochs";
/// The queued material inputs.
pub const ENGINE_INPUTS: &str = "rh-core/engine.inputs";
/// The cursor, timeline, applied-input count and command counter.
pub const ENGINE_META: &str = "rh-core/engine.meta";
/// The undrained events.
pub const EVENTS_HELD: &str = "rh-core/events.held";
/// The event log's counter, last `(tick, phase)` and running hash.
pub const EVENTS_META: &str = "rh-core/events.meta";

/// Every engine table name, in byte order.
pub const ENGINE_TABLES: [&str; 8] = [
    AGENDA_META,
    AGENDA_TIMERS,
    ENGINE_COMMANDS,
    ENGINE_EPOCHS,
    ENGINE_INPUTS,
    ENGINE_META,
    EVENTS_HELD,
    EVENTS_META,
];

/// `state_hash`: the plane-A hash of an engine state (world columns and engine tables).
#[must_use]
pub fn state_hash(state: &EngineState) -> u64 {
    state_digest(state).hash
}

/// The full plane-A digest of an engine state.
#[must_use]
pub fn state_digest(state: &EngineState) -> PlaneDigest {
    let mut tables: BTreeMap<String, TableDigest> = state
        .tables
        .columns()
        .map(|(name, _, rows)| (name.to_owned(), TableDigest::of(name, rows)))
        .collect();
    for (name, rows) in engine_tables(state) {
        tables.insert(name.to_owned(), TableDigest::of(name, rows.as_slice()));
    }
    PlaneDigest::from_tables(Plane::A, tables)
}

/// The engine tables of a state, each as its encoded rows, in name order.
#[must_use]
pub fn engine_tables(state: &EngineState) -> Vec<(&'static str, Vec<Vec<u8>>)> {
    let mut agenda_meta = W::default();
    agenda_meta.u64(state.agenda.next_seq);
    let mut meta = W::default();
    meta.opt_i64(state.cursor.map(SimTime::minutes))
        .u32(state.timeline.to_raw())
        .u64(state.applied_inputs)
        .u64(state.commands.next_seq);
    vec![
        (AGENDA_META, vec![agenda_meta.0]),
        (AGENDA_TIMERS, timer_rows(&state.agenda)),
        (ENGINE_COMMANDS, command_rows(&state.commands)),
        (ENGINE_EPOCHS, epoch_rows(&state.epochs)),
        (ENGINE_INPUTS, input_rows(&state.inputs)),
        (ENGINE_META, vec![meta.0]),
        (EVENTS_HELD, held_rows(&state.events.events)),
        (EVENTS_META, vec![events_meta_row(&state.events)]),
    ]
}

fn timer_rows(agenda: &AgendaState) -> Vec<Vec<u8>> {
    let tombstoned = |seq: u64| {
        agenda
            .tombstones
            .binary_search(&TimerId::from_raw(seq))
            .is_ok()
    };
    agenda
        .entries
        .iter()
        .map(|(key, timer)| {
            let mut w = W::default();
            w.i64(key.time.minutes())
                .u8(key.phase.index())
                .entity(key.entity)
                .u64(key.seq)
                .u32(timer.owner.to_raw())
                .u64(timer.purpose.to_raw())
                .u32(timer.kind)
                .i64(timer.payload)
                .u8(u8::from(tombstoned(key.seq)));
            w.0
        })
        .collect()
}

fn command_rows(commands: &CommandQueuesState) -> Vec<Vec<u8>> {
    commands
        .queues
        .iter()
        .flat_map(|(owner, queue)| queue.iter().map(move |c| (*owner, c)))
        .map(|(owner, queued)| {
            let mut w = W::default();
            w.str(owner)
                .i64(queued.tick.minutes())
                .u8(queued.phase.index())
                .u64(queued.command.person.to_raw())
                .u64(queued.seq)
                .str(queued.sender)
                .opt_u64(queued.cause.map(EventId::to_raw))
                .str(queued.command.kind)
                .list_i64(&queued.command.args)
                .opt_u64(queued.command.cause.map(|d| d.0 as u64));
            w.0
        })
        .collect()
}

fn epoch_rows(epochs: &[(EntityRef, u32)]) -> Vec<Vec<u8>> {
    epochs
        .iter()
        .map(|(entity, epoch)| {
            let mut w = W::default();
            w.entity(*entity).u32(*epoch);
            w.0
        })
        .collect()
}

fn input_rows(inputs: &[(SimTime, Vec<MaterialInput>)]) -> Vec<Vec<u8>> {
    inputs
        .iter()
        .flat_map(|(at, list)| {
            list.iter()
                .enumerate()
                .map(move |(k, input)| (*at, k, input))
        })
        .map(|(at, k, input)| {
            let mut w = W::default();
            w.i64(at.minutes())
                .u32(len32(k))
                .u32(len32(input.scope.len()));
            for entity in &input.scope {
                w.entity(*entity);
            }
            w.0
        })
        .collect()
}

fn held_rows(events: &[Event]) -> Vec<Vec<u8>> {
    events
        .iter()
        .map(|event| {
            let mut w = W::default();
            w.u64(event.id.to_raw())
                .i64(event.tick)
                .u8(event.phase)
                .str(event.emitter)
                .str(event.kind)
                .u32(len32(event.subjects.len()));
            for subject in &event.subjects {
                w.entity(*subject);
            }
            w.u32(len32(event.causes.len()));
            for cause in &event.causes {
                w.u64(cause.to_raw());
            }
            w.list_i64(&event.payload);
            w.0
        })
        .collect()
}

fn events_meta_row(log: &EventLogState) -> Vec<u8> {
    let mut w = W::default();
    w.u64(log.next_seq);
    match log.last {
        None => {
            w.u8(0);
        }
        Some((tick, phase)) => {
            w.u8(1).i64(tick).u8(phase);
        }
    }
    w.u64(log.running);
    w.0
}

fn len32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A row writer.
#[derive(Default)]
struct W(Vec<u8>);

impl W {
    fn u8(&mut self, x: u8) -> &mut Self {
        self.0.push(x);
        self
    }
    fn u32(&mut self, x: u32) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }
    fn u64(&mut self, x: u64) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }
    fn i64(&mut self, x: i64) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }
    fn str(&mut self, s: &str) -> &mut Self {
        self.u32(len32(s.len()));
        self.0.extend_from_slice(s.as_bytes());
        self
    }
    fn entity(&mut self, e: EntityRef) -> &mut Self {
        self.u8(e.kind().tag()).u64(e.raw())
    }
    fn opt_u64(&mut self, x: Option<u64>) -> &mut Self {
        match x {
            None => self.u8(0),
            Some(v) => self.u8(1).u64(v),
        }
    }
    fn opt_i64(&mut self, x: Option<i64>) -> &mut Self {
        match x {
            None => self.u8(0),
            Some(v) => self.u8(1).i64(v),
        }
    }
    fn list_i64(&mut self, xs: &[i64]) -> &mut Self {
        self.u32(len32(xs.len()));
        for &x in xs {
            self.i64(x);
        }
        self
    }
}

/// Engine tables that do not decode into a state the registered systems can hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeError(pub String);

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "engine tables: {}", self.0)
    }
}

impl std::error::Error for DecodeError {}

/// A row reader; every read is checked, and a row must be read to its end.
struct R<'a> {
    b: &'a [u8],
    i: usize,
    table: &'static str,
}

impl<'a> R<'a> {
    fn new(table: &'static str, b: &'a [u8]) -> Self {
        Self { b, i: 0, table }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.i.checked_add(n).filter(|&e| e <= self.b.len());
        let Some(end) = end else {
            return Err(DecodeError(format!("{}: a row ends early", self.table)));
        };
        let s = &self.b[self.i..end];
        self.i = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, DecodeError> {
        let mut a = [0; 4];
        a.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(a))
    }
    fn u64(&mut self) -> Result<u64, DecodeError> {
        let mut a = [0; 8];
        a.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(a))
    }
    fn i64(&mut self) -> Result<i64, DecodeError> {
        Ok(self.u64()?.cast_signed())
    }
    fn str(&mut self) -> Result<String, DecodeError> {
        let n = self.u32()? as usize;
        String::from_utf8(self.take(n)?.to_vec())
            .map_err(|_| DecodeError(format!("{}: a string is not UTF-8", self.table)))
    }
    fn flag(&mut self) -> Result<bool, DecodeError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            b => Err(DecodeError(format!("{}: flag byte {b}", self.table))),
        }
    }
    fn entity(&mut self) -> Result<EntityRef, DecodeError> {
        let tag = self.u8()?;
        let kind = EntityKind::from_tag(tag)
            .ok_or_else(|| DecodeError(format!("{}: entity kind {tag}", self.table)))?;
        Ok(EntityRef::new(kind, self.u64()?))
    }
    fn phase(&mut self) -> Result<Phase, DecodeError> {
        let i = self.u8()?;
        Phase::ALL
            .get(usize::from(i))
            .copied()
            .ok_or_else(|| DecodeError(format!("{}: phase {i}", self.table)))
    }
    fn count(&mut self) -> Result<usize, DecodeError> {
        let n = self.u32()? as usize;
        if n > self.b.len() {
            return Err(DecodeError(format!(
                "{}: a count of {n} exceeds the row",
                self.table
            )));
        }
        Ok(n)
    }
    fn list_i64(&mut self) -> Result<Vec<i64>, DecodeError> {
        let count = self.count()?;
        (0..count).map(|_| self.i64()).collect()
    }
    fn end(&self) -> Result<(), DecodeError> {
        if self.i == self.b.len() {
            Ok(())
        } else {
            Err(DecodeError(format!(
                "{}: a row has trailing bytes",
                self.table
            )))
        }
    }
}

/// Decodes engine tables (rows by table name) and the world tables into an engine state, with
/// every name resolved through `registered` ([`Registered::intern`]). The result still has to pass
/// `Engine::restore`, which checks its canonical form.
///
/// # Errors
/// [`DecodeError`] for a missing or extra engine table, a malformed row, a meta table without
/// exactly one row, or a name the registered systems do not know.
pub fn decode_engine_tables(
    tables: &BTreeMap<String, Vec<Vec<u8>>>,
    registered: &Registered,
    world: WorldTables,
) -> Result<EngineState, DecodeError> {
    let extra: Vec<&String> = tables
        .keys()
        .filter(|k| !ENGINE_TABLES.contains(&k.as_str()))
        .collect();
    if !extra.is_empty() {
        return Err(DecodeError(format!("unknown engine tables {extra:?}")));
    }
    let d = Decoder { tables, registered };
    let mut r = R::new(AGENDA_META, d.one(AGENDA_META)?);
    let next_timer = r.u64()?;
    r.end()?;
    let mut r = R::new(ENGINE_META, d.one(ENGINE_META)?);
    let cursor = if r.flag()? {
        Some(SimTime::from_minutes(r.i64()?))
    } else {
        None
    };
    let timeline = TimelineId::from_raw(r.u32()?);
    let applied_inputs = r.u64()?;
    let next_command = r.u64()?;
    r.end()?;
    let (entries, tombstones) = d.timers()?;
    Ok(EngineState {
        cursor,
        timeline,
        applied_inputs,
        tables: world,
        agenda: AgendaState {
            entries,
            next_seq: next_timer,
            tombstones,
        },
        commands: CommandQueuesState {
            queues: d.commands()?,
            next_seq: next_command,
        },
        epochs: d.epochs()?,
        inputs: d.inputs()?,
        events: d.events()?,
    })
}

/// The agenda's entries and tombstones, as decoded.
type Timers = (Vec<(AgendaKey, Timer)>, Vec<TimerId>);

/// The engine tables being decoded and the names they may use.
struct Decoder<'a> {
    tables: &'a BTreeMap<String, Vec<Vec<u8>>>,
    registered: &'a Registered,
}

impl Decoder<'_> {
    fn rows(&self, name: &'static str) -> Result<&Vec<Vec<u8>>, DecodeError> {
        self.tables
            .get(name)
            .ok_or_else(|| DecodeError(format!("missing table {name}")))
    }

    fn one(&self, name: &'static str) -> Result<&Vec<u8>, DecodeError> {
        match self.rows(name)?.as_slice() {
            [row] => Ok(row),
            other => Err(DecodeError(format!(
                "{name} has {} rows, not 1",
                other.len()
            ))),
        }
    }

    fn intern(&self, table: &'static str, name: &str) -> Result<&'static str, DecodeError> {
        self.registered
            .intern(name)
            .ok_or_else(|| DecodeError(format!("{table}: {name:?} is not a registered name")))
    }

    fn timers(&self) -> Result<Timers, DecodeError> {
        let mut entries = Vec::new();
        let mut tombstones = Vec::new();
        for row in self.rows(AGENDA_TIMERS)? {
            let mut r = R::new(AGENDA_TIMERS, row);
            let key = AgendaKey {
                time: SimTime::from_minutes(r.i64()?),
                phase: r.phase()?,
                entity: r.entity()?,
                seq: r.u64()?,
            };
            let timer = Timer {
                owner: SystemId::from_raw(r.u32()?),
                purpose: PurposeId::from_raw(r.u64()?),
                kind: r.u32()?,
                payload: r.i64()?,
            };
            if r.flag()? {
                tombstones.push(TimerId::from_raw(key.seq));
            }
            r.end()?;
            entries.push((key, timer));
        }
        Ok((entries, tombstones))
    }

    fn commands(&self) -> Result<Vec<(&'static str, Vec<QueuedCommand>)>, DecodeError> {
        let mut queues: Vec<(&'static str, Vec<QueuedCommand>)> = Vec::new();
        for row in self.rows(ENGINE_COMMANDS)? {
            let mut r = R::new(ENGINE_COMMANDS, row);
            let owner = self.intern(ENGINE_COMMANDS, &r.str()?)?;
            let tick = SimTime::from_minutes(r.i64()?);
            let phase = r.phase()?;
            let person = PersonId::from_raw(r.u64()?);
            let seq = r.u64()?;
            let sender = self.intern(ENGINE_COMMANDS, &r.str()?)?;
            let cause = if r.flag()? {
                Some(EventId::from_raw(r.u64()?))
            } else {
                None
            };
            let kind = self.intern(ENGINE_COMMANDS, &r.str()?)?;
            let args = r.list_i64()?;
            let draft = if r.flag()? {
                Some(DraftRef(usize::try_from(r.u64()?).unwrap_or(usize::MAX)))
            } else {
                None
            };
            r.end()?;
            let entry = QueuedCommand {
                tick,
                phase,
                seq,
                sender,
                cause,
                command: Command {
                    kind,
                    person,
                    args,
                    cause: draft,
                },
            };
            match queues.last_mut() {
                Some((current, queue)) if *current == owner => queue.push(entry),
                _ => queues.push((owner, vec![entry])),
            }
        }
        Ok(queues)
    }

    fn epochs(&self) -> Result<Vec<(EntityRef, u32)>, DecodeError> {
        self.rows(ENGINE_EPOCHS)?
            .iter()
            .map(|row| {
                let mut r = R::new(ENGINE_EPOCHS, row);
                let pair = (r.entity()?, r.u32()?);
                r.end()?;
                Ok(pair)
            })
            .collect()
    }

    fn inputs(&self) -> Result<Vec<(SimTime, Vec<MaterialInput>)>, DecodeError> {
        let mut inputs: Vec<(SimTime, Vec<MaterialInput>)> = Vec::new();
        for row in self.rows(ENGINE_INPUTS)? {
            let mut r = R::new(ENGINE_INPUTS, row);
            let at = SimTime::from_minutes(r.i64()?);
            let index = r.u32()? as usize;
            let count = r.count()?;
            let scope = (0..count).map(|_| r.entity()).collect::<Result<_, _>>()?;
            r.end()?;
            let input = MaterialInput { scope };
            match inputs.last_mut() {
                Some((previous, list)) if *previous == at && list.len() == index => {
                    list.push(input);
                }
                _ if index == 0 => inputs.push((at, vec![input])),
                _ => {
                    return Err(DecodeError(format!(
                        "{ENGINE_INPUTS}: input {index} at {at} is out of order"
                    )));
                }
            }
        }
        Ok(inputs)
    }

    fn events(&self) -> Result<EventLogState, DecodeError> {
        let mut events = Vec::new();
        for row in self.rows(EVENTS_HELD)? {
            let mut r = R::new(EVENTS_HELD, row);
            let id = EventId::from_raw(r.u64()?);
            let tick = r.i64()?;
            let phase = r.u8()?;
            let emitter = self.intern(EVENTS_HELD, &r.str()?)?;
            let kind = self.intern(EVENTS_HELD, &r.str()?)?;
            let count = r.count()?;
            let subjects = (0..count).map(|_| r.entity()).collect::<Result<_, _>>()?;
            let count = r.count()?;
            let causes = (0..count)
                .map(|_| r.u64().map(EventId::from_raw))
                .collect::<Result<_, _>>()?;
            let payload = r.list_i64()?;
            r.end()?;
            events.push(Event {
                id,
                tick,
                phase,
                emitter,
                kind,
                subjects,
                causes,
                payload,
            });
        }
        let mut r = R::new(EVENTS_META, self.one(EVENTS_META)?);
        let next_seq = r.u64()?;
        let last = if r.flag()? {
            Some((r.i64()?, r.u8()?))
        } else {
            None
        };
        let running = r.u64()?;
        r.end()?;
        Ok(EventLogState {
            events,
            next_seq,
            last,
            running,
        })
    }
}
