//! Sifting sealed channel A into channel-C segments (`specs/stream/channel_c_v0.sql`; SPR-011
//! ruling 14; SPR-013 T-11).
//!
//! **Matching.** Channel A is read in `seq` order, segment by segment. Every `EVENT` is tested
//! against every pattern: its verb (or one of its act classes) is the clause's, and each role the
//! clause names is filled by an entity of the event's roles; an event with several entities in one
//! role completes one match per combination (a `DEATH` with two targets gives two `life_ends`
//! matches). A match is complete at once (one clause), with `opened_tick` = its tick, and its
//! `match_id` is content-addressed (R5), so a re-sift and the SQL twin agree on it without sharing
//! state.
//!
//! **Segments.** One channel-C segment per channel-A sim-year, a year with no match included (R7:
//! `tail` can tell "no match" from "not sifted yet"), its records in R4 order (closing seq, pack,
//! pattern, version, event ids, bindings), `seq` contiguous from 0 across channel C. Each carries
//! the pattern dictionary, the provenance of the channel A it was derived from, that segment's
//! canonical digest and the chain of every channel-A digest up to it, and its own canonical
//! digest. Sealing is `rh-stream`'s, atomic.
//!
//! **Re-sifting** replaces channel C: the old channel-C tree is removed first. Channel A, the
//! journal and the keyframes are never touched (V-PLN-04).
//!
//! REF: `specs/stream/channel_c_v0.sql` R3–R8; `specs/stream/canonical_v0.md` §3, §6;
//! `05.4` §2.3; ADR-0024.

use std::collections::BTreeMap;
use std::path::Path;

use rh_snapshot::hash::xxh3_128;
use rh_stream::canonical::{self, match_id};
use rh_stream::parquet_io::{Column, ColumnSpec, ColumnType as T, read_table, write_table};
use rh_stream::record::{Binding, Body, Channel, ClauseEvent, Header, Match, Record};
use rh_stream::seal::{NoHook, channel_dir, read_segment, seal, sealed_segments};
use rh_stream::segment::{SEGMENT_FILE, SegmentA, TableSpec};

use crate::SiftError;
use crate::pattern::Pattern;

/// `segment.format` of channel C v0.
pub const FORMAT_C: &str = "rehoboam/stream/channel-c/v0";
/// The chain's domain tag (R7).
pub const D_CHAIN: &[u8] = b"rehoboam/stream/channel-c/v0/a-chain";
/// This sifter's build, the plane-C half of the provenance.
pub const SIFTER_BUILD: &str = concat!("rh-sifter ", env!("CARGO_PKG_VERSION"));
/// The DSL grammar version (`05.4` §2.2).
pub const DSL_VERSION: u16 = 0;

const fn c(name: &'static str, ty: T) -> ColumnSpec {
    ColumnSpec { name, ty }
}

