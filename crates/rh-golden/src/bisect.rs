//! Bisect v0: name the first month, table, row and column where two builds' smoke runs diverge
//! (SPR-008 T-03, ruling 7; `docs/13` §4.5; V-DET-11).
//!
//! Given a baseline and a candidate [`Build`] (two binaries, or two revisions built in CI), it:
//! 1. runs both to the horizon with a **yearly** ladder, writing keyframes, and compares the
//!    ladders: the first differing rung is the first divergent year `Y`;
//! 2. resumes both from their keyframe of the latest permanent year before `Y` (each from its own
//!    keyframe and journal, through `rh_snapshot::resume::resume`, which checks the anchor) and
//!    runs to the end of `Y` with a **monthly** ladder: the first differing rung is the month `M`.
//!    The keyframes let it skip ahead instead of re-running from year 0 (ruling 9);
//! 3. resumes both again to the end of `M` and dumps the state; it compares the per-table Merkle
//!    roots and then the chunks (`PlaneDigest::first_divergence`, SPR-007), then the rows of the
//!    first differing chunk, and reports the first differing row and its value on each side.
//!
//! **Tables and columns.** The stub world stores each column as its own table (`person.alive`),
//! so for a world table the column is the table. An engine-held table (`rh-core/…`) stores
//! encoded rows; for it the report names the first differing byte of the row.
//!
//! A run that fails is a divergence too: its ladder ends in `failed`, and the report says where
//! the failing side stopped.
//!
//! REF: `05.1` §6 item 6; `CODEX/05_PROJECT/SPR-008_GoldenRunCI.md` T-03, rulings 7 and 9.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use rh_snapshot::hash::Divergence;
use rh_snapshot::store::KeyframeStore;

use crate::dump::{Dump, Rows};
use crate::ladder::{Difference, Ladder, RungKind, compare};
use crate::run::{Error, Inject, RunArgs, keyframe_path, run};

/// A build that can run the smoke world.
pub trait Build {
    /// How the report names it.
    fn name(&self) -> String;
    /// Runs `args` (the ladder, keyframes and dump go where `args` says).
    ///
    /// # Errors
    /// [`Error`] if the run could not be made at all.
    fn run(&self, args: &RunArgs) -> Result<(), Error>;
}

/// This build, in this process, optionally perturbed (the V-DET-11 candidate).
#[derive(Clone, Debug, Default)]
pub struct InProcess {
    /// The perturbation every run of this build makes.
    pub inject: Option<Inject>,
}

impl Build for InProcess {
    fn name(&self) -> String {
        match &self.inject {
            None => "this build (in process)".to_owned(),
            Some(i) => format!(
                "this build, perturbed at month {} {} row {}",
                i.month, i.table, i.row
            ),
        }
    }

    fn run(&self, args: &RunArgs) -> Result<(), Error> {
        let mut args = args.clone();
        if self.inject.is_some() {
            args.inject.clone_from(&self.inject);
        }
        run(&args).map(drop)
    }
}

/// Another binary: `<bin> run smoke <args> <extra>`. It reads the smoke world from its own
/// source tree, so a binary built from another revision runs that revision's world.
#[derive(Clone, Debug)]
pub struct Process {
    /// The `rh-cli` binary.
    pub bin: PathBuf,
    /// Extra arguments for every run (e.g. `--inject 120:person.alive:3`).
    pub extra: Vec<String>,
}

impl Build for Process {
    fn name(&self) -> String {
        let mut name = self.bin.display().to_string();
        for a in &self.extra {
            name.push(' ');
            name.push_str(a);
        }
        name
    }

    fn run(&self, args: &RunArgs) -> Result<(), Error> {
        let mut command = vec!["run".to_owned(), "smoke".to_owned()];
        command.extend(crate::cli::to_argv(args, false));
        command.extend(self.extra.iter().cloned());
        let out = Command::new(&self.bin)
            .args(&command)
            .output()
            .map_err(|e| Error(format!("{}: {e}", self.bin.display())))?;
        // A failed simulation still writes its ladder and exits 1; anything else is a tool error.
        if out.status.success() || out.status.code() == Some(1) {
            Ok(())
        } else {
            Err(Error(format!(
                "{} {}: {}",
                self.bin.display(),
                command.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            )))
        }
    }
}

