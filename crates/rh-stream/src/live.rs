//! The live edge of a channel (`specs/stream/wire_v1.md` §6; SPR-026 T-02): the records of the
//! newest sim-years still in the host's memory, which a socket host serves to viewers.
//!
//! **One snapshot.** The whole published state is one immutable [`Snapshot`] behind an
//! `ArcSwap` (`arc-swap`, REV-SPR-026 R22): `w0`, `next_seq`, `s`, `last_year`, the tip, the chunk
//! list that holds the window from `w0`, the seal log, and the end. Every change builds the next
//! snapshot from the current one and stores it; a reader loads the `Arc`, reads and copies from
//! that snapshot alone, and drops it. The writer takes no lock a reader can hold and never waits
//! on a reader; a chunk lives as long as some snapshot references it. Two threads publish: the
//! host's (appends, year ends) and the seal thread's (a seal landed); each change is a
//! compare-and-swap loop over the current snapshot (`rcu`), which a reader's load never makes
//! retry.
//!
//! **Order (I4).** Per channel: a chunk is appended and `next_seq` moves; a year completes; a seal
//! lands and `s`, `last_year` and the seal log are published ([`LiveEdge::publish_seal`]); then a
//! year is dropped and `w0` moves ([`LiveEdge::drop_sealed`]), two separate publications in that
//! order, so a reader that loads `w0` and then `s`, or both from one snapshot, never sees
//! `s + 1 < w0` (I1). `w0` moves only past a year whose seal has landed.
//!
//! **The window policy** (§6.1, D42): [`Policy::YearEnd`] (option (b), the default) drops a year
//! when the next one completes or, if its seal lands later, when the seal lands; the seal runs
//! off the host thread, so the window holds the current and the previous sim-year, plus a slow
//! seal's lateness. [`Policy::Sync`] (option (a)) drops a year as soon as its (synchronous) seal
//! lands: one sim-year.
//!
//! **Appends stay small (the SPR-026 review's C1).** The window is three chunk lists: one chunk
//! per completed year, the open year's merged blocks, and its newest appends (at most
//! [`TAIL`] of them). An append copies the tail's pointers alone, and every [`TAIL`] appends
//! merge it into one block; a year's completion merges its blocks and tail into one chunk. Under
//! (b) at full speed the window is the whole unsealed backlog, so an append that copied the whole
//! list made a run quadratic in the host's appends.
//!
//! **Reading.** [`LiveEdge::read`] answers `read(from, max)` from one snapshot: [`Behind`] when
//! `from < w0` (with that snapshot's `s`, `last_year` and `w0`), else whole lines from `from`, at
//! most `max` bytes, copied from that snapshot's chunks: a seal and a drop that complete between the
//! load and the copy change nothing the copy sees, so a frame carries exactly the seqs it claims.
//!
//! It holds no simulation state: the lines are the ones the writer already rendered and validated
//! (the canonical printer is total on valid records). Built only when a host serves (`--socket`):
//! a headless run never builds one.
//!
//! REF: `specs/stream/wire_v1.md` §6.1, §6.2; REV-SPR-026 R2–R4, R16–R18, R22; ADR-0030.

use std::sync::Arc;
// LINT-ALLOW(hidden-state): live stream edge queue counter
use std::sync::atomic::{AtomicUsize, Ordering};

use arc_swap::ArcSwap;

/// The window policy (§6.1; D42).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Option (a): a year leaves memory when its synchronous seal lands (one sim-year).
    Sync,
    /// Option (b), the default: a year leaves memory when the next one completes or when its
    /// seal lands, whichever is later (the current and the previous sim-year).
    YearEnd,
}

/// Why a writer stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// The run ended normally; the last segment is sealed.
    Finished,
    /// The host stopped on an error; the open year is not sealed (BCK-002 A-42).
    Aborted,
}

/// One seal-log entry: a sealed segment of the channel (§6.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealEntry {
    /// The sim-year.
    pub sim_year: i64,
    /// Its first seq (an empty year: the seq its first record would have had).
    pub first_seq: u64,
    /// Its last seq (`None`: an empty year, channel C).
    pub last_seq: Option<u64>,
    /// Its `canonical_digest`.
    pub canonical_digest: [u8; 16],
}

/// A channel-C sifter's position on channel A (`stream.status.sifted_through`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Upstream {
    /// The last channel-A seq sifted.
    pub a_seq: Option<u64>,
    /// The last sealed channel-A sim-year sifted.
    pub a_year: Option<i64>,
}

