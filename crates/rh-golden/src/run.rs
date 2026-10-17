//! The smoke runner: a full run, or a resumed one, writing its hash ladder (SPR-008 T-02).
//!
//! **A full run** builds the smoke engine from `worlds/smoke` (the specs and the Registry
//! overlay; registration compares the code with them), creates the input journal, and runs to
//! the end of year `years`, taking a rung at every year or month end. Before the first instant it
//! journals every synthetic input of the horizon, in the host order SPR-007 fixes: check the
//! input against the engine's cursor, append it to the journal, then submit the journal row's
//! engine input (`rh_snapshot::resume::material_input`), so journal order is acceptance order and
//! every row is exactly one engine input. With a keyframe directory it writes a keyframe at the
//! start and at every permanent year (`KeyframeStore`'s 10-year cadence), anchored at the
//! journal's position.
//!
//! DECISION: the inputs are journaled up front, not as the run reaches them. `resume` submits
//! every journal row after a keyframe's anchor at once, so a run that journaled lazily would hold
//! different queued inputs from the run resumed from its keyframe, and the two ladders would
//! differ (the engine's queued inputs are in `state_hash`). Journaled up front, a resumed run
//! equals the continuous one rung for rung, and a run that fails cannot leave a shorter journal.
//! The inputs depend on the horizon, which the ladder header records: ladders of two horizons
//! are not compared. ALTERNATIVE CONSIDERED: journaling each input when the run reaches its year,
//! which is closer to a live host, but breaks resume equality without a second replay loop.
//!
//! **A resumed run** loads a keyframe through `rh_snapshot::resume::resume`, which checks the
//! keyframe's anchor against the journal (`Replay::check_anchor`) and submits the journal's rows
//! after it, and runs on to year `years`. It journals nothing: its inputs are the journal's.
//! There is no second replay loop (Architect ruling on VER-SPR-007-TEST GAP-7).
//!
//! **Test hooks** for the bisect (V-DET-11), never used for a baseline:
//! - `inject`: at the end of month `month`, flip the low bit of row `row` of world table `table`
//!   (through `Engine::snapshot` and `Engine::restore`, the journal untouched), then take the
//!   rung. The perturbed run may later break an invariant; the ladder then ends in `failed`.
//! - `dump`: at the end of month `month`, write the whole plane-A state ([`crate::dump`]).
//! - `leak_thread`: `X7.smoke_market` runs inside [`crate::leak::Leaky`], which hashes a
//!   per-thread step counter into `market.ticks`, and the ladder's header says `leak thread`. The
//!   1-versus-N ladder comparison must fail on it (VER-SPR-008 DEF-2's planted control).
//!
//! **Dispatch evidence** (DEF-2): [`run_reporting`] also returns the scheduler's
//! [`DispatchStats`] over the run (levels, levels with two or more jobs, the most jobs in a
//! level). They go in the run's summary, never in the ladder: they do not depend on the thread
//! count, but they describe the dispatch, not the history. `require_parallel` makes the run fail
//! when no level held two jobs or the share of such levels is below the floor.
//!
//! REF: `CODEX/05_PROJECT/SPR-008_GoldenRunCI.md` T-02, T-03, rulings 3, 4 and 7; `05.1` §6;
//! ADR-0025 item 3; SPR-007 T-02, T-03 (the journal position counts accepted inputs).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use rh_core::crash::Provenance;
use rh_core::engine::{DispatchStats, Engine};
use rh_core::ownership::SpecTable;
use rh_core::register::register;
use rh_core::system::SystemModel;
use rh_core::time::{Calendar, Pulse, SimTime};
use rh_ids::BranchSalt;
use rh_manifest::specs::{load_specs, registry_system_ids};
use rh_registry::ownership::Ownership;
use rh_snapshot::hash::{LAYOUT, PlaneDigest, xxh3};
use rh_snapshot::journal::{
    HistoryKey, InputKind, Journal, JournalPosition, NewInput, Producer, Replay, Scope,
};
use rh_snapshot::keyframe::{History, write_keyframe};
use rh_snapshot::resume::{material_input, resume};
use rh_snapshot::state::state_digest;
use rh_snapshot::store::KeyframeStore;
use rh_sys_smoke::{CALENDAR, OVERLAY, SPECS_DIR};

