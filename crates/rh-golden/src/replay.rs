//! `rh-cli replay <run-dir>` (SPR-013 T-12; V-DET-04, V-DET-05, V-DET-18; ADR-0025).
//!
//! A replay rebuilds a run from its history tuple and its journal alone: `run.toml` names the
//! world, the seed, the horizon and the rung kind, and records the journal's input count and chain
//! head when the run started. The replay
//!
//! 0. refuses `--into` the run directory or a directory inside it, and a journal that still has a
//!    write-ahead log (a killed run not yet resumed);
//! 1. opens the run's journal read-only (`Replay::open`: the history key, the count, the `seq`
//!    sequence, every payload hash and row digest, the chain) and requires its position to be
//!    `run.toml`'s exactly, so a missing, extra or edited row hard-fails, even in a journal that
//!    was re-chained to be self-consistent;
//! 2. copies `run.toml` and the journal into a fresh directory `--into` and runs the world there
//!    with [`RunArgs::replay`]: the engine at genesis, the journal's rows submitted in `seq` order,
//!    nothing journaled. With `--from-keyframe TICK` (the last minute of a sim-year whose keyframe
//!    the run wrote), it copies that keyframe and the sealed channel-A segments before it as well,
//!    and resumes from the keyframe through the journal (V-DET-05);
//! 3. compares the replay with the run: the whole ladder (a resumed replay keeps the run's
//!    `run.ladder` up to the keyframe and appends its own rungs), every channel-A segment's `canonical_digest` and file
//!    bytes, and every keyframe file. It prints `REPLAY EQUAL` or `REPLAY DIFFERS` with the first difference.
//!
//! In a run directory with a stream, the journal is cross-checked against channel A's `INPUT`
//! records first (journal seq, tick, scope and row digest): a journal swapped for another,
//! self-consistent one with `run.toml` edited to match is refused by name, the tool exit code
//! (REV-SPR-013 MIN-5). **Limit:** a directory holding only `run.toml` and the journal (the
//! V-DET-18 case) has no stream to cross-check against; its journal is checked against `run.toml`
//! alone.
//!
//! The source directory is never written: the journal is opened immutable (no `-wal`, no `-shm`).
//!
//! The replay's inputs are the journal's, never the input script's (V-DET-18). A run directory
//! holding only `run.toml` and the journal is replayed without the comparison (`compared` is
//! false): the rebuilt directory is then compared with the run by replaying the run, or file by
//! file.
//!
//! REF: `CODEX/05_PROJECT/SPR-013_WalkingSkeletonC.md` T-12; `docs/05_architecture/05.9` §3.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rh_snapshot::journal::{Replay, row_digest, stream_fields};
use rh_stream::record::Body;
use rh_stream::writer::read_channel_a;

use crate::dump;
use crate::ladder::{Ladder, RungKind, compare};
use crate::run::{Error, RunArgs, keyframe_path, open_journal, run};
use crate::stream::{RunDir, RunToml};

/// `rh-cli replay`'s arguments.
#[derive(Clone, Debug)]
pub struct ReplayArgs {
    /// The repository root.
    pub repo: PathBuf,
    /// The run directory replayed.
    pub run_dir: PathBuf,
    /// The fresh directory the replay is written into (replaced if it exists).
    pub into: PathBuf,
    /// Resume from the keyframe at this tick (the last minute of a sim-year) instead of genesis.
    pub from_keyframe: Option<i64>,
    /// Worker threads.
    pub threads: usize,
}

/// What a replay found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayReport {
    /// Channel-A segments compared.
    pub segments: usize,
    /// Channel-A records in them.
    pub records: u64,
    /// Ladder rungs compared.
    pub rungs: usize,
    /// Keyframe files compared.
    pub keyframes: usize,
    /// Journal inputs replayed.
    pub inputs: u32,
    /// The differences, first first; empty when the replay equals the run.
    pub differences: Vec<String>,
    /// Whether the run directory held a run to compare with (its ladder), or only `run.toml` and
    /// the journal.
    pub compared: bool,
}

impl ReplayReport {
    /// Whether the replay equals the run.
    #[must_use]
    pub fn equal(&self) -> bool {
        self.differences.is_empty()
    }
}

fn io(context: &str) -> impl Fn(std::io::Error) -> Error + '_ {
    move |e| Error(format!("{context}: {e}"))
}

