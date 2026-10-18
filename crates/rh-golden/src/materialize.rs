//! `rh-cli materialize <run-dir>` (SPR-027 T-02; V-STR-02; ADR-0030 item 3; 05.10 §4).
//!
//! The materializer rebuilds the exact plane-A state at any published tick:
//! - finds the nearest permanent keyframe at or before `tick` (genesis when none);
//! - resumes through the replay path with the journal's rows in `seq` order and **no stream
//!   writer and no ladder attached** (it emits nothing, seals nothing, writes nothing);
//! - advances to `tick` and stops after Bookkeeping of `tick`;
//! - computes `state_hash`, `Dump`, and table roots;
//! - cross-checks against the published hash in channel A (`STEP_DIGEST`/`KEYFRAME`) or
//!   `run.ladder`, unless `--any-tick` is given.
//!
//! REF: `CODEX/05_PROJECT/SPR-027_Materializer.md` T-02; `docs/05_architecture/05.10_world_stream.md` §4.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rh_core::engine::Engine;
use rh_core::register::register;
use rh_core::time::SimTime;
use rh_ids::BranchSalt;
use rh_snapshot::keyframe::History;
use rh_snapshot::resume::material_input;
use rh_stream::record::Body;
use rh_stream::writer::read_channel_a;

use crate::dump::{self, Dump};
use crate::ladder::Ladder;
use crate::run::{Error, keyframe_path, open_journal};
use crate::stream::{RunDir, RunToml};

/// The materialized state at a tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Materialized {
    /// The instant materialized.
    pub tick: i64,
    /// The plane-A state hash after Bookkeeping of `tick`.
    pub state_hash: u64,
    /// The dump of all plane-A tables at `tick`.
    pub dump: Dump,
    /// The keyframe tick the resume started from (-1 when started from genesis).
    pub keyframe_tick: i64,
    /// The root hash of every table at `tick`.
    pub table_roots: BTreeMap<String, u64>,
}

/// Arguments for materializing a tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterializeArgs {
    /// The repository root.
    pub repo: PathBuf,
    /// The run directory.
    pub run_dir: PathBuf,
    /// The tick to materialize.
    pub tick: Option<i64>,
    /// The sequence number of a `STEP_DIGEST` record to materialize.
    pub at_digest: Option<u64>,
    /// Dump destination file.
    pub dump: Option<PathBuf>,
    /// Dump to standard output (`--dump -`).
    pub dump_stdout: bool,
    /// Only print the state hash.
    pub hash_only: bool,
    /// Allow materializing a tick that has no published hash.
    pub any_tick: bool,
}

fn default_repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Materializes the state of `run_dir` at `tick`.
///
/// # Errors
/// [`Error`] if the run directory cannot be read, the tick is invalid or has no published hash,
/// or the materialized state hash differs from the published hash.
pub fn materialize(run_dir: &Path, tick: i64) -> Result<Materialized, Error> {
    let repo = default_repo();
    materialize_with(&repo, run_dir, Some(tick), None, false)
}

/// Materializes according to [`MaterializeArgs`].
///
/// # Errors
/// As [`materialize_with`].
pub fn materialize_args(args: &MaterializeArgs) -> Result<Materialized, Error> {
    materialize_with(
        &args.repo,
        &args.run_dir,
        args.tick,
        args.at_digest,
        args.any_tick,
    )
}

