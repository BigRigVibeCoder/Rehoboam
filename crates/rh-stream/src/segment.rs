//! Channel-A segments: records to Parquet tables and back (`specs/stream/channel_a_v0.sql`).
//!
//! A segment is one sim-year of one timeline's channel A (R5): every table of [`CHANNEL_A`],
//! empty or not, the dictionaries in force (`record_kind`, `verb`, `act_class`, R6) and its
//! `segment` row, written last. [`encode`] turns the records into the files, in the order they
//! are sealed; [`decode`] reads them back into the records and checks everything a reader can
//! check: the exact table set, the schema of every file, each table's kind code, contiguous `seq`
//! from `first_seq`, the R4 order, child rows that hang off an event of the segment with
//! contiguous ordinals, the verb pair of every event against the dictionary, and the canonical
//! digest recomputed from the records.
//!
//! The table set is data ([`CHANNEL_A`]), so a format that adds a child table (the SPR-020
//! envelope, PR #61) adds an entry and its rows, not a new writer.
//!
//! REF: `specs/stream/channel_a_v0.sql` R1–R7; SPR-011 rulings 11–13.

#![allow(
    clippy::many_single_char_names,
    reason = "column-to-field decoding: one short name per column of a DDL table, in DDL order"
)]

use std::collections::{BTreeMap, BTreeSet};

use crate::StreamError;
use crate::canonical;
use crate::parquet_io::{Column, ColumnSpec, ColumnType as T, read_table, write_table};
use crate::record::{
    Body, Cause, Event, Header, Input, Keyframe, Kind, LifeEntry, LifeExit, Record, Role,
    StepDigest, sim_year,
};

/// `segment.format` of channel A v0.
pub const FORMAT_A: &str = "rehoboam/stream/channel-a/v0";
/// The file of the segment row, written last.
pub const SEGMENT_FILE: &str = "segment.parquet";

const fn c(name: &'static str, ty: T) -> ColumnSpec {
    ColumnSpec { name, ty }
}

/// One table of a segment format: its name (the file is `<name>.parquet`) and columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableSpec {
    /// The DDL's table name.
    pub name: &'static str,
    /// Its columns, in DDL order.
    pub columns: &'static [ColumnSpec],
}

impl TableSpec {
    /// The table's file name in a segment directory.
    #[must_use]
    pub fn file(&self) -> String {
        format!("{}.parquet", self.name)
    }
}

/// `record_kind`.
pub const RECORD_KIND: TableSpec = TableSpec {
    name: "record_kind",
    columns: &[c("kind", T::U16), c("name", T::Str), c("class", T::U8)],
};
/// `verb`.
pub const VERB: TableSpec = TableSpec {
    name: "verb",
    columns: &[
        c("verb_id", T::U16),
        c("name", T::Str),
        c("act_classes", T::U64),
        c("tellability", T::U8),
        c("emitted_by", T::Str),
        c("aggregate", T::Bool),
    ],
};
/// `act_class`.
pub const ACT_CLASS: TableSpec = TableSpec {
    name: "act_class",
    columns: &[c("act_class_id", T::U8), c("name", T::Str)],
};
/// `segment` (channel A).
pub const SEGMENT_A: TableSpec = TableSpec {
    name: "segment",
    columns: &[
        c("timeline_id", T::U32),
        c("channel", T::Str),
        c("sim_year", T::I32),
        c("format", T::Str),
        c("first_seq", T::U64),
        c("last_seq", T::U64),
        c("record_count", T::U64),
        c("first_tick", T::I64),
        c("last_tick", T::I64),
        c("engine_build", T::Str),
        c("registry_version", T::Str),
        c("manifest_hash", T::Blob),
        c("world_seed", T::U64),
        c("built_systems", T::ListStr),
        c("not_built", T::ListStr),
        c("canonical_digest", T::Blob),
    ],
};
/// `input`.
pub const INPUT: TableSpec = TableSpec {
    name: "input",
    columns: &[
        c("timeline_id", T::U32),
        c("seq", T::U64),
        c("tick", T::I64),
        c("kind", T::U16),
        c("journal_seq", T::U64),
        c("class", T::U8),
        c("input_kind", T::Str),
        c("scope", T::Str),
        c("scope_id", T::OptU64),
        c("producer", T::Str),
        c("payload_hash", T::Blob),
        c("row_digest", T::Blob),
    ],
};
/// `event`.
pub const EVENT: TableSpec = TableSpec {
    name: "event",
    columns: &[
        c("timeline_id", T::U32),
        c("seq", T::U64),
        c("tick", T::I64),
        c("kind", T::U16),
        c("event_id", T::U64),
        c("phase", T::U8),
        c("verb_id", T::U16),
        c("act_classes", T::U64),
        c("place_id", T::OptU32),
        c("magnitude", T::I32),
        c("tellability", T::U8),
        c("secrecy", T::U8),
        c("origin", T::U8),
        c("payload", T::ListI64),
    ],
};
/// `event_role`.
pub const EVENT_ROLE: TableSpec = TableSpec {
    name: "event_role",
    columns: &[
        c("timeline_id", T::U32),
        c("event_id", T::U64),
        c("ordinal", T::U8),
        c("entity_kind", T::U8),
        c("entity_id", T::U64),
        c("role", T::U8),
    ],
};
/// `causal_edge`.
pub const CAUSAL_EDGE: TableSpec = TableSpec {
    name: "causal_edge",
    columns: &[
        c("timeline_id", T::U32),
        c("cause_id", T::U64),
        c("effect_id", T::U64),
        c("kind", T::U8),
    ],
};
/// `life_course_entry`.
pub const LIFE_ENTRY: TableSpec = TableSpec {
    name: "life_course_entry",
    columns: &[
        c("timeline_id", T::U32),
        c("seq", T::U64),
        c("tick", T::I64),
        c("kind", T::U16),
        c("person_id", T::U64),
        c("origin", T::U8),
        c("sex", T::U8),
        c("mother_id", T::U64),
        c("father_id", T::U64),
        c("birth_tick", T::I64),
    ],
};
/// `life_course_exit`.
pub const LIFE_EXIT: TableSpec = TableSpec {
    name: "life_course_exit",
    columns: &[
        c("timeline_id", T::U32),
        c("seq", T::U64),
        c("tick", T::I64),
        c("kind", T::U16),
        c("person_id", T::U64),
        c("exit_kind", T::U8),
        c("death_cause", T::U8),
        c("event_id", T::OptU64),
    ],
};
/// `step_digest`.
pub const STEP_DIGEST: TableSpec = TableSpec {
    name: "step_digest",
    columns: &[
        c("timeline_id", T::U32),
        c("seq", T::U64),
        c("tick", T::I64),
        c("kind", T::U16),
        c("living", T::U64),
        c("births", T::U32),
        c("deaths", T::U32),
        c("state_hash", T::U64),
        c("observation_hash", T::U64),
        c("narrative_hash", T::U64),
    ],
};
/// `keyframe_marker`.
pub const KEYFRAME_MARKER: TableSpec = TableSpec {
    name: "keyframe_marker",
    columns: &[
        c("timeline_id", T::U32),
        c("seq", T::U64),
        c("tick", T::I64),
        c("kind", T::U16),
        c("keyframe_format", T::Str),
        c("keyframe_path", T::Str),
        c("state_hash", T::U64),
        c("journal_position", T::U64),
    ],
};

