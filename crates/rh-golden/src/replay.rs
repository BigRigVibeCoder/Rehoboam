//! `rh-cli replay <run-dir>` (SPR-013 T-12; V-DET-04, V-DET-05, V-DET-18; ADR-0025).
//!
//! A replay rebuilds a run from its history tuple and its journal alone: `run.toml` names the
//! world, the seed, the horizon and the rung kind, and records the journal's input count and chain
//! head when the run started. The replay
//!
//! 1. opens the run's journal read-only (`Replay::open`: the history key, the count, the `seq`
//!    sequence, every payload hash and row digest, the chain) and requires its position to be
//!    `run.toml`'s exactly, so a missing, extra or edited row hard-fails, even in a journal that
//!    was re-chained to be self-consistent;
//! 2. copies `run.toml` and the journal into a fresh directory `--into` and runs the world there
//!    with [`RunArgs::replay`]: the engine at genesis, the journal's rows submitted in `seq` order,
//!    nothing journaled. With `--from-keyframe TICK` (the last minute of a sim-year whose keyframe
//!    the run wrote), it copies that keyframe and the sealed channel-A segments before it as well,
//!    and resumes from the keyframe through the journal (V-DET-05);
//! 3. compares the replay with the run: the whole ladder (a resumed run directory's ladder starts
//!    from its keyframe's ladder sidecar), every channel-A segment's `canonical_digest` and file
//!    bytes, and every keyframe file. It prints `REPLAY EQUAL` or `REPLAY DIFFERS` with the first difference.
//!
//! The replay's inputs are the journal's, never the input script's (V-DET-18). A run directory
//! holding only `run.toml` and the journal is replayed without the comparison (`compared` is
//! false): the rebuilt directory is then compared with the run by replaying the run, or file by
//! file.
//!
//! REF: `CODEX/05_PROJECT/SPR-013_WalkingSkeletonC.md` T-12; `docs/05_architecture/05.9` §3.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rh_stream::writer::read_channel_a;

use crate::dump;
use crate::ladder::{Ladder, RungKind, compare};
use crate::run::{Error, RunArgs, keyframe_path, ladder_sidecar, open_journal, run};
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
                copy(
                    &ladder_sidecar(&from),
                    &ladder_sidecar(&keyframe_path(&dst.keyframes(), y)),
                )?;
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
    // A resumed run directory's ladder is the whole run's (its keyframe's sidecar and the resumed
    // rungs; REV-SPR-013 MAJ-3), so both replays compare the whole ladder.
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
    // Keyframes byte for byte. A keyframe's ladder sidecar carries the horizon of the run that
    // wrote it (an extended run's earlier ones the shorter horizon), so it is checked as what it
    // is for: its rungs are the run's ladder up to the keyframe.
    let (mut ka, mut kb) = (files(&src.keyframes())?, files(&dst.keyframes())?);
    for (rel, bytes) in std::mem::take(&mut ka) {
        if rel.ends_with(".ladder") {
            let prefix = std::str::from_utf8(&bytes)
                .ok()
                .and_then(|t| Ladder::parse(t).ok())
                .map(|l| l.rungs);
            if prefix.is_none_or(|p| want.rungs.get(..p.len()) != Some(&p[..])) {
                differences.push(format!(
                    "KEYFRAME: keyframes/{rel} is not the run's ladder up to its keyframe"
                ));
            }
        } else {
            ka.insert(rel, bytes);
        }
    }
    kb.retain(|rel, _| !rel.ends_with(".ladder"));
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
