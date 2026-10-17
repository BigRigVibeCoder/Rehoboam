//! The golden tools on the smoke world (SPR-008 T-02, T-03).
//!
//! - A run resumed from a keyframe through the journal writes the continuous run's ladder, rung
//!   for rung (the bisect's skip-ahead is sound: ruling 9).
//! - The ladder does not depend on the thread count (V-DET-01 at small scale; the golden gate in
//!   `bin/check_all.sh` runs 1,000 years).
//! - V-DET-11: a one-row perturbation injected at a random month, world table and row (seeded,
//!   printed) is named exactly by the bisect: that month, that table, that row; and a
//!   perturbation that breaks an invariant later is still named at its month.
//! - The bisect reports no divergence between a build and itself.
//!
//! Refs: SPR-008 T-02, T-03, rulings 4, 7 and 9; `docs/09` V-DET-01, V-DET-11.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken run must fail the test loudly"
)]

use std::path::{Path, PathBuf};

use rh_golden::bisect::{BisectArgs, InProcess, bisect};
use rh_golden::ladder::{Ladder, RungKind, compare};
use rh_golden::run::{Inject, RunArgs, keyframe_path, run};
use rh_ids::rng::{h64_words, tag};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh scratch directory under the target directory.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_resumed_run_writes_the_continuous_runs_ladder() {
    let dir = scratch("resume");
    let mut full = RunArgs::new(&repo(), 24, 1, RungKind::Monthly, &dir.join("full.ladder"));
    full.keyframes = Some(dir.join("kf"));
    let continuous = run(&full).unwrap();
    assert!(continuous.failed.is_none());
    assert_eq!(continuous.rungs.len(), 24 * 12);
    for year in [10, 20] {
        let mut resumed = RunArgs::new(
            &repo(),
            24,
            1,
            RungKind::Monthly,
            &dir.join(format!("from{year}.ladder")),
        );
        resumed.journal.clone_from(&full.journal);
        resumed.resume = Some(keyframe_path(&dir.join("kf"), year));
        let got = run(&resumed).unwrap();
        // The resumed ladder starts at the first month after the keyframe.
        let from = continuous
            .rungs
            .iter()
            .position(|r| r.index == got.rungs[0].index)
            .unwrap();
        assert_eq!(got.rungs, continuous.rungs[from..], "from year {year}");
        // Resumed with yearly rungs too: the same as the continuous yearly ladder's tail.
        let mut yearly = resumed.clone();
        yearly.rung = RungKind::Yearly;
        yearly.out = dir.join(format!("from{year}.yearly.ladder"));
        let tail = run(&yearly).unwrap();
        let mut whole = full.clone();
        whole.rung = RungKind::Yearly;
        whole.keyframes = None;
        whole.out = dir.join("whole.yearly.ladder");
        whole.journal = dir.join("whole.journal.sqlite");
        let all = run(&whole).unwrap();
        let skip = usize::try_from(year).unwrap();
        assert_eq!(tail.rungs, all.rungs[skip..], "yearly from year {year}");
        assert_eq!(got.rungs[0].index, u64::try_from(year * 12 + 1).unwrap());
    }
}

#[test]
fn the_ladder_is_the_same_at_every_thread_count() {
    let dir = scratch("threads");
    let ladder = |threads: usize| {
        let args = RunArgs::new(
            &repo(),
            12,
            threads,
            RungKind::Monthly,
            &dir.join(format!("t{threads}.ladder")),
        );
        run(&args).unwrap();
        std::fs::read_to_string(&args.out).unwrap()
    };
    let one = ladder(1);
    for threads in [2, 4] {
        assert_eq!(ladder(threads), one, "{threads} threads, byte for byte");
    }
    let parsed = Ladder::parse(&one).unwrap();
    assert_eq!(parsed.rungs.len(), 144);
    assert!(
        parsed.rungs[0]
            .roots
            .iter()
            .any(|(t, _)| t == "rh-core/agenda.timers")
    );
}

/// The smoke world's tables and their row counts, from a short run's first rung dump.
fn world_tables() -> Vec<(String, usize)> {
    rh_sys_smoke::initial_tables()
        .columns()
        .map(|(name, _, rows)| (name.to_owned(), rows.len()))
        .collect()
}

const YEARS: i64 = 16;
/// Cases in the PR tier; `RH_GOLDEN_V_DET_11_CASES` raises it (the nightly tier runs 64).
const CASES: u64 = 4;
const TAG_CASES: u64 = tag("rehoboam/rh-golden/tests/v-det-11/v1");

