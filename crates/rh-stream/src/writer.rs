//! The channel-A writer: one per timeline, the only appender of channel A (`05.10` §7 item 1).
//!
//! The host feeds it after each stop of its run loop, with every record of every instant up to
//! that stop ([`StreamWriter::push`]). The writer sorts them into R4 order, refuses any record
//! that would come before one it already holds, assigns `seq`, fills `STEP_DIGEST`'s `births` and
//! `deaths` from the life records since the previous digest, and seals each sim-year's segment as
//! soon as the stop reaches the year's last minute, or at [`StreamWriter::finish`].
//!
//! **Resume** ([`StreamWriter::resume`]). A run resumed from a keyframe at the last minute of
//! sim-year `y` (the 10-year keyframes; their file is written only after `y`'s segment is
//! sealed) continues from `y`'s sealed segment: `seq` follows its `last_seq`, and the digest
//! counters are those after its last `STEP_DIGEST`. A segment already sealed for a later year (a
//! crash after it) is not written again: the resumed run's records for that year must equal it,
//! and a difference is a hard error (no seq below the first unsealed year is re-emitted). A
//! `.partial` directory a crash left is removed, and its year is sealed afresh.
//!
//! REF: SPR-011 ruling 13; `specs/stream/channel_a_v0.sql` R3–R5; T-09 acceptance; PR #60's
//! mid-year `kill -9` resume.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::StreamError;
use crate::record::{Body, Channel, Header, Record, sim_year};
use crate::seal::{SealHook, channel_dir, read_segment, remove_partials, seal, sealed_segments};
use crate::segment::{self, Dictionaries, Provenance};

/// What a writer needs to know of its run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriterConfig {
    /// The run directory's `stream/`.
    pub root: PathBuf,
    /// The timeline.
    pub timeline_id: u32,
    /// The calendar's minutes per year (R5's sim-year).
    pub minutes_per_year: i64,
    /// The history tuple every segment carries.
    pub provenance: Provenance,
    /// The dictionaries every segment carries.
    pub dictionaries: Dictionaries,
}

/// What a writer did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Segments sealed by this writer.
    pub sealed: u64,
    /// Segments already sealed that the resumed run reproduced exactly.
    pub verified: u64,
    /// Records written or verified.
    pub records: u64,
    /// The next seq.
    pub next_seq: u64,
}

/// The channel-A writer of one timeline (module docs).
pub struct StreamWriter {
    cfg: WriterConfig,
    dir: PathBuf,
    next_seq: u64,
    /// The sim-year whose records `buffer` holds.
    year: i64,
    buffer: Vec<Record>,
    last_key: Option<(i64, u8, u64, u64)>,
    births: u32,
    deaths: u32,
    /// Segments already sealed for years after the resume point (verified, never rewritten).
    existing: BTreeMap<i64, PathBuf>,
    summary: Summary,
}

impl StreamWriter {
    /// The records of the open (unsealed) sim-year held in memory.
    #[must_use]
    pub const fn buffered(&self) -> usize {
        self.buffer.len()
    }

    /// The open sim-year.
    #[must_use]
    pub const fn open_year(&self) -> i64 {
        self.year
    }

    /// A writer for a new run: the timeline's channel A must hold no sealed segment.
    ///
    /// # Errors
    /// [`StreamError`] if the dictionaries are malformed, the directory cannot be read, or a
    /// segment is already sealed there.
    pub fn create(cfg: WriterConfig) -> Result<Self, StreamError> {
        cfg.dictionaries.check()?;
        let dir = channel_dir(&cfg.root, cfg.timeline_id, Channel::A);
        remove_partials(&dir)?;
        if let Some((year, path)) = sealed_segments(&dir)?.into_iter().next() {
            return Err(StreamError::Segment(format!(
                "{} already holds sealed segment {year} ({}): a new run writes a new stream",
                dir.display(),
                path.display()
            )));
        }
        Ok(Self {
            cfg,
            dir,
            next_seq: 0,
            year: 0,
            buffer: Vec::new(),
            last_key: None,
            births: 0,
            deaths: 0,
            existing: BTreeMap::new(),
            summary: Summary::default(),
        })
    }

