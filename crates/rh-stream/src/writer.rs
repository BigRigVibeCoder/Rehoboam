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
//! **The live edge** ([`StreamWriter::attach_live`]; SPR-026 T-02, only when a host serves a
//! socket). The writer renders each record's canonical line as it assigns its seq and appends the
//! instant's lines to the [`LiveEdge`] in one chunk per push; a year's completion is published
//! before its seal. Under [`Policy::YearEnd`] (D42's option (b)) the seal runs on a seal thread the
//! writer owns: the host thread hands it the year's records and goes on, and the thread publishes
//! the landing (`s`, then the drop). Work that must follow a seal (the host's keyframe file) is
//! queued behind it ([`StreamWriter::after_seal`]). Under [`Policy::Sync`] (option (a)) the seal
//! stays on the host thread. A writer dropped before [`StreamWriter::finish`] (the host stopped on
//! an error) lets the seals of completed years land, never seals its open year, and ends the edge
//! `Aborted` (BCK-002 A-42). A headless run attaches nothing and pays one `Option` check per record.
//!
//! REF: SPR-011 ruling 13; `specs/stream/channel_a_v0.sql` R3–R5; T-09 acceptance; PR #60's
//! mid-year `kill -9` resume.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};

use crate::StreamError;
use crate::live::{End, LiveEdge, Policy, SealEntry, Start};
use crate::record::{Body, Channel, Header, Record, sim_year};
use crate::seal::{
    SealHook, channel_dir, read_segment, read_segment_row, remove_partials, seal, sealed_segments,
};
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

/// A clock the host lends the writer to time its seals: plane A reads no clock, the host does
/// (`wire_v1.md` §6.2: every seal's latency is logged against its sim-year's wall time).
pub trait SealClock: Send + Sync {
    /// Now, in the host's units.
    fn now(&self) -> u64;
    /// The seal of `year`, handed over at `closed`, landed on disk at `landed`.
    fn landed(&self, year: i64, closed: u64, landed: u64);
}

/// What a writer feeding a live edge needs (module docs).
pub struct LiveConfig {
    /// The window policy.
    pub policy: Policy,
    /// The host's clock for the seal-latency log, if any.
    pub clock: Option<Arc<dyn SealClock>>,
    /// The seal hook of the seal thread (the host's test stalls; `NoHook` otherwise).
    pub hook: Box<dyn SealHook + Send>,
}

/// Work that must follow a year's seal (the host's keyframe file).
pub type AfterSeal = Box<dyn FnOnce() -> Result<(), String> + Send>;

enum Job {
    Seal {
        year: i64,
        records: Vec<Record>,
        closed: u64,
        /// The year's records as allocated, counted in `LiveEdge::queued_bytes` until the job has
        /// run.
        queued: usize,
    },
    Verify {
        path: PathBuf,
        records: Vec<Record>,
        queued: usize,
    },
    Skip {
        year: i64,
    },
    Then(AfterSeal),
}

struct Sealer {
    tx: Option<mpsc::Sender<Job>>,
    errors: mpsc::Receiver<StreamError>,
    handle: Option<std::thread::JoinHandle<(u64, u64)>>,
}

impl Sealer {
    fn send(&self, job: Job) {
        if let Some(tx) = &self.tx {
            // A thread that stopped on an error drops later jobs; the error is reported.
            let _ = tx.send(job);
        }
    }

    /// Lets every queued job run and joins the thread: (sealed, verified).
    fn join(&mut self) -> Result<(u64, u64), StreamError> {
        self.tx = None;
        let counts = match self.handle.take() {
            Some(h) => h
                .join()
                .map_err(|_| StreamError::Io("the seal thread panicked".to_owned()))?,
            None => (0, 0),
        };
        match self.errors.try_recv() {
            Ok(e) => Err(e),
            Err(_) => Ok(counts),
        }
    }
}

