//! The command-line form of the golden tools, shared by `rh-cli` and by [`crate::bisect::Process`]
//! so that a spawned build reads exactly the arguments the bisect wrote.
//!
//! - `rh-cli run smoke [--repo R] [--seed S] --years N [--until Y] [--threads T]
//!   [--ladder yearly|monthly] --out F [--journal J] [--keyframes D] [--resume K [--extend]]
//!   [--dump M:F] [--inject M:TABLE:ROW] [--leak-thread] [--require-parallel PERCENT] [--vitals]
//!   [--run-dir D] [--inputs SCRIPT] [--expect-serial]`: `--years`
//!   is the history's horizon (with `--resume`, the journal's own, unless `--extend`); `--until`
//!   stops earlier (DEF-1). The summary line reports the dispatch (levels, levels with two or
//!   more jobs, the most jobs in a level); `--require-parallel` fails the run when no level held
//!   two jobs or their share is below PERCENT, and `--leak-thread` plants the thread-identity leak
//!   (DEF-2).
//!   exit 0 when the run reaches the horizon, 1 when the simulation fails (the ladder ends in
//!   `failed`), 2 on a tool error.
//! - `rh-cli ladder cut-yearly MONTHLY OUT`: the yearly ladder of a monthly one (GAP-6).
//! - `rh-cli ladder compare A B`: exit 0 when equal, 1 when they differ (the first differing rung
//!   is printed), 2 on a tool error.
//! - `rh-cli bisect --baseline BIN --candidate BIN [--candidate-arg ARG ...] [--seed S]
//!   [--years N] [--threads T] --work DIR [--out REPORT]`: the report on stdout (and in REPORT);
//!   exit 0 when no divergence, 1 when one is located, 2 on a tool error.

use std::path::{Path, PathBuf};

use crate::bisect::{BisectArgs, Process, bisect};
use rh_snapshot::hash::{Plane, empty_plane_hash};

use crate::ladder::{Ladder, Rung, RungKind, compare};
use crate::run::{Error, Inject, Outcome, RunArgs};
use crate::vitals::Vitals;

/// Exit code: success, no divergence.
pub const OK: u8 = 0;
/// Exit code: the simulation failed, or the ladders differ, or a divergence was located.
pub const DIFFERENT: u8 = 1;
/// Exit code: a tool error.
pub const TOOL: u8 = 2;
/// Exit code: a command line that does not parse or names a filter that does not exist (`EX_USAGE`,
/// as `rh-cli`'s usage).
pub const USAGE: u8 = 64;

/// The arguments of `run smoke` for `args`. With `repo`, `--repo` is included; a spawned
/// build is given none, so it reads its own source tree.
#[must_use]
pub fn to_argv(args: &RunArgs, repo: bool) -> Vec<String> {
    let mut v = Vec::new();
    let mut push = |k: &str, val: String| {
        v.push(k.to_owned());
        v.push(val);
    };
    if repo {
        push("--repo", args.repo.display().to_string());
    }
    push("--seed", args.seed.to_string());
    push("--years", args.years.to_string());
    push("--threads", args.threads.to_string());
    push("--ladder", args.rung.name().to_owned());
    push("--out", args.out.display().to_string());
    push("--journal", args.journal.display().to_string());
    if let Some(u) = args.until {
        push("--until", u.to_string());
    }
    if let Some(d) = &args.keyframes {
        push("--keyframes", d.display().to_string());
    }
    if let Some(k) = &args.resume {
        push("--resume", k.display().to_string());
    }
    if let Some((m, p)) = &args.dump {
        push("--dump", format!("{m}:{}", p.display()));
    }
    if let Some(i) = &args.inject {
        push("--inject", format!("{}:{}:{}", i.month, i.table, i.row));
    }
    if let Some(p) = args.require_parallel {
        push("--require-parallel", p.to_string());
    }
    if args.extend {
        v.push("--extend".to_owned());
    }
    if args.expect_serial {
        v.push("--expect-serial".to_owned());
    }
    if args.leak_thread {
        v.push("--leak-thread".to_owned());
    }
    if args.strict {
        v.push("--strict".to_owned());
    }
    v
}