use std::sync::Arc;

use crate::dump;
use crate::ladder::{Header, Ladder, Rung, RungKind};
use crate::stream::{Hook, RunDir, RunToml, Stall, StreamHost, keyframe_rel, writer_config};
use crate::vitals::{Timing, Vitals};
use crate::world::{SMOKE, Setup, setup};
use rh_snapshot::journal::JournaledInput;
use rh_stream::writer::Summary as StreamSummary;

/// The world name in ladders.
pub const WORLD: &str = "smoke";
/// The timeline every smoke run uses.
const TIMELINE: u32 = 0;

/// A golden-run failure that is not the simulation's (that one ends the ladder in `failed`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn error(context: &str) -> impl Fn(&dyn fmt::Display) -> Error + '_ {
    move |e| Error(format!("{context}: {e}"))
}

/// A one-row perturbation (V-DET-11 test hook).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inject {
    /// The month (1-based, counted from the epoch) at whose end the row is flipped.
    pub month: u64,
    /// The world table.
    pub table: String,
    /// The row.
    pub row: usize,
}

/// What to run.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each bool is one command-line flag"
)]
pub struct RunArgs {
    /// The world: `smoke`, or a manifest world under `worlds/` (SPR-012 T-08; [`crate::world`]).
    pub world: String,
    /// The repository root, where `worlds/` is.
    pub repo: PathBuf,
    /// The world seed. A manifest world takes its manifest's unless `seed_set`.
    pub seed: u64,
    /// Whether `--seed` was given (a manifest world's seed is otherwise its manifest's).
    pub seed_set: bool,
    /// Refuse a manifest that names a system this build lacks (`--strict`; SPR-011 ruling 3).
    pub strict: bool,
    /// The horizon: the history is the smoke world's synthetic inputs of this many sim-years, and
    /// the run ends at the end of this year unless `until` stops it earlier.
    pub years: i64,
    /// Stop at the end of this sim-year (at most `years`), keeping the horizon's history: the
    /// bisect's resumed runs stop at the divergent year (VER-SPR-008 DEF-1).
    pub until: Option<i64>,
    /// A resumed run may go past the journal's last input (DEF-1): without it, `years` must be
    /// the journal's own horizon.
    pub extend: bool,
    /// Worker threads.
    pub threads: usize,
    /// Rung kind.
    pub rung: RungKind,
    /// Where the ladder is written.
    pub out: PathBuf,
    /// The input journal: created by a full run (replacing any file there), read by a resumed one.
    pub journal: PathBuf,
    /// Where a full run writes its keyframes, if anywhere.
    pub keyframes: Option<PathBuf>,
    /// The keyframe a resumed run starts from.
    pub resume: Option<PathBuf>,
    /// The V-DET-11 perturbation.
    pub inject: Option<Inject>,
    /// `(month, path)`: dump the state at the end of that month.
    pub dump: Option<(u64, PathBuf)>,
    /// The planted thread-identity leak (DEF-2 control).
    pub leak_thread: bool,
    /// The floor, in percent, of the share of levels that held two or more jobs (DEF-2): below it,
    /// or with no such level at all, the run fails.
    pub require_parallel: Option<u64>,
    /// Attach the timing observer and report the vitals (SPR-009 T-03): per-system times, the
    /// serial fraction and the vitals line. It changes no result.
    pub vitals: bool,
    /// The run directory (SPR-013 T-09; [`crate::stream`]): the World Stream, the journal, the
    /// keyframes and `run.toml`. A manifest world only.
    pub run_dir: Option<PathBuf>,
    /// A test hook that blocks the run while it seals (never on a baseline).
    pub stall: Option<Stall>,
}

impl RunArgs {
    /// A full run of `years` with `threads` and `rung`, writing `out` and a journal next to it.
    #[must_use]
    pub fn new(repo: &Path, years: i64, threads: usize, rung: RungKind, out: &Path) -> Self {
        Self {
            world: SMOKE.to_owned(),
            repo: repo.to_path_buf(),
            seed: 1,
            seed_set: false,
            strict: false,
            years,
            threads,
            rung,
            out: out.to_path_buf(),
            journal: out.with_extension("journal.sqlite"),
            until: None,
            extend: false,
            keyframes: None,
            resume: None,
            inject: None,
            dump: None,
            leak_thread: false,
            require_parallel: None,
            vitals: false,
            run_dir: None,
            stall: None,
        }
    }
}

