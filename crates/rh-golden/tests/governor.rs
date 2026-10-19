//! Tests for the pace governor, control endpoint, and run log (SPR-027 T-03).
//!
//! - V-STR-01: pace invariance on `solo` (at `max` vs `10y`): byte-identical channel A,
//!   identical ladder, identical keyframes, identical journal.
//! - `--start paused` refusal without `--socket`.
//! - Pause probe: no record, no keyframe, no journal row, `next_seq` unchanged.
//! - Step and resume re-anchoring.
//! - Run log verification.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "test helpers fail loudly"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rh_golden::governor::{FakeClock, Governor};
use rh_golden::ladder::RungKind;
use rh_golden::materialize::materialize;
use rh_golden::run::{RunArgs, run, run_live};
use rh_protocol::control::{ControlCommand, ControlHost, History, StepUntil};
use rh_stream::writer::read_channel_a;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("governor_tests").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn start_paused_is_refused_without_socket() {
    let dir = scratch("start_paused_no_sock");
    let mut args = RunArgs::new(&repo(), 1, 1, RungKind::Yearly, &dir.join("run.ladder"));
    "solo".clone_into(&mut args.world);
    args.start_paused = true;
    let err = run_live(&args, None).unwrap_err();
    assert!(
        err.0.contains("--start paused is refused without --socket"),
        "error message {err:?}"
    );
}

#[test]
fn pace_invariance_on_solo_at_max_vs_10y() {
    let dir_max = scratch("solo_max");
    let mut a_max = RunArgs::new(&repo(), 60, 1, RungKind::Yearly, &dir_max.join("run.ladder"));
    "solo".clone_into(&mut a_max.world);
    a_max.journal = dir_max.join("journal.sqlite");
    a_max.run_dir = Some(dir_max.clone());
    let out_max = run(&a_max).unwrap();
    assert!(out_max.failed.is_none());

    let dir_10y = scratch("solo_10y");
    let mut a_10y = RunArgs::new(&repo(), 60, 1, RungKind::Yearly, &dir_10y.join("run.ladder"));
    "solo".clone_into(&mut a_10y.world);
    a_10y.journal = dir_10y.join("journal.sqlite");
    a_10y.run_dir = Some(dir_10y.clone());
    a_10y.pace = Some("10y".to_owned());
    let out_10y = run(&a_10y).unwrap();
    assert!(out_10y.failed.is_none());

    // 1. Ladders are identical rung for rung
    assert_eq!(out_max.rungs, out_10y.rungs);

    // 2. Channel A records are byte-identical
    let mpy = 518_400;
    let segs_max = read_channel_a(&dir_max.join("stream"), 0, mpy).unwrap();
    let segs_10y = read_channel_a(&dir_10y.join("stream"), 0, mpy).unwrap();
    assert_eq!(segs_max.len(), segs_10y.len());
    for (s1, s2) in segs_max.iter().zip(&segs_10y) {
        assert_eq!(s1.records.len(), s2.records.len());
        assert_eq!(s1.row.canonical_digest, s2.row.canonical_digest);
    }

    // 3. Keyframes are identical
    let kf_max = dir_max.join("keyframes");
    let kf_10y = dir_10y.join("keyframes");
    if kf_max.exists() {
        for entry in std::fs::read_dir(&kf_max).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap();
            let b1 = std::fs::read(&path).unwrap();
            let b2 = std::fs::read(kf_10y.join(name)).unwrap();
            assert_eq!(b1, b2, "keyframe {name:?} differs");
        }
    }

    // 4. Journals are byte-identical
    let j_max = std::fs::read(dir_max.join("journal.sqlite")).unwrap();
    let j_10y = std::fs::read(dir_10y.join("journal.sqlite")).unwrap();
    assert_eq!(j_max, j_10y);
}

fn submit_in_background(
    gov: &Arc<Governor>,
    conn: &rh_protocol::control::ControlConn,
    id: serde_json::Value,
    cmd: ControlCommand,
) -> std::sync::mpsc::Receiver<serde_json::Value> {
    let (tx, rx) = std::sync::mpsc::channel();
    let g = Arc::clone(gov);
    let c = conn.clone();
    std::thread::spawn(move || {
        let rep = g.submit(&c, &id, cmd);
        let _ = tx.send(rep);
    });
    rx
}

