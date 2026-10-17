//! The host's half of World Stream v0 (SPR-013 T-09): feeding `rh-stream`'s channel-A writer from
//! a golden run, and the run directory it writes.
//!
//! **Feeding.** After each instant (the `advance_until` callback) the host takes the life-course
//! delta ([`rh_stream::convert::LifeTracker`]); at each stop of the run loop it drains the events
//! (`Engine::drain_events`), adds the journal rows applied up to the stop (`INPUT`, at the row's
//! tick), a `STEP_DIGEST` when the stop is a Month pulse (living count and the three plane hashes
//! after `Bookkeeping`, B and C being the empty-plane constants in P1a; the writer fills the births
//! and deaths) and a `KEYFRAME` marker when it
//! writes a keyframe, and pushes them all to the writer, which orders them by R4 and seals each
//! sim-year as it completes. The genesis founders' `LIFE_ENTRY` records are at tick 0.
//!
//! **Keyframes.** A run with a run directory writes a keyframe at every permanent year end
//! (`KeyframeStore`'s 10-year cadence), its marker in that year's last instant: the marker is
//! pushed, the year's segment seals, and only then is the keyframe file written, atomically. A
//! keyframe on disk therefore always has its year's sealed segment before it, which is what a
//! resume needs. A run directory holds no genesis keyframe (year 0): a marker at tick 0 would sort
//! after that instant's records while the state is the one before them, so every keyframe in a
//! run directory has its marker, and a tick of years 0-9 is rebuilt from genesis, which the history
//! tuple determines (the Architect's ruling on SPR-013 deviation 6). A full run's `--keyframes`
//! directory (SPR-008) keeps its year-0 file.
//!
//! **The run directory** (`--run-dir D`): `D/run.toml` (what the tools that read the stream need:
//! the world, the seed, the horizon, the calendar and the history tuple), `D/journal.sqlite`,
//! `D/keyframes/year-NNNN.rhkf`, `D/run.ladder` (unless `--out` names another file) and
//! `D/stream/t0000000000/A/y######/`. The engine never reads it: it is the host's.
//!
//! **Test hooks** (`--test-stall`, never on a baseline): the run prints a `STALL` line and blocks
//! after writing file `n` of a segment (`seal:<year>:<n>`), after sealing it (`sealed:<year>`),
//! or after the first push at or past a tick (`mid:<tick>`, naming the open year and its buffered
//! records), so a test can `kill -9` it there (GOV-002 tier 15; PR #60).
//!
//! REF: `CODEX/05_PROJECT/SPR-013_WalkingSkeletonC.md` T-09; SPR-011 ruling 13; `05.10` §5, §7.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use rh_core::engine::Engine;
use rh_core::time::{Calendar, SimTime};
use rh_core::world::WorldTables;
use rh_snapshot::hash::{Plane, empty_plane_hash};
use rh_snapshot::journal::JournaledInput;
use rh_snapshot::keyframe::{KEYFRAME_FORMAT, KeyframeHeader};
use rh_snapshot::state::state_hash;
use rh_stream::convert::{LifeTracker, event_body, input_body};
use rh_stream::record::{Body, Channel, Keyframe, StepDigest};
use rh_stream::seal::SealHook;
use rh_stream::segment::{AttributeEntry, Dictionaries, Provenance, VerbEntry};
use rh_stream::writer::{StreamWriter, Summary, WriterConfig};

use crate::run::Error;

/// The timeline every golden run streams.
pub const TIMELINE: u32 = 0;
/// The run directory's description of the run.
pub const RUN_TOML: &str = "run.toml";
/// Its format line.
pub const RUN_FORMAT: &str = "rehoboam/run/v0";

/// Where a stall hook blocks (`--test-stall`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stall {
    /// After file `index` (from 0) of channel A's segment of sim-year `year`.
    Seal {
        /// The sim-year.
        year: i64,
        /// The file index.
        index: usize,
    },
    /// After channel A's segment of sim-year `year` is sealed.
    Sealed {
        /// The sim-year.
        year: i64,
    },
    /// After the first push at or past `tick`: a kill there is mid-year, with the open year's
    /// records in memory and unsealed (REV-SPR-013 MIN-10: the line names the tick and the
    /// buffered records, so a test can prove it).
    Mid {
        /// The tick.
        tick: i64,
    },
}