/// Parses the arguments of `run smoke` (after `run smoke`), with `repo` as the default root.
///
/// # Errors
/// [`Error`] naming the first argument that does not parse, or a missing `--years` or `--out`.
#[allow(
    clippy::too_many_lines,
    reason = "one match arm per flag of `rh-cli run`, then the run directory's consistency checks"
)]
pub fn parse_run(words: &[String], repo: &Path) -> Result<RunArgs, Error> {
    let bad = |what: &str| Error(format!("run smoke: {what}"));
    let mut args = RunArgs::new(repo, 0, 1, RungKind::Yearly, Path::new(""));
    let (mut years, mut out, mut journal) = (None, None, None);
    let mut it = words.iter();
    while let Some(flag) = it.next() {
        if [
            "--extend",
            "--leak-thread",
            "--vitals",
            "--strict",
            "--expect-serial",
        ]
        .contains(&flag.as_str())
        {
            args.expect_serial |= flag == "--expect-serial";
            args.extend |= flag == "--extend";
            args.leak_thread |= flag == "--leak-thread";
            args.vitals |= flag == "--vitals";
            args.strict |= flag == "--strict";
            continue;
        }
        let value = it
            .next()
            .ok_or_else(|| bad(&format!("{flag} needs a value")))?;
        let number = |what: &str| -> Result<u64, Error> {
            value
                .parse()
                .map_err(|_| bad(&format!("{what} `{value}` is not a number")))
        };
        match flag.as_str() {
            "--until" => {
                args.until = Some(
                    i64::try_from(number("--until")?).map_err(|_| bad("--until is too large"))?,
                );
            }
            "--repo" => args.repo = PathBuf::from(value),
            "--require-parallel" => args.require_parallel = Some(number("--require-parallel")?),
            "--seed" => {
                args.seed = number("--seed")?;
                args.seed_set = true;
            }
            "--years" => {
                years = Some(
                    i64::try_from(number("--years")?).map_err(|_| bad("--years is too large"))?,
                );
            }
            "--threads" => {
                args.threads = usize::try_from(number("--threads")?)
                    .map_err(|_| bad("--threads is too large"))?;
            }
            "--ladder" => {
                args.rung = RungKind::from_name(value)
                    .ok_or_else(|| bad("--ladder is yearly or monthly"))?;
                args.rung_set = true;
            }
            "--out" => out = Some(PathBuf::from(value)),
            "--journal" => journal = Some(PathBuf::from(value)),
            "--keyframes" => args.keyframes = Some(PathBuf::from(value)),
            "--resume" => args.resume = Some(PathBuf::from(value)),
            "--run-dir" => args.run_dir = Some(PathBuf::from(value)),
            "--inputs" => args.inputs = Some(PathBuf::from(value)),
            "--test-pace-ms" => args.pace_ms = Some(number("--test-pace-ms")?),
            "--test-poison" => {
                args.poison = Some(
                    i64::try_from(number("--test-poison")?)
                        .map_err(|_| bad("--test-poison is too large"))?,
                );
            }
            "--test-stall" => {
                args.stall = Some(
                    crate::stream::Stall::parse(value)
                        .ok_or_else(|| bad("--test-stall is seal:YEAR:FILE, sealed:YEAR, mid:TICK or seal:YEAR:FILE+mid:TICK"))?,
                );
            }
            "--dump" => {
                let (m, p) = value
                    .split_once(':')
                    .ok_or_else(|| bad("--dump is MONTH:FILE"))?;
                let m = m.parse().map_err(|_| bad("--dump month is not a number"))?;
                args.dump = Some((m, PathBuf::from(p)));
            }
            "--inject" => {
                let parts: Vec<&str> = value.splitn(3, ':').collect();
                let [m, table, row] = parts.as_slice() else {
                    return Err(bad("--inject is MONTH:TABLE:ROW"));
                };
                args.inject = Some(Inject {
                    month: m
                        .parse()
                        .map_err(|_| bad("--inject month is not a number"))?,
                    table: (*table).to_owned(),
                    row: row
                        .parse()
                        .map_err(|_| bad("--inject row is not a number"))?,
                });
            }
            other => return Err(bad(&format!("unknown flag {other}"))),
        }
    }
    args.years = years.ok_or_else(|| bad("--years is required"))?;
    // SPR-013 T-09: a run directory holds the ladder and the journal (REV-SPR-013 MIN-8: naming
    // others is refused, so replay and resume always find them there).
    let run_dir = args.run_dir.clone().map(crate::stream::RunDir);
    if run_dir.is_some() && (out.is_some() || journal.is_some()) {
        return Err(bad(
            "--out and --journal are the run directory's own (D/run.ladder, D/journal.sqlite) \
             and are refused with --run-dir",
        ));
    }
    args.out = out
        .or_else(|| run_dir.as_ref().map(crate::stream::RunDir::ladder))
        .ok_or_else(|| bad("--out is required (or --run-dir)"))?;
    args.journal = journal
        .or_else(|| run_dir.as_ref().map(crate::stream::RunDir::journal))
        .unwrap_or_else(|| args.out.with_extension("journal.sqlite"));
    Ok(args)
}