/// A block of lines of one sim-year, contiguous in seq, each ending with LF.
#[derive(Debug)]
pub struct Chunk {
    year: i64,
    first_seq: u64,
    bytes: Vec<u8>,
    /// The end offset (after the LF) of each line.
    ends: Vec<u32>,
}

impl Chunk {
    /// One chunk holding `parts`' lines, in order (they are contiguous in seq, of one year).
    /// `None` when its offsets would not fit a `u32`: the parts then stay as they are.
    fn merged(parts: &[Arc<Chunk>]) -> Option<Self> {
        let first = parts.first()?;
        let len: usize = parts.iter().map(|c| c.bytes.len()).sum();
        let lines: usize = parts.iter().map(|c| c.ends.len()).sum();
        u32::try_from(len).ok()?;
        let mut bytes = Vec::with_capacity(len);
        let mut ends = Vec::with_capacity(lines);
        for c in parts {
            let base = u32::try_from(bytes.len()).ok()?;
            ends.extend(c.ends.iter().map(|&e| base + e));
            bytes.extend_from_slice(&c.bytes);
        }
        Some(Self {
            year: first.year,
            first_seq: first.first_seq,
            bytes,
            ends,
        })
    }

    fn count(&self) -> u64 {
        u64::try_from(self.ends.len()).unwrap_or(u64::MAX)
    }

    fn start_of(&self, i: usize) -> usize {
        if i == 0 {
            0
        } else {
            usize::try_from(self.ends[i - 1]).unwrap_or(usize::MAX)
        }
    }

    fn allocated(&self) -> usize {
        self.bytes.capacity() + self.ends.capacity() * size_of::<u32>()
    }
}

/// Where a live edge starts (§6.2; R18): a new run at seq 0 of year 0, or a resumed host at the
/// first seq of the first sim-year not on disk, with `s` and `last_year` read from disk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Start {
    /// The first sim-year this host writes.
    pub year: i64,
    /// Its first seq (`w0 = w1`).
    pub next_seq: u64,
    /// `sealed.last_seq` on disk.
    pub s: Option<u64>,
    /// `sealed.last_year` on disk.
    pub last_year: Option<i64>,
    /// The last record's tick from disk, if resumed (C20).
    pub tip_tick: Option<i64>,
}

/// The published state (module docs). Every field a reader sees comes from one snapshot.
#[derive(Clone, Debug)]
pub struct Snapshot {
    /// `live.first_seq`.
    pub w0: u64,
    /// `live.next_seq` = `tip.next_seq`.
    pub next_seq: u64,
    /// `sealed.last_seq`: the last seq of the newest non-empty sealed segment.
    pub s: Option<u64>,
    /// `sealed.last_year`: the newest sealed sim-year, empty or not.
    pub last_year: Option<i64>,
    /// The tick of the last record written.
    pub tip_tick: Option<i64>,
    /// The seal log since this host started.
    pub seal_log: Arc<Vec<SealEntry>>,
    /// Set when the writer has stopped.
    pub end: Option<End>,
    /// A sifter's position on channel A (channel C only).
    pub upstream: Option<Upstream>,
    /// One chunk per completed year in the window (or its parts, when they could not merge).
    chunks: Arc<Vec<Arc<Chunk>>>,
    /// The open year's merged blocks of [`TAIL`] appends.
    open: Arc<Vec<Arc<Chunk>>>,
    /// The open year's newest appends, at most [`TAIL`].
    tail: Arc<Vec<Arc<Chunk>>>,
    policy: Policy,
    /// The sim-year being written.
    open_year: i64,
    /// The newest completed year.
    completed: Option<i64>,
    /// The newest year whose seal has landed (seals land in year order).
    sealed_through: Option<i64>,
    /// `(year, first seq)` of every year from the window's first on.
    year_first: Arc<Vec<(i64, u64)>>,
}

/// A copy of window lines: `count` lines from `first_seq`, LF-terminated, back to back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lines {
    /// The seq of the first line.
    pub first_seq: u64,
    /// The number of lines.
    pub count: u64,
    /// The bytes (exactly what `canonical_digest` hashes for these records).
    pub body: Vec<u8>,
}

/// A read below the window: the reader's next seq left it (§6.2). From the same snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Behind {
    /// `s`.
    pub s: Option<u64>,
    /// `last_year`.
    pub last_year: Option<i64>,
    /// `w0`.
    pub w0: u64,
}

/// The most appends the open year's tail holds before they merge into one block (C1).
pub const TAIL: usize = 64;

impl Snapshot {
    /// The window's chunks, in seq order: the completed years, the open year's blocks, its tail.
    fn parts(&self) -> [&[Arc<Chunk>]; 3] {
        [&self.chunks, &self.open, &self.tail]
    }