/// The writer's side of a live edge.
struct Live {
    edge: LiveEdge,
    start_year: i64,
    pending: Vec<u8>,
    ends: Vec<u32>,
    first: u64,
    year: i64,
    tick: i64,
    sealer: Option<Sealer>,
    clock: Option<Arc<dyn SealClock>>,
    ended: bool,
}

impl Live {
    fn add(&mut self, record: &Record, year: i64) -> Result<(), StreamError> {
        if year < self.start_year {
            return Ok(());
        }
        if self.ends.is_empty() {
            self.first = record.header.seq;
            self.year = year;
        }
        let line = crate::canonical::render(record)?;
        self.pending.extend_from_slice(line.as_bytes());
        self.pending.push(b'\n');
        self.ends.push(
            u32::try_from(self.pending.len())
                .map_err(|_| StreamError::Order("a live chunk over 4 GiB".to_owned()))?,
        );
        self.tick = record.header.tick;
        Ok(())
    }

    fn flush(&mut self) {
        if self.ends.is_empty() {
            return;
        }
        self.edge.append(
            self.year,
            self.first,
            std::mem::take(&mut self.pending),
            std::mem::take(&mut self.ends),
            self.tick,
        );
    }

    fn landed(&self, year: i64, entry: Option<&SealEntry>, closed: u64) {
        self.edge.seal_landed(year, entry);
        if let (Some(c), Some(_)) = (&self.clock, entry) {
            c.landed(year, closed, c.now());
        }
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        if self.ended {
            return;
        }
        // The host stopped on an error: completed years' seals land, the open year is never
        // sealed, and every subscription ends aborted (A-42).
        if let Some(s) = self.sealer.as_mut() {
            let _ = s.join();
        }
        self.edge.end(End::Aborted);
    }
}

fn entry_of(year: i64, files: &[(String, Vec<u8>)]) -> Result<SealEntry, StreamError> {
    let row = segment::read_row(files.last().map_or(&[][..], |(_, b)| b.as_slice()))?;
    Ok(SealEntry {
        sim_year: year,
        first_seq: row.first_seq,
        last_seq: Some(row.last_seq),
        canonical_digest: row.canonical_digest,
    })
}

/// Encodes and seals one year's segment; its seal-log entry.
fn seal_year(
    dir: &Path,
    cfg: &WriterConfig,
    year: i64,
    records: &[Record],
    hook: &mut dyn SealHook,
) -> Result<SealEntry, StreamError> {
    let files = segment::encode(
        records,
        year,
        cfg.minutes_per_year,
        &cfg.provenance,
        &cfg.dictionaries,
    )?;
    let entry = entry_of(year, &files)?;
    seal(dir, Channel::A, year, &files, hook)?;
    Ok(entry)
}

/// Checks that a resumed run's records of a year equal its sealed segment.
fn verify_year(path: &Path, records: &[Record], mpy: i64) -> Result<(), StreamError> {
    let sealed = segment::decode(&read_segment(path)?, mpy)?;
    if sealed.records != records {
        let at = sealed
            .records
            .iter()
            .zip(records)
            .position(|(a, b)| a != b)
            .unwrap_or(sealed.records.len().min(records.len()));
        return Err(StreamError::Segment(format!(
            "the resumed run differs from sealed segment {} at its record {at}",
            path.display()
        )));
    }
    Ok(())
}