/// What a run leaves: its ladder and what the scheduler dispatched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The ladder, as written.
    pub ladder: Ladder,
    /// The dispatch over every instant the run ran (a resumed run: from its keyframe).
    pub dispatch: DispatchStats,
    /// The vitals, when the run was asked for them (`--vitals`; SPR-009 T-03).
    pub vitals: Option<Vitals>,
    /// What the stream writer did, for a run with a run directory (SPR-013 T-09).
    pub stream: Option<StreamSummary>,
}

impl Outcome {
    /// The share of levels that held two or more jobs, in whole percent (rounded down).
    #[must_use]
    pub const fn parallel_percent(&self) -> u64 {
        match self.dispatch.levels {
            0 => 0,
            n => self.dispatch.parallel_levels * 100 / n,
        }
    }

    /// `Err` naming the shortfall when the run's parallel levels fall short of `floor` percent, or
    /// when no level held two or more jobs (DEF-2).
    ///
    /// # Errors
    /// The message to print.
    pub fn check_parallel(&self, floor: u64) -> Result<(), String> {
        let d = self.dispatch;
        if d.parallel_levels == 0 || d.max_jobs < 2 {
            return Err(format!(
                "PARALLEL LEVELS MISSING: no level held two or more jobs ({} levels), so a \
                 thread-count comparison is vacuous (VER-SPR-008 DEF-2)",
                d.levels
            ));
        }
        if self.parallel_percent() < floor {
            return Err(format!(
                "PARALLEL LEVELS BELOW THE FLOOR: {} of {} levels ({}%) held two or more jobs; \
                 the floor is {floor}% (VER-SPR-008 DEF-2)",
                d.parallel_levels,
                d.levels,
                self.parallel_percent()
            ));
        }
        Ok(())
    }
}

/// The smoke world's systems; with `leak`, `X7.smoke_market` inside [`crate::leak::Leaky`].
#[must_use]
pub fn smoke_systems(leak: bool) -> Vec<Box<dyn SystemModel>> {
    rh_sys_smoke::systems()
        .into_iter()
        .map(|s| {
            if leak && s.descriptor().id == "X7.smoke_market" {
                Box::new(crate::leak::Leaky(s)) as Box<dyn SystemModel>
            } else {
                s
            }
        })
        .collect()
}

/// The provenance every smoke keyframe and journal carries. It never enters a ladder.
#[must_use]
pub fn provenance() -> Provenance {
    Provenance {
        build: format!("rh-golden {}", env!("CARGO_PKG_VERSION")),
        registry_version: "smoke-overlay v1".to_owned(),
        manifest_hash: WORLD.to_owned(),
    }
}

/// The history tuple of a smoke run with `seed`.
#[must_use]
pub fn history(seed: u64) -> History {
    History {
        provenance: provenance(),
        world_seed: seed,
        branch_salt: 0,
    }
}

fn journal_key(p: &Provenance, seed: u64) -> HistoryKey {
    let p = p.clone();
    HistoryKey {
        engine_build: p.build,
        registry_version: p.registry_version,
        manifest_hash: p.manifest_hash.into_bytes(),
        world_seed: seed,
    }
}

/// The smoke spec table, from `worlds/smoke` under `repo`, as `rh-cli ownership` builds one.
///
/// # Errors
/// [`Error`] if the specs or the overlay cannot be read.
pub fn spec_table(repo: &Path) -> Result<SpecTable, Error> {
    let ids = registry_system_ids(&repo.join(OVERLAY)).map_err(|e| error("overlay")(&e))?;
    let specs = load_specs(&repo.join(SPECS_DIR)).map_err(|e| error("specs")(&e))?;
    Ok(Ownership::build(&specs, &ids).spec_table())
}

/// The last minute of sim-year `year` (1-based), on the smoke calendar.
#[must_use]
pub fn year_end(year: i64) -> SimTime {
    year_end_in(CALENDAR, year)
}

fn year_end_in(calendar: Calendar, year: i64) -> SimTime {
    SimTime::from_minutes(year * calendar.minutes_per_year() - 1)
}