impl Stall {
    /// Parses `seal:<year>:<n>`, `sealed:<year>` or `mid:<tick>`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        match parts.as_slice() {
            ["seal", y, n] => Some(Self::Seal {
                year: y.parse().ok()?,
                index: n.parse().ok()?,
            }),
            ["sealed", y] => Some(Self::Sealed {
                year: y.parse().ok()?,
            }),
            ["mid", t] => Some(Self::Mid {
                tick: t.parse().ok()?,
            }),
            _ => None,
        }
    }
}

/// The seal hook of a run: nothing, or a stall.
pub struct Hook(pub Option<Stall>);

impl Hook {
    #[allow(
        clippy::infinite_loop,
        reason = "a test hook: it blocks until the test kills the process"
    )]
    fn stall(what: &str) -> ! {
        println!("STALL {what}");
        let _ = std::io::stdout().flush();
        loop {
            std::thread::park();
        }
    }
}

impl SealHook for Hook {
    fn file_written(&mut self, channel: Channel, year: i64, index: usize, name: &str) {
        if channel == Channel::A && self.0 == Some(Stall::Seal { year, index }) {
            Self::stall(&format!("seal y{year:06} file {index} {name}"));
        }
    }

    fn sealed(&mut self, channel: Channel, year: i64) {
        if channel == Channel::A && self.0 == Some(Stall::Sealed { year }) {
            Self::stall(&format!("sealed y{year:06}"));
        }
    }
}

/// The writer's lock file of a run directory: a run holds it exclusively for its whole length,
/// and the OS releases it when the run ends or is killed (REV-SPR-013 MIN-9).
pub const WRITER_LOCK: &str = ".writer.lock";

/// Takes the run directory's writer lock, refusing a second writer.
///
/// # Errors
/// [`Error`] if the lock cannot be taken (another run writes this directory) or the file cannot
/// be opened.
pub fn lock_writer(dir: &Path) -> Result<std::fs::File, Error> {
    let path = dir.join(WRITER_LOCK);
    let f = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|e| Error(format!("{}: {e}", path.display())))?;
    f.try_lock().map_err(|_| {
        Error(format!(
            "{}: another run is writing this run directory",
            dir.display()
        ))
    })?;
    Ok(f)
}

/// Whether a run directory's writer is gone: its lock file exists and is not held. Before a run
/// has started (no lock file yet) it is not gone.
#[must_use]
pub fn writer_gone(dir: &Path) -> bool {
    std::fs::File::open(dir.join(WRITER_LOCK)).is_ok_and(|f| f.try_lock_shared().is_ok())
}

/// Writes `bytes` to `path` atomically: a temporary sibling, fsync, rename, fsync of the
/// directory. The host's files of a run directory (`run.toml`, keyframes and their ladder
/// sidecars); `rh-stream` writes only sealed segments (REV-SPR-013 MIN-3).
///
/// # Errors
/// [`Error`] naming the path.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let fail = |e: std::io::Error| Error(format!("{}: {e}", path.display()));
    let dir = path
        .parent()
        .ok_or_else(|| Error(format!("{}: no parent directory", path.display())))?;
    std::fs::create_dir_all(dir).map_err(fail)?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error(format!("{}: no file name", path.display())))?;
    let tmp = dir.join(format!(".{name}.tmp"));
    {
        let mut f = std::fs::File::create(&tmp).map_err(fail)?;
        f.write_all(bytes).map_err(fail)?;
        f.sync_all().map_err(fail)?;
    }
    std::fs::rename(&tmp, path).map_err(fail)?;
    std::fs::File::open(dir)
        .and_then(|d| d.sync_all())
        .map_err(fail)
}

