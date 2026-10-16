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
//!
//! REF: `CODEX/05_PROJECT/SPR-008_GoldenRunCI.md` T-02, T-03, rulings 3, 4 and 7; `05.1` §6;
//! ADR-0025 item 3; SPR-007 T-02, T-03 (the journal position counts accepted inputs).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use rh_core::crash::Provenance;
use rh_core::engine::Engine;
use rh_core::ownership::SpecTable;
use rh_core::register::register;
use rh_core::time::{Pulse, SimTime};
use rh_ids::BranchSalt;
use rh_registry::ownership::{Ownership, load_specs, registry_system_ids};
use rh_snapshot::hash::{LAYOUT, PlaneDigest, xxh3};
use rh_snapshot::journal::{HistoryKey, InputKind, Journal, NewInput, Producer, Replay, Scope};
use rh_snapshot::keyframe::{History, write_keyframe};
use rh_snapshot::resume::{material_input, resume};
use rh_snapshot::state::state_digest;
use rh_snapshot::store::KeyframeStore;
use rh_sys_smoke::{CALENDAR, OVERLAY, SPECS_DIR, synthetic_inputs};

use crate::dump;
use crate::ladder::{Header, Ladder, Rung, RungKind};

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
pub struct RunArgs {
    /// The repository root, where `worlds/smoke` is.
    pub repo: PathBuf,
    /// The world seed.
    pub seed: u64,
    /// The run ends at the end of this sim-year.
    pub years: i64,
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
}