/// `record_kind` (channel C).
pub const RECORD_KIND_C: TableSpec = TableSpec {
    name: "record_kind",
    columns: &[c("kind", T::U16), c("name", T::Str), c("class", T::U8)],
};
/// `pattern`.
pub const PATTERN: TableSpec = TableSpec {
    name: "pattern",
    columns: &[
        c("pack", T::Str),
        c("pattern_id", T::Str),
        c("pattern_version", T::U16),
        c("selection", T::U8),
        c("on_complete", T::U8),
        c("clauses", T::U8),
        c("roles", T::ListStr),
        c("dedupe_key", T::ListStr),
        c("dedupe_days", T::U32),
        c("budget_num", T::U64),
        c("budget_den", T::U64),
        c("pack_digest", T::Blob),
    ],
};
/// `segment` (channel C).
pub const SEGMENT_C: TableSpec = TableSpec {
    name: "segment",
    columns: &[
        c("timeline_id", T::U32),
        c("channel", T::Str),
        c("sim_year", T::I32),
        c("format", T::Str),
        c("first_seq", T::U64),
        c("last_seq", T::OptU64),
        c("record_count", T::U64),
        c("first_tick", T::OptI64),
        c("last_tick", T::OptI64),
        c("sifter_build", T::Str),
        c("dsl_version", T::U16),
        c("packs", T::ListStr),
        c("engine_build", T::Str),
        c("registry_version", T::Str),
        c("manifest_hash", T::Blob),
        c("world_seed", T::U64),
        c("a_last_seq", T::U64),
        c("a_canonical_digest", T::Blob),
        c("a_chain", T::Blob),
        c("canonical_digest", T::Blob),
    ],
};
/// `sift_match`.
pub const SIFT_MATCH: TableSpec = TableSpec {
    name: "sift_match",
    columns: &[
        c("timeline_id", T::U32),
        c("seq", T::U64),
        c("tick", T::I64),
        c("kind", T::U16),
        c("match_id", T::U64),
        c("pack", T::Str),
        c("pattern_id", T::Str),
        c("pattern_version", T::U16),
        c("selection", T::U8),
        c("on_complete", T::U8),
        c("status", T::U8),
        c("opened_tick", T::I64),
        c("closed_by_event_id", T::U64),
        c("closed_by_seq", T::U64),
    ],
};
/// `sift_binding`.
pub const SIFT_BINDING: TableSpec = TableSpec {
    name: "sift_binding",
    columns: &[
        c("timeline_id", T::U32),
        c("match_id", T::U64),
        c("role", T::Str),
        c("entity_kind", T::U8),
        c("entity_id", T::U64),
    ],
};
/// `sift_match_event`.
pub const SIFT_MATCH_EVENT: TableSpec = TableSpec {
    name: "sift_match_event",
    columns: &[
        c("timeline_id", T::U32),
        c("match_id", T::U64),
        c("clause", T::Str),
        c("ordinal", T::U8),
        c("event_id", T::U64),
    ],
};

/// Every table of a channel-C v0 segment, in sealing order, `segment` last.
pub const CHANNEL_C: [TableSpec; 6] = [
    PATTERN,
    RECORD_KIND_C,
    SIFT_BINDING,
    SIFT_MATCH,
    SIFT_MATCH_EVENT,
    SEGMENT_C,
];

/// The matches one event completes, in R4 order within the event.
fn matches_of(
    event: &rh_stream::record::Event,
    act_names: &[String],
    patterns: &[Pattern],
) -> Vec<(Pattern, Vec<Binding>)> {
    let mut out = Vec::new();
    for p in patterns {
        let verb_ok = p.verbs.is_empty() || p.verbs.contains(&event.verb);
        let act_ok = p.acts.is_empty()
            || act_names.iter().enumerate().any(|(i, name)| {
                i < 64 && event.act_classes & (1 << i) != 0 && p.acts.iter().any(|a| a == name)
            });
        if !(verb_ok && act_ok) {
            continue;
        }
        // Every combination of entities filling the clause's roles; a variable named twice must
        // be the same entity.
        let mut combos: Vec<BTreeMap<String, (u8, u64)>> = vec![BTreeMap::new()];
        for (code, var) in &p.clause_roles {
            let fillers: Vec<(u8, u64)> = event
                .roles
                .iter()
                .filter(|r| r.role == *code)
                .map(|r| (r.entity_kind, r.entity_id))
                .collect();
            let mut next = Vec::new();
            for combo in &combos {
                for &f in &fillers {
                    match combo.get(var) {
                        Some(&bound) if bound != f => {}
                        _ => {
                            let mut c = combo.clone();
                            c.insert(var.clone(), f);
                            next.push(c);
                        }
                    }
                }
            }
            combos = next;
        }
        let mut bindings: Vec<Vec<Binding>> = combos
            .into_iter()
            .map(|c| {
                let mut b: Vec<Binding> = c
                    .into_iter()
                    .map(|(role, (entity_kind, entity_id))| Binding {
                        role,
                        entity_kind,
                        entity_id,
                    })
                    .collect();
                b.sort_by(|x, y| x.role.as_bytes().cmp(y.role.as_bytes()));
                b
            })
            .collect();
        bindings.sort_by_key(|b| binding_key(b));
        bindings.dedup();
        out.extend(bindings.into_iter().map(|b| (p.clone(), b)));
    }
    out
}