    fn chunk(&self, mut i: usize) -> Option<&Arc<Chunk>> {
        for part in self.parts() {
            if i < part.len() {
                return part.get(i);
            }
            i -= part.len();
        }
        None
    }

    /// The index of the first chunk that does not end at or before `seq`.
    fn chunk_past(&self, seq: u64) -> usize {
        let mut base = 0;
        for part in self.parts() {
            let at = part.partition_point(|c| c.first_seq + c.count() <= seq);
            if at < part.len() {
                return base + at;
            }
            base += part.len();
        }
        base
    }

    /// The window's bytes as allocated (lines and their offsets): `LiveEdge::window_bytes()`.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.parts()
            .iter()
            .flat_map(|p| p.iter())
            .map(|c| c.allocated())
            .sum()
    }

    /// The window's chunks now (the completed years, the open year's blocks and its tail).
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.parts().iter().map(|p| p.len()).sum()
    }

    /// The distinct sim-years with a record in the window.
    #[must_use]
    pub fn window_years(&self) -> usize {
        let mut years: Vec<i64> = self
            .parts()
            .iter()
            .flat_map(|p| p.iter())
            .map(|c| c.year)
            .collect();
        years.dedup();
        years.len()
    }

    /// The window's `i`-th line (from `w0`), with its LF; `None` past the tip.
    #[must_use]
    pub fn window_line(&self, i: u64) -> Option<&[u8]> {
        let seq = self.w0.checked_add(i)?;
        let (chunk, k) = self.locate(seq)?;
        chunk
            .bytes
            .get(chunk.start_of(k)..usize::try_from(chunk.ends[k]).ok()?)
    }

    fn locate(&self, seq: u64) -> Option<(&Chunk, usize)> {
        let chunk = self.chunk(self.chunk_past(seq))?;
        let k = usize::try_from(seq.checked_sub(chunk.first_seq)?).ok()?;
        (k < chunk.ends.len()).then_some((chunk.as_ref(), k))
    }

    /// The lines from `from` in this snapshot alone, at most `max` bytes (at least one line when
    /// one is there), or [`Behind`] when `from < w0`.
    ///
    /// # Errors
    /// [`Behind`] with this snapshot's `s`, `last_year` and `w0`.
    pub fn read(&self, from: u64, max: usize) -> Result<Lines, Behind> {
        if from < self.w0 {
            return Err(Behind {
                s: self.s,
                last_year: self.last_year,
                w0: self.w0,
            });
        }
        let mut out = Lines {
            first_seq: from,
            count: 0,
            body: Vec::new(),
        };
        let mut seq = from;
        let mut i = self.chunk_past(seq);
        while let Some(chunk) = self.chunk(i) {
            // Chunks are contiguous from w0, so `seq` is inside this one.
            let Some(k0) = seq
                .checked_sub(chunk.first_seq)
                .and_then(|k| usize::try_from(k).ok())
            else {
                break;
            };
            let start = chunk.start_of(k0);
            let mut k1 = k0;
            while let Some(&end) = chunk.ends.get(k1) {
                let len = usize::try_from(end).unwrap_or(usize::MAX) - start;
                if out.body.len() + len > max && out.count + u64::try_from(k1 - k0).unwrap_or(0) > 0
                {
                    break;
                }
                k1 += 1;
            }
            out.body
                .extend_from_slice(&chunk.bytes[start..chunk.start_of(k1)]);
            let n = u64::try_from(k1 - k0).unwrap_or(0);
            out.count += n;
            seq += n;
            if k1 < chunk.ends.len() {
                break;
            }
            i += 1;
        }
        Ok(out)
    }

    fn first_of(&self, year: i64) -> Option<u64> {
        self.year_first
            .iter()
            .find(|(y, _)| *y == year)
            .map(|(_, f)| *f)
    }

    /// The year through which the window may drop, under the policy (I4: only sealed years).
    fn droppable(&self) -> Option<i64> {
        let completed = self.completed?;
        let bound = match self.policy {
            Policy::Sync => completed,
            Policy::YearEnd => completed - 1,
        };
        if cfg!(feature = "mutant-window-gap") {
            // The planted control (SPR-026 T-02): the window moves at the year end, past the year
            // just completed, whether or not it is on disk (I4 broken).
            return Some(completed);
        }
        Some(self.sealed_through?.min(bound))
    }

    /// The next snapshot with every droppable year dropped and `w0` moved.
    fn dropped(&self) -> Option<Self> {
        let through = self.droppable()?;
        let keep_seq = self.first_of(through + 1)?;
        if keep_seq <= self.w0 {
            return None;
        }
        let mut next = self.clone();
        next.chunks = Arc::new(
            self.chunks
                .iter()
                .filter(|c| c.year > through)
                .cloned()
                .collect(),
        );
        next.year_first = Arc::new(
            self.year_first
                .iter()
                .filter(|(y, _)| *y > through)
                .copied()
                .collect(),
        );
        next.w0 = keep_seq;
        Some(next)
    }
}