impl RunArgs {
    /// A full run of `years` with `threads` and `rung`, writing `out` and a journal next to it.
    #[must_use]
    pub fn new(repo: &Path, years: i64, threads: usize, rung: RungKind, out: &Path) -> Self {
        Self {
            repo: repo.to_path_buf(),
            seed: 1,
            years,
            threads,
            rung,
            out: out.to_path_buf(),
            journal: out.with_extension("journal.sqlite"),
            keyframes: None,
            resume: None,
            inject: None,
            dump: None,
        }
    }
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

fn journal_key(seed: u64) -> HistoryKey {
    let p = provenance();
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

/// The last minute of sim-year `year` (1-based).
#[must_use]
pub fn year_end(year: i64) -> SimTime {
    SimTime::from_minutes(year * CALENDAR.minutes_per_year() - 1)
}

/// The last minute of every month (1-based from the epoch) up to the end of year `years`.
#[must_use]
pub fn month_ends(years: i64) -> Vec<SimTime> {
    let end = year_end(years);
    let mut out = Vec::new();
    let mut t = SimTime::EPOCH;
    while let Some(next) = CALENDAR.next_boundary(Pulse::Month, t) {
        let last = SimTime::from_minutes(next.minutes() - 1);
        if last > end {
            break;
        }
        out.push(last);
        t = next;
    }
    out
}

/// `YYYY-MM` of the instant `t`.
#[must_use]
pub fn date(t: SimTime) -> String {
    let d = CALENDAR.to_date(t);
    format!("{:04}-{:02}", d.year, d.month)
}

/// One place the run stops: a rung, a perturbation, a dump, or a keyframe year.
#[derive(Clone, Debug, Default)]
struct Stop {
    rung: Option<u64>,
    month_end: bool,
    inject: bool,
    dump: bool,
    year_end: Option<i64>,
}

fn stops(args: &RunArgs) -> BTreeMap<SimTime, Stop> {
    let mut stops: BTreeMap<SimTime, Stop> = BTreeMap::new();
    let months = month_ends(args.years);
    for year in 1..=args.years {
        stops.entry(year_end(year)).or_default().year_end = Some(year);
        if args.rung == RungKind::Yearly {
            stops.entry(year_end(year)).or_default().rung = u64::try_from(year).ok();
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
    let specs = spec_table(&args.repo)?;
    let registered =
        || register(rh_sys_smoke::systems(), &specs).map_err(|e| error("registration")(&e));
    let mut journal = None;
    let mut engine = if let Some(keyframe) = &args.resume {
        let bytes = std::fs::read(keyframe).map_err(|e| error("keyframe")(&e))?;
        let replay = Replay::open(&args.journal, &journal_key(args.seed), TIMELINE)
            .map_err(|e| error("journal")(&e))?;
        resume(
            &bytes,
            CALENDAR,
            registered()?,
            &history(args.seed),
            &replay,
        )
        .map_err(|e| error("resume")(&e))?
        .0
    } else {
        {
            for suffix in ["", "-wal", "-shm"] {
                let mut path = args.journal.clone().into_os_string();
                path.push(suffix);
                let _ = std::fs::remove_file(path);
            }
            journal = Some(
                Journal::create(&args.journal, &journal_key(args.seed))
                    .map_err(|e| error("journal")(&e))?,
            );
            let mut e = rh_sys_smoke::engine(&specs, args.seed, 1)
                .map_err(|e| error("registration")(&e))?;
            e.set_provenance(provenance());
            e
        }
    };
    engine.set_threads(args.threads);
    let start = engine.snapshot().cursor.unwrap_or(SimTime::EPOCH);
    let mut ladder = Ladder {
        header: Header {
            world: WORLD.to_owned(),
            seed: args.seed,
            years: args.years,
            rung: args.rung,
            hash: LAYOUT.to_owned(),
        },
        rungs: Vec::new(),
        failed: None,
    };
    if let Some(j) = journal.as_mut() {
        for (at, entity) in synthetic_inputs(args.years) {
            accept(&mut engine, j, at, entity)?;
        }
    }
    let store = KeyframeStore::default();
    if let (Some(dir), Some(j)) = (&args.keyframes, &journal) {
        std::fs::create_dir_all(dir).map_err(|e| error("keyframes")(&e))?;
        keyframe(&engine, j, dir, 0)?;
    }
    let mut months: Vec<u8> = Vec::new();
    // A resumed run starts after its keyframe's own instant: the stops it has nothing to run up
    // to are the keyframe's state, which the continuous run already took its rung at.
    let resumed = args.resume.is_some();
    let first = stops(args)
        .into_keys()
        .find(|&t| t >= start && !(resumed && engine.next_instant().is_none_or(|n| n > t)));
    for (t, stop) in stops(args)
        .into_iter()
        .filter(|(t, _)| first.is_some_and(|f| *t >= f))
    {
        if let Err(e) = engine.advance_until(t, |_, _| {}) {
            let tick = engine.crash_artifact().map_or(t.minutes(), |a| a.tick);
            ladder.failed = Some((tick, e.to_string()));
            break;
        }
        drop(engine.drain_events());
        if let Some(inject) = args.inject.as_ref().filter(|_| stop.inject) {
            engine = perturb(&engine, inject, args, registered()?)?;
        }
        let digest = state_digest(&engine.snapshot());
        if stop.month_end {
            months.extend_from_slice(&digest.hash.to_le_bytes());
        }
        if let Some(index) = stop.rung {
            ladder.rungs.push(rung(&digest, index, t, xxh3(&months)));
            months.clear();
        }
        if let Some((_, path)) = args.dump.as_ref().filter(|_| stop.dump) {
            dump::write(&engine.snapshot(), t, path)?;
        }
        if let (Some(year), Some(dir), Some(j)) = (stop.year_end, &args.keyframes, &journal)
            && store.is_permanent_year(year)
        {
            keyframe(&engine, j, dir, year)?;
        }
    }
    std::fs::write(&args.out, ladder.render()).map_err(|e| error("ladder")(&e))?;
    Ok(ladder)
}

/// The host order (SPR-007): check `at` against the engine's cursor, journal the input, then
/// submit the journal row's engine input.
fn accept(
    engine: &mut Engine,
    journal: &mut Journal,
    at: SimTime,
    entity: rh_ids::EntityRef,
) -> Result<(), Error> {
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
    Ok(())
}

fn keyframe(engine: &Engine, journal: &Journal, dir: &Path, year: i64) -> Result<(), Error> {
    let position = journal
        .position(TIMELINE)
        .map_err(|e| error("journal")(&e))?;
    let (bytes, _) = write_keyframe(engine, position).map_err(|e| error("keyframe")(&e))?;
    std::fs::write(keyframe_path(dir, year), bytes).map_err(|e| error("keyframe")(&e))
}

/// Where a full run writes the keyframe of the end of `year` (year 0: the start).
#[must_use]
pub fn keyframe_path(dir: &Path, year: i64) -> PathBuf {
    dir.join(format!("year-{year:04}.rhkf"))
}

fn rung(digest: &PlaneDigest, index: u64, t: SimTime, months: u64) -> Rung {
    Rung {
        index,
        tick: t.minutes(),
        date: date(t),
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
        CALENDAR,
        args.seed,
        BranchSalt::from_raw(0),
        registered,
        provenance(),
        state,
    )
    .map_err(|e| error("inject")(&e))?;
    restored.set_threads(args.threads);
    Ok(restored)
}