fn seal_thread(
    dir: &Path,
    cfg: &WriterConfig,
    live: &(LiveEdge, Option<Arc<dyn SealClock>>),
    hook: &mut dyn SealHook,
    jobs: &mpsc::Receiver<Job>,
    errors: &mpsc::Sender<StreamError>,
) -> (u64, u64) {
    let (edge, clock) = live;
    let (mut sealed, mut verified) = (0, 0);
    let mut failed = false;
    for job in jobs {
        // A queued year leaves the count once its job has run, or is dropped after a failure.
        let queued = match &job {
            Job::Seal { queued, .. } | Job::Verify { queued, .. } => *queued,
            Job::Skip { .. } | Job::Then(_) => 0,
        };
        if failed {
            drop(job);
            edge.dequeue(queued);
            continue;
        }
        let done = match job {
            Job::Seal {
                year,
                records,
                closed,
                ..
            } => seal_year(dir, cfg, year, &records, hook).map(|entry| {
                edge.seal_landed(year, Some(&entry));
                if let Some(c) = clock {
                    c.landed(year, closed, c.now());
                }
                sealed += 1;
            }),
            Job::Verify { path, records, .. } => {
                verify_year(&path, &records, cfg.minutes_per_year).map(|()| verified += 1)
            }
            Job::Skip { year } => {
                edge.seal_landed(year, None);
                Ok(())
            }
            Job::Then(f) => f().map_err(StreamError::Io),
        };
        edge.dequeue(queued);
        if let Err(e) = done {
            // Nothing after a failed seal is sealed: a later segment would leave a gap.
            failed = true;
            let _ = errors.send(e);
        }
    }
    (sealed, verified)
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
    live: Option<Live>,
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
            live: None,
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
                live: None,
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
            live: None,
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
        self.check_sealer()?;
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
            let record = Record {
                header: Header {
                    timeline_id: self.cfg.timeline_id,
                    seq: self.next_seq,
                    tick,
                },
                body,
            };
            if let Some(live) = self.live.as_mut() {
                live.add(&record, year)?;
            }
            self.buffer.push(record);
            self.next_seq += 1;
            self.last_key = Some(key);
        }
        while (self.year + 1) * self.cfg.minutes_per_year - 1 <= at {
            self.close_year(hook)?;
        }
        if let Some(live) = self.live.as_mut() {
            live.flush();
        }
        Ok(())
    }

    /// The first error the seal thread reported, if any (never waits).
    fn check_sealer(&self) -> Result<(), StreamError> {
        match self
            .live
            .as_ref()
            .and_then(|l| l.sealer.as_ref())
            .map(|s| s.errors.try_recv())
        {
            Some(Ok(e)) => Err(e),
            _ => Ok(()),
        }
    }

    /// Feeds a live edge from now on (module docs): a new run's edge starts at seq 0; a resumed
    /// run's at the first seq of the first sim-year not on disk, with `s` and `last_year` from
    /// disk (R18), and the years it re-verifies below it are not appended. Under
    /// [`Policy::YearEnd`] this starts the seal thread.
    ///
    /// # Errors
    /// [`StreamError`] if a sealed segment's row cannot be read, or the thread cannot start.
    pub fn attach_live(&mut self, cfg: LiveConfig) -> Result<LiveEdge, StreamError> {
        let mut start = Start {
            year: self.year,
            next_seq: self.next_seq,
            s: self.next_seq.checked_sub(1),
            last_year: (self.year > 0).then(|| self.year - 1),
        };
        while let Some(path) = self.existing.get(&start.year) {
            let row = read_segment_row(path)?;
            start.next_seq = row.last_seq + 1;
            start.s = Some(row.last_seq);
            start.last_year = Some(start.year);
            start.year += 1;
        }
        let edge = LiveEdge::new(cfg.policy, start);
        let sealer = match cfg.policy {
            Policy::Sync => None,
            Policy::YearEnd => {
                let (tx, rx) = mpsc::channel::<Job>();
                let (etx, erx) = mpsc::channel::<StreamError>();
                let (dir, wcfg) = (self.dir.clone(), self.cfg.clone());
                let shared = (edge.clone(), cfg.clock.clone());
                let mut hook = cfg.hook;
                let handle = std::thread::Builder::new()
                    .name("rh-stream-seal".to_owned())
                    .spawn(move || seal_thread(&dir, &wcfg, &shared, hook.as_mut(), &rx, &etx))
                    .map_err(|e| StreamError::Io(format!("the seal thread: {e}")))?;
                Some(Sealer {
                    tx: Some(tx),
                    errors: erx,
                    handle: Some(handle),
                })
            }
        };
        self.live = Some(Live {
            edge: edge.clone(),
            start_year: start.year,
            pending: Vec::new(),
            ends: Vec::new(),
            first: 0,
            year: start.year,
            tick: 0,
            sealer,
            clock: cfg.clock,
            ended: false,
        });
        Ok(edge)
    }

    /// Runs `f` once every year closed so far is sealed: at once without a seal thread, else
    /// queued behind the seals in flight (the host's keyframe file, which must follow its year's
    /// segment).
    ///
    /// # Errors
    /// [`StreamError::Io`] with `f`'s error, when it runs now.
    pub fn after_seal(&mut self, f: AfterSeal) -> Result<(), StreamError> {
        match self.live.as_ref().and_then(|l| l.sealer.as_ref()) {
            Some(s) => {
                s.send(Job::Then(f));
                Ok(())
            }
            None => f().map_err(StreamError::Io),
        }
    }

    /// Seals (or verifies) the open year, if it holds records, and opens the next. With a live
    /// edge, the year's completion is published first, and the seal goes to the seal thread.
    fn close_year(&mut self, hook: &mut dyn SealHook) -> Result<(), StreamError> {
        let year = self.year;
        self.year += 1;
        let live_year = self.live.as_ref().is_some_and(|l| year >= l.start_year);
        if let Some(live) = self.live.as_mut().filter(|_| live_year) {
            live.flush();
            live.edge.year_completes(year);
        }
        if self.buffer.is_empty() {
            if let Some(live) = self.live.as_ref().filter(|_| live_year) {
                match &live.sealer {
                    Some(s) => s.send(Job::Skip { year }),
                    None => live.landed(year, None, 0),
                }
            }
            return Ok(());
        }
        let records = std::mem::take(&mut self.buffer);
        self.summary.records += u64::try_from(records.len()).unwrap_or(u64::MAX);
        // The year's records as allocated, counted in `LiveEdge::queued_bytes` while they wait
        // for the seal thread (GAP-1).
        let queued = crate::record::allocated(&records);
        let sealer = self.live.as_ref().and_then(|l| l.sealer.as_ref());
        // Counted before the job is sent, so the seal thread never subtracts first.
        let enqueue = |n: usize| {
            if let Some(l) = self.live.as_ref() {
                l.edge.queue(n);
            }
        };
        if let Some(path) = self.existing.remove(&year) {
            if let Some(s) = sealer {
                enqueue(queued);
                s.send(Job::Verify {
                    path,
                    records,
                    queued,
                });
                return Ok(());
            }
            verify_year(&path, &records, self.cfg.minutes_per_year)?;
            self.summary.verified += 1;
            return Ok(());
        }
        if let Some(s) = sealer {
            let closed = self
                .live
                .as_ref()
                .and_then(|l| l.clock.as_ref())
                .map_or(0, |c| c.now());
            enqueue(queued);
            s.send(Job::Seal {
                year,
                records,
                closed,
                queued,
            });
            return Ok(());
        }
        let closed = self
            .live
            .as_ref()
            .and_then(|l| l.clock.as_ref())
            .map_or(0, |c| c.now());
        let entry = seal_year(&self.dir, &self.cfg, year, &records, hook)?;
        self.summary.sealed += 1;
        if let Some(live) = self.live.as_ref().filter(|_| live_year) {
            live.landed(year, Some(&entry), closed);
        }
        Ok(())
    }

    /// Seals the open year, if it holds records, and reports.
    ///
    /// # Errors
    /// [`StreamError`] from sealing.
    pub fn finish(mut self, hook: &mut dyn SealHook) -> Result<Summary, StreamError> {
        self.close_year(hook)?;
        if let Some(live) = self.live.as_mut() {
            if let Some(s) = live.sealer.as_mut() {
                let (sealed, verified) = s.join()?;
                self.summary.sealed += sealed;
                self.summary.verified += verified;
            }
            live.edge.end(End::Finished);
            live.ended = true;
        }
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