/// What to bisect over.
#[derive(Clone, Debug)]
pub struct BisectArgs {
    /// The repository root (for in-process builds).
    pub repo: PathBuf,
    /// The world seed.
    pub seed: u64,
    /// The horizon, in sim-years.
    pub years: i64,
    /// Worker threads for every run.
    pub threads: usize,
    /// A scratch directory: each build's ladders, journal, keyframes and dumps go under it.
    pub work: PathBuf,
}

/// Where the two builds first differ, down to the row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located {
    /// The first divergent year (1-based).
    pub year: i64,
    /// The keyframe year both resumed from.
    pub keyframe_year: i64,
    /// The first divergent month (1-based from the epoch), its tick and date.
    pub month: u64,
    /// The month's last tick.
    pub tick: i64,
    /// The month's date.
    pub date: String,
    /// The tables whose roots differ at that month, in name order.
    pub tables: Vec<String>,
    /// The first divergent table, chunk and row, and the column and values, when both sides
    /// reached the end of the month.
    pub row: Option<RowDifference>,
    /// A side that failed before the end of the month: `(side, tick, message)`.
    pub failure: Option<(&'static str, i64, String)>,
}

/// The first differing row of the first differing table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowDifference {
    /// The table.
    pub table: String,
    /// The chunk (`rh_snapshot::hash::CHUNK_ROWS` rows each).
    pub chunk: usize,
    /// The row.
    pub row: usize,
    /// The column: the table itself for a world table, `byte <n>` for an engine table.
    pub column: String,
    /// The baseline's value (`absent` past its last row).
    pub baseline: String,
    /// The candidate's value.
    pub candidate: String,
}

/// The bisect's result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// The baseline's name.
    pub baseline: String,
    /// The candidate's name.
    pub candidate: String,
    /// The world seed.
    pub seed: u64,
    /// The horizon.
    pub years: i64,
    /// `None`: the ladders agree over the whole horizon.
    pub located: Option<Located>,
}

impl Report {
    /// The report as text: what CI attaches to the PR (`docs/13` §4.5).
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = format!(
            "rehoboam-bisect v0\nworld smoke seed {} years {}\nbaseline: {}\ncandidate: {}\n",
            self.seed, self.years, self.baseline, self.candidate
        );
        let Some(l) = &self.located else {
            out.push_str("verdict: no divergence over the horizon\n");
            return out;
        };
        let _ = writeln!(out, "verdict: divergence");
        let _ = writeln!(out, "year: {}", l.year);
        let _ = writeln!(
            out,
            "resumed from: the keyframe of year {}",
            l.keyframe_year
        );
        let _ = writeln!(out, "month: {} ({}, tick {})", l.month, l.date, l.tick);
        let _ = writeln!(out, "tables differing: {}", l.tables.join(", "));
        if let Some((side, tick, message)) = &l.failure {
            let _ = writeln!(out, "failed: the {side} stops at tick {tick}: {message}");
        }
        if let Some(r) = &l.row {
            let _ = writeln!(out, "table: {}", r.table);
            let _ = writeln!(out, "chunk: {}", r.chunk);
            let _ = writeln!(out, "row: {}", r.row);
            let _ = writeln!(out, "column: {}", r.column);
            let _ = writeln!(out, "baseline value: {}", r.baseline);
            let _ = writeln!(out, "candidate value: {}", r.candidate);
        }
        out
    }
}

/// One build's working files.
struct Side<'a> {
    label: &'static str,
    build: &'a dyn Build,
    dir: PathBuf,
}