/// A binding list's order key: (role bytes, entity kind, entity id) per binding.
type BindingKey = Vec<(Vec<u8>, u8, u64)>;
/// R4's order key of a match: closing seq, pack, pattern, version, event ids, bindings.
type MatchKey = (u64, Vec<u8>, Vec<u8>, u16, Vec<u64>, BindingKey);

fn binding_key(b: &[Binding]) -> BindingKey {
    b.iter()
        .map(|x| (x.role.as_bytes().to_vec(), x.entity_kind, x.entity_id))
        .collect()
}

/// R4's order key of a match.
fn match_key(m: &Match) -> MatchKey {
    (
        m.closed_by_seq,
        m.pack.as_bytes().to_vec(),
        m.pattern_id.as_bytes().to_vec(),
        m.pattern_version,
        m.events.iter().map(|e| e.event_id).collect(),
        binding_key(&m.bindings),
    )
}

/// The matches of one channel-A segment, in R4 order, with seq from `first_seq`.
#[must_use]
pub fn sift_segment(seg: &SegmentA, patterns: &[Pattern], first_seq: u64) -> Vec<Record> {
    let mut matches: Vec<Match> = Vec::new();
    for r in &seg.records {
        let Body::Event(e) = &r.body else { continue };
        for (p, bindings) in matches_of(e, &seg.dictionaries.act_classes, patterns) {
            matches.push(Match {
                match_id: match_id(&p.pack, &p.id, p.version, &[e.event_id], &bindings),
                pack: p.pack.clone(),
                pattern_id: p.id.clone(),
                pattern_version: p.version,
                selection: p.selection,
                on_complete: p.on_complete,
                status: 1,
                opened_tick: r.header.tick,
                closed_by_event_id: e.event_id,
                closed_by_seq: r.header.seq,
                bindings,
                events: vec![ClauseEvent {
                    clause: p.bind.clone(),
                    event_id: e.event_id,
                }],
            });
        }
    }
    matches.sort_by_key(match_key);
    let ticks: BTreeMap<u64, i64> = seg
        .records
        .iter()
        .map(|r| (r.header.seq, r.header.tick))
        .collect();
    matches
        .into_iter()
        .zip(first_seq..)
        .map(|(m, seq)| Record {
            header: Header {
                timeline_id: seg.row.timeline_id,
                seq,
                tick: ticks
                    .get(&m.closed_by_seq)
                    .copied()
                    .unwrap_or(m.opened_tick),
            },
            body: Body::Match(m),
        })
        .collect()
}

/// The chain value before a timeline's first channel-A segment (R7).
#[must_use]
pub fn chain_genesis(timeline_id: u32) -> [u8; 16] {
    let mut b = D_CHAIN.to_vec();
    b.extend_from_slice(&timeline_id.to_le_bytes());
    xxh3_128(&b)
}

/// The chain after a channel-A segment whose digest is `digest` (R7).
#[must_use]
pub fn chain_next(previous: &[u8; 16], digest: &[u8; 16]) -> [u8; 16] {
    let mut b = D_CHAIN.to_vec();
    b.extend_from_slice(previous);
    b.extend_from_slice(digest);
    xxh3_128(&b)
}