/// Every table of a channel-A v0 segment, in the order they are sealed: the dictionaries, the
/// record and child tables, and `segment` last (R5).
pub const CHANNEL_A: [TableSpec; 12] = [
    ACT_CLASS,
    CAUSAL_EDGE,
    EVENT,
    EVENT_ROLE,
    INPUT,
    KEYFRAME_MARKER,
    LIFE_ENTRY,
    LIFE_EXIT,
    RECORD_KIND,
    STEP_DIGEST,
    VERB,
    SEGMENT_A,
];

/// One `verb` row: a Registry verb at sealing (R6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerbEntry {
    /// The verb's index in `[verbs]` sorted by name.
    pub verb_id: u16,
    /// Its name.
    pub name: String,
    /// Its act classes as a bitset over `act_class.act_class_id`.
    pub act_classes: u64,
    /// Its Registry tellability prior.
    pub tellability: u8,
    /// The system that emits it.
    pub emitted_by: String,
    /// Whether it is an aggregate verb.
    pub aggregate: bool,
}

/// The dictionaries a segment carries (R6): the Registry's verbs and act classes at sealing.
/// The host builds them from the Registry (a tool crate); plane A only carries them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dictionaries {
    /// The Registry version.
    pub registry_version: String,
    /// `[verbs]` sorted by name: `verbs[i].verb_id == i`.
    pub verbs: Vec<VerbEntry>,
    /// `[act_classes].ids` in list order: bit i is `act_classes[i]`.
    pub act_classes: Vec<String>,
}

impl Dictionaries {
    /// The verb named `name`.
    #[must_use]
    pub fn verb(&self, name: &str) -> Option<&VerbEntry> {
        self.verbs
            .binary_search_by(|v| v.name.as_bytes().cmp(name.as_bytes()))
            .ok()
            .and_then(|i| self.verbs.get(i))
    }

    /// The verb with id `verb_id`.
    #[must_use]
    pub fn verb_by_id(&self, verb_id: u16) -> Option<&VerbEntry> {
        self.verbs.get(usize::from(verb_id))
    }

    /// `Err` unless the verbs are in name order with `verb_id` their index, the act classes fit
    /// a 64-bit set, and every verb's classes are within it (ruling 11, R6).
    ///
    /// # Errors
    /// [`StreamError::Dictionary`].
    pub fn check(&self) -> Result<(), StreamError> {
        if self.act_classes.len() > 64 {
            return Err(StreamError::Dictionary(format!(
                "{} act classes do not fit a 64-bit set",
                self.act_classes.len()
            )));
        }
        let all = if self.act_classes.len() == 64 {
            u64::MAX
        } else {
            (1u64 << self.act_classes.len()) - 1
        };
        for (i, v) in self.verbs.iter().enumerate() {
            if usize::from(v.verb_id) != i {
                return Err(StreamError::Dictionary(format!(
                    "verb {} has id {}, not its index {i}",
                    v.name, v.verb_id
                )));
            }
            if v.act_classes & !all != 0 {
                return Err(StreamError::Dictionary(format!(
                    "verb {} names an act class outside the dictionary",
                    v.name
                )));
            }
        }
        if self
            .verbs
            .windows(2)
            .any(|w| w[0].name.as_bytes() >= w[1].name.as_bytes())
        {
            return Err(StreamError::Dictionary(
                "verbs are not in strictly ascending name order".to_owned(),
            ));
        }
        Ok(())
    }
}