/// Materializes the state of `run_dir` at `tick` or `--at-digest`.
///
/// # Errors
/// [`Error`] if the run directory cannot be read, the tick is invalid or has no published hash,
/// or the materialized state hash differs from the published hash.
#[allow(
    clippy::too_many_lines,
    reason = "the materializer flow: check, locate keyframe, resume/replay, advance, verify hash"
)]
pub fn materialize_with(
    repo: &Path,
    run_dir: &Path,
    tick: Option<i64>,
    at_digest: Option<u64>,
    any_tick: bool,
) -> Result<Materialized, Error> {
    let src = RunDir(run_dir.to_path_buf());
    let toml = RunToml::read(&src.0)?;
    let world = crate::world::setup_with(
        repo,
        &toml.world,
        Some(toml.seed),
        false,
        toml.inputs.as_deref().map(Path::new),
    )?;

    let (Some(count), Some(head)) = (toml.journal_inputs, toml.journal_head.as_deref()) else {
        return Err(Error(format!(
            "{}: run.toml records no journal position; materialize needs one (SPR-027 T-02)",
            src.run_toml().display()
        )));
    };

    let journal = open_journal(&src.journal(), &world)?;
    let position = journal.position();
    if position.inputs != count || dump::hex(&position.head) != head {
        return Err(Error(format!(
            "journal: {} holds {} inputs with chain head {}, but run.toml records {count} with \
             head {head}: an input was added, removed or edited after the run (SPR-027 T-02)",
            src.journal().display(),
            position.inputs,
            dump::hex(&position.head)
        )));
    }

    let mpy = toml.minutes_per_year;
    crate::replay::cross_check_inputs(&src, &journal, mpy)?;
    crate::replay::cross_check_tuple(&src, &toml, &world)?;

    // Determine target tick and reference hash.
    let (target_tick, reference_hash) = match (tick, at_digest) {
        (Some(_), Some(_)) => {
            return Err(Error(
                "--tick and --at-digest are mutually exclusive".to_owned(),
            ));
        }
        (None, None) => {
            return Err(Error(
                "materialize requires either --tick <T> or --at-digest <SEQ>".to_owned(),
            ));
        }
        (None, Some(seq)) => {
            if !src.stream().is_dir() {
                return Err(Error(
                    "--at-digest requires a run directory with a stream".to_owned(),
                ));
            }
            let segs =
                read_channel_a(&src.stream(), 0, mpy).map_err(|e| Error(format!("stream: {e}")))?;
            let found = segs
                .iter()
                .flat_map(|s| &s.records)
                .find(|r| r.header.seq == seq);
            let Some(record) = found else {
                return Err(Error(format!(
                    "--at-digest {seq}: no record with seq {seq} in channel A"
                )));
            };
            let Body::StepDigest(sd) = &record.body else {
                return Err(Error(format!(
                    "--at-digest {seq}: record with seq {seq} is not a STEP_DIGEST (it is {})",
                    record.body.kind().name()
                )));
            };
            (record.header.tick, Some(sd.state_hash))
        }
        (Some(t), None) => {
            // Find published hash from stream or ladder if present.
            let mut published = None;
            if src.stream().is_dir()
                && let Ok(segs) = read_channel_a(&src.stream(), 0, mpy)
            {
                for rec in segs.iter().flat_map(|s| &s.records) {
                    if rec.header.tick == t {
                        match &rec.body {
                            Body::StepDigest(sd) => {
                                published = Some(sd.state_hash);
                                break;
                            }
                            Body::Keyframe(kf) => {
                                published = Some(kf.state_hash);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            if published.is_none() && src.ladder().is_file() {
                if let Ok(text) = std::fs::read_to_string(src.ladder())
                    && let Ok(ladder) = Ladder::parse(&text)
                    && let Some(rung) = ladder.rungs.iter().find(|r| r.tick == t)
                {
                    published = Some(rung.hash);
                }
            }
            if published.is_none() && !any_tick {
                return Err(Error(format!(
                    "tick {t} has no published hash in stream or ladder (use --any-tick to materialize without a published hash)"
                )));
            }
            (t, published)
        }
    };

    if target_tick < 0 {
        return Err(Error(format!(
            "tick {target_tick} is before genesis (tick 0)"
        )));
    }
    let horizon_tick = toml.years * mpy - 1;
    if target_tick > horizon_tick {
        return Err(Error(format!(
            "tick {target_tick} is outside the horizon of {} years (tick {horizon_tick})",
            toml.years
        )));
    }

    // Find nearest permanent keyframe at or before target_tick.
    // Permanent keyframes occur every 10 years: tick = year * mpy - 1 for year in 10, 20, 30...
    let year = (target_tick + 1) / mpy;
    let permanent_year = (year / 10) * 10;
    let (keyframe_tick, resume_from) = if permanent_year >= 10 {
        let kf = keyframe_path(&src.keyframes(), permanent_year);
        if kf.is_file() {
            (permanent_year * mpy - 1, Some(kf))
        } else {
            (-1, None)
        }
    } else {
        (-1, None)
    };

    let registered =
        register((world.systems)(false), &world.specs).map_err(|e| Error(e.to_string()))?;

    let mut engine = if let Some(kf) = resume_from {
        let bytes =
            std::fs::read(&kf).map_err(|e| Error(format!("keyframe {}: {e}", kf.display())))?;
        let history = History {
            provenance: world.provenance.clone(),
            world_seed: world.seed,
            branch_salt: 0,
        };
        #[cfg(not(feature = "mutant-materialize-no-journal"))]
        let (engine, _) =
            rh_snapshot::resume::resume(&bytes, world.calendar, registered, &history, &journal)
                .map_err(|e| Error(e.to_string()))?;
        #[cfg(feature = "mutant-materialize-no-journal")]
        let (engine, _) =
            rh_snapshot::keyframe::load_engine(&bytes, world.calendar, registered, &history)
                .map_err(|e| Error(e.to_string()))?;
        engine
    } else {
        let mut engine = Engine::new(
            world.calendar,
            world.seed,
            BranchSalt::from_raw(0),
            registered,
            world.tables.clone(),
            SimTime::EPOCH,
        );
        engine.set_provenance(world.provenance.clone());
        #[cfg(not(feature = "mutant-materialize-no-journal"))]
        {
            let rows = journal.inputs().to_vec();
            for row in &rows {
                let input = material_input(row)
                    .ok_or_else(|| Error(format!("journal row {} is not material", row.seq)))?;
                engine
                    .submit_input(row.tick, input)
                    .map_err(|e| Error(e.to_string()))?;
            }
        }
        let pos = journal.position();
        engine.set_journal_head(Some(dump::hex(&pos.head)));
        engine
    };

    engine.set_threads(1);
    engine
        .advance_until(SimTime::from_minutes(target_tick), |_, _| {})
        .map_err(|e| Error(format!("simulation: {e}")))?;

    let digest = rh_snapshot::state::state_digest(&engine.snapshot());
    let state_hash = digest.hash;
    let dump = Dump::of(&engine.snapshot(), SimTime::from_minutes(target_tick));
    let table_roots = digest.tables.into_iter().map(|(k, v)| (k, v.root)).collect();

    if let Some(expected) = reference_hash {
        if state_hash != expected {
            return Err(Error(format!(
                "materialize: tick {target_tick} state_hash {state_hash:016x} != published hash {expected:016x}"
            )));
        }
    }

    Ok(Materialized {
        tick: target_tick,
        state_hash,
        dump,
        keyframe_tick,
        table_roots,
    })
}