/// A channel-C segment's `segment` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentRowC {
    /// The timeline.
    pub timeline_id: u32,
    /// The sim-year.
    pub sim_year: i32,
    /// The first record's seq, or the next seq for an empty segment.
    pub first_seq: u64,
    /// The record count.
    pub record_count: u64,
    /// The packs sifted, in the manifest's order.
    pub packs: Vec<String>,
    /// The channel-A segment's last seq.
    pub a_last_seq: u64,
    /// The channel-A segment's canonical digest.
    pub a_canonical_digest: [u8; 16],
    /// The chain of channel-A digests up to it.
    pub a_chain: [u8; 16],
    /// This segment's canonical digest.
    pub canonical_digest: [u8; 16],
}

/// A decoded channel-C segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentC {
    /// Its row.
    pub row: SegmentRowC,
    /// Its records, in seq order.
    pub records: Vec<Record>,
}

fn table_err(e: &rh_stream::StreamError) -> SiftError {
    SiftError::Segment(e.to_string())
}

/// The files of one channel-C segment, in sealing order.
#[allow(
    clippy::too_many_lines,
    reason = "one block per table of the format, in DDL order, as rh-stream's channel A"
)]
fn encode_c(
    a: &SegmentA,
    records: &[Record],
    patterns: &[Pattern],
    packs: &[String],
    chain: &[u8; 16],
    first_seq: u64,
) -> Result<Vec<(String, Vec<u8>)>, SiftError> {
    let lines = records
        .iter()
        .map(canonical::render)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| SiftError::Segment(e.to_string()))?;
    let digest = canonical::digest(&lines);
    let ms: Vec<(&Header, &Match)> = records
        .iter()
        .filter_map(|r| match &r.body {
            Body::Match(m) => Some((&r.header, m)),
            _ => None,
        })
        .collect();
    let tl = a.row.timeline_id;
    let mut t: BTreeMap<&str, Vec<Column>> = BTreeMap::new();
    t.insert(
        RECORD_KIND_C.name,
        vec![
            Column::U16(vec![1]),
            Column::Str(vec!["MATCH".to_owned()]),
            Column::U8(vec![0]),
        ],
    );
    t.insert(
        PATTERN.name,
        vec![
            Column::Str(patterns.iter().map(|p| p.pack.clone()).collect()),
            Column::Str(patterns.iter().map(|p| p.id.clone()).collect()),
            Column::U16(patterns.iter().map(|p| p.version).collect()),
            Column::U8(patterns.iter().map(|p| p.selection).collect()),
            Column::U8(patterns.iter().map(|p| p.on_complete).collect()),
            Column::U8(patterns.iter().map(|_| 1).collect()),
            Column::ListStr(patterns.iter().map(|p| p.roles.clone()).collect()),
            Column::ListStr(patterns.iter().map(|_| Vec::new()).collect()),
            Column::U32(patterns.iter().map(|_| 0).collect()),
            Column::U64(patterns.iter().map(|p| p.budget.0).collect()),
            Column::U64(patterns.iter().map(|p| p.budget.1).collect()),
            Column::Blob(patterns.iter().map(|p| p.pack_digest.to_vec()).collect()),
        ],
    );
    t.insert(
        SIFT_MATCH.name,
        vec![
            Column::U32(ms.iter().map(|_| tl).collect()),
            Column::U64(ms.iter().map(|(h, _)| h.seq).collect()),
            Column::I64(ms.iter().map(|(h, _)| h.tick).collect()),
            Column::U16(ms.iter().map(|_| 1).collect()),
            Column::U64(ms.iter().map(|(_, m)| m.match_id).collect()),
            Column::Str(ms.iter().map(|(_, m)| m.pack.clone()).collect()),
            Column::Str(ms.iter().map(|(_, m)| m.pattern_id.clone()).collect()),
            Column::U16(ms.iter().map(|(_, m)| m.pattern_version).collect()),
            Column::U8(ms.iter().map(|(_, m)| m.selection).collect()),
            Column::U8(ms.iter().map(|(_, m)| m.on_complete).collect()),
            Column::U8(ms.iter().map(|(_, m)| m.status).collect()),
            Column::I64(ms.iter().map(|(_, m)| m.opened_tick).collect()),
            Column::U64(ms.iter().map(|(_, m)| m.closed_by_event_id).collect()),
            Column::U64(ms.iter().map(|(_, m)| m.closed_by_seq).collect()),
        ],
    );
    let binds: Vec<(u64, &Binding)> = ms
        .iter()
        .flat_map(|(_, m)| m.bindings.iter().map(move |b| (m.match_id, b)))
        .collect();
    t.insert(
        SIFT_BINDING.name,
        vec![
            Column::U32(binds.iter().map(|_| tl).collect()),
            Column::U64(binds.iter().map(|(id, _)| *id).collect()),
            Column::Str(binds.iter().map(|(_, b)| b.role.clone()).collect()),
            Column::U8(binds.iter().map(|(_, b)| b.entity_kind).collect()),
            Column::U64(binds.iter().map(|(_, b)| b.entity_id).collect()),
        ],
    );
    let evs: Vec<(u64, u8, &ClauseEvent)> = ms
        .iter()
        .flat_map(|(_, m)| {
            m.events
                .iter()
                .enumerate()
                .map(move |(i, e)| (m.match_id, u8::try_from(i).unwrap_or(u8::MAX), e))
        })
        .collect();
    t.insert(
        SIFT_MATCH_EVENT.name,
        vec![
            Column::U32(evs.iter().map(|_| tl).collect()),
            Column::U64(evs.iter().map(|(id, _, _)| *id).collect()),
            Column::Str(evs.iter().map(|(_, _, e)| e.clause.clone()).collect()),
            Column::U8(evs.iter().map(|(_, o, _)| *o).collect()),
            Column::U64(evs.iter().map(|(_, _, e)| e.event_id).collect()),
        ],
    );
    let count = u64::try_from(ms.len()).unwrap_or(u64::MAX);
    let p = &a.row.provenance;
    t.insert(
        SEGMENT_C.name,
        vec![
            Column::U32(vec![tl]),
            Column::Str(vec!["C".to_owned()]),
            Column::I32(vec![a.row.sim_year]),
            Column::Str(vec![FORMAT_C.to_owned()]),
            Column::U64(vec![first_seq]),
            Column::OptU64(vec![ms.last().map(|(h, _)| h.seq)]),
            Column::U64(vec![count]),
            Column::OptI64(vec![ms.first().map(|(h, _)| h.tick)]),
            Column::OptI64(vec![ms.last().map(|(h, _)| h.tick)]),
            Column::Str(vec![SIFTER_BUILD.to_owned()]),
            Column::U16(vec![DSL_VERSION]),
            Column::ListStr(vec![packs.to_vec()]),
            Column::Str(vec![p.engine_build.clone()]),
            Column::Str(vec![a.row.registry_version.clone()]),
            Column::Blob(vec![p.manifest_hash.clone()]),
            Column::U64(vec![p.world_seed]),
            Column::U64(vec![a.row.last_seq]),
            Column::Blob(vec![a.row.canonical_digest.to_vec()]),
            Column::Blob(vec![chain.to_vec()]),
            Column::Blob(vec![digest.to_vec()]),
        ],
    );
    let files = CHANNEL_C
        .iter()
        .map(|spec| {
            let cols = t
                .remove(spec.name)
                .ok_or_else(|| SiftError::Segment(format!("no rows for {}", spec.name)))?;
            Ok((
                spec.file(),
                write_table(spec.name, spec.columns, &cols).map_err(|e| table_err(&e))?,
            ))
        })
        .collect::<Result<Vec<_>, SiftError>>()?;
    Ok(files)
}