/// The dictionaries of a Registry (R6): `[verbs]` sorted by name and `[act_classes].ids`.
///
/// # Errors
/// [`Error`] if a verb names an act class the Registry does not list, or there are too many.
pub fn dictionaries(registry: &rh_manifest::Registry) -> Result<Dictionaries, Error> {
    let classes = &registry.act_classes.ids;
    let verbs = registry
        .verbs
        .iter()
        .enumerate()
        .map(|(i, (name, v))| {
            let mut bits = 0u64;
            for c in &v.act_classes {
                let bit = classes
                    .iter()
                    .position(|x| x == c)
                    .filter(|&b| b < 64)
                    .ok_or_else(|| Error(format!("verb {name}: act class {c} is not listed")))?;
                bits |= 1 << bit;
            }
            Ok(VerbEntry {
                verb_id: u16::try_from(i).map_err(|_| Error("too many verbs".to_owned()))?,
                name: name.clone(),
                act_classes: bits,
                tellability: v.tellability,
                emitted_by: v.emitted_by.clone(),
                aggregate: v.aggregate,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let ids = |section: &str| -> Result<Vec<String>, Error> {
        registry
            .sections
            .get(section)
            .and_then(|v| v.get("ids"))
            .and_then(toml::Value::as_array)
            .ok_or_else(|| Error(format!("the Registry has no [{section}] ids")))?
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| Error(format!("[{section}]: an id that is not a string")))
            })
            .collect()
    };
    Ok(Dictionaries {
        registry_version: registry.registry_version.clone(),
        verbs,
        act_classes: classes.clone(),
        event_roles: ids("event_roles")?,
        motive_edges: ids("motive_edges")?,
        // `[attributes]` sorted by name: the map's order (envelope_v0.md §2).
        attributes: registry
            .attributes
            .iter()
            .map(|(name, a)| AttributeEntry {
                name: name.clone(),
                ty: a.ty.clone(),
            })
            .collect(),
    })
}

/// The run directory's layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunDir(pub PathBuf);

impl RunDir {
    /// `stream/`.
    #[must_use]
    pub fn stream(&self) -> PathBuf {
        self.0.join("stream")
    }
    /// `journal.sqlite`.
    #[must_use]
    pub fn journal(&self) -> PathBuf {
        self.0.join("journal.sqlite")
    }
    /// `keyframes/`.
    #[must_use]
    pub fn keyframes(&self) -> PathBuf {
        self.0.join("keyframes")
    }
    /// `run.ladder`.
    #[must_use]
    pub fn ladder(&self) -> PathBuf {
        self.0.join("run.ladder")
    }
    /// `run.toml`.
    #[must_use]
    pub fn run_toml(&self) -> PathBuf {
        self.0.join(RUN_TOML)
    }
}

/// The keyframe of the end of `year`, relative to the run directory.
#[must_use]
pub fn keyframe_rel(year: i64) -> String {
    format!("keyframes/year-{year:04}.rhkf")
}

/// What `run.toml` says (written by the run, read by `tail`, `sift` and `replay`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunToml {
    /// [`RUN_FORMAT`].
    pub format: String,
    /// The world (`worlds/<world>`).
    pub world: String,
    /// The seed.
    pub seed: u64,
    /// The horizon in sim-years.
    pub years: i64,
    /// The calendar's minutes per year.
    pub minutes_per_year: i64,
    /// The engine build.
    pub engine_build: String,
    /// The Registry version.
    pub registry_version: String,
    /// The manifest hash.
    pub manifest_hash: String,
    /// The input script, relative to the repository, if the run took one other than the world's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inputs: Option<String>,
    /// The journal's input count when the run started (SPR-013 T-12): a replay refuses a journal
    /// that is not this one, even a self-consistent one with a row added or removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal_inputs: Option<u32>,
    /// The journal's chain head then (hex).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal_head: Option<String>,
    /// The ladder's rung kind (`yearly` or `monthly`), so a replay writes a comparable ladder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rung: Option<String>,
}