/// The history tuple a segment carries (`05.1` §6 item 1; ruling 3), less the Registry version,
/// which is the dictionaries'.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    /// The engine build.
    pub engine_build: String,
    /// The manifest's hash (the SHA-256 bytes).
    pub manifest_hash: Vec<u8>,
    /// The world seed.
    pub world_seed: u64,
    /// The systems this build ran.
    pub built_systems: Vec<String>,
    /// The manifest's systems it skipped.
    pub not_built: Vec<String>,
}

/// A channel-A segment's `segment` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentRow {
    /// The timeline.
    pub timeline_id: u32,
    /// The sim-year.
    pub sim_year: i32,
    /// The first record's seq.
    pub first_seq: u64,
    /// The last record's seq.
    pub last_seq: u64,
    /// The record count.
    pub record_count: u64,
    /// The first record's tick.
    pub first_tick: i64,
    /// The last record's tick.
    pub last_tick: i64,
    /// The history tuple.
    pub provenance: Provenance,
    /// The Registry version.
    pub registry_version: String,
    /// The canonical digest of the records (R7).
    pub canonical_digest: [u8; 16],
}

/// A decoded channel-A segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentA {
    /// Its `segment` row.
    pub row: SegmentRow,
    /// Its dictionaries.
    pub dictionaries: Dictionaries,
    /// Its records, in seq order.
    pub records: Vec<Record>,
}

fn bad(message: impl Into<String>) -> StreamError {
    StreamError::Segment(message.into())
}

/// Checks that `records` can be one channel-A segment of sim-year `year`: non-empty, one
/// timeline, channel A, contiguous `seq`, each in the year, and in strictly increasing R4 order.
fn check_records(records: &[Record], year: i64, minutes_per_year: i64) -> Result<(), StreamError> {
    let Some(first) = records.first() else {
        return Err(bad("a channel-A segment holds at least one record"));
    };
    for (i, r) in records.iter().enumerate() {
        if r.kind().channel() != crate::record::Channel::A {
            return Err(bad(format!("a {} record in channel A", r.kind().name())));
        }
        if r.header.timeline_id != first.header.timeline_id {
            return Err(bad("records of two timelines in one segment"));
        }
        let expected = first.header.seq + u64::try_from(i).unwrap_or(u64::MAX);
        if r.header.seq != expected {
            return Err(bad(format!(
                "seq {} where {expected} is due: seq is contiguous (R3)",
                r.header.seq
            )));
        }
        if sim_year(r.header.tick, minutes_per_year) != year {
            return Err(bad(format!(
                "a record at tick {} outside sim-year {year}",
                r.header.tick
            )));
        }
    }
    if let Some(w) = records
        .windows(2)
        .find(|w| w[0].order_key() >= w[1].order_key())
    {
        return Err(bad(format!(
            "seq {} and {} are out of R4 order",
            w[0].header.seq, w[1].header.seq
        )));
    }
    Ok(())
}

/// The verb pair of an event against the dictionary (`canonical_v0.md` §2): id and name agree, and
/// the event carries the verb's act classes and tellability.
fn check_verb(e: &Event, dictionaries: &Dictionaries) -> Result<(), StreamError> {
    let v = dictionaries
        .verb_by_id(e.verb_id)
        .filter(|v| v.name == e.verb)
        .ok_or_else(|| {
            bad(format!(
                "event {}: verb {} ({}) is not in the dictionary",
                e.event_id, e.verb_id, e.verb
            ))
        })?;
    if v.act_classes != e.act_classes || v.tellability != e.tellability {
        return Err(bad(format!(
            "event {}: act classes or tellability differ from verb {}'s",
            e.event_id, e.verb
        )));
    }
    Ok(())
}

/// The files of a sealed channel-A segment of sim-year `sim_year` (calendar of
/// `minutes_per_year`), in sealing order, `segment.parquet` last.
///
/// # Errors
/// [`StreamError`] if the records cannot form the segment (see [`check_records`]), an event's
/// verb is not the dictionary's, a record does not render, or Parquet fails.
pub fn encode(
    records: &[Record],
    sim_year: i64,
    minutes_per_year: i64,
    provenance: &Provenance,
    dictionaries: &Dictionaries,
) -> Result<Vec<(String, Vec<u8>)>, StreamError> {
    dictionaries.check()?;
    check_records(records, sim_year, minutes_per_year)?;
    let year =
        i32::try_from(sim_year).map_err(|_| bad(format!("sim-year {sim_year} overflows")))?;
    let mut t = Tables::default();
    for r in records {
        if let Body::Event(e) = &r.body {
            check_verb(e, dictionaries)?;
        }
        t.add(r)?;
    }
    let lines = records
        .iter()
        .map(canonical::render)
        .collect::<Result<Vec<_>, _>>()?;
    let (first, last) = match (records.first(), records.last()) {
        (Some(f), Some(l)) => (f.header, l.header),
        _ => return Err(bad("a channel-A segment holds at least one record")),
    };
    let row = SegmentRow {
        timeline_id: first.timeline_id,
        sim_year: year,
        first_seq: first.seq,
        last_seq: last.seq,
        record_count: u64::try_from(records.len()).unwrap_or(u64::MAX),
        first_tick: first.tick,
        last_tick: last.tick,
        provenance: provenance.clone(),
        registry_version: dictionaries.registry_version.clone(),
        canonical_digest: canonical::digest(&lines),
    };
    let mut tables: BTreeMap<&'static str, Vec<Column>> = t.into_columns();
    tables.insert(RECORD_KIND.name, record_kind_columns());
    tables.insert(VERB.name, verb_columns(dictionaries));
    tables.insert(ACT_CLASS.name, act_class_columns(dictionaries)?);
    tables.insert(SEGMENT_A.name, segment_columns(&row));
    CHANNEL_A
        .iter()
        .map(|spec| {
            let columns = tables
                .remove(spec.name)
                .ok_or_else(|| bad(format!("no rows built for {}", spec.name)))?;
            Ok((spec.file(), write_table(spec.name, spec.columns, &columns)?))
        })
        .collect()
}