/// What a sift did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SiftSummary {
    /// Channel-C segments sealed (one per channel-A segment).
    pub segments: u64,
    /// Matches written.
    pub matches: u64,
    /// The channel-A records read.
    pub a_records: u64,
}

/// Sifts a run directory's sealed channel A with `patterns` (of `packs`, in order) and rewrites its
/// channel C (module docs).
///
/// # Errors
/// [`SiftError`] if channel A cannot be read or channel C cannot be written.
pub fn sift_run(
    run_dir: &Path,
    timeline_id: u32,
    minutes_per_year: i64,
    patterns: &[Pattern],
    packs: &[String],
) -> Result<SiftSummary, SiftError> {
    let root = run_dir.join("stream");
    let a = rh_stream::writer::read_channel_a(&root, timeline_id, minutes_per_year)
        .map_err(|e| table_err(&e))?;
    let dir_c = channel_dir(&root, timeline_id, Channel::C);
    if dir_c.exists() {
        std::fs::remove_dir_all(&dir_c)
            .map_err(|e| SiftError::Io(format!("{}: {e}", dir_c.display())))?;
    }
    let mut chain = chain_genesis(timeline_id);
    let mut next = 0u64;
    let mut summary = SiftSummary::default();
    for seg in &a {
        chain = chain_next(&chain, &seg.row.canonical_digest);
        let records = sift_segment(seg, patterns, next);
        let files = encode_c(seg, &records, patterns, packs, &chain, next)?;
        seal(
            &dir_c,
            Channel::C,
            i64::from(seg.row.sim_year),
            &files,
            &mut NoHook,
        )
        .map_err(|e| table_err(&e))?;
        let n = u64::try_from(records.len()).unwrap_or(u64::MAX);
        next += n;
        summary.matches += n;
        summary.segments += 1;
        summary.a_records += seg.row.record_count;
    }
    Ok(summary)
}

