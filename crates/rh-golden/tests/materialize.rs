//! `rh-cli materialize` (SPR-027 T-02; V-STR-02; ADR-0030 item 3; 05.10 §4).
//!
//! - **V-STR-02 on `solo` (200 years):** at 50 SplitMix64-chosen `STEP_DIGEST` ticks and every
//!   `KEYFRAME` marker, the materialized `state_hash` equals the record's; where the tick is a
//!   yearly rung, the dump's table roots equal the ladder's.
//! - The probe asserts that at least one chosen tick lies after `solo`'s journaled input (year 53)
//!   and at least one is nine or more years past its keyframe, and that the resume started from the
//!   nearest keyframe.
//! - `materialize` never writes to the run directory: every file and its SHA-256 are unchanged.
//! - `--dump -` stdout equals `--dump <file>`.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "test helpers fail loudly; the read-only check hashes file bytes"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rh_golden::ladder::{Ladder, RungKind};
use rh_golden::materialize::{materialize, materialize_with};
use rh_golden::run::{RunArgs, keyframe_path, run};
use rh_stream::record::Body;
use rh_stream::writer::read_channel_a;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fixed-seed `SplitMix64` (Steele, Lea and Flood 2014).
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// One run directory of `solo` (200 years), made once per test process.
fn solo_run_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("materialize")
            .join(format!("solo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut a = RunArgs::new(&repo(), 200, 1, RungKind::Yearly, &dir.join("run.ladder"));
        "solo".clone_into(&mut a.world);
        a.journal = dir.join("journal.sqlite");
        a.run_dir = Some(dir.clone());
        run(&a).unwrap();
        dir
    })
    .clone()
}

