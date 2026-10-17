//! The golden runner on the manifest world `solo` (SPR-012 T-08; SPR-011 rulings 3, 15, 16).
//!
//! - A run resumed from one of its 10-year keyframes through the journal writes the continuous
//!   run's ladder, rung for rung (V-DET-05 on `solo`).
//! - The monthly ladder does not depend on the thread count (V-DET-01/02 at small scale; the PR
//!   gate compares 200 yearly rungs, the nightly tier 1,000 monthly ones at 1, 2, 4 and N threads).
//! - The header carries the history tuple and the built and not-built systems.
//! - The manifest pin: a manifest whose bytes drift from `world.toml`'s SHA-256 is refused.
//! - `--strict` refuses `solo`; genesis gives Ada, female, aged 25 at tick 0.
//!
//! Refs: SPR-012 T-08; `docs/09` V-DET-01, V-DET-02, V-DET-05.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers: a broken run must fail the test loudly"
)]

use std::path::{Path, PathBuf};

use rh_golden::ladder::{RungKind, compare};
use rh_golden::run::{RunArgs, keyframe_path, run};
use rh_golden::world::{check_pin, setup};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn solo(years: i64, threads: usize, rung: RungKind, out: &Path) -> RunArgs {
    let mut args = RunArgs::new(&repo(), years, threads, rung, out);
    "solo".clone_into(&mut args.world);
    args
}

#[test]
fn a_resumed_solo_run_writes_the_continuous_runs_ladder() {
    let dir = scratch("solo_resume");
    let mut full = solo(60, 1, RungKind::Monthly, &dir.join("full.ladder"));
    full.keyframes = Some(dir.join("kf"));
    let continuous = run(&full).unwrap();
    assert!(continuous.failed.is_none());
    assert_eq!(continuous.rungs.len(), 60 * 12);
    for year in [10, 20, 50] {
        let mut resumed = solo(
            60,
            1,
            RungKind::Monthly,
            &dir.join(format!("from{year}.ladder")),
        );
        resumed.journal.clone_from(&full.journal);
        resumed.resume = Some(keyframe_path(&dir.join("kf"), year));
        let got = run(&resumed).unwrap();
        let from = continuous
            .rungs
            .iter()
            .position(|r| r.index == got.rungs[0].index)
            .unwrap();
        assert_eq!(got.rungs, continuous.rungs[from..], "from year {year}");
        assert_eq!(got.rungs[0].index, u64::try_from(year * 12 + 1).unwrap());
    }
    println!(
        "T-08: solo resumed from the keyframes of years 10, 20 and 50 equals the continuous run, month for month"
    );
}

#[test]
fn the_solo_ladder_does_not_depend_on_the_thread_count() {
    let dir = scratch("solo_threads");
    let one = run(&solo(60, 1, RungKind::Monthly, &dir.join("t1.ladder"))).unwrap();
    for threads in [2, 4] {
        let other = run(&solo(
            60,
            threads,
            RungKind::Monthly,
            &dir.join(format!("t{threads}.ladder")),
        ))
        .unwrap();
        assert_eq!(compare(&one, &other), None, "{threads} threads");
    }
}

#[test]
fn the_header_carries_the_history_tuple_and_the_built_systems() {
    let dir = scratch("solo_header");
    let ladder = run(&solo(60, 1, RungKind::Yearly, &dir.join("h.ladder"))).unwrap();
    let h = &ladder.header;
    assert_eq!((h.world.as_str(), h.seed), ("solo", 1));
    let tuple = h.tuple.as_deref().unwrap();
    assert!(tuple.starts_with("engine_build=rh-golden-"), "{tuple}");
    assert!(tuple.contains("built:M2.demography"), "{tuple}");
    assert!(
        tuple.contains(" registry_version=0.2.0 manifest_hash=sha256:"),
        "{tuple}"
    );
    assert_eq!(
        h.systems.as_deref(),
        Some("built M2.demography not_built M1.regions,M3.agriculture,M4.economy,S5.daily_life")
    );
    assert_eq!(
        h.journal.as_ref().map(|j| j.0),
        Some(1),
        "one scripted input"
    );
    // A seed given on the command line is the run's, and the header says so.
    // 60 years: the committed input on Ada (sim-year 53) is within the horizon (GAP-3).
    let mut other = solo(60, 1, RungKind::Yearly, &dir.join("s.ladder"));
    other.seed = 7;
    other.seed_set = true;
    let seeded = run(&other).unwrap();
    assert_eq!(seeded.header.seed, 7);
    assert!(seeded.header.tuple.unwrap().ends_with(" seed=7"));
}