fn blob16(v: &[u8]) -> Result<[u8; 16], SiftError> {
    <[u8; 16]>::try_from(v)
        .map_err(|_| SiftError::Segment("a 16-byte blob of another length".to_owned()))
}

/// Reads a run's channel-C segments in sim-year order, checking each: the format's files, its
/// row, its seq run, and its canonical digest recomputed from its records.
///
/// # Errors
/// [`SiftError`] for a missing or inconsistent segment.
pub fn read_channel_c(run_dir: &Path, timeline_id: u32) -> Result<Vec<SegmentC>, SiftError> {
    let dir = channel_dir(&run_dir.join("stream"), timeline_id, Channel::C);
    let mut out: Vec<SegmentC> = Vec::new();
    for (year, path) in sealed_segments(&dir).map_err(|e| table_err(&e))? {
        let files = read_segment(&path).map_err(|e| table_err(&e))?;
        let get = |spec: &TableSpec| -> Result<Vec<Column>, SiftError> {
            let bytes = files.get(&spec.file()).ok_or_else(|| {
                SiftError::Segment(format!("{}: no {}", path.display(), spec.file()))
            })?;
            read_table(spec.name, spec.columns, bytes).map_err(|e| table_err(&e))
        };
        if files.len() != CHANNEL_C.len() || !files.contains_key(SEGMENT_FILE) {
            return Err(SiftError::Segment(format!(
                "{}: not the format's files",
                path.display()
            )));
        }
        let row = get(&SEGMENT_C)?;
        let m = get(&SIFT_MATCH)?;
        let b = get(&SIFT_BINDING)?;
        let e = get(&SIFT_MATCH_EVENT)?;
        let records = rebuild(&m, &b, &e)?;
        let Column::U64(first) = &row[4] else {
            return Err(bad_col());
        };
        let Column::U64(count) = &row[6] else {
            return Err(bad_col());
        };
        let Column::ListStr(packs) = &row[11] else {
            return Err(bad_col());
        };
        let Column::U64(a_last) = &row[16] else {
            return Err(bad_col());
        };
        let (Column::Blob(ad), Column::Blob(ch), Column::Blob(cd)) = (&row[17], &row[18], &row[19])
        else {
            return Err(bad_col());
        };
        let Column::I32(sim_year) = &row[2] else {
            return Err(bad_col());
        };
        let r = SegmentRowC {
            timeline_id,
            sim_year: sim_year[0],
            first_seq: first[0],
            record_count: count[0],
            packs: packs[0].clone(),
            a_last_seq: a_last[0],
            a_canonical_digest: blob16(&ad[0])?,
            a_chain: blob16(&ch[0])?,
            canonical_digest: blob16(&cd[0])?,
        };
        let expected = out
            .last()
            .map_or(0, |s| s.row.first_seq + s.row.record_count);
        let contiguous = r.first_seq == expected
            && i64::from(r.sim_year) == year
            && records
                .iter()
                .zip(r.first_seq..)
                .all(|(x, s)| x.header.seq == s)
            && u64::try_from(records.len()).ok() == Some(r.record_count);
        if !contiguous {
            return Err(SiftError::Segment(format!(
                "{}: seq is not contiguous (R3)",
                path.display()
            )));
        }
        let digest =
            canonical::digest_records(&records).map_err(|e| SiftError::Segment(e.to_string()))?;
        if digest != r.canonical_digest {
            return Err(SiftError::Segment(format!(
                "{}: the canonical digest differs",
                path.display()
            )));
        }
        out.push(SegmentC { row: r, records });
    }
    Ok(out)
}