impl RunToml {
    /// Reads `run.toml` from a run directory.
    ///
    /// # Errors
    /// [`Error`] if it cannot be read or parsed, or is not format v0.
    pub fn read(dir: &Path) -> Result<Self, Error> {
        let path = dir.join(RUN_TOML);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| Error(format!("{}: {e}", path.display())))?;
        let t: Self =
            toml::from_str(&text).map_err(|e| Error(format!("{}: {e}", path.display())))?;
        if t.format != RUN_FORMAT {
            return Err(Error(format!(
                "{}: format {}, not {RUN_FORMAT}",
                path.display(),
                t.format
            )));
        }
        Ok(t)
    }

    /// Writes it atomically.
    ///
    /// # Errors
    /// [`Error`] if it cannot be written.
    pub fn write(&self, dir: &Path) -> Result<(), Error> {
        let text = toml::to_string(self).map_err(|e| Error(format!("run.toml: {e}")))?;
        write_atomic(&dir.join(RUN_TOML), text.as_bytes())
    }
}

/// A run's stream: the writer and what the host collects between stops (module docs).
pub struct StreamHost {
    writer: StreamWriter,
    life: LifeTracker,
    dictionaries: Dictionaries,
    /// The journal rows, by (tick, seq), and how many have been emitted.
    inputs: Vec<JournaledInput>,
    next_input: usize,
    /// Records collected since the last push.
    pending: Vec<(i64, Body)>,
    /// The first error of the per-instant callback.
    failed: Option<Error>,
    hook: Hook,
}

fn stream_error(e: &rh_stream::StreamError) -> Error {
    Error(format!("stream: {e}"))
}

impl StreamHost {
    /// The stream of a new run whose genesis tables are `genesis`: the founders' `LIFE_ENTRY`
    /// records at tick 0, and every journal row to come.
    ///
    /// # Errors
    /// [`Error`] if the writer cannot be created or the tables have no life-course log.
    pub fn create(
        cfg: WriterConfig,
        genesis: &WorldTables,
        mut inputs: Vec<JournaledInput>,
        hook: Hook,
    ) -> Result<Self, Error> {
        let dictionaries = cfg.dictionaries.clone();
        let writer = StreamWriter::create(cfg).map_err(|e| stream_error(&e))?;
        let mut life = LifeTracker::new();
        let pending = life.observe(genesis).map_err(|e| stream_error(&e))?;
        inputs.sort_by_key(|r| (r.tick, r.seq));
        Ok(Self {
            writer,
            life,
            dictionaries,
            inputs,
            next_input: 0,
            pending,
            failed: None,
            hook,
        })
    }

    /// The stream of a run resumed after `tick` (a keyframe's: the last minute of a sim-year, or
    /// -1 for the genesis keyframe) with the restored `tables`; journal rows at or before `tick`
    /// are in the sealed segments already.
    ///
    /// # Errors
    /// [`Error`] if the stream cannot resume there.
    pub fn resume(
        cfg: WriterConfig,
        tick: i64,
        tables: &WorldTables,
        mut inputs: Vec<JournaledInput>,
        hook: Hook,
    ) -> Result<Self, Error> {
        let dictionaries = cfg.dictionaries.clone();
        let writer = StreamWriter::resume(cfg, tick).map_err(|e| stream_error(&e))?;
        let (life, pending) = if tick < 0 {
            let mut life = LifeTracker::new();
            let pending = life.observe(tables).map_err(|e| stream_error(&e))?;
            (life, pending)
        } else {
            (
                LifeTracker::resumed(tables).map_err(|e| stream_error(&e))?,
                Vec::new(),
            )
        };
        inputs.sort_by_key(|r| (r.tick, r.seq));
        let next_input = inputs
            .iter()
            .take_while(|r| r.tick.minutes() <= tick)
            .count();
        Ok(Self {
            writer,
            life,
            dictionaries,
            inputs,
            next_input,
            pending,
            failed: None,
            hook,
        })
    }