/// The last minute of every month (1-based from the epoch) up to the end of year `years`, on the
/// smoke calendar.
#[must_use]
pub fn month_ends(years: i64) -> Vec<SimTime> {
    month_ends_in(CALENDAR, years)
}

fn month_ends_in(calendar: Calendar, years: i64) -> Vec<SimTime> {
    let end = year_end_in(calendar, years);
    let mut out = Vec::new();
    let mut t = SimTime::EPOCH;
    while let Some(next) = calendar.next_boundary(Pulse::Month, t) {
        let last = SimTime::from_minutes(next.minutes() - 1);
        if last > end {
            break;
        }
        out.push(last);
        t = next;
    }
    out
}

/// `YYYY-MM` of the instant `t`, on the smoke calendar.
#[must_use]
pub fn date(t: SimTime) -> String {
    date_in(CALENDAR, t)
}

fn date_in(calendar: Calendar, t: SimTime) -> String {
    let d = calendar.to_date(t);
    format!("{:04}-{:02}", d.year, d.month)
}

/// One place the run stops: a rung, a perturbation, a dump, or a keyframe year.
#[derive(Clone, Debug, Default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each bool says what one stop of the run loop does"
)]
struct Stop {
    rung: Option<u64>,
    month_end: bool,
    /// A Month pulse instant: the stream's `STEP_DIGEST` (SPR-013 T-09).
    digest: bool,
    inject: bool,
    dump: bool,
    year_end: Option<i64>,
}

fn stops(args: &RunArgs, calendar: Calendar) -> BTreeMap<SimTime, Stop> {
    let mut stops: BTreeMap<SimTime, Stop> = BTreeMap::new();
    let last = args.until.unwrap_or(args.years).min(args.years);
    let months = month_ends_in(calendar, last);
    for year in 1..=last {
        let end = year_end_in(calendar, year);
        stops.entry(end).or_default().year_end = Some(year);
        if args.rung == RungKind::Yearly {
            stops.entry(end).or_default().rung = u64::try_from(year).ok();
        }
    }
    if args.run_dir.is_some() {
        // A Month pulse at the start of every month of the run: tick 0 and each month end + 1.
        let end = year_end_in(calendar, last);
        for t in std::iter::once(SimTime::EPOCH).chain(
            months
                .iter()
                .filter_map(|m| m.checked_add_minutes(1))
                .filter(|&t| t <= end),
        ) {
            stops.entry(t).or_default().digest = true;
        }
    }
    for (i, &t) in months.iter().enumerate() {
        let month = u64::try_from(i).unwrap_or(u64::MAX) + 1;
        stops.entry(t).or_default().month_end = true;
        if args.rung == RungKind::Monthly {
            stops.entry(t).or_default().rung = Some(month);
        }
        if args.inject.as_ref().is_some_and(|x| x.month == month) {
            stops.entry(t).or_default().inject = true;
        }
        if args.dump.as_ref().is_some_and(|(m, _)| *m == month) {
            stops.entry(t).or_default().dump = true;
        }
    }
    stops
}

/// Runs `args` and writes its ladder to `args.out`. A simulation failure is recorded in the
/// ladder (`failed`) and returned with it, not as an error.
///
/// # Errors
/// [`Error`] if the world, the journal, a keyframe or a file cannot be set up or written.
pub fn run(args: &RunArgs) -> Result<Ladder, Error> {
    run_reporting(args).map(|o| o.ladder)
}