/// `run smoke`: runs and reports. Returns the exit code.
#[must_use]
pub fn run_smoke(words: &[String], repo: &Path) -> u8 {
    run_world(crate::world::SMOKE, words, repo)
}

/// `run <world>`: runs world `world` (`smoke`, or a manifest world under `worlds/`; SPR-012 T-08)
/// and reports. Returns the exit code.
#[must_use]
pub fn run_world(world: &str, words: &[String], repo: &Path) -> u8 {
    run_world_live(world, words, repo, None)
}

/// [`run_world`] serving a live edge (`rh-cli run … --socket`; SPR-026): `live` is built by the
/// host from the run's arguments.
#[must_use]
pub fn run_world_live(
    world: &str,
    words: &[String],
    repo: &Path,
    live: Option<crate::run::LiveHost>,
) -> u8 {
    let result = parse_run(words, repo).and_then(|mut args| {
        world.clone_into(&mut args.world);
        crate::run::run_live(&args, live).map(|o| (args, o))
    });
    match result {
        Ok((args, outcome)) => {
            let Outcome {
                ladder, dispatch, ..
            } = &outcome;
            let d = dispatch;
            println!(
                "dispatch: {} levels, {} with two or more jobs ({}%), at most {} jobs in a level",
                d.levels,
                d.parallel_levels,
                outcome.parallel_percent(),
                d.max_jobs
            );
            let last = ladder.rungs.last().map_or_else(String::new, |r| {
                format!(
                    "; last rung {} at {}, state hash {:016x}",
                    r.index, r.date, r.hash
                )
            });
            let parallel = args
                .require_parallel
                .map_or(Ok(()), |floor| outcome.check_parallel(floor))
                .and_then(|()| {
                    if args.expect_serial {
                        outcome.check_serial()
                    } else {
                        Ok(())
                    }
                });
            match (&ladder.failed, parallel) {
                (Some(_), _) | (None, Ok(())) => {}
                (None, Err(message)) => {
                    println!("{world}: {message}");
                    return DIFFERENT;
                }
            }
            if let (Some(v), Some(r)) = (&outcome.vitals, ladder.rungs.last()) {
                print_vitals(v, r, &outcome.dropped);
            }
            match &ladder.failed {
                None => {
                    if let Some(systems) = &ladder.header.systems {
                        println!("{world}: {systems}");
                    }
                    if let (Some(s), Some(dir)) = (&outcome.stream, &args.run_dir) {
                        println!(
                            "{world}: stream: {} segments sealed, {} verified, {} records, next seq {}; {}",
                            s.sealed,
                            s.verified,
                            s.records,
                            s.next_seq,
                            dir.join("stream").display()
                        );
                    }
                    println!(
                        "{world}: {} {} rungs to year {} at {} thread(s){last}; ladder {}",
                        ladder.rungs.len(),
                        ladder.header.rung.name(),
                        args.years,
                        args.threads,
                        args.out.display()
                    );
                    OK
                }
                Some((tick, message)) => {
                    println!("{world}: FAILED at tick {tick}: {message}{last}");
                    DIFFERENT
                }
            }
        }
        Err(e) => {
            eprintln!("{e}");
            TOOL
        }
    }
}

