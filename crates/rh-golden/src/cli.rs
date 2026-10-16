//! The command-line form of the golden tools, shared by `rh-cli` and by [`crate::bisect::Process`]
//! so that a spawned build reads exactly the arguments the bisect wrote.
//!
//! - `rh-cli run smoke [--repo R] [--seed S] --years N [--threads T] [--ladder yearly|monthly]
//!   --out F [--journal J] [--keyframes D] [--resume K] [--dump M:F] [--inject M:TABLE:ROW]`:
//!   exit 0 when the run reaches the horizon, 1 when the simulation fails (the ladder ends in
//!   `failed`), 2 on a tool error.
//! - `rh-cli ladder compare A B`: exit 0 when equal, 1 when they differ (the first differing rung
//!   is printed), 2 on a tool error.
//! - `rh-cli bisect --baseline BIN --candidate BIN [--candidate-arg ARG ...] [--seed S]
//!   [--years N] [--threads T] --work DIR [--out REPORT]`: the report on stdout (and in REPORT);
//!   exit 0 when no divergence, 1 when one is located, 2 on a tool error.

use std::path::{Path, PathBuf};

use crate::bisect::{BisectArgs, Process, bisect};
use crate::ladder::{Ladder, RungKind, compare};
use crate::run::{Error, Inject, RunArgs, run};

/// Exit code: success, no divergence.
pub const OK: u8 = 0;
/// Exit code: the simulation failed, or the ladders differ, or a divergence was located.
pub const DIFFERENT: u8 = 1;
/// Exit code: a tool error.
pub const TOOL: u8 = 2;

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
    v
}

/// Parses the arguments of `run smoke` (after `run smoke`), with `repo` as the default root.
///
/// # Errors
/// [`Error`] naming the first argument that does not parse, or a missing `--years` or `--out`.
pub fn parse_run(words: &[String], repo: &Path) -> Result<RunArgs, Error> {
    let bad = |what: &str| Error(format!("run smoke: {what}"));
    let mut args = RunArgs::new(repo, 0, 1, RungKind::Yearly, Path::new(""));
    let (mut years, mut out, mut journal) = (None, None, None);
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
            "--repo" => args.repo = PathBuf::from(value),
            "--seed" => args.seed = number("--seed")?,
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
            }
            "--out" => out = Some(PathBuf::from(value)),
            "--journal" => journal = Some(PathBuf::from(value)),
            "--keyframes" => args.keyframes = Some(PathBuf::from(value)),
            "--resume" => args.resume = Some(PathBuf::from(value)),
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
    args.out = out.ok_or_else(|| bad("--out is required"))?;
    args.journal = journal.unwrap_or_else(|| args.out.with_extension("journal.sqlite"));
    Ok(args)
}

/// `run smoke`: runs and reports. Returns the exit code.
#[must_use]
pub fn run_smoke(words: &[String], repo: &Path) -> u8 {
    let result = parse_run(words, repo).and_then(|args| run(&args).map(|l| (args, l)));
    match result {
        Ok((args, ladder)) => {
            let last = ladder.rungs.last().map_or_else(String::new, |r| {
                format!(
                    "; last rung {} at {}, state hash {:016x}",
                    r.index, r.date, r.hash
                )
            });
            match &ladder.failed {
                None => {
                    println!(
                        "smoke: {} {} rungs to year {} at {} thread(s){last}; ladder {}",
                        ladder.rungs.len(),
                        args.rung.name(),
                        args.years,
                        args.threads,
                        args.out.display()
                    );
                    OK
                }
                Some((tick, message)) => {
                    println!("smoke: FAILED at tick {tick}: {message}{last}");
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