/// [`run`], with the dispatch evidence.
///
/// # Errors
/// As [`run`].
#[allow(
    clippy::too_many_lines,
    reason = "the run loop in one place: every stop's steps in their order (ladder, keyframe, stream)"
)]
pub fn run_reporting(args: &RunArgs) -> Result<Outcome, Error> {
    refuse_mutants()?;
    let smoke = args.world == SMOKE;
    if args.leak_thread && !smoke {
        return Err(Error(format!(
            "--leak-thread plants its leak in the smoke world only, not {}",
            args.world
        )));
    }
    let seed = (smoke || args.seed_set).then_some(args.seed);
    let world = setup(&args.repo, &args.world, seed, args.strict)?;
    let registered = || {
        register((world.systems)(args.leak_thread), &world.specs)
            .map_err(|e| error("registration")(&e))
    };
    let run_dir = args.run_dir.clone().map(RunDir);
    if run_dir.is_some() && world.stream.is_none() {
        return Err(Error(format!(
            "--run-dir streams a manifest world; {} has no World Stream (SPR-013 T-09)",
            world.name
        )));
    }
    if let Some(dir) = &run_dir {
        std::fs::create_dir_all(&dir.0).map_err(|e| error("run directory")(&e))?;
    }
    let (mut engine, journal, identity, rows) = start(args, &world, &registered)?;
    // SPR-009 T-03: the host's clock-free observer; the engine carries no clock (ruling 4).
    let timing = args.vitals.then(|| Arc::new(Timing::default()));
    let attach = |e: &mut Engine| {
        if let Some(t) = &timing {
            e.set_observer(Some(t.clone()));
        }
    };
    attach(&mut engine);
    let start = engine.snapshot().cursor.unwrap_or(SimTime::EPOCH);
    let mut ladder = Ladder {
        header: Header {
            world: world.name.clone(),
            seed: world.seed,
            years: args.years,
            rung: args.rung,
            hash: LAYOUT.to_owned(),
            journal: None,
            until: None,
            inject: args
                .inject
                .as_ref()
                .map(|i| format!("{}:{}:{}", i.month, i.table, i.row)),
            leak: args.leak_thread,
            tuple: world.tuple.clone(),
            systems: world.systems_line.clone(),
        },
        rungs: Vec::new(),
        failed: None,
    };
    ladder.header.journal = identity.map(|p| (p.inputs, dump::hex(&p.head)));
    ladder.header.until = args.until.filter(|&u| u < args.years);
    let store = KeyframeStore::default();
    // Keyframes: the --keyframes directory of a full run (SPR-008), or the run directory's of
    // any streaming run, resumed ones included (their markers are part of channel A).
    let keyframes: Option<PathBuf> = run_dir
        .as_ref()
        .map(RunDir::keyframes)
        .or_else(|| args.keyframes.clone().filter(|_| journal.is_some()));
    let mut host = match (&run_dir, &world.stream) {
        (Some(dir), Some(info)) => {
            Some(stream_host(args, dir, &world, info, &engine, start, rows)?)
        }
        _ => None,
    };
    if let (Some(dir), Some(position), false) = (&keyframes, identity, args.resume.is_some()) {
        std::fs::create_dir_all(dir).map_err(|e| error("keyframes")(&e))?;
        let (bytes, _) = write_keyframe(&engine, position).map_err(|e| error("keyframe")(&e))?;
        write_file(&keyframe_path(dir, 0), &bytes)?;
    }
    let mut months: Vec<u8> = Vec::new();
    // A perturbation restores the engine, which starts its count again: keep the earlier part.
    let mut dispatched = DispatchStats::default();
    // A resumed run starts after its keyframe's own instant: the stops it has nothing to run up
    // to are the keyframe's state, which the continuous run already took its rung at.
    let resumed = args.resume.is_some();
    let first = stops(args, world.calendar)
        .into_keys()
        .find(|&t| t >= start && !(resumed && engine.next_instant().is_none_or(|n| n > t)));
    for (t, stop) in stops(args, world.calendar)
        .into_iter()
        .filter(|(t, _)| first.is_some_and(|f| *t >= f))
    {
        let advanced = match host.as_mut() {
            Some(h) => engine.advance_until(t, |_, tables| h.after_instant(tables)),
            None => engine.advance_until(t, |_, _| {}),
        };
        if let Err(e) = advanced {
            let tick = engine.crash_artifact().map_or(t.minutes(), |a| a.tick);
            ladder.failed = Some((tick, e.to_string()));
            host = None;
            break;
        }
        match host.as_mut() {
            Some(h) => h.collect(&mut engine, t, stop.digest)?,
            None => drop(engine.drain_events()),
        }
        if let Some(inject) = args.inject.as_ref().filter(|_| stop.inject) {
            dispatched = add(dispatched, engine.dispatch_stats());
            engine = perturb(&engine, inject, args, &world, registered()?)?;
            attach(&mut engine);
        }
        let digest = state_digest(&engine.snapshot());
        if stop.month_end {
            months.extend_from_slice(&digest.hash.to_le_bytes());
        }
        if let Some(index) = stop.rung {
            ladder
                .rungs
                .push(rung(&digest, index, t, xxh3(&months), world.calendar));
            months.clear();
        }
        if let Some((_, path)) = args.dump.as_ref().filter(|_| stop.dump) {
            dump::write(&engine.snapshot(), t, path)?;
        }
        // A permanent year's keyframe: its marker goes into the stream (which seals the year),
        // and only then is the file written (crate::stream module docs).
        let kf = match (stop.year_end, &keyframes, identity) {
            (Some(year), Some(_), Some(position)) if store.is_permanent_year(year) => Some((
                year,
                write_keyframe(&engine, position).map_err(|e| error("keyframe")(&e))?,
            )),
            _ => None,
        };
        if let Some(h) = host.as_mut() {
            let rel = kf.as_ref().map(|(year, _)| keyframe_rel(*year));
            let marker = rel
                .as_deref()
                .zip(kf.as_ref().map(|(_, (_, header))| header));
            h.push(t, marker)?;
        }
        if let (Some((year, (bytes, _))), Some(dir)) = (&kf, &keyframes) {
            write_file(&keyframe_path(dir, *year), bytes)?;
        }
    }
    let stream = host.map(StreamHost::finish).transpose()?;
    std::fs::write(&args.out, ladder.render()).map_err(|e| error("ladder")(&e))?;
    Ok(Outcome {
        ladder,
        dispatch: add(dispatched, engine.dispatch_stats()),
        vitals: timing.and_then(|t| t.vitals()),
        stream,
    })
}