/// Prints the vitals line and the per-system job times (SPR-009 T-03; GOV-006 §1). Planes B and C
/// have no tables yet, so their hashes are the empty planes'.
fn print_vitals(v: &Vitals, last: &Rung, dropped: &[(String, String, &'static str, u64)]) {
    let hashes = [
        last.hash,
        empty_plane_hash(Plane::B),
        empty_plane_hash(Plane::C),
    ];
    println!("{}", v.line(&last.date, hashes));
    for (system, time) in &v.per_system {
        println!(
            "vitals: system {system} {} ms of job time",
            time.as_millis()
        );
    }
    // SPR-014 T-03: the one drop diagnostic, outside every hash (`05.1` §2 rule 2).
    if dropped.is_empty() {
        println!("vitals: dropped nothing (dropped{{system, kind, reason}} is empty)");
    }
    for (system, kind, reason, n) in dropped {
        println!("vitals: dropped{{{system}, {kind}, {reason}}} = {n}");
    }
    let permille = v.serial_permille();
    println!(
        "vitals: serial fraction {}.{}% over {} instants",
        permille / 10,
        permille % 10,
        v.instants
    );
}

/// `ladder compare A B`. Returns the exit code.
#[must_use]
pub fn ladder_compare(first: &Path, second: &Path) -> u8 {
    let read = |p: &Path| -> Result<Ladder, Error> {
        let text =
            std::fs::read_to_string(p).map_err(|e| Error(format!("{}: {e}", p.display())))?;
        Ladder::parse(&text).map_err(|e| Error(format!("{}: {e}", p.display())))
    };
    match (read(first), read(second)) {
        (Ok(left), Ok(right)) => match compare(&left, &right) {
            None => {
                println!(
                    "LADDERS EQUAL: {} rungs ({} {})",
                    left.rungs.len(),
                    first.display(),
                    second.display()
                );
                OK
            }
            Some(d) => {
                println!(
                    "LADDERS DIFFER: {d} ({} vs {})",
                    first.display(),
                    second.display()
                );
                DIFFERENT
            }
        },
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("{e}");
            TOOL
        }
    }
}

/// `ladder cut-yearly MONTHLY OUT`: writes the yearly cut of a monthly ladder (GAP-6). Returns the
/// exit code: 0 written, 2 on a tool error.
#[must_use]
pub fn ladder_cut_yearly(monthly: &Path, out: &Path) -> u8 {
    let cut = std::fs::read_to_string(monthly)
        .map_err(|e| format!("{}: {e}", monthly.display()))
        .and_then(|t| Ladder::parse(&t).map_err(|e| format!("{}: {e}", monthly.display())))
        .and_then(|l| l.cut_yearly())
        .and_then(|l| {
            std::fs::write(out, l.render())
                .map(|()| l.rungs.len())
                .map_err(|e| format!("{}: {e}", out.display()))
        });
    match cut {
        Ok(n) => {
            println!(
                "CUT: {n} yearly rungs from {} to {}",
                monthly.display(),
                out.display()
            );
            OK
        }
        Err(e) => {
            eprintln!("{e}");
            TOOL
        }
    }
}

/// `bisect ...`. Returns the exit code.
#[must_use]
pub fn bisect_cmd(words: &[String], repo: &Path) -> u8 {
    match parse_bisect(words, repo)
        .and_then(|(base, cand, args, out)| bisect(&base, &cand, &args).map(|r| (r, out)))
    {
        Ok((report, out)) => {
            let text = report.render();
            print!("{text}");
            if let Some(path) = out
                && let Err(e) = std::fs::write(&path, &text)
            {
                eprintln!("{}: {e}", path.display());
                return TOOL;
            }
            if report.located.is_some() {
                DIFFERENT
            } else {
                OK
            }
        }
        Err(e) => {
            eprintln!("{e}");
            TOOL
        }
    }
}

type Parsed = (Process, Process, BisectArgs, Option<PathBuf>);

fn parse_bisect(words: &[String], repo: &Path) -> Result<Parsed, Error> {
    let bad = |what: &str| Error(format!("bisect: {what}"));
    let (mut base, mut cand, mut work, mut out) = (None, None, None, None);
    let mut extra = Vec::new();
    let mut args = BisectArgs {
        repo: repo.to_path_buf(),
        seed: 1,
        years: 1_000,
        threads: 1,
        work: PathBuf::new(),
    };
    let mut it = words.iter();
    while let Some(flag) = it.next() {
        let value = it
            .next()
            .ok_or_else(|| bad(&format!("{flag} needs a value")))?;
        let number = |what: &str| -> Result<u64, Error> {
            value
                .parse()
                .map_err(|_| bad(&format!("{what} `{value}` is not a number")))
        };
        match flag.as_str() {
            "--baseline" => base = Some(PathBuf::from(value)),
            "--candidate" => cand = Some(PathBuf::from(value)),
            "--candidate-arg" => extra.push(value.clone()),
            "--seed" => args.seed = number("--seed")?,
            "--years" => {
                args.years =
                    i64::try_from(number("--years")?).map_err(|_| bad("--years is too large"))?;
            }
            "--threads" => {
                args.threads = usize::try_from(number("--threads")?)
                    .map_err(|_| bad("--threads is too large"))?;
            }
            "--work" => work = Some(PathBuf::from(value)),
            "--out" => out = Some(PathBuf::from(value)),
            other => return Err(bad(&format!("unknown flag {other}"))),
        }
    }
    args.work = work.ok_or_else(|| bad("--work is required"))?;
    let base = Process {
        bin: base.ok_or_else(|| bad("--baseline is required"))?,
        extra: Vec::new(),
    };
    let cand = Process {
        bin: cand.ok_or_else(|| bad("--candidate is required"))?,
        extra,
    };
    Ok((base, cand, args, out))
}

/// `tail <run-dir> [--from-seq N] [--to-seq M] [--follow P] [--kind K ...] [--canonical] [--wait]`
/// (SPR-013 T-10; [`crate::tail`]). Exit 0, 64 on a usage error (an unknown flag, or a `--kind`
/// that is neither a record kind nor a Registry verb: VER-SPR-013-TEST GAP-2), or 2 on a tool error.
#[must_use]
pub fn tail_cmd(words: &[String], repo: &Path) -> u8 {
    let parsed = match parse_tail(words, repo) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return USAGE;
        }
    };
    let result = Ok(parsed).and_then(|args| {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        let repo = args.repo.clone();
        let notes = move |dir: &Path| crate::sift::annotations(&repo, dir);
        crate::tail::tail(&args, &notes, &mut lock)
    });
    match result {
        Ok(()) => OK,
        // A reader that stops reading (`| head`) ends the tail, not as an error.
        Err(e) if e.0.contains("Broken pipe") => OK,
        Err(e) => {
            eprintln!("{e}");
            TOOL
        }
    }
}