    /// The per-instant callback: the life-course delta of the instant.
    pub fn after_instant(&mut self, tables: &WorldTables) {
        if self.failed.is_some() {
            return;
        }
        match self.life.observe(tables) {
            Ok(records) => self.pending.extend(records),
            Err(e) => self.failed = Some(stream_error(&e)),
        }
    }

    /// The first half of the stop at `at`: drains the events (before any keyframe is taken, so
    /// that a keyframe never carries events the stream already holds), and collects the applied
    /// inputs and, when `digest`, the Month pulse's `STEP_DIGEST`.
    ///
    /// # Errors
    /// [`Error`] from the per-instant callback or an event that is not a record.
    pub fn collect(&mut self, engine: &mut Engine, at: SimTime, digest: bool) -> Result<(), Error> {
        if let Some(e) = self.failed.take() {
            return Err(e);
        }
        let t = at.minutes();
        for e in engine.drain_events() {
            let body = event_body(&e, &self.dictionaries).map_err(|e| stream_error(&e))?;
            self.pending.push((e.tick, body));
        }
        while let Some(row) = self
            .inputs
            .get(self.next_input)
            .filter(|r| r.tick.minutes() <= t)
        {
            self.pending.push((row.tick.minutes(), input_body(row)));
            self.next_input += 1;
        }
        if digest {
            let state = engine.snapshot();
            let living = engine
                .tables()
                .get(rh_core::ledger::PERSON_ID)
                .map_or(0, Vec::len);
            self.pending.push((
                t,
                Body::StepDigest(StepDigest {
                    living: u64::try_from(living).unwrap_or(u64::MAX),
                    births: 0,
                    deaths: 0,
                    state_hash: state_hash(&state),
                    // Planes B and C hold nothing the engine hashes in P1a, so these are the
                    // empty-plane constants of rh-snapshot (REV-SPR-013 NIT): equal in every digest.
                    observation_hash: empty_plane_hash(Plane::B),
                    narrative_hash: empty_plane_hash(Plane::C),
                }),
            ));
        }
        Ok(())
    }

    /// The second half of the stop at `at`: adds the keyframe marker, if a keyframe was taken
    /// there, and pushes everything collected to the writer.
    ///
    /// # Errors
    /// [`Error`] from the writer.
    pub fn push(
        &mut self,
        at: SimTime,
        keyframe: Option<(&str, &KeyframeHeader)>,
    ) -> Result<(), Error> {
        let t = at.minutes();
        let mut batch = std::mem::take(&mut self.pending);
        if let Some((path, header)) = keyframe {
            batch.push((
                t,
                Body::Keyframe(Keyframe {
                    keyframe_format: KEYFRAME_FORMAT.to_owned(),
                    keyframe_path: path.to_owned(),
                    state_hash: header.state_hash,
                    journal_position: u64::from(header.journal.inputs),
                }),
            ));
        }
        self.writer
            .push(t, batch, &mut self.hook)
            .map_err(|e| stream_error(&e))?;
        if let Some(Stall::Mid { tick }) = self.hook.0
            && t >= tick
        {
            Hook::stall(&format!(
                "mid tick {t} sim-year {} buffered {}",
                self.writer.open_year(),
                self.writer.buffered()
            ));
        }
        Ok(())
    }

    /// Seals the open year and reports.
    ///
    /// # Errors
    /// [`Error`] from the writer.
    pub fn finish(mut self) -> Result<Summary, Error> {
        if let Some(e) = self.failed.take() {
            return Err(e);
        }
        self.writer
            .finish(&mut self.hook)
            .map_err(|e| stream_error(&e))
    }
}

/// The writer configuration of a run of `world` streaming into `run_dir`.
#[must_use]
pub fn writer_config(
    run_dir: &RunDir,
    calendar: Calendar,
    provenance: Provenance,
    dictionaries: Dictionaries,
) -> WriterConfig {
    WriterConfig {
        root: run_dir.stream(),
        timeline_id: TIMELINE,
        minutes_per_year: calendar.minutes_per_year(),
        provenance,
        dictionaries,
    }
}