    /// A writer for a run resumed after `tick`, the last minute of a sim-year whose segment is
    /// sealed (module docs), or -1: the genesis keyframe, before any record.
    ///
    /// # Errors
    /// [`StreamError::Segment`] if `tick` is not a year's last minute, that year's segment is not
    /// sealed or does not end by `tick`, or a sealed segment does not decode.
    pub fn resume(cfg: WriterConfig, tick: i64) -> Result<Self, StreamError> {
        cfg.dictionaries.check()?;
        let mpy = cfg.minutes_per_year;
        if (tick + 1).rem_euclid(mpy) != 0 {
            return Err(StreamError::Segment(format!(
                "a stream resumes after the last minute of a sim-year, not tick {tick}"
            )));
        }
        let year = sim_year(tick, mpy);
        let dir = channel_dir(&cfg.root, cfg.timeline_id, Channel::A);
        remove_partials(&dir)?;
        let mut sealed = sealed_segments(&dir)?;
        if year < 0 {
            // The genesis keyframe (before tick 0): nothing precedes it, and every sealed
            // segment is verified.
            return Ok(Self {
                cfg,
                dir,
                next_seq: 0,
                year: 0,
                buffer: Vec::new(),
                last_key: None,
                births: 0,
                deaths: 0,
                existing: sealed,
                summary: Summary::default(),
            });
        }
        let path = sealed.remove(&year).ok_or_else(|| {
            StreamError::Segment(format!(
                "sim-year {year}'s segment is not sealed in {}: resume from an earlier keyframe",
                dir.display()
            ))
        })?;
        let seg = segment::decode(&read_segment(&path)?, mpy)?;
        if seg.row.last_tick > tick {
            return Err(StreamError::Segment(format!(
                "sim-year {year}'s segment ends at tick {}, after the resume point {tick}",
                seg.row.last_tick
            )));
        }
        let mut w = Self {
            cfg,
            dir,
            next_seq: seg.row.last_seq + 1,
            year: year + 1,
            buffer: Vec::new(),
            last_key: seg.records.last().map(Record::order_key),
            births: 0,
            deaths: 0,
            existing: sealed.into_iter().filter(|(y, _)| *y > year).collect(),
            summary: Summary::default(),
        };
        for r in &seg.records {
            w.count(&r.body);
        }
        Ok(w)
    }

    /// Counts a record towards the next `STEP_DIGEST` (or resets at one).
    fn count(&mut self, body: &Body) {
        match body {
            Body::LifeEntry(e) if e.origin == 0 => self.births = self.births.saturating_add(1),
            Body::LifeExit(x) if x.exit_kind == 0 => self.deaths = self.deaths.saturating_add(1),
            Body::StepDigest(_) => {
                self.births = 0;
                self.deaths = 0;
            }
            _ => {}
        }
    }

    /// Appends every record of the instants up to and including `at` (`(tick, body)`, any order;
    /// all their ticks at most `at`), then seals the years that `at` completes.
    ///
    /// # Errors
    /// [`StreamError::Order`] for a record after `at`, before one already written, or tied with
    /// another on the R4 key; [`StreamError`] from sealing.
    pub fn push(
        &mut self,
        at: i64,
        mut records: Vec<(i64, Body)>,
        hook: &mut dyn SealHook,
    ) -> Result<(), StreamError> {
        crate::convert::link_exits(&mut records);
        let mut keyed: Vec<((i64, u8, u64, u64), Body)> = records
            .into_iter()
            .map(|(tick, body)| {
                let (k1, k2) = body.order_k();
                ((tick, body.kind().class(), k1, k2), body)
            })
            .collect();
        keyed.sort_by_key(|(k, _)| *k);
        for (key, mut body) in keyed {
            let tick = key.0;
            if tick > at {
                return Err(StreamError::Order(format!(
                    "a {} record at tick {tick} pushed with the stop {at}",
                    body.kind().name()
                )));
            }
            if self.last_key.is_some_and(|last| key <= last) {
                return Err(StreamError::Order(format!(
                    "a {} record at tick {tick} does not follow the last record in R4 order",
                    body.kind().name()
                )));
            }
            if body.kind().channel() != Channel::A {
                return Err(StreamError::Order(format!(
                    "a {} record in channel A",
                    body.kind().name()
                )));
            }
            let year = sim_year(tick, self.cfg.minutes_per_year);
            if year < self.year {
                return Err(StreamError::Order(format!(
                    "a record at tick {tick} of sim-year {year}, which is closed"
                )));
            }
            while year > self.year {
                self.close_year(hook)?;
            }
            if let Body::StepDigest(d) = &mut body {
                d.births = self.births;
                d.deaths = self.deaths;
            }
            self.count(&body);
            self.buffer.push(Record {
                header: Header {
                    timeline_id: self.cfg.timeline_id,
                    seq: self.next_seq,
                    tick,
                },
                body,
            });
            self.next_seq += 1;
            self.last_key = Some(key);
        }
        while (self.year + 1) * self.cfg.minutes_per_year - 1 <= at {
            self.close_year(hook)?;
        }
        Ok(())
    }