#[test]
fn a_drifted_manifest_is_refused_by_its_pin() {
    let dir = scratch("solo_pin");
    let manifest = "specs/manifest/examples/solo.yaml";
    std::fs::create_dir_all(dir.join("worlds/solo")).unwrap();
    std::fs::create_dir_all(dir.join("specs/manifest/examples")).unwrap();
    std::fs::copy(
        repo().join("worlds/solo/world.toml"),
        dir.join("worlds/solo/world.toml"),
    )
    .unwrap();
    let mut bytes = std::fs::read(repo().join(manifest)).unwrap();
    std::fs::write(dir.join(manifest), &bytes).unwrap();
    assert!(check_pin(&dir, "solo").is_ok(), "the committed pin holds");
    bytes.extend_from_slice(b"# one more line\n");
    std::fs::write(dir.join(manifest), &bytes).unwrap();
    let e = check_pin(&dir, "solo").unwrap_err();
    assert!(e.0.starts_with("MANIFEST PIN: "), "{e}");
    assert!(setup(&dir, "solo", None, false).is_err());
}

#[test]
fn strict_refuses_solo_and_genesis_gives_ada() {
    let Err(e) = setup(&repo(), "solo", None, true) else {
        panic!("--strict ran a manifest with systems this build lacks")
    };
    assert!(e.0.starts_with("--strict refuses solo"), "{e}");
    let world = setup(&repo(), "solo", None, false).unwrap();
    assert_eq!(world.not_built.len(), 4);
    let t = &world.tables;
    assert_eq!(t.get("person.id").unwrap().len(), 1);
    assert_eq!(t.get("person.sex").unwrap(), &vec![rh_worldgen::SEX_FEMALE]);
    let birth = t.get("person.birth_tick").unwrap()[0];
    let year = world.calendar.minutes_per_year();
    assert!(
        (25 * year..26 * year).contains(&-birth),
        "Ada's age at tick 0: {birth}"
    );
}

/// The golden history takes the re-sample path (the Architect's ruling on the Tester's interim
/// report): Ada's death is pending at the scripted input, the input cancels it, the re-sample at
/// epoch 1 misses, and she dies later, at the instant `worlds/solo/inputs.toml` states.
#[test]
fn solos_input_re_samples_adas_pending_death() {
    use rh_core::engine::{Engine, MaterialInput};
    use rh_core::register::register;
    use rh_core::time::SimTime;
    use rh_ids::BranchSalt;

    let world = setup(&repo(), "solo", None, false).unwrap();
    let registered = register((world.systems)(false), &world.specs).unwrap();
    let mut e = Engine::new(
        world.calendar,
        world.seed,
        BranchSalt::from_raw(0),
        registered,
        world.tables.clone(),
        SimTime::EPOCH,
    );
    let inputs = (world.inputs)(200);
    assert_eq!(inputs.len(), 1);
    let (t_in, ada) = inputs[0];
    assert_eq!(t_in.minutes(), 27_786_607);
    e.submit_input(
        t_in,
        MaterialInput {
            scope: std::collections::BTreeSet::from([ada]),
        },
    )
    .unwrap();
    e.advance_until(SimTime::from_minutes(t_in.minutes() - 1), |_, _| {})
        .unwrap();
    let pending: Vec<i64> = e
        .agenda()
        .pending()
        .iter()
        .map(|(k, _)| k.time.minutes())
        .collect();
    assert_eq!(
        pending,
        vec![27_796_320],
        "Ada's death is pending at the input"
    );
    e.advance_until(t_in, |_, _| {}).unwrap();
    assert_eq!(e.epoch(ada), 1);
    assert!(
        e.agenda().pending().is_empty(),
        "the re-sample missed: the death is dropped"
    );
    e.advance_until(
        SimTime::from_minutes(60 * world.calendar.minutes_per_year()),
        |_, _| {},
    )
    .unwrap();
    let deaths: Vec<i64> = e
        .drain_events()
        .into_iter()
        .filter(|ev| ev.kind == "DEATH")
        .map(|ev| ev.tick)
        .collect();
    assert_eq!(
        deaths,
        vec![29_975_040],
        "Ada dies at the instant inputs.toml states"
    );
    println!(
        "T-08: solo's input re-samples Ada's pending death (a miss); she dies at tick 29975040"
    );
}