fn record_kind_columns() -> Vec<Column> {
    vec![
        Column::U16(Kind::CHANNEL_A.iter().map(|k| k.code()).collect()),
        Column::Str(
            Kind::CHANNEL_A
                .iter()
                .map(|k| k.name().to_owned())
                .collect(),
        ),
        Column::U8(Kind::CHANNEL_A.iter().map(|k| k.class()).collect()),
    ]
}

fn verb_columns(d: &Dictionaries) -> Vec<Column> {
    vec![
        Column::U16(d.verbs.iter().map(|v| v.verb_id).collect()),
        Column::Str(d.verbs.iter().map(|v| v.name.clone()).collect()),
        Column::U64(d.verbs.iter().map(|v| v.act_classes).collect()),
        Column::U8(d.verbs.iter().map(|v| v.tellability).collect()),
        Column::Str(d.verbs.iter().map(|v| v.emitted_by.clone()).collect()),
        Column::Bool(d.verbs.iter().map(|v| v.aggregate).collect()),
    ]
}

fn act_class_columns(d: &Dictionaries) -> Result<Vec<Column>, StreamError> {
    let ids = (0..d.act_classes.len())
        .map(|i| u8::try_from(i).map_err(|_| bad("too many act classes")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(vec![Column::U8(ids), Column::Str(d.act_classes.clone())])
}

fn segment_columns(r: &SegmentRow) -> Vec<Column> {
    vec![
        Column::U32(vec![r.timeline_id]),
        Column::Str(vec!["A".to_owned()]),
        Column::I32(vec![r.sim_year]),
        Column::Str(vec![FORMAT_A.to_owned()]),
        Column::U64(vec![r.first_seq]),
        Column::U64(vec![r.last_seq]),
        Column::U64(vec![r.record_count]),
        Column::I64(vec![r.first_tick]),
        Column::I64(vec![r.last_tick]),
        Column::Str(vec![r.provenance.engine_build.clone()]),
        Column::Str(vec![r.registry_version.clone()]),
        Column::Blob(vec![r.provenance.manifest_hash.clone()]),
        Column::U64(vec![r.provenance.world_seed]),
        Column::ListStr(vec![r.provenance.built_systems.clone()]),
        Column::ListStr(vec![r.provenance.not_built.clone()]),
        Column::Blob(vec![r.canonical_digest.to_vec()]),
    ]
}

/// The record and child tables of a segment, built row by row in seq order.
#[derive(Default)]
struct Tables {
    input: Vec<(Header, Input)>,
    event: Vec<(Header, Event)>,
    entry: Vec<(Header, LifeEntry)>,
    exit: Vec<(Header, LifeExit)>,
    digest: Vec<(Header, StepDigest)>,
    keyframe: Vec<(Header, Keyframe)>,
}

impl Tables {
    fn add(&mut self, r: &Record) -> Result<(), StreamError> {
        let h = r.header;
        match &r.body {
            Body::Input(i) => self.input.push((h, i.clone())),
            Body::Event(e) => {
                if e.roles.len() > 256 {
                    return Err(bad(format!("event {}: over 256 roles", e.event_id)));
                }
                self.event.push((h, e.clone()));
            }
            Body::LifeEntry(e) => self.entry.push((h, *e)),
            Body::LifeExit(x) => self.exit.push((h, *x)),
            Body::StepDigest(d) => self.digest.push((h, *d)),
            Body::Keyframe(k) => self.keyframe.push((h, k.clone())),
            Body::Match(_) => return Err(bad("a MATCH record in channel A")),
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one block per table of the format, in DDL order: splitting it hides the layout"
    )]
    fn into_columns(self) -> BTreeMap<&'static str, Vec<Column>> {
        fn head<B>(rows: &[(Header, B)], kind: Kind) -> Vec<Column> {
            vec![
                Column::U32(rows.iter().map(|(h, _)| h.timeline_id).collect()),
                Column::U64(rows.iter().map(|(h, _)| h.seq).collect()),
                Column::I64(rows.iter().map(|(h, _)| h.tick).collect()),
                Column::U16(rows.iter().map(|_| kind.code()).collect()),
            ]
        }
        let mut m = BTreeMap::new();
        let i = &self.input;
        let mut cols = head(i, Kind::Input);
        cols.extend([
            Column::U64(i.iter().map(|(_, x)| x.journal_seq).collect()),
            Column::U8(i.iter().map(|(_, x)| x.class).collect()),
            Column::Str(i.iter().map(|(_, x)| x.input_kind.clone()).collect()),
            Column::Str(i.iter().map(|(_, x)| x.scope.clone()).collect()),
            Column::OptU64(i.iter().map(|(_, x)| x.scope_id).collect()),
            Column::Str(i.iter().map(|(_, x)| x.producer.clone()).collect()),
            Column::Blob(i.iter().map(|(_, x)| x.payload_hash.to_vec()).collect()),
            Column::Blob(i.iter().map(|(_, x)| x.row_digest.to_vec()).collect()),
        ]);
        m.insert(INPUT.name, cols);
        let e = &self.event;
        let mut cols = head(e, Kind::Event);
        cols.extend([
            Column::U64(e.iter().map(|(_, x)| x.event_id).collect()),
            Column::U8(e.iter().map(|(_, x)| x.phase).collect()),
            Column::U16(e.iter().map(|(_, x)| x.verb_id).collect()),
            Column::U64(e.iter().map(|(_, x)| x.act_classes).collect()),
            Column::OptU32(e.iter().map(|(_, x)| x.place_id).collect()),
            Column::I32(e.iter().map(|(_, x)| x.magnitude).collect()),
            Column::U8(e.iter().map(|(_, x)| x.tellability).collect()),
            Column::U8(e.iter().map(|(_, x)| x.secrecy).collect()),
            Column::U8(e.iter().map(|(_, x)| x.origin).collect()),
            Column::ListI64(e.iter().map(|(_, x)| x.payload.clone()).collect()),
        ]);
        m.insert(EVENT.name, cols);
        // Child rows: roles by (event_id, ordinal); edges by (effect_id, cause_id). Events are
        // in seq order, which is event_id order (R4), and causes ascend within each event.
        let roles: Vec<(u32, u64, u8, Role)> = e
            .iter()
            .flat_map(|(h, x)| {
                x.roles.iter().enumerate().map(move |(o, r)| {
                    (
                        h.timeline_id,
                        x.event_id,
                        u8::try_from(o).unwrap_or(u8::MAX),
                        *r,
                    )
                })
            })
            .collect();
        m.insert(
            EVENT_ROLE.name,
            vec![
                Column::U32(roles.iter().map(|r| r.0).collect()),
                Column::U64(roles.iter().map(|r| r.1).collect()),
                Column::U8(roles.iter().map(|r| r.2).collect()),
                Column::U8(roles.iter().map(|r| r.3.entity_kind).collect()),
                Column::U64(roles.iter().map(|r| r.3.entity_id).collect()),
                Column::U8(roles.iter().map(|r| r.3.role).collect()),
            ],
        );
        let edges: Vec<(u32, Cause, u64)> = e
            .iter()
            .flat_map(|(h, x)| {
                x.causes
                    .iter()
                    .map(move |c| (h.timeline_id, *c, x.event_id))
            })
            .collect();
        m.insert(
            CAUSAL_EDGE.name,
            vec![
                Column::U32(edges.iter().map(|r| r.0).collect()),
                Column::U64(edges.iter().map(|r| r.1.cause_id).collect()),
                Column::U64(edges.iter().map(|r| r.2).collect()),
                Column::U8(edges.iter().map(|r| r.1.kind).collect()),
            ],
        );
        let n = &self.entry;
        let mut cols = head(n, Kind::LifeEntry);
        cols.extend([
            Column::U64(n.iter().map(|(_, x)| x.person_id).collect()),
            Column::U8(n.iter().map(|(_, x)| x.origin).collect()),
            Column::U8(n.iter().map(|(_, x)| x.sex).collect()),
            Column::U64(n.iter().map(|(_, x)| x.mother_id).collect()),
            Column::U64(n.iter().map(|(_, x)| x.father_id).collect()),
            Column::I64(n.iter().map(|(_, x)| x.birth_tick).collect()),
        ]);
        m.insert(LIFE_ENTRY.name, cols);
        let x = &self.exit;
        let mut cols = head(x, Kind::LifeExit);
        cols.extend([
            Column::U64(x.iter().map(|(_, r)| r.person_id).collect()),
            Column::U8(x.iter().map(|(_, r)| r.exit_kind).collect()),
            Column::U8(x.iter().map(|(_, r)| r.death_cause).collect()),
            Column::OptU64(x.iter().map(|(_, r)| r.event_id).collect()),
        ]);
        m.insert(LIFE_EXIT.name, cols);
        let d = &self.digest;
        let mut cols = head(d, Kind::StepDigest);
        cols.extend([
            Column::U64(d.iter().map(|(_, r)| r.living).collect()),
            Column::U32(d.iter().map(|(_, r)| r.births).collect()),
            Column::U32(d.iter().map(|(_, r)| r.deaths).collect()),
            Column::U64(d.iter().map(|(_, r)| r.state_hash).collect()),
            Column::U64(d.iter().map(|(_, r)| r.observation_hash).collect()),
            Column::U64(d.iter().map(|(_, r)| r.narrative_hash).collect()),
        ]);
        m.insert(STEP_DIGEST.name, cols);
        let k = &self.keyframe;
        let mut cols = head(k, Kind::Keyframe);
        cols.extend([
            Column::Str(k.iter().map(|(_, r)| r.keyframe_format.clone()).collect()),
            Column::Str(k.iter().map(|(_, r)| r.keyframe_path.clone()).collect()),
            Column::U64(k.iter().map(|(_, r)| r.state_hash).collect()),
            Column::U64(k.iter().map(|(_, r)| r.journal_position).collect()),
        ]);
        m.insert(KEYFRAME_MARKER.name, cols);
        m
    }
}

// ------------------------------------------------------------------------------------- decode

fn col_u8(c: &Column) -> Result<&[u8], StreamError> {
    match c {
        Column::U8(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_u16(c: &Column) -> Result<&[u16], StreamError> {
    match c {
        Column::U16(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_u32(c: &Column) -> Result<&[u32], StreamError> {
    match c {
        Column::U32(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_u64(c: &Column) -> Result<&[u64], StreamError> {
    match c {
        Column::U64(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_i32(c: &Column) -> Result<&[i32], StreamError> {
    match c {
        Column::I32(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_i64(c: &Column) -> Result<&[i64], StreamError> {
    match c {
        Column::I64(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_bool(c: &Column) -> Result<&[bool], StreamError> {
    match c {
        Column::Bool(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_str(c: &Column) -> Result<&[String], StreamError> {
    match c {
        Column::Str(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_blob(c: &Column) -> Result<&[Vec<u8>], StreamError> {
    match c {
        Column::Blob(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_list_i64(c: &Column) -> Result<&[Vec<i64>], StreamError> {
    match c {
        Column::ListI64(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_list_str(c: &Column) -> Result<&[Vec<String>], StreamError> {
    match c {
        Column::ListStr(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_opt_u32(c: &Column) -> Result<&[Option<u32>], StreamError> {
    match c {
        Column::OptU32(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}
fn col_opt_u64(c: &Column) -> Result<&[Option<u64>], StreamError> {
    match c {
        Column::OptU64(v) => Ok(v),
        _ => Err(bad("a column of the wrong type")),
    }
}

fn blob16(v: &[u8]) -> Result<[u8; 16], StreamError> {
    <[u8; 16]>::try_from(v).map_err(|_| bad("a 16-byte blob of another length"))
}

/// The headers of a record table's rows, checking its kind code.
fn headers(cols: &[Column], kind: Kind) -> Result<Vec<Header>, StreamError> {
    let (tl, seq, tick, k) = (
        col_u32(&cols[0])?,
        col_u64(&cols[1])?,
        col_i64(&cols[2])?,
        col_u16(&cols[3])?,
    );
    if k.iter().any(|&x| x != kind.code()) {
        return Err(bad(format!("a {} row with another kind code", kind.name())));
    }
    Ok((0..tl.len())
        .map(|i| Header {
            timeline_id: tl[i],
            seq: seq[i],
            tick: tick[i],
        })
        .collect())
}

/// Reads a segment's files (by file name) back into its row, dictionaries and records, with every
/// check of the module docs.
///
/// # Errors
/// [`StreamError`] naming the first problem.
pub fn decode(
    files: &BTreeMap<String, Vec<u8>>,
    minutes_per_year: i64,
) -> Result<SegmentA, StreamError> {
    let expected: BTreeSet<String> = CHANNEL_A.iter().map(TableSpec::file).collect();
    let present: BTreeSet<String> = files.keys().cloned().collect();
    if present != expected {
        return Err(bad(format!(
            "the segment's files are {present:?}, not the format's {expected:?}"
        )));
    }
    let mut t: BTreeMap<&str, Vec<Column>> = BTreeMap::new();
    for spec in CHANNEL_A {
        let bytes = files
            .get(&spec.file())
            .ok_or_else(|| bad(format!("no {}", spec.file())))?;
        t.insert(spec.name, read_table(spec.name, spec.columns, bytes)?);
    }
    let table = |name: &str| t.get(name).ok_or_else(|| bad(format!("no table {name}")));
    let row = decode_row(table(SEGMENT_A.name)?)?;
    let dictionaries = decode_dictionaries(
        table(VERB.name)?,
        table(ACT_CLASS.name)?,
        table(RECORD_KIND.name)?,
        &row.registry_version,
    )?;
    let mut records = Vec::new();
    decode_inputs(table(INPUT.name)?, &mut records)?;
    decode_events(
        table(EVENT.name)?,
        table(EVENT_ROLE.name)?,
        table(CAUSAL_EDGE.name)?,
        &dictionaries,
        row.timeline_id,
        &mut records,
    )?;
    decode_life(
        table(LIFE_ENTRY.name)?,
        table(LIFE_EXIT.name)?,
        &mut records,
    )?;
    decode_digests(table(STEP_DIGEST.name)?, &mut records)?;
    decode_keyframes(table(KEYFRAME_MARKER.name)?, &mut records)?;
    records.sort_by_key(|r| r.header.seq);
    check_records(&records, i64::from(row.sim_year), minutes_per_year)?;
    let (first, last) = match (records.first(), records.last()) {
        (Some(f), Some(l)) => (f.header, l.header),
        _ => return Err(bad("a sealed channel-A segment with no record")),
    };
    let count = u64::try_from(records.len()).unwrap_or(u64::MAX);
    let consistent = row.first_seq == first.seq
        && row.last_seq == last.seq
        && row.record_count == count
        && row.first_tick == first.tick
        && row.last_tick == last.tick
        && first.timeline_id == row.timeline_id;
    if !consistent {
        return Err(bad("the segment row disagrees with its records"));
    }
    let digest = canonical::digest_records(&records)?;
    if digest != row.canonical_digest {
        return Err(bad(format!(
            "canonical digest {} recomputed, {} stored",
            canonical::hex16(&digest),
            canonical::hex16(&row.canonical_digest)
        )));
    }
    Ok(SegmentA {
        row,
        dictionaries,
        records,
    })
}

fn decode_row(c: &[Column]) -> Result<SegmentRow, StreamError> {
    if c[0].len() != 1 {
        return Err(bad("the segment table holds other than one row"));
    }
    if col_str(&c[1])?[0] != "A" || col_str(&c[3])?[0] != FORMAT_A {
        return Err(bad("not a channel-A v0 segment row"));
    }
    let sim_year = col_i32(&c[2])?[0];
    if sim_year < 0 {
        return Err(bad("a negative sim-year"));
    }
    Ok(SegmentRow {
        timeline_id: col_u32(&c[0])?[0],
        sim_year,
        first_seq: col_u64(&c[4])?[0],
        last_seq: col_u64(&c[5])?[0],
        record_count: col_u64(&c[6])?[0],
        first_tick: col_i64(&c[7])?[0],
        last_tick: col_i64(&c[8])?[0],
        provenance: Provenance {
            engine_build: col_str(&c[9])?[0].clone(),
            manifest_hash: col_blob(&c[11])?[0].clone(),
            world_seed: col_u64(&c[12])?[0],
            built_systems: col_list_str(&c[13])?[0].clone(),
            not_built: col_list_str(&c[14])?[0].clone(),
        },
        registry_version: col_str(&c[10])?[0].clone(),
        canonical_digest: blob16(&col_blob(&c[15])?[0])?,
    })
}

fn decode_dictionaries(
    verb: &[Column],
    act: &[Column],
    kinds: &[Column],
    registry_version: &str,
) -> Result<Dictionaries, StreamError> {
    if kinds.to_vec() != record_kind_columns() {
        return Err(bad("record_kind is not format v0's"));
    }
    let (ids, names, classes, tell, by, agg) = (
        col_u16(&verb[0])?,
        col_str(&verb[1])?,
        col_u64(&verb[2])?,
        col_u8(&verb[3])?,
        col_str(&verb[4])?,
        col_bool(&verb[5])?,
    );
    let verbs = (0..ids.len())
        .map(|i| VerbEntry {
            verb_id: ids[i],
            name: names[i].clone(),
            act_classes: classes[i],
            tellability: tell[i],
            emitted_by: by[i].clone(),
            aggregate: agg[i],
        })
        .collect();
    let act_ids = col_u8(&act[0])?;
    if act_ids
        .iter()
        .enumerate()
        .any(|(i, &x)| usize::from(x) != i)
    {
        return Err(bad("act_class ids are not 0..n"));
    }
    let d = Dictionaries {
        registry_version: registry_version.to_owned(),
        verbs,
        act_classes: col_str(&act[1])?.to_vec(),
    };
    d.check()?;
    Ok(d)
}

fn decode_inputs(c: &[Column], out: &mut Vec<Record>) -> Result<(), StreamError> {
    let h = headers(c, Kind::Input)?;
    let (js, cl, ik, sc, sid, pr, ph, rd) = (
        col_u64(&c[4])?,
        col_u8(&c[5])?,
        col_str(&c[6])?,
        col_str(&c[7])?,
        col_opt_u64(&c[8])?,
        col_str(&c[9])?,
        col_blob(&c[10])?,
        col_blob(&c[11])?,
    );
    for (i, header) in h.into_iter().enumerate() {
        out.push(Record {
            header,
            body: Body::Input(Input {
                journal_seq: js[i],
                class: cl[i],
                input_kind: ik[i].clone(),
                scope: sc[i].clone(),
                scope_id: sid[i],
                producer: pr[i].clone(),
                payload_hash: blob16(&ph[i])?,
                row_digest: blob16(&rd[i])?,
            }),
        });
    }
    Ok(())
}

fn decode_events(
    c: &[Column],
    roles: &[Column],
    edges: &[Column],
    dictionaries: &Dictionaries,
    timeline: u32,
    out: &mut Vec<Record>,
) -> Result<(), StreamError> {
    let h = headers(c, Kind::Event)?;
    let (id, ph, vb, ac, pl, mg, tl, se, or, pa) = (
        col_u64(&c[4])?,
        col_u8(&c[5])?,
        col_u16(&c[6])?,
        col_u64(&c[7])?,
        col_opt_u32(&c[8])?,
        col_i32(&c[9])?,
        col_u8(&c[10])?,
        col_u8(&c[11])?,
        col_u8(&c[12])?,
        col_list_i64(&c[13])?,
    );
    let mut by_event: BTreeMap<u64, (Vec<Role>, Vec<Cause>)> =
        id.iter().map(|&e| (e, (Vec::new(), Vec::new()))).collect();
    if by_event.len() != id.len() {
        return Err(bad("an event id appears twice"));
    }
    let (rt, re, ro, rk, ri, rr) = (
        col_u32(&roles[0])?,
        col_u64(&roles[1])?,
        col_u8(&roles[2])?,
        col_u8(&roles[3])?,
        col_u64(&roles[4])?,
        col_u8(&roles[5])?,
    );
    for i in 0..rt.len() {
        let slot = by_event
            .get_mut(&re[i])
            .filter(|_| rt[i] == timeline)
            .ok_or_else(|| {
                bad(format!(
                    "an event_role row of no event of the segment ({})",
                    re[i]
                ))
            })?;
        if usize::from(ro[i]) != slot.0.len() {
            return Err(bad(format!(
                "event {}: role ordinals are not contiguous from 0, in order",
                re[i]
            )));
        }
        slot.0.push(Role {
            entity_kind: rk[i],
            entity_id: ri[i],
            role: rr[i],
        });
    }
    let (et, ec, ee, ek) = (
        col_u32(&edges[0])?,
        col_u64(&edges[1])?,
        col_u64(&edges[2])?,
        col_u8(&edges[3])?,
    );
    for i in 0..et.len() {
        let slot = by_event
            .get_mut(&ee[i])
            .filter(|_| et[i] == timeline)
            .ok_or_else(|| {
                bad(format!(
                    "a causal_edge of no event of the segment ({})",
                    ee[i]
                ))
            })?;
        slot.1.push(Cause {
            cause_id: ec[i],
            kind: ek[i],
        });
    }
    for (i, header) in h.into_iter().enumerate() {
        let verb = dictionaries.verb_by_id(vb[i]).ok_or_else(|| {
            bad(format!(
                "event {}: verb id {} is not in the dictionary",
                id[i], vb[i]
            ))
        })?;
        let (roles, causes) = by_event.remove(&id[i]).unwrap_or_default();
        let e = Event {
            event_id: id[i],
            phase: ph[i],
            verb_id: vb[i],
            verb: verb.name.clone(),
            act_classes: ac[i],
            place_id: pl[i],
            magnitude: mg[i],
            tellability: tl[i],
            secrecy: se[i],
            origin: or[i],
            payload: pa[i].clone(),
            roles,
            causes,
        };
        check_verb(&e, dictionaries)?;
        out.push(Record {
            header,
            body: Body::Event(e),
        });
    }
    Ok(())
}

fn decode_life(
    entry: &[Column],
    exit: &[Column],
    out: &mut Vec<Record>,
) -> Result<(), StreamError> {
    let h = headers(entry, Kind::LifeEntry)?;
    let (p, o, s, m, f, b) = (
        col_u64(&entry[4])?,
        col_u8(&entry[5])?,
        col_u8(&entry[6])?,
        col_u64(&entry[7])?,
        col_u64(&entry[8])?,
        col_i64(&entry[9])?,
    );
    for (i, header) in h.into_iter().enumerate() {
        out.push(Record {
            header,
            body: Body::LifeEntry(LifeEntry {
                person_id: p[i],
                origin: o[i],
                sex: s[i],
                mother_id: m[i],
                father_id: f[i],
                birth_tick: b[i],
            }),
        });
    }
    let h = headers(exit, Kind::LifeExit)?;
    let (p, k, c, e) = (
        col_u64(&exit[4])?,
        col_u8(&exit[5])?,
        col_u8(&exit[6])?,
        col_opt_u64(&exit[7])?,
    );
    for (i, header) in h.into_iter().enumerate() {
        out.push(Record {
            header,
            body: Body::LifeExit(LifeExit {
                person_id: p[i],
                exit_kind: k[i],
                death_cause: c[i],
                event_id: e[i],
            }),
        });
    }
    Ok(())
}

fn decode_digests(c: &[Column], out: &mut Vec<Record>) -> Result<(), StreamError> {
    let h = headers(c, Kind::StepDigest)?;
    let (l, b, d, s, o, n) = (
        col_u64(&c[4])?,
        col_u32(&c[5])?,
        col_u32(&c[6])?,
        col_u64(&c[7])?,
        col_u64(&c[8])?,
        col_u64(&c[9])?,
    );
    for (i, header) in h.into_iter().enumerate() {
        out.push(Record {
            header,
            body: Body::StepDigest(StepDigest {
                living: l[i],
                births: b[i],
                deaths: d[i],
                state_hash: s[i],
                observation_hash: o[i],
                narrative_hash: n[i],
            }),
        });
    }
    Ok(())
}

fn decode_keyframes(c: &[Column], out: &mut Vec<Record>) -> Result<(), StreamError> {
    let h = headers(c, Kind::Keyframe)?;
    let (f, p, s, j) = (
        col_str(&c[4])?,
        col_str(&c[5])?,
        col_u64(&c[6])?,
        col_u64(&c[7])?,
    );
    for (i, header) in h.into_iter().enumerate() {
        out.push(Record {
            header,
            body: Body::Keyframe(Keyframe {
                keyframe_format: f[i].clone(),
                keyframe_path: p[i].clone(),
                state_hash: s[i],
                journal_position: j[i],
            }),
        });
    }
    Ok(())
}