fn parse_tail(words: &[String], repo: &Path) -> Result<crate::tail::TailArgs, Error> {
    let bad = |what: &str| Error(format!("tail: {what}"));
    let mut args = crate::tail::TailArgs {
        repo: repo.to_path_buf(),
        ..crate::tail::TailArgs::default()
    };
    let mut run_dir = None;
    let mut it = words.iter();
    while let Some(w) = it.next() {
        let mut value = || it.next().ok_or_else(|| bad(&format!("{w} needs a value")));
        match w.as_str() {
            "--canonical" => args.canonical = true,
            "--wait" => args.wait = true,
            "--timeout" => {
                args.timeout = Some(value()?.parse().map_err(|_| bad("--timeout is a number"))?);
            }
            "--from-seq" => {
                args.from_seq = value()?
                    .parse()
                    .map_err(|_| bad("--from-seq is a number"))?;
            }
            "--to-seq" => {
                args.to_seq = Some(value()?.parse().map_err(|_| bad("--to-seq is a number"))?);
            }
            "--follow" => args.follow = Some(value()?.clone()),
            "--kind" => args.kinds.push(value()?.clone()),
            // SPR-026 T-04: the sealed channel C.
            "--channel" => match value()?.as_str() {
                "A" => args.channel_c = false,
                "C" => args.channel_c = true,
                other => return Err(bad(&format!("--channel {other}: A or C"))),
            },
            "--repo" => args.repo = PathBuf::from(value()?),
            flag if flag.starts_with("--") => return Err(bad(&format!("unknown flag {flag}"))),
            dir if run_dir.is_none() => run_dir = Some(PathBuf::from(dir)),
            extra => return Err(bad(&format!("unexpected {extra}"))),
        }
    }
    args.run_dir = run_dir.ok_or_else(|| bad("a run directory is required"))?;
    // GAP-2: a filter that names nothing is refused, never an empty tail.
    if !args.kinds.is_empty() {
        let registry = rh_manifest::Registry::load(&args.repo.join(crate::world::DEFAULT_REGISTRY))
            .map_err(|e| bad(&format!("the Registry: {e}")))?;
        for k in &args.kinds {
            let record = rh_stream::record::Kind::CHANNEL_A
                .iter()
                .any(|x| x.name() == k);
            if !record && !registry.verbs.contains_key(k) {
                return Err(bad(&format!(
                    "--kind {k} is neither a channel-A record kind ({}) nor a Registry verb",
                    rh_stream::record::Kind::CHANNEL_A
                        .map(rh_stream::record::Kind::name)
                        .join(", ")
                )));
            }
        }
    }
    Ok(args)
}