    /// Seals (or verifies) the open year, if it holds records, and opens the next.
    fn close_year(&mut self, hook: &mut dyn SealHook) -> Result<(), StreamError> {
        let year = self.year;
        self.year += 1;
        if self.buffer.is_empty() {
            return Ok(());
        }
        let records = std::mem::take(&mut self.buffer);
        self.summary.records += u64::try_from(records.len()).unwrap_or(u64::MAX);
        if let Some(path) = self.existing.remove(&year) {
            let sealed = segment::decode(&read_segment(&path)?, self.cfg.minutes_per_year)?;
            if sealed.records != records {
                let at = sealed
                    .records
                    .iter()
                    .zip(&records)
                    .position(|(a, b)| a != b)
                    .unwrap_or(sealed.records.len().min(records.len()));
                return Err(StreamError::Segment(format!(
                    "the resumed run differs from sealed segment {} at its record {at}",
                    path.display()
                )));
            }
            self.summary.verified += 1;
            return Ok(());
        }
        let files = segment::encode(
            &records,
            year,
            self.cfg.minutes_per_year,
            &self.cfg.provenance,
            &self.cfg.dictionaries,
        )?;
        seal(&self.dir, Channel::A, year, &files, hook)?;
        self.summary.sealed += 1;
        Ok(())
    }

    /// Seals the open year, if it holds records, and reports.
    ///
    /// # Errors
    /// [`StreamError`] from sealing.
    pub fn finish(mut self, hook: &mut dyn SealHook) -> Result<Summary, StreamError> {
        self.close_year(hook)?;
        self.summary.next_seq = self.next_seq;
        Ok(self.summary)
    }

    /// The channel directory this writer seals into.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The next seq.
    #[must_use]
    pub const fn next_seq(&self) -> u64 {
        self.next_seq
    }
}

/// A timeline's sealed channel-A segments under `root`, decoded, in sim-year order, with their
/// `seq` contiguous from 0 across them (R3).
///
/// # Errors
/// [`StreamError`] if a segment does not decode or `seq` has a gap or an overlap across segments.
pub fn read_channel_a(
    root: &Path,
    timeline_id: u32,
    minutes_per_year: i64,
) -> Result<Vec<segment::SegmentA>, StreamError> {
    let dir = channel_dir(root, timeline_id, Channel::A);
    let mut out: Vec<segment::SegmentA> = Vec::new();
    for (year, path) in sealed_segments(&dir)? {
        let seg = segment::decode(&read_segment(&path)?, minutes_per_year)?;
        if i64::from(seg.row.sim_year) != year || seg.row.timeline_id != timeline_id {
            return Err(StreamError::Segment(format!(
                "{} holds sim-year {} of timeline {}",
                path.display(),
                seg.row.sim_year,
                seg.row.timeline_id
            )));
        }
        let expected = out.last().map_or(0, |p| p.row.last_seq + 1);
        if seg.row.first_seq != expected {
            return Err(StreamError::Segment(format!(
                "{} starts at seq {}, not {expected}: seq is contiguous across segments (R3)",
                path.display(),
                seg.row.first_seq
            )));
        }
        out.push(seg);
    }
    Ok(out)
}