/// Writes a run's file: atomically for the run directory's keyframes, so that a keyframe on disk
/// is whole (SPR-013 T-09).
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    rh_stream::seal::write_atomic(path, bytes).map_err(|e| error("keyframe")(&e))
}

/// The stream of a run into `dir` (SPR-013 T-09): `run.toml`, then a new stream at genesis or a
/// resumed one after the keyframe's year end.
fn stream_host(
    args: &RunArgs,
    dir: &RunDir,
    world: &Setup,
    info: &crate::world::StreamInfo,
    engine: &Engine,
    start: SimTime,
    rows: Vec<JournaledInput>,
) -> Result<StreamHost, Error> {
    std::fs::create_dir_all(&dir.0).map_err(|e| error("run directory")(&e))?;
    let cfg = writer_config(
        dir,
        world.calendar,
        info.provenance.clone(),
        info.dictionaries.clone(),
    );
    let hook = Hook(args.stall);
    if args.resume.is_some() {
        // Keyframes are taken at year ends; the restored cursor is the last instant run + 1, so the
        // resume point is the last minute of the sim-year that instant is in (-1 at genesis).
        let mpy = world.calendar.minutes_per_year();
        let tick = ((start.minutes() - 1).div_euclid(mpy) + 1) * mpy - 1;
        StreamHost::resume(cfg, tick, engine.tables(), rows, hook)
    } else {
        RunToml {
            format: crate::stream::RUN_FORMAT.to_owned(),
            world: world.name.clone(),
            seed: world.seed,
            years: args.years,
            minutes_per_year: world.calendar.minutes_per_year(),
            engine_build: world.provenance.build.clone(),
            registry_version: world.provenance.registry_version.clone(),
            manifest_hash: world.provenance.manifest_hash.clone(),
            inputs: None,
        }
        .write(&dir.0)?;
        StreamHost::create(cfg, engine.tables(), rows, hook)
    }
}

/// The `mutant-*` Cargo features compiled into this build, from every crate that has any
/// (BCK-002 A-52).
#[must_use]
pub fn mutant_features() -> Vec<&'static str> {
    rh_sys_demography::MUTANTS.to_vec()
}

/// BCK-002 A-52: a build with a `mutant-*` feature compiled in never runs a world, so a mutant can
/// never make a ladder, a baseline or a stream.
///
/// # Errors
/// [`Error`] naming the features.
pub fn refuse_mutants() -> Result<(), Error> {
    let mutants = mutant_features();
    if mutants.is_empty() {
        return Ok(());
    }
    Err(Error(format!(
        "MUTANT BUILD: this build has the planted control feature(s) {} compiled in; a mutant \
         never runs a world or makes a baseline (BCK-002 A-52). Rebuild without them",
        mutants.join(", ")
    )))
}