fn bad_col() -> SiftError {
    SiftError::Segment("a column of the wrong type".to_owned())
}

fn rebuild(m: &[Column], b: &[Column], e: &[Column]) -> Result<Vec<Record>, SiftError> {
    let (
        Column::U32(tl),
        Column::U64(seq),
        Column::I64(tick),
        Column::U64(id),
        Column::Str(pack),
        Column::Str(pat),
        Column::U16(ver),
        Column::U8(sel),
        Column::U8(onc),
        Column::U8(st),
        Column::I64(opened),
        Column::U64(cev),
        Column::U64(cseq),
    ) = (
        &m[0], &m[1], &m[2], &m[4], &m[5], &m[6], &m[7], &m[8], &m[9], &m[10], &m[11], &m[12],
        &m[13],
    )
    else {
        return Err(bad_col());
    };
    let (Column::U64(bid), Column::Str(brole), Column::U8(bkind), Column::U64(bent)) =
        (&b[1], &b[2], &b[3], &b[4])
    else {
        return Err(bad_col());
    };
    let (Column::U64(eid), Column::Str(eclause), Column::U64(eev)) = (&e[1], &e[2], &e[4]) else {
        return Err(bad_col());
    };
    let mut binds: BTreeMap<u64, Vec<Binding>> = BTreeMap::new();
    for i in 0..bid.len() {
        binds.entry(bid[i]).or_default().push(Binding {
            role: brole[i].clone(),
            entity_kind: bkind[i],
            entity_id: bent[i],
        });
    }
    let mut events: BTreeMap<u64, Vec<ClauseEvent>> = BTreeMap::new();
    for i in 0..eid.len() {
        events.entry(eid[i]).or_default().push(ClauseEvent {
            clause: eclause[i].clone(),
            event_id: eev[i],
        });
    }
    Ok((0..seq.len())
        .map(|i| Record {
            header: Header {
                timeline_id: tl[i],
                seq: seq[i],
                tick: tick[i],
            },
            body: Body::Match(Match {
                match_id: id[i],
                pack: pack[i].clone(),
                pattern_id: pat[i].clone(),
                pattern_version: ver[i],
                selection: sel[i],
                on_complete: onc[i],
                status: st[i],
                opened_tick: opened[i],
                closed_by_event_id: cev[i],
                closed_by_seq: cseq[i],
                bindings: binds.remove(&id[i]).unwrap_or_default(),
                events: events.remove(&id[i]).unwrap_or_default(),
            }),
        })
        .collect())
}