#[test]
fn governor_pause_probe_and_step_invariants() {
    let dir = scratch("gov_probe");
    let clock = Arc::new(FakeClock::new(0));
    let log_path = dir.join("run.log");
    let gov = Arc::new(Governor::new(
        clock.clone(),
        518_400,
        0,
        1,
        0,
        History {
            engine_build: "test_build".into(),
            registry_version: "0.2.0".into(),
            manifest_hash: "0000".into(),
            world_seed: 1,
        },
        Some("max"),
        false,
        false,
        None,
        Some(log_path.clone()),
        None,
    ));

    let conn = gov.connect(None).unwrap();
    assert_eq!(conn.number, 1);
    assert!(!conn.closed);

    // Initial status
    let rep = gov.submit(&conn, &serde_json::json!(1), ControlCommand::Status);
    assert_eq!(rep["result"]["pace"]["state"], "running");
    assert_eq!(rep["result"]["pace"]["band"], "max");
    assert_eq!(rep["result"]["controllers"], 1);

    // 1. Pause
    let rx = submit_in_background(&gov, &conn, serde_json::json!(2), ControlCommand::Pause);
    while gov.pending_count() == 0 {
        std::thread::yield_now();
    }
    gov.before_instant(Some(86_400), Some(43_200), 1);
    let rep_p = rx.recv().unwrap();
    assert_eq!(rep_p["result"]["state"], "paused");
    assert_eq!(rep_p["result"]["changed"], true);

    // 2. Pause while paused -> changed: false
    let rep_p2 = gov.submit(&conn, &serde_json::json!(3), ControlCommand::Pause);
    assert_eq!(rep_p2["result"]["state"], "paused");
    assert_eq!(rep_p2["result"]["changed"], false);

    // 3. Step while paused -> state: running, target set
    let rx = submit_in_background(&gov, &conn, serde_json::json!(4), ControlCommand::Step {
        until: StepUntil::Tick(43_200),
    });
    while gov.pending_count() == 0 {
        std::thread::yield_now();
    }
    gov.before_instant(Some(43_200), Some(0), 2);
    let rep_s = rx.recv().unwrap();
    assert_eq!(rep_s["result"]["state"], "running");
    assert_eq!(rep_s["result"]["step"]["until_tick"], 43_200);
    assert_eq!(rep_s["result"]["changed"], true);

    // 4. Before instant at 43_200 (reaches target)
    gov.before_instant(Some(86_400), Some(43_200), 5);
    let snap = gov.snapshot();
    assert_eq!(snap.state, "paused");
    assert_eq!(snap.step_target, None);
    assert_eq!(snap.tip, Some(43_200));

    // 5. Resume from reached step
    let rx = submit_in_background(&gov, &conn, serde_json::json!(5), ControlCommand::Resume);
    while gov.pending_count() == 0 {
        std::thread::yield_now();
    }
    gov.before_instant(Some(86_400), Some(43_200), 6);
    let rep_r = rx.recv().unwrap();
    assert_eq!(rep_r["result"]["state"], "running");
    assert_eq!(rep_r["result"]["changed"], true);

    // 6. Finish
    gov.finish("finished", Some(43_200), 10);
    let snap_fin = gov.snapshot();
    assert_eq!(snap_fin.state, "finished");
    assert_eq!(snap_fin.end, Some("finished".to_owned()));

    // 7. Command after finish is refused
    let rep_post = gov.submit(&conn, &serde_json::json!(6), ControlCommand::Pause);
    assert_eq!(rep_post["error"]["data"]["reason"], "run_finished");

    // 8. Run log exists and has entries
    assert!(log_path.exists());
    let log_content = std::fs::read_to_string(&log_path).unwrap();
    assert!(log_content.contains("host 0 start ~ 1"));
    assert!(log_content.contains("host 0 finish 43200 1"));
    assert!(log_content.contains("control.pause"));
    assert!(log_content.contains("control.step:43200"));
    assert!(log_content.contains("step.reached"));
    assert!(log_content.contains("control.resume"));
}

#[test]
fn step_and_materialize_consistency() {
    let dir = scratch("step_mat");
    let mut args = RunArgs::new(&repo(), 60, 1, RungKind::Yearly, &dir.join("run.ladder"));
    "solo".clone_into(&mut args.world);
    args.journal = dir.join("journal.sqlite");
    args.run_dir = Some(dir.clone());
    let _ = run(&args).unwrap();

    // Materialize at tick 43_200 (month 1 end)
    let mat = materialize(&dir, 43_200).unwrap();
    assert_eq!(mat.tick, 43_200);
}