/// The run's files under `dir`, by path relative to it, skipping channel C.
fn files(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> Result<(), Error> {
        for e in std::fs::read_dir(at).map_err(io("replay"))? {
            let p = e.map_err(io("replay"))?.path();
            let rel = p
                .strip_prefix(root)
                .map_err(|e| Error(e.to_string()))?
                .display()
                .to_string();
            if p.is_dir() {
                if !rel.ends_with("/C") {
                    walk(root, &p, out)?;
                }
            } else {
                out.insert(rel, std::fs::read(&p).map_err(io("replay"))?);
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    if dir.is_dir() {
        walk(dir, dir, &mut out)?;
    }
    Ok(out)
}

/// `path` made absolute through its nearest existing ancestor (resolved), the rest appended: a
/// path that does not exist yet still compares by where it would be.
fn resolved(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut at = path.to_path_buf();
    loop {
        if let Ok(real) = at.canonicalize() {
            return rest.iter().rev().fold(real, |p, c| p.join(c));
        }
        match (at.file_name().map(std::ffi::OsStr::to_owned), at.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name);
                at = if parent.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    parent.to_path_buf()
                };
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// VER-SPR-013-TEST GAP-3: `run.toml`'s horizon, seed and manifest hash are the run's. The
/// manifest hash must be the world's pinned manifest (the history tuple replay rebuilds), the
/// ladder header must carry the same horizon, seed and tuple, and the stream's segments the same
/// manifest hash and seed. An edited `run.toml` is refused by name, the tool exit code, as MIN-5.
fn cross_check_tuple(
    src: &RunDir,
    toml: &RunToml,
    world: &crate::world::Setup,
) -> Result<(), Error> {
    let tampered = |what: String| {
        Err(Error(format!(
            "run.toml: {what}: run.toml was edited after the run (VER-SPR-013-TEST GAP-3)"
        )))
    };
    if toml.manifest_hash != world.provenance.manifest_hash {
        return tampered(format!(
            "manifest_hash {} is not the world's pinned manifest {}",
            toml.manifest_hash, world.provenance.manifest_hash
        ));
    }
    if src.ladder().is_file() {
        let text = std::fs::read_to_string(src.ladder())
            .map_err(|e| Error(format!("{}: {e}", src.ladder().display())))?;
        let ladder =
            Ladder::parse(&text).map_err(|e| Error(format!("{}: {e}", src.ladder().display())))?;
        let h = &ladder.header;
        if h.years != toml.years || h.seed != toml.seed {
            return tampered(format!(
                "years {} and seed {} are not the ladder's ({} and {})",
                toml.years, toml.seed, h.years, h.seed
            ));
        }
        let needle = format!(" manifest_hash={} ", toml.manifest_hash);
        if h.tuple.as_deref().is_none_or(|t| !t.contains(&needle)) {
            return tampered(format!(
                "manifest_hash {} is not the ladder's history tuple's",
                toml.manifest_hash
            ));
        }
    }
    let first = rh_stream::seal::sealed_segments(&rh_stream::seal::channel_dir(
        &src.stream(),
        0,
        rh_stream::record::Channel::A,
    ))
    .map_err(|e| Error(format!("stream: {e}")))?
    .into_values()
    .next();
    if let Some(dir) = first {
        let bytes = std::fs::read(dir.join(rh_stream::segment::SEGMENT_FILE))
            .map_err(|e| Error(format!("{}: {e}", dir.display())))?;
        let row =
            rh_stream::segment::read_row(&bytes).map_err(|e| Error(format!("stream: {e}")))?;
        let p = &row.provenance;
        let hex = format!("sha256:{}", dump::hex(&p.manifest_hash));
        if hex != toml.manifest_hash || p.world_seed != toml.seed {
            return tampered(format!(
                "manifest_hash {} and seed {} are not the stream's ({hex} and {})",
                toml.manifest_hash, toml.seed, p.world_seed
            ));
        }
    }
    Ok(())
}

/// REV-SPR-013 MIN-5: in a run directory with a stream, the journal must be the one the stream
/// recorded: every journal row applied within the sealed years has its `INPUT` record (seq, tick,
/// scope and row digest), and every `INPUT` record its row. A mismatch is a tampered journal
/// (refused by name, the tool exit code), never a "differs". A directory holding only `run.toml`
/// and the journal (V-DET-18) has nothing to cross-check against.
fn cross_check_inputs(src: &RunDir, journal: &Replay, mpy: i64) -> Result<(), Error> {
    if !src.stream().is_dir() {
        return Ok(());
    }
    let segs = read_channel_a(&src.stream(), 0, mpy).map_err(|e| Error(format!("stream: {e}")))?;
    let Some(last) = segs.last().map(|s| s.row.last_tick) else {
        return Ok(());
    };
    let recorded: Vec<(u64, i64, Option<u64>, [u8; 16])> = segs
        .iter()
        .flat_map(|s| &s.records)
        .filter_map(|r| match &r.body {
            Body::Input(i) => Some((i.journal_seq, r.header.tick, i.scope_id, i.row_digest)),
            _ => None,
        })
        .collect();
    let journaled: Vec<(u64, i64, Option<u64>, [u8; 16])> = journal
        .inputs()
        .iter()
        .filter(|r| r.tick.minutes() <= last)
        .map(|r| {
            let (_, _, scope_id, _) = stream_fields(r);
            (u64::from(r.seq), r.tick.minutes(), scope_id, row_digest(r))
        })
        .collect();
    if recorded == journaled {
        return Ok(());
    }
    let at = recorded
        .iter()
        .zip(&journaled)
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| recorded.len().min(journaled.len()));
    Err(Error(format!(
        "journal: {} does not match channel A's INPUT records: {} journal rows within the sealed \
         years against {} INPUT records, first difference at input {at} (journal {:?}, stream \
         {:?}). The journal was edited after the run (REV-SPR-013 MIN-5)",
        src.journal().display(),
        journaled.len(),
        recorded.len(),
        journaled.get(at).map(|r| (r.0, r.1)),
        recorded.get(at).map(|r| (r.0, r.1)),
    )))
}

fn copy(from: &Path, to: &Path) -> Result<(), Error> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(io("replay"))?;
    }
    std::fs::copy(from, to)
        .map(drop)
        .map_err(|e| Error(format!("{}: {e}", from.display())))
}

/// Replays `args.run_dir` into `args.into` and compares the two.
///
/// # Errors
/// [`Error`] when the run directory cannot be read, its journal is not the one `run.toml` records
/// (a missing, extra or edited row), or the replay cannot run. A replay that runs but differs is
/// not an error: the report lists the differences.
#[allow(
    clippy::too_many_lines,
    reason = "a replay in its order: check, copy, run, compare ladder, segments, keyframes"
)]
pub fn replay(args: &ReplayArgs) -> Result<ReplayReport, Error> {
    let src = RunDir(args.run_dir.clone());
    let toml = RunToml::read(&src.0)?;
    let world = crate::world::setup_with(
        &args.repo,
        &toml.world,
        Some(toml.seed),
        false,
        toml.inputs.as_deref().map(Path::new),
    )?;
    let (Some(count), Some(head)) = (toml.journal_inputs, toml.journal_head.as_deref()) else {
        return Err(Error(format!(
            "{}: run.toml records no journal position; replay needs one (SPR-013 T-12)",
            src.run_toml().display()
        )));
    };
    let journal = open_journal(&src.journal(), &world)?;
    let position = journal.position();
    if position.inputs != count || dump::hex(&position.head) != head {
        return Err(Error(format!(
            "journal: {} holds {} inputs with chain head {}, but run.toml records {count} with \
             head {head}: an input was added, removed or edited after the run (SPR-013 T-12)",
            src.journal().display(),
            position.inputs,
            dump::hex(&position.head)
        )));
    }
    let mpy = toml.minutes_per_year;
    cross_check_inputs(&src, &journal, mpy)?;
    cross_check_tuple(&src, &toml, &world)?;
    let resume_year = match args.from_keyframe {
        None => None,
        Some(tick) if tick >= mpy - 1 && (tick + 1) % mpy == 0 => Some((tick + 1) / mpy),
        Some(tick) => {
            return Err(Error(format!(
                "--from-keyframe {tick} is not the last minute of a sim-year ({mpy} minutes each)"
            )));
        }
    };
    let dst = RunDir(args.into.clone());
    // REV-SPR-013 MIN-7: the replay writes a fresh directory, never the run's own.
    if resolved(&dst.0).starts_with(resolved(&src.0)) {
        return Err(Error(format!(
            "--into {} is the run directory (or inside it): a replay writes a fresh directory",
            dst.0.display()
        )));
    }
    if dst.0.exists() {
        std::fs::remove_dir_all(&dst.0).map_err(io("replay"))?;
    }
    std::fs::create_dir_all(&dst.0).map_err(io("replay"))?;
    copy(&src.run_toml(), &dst.run_toml())?;
    copy(&src.journal(), &dst.journal())?;
    let mut run_args = RunArgs::new(
        &args.repo,
        toml.years,
        args.threads,
        RungKind::Yearly,
        &dst.ladder(),
    );
    run_args.rung = toml
        .rung
        .as_deref()
        .and_then(RungKind::from_name)
        .unwrap_or(RungKind::Yearly);
    toml.world.clone_into(&mut run_args.world);
    run_args.seed = toml.seed;
    run_args.seed_set = true;
    run_args.inputs = toml.inputs.as_ref().map(PathBuf::from);
    run_args.journal = dst.journal();
    run_args.run_dir = Some(dst.0.clone());
    run_args.replay = true;
    if let Some(year) = resume_year {
        let kf = keyframe_path(&src.keyframes(), year);
        if !kf.is_file() {
            return Err(Error(format!(
                "--from-keyframe: the run wrote no keyframe {}",
                kf.display()
            )));
        }
        for y in 0..=year {
            let from = keyframe_path(&src.keyframes(), y);
            if from.is_file() {
                copy(&from, &keyframe_path(&dst.keyframes(), y))?;
            }
        }
        // The sealed channel-A segments before the keyframe's year end: sim-years 0..year.
        for (rel, _) in files(&src.stream())? {
            let sealed_before = rel
                .split('/')
                .find_map(|part| part.strip_prefix('y')?.parse::<i64>().ok())
                .is_some_and(|y| y < year);
            if sealed_before && rel.contains("/A/") {
                copy(&src.stream().join(&rel), &dst.stream().join(&rel))?;
            }
        }
        // The run's ladder up to the keyframe: the resumed replay keeps it and appends its own.
        copy(&src.ladder(), &dst.ladder())?;
        run_args.resume = Some(kf);
    }
    let got = run(&run_args)?;
    let mut differences = Vec::new();
    if !src.ladder().is_file() && !src.stream().is_dir() {
        let b = read_channel_a(&dst.stream(), 0, mpy).map_err(|e| Error(format!("stream: {e}")))?;
        return Ok(ReplayReport {
            segments: b.len(),
            records: b.iter().map(|s| s.row.record_count).sum(),
            rungs: got.rungs.len(),
            keyframes: files(&dst.keyframes())?.len(),
            inputs: count,
            differences,
            compared: false,
        });
    }
    let want = Ladder::parse(
        &std::fs::read_to_string(src.ladder())
            .map_err(|e| Error(format!("{}: {e}", src.ladder().display())))?,
    )
    .map_err(|e| Error(format!("{}: {e}", src.ladder().display())))?;
    // A resumed run directory's ladder is the whole run's (its rungs up to the keyframe, then the
    // resumed ones; REV-SPR-013 MAJ-3), so both replays compare the whole ladder.
    if let Some(d) = compare(&want, &got) {
        differences.push(format!("LADDER: {d}"));
    }
    if got.render() != want.render() {
        differences.push("LADDER: the ladder files differ".to_owned());
    }
    let rungs = got.rungs.len();
    let read = |dir: &RunDir| {
        read_channel_a(&dir.stream(), 0, mpy).map_err(|e| Error(format!("stream: {e}")))
    };
    let (a, b) = (read(&src)?, read(&dst)?);
    if a.len() != b.len() {
        differences.push(format!(
            "STREAM: the run sealed {} channel-A segments, the replay {}",
            a.len(),
            b.len()
        ));
    }
    for (sa, sb) in a.iter().zip(&b) {
        if sa.row.canonical_digest != sb.row.canonical_digest {
            differences.push(format!(
                "STREAM: y{:06} canonical_digest {} != {}",
                sa.row.sim_year,
                dump::hex(&sa.row.canonical_digest),
                dump::hex(&sb.row.canonical_digest)
            ));
            break;
        }
    }
    let (fa, fb) = (files(&src.stream())?, files(&dst.stream())?);
    if let Some(rel) = fa
        .keys()
        .chain(fb.keys())
        .find(|k| fa.get(*k) != fb.get(*k))
    {
        differences.push(format!("STREAM: stream/{rel} is not byte-identical"));
    }
    let (ka, kb) = (files(&src.keyframes())?, files(&dst.keyframes())?);
    if let Some(rel) = ka
        .keys()
        .chain(kb.keys())
        .find(|k| ka.get(*k) != kb.get(*k))
    {
        differences.push(format!("KEYFRAME: keyframes/{rel} is not byte-identical"));
    }
    Ok(ReplayReport {
        segments: b.len(),
        records: b.iter().map(|s| s.row.record_count).sum(),
        rungs,
        keyframes: kb.len(),
        inputs: count,
        differences,
        compared: true,
    })
}
