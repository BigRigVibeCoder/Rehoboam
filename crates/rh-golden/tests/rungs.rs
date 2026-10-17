//! Rungs 0, 1 and 2 on the same build (SPR-013 T-13; V-SCALE-04, PR subset).
//!
//! One `rh-golden` build runs `solo` (1 person), `household` (5) and `hamlet` (60); only the
//! manifest changes. Each of `household` and `hamlet` runs past its extinction (date years 0061 and 0078)
//! with its committed input script, and:
//! - every founder's life closes: as many `LIFE_EXIT` records as `LIFE_ENTRY` records, each exit
//!   a death with its `DEATH` event;
//! - exactly one `EXTINCTION`, after the last death, and after it nothing but the last death's
//!   `LIFE_EXIT` at the same instant (R4 orders EVENT before LIFE), then `STEP_DIGEST` (all with
//!   `living` 0) and `KEYFRAME` records;
//! - the chronicle stays consistent past it: `tail --canonical` prints every record's canonical
//!   line once, in seq order; `sift` gives one `life_ends` match per death; a replay from the
//!   journal alone equals the run.
//!
//! The 1,000-year form (and V-SCALE-01 at 1, 2, 4 and N threads) is nightly (`bin/golden_nightly.sh`).
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers fail loudly"
)]

use std::path::{Path, PathBuf};

use rh_golden::ladder::RungKind;
use rh_golden::replay::{ReplayArgs, replay};
use rh_golden::run::{RunArgs, run_reporting};
use rh_golden::sift::sift;
use rh_golden::tail::{TailArgs, no_annotations, tail};
use rh_stream::canonical::render;
use rh_stream::record::{Body, Kind};
use rh_stream::writer::read_channel_a;

const MPY: i64 = 518_400;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn stream(world: &str, years: i64) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("rungs")
        .join(world);
    let _ = std::fs::remove_dir_all(&dir);
    let mut a = RunArgs::new(&repo(), years, 1, RungKind::Yearly, &dir.join("run.ladder"));
    world.clone_into(&mut a.world);
    a.journal = dir.join("journal.sqlite");
    a.run_dir = Some(dir.clone());
    let o = run_reporting(&a).unwrap();
    assert!(o.ladder.failed.is_none(), "{world}");
    assert_eq!(
        o.dispatch.max_jobs, 1,
        "{world}: a one-system world is serial"
    );
    dir
}

#[test]
fn every_rung_runs_past_its_extinction_and_its_chronicle_stays_consistent() {
    let solo = stream("solo", 60);
    assert_eq!(read_channel_a(&solo.join("stream"), 0, MPY).unwrap().len(), 60);
    for (world, years, extinct_by) in [("household", 80, 62), ("hamlet", 100, 79)] {
        let dir = stream(world, years);
        let records: Vec<_> = read_channel_a(&dir.join("stream"), 0, MPY)
            .unwrap()
            .into_iter()
            .flat_map(|s| s.records)
            .collect();
        let count = |k: Kind| records.iter().filter(|r| r.kind() == k).count();
        let deaths = records
            .iter()
            .filter(|r| matches!(&r.body, Body::Event(e) if e.verb == "DEATH"))
            .count();
        assert!(deaths > 0);
        assert_eq!(
            count(Kind::LifeEntry),
            deaths,
            "{world}: every founder dies"
        );
        assert_eq!(count(Kind::LifeExit), deaths, "{world}: one exit per death");
        let ext: Vec<usize> = records
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(&r.body, Body::Event(e) if e.verb == "EXTINCTION"))
            .map(|(i, _)| i)
            .collect();
        let [at] = ext.as_slice() else {
            panic!("{world}: {} EXTINCTION records", ext.len());
        };
        let tick = records[*at].header.tick;
        assert!(tick < extinct_by * MPY, "{world}: extinct at tick {tick}");
        for r in &records[at + 1..] {
            match &r.body {
                Body::StepDigest(d) => assert_eq!(d.living, 0, "{world}: life after EXTINCTION"),
                Body::Keyframe(_) => {}
                // R4: at one instant EVENT records precede LIFE records, so the last death's
                // LIFE_EXIT follows the EXTINCTION at the same tick.
                Body::LifeExit(_) if r.header.tick == tick => {}
                other => panic!("{world}: a {:?} record after EXTINCTION", other.kind()),
            }
        }
        // tail --canonical prints every record's line once, in seq order.
        let mut out = Vec::new();
        tail(
            &TailArgs {
                run_dir: dir.clone(),
                repo: repo(),
                canonical: true,
                ..TailArgs::default()
            },
            &no_annotations,
            &mut out,
        )
        .unwrap();
        let printed: Vec<String> = String::from_utf8(out)
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(str::to_owned)
            .collect();
        let want: Vec<String> = records.iter().map(|r| render(r).unwrap()).collect();
        assert_eq!(printed, want, "{world}: tail");
        let (_, sifted) = sift(&repo(), &dir).unwrap();
        assert_eq!(
            usize::try_from(sifted.matches).unwrap(),
            deaths,
            "{world}: sift"
        );
        let r = replay(&ReplayArgs {
            repo: repo(),
            run_dir: dir.clone(),
            into: dir.with_extension("replayed"),
            from_keyframe: None,
            threads: 1,
        })
        .unwrap();
        assert!(r.equal(), "{world}: {:?}", r.differences);
        println!(
            "T-13 V-SCALE-04: {world}: {deaths} founders die, one EXTINCTION at tick {tick} (date year {:04}), then {} digests and keyframes only; tail, sift ({} matches) and replay consistent past it",
            tick / MPY,
            records.len() - at - 1,
            sifted.matches
        );
    }
    println!(
        "T-13: one build ran rungs 0 (solo), 1 (household) and 2 (hamlet); only the manifest changed"
    );
}