const fn add(a: DispatchStats, b: DispatchStats) -> DispatchStats {
    DispatchStats {
        levels: a.levels + b.levels,
        parallel_levels: a.parallel_levels + b.parallel_levels,
        max_jobs: if a.max_jobs > b.max_jobs {
            a.max_jobs
        } else {
            b.max_jobs
        },
    }
}

type Started = (
    Engine,
    Option<Journal>,
    Option<JournalPosition>,
    Vec<JournaledInput>,
);

/// The engine a run starts from, the journal a full run writes, and the journal's identity: a
/// full run creates the journal and journals every synthetic input of the horizon in the host
/// order; a resumed run checks the journal's horizon (DEF-1) and resumes through it.
fn start(
    args: &RunArgs,
    world: &Setup,
    registered: &dyn Fn() -> Result<rh_core::register::Registered, Error>,
) -> Result<Started, Error> {
    if args.until.is_some_and(|u| u < 0 || u > args.years) {
        return Err(Error(format!(
            "--until {} is outside the horizon of {} years",
            args.until.unwrap_or_default(),
            args.years
        )));
    }
    if let Some(keyframe) = &args.resume {
        let bytes = std::fs::read(keyframe).map_err(|e| error("keyframe")(&e))?;
        let replay = Replay::open(
            &args.journal,
            &journal_key(&world.provenance, world.seed),
            TIMELINE,
        )
        .map_err(|e| error("journal")(&e))?;
        check_horizon(&replay, args, world)?;
        let history = History {
            provenance: world.provenance.clone(),
            world_seed: world.seed,
            branch_salt: 0,
        };
        let (mut engine, _) = resume(&bytes, world.calendar, registered()?, &history, &replay)
            .map_err(|e| error("resume")(&e))?;
        engine.set_threads(args.threads);
        let rows = replay.inputs().to_vec();
        return Ok((engine, None, Some(replay.position()), rows));
    }
    for suffix in ["", "-wal", "-shm"] {
        let mut path = args.journal.clone().into_os_string();
        path.push(suffix);
        let _ = std::fs::remove_file(path);
    }
    let mut journal = Journal::create(&args.journal, &journal_key(&world.provenance, world.seed))
        .map_err(|e| error("journal")(&e))?;
    let mut engine = Engine::new(
        world.calendar,
        world.seed,
        BranchSalt::from_raw(0),
        registered()?,
        world.tables.clone(),
        SimTime::EPOCH,
    );
    engine.set_provenance(world.provenance.clone());
    engine.set_threads(args.threads);
    let mut rows = Vec::new();
    for (at, entity) in (world.inputs)(args.years) {
        rows.push(accept(&mut engine, &mut journal, at, entity)?);
    }
    let identity = journal
        .position(TIMELINE)
        .map_err(|e| error("journal")(&e))?;
    Ok((engine, Some(journal), Some(identity), rows))
}