#[test]
#[allow(
    clippy::disallowed_methods,
    reason = "tool plane: command-line arguments and environment overrides (SPR-011 A-4)"
)]
fn v_det_11_the_bisect_names_an_injected_month_table_and_row() {
    let tables = world_tables();
    let months = u64::try_from(YEARS).unwrap() * 12;
    let cases = std::env::var("RH_GOLDEN_V_DET_11_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(CASES);
    for case in 0..cases {
        let pick = |k: u64| h64_words(TAG_CASES, &[case, k]);
        let month = 1 + pick(0) % months;
        let (table, rows) = &tables[usize::try_from(pick(1)).unwrap() % tables.len()];
        let row = usize::try_from(pick(2)).unwrap() % rows;
        let inject = Inject {
            month,
            table: table.clone(),
            row,
        };
        let args = BisectArgs {
            repo: repo(),
            seed: 1,
            years: YEARS,
            threads: 1,
            work: scratch(&format!("bisect{case}")),
        };
        let candidate = InProcess {
            inject: Some(inject.clone()),
        };
        let report = bisect(&InProcess::default(), &candidate, &args).unwrap();
        let text = report.render();
        let l = report
            .located
            .as_ref()
            .unwrap_or_else(|| panic!("case {case} {inject:?}: not located\n{text}"));
        assert_eq!(l.month, month, "case {case} {inject:?}\n{text}");
        assert_eq!(l.year, i64::try_from((month - 1) / 12 + 1).unwrap());
        let r = l.row.as_ref().unwrap_or_else(|| panic!("no row\n{text}"));
        assert_eq!((&r.table, r.row, &r.column), (table, row, table), "{text}");
        assert!(text.contains(&format!("row: {row}\n")), "{text}");
        assert!(
            l.keyframe_year < l.year && l.keyframe_year % 10 == 0,
            "{text}"
        );
    }
    println!("V-DET-11: {cases} injected perturbations named exactly (month, table, row)");
}

#[test]
fn a_build_against_itself_has_no_divergence() {
    let args = BisectArgs {
        repo: repo(),
        seed: 3,
        years: 6,
        threads: 2,
        work: scratch("same"),
    };
    let report = bisect(&InProcess::default(), &InProcess::default(), &args).unwrap();
    assert_eq!(report.located, None);
    assert!(
        report
            .render()
            .ends_with("verdict: no divergence over the horizon\n")
    );
}

#[test]
fn ladders_from_two_seeds_differ_at_the_header() {
    let dir = scratch("seeds");
    let mut a = RunArgs::new(&repo(), 2, 1, RungKind::Yearly, &dir.join("a.ladder"));
    let mut b = a.clone();
    b.out = dir.join("b.ladder");
    b.journal = dir.join("b.journal.sqlite");
    b.seed = 2;
    a.seed = 1;
    let (la, lb) = (run(&a).unwrap(), run(&b).unwrap());
    assert!(matches!(
        compare(&la, &lb),
        Some(rh_golden::ladder::Difference::Header { field: "seed" })
    ));
}

/// VER-SPR-008 DEF-1: a resumed run's horizon is its journal's. A shorter `--years` (whose
/// history is a strict prefix of the journal's) is refused; a longer one is refused unless
/// `--extend`; `--until` stops early and says so; the header names the journal's identity.
#[test]
fn def_1_a_resumed_run_keeps_its_journals_horizon() {
    let dir = scratch("def1");
    let mut full = RunArgs::new(&repo(), 24, 1, RungKind::Yearly, &dir.join("full.ladder"));
    full.keyframes = Some(dir.join("kf"));
    let continuous = run(&full).unwrap();
    let journal = continuous.header.journal.clone().unwrap();
    assert_eq!(journal.0, 3 * 8, "inputs in years 7, 14 and 21, eight each");
    let resumed = |years: i64, extend: bool, until: Option<i64>, name: &str| {
        let mut args = RunArgs::new(&repo(), years, 1, RungKind::Yearly, &dir.join(name));
        args.journal.clone_from(&full.journal);
        args.resume = Some(keyframe_path(&dir.join("kf"), 10));
        args.extend = extend;
        args.until = until;
        run(&args)
    };
    let shorter = resumed(13, false, None, "shorter.ladder").unwrap_err();
    assert!(
        shorter.0.contains("disagrees with the journal's horizon"),
        "{shorter}"
    );
    assert!(
        resumed(13, true, None, "shorter_ext.ladder").is_err(),
        "--extend never shortens"
    );
    let longer = resumed(30, false, None, "longer.ladder").unwrap_err();
    assert!(
        longer.0.contains("past the journal's last input"),
        "{longer}"
    );
    let extended = resumed(30, true, None, "longer_ext.ladder").unwrap();
    assert_eq!(extended.header.journal, Some(journal.clone()));
    assert_eq!(extended.header.years, 30);
    assert_ne!(
        compare(&extended, &continuous),
        None,
        "an extended ladder never passes for the journal's own horizon"
    );
    // Within the horizon of 24 (the inputs of 21..27 are the same three batches), it resumes.
    let same = resumed(24, false, Some(18), "until.ladder").unwrap();
    assert_eq!(same.header.until, Some(18));
    assert_eq!(same.header.journal, Some(journal));
    assert_eq!(same.rungs, continuous.rungs[10..18], "years 11 to 18");
    assert!(
        std::fs::read_to_string(dir.join("until.ladder"))
            .unwrap()
            .contains("\nuntil 18\n")
    );
}