impl Side<'_> {
    fn journal(&self) -> PathBuf {
        self.dir.join("journal.sqlite")
    }
    fn keyframes(&self) -> PathBuf {
        self.dir.join("keyframes")
    }

    fn args(&self, a: &BisectArgs, rung: RungKind, years: i64, out: &str) -> RunArgs {
        let mut args = RunArgs::new(&a.repo, years, a.threads, rung, &self.dir.join(out));
        args.seed = a.seed;
        args.journal = self.journal();
        args
    }

    fn ladder(&self, file: &str) -> Result<Ladder, Error> {
        let path = self.dir.join(file);
        let text = std::fs::read_to_string(&path).map_err(|e| {
            Error(format!(
                "{} wrote no ladder {}: {e}",
                self.label,
                path.display()
            ))
        })?;
        Ladder::parse(&text).map_err(|e| Error(format!("{}: {e}", path.display())))
    }
}

/// Bisects `baseline` against `candidate` (module docs).
///
/// # Errors
/// [`Error`] if a run cannot be made, a file is missing, or the ladders are not comparable.
pub fn bisect(
    baseline: &dyn Build,
    candidate: &dyn Build,
    a: &BisectArgs,
) -> Result<Report, Error> {
    let sides = [
        Side {
            label: "baseline",
            build: baseline,
            dir: a.work.join("baseline"),
        },
        Side {
            label: "candidate",
            build: candidate,
            dir: a.work.join("candidate"),
        },
    ];
    let mut report = Report {
        baseline: baseline.name(),
        candidate: candidate.name(),
        seed: a.seed,
        years: a.years,
        located: None,
    };
    // 1. Yearly, with keyframes.
    let Some(year) = divergent_year(&sides, a)? else {
        return Ok(report);
    };
    // 2. Monthly, from the latest keyframe year before `year` that both sides wrote.
    let keyframe_year = keyframe_year(&sides, year)?;
    let resumed = |s: &Side<'_>, out: &str, dump: Option<(u64, PathBuf)>| {
        let mut args = s.args(a, RungKind::Monthly, year, out);
        args.resume = Some(keyframe_path(&s.keyframes(), keyframe_year));
        args.dump = dump;
        s.build.run(&args)
    };
    for s in &sides {
        resumed(s, "monthly.ladder", None)?;
    }
    let monthly = [
        sides[0].ladder("monthly.ladder")?,
        sides[1].ladder("monthly.ladder")?,
    ];
    let Some(Difference::Rung {
        index: month,
        a: ra,
        b: rb,
        tables,
    }) = compare(&monthly[0], &monthly[1])
    else {
        return Err(Error(format!(
            "the yearly ladders differ at year {year}, but the monthly ladders from the keyframe \
             of year {keyframe_year} do not: a determinism leak in the runs themselves"
        )));
    };
    let (tick, date) = ra.or(rb).unwrap_or_default();
    let failure = [("baseline", &monthly[0]), ("candidate", &monthly[1])]
        .into_iter()
        .find_map(|(side, l)| {
            let missing = l.rungs.iter().all(|r| r.index != month);
            l.failed
                .clone()
                .filter(|_| missing)
                .map(|(t, m)| (side, t, m))
        });
    // 3. Dumps at the end of the month: roots, chunks, rows.
    let mut row = None;
    if failure.is_none() {
        let mut dumps = Vec::new();
        for s in &sides {
            let path = s.dir.join("dump.txt");
            resumed(s, "dump.ladder", Some((month, path.clone())))?;
            let text = std::fs::read_to_string(&path)
                .map_err(|e| Error(format!("{}: {e}", path.display())))?;
            dumps.push(Dump::parse(&text)?);
        }
        row = first_row(&dumps[0], &dumps[1]);
    }
    report.located = Some(Located {
        year,
        keyframe_year,
        month,
        tick,
        date,
        tables,
        row,
        failure,
    });
    Ok(report)
}