/// DEF-1: a resumed run's horizon must be the journal's. The journal must hold exactly the
/// synthetic inputs of `args.years`, in order; with `--extend`, a strict prefix of them is
/// allowed too (the run goes past the journal's last input, and the ladder's journal line says
/// which journal it ran).
fn check_horizon(replay: &Replay, args: &RunArgs, world: &Setup) -> Result<(), Error> {
    let rows: Vec<(SimTime, Scope)> = replay.inputs().iter().map(|r| (r.tick, r.scope)).collect();
    let expected = (world.inputs)(args.years)
        .into_iter()
        .map(|(t, e)| {
            Scope::of_entity(e)
                .map(|s| (t, s))
                .ok_or_else(|| Error(format!("{e} has no journal scope")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if rows == expected {
        return Ok(());
    }
    let n = rows.len();
    if n < expected.len() && rows[..] == expected[..n] {
        if args.extend {
            return Ok(());
        }
        return Err(Error(format!(
            "--years {} goes past the journal's last input: it holds {n} of the {} inputs of a \
             {}-year history. Pass --extend to run past it on purpose (VER-SPR-008 DEF-1)",
            args.years,
            expected.len(),
            args.years
        )));
    }
    Err(Error(format!(
        "--years {} disagrees with the journal's horizon: the journal holds {n} inputs, not the \
         {} of a {}-year history. Resume with the journal's own horizon, and stop earlier with \
         --until (VER-SPR-008 DEF-1)",
        args.years,
        expected.len(),
        args.years
    )))
}

/// The host order (SPR-007): check `at` against the engine's cursor, journal the input, then
/// submit the journal row's engine input.
fn accept(
    engine: &mut Engine,
    journal: &mut Journal,
    at: SimTime,
    entity: rh_ids::EntityRef,
) -> Result<JournaledInput, Error> {
    if engine.snapshot().cursor.is_none_or(|c| at < c) {
        return Err(Error(format!(
            "the synthetic input at {at} is before the cursor"
        )));
    }
    let scope =
        Scope::of_entity(entity).ok_or_else(|| Error(format!("{entity} has no journal scope")))?;
    let row = journal
        .append(NewInput {
            timeline_id: TIMELINE,
            tick: at,
            kind: InputKind::OracleMove,
            scope,
            producer: Producer::System,
            payload: b"smoke synthetic material input".to_vec(),
        })
        .map_err(|e| error("journal")(&e))?;
    let input = material_input(&row)
        .ok_or_else(|| Error(format!("journal row {} is not material", row.seq)))?;
    engine
        .submit_input(at, input)
        .map_err(|e| error("input")(&e))?;
    let head = journal
        .position(TIMELINE)
        .map_err(|e| error("journal")(&e))?
        .head;
    engine.set_journal_head(Some(dump::hex(&head)));
    Ok(row)
}

/// Where a full run writes the keyframe of the end of `year` (year 0: the start).
#[must_use]
pub fn keyframe_path(dir: &Path, year: i64) -> PathBuf {
    dir.join(format!("year-{year:04}.rhkf"))
}

fn rung(digest: &PlaneDigest, index: u64, t: SimTime, months: u64, calendar: Calendar) -> Rung {
    Rung {
        index,
        tick: t.minutes(),
        date: date_in(calendar, t),
        hash: digest.hash,
        months,
        roots: digest
            .tables
            .iter()
            .map(|(name, d)| (name.clone(), d.root))
            .collect(),
    }
}

fn perturb(
    engine: &Engine,
    inject: &Inject,
    args: &RunArgs,
    world: &Setup,
    registered: rh_core::register::Registered,
) -> Result<Engine, Error> {
    let mut state = engine.snapshot();
    let mut rows = state
        .tables
        .get(&inject.table)
        .cloned()
        .ok_or_else(|| Error(format!("inject: no world table {}", inject.table)))?;
    let value = rows.get_mut(inject.row).ok_or_else(|| {
        Error(format!(
            "inject: {} has no row {}",
            inject.table, inject.row
        ))
    })?;
    *value ^= 1;
    state.tables.insert(&inject.table, rows);
    let mut restored = Engine::restore(
        world.calendar,
        world.seed,
        BranchSalt::from_raw(0),
        registered,
        world.provenance.clone(),
        state,
    )
    .map_err(|e| error("inject")(&e))?;
    restored.set_threads(args.threads);
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(levels: u64, parallel_levels: u64, max_jobs: usize) -> Outcome {
        Outcome {
            ladder: Ladder {
                header: Header {
                    world: WORLD.to_owned(),
                    seed: 1,
                    years: 1,
                    rung: RungKind::Yearly,
                    hash: LAYOUT.to_owned(),
                    journal: None,
                    until: None,
                    inject: None,
                    leak: false,
                    tuple: None,
                    systems: None,
                },
                rungs: Vec::new(),
                failed: None,
            },
            dispatch: DispatchStats {
                levels,
                parallel_levels,
                max_jobs,
            },
            vitals: None,
            stream: None,
        }
    }

    #[test]
    fn the_parallel_gate_fails_with_no_parallel_level_or_below_the_floor() {
        let none = outcome(1_000, 0, 1).check_parallel(0);
        assert!(none.is_err_and(|m| m.starts_with("PARALLEL LEVELS MISSING")));
        let low = outcome(1_000, 40, 2);
        assert_eq!(low.parallel_percent(), 4);
        assert!(
            low.check_parallel(5)
                .is_err_and(|m| m.starts_with("PARALLEL LEVELS BELOW THE FLOOR"))
        );
        assert_eq!(outcome(1_000, 70, 2).check_parallel(5), Ok(()));
        assert_eq!(outcome(0, 0, 0).parallel_percent(), 0);
    }
}