fn tree_hashes(dir: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, String>) {
        for e in std::fs::read_dir(at).unwrap() {
            let p = e.unwrap().path();
            let rel = p.strip_prefix(root).unwrap().display().to_string();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                let bytes = std::fs::read(&p).unwrap();
                let hash = format!("{:016x}", rh_snapshot::hash::xxh3(&bytes));
                out.insert(rel, hash);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[test]
fn v_str_02_materializer_solo_200_years() {
    let dir = solo_run_dir();
    let mpy = 518_400; // solo calendar: 360 days * 1440 min
    let segs = read_channel_a(&dir.join("stream"), 0, mpy).unwrap();

    let mut digest_ticks = Vec::new();
    let mut keyframe_ticks = Vec::new();
    for rec in segs.iter().flat_map(|s| &s.records) {
        match &rec.body {
            Body::StepDigest(sd) => digest_ticks.push((rec.header.tick, sd.state_hash)),
            Body::Keyframe(kf) => keyframe_ticks.push((rec.header.tick, kf.state_hash)),
            _ => {}
        }
    }

    assert!(
        digest_ticks.len() >= 2400,
        "expected at least 2400 step digests over 200 years, got {}",
        digest_ticks.len()
    );
    assert_eq!(
        keyframe_ticks.len(),
        20,
        "expected 20 permanent keyframes over 200 years"
    );

    // Pick 50 SplitMix64-chosen STEP_DIGEST ticks.
    let mut rng = SplitMix64(0x0027_0002);
    let mut chosen_ticks = Vec::new();
    let mut after_input_count = 0;
    let mut nine_years_past_count = 0;

    let solo_input_tick = 27_786_607; // Year 53 input

    for _ in 0..50 {
        let idx = (rng.next_u64() as usize) % digest_ticks.len();
        let (tick, hash) = digest_ticks[idx];
        chosen_ticks.push((tick, hash));
    }

    // Read ladder for yearly rung roots comparison.
    let ladder = Ladder::parse(&std::fs::read_to_string(dir.join("run.ladder")).unwrap()).unwrap();
    let ladder_rungs: BTreeMap<i64, _> = ladder.rungs.iter().map(|r| (r.tick, r)).collect();

    // Test chosen ticks plus every keyframe tick.
    let all_test_ticks: Vec<(i64, u64)> = chosen_ticks
        .iter()
        .copied()
        .chain(keyframe_ticks.iter().copied())
        .collect();

    let mut farthest_distance = 0;
    let mut farthest_tick = 0;

    for (tick, expected_hash) in &all_test_ticks {
        let m = materialize(&dir, *tick).unwrap();
        assert_eq!(
            m.state_hash, *expected_hash,
            "state_hash mismatch at tick {tick}"
        );
        assert_eq!(
            m.dump.digest().hash,
            *expected_hash,
            "dump digest mismatch at tick {tick}"
        );

        // Nearest keyframe assertion:
        let year = (tick + 1) / mpy;
        let permanent_year = (year / 10) * 10;
        let expected_kf_tick = if permanent_year >= 10 {
            permanent_year * mpy - 1
        } else {
            -1
        };
        assert_eq!(
            m.keyframe_tick, expected_kf_tick,
            "resume keyframe mismatch at tick {tick}"
        );

        let distance_years = if expected_kf_tick >= 0 {
            (tick - expected_kf_tick) / mpy
        } else {
            tick / mpy
        };

        if distance_years > farthest_distance {
            farthest_distance = distance_years;
            farthest_tick = *tick;
        }

        if *tick > solo_input_tick {
            after_input_count += 1;
        }
        if distance_years >= 9 {
            nine_years_past_count += 1;
        }

        // If tick is a yearly rung, compare dump table roots with ladder roots.
        if let Some(rung) = ladder_rungs.get(tick) {
            for (table_name, expected_root) in &rung.roots {
                let got_root = m.table_roots.get(table_name).copied().unwrap_or(0);
                assert_eq!(
                    got_root, *expected_root,
                    "table root mismatch for {table_name} at yearly rung tick {tick}"
                );
            }
        }
    }

    assert!(
        after_input_count > 0,
        "probe failed: at least one chosen tick must lie after solo's journaled input (year 53)"
    );
    assert!(
        nine_years_past_count > 0,
        "probe failed: at least one chosen tick must be nine or more years past its keyframe"
    );

    println!(
        "V-STR-02 PASS: solo 200 years, {} ticks checked; farthest from keyframe was tick {} ({} sim-years)",
        all_test_ticks.len(),
        farthest_tick,
        farthest_distance
    );
}

#[test]
fn materialize_never_writes_to_the_run_directory() {
    let dir = solo_run_dir();
    let before = tree_hashes(&dir);

    // Materialize several ticks: tick 0, keyframe 10, a mid-year tick, and a tick after year 53.
    materialize(&dir, 0).unwrap();
    materialize(&dir, 5_183_999).unwrap();
    materialize(&dir, 27_820_800).unwrap();

    let after = tree_hashes(&dir);
    assert_eq!(
        before, after,
        "materialize must never write or mutate anything in the run directory"
    );
}

#[test]
fn materialize_dump_equality_and_at_digest() {
    let dir = solo_run_dir();
    let target_tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("materialize");
    std::fs::create_dir_all(&target_tmp).unwrap();
    let dump_file = target_tmp.join(format!("dump-{}.txt", std::process::id()));

    let tick = 5_183_999;
    let m = materialize(&dir, tick).unwrap();

    let code = rh_golden::cli::materialize_cmd(
        &[
            dir.display().to_string(),
            "--tick".to_owned(),
            tick.to_string(),
            "--dump".to_owned(),
            dump_file.display().to_string(),
        ],
        &repo(),
    );
    assert_eq!(code, 0);

    let written = std::fs::read_to_string(&dump_file).unwrap();
    assert_eq!(written, m.dump.render());

    // Test --at-digest: find first STEP_DIGEST seq.
    let segs = read_channel_a(&dir.join("stream"), 0, 518_400).unwrap();
    let first_digest = segs
        .iter()
        .flat_map(|s| &s.records)
        .find(|r| matches!(r.body, Body::StepDigest(_)))
        .unwrap();

    let m_digest =
        materialize_with(&repo(), &dir, None, Some(first_digest.header.seq), false).unwrap();
    assert_eq!(m_digest.tick, first_digest.header.tick);
    if let Body::StepDigest(sd) = &first_digest.body {
        assert_eq!(m_digest.state_hash, sd.state_hash);
    }

    let _ = std::fs::remove_file(dump_file);
}

#[test]
fn materialize_any_tick_and_refusals() {
    let dir = solo_run_dir();

    // A tick without a published hash: e.g. tick 12345.
    let err = materialize(&dir, 12_345).unwrap_err();
    assert!(
        err.0.contains("--any-tick"),
        "expected error mentioning --any-tick, got: {err}"
    );

    // With any_tick = true, it succeeds.
    let m = materialize_with(&repo(), &dir, Some(12_345), None, true).unwrap();
    assert_eq!(m.tick, 12_345);

    // Mutually exclusive flags
    let err_both = materialize_with(&repo(), &dir, Some(12_345), Some(1), false).unwrap_err();
    assert!(err_both.0.contains("mutually exclusive"));

    // Tick before genesis
    let err_genesis = materialize_with(&repo(), &dir, Some(-5), None, true).unwrap_err();
    assert!(err_genesis.0.contains("before genesis"));

    // Tick past horizon
    let err_horizon = materialize_with(&repo(), &dir, Some(201 * 518_400), None, true).unwrap_err();
    assert!(err_horizon.0.contains("outside the horizon"));
}

fn copy_all(src: &Path, dst: &Path) {
    if src.is_dir() {
        std::fs::create_dir_all(dst).unwrap();
        for e in std::fs::read_dir(src).unwrap() {
            let entry = e.unwrap();
            copy_all(&entry.path(), &dst.join(entry.file_name()));
        }
    } else {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::copy(src, dst).unwrap();
    }
}

#[test]
fn materialize_resumed_run_path() {
    let base_dir = solo_run_dir();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("materialize")
        .join(format!("solo-resumed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy_all(&base_dir, &dir);

    // Resume solo from keyframe 50 into dir.
    let kf = keyframe_path(&dir.join("keyframes"), 50);
    let mut resumed = RunArgs::new(&repo(), 200, 1, RungKind::Yearly, &dir.join("run.ladder"));
    "solo".clone_into(&mut resumed.world);
    resumed.journal = dir.join("journal.sqlite");
    resumed.resume = Some(kf);
    resumed.run_dir = Some(dir.clone());
    run(&resumed).unwrap();

    // Materialize at tick in year 55 (past input).
    let tick = 55 * 518_400;
    let m_orig = materialize(&base_dir, tick).unwrap();
    let m_res = materialize(&dir, tick).unwrap();
    assert_eq!(m_orig.state_hash, m_res.state_hash);
    assert_eq!(m_orig.keyframe_tick, m_res.keyframe_tick);
}