/// Step 1: both sides run yearly with keyframes; the first divergent year, or `None`.
fn divergent_year(sides: &[Side<'_>; 2], a: &BisectArgs) -> Result<Option<i64>, Error> {
    for s in sides {
        std::fs::create_dir_all(&s.dir).map_err(|e| Error(format!("{}: {e}", s.dir.display())))?;
        let mut args = s.args(a, RungKind::Yearly, a.years, "yearly.ladder");
        args.keyframes = Some(s.keyframes());
        s.build.run(&args)?;
    }
    let yearly = [
        sides[0].ladder("yearly.ladder")?,
        sides[1].ladder("yearly.ladder")?,
    ];
    let year = match compare(&yearly[0], &yearly[1]) {
        None => return Ok(None),
        Some(Difference::Header { field }) => {
            return Err(Error(format!(
                "the yearly ladders differ in their header ({field})"
            )));
        }
        Some(Difference::Rung { index, .. }) => i64::try_from(index).unwrap_or(i64::MAX),
        Some(Difference::Failure) => i64::try_from(yearly[0].rungs.len())
            .unwrap_or(i64::MAX)
            .saturating_add(1),
    };
    Ok(Some(year.min(a.years)))
}

/// The latest keyframe year before `year` (the start, year 0, or a permanent year) that both
/// sides wrote.
fn keyframe_year(sides: &[Side<'_>; 2], year: i64) -> Result<i64, Error> {
    let store = KeyframeStore::default();
    (0..year)
        .rev()
        .find(|&y| {
            (y == 0 || store.is_permanent_year(y))
                && sides
                    .iter()
                    .all(|s| keyframe_path(&s.keyframes(), y).is_file())
        })
        .ok_or_else(|| Error("no keyframe before the divergent year".to_owned()))
}

/// The first differing row of two dumps: the first differing table and chunk by the Merkle
/// digests (`first_divergence`), then the first differing row in that chunk.
#[must_use]
pub fn first_row(baseline: &Dump, candidate: &Dump) -> Option<RowDifference> {
    let divergence = baseline.digest().first_divergence(&candidate.digest())?;
    let (table, chunk, from) = match divergence {
        Divergence::Chunk {
            table,
            chunk,
            first_row,
        } => (table, chunk, first_row),
        Divergence::Table { table } => (table, 0, 0),
        Divergence::Plane => return None,
    };
    let (a, b) = (baseline.tables.get(&table), candidate.tables.get(&table));
    let len = a.map_or(0, Rows::len).max(b.map_or(0, Rows::len));
    (from..len).find_map(|r| {
        let (va, vb) = (value(a, r), value(b, r));
        (va != vb).then(|| {
            let (column, baseline, candidate) = column(&table, &va, &vb);
            RowDifference {
                table: table.clone(),
                chunk,
                row: r,
                column,
                baseline,
                candidate,
            }
        })
    })
}

/// One row of a table, if it has it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Int(i64),
    Bytes(Vec<u8>),
    Absent,
}

fn value(rows: Option<&Rows>, r: usize) -> Value {
    match rows {
        Some(Rows::Ints(v)) => v.get(r).map_or(Value::Absent, |&x| Value::Int(x)),
        Some(Rows::Bytes(v)) => v.get(r).map_or(Value::Absent, |x| Value::Bytes(x.clone())),
        None => Value::Absent,
    }
}

/// The column and the two values: the table itself for a world table; the first differing byte
/// for an engine table.
fn column(table: &str, a: &Value, b: &Value) -> (String, String, String) {
    let show = |v: &Value| match v {
        Value::Int(x) => x.to_string(),
        Value::Bytes(x) => crate::dump::hex(x),
        Value::Absent => "absent".to_owned(),
    };
    if let (Value::Bytes(x), Value::Bytes(y)) = (a, b) {
        let at = x
            .iter()
            .zip(y)
            .position(|(p, q)| p != q)
            .unwrap_or_else(|| x.len().min(y.len()));
        let byte = |v: &[u8]| {
            v.get(at)
                .map_or("absent".to_owned(), |c| format!("{c:02x}"))
        };
        return (format!("byte {at}"), byte(x), byte(y));
    }
    (table.to_owned(), show(a), show(b))
}

/// The keyframe path a resumed run of a build in `dir` would use for `year`.
#[must_use]
pub fn side_keyframe(dir: &Path, year: i64) -> PathBuf {
    keyframe_path(&dir.join("keyframes"), year)
}