/// `sift <run-dir>` (SPR-013 T-11; [`crate::sift`]). Exit 0, or 2 on a tool error.
#[must_use]
pub fn sift_cmd(words: &[String], repo: &Path) -> u8 {
    let (dir, repo) = match words {
        [dir] => (PathBuf::from(dir), repo.to_path_buf()),
        [dir, flag, r] if flag == "--repo" => (PathBuf::from(dir), PathBuf::from(r)),
        _ => {
            eprintln!("usage: rh-cli sift <run-dir> [--repo R]");
            return TOOL;
        }
    };
    match crate::sift::sift(&repo, &dir) {
        Ok((packs, s)) => {
            println!(
                "sift: {} matches of {} in {} channel-C segments from {} channel-A records; {}",
                s.matches,
                packs.join(","),
                s.segments,
                s.a_records,
                dir.join("stream").display()
            );
            OK
        }
        Err(e) => {
            eprintln!("{e}");
            TOOL
        }
    }
}

/// `replay <run-dir> --into D [--from-keyframe TICK] [--threads T] [--repo R]` (SPR-013 T-12;
/// [`crate::replay`]). Exit 0 when the replay equals the run, 1 when it differs, 2 on a tool error
/// (a journal that is not the run's among them: against `run.toml`, and in a run directory with a
/// stream against its `INPUT` records; a journal-only directory is checked against `run.toml`
/// alone).
#[must_use]
pub fn replay_cmd(words: &[String], repo: &Path) -> u8 {
    match parse_replay(words, repo).and_then(|a| crate::replay::replay(&a).map(|r| (a, r))) {
        Ok((args, r)) if !r.compared => {
            println!(
                "REPLAY REBUILT: {} (run.toml and the journal only) into {}: {} journal inputs, {} \
                 ladder rungs, {} channel-A segments ({} records), {} keyframes; nothing to compare",
                args.run_dir.display(),
                args.into.display(),
                r.inputs,
                r.rungs,
                r.segments,
                r.records,
                r.keyframes
            );
            OK
        }
        Ok((args, r)) if r.equal() => {
            println!(
                "REPLAY EQUAL: {} from {}: {} journal inputs, {} ladder rungs, {} channel-A segments \
                 ({} records) and {} keyframes equal to the run's, byte for byte",
                args.run_dir.display(),
                args.from_keyframe.map_or_else(
                    || "genesis".to_owned(),
                    |t| format!("the keyframe at tick {t}")
                ),
                r.inputs,
                r.rungs,
                r.segments,
                r.records,
                r.keyframes
            );
            OK
        }
        Ok((args, r)) => {
            println!(
                "REPLAY DIFFERS: {} replayed into {}",
                args.run_dir.display(),
                args.into.display()
            );
            for d in &r.differences {
                println!("  {d}");
            }
            DIFFERENT
        }
        Err(e) => {
            eprintln!("replay: {e}");
            TOOL
        }
    }
}