/// The live edge of one channel (module docs). Clones share one published state: the writer's
/// clone publishes, the serving threads' clones read.
#[derive(Clone, Debug)]
pub struct LiveEdge {
    state: Arc<ArcSwap<Snapshot>>,
    /// The bytes of the completed years queued for the seal thread and not yet sealed
    /// (`LiveEdge::queued_bytes`; the Architect's GAP-1 ruling).
    // LINT-ALLOW(hidden-state): live stream edge queue counter
    queued: Arc<AtomicUsize>,
}

impl LiveEdge {
    /// A live edge under `policy` starting at `start`: an empty window, `w0 = w1`.
    #[must_use]
    pub fn new(policy: Policy, start: Start) -> Self {
        let snap = Snapshot {
            w0: start.next_seq,
            next_seq: start.next_seq,
            s: start.s,
            last_year: start.last_year,
            tip_tick: start.tip_tick,
            seal_log: Arc::new(Vec::new()),
            end: None,
            upstream: None,
            chunks: Arc::new(Vec::new()),
            open: Arc::new(Vec::new()),
            tail: Arc::new(Vec::new()),
            policy,
            open_year: start.year,
            completed: start.year.checked_sub(1).filter(|&y| y >= 0),
            sealed_through: start.year.checked_sub(1).filter(|&y| y >= 0),
            year_first: Arc::new(vec![(start.year, start.next_seq)]),
        };
        Self {
            state: Arc::new(ArcSwap::from_pointee(snap)),
            // LINT-ALLOW(hidden-state): live stream edge queue counter
            queued: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// The published state now: one atomic load.
    #[must_use]
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.state.load_full()
    }

    /// `read(from, max)` against one snapshot (module docs).
    ///
    /// # Errors
    /// [`Behind`] when `from` is below the window.
    pub fn read(&self, from: u64, max: usize) -> Result<Lines, Behind> {
        self.read_with(from, max, || {})
    }

    /// [`Self::read`] with `pause` run between the snapshot's load and the copy: the seam where a
    /// test freezes a reader across a seal and a drop (§9 probe (b)).
    ///
    /// # Errors
    /// [`Behind`] when `from` is below the snapshot's window.
    pub fn read_with(&self, from: u64, max: usize, pause: impl FnOnce()) -> Result<Lines, Behind> {
        let snap = self.snapshot();
        pause();
        snap.read(from, max)
    }

    /// The live edge's bytes now: the window as allocated ([`Self::window_bytes`]) plus the
    /// completed years queued for the seal thread ([`Self::queued_bytes`]). The term V-PERF-09
    /// sets the host's live bytes against (GAP-1: under (b) at full speed the queue holds most of
    /// the run, and a window-only count understated the host by about five times).
    #[must_use]
    pub fn bytes(&self) -> usize {
        #[cfg(feature = "mutant-bytes-window-only")]
        {
            self.window_bytes()
        }
        #[cfg(not(feature = "mutant-bytes-window-only"))]
        {
            self.window_bytes() + self.queued_bytes()
        }
    }

    /// The window's bytes as allocated (lines and their offsets), now.
    #[must_use]
    pub fn window_bytes(&self) -> usize {
        self.snapshot().bytes()
    }

    /// The seal queue's bytes, now: each completed year queued for sealing (or verifying) counts
    /// its records as allocated (`rh_stream::record::allocated`: the vector's capacity and each
    /// record's strings and vectors), from the moment it is queued until its job has run. The
    /// canonical text would understate it: a queued `Record` takes the size of its largest body
    /// variant, about 2.5 times its line on solo.
    #[must_use]
    pub fn queued_bytes(&self) -> usize {
        self.queued.load(Ordering::Relaxed)
    }

    /// A completed year of `n` bytes was queued for the seal thread.
    pub(crate) fn queue(&self, n: usize) {
        self.queued.fetch_add(n, Ordering::Relaxed);
    }

    /// A queued year of `n` bytes was sealed, verified or dropped by the seal thread.
    pub(crate) fn dequeue(&self, n: usize) {
        self.queued.fetch_sub(n, Ordering::Relaxed);
    }

    fn update(&self, f: impl Fn(&Snapshot) -> Option<Snapshot>) {
        self.state
            .rcu(|cur| f(cur).map_or_else(|| Arc::clone(cur), Arc::new));
    }

    /// Appends `count` lines of `year` from `first_seq` (`bytes` LF-terminated, `ends` the end
    /// offset of each), the last at `last_tick`. The writer's only publication on the tick path.
    /// A chunk that does not follow `next_seq`, or of a year other than the open one, is ignored
    /// (the writer assigns seqs contiguously; this never happens).
    pub fn append(
        &self,
        year: i64,
        first_seq: u64,
        bytes: Vec<u8>,
        ends: Vec<u32>,
        last_tick: i64,
    ) {
        if ends.is_empty() {
            return;
        }
        let chunk = Arc::new(Chunk {
            year,
            first_seq,
            bytes,
            ends,
        });
        self.update(|cur| {
            if cur.next_seq != first_seq || cur.open_year != year {
                return None;
            }
            let mut next = cur.clone();
            // C1: copy at most TAIL pointers; a full tail merges into one block of the open year.
            let mut tail = Vec::with_capacity(TAIL);
            tail.extend(cur.tail.iter().cloned());
            tail.push(Arc::clone(&chunk));
            if tail.len() >= TAIL
                && let Some(block) = Chunk::merged(&tail)
            {
                let mut open = (*cur.open).clone();
                open.push(Arc::new(block));
                next.open = Arc::new(open);
                tail.clear();
            }
            next.tail = Arc::new(tail);
            next.next_seq = first_seq + chunk.count();
            next.tip_tick = Some(last_tick);
            Some(next)
        });
    }

    /// Sim-year `year` is complete: the next record is the next year's (`next_seq` is its first).
    /// Under (b), the previous year leaves memory now if its seal has landed.
    pub fn year_completes(&self, year: i64) {
        self.update(|cur| {
            if cur.open_year != year {
                return None;
            }
            let mut next = cur.clone();
            // C1: the year's blocks and tail become one chunk of the completed years.
            let parts: Vec<Arc<Chunk>> = cur.open.iter().chain(cur.tail.iter()).cloned().collect();
            if !parts.is_empty() {
                let mut chunks = (*cur.chunks).clone();
                match Chunk::merged(&parts) {
                    Some(one) => chunks.push(Arc::new(one)),
                    None => chunks.extend(parts),
                }
                next.chunks = Arc::new(chunks);
                next.open = Arc::new(Vec::new());
                next.tail = Arc::new(Vec::new());
            }
            next.completed = Some(year);
            next.open_year = year + 1;
            let mut firsts = (*cur.year_first).clone();
            firsts.push((year + 1, cur.next_seq));
            next.year_first = Arc::new(firsts);
            Some(next.dropped().unwrap_or(next))
        });
    }

    /// The first step of a landed seal (I4): `s` (a non-empty year), `last_year` and the seal log
    /// are published; `w0` does not move. `entry` `None`: a year with nothing to seal (a channel-A
    /// year with no record), which is on disk trivially and joins no log.
    pub fn publish_seal(&self, year: i64, entry: Option<&SealEntry>) {
        self.update(|cur| {
            if cur.sealed_through.is_some_and(|t| t >= year) {
                return None;
            }
            let mut next = cur.clone();
            next.sealed_through = Some(year);
            if let Some(e) = entry {
                if let Some(last) = e.last_seq {
                    next.s = Some(last);
                }
                next.last_year = Some(e.sim_year);
                let mut log = (*cur.seal_log).clone();
                log.push(e.clone());
                next.seal_log = Arc::new(log);
            }
            Some(next)
        });
    }

    /// The second step (I4): every year the policy lets go, now that its seal has landed, leaves
    /// memory and `w0` moves.
    pub fn drop_sealed(&self) {
        self.update(Snapshot::dropped);
    }

    /// A seal landed: [`Self::publish_seal`], then [`Self::drop_sealed`], in I4's order.
    pub fn seal_landed(&self, year: i64, entry: Option<&SealEntry>) {
        self.publish_seal(year, entry);
        self.drop_sealed();
    }

    /// The writer stopped (`stream.end`).
    pub fn end(&self, end: End) {
        self.update(|cur| {
            let mut next = cur.clone();
            next.end = Some(end);
            Some(next)
        });
    }

    /// A sifter's position on channel A.
    pub fn set_upstream(&self, upstream: Upstream) {
        self.update(|cur| {
            let mut next = cur.clone();
            next.upstream = Some(upstream);
            Some(next)
        });
    }
}