fn parse_replay(words: &[String], repo: &Path) -> Result<crate::replay::ReplayArgs, Error> {
    let bad = |what: &str| Error(format!("replay: {what}"));
    let (mut run_dir, mut into, mut from_keyframe, mut threads) = (None, None, None, 1);
    let mut repo = repo.to_path_buf();
    let mut it = words.iter();
    while let Some(w) = it.next() {
        let mut value = || it.next().ok_or_else(|| bad(&format!("{w} needs a value")));
        match w.as_str() {
            "--into" => into = Some(PathBuf::from(value()?)),
            "--from-keyframe" => {
                from_keyframe = Some(
                    value()?
                        .parse()
                        .map_err(|_| bad("--from-keyframe is a tick"))?,
                );
            }
            "--threads" => threads = value()?.parse().map_err(|_| bad("--threads is a number"))?,
            "--repo" => repo = PathBuf::from(value()?),
            flag if flag.starts_with("--") => return Err(bad(&format!("unknown flag {flag}"))),
            dir if run_dir.is_none() => run_dir = Some(PathBuf::from(dir)),
            extra => return Err(bad(&format!("unexpected {extra}"))),
        }
    }
    Ok(crate::replay::ReplayArgs {
        repo,
        run_dir: run_dir.ok_or_else(|| bad("a run directory is required"))?,
        into: into.ok_or_else(|| bad("--into is required"))?,
        from_keyframe,
        threads,
    })
}

/// `canonical parse` and `canonical files F...`: `rh-stream`'s canonical parser, for the
/// differential gate against the reference (`specs/stream/differential_v0.py`; REV-SPR-013 MAJ-1).
///
/// - `parse` reads lines from stdin (split at LF; a line may hold any other byte) and prints one
///   line per input: `ok <the record rendered again>` or `err`.
/// - `files` checks each canonical file (`digest_file`) and prints `ok <digest>` or `err` per file.
///
/// Exit 0, or 2 on a usage or I/O error.
#[must_use]
pub fn canonical_cmd(words: &[String], _repo: &Path) -> u8 {
    use rh_stream::canonical;
    use std::io::{Read, Write};
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let mut emit = |line: String| writeln!(out, "{line}").is_ok();
    match words {
        [cmd] if cmd == "parse" => {
            let mut data = Vec::new();
            if std::io::stdin().read_to_end(&mut data).is_err() {
                return TOOL;
            }
            if data.last() == Some(&b'\n') {
                data.pop();
            }
            for raw in data.split(|&b| b == b'\n') {
                let verdict = std::str::from_utf8(raw)
                    .ok()
                    .and_then(|l| canonical::parse(l).ok())
                    .and_then(|r| canonical::render(&r).ok())
                    .map_or_else(|| "err".to_owned(), |l| format!("ok {l}"));
                if !emit(verdict) {
                    return TOOL;
                }
            }
            OK
        }
        [cmd, files @ ..] if cmd == "files" => {
            for f in files {
                let Ok(bytes) = std::fs::read(f) else {
                    return TOOL;
                };
                let verdict = canonical::digest_file(&bytes).map_or_else(
                    |_| "err".to_owned(),
                    |c| format!("ok {}", canonical::hex16(&c.digest)),
                );
                if !emit(verdict) {
                    return TOOL;
                }
            }
            OK
        }
        _ => {
            eprintln!("usage: rh-cli canonical parse < LINES | rh-cli canonical files F...");
            TOOL
        }
    }
}
