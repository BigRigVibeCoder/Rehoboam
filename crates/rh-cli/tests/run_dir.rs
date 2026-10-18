//! Run-directory robustness left by SPR-013 (BCK-002 A-58; SPR-026, the live edge hosts the run
//! directory):
//! - **DEF-3:** a SIGKILL at each step of a new run directory's creation (the directory, the
//!   writer's lock, the journal, the ladder, `run.toml`) never leaves it unusable: before
//!   `run.toml` the same command creates the run afresh; after it, `--resume genesis` completes
//!   it. Either way the ladder is the baseline.
//! - **GAP-8:** a resume takes its rung from `run.toml` and refuses a conflicting `--ladder`.
//! - **GAP-12:** a resume refuses a torn rung at its keyframe's own tick.
//! - **Wording:** `replay` of a directory whose writer is running says so.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a tool-crate test that runs rh-cli as child processes; helpers fail loudly"
)]

mod support;

use std::path::Path;
use std::process::Command;

use support::{BIN, Host, baseline, ladders_equal, scratch};

const MPY: i64 = 518_400;

fn run(dir: &Path, extra: &[&str]) -> (bool, String) {
    let out = Command::new(BIN)
        .args(["run", "solo", "--years", "200", "--run-dir"])
        .arg(dir)
        .args(extra)
        .output()
        .unwrap();
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn killed_at(dir: &Path, stall: &str) {
    let d = dir.display().to_string();
    let mut h = Host::start(&[
        "run",
        "solo",
        "--years",
        "200",
        "--run-dir",
        &d,
        "--test-stall",
        stall,
    ]);
    h.wait_line("STALL");
    h.kill();
}

#[test]
fn def_3_no_kill_during_creation_leaves_an_unusable_directory() {
    for step in 1..=5 {
        let dir = scratch(&format!("create-{step}"));
        killed_at(&dir, &format!("create:{step}"));
        if step < 5 {
            assert!(
                !dir.join("run.toml").exists(),
                "step {step}: run.toml is written last"
            );
            let (ok, out) = run(&dir, &[]);
            assert!(
                ok,
                "step {step}: the same command creates the run afresh: {out}"
            );
        } else {
            let (ok, out) = run(&dir, &[]);
            assert!(!ok && out.contains("already holds a run"), "step 5: {out}");
            let (ok, out) = run(&dir, &["--resume", "genesis"]);
            assert!(ok, "step 5: --resume genesis completes the run: {out}");
        }
        assert!(
            ladders_equal(&dir.join("run.ladder"), &baseline("solo")),
            "step {step}: the ladder is the baseline"
        );
    }
    println!("DEF-3: a kill at each of the 5 creation steps leaves a usable run directory");
}

/// A monthly run killed mid-year 15, with its year-10 keyframe on disk.
fn monthly_killed(name: &str) -> std::path::PathBuf {
    let dir = scratch(name);
    let d = dir.display().to_string();
    let mid = format!("mid:{}", 15 * MPY + 43_200);
    let mut h = Host::start(&[
        "run",
        "solo",
        "--years",
        "200",
        "--run-dir",
        &d,
        "--ladder",
        "monthly",
        "--test-stall",
        &mid,
    ]);
    h.wait_line("STALL");
    h.kill();
    assert!(dir.join("keyframes/year-0010.rhkf").exists());
    dir
}

#[test]
fn gap_8_a_resume_takes_its_rung_from_run_toml() {
    let dir = monthly_killed("rung");
    let kf = dir.join("keyframes/year-0010.rhkf").display().to_string();
    let (ok, out) = run(&dir, &["--resume", &kf, "--ladder", "yearly"]);
    assert!(
        !ok && out.contains("conflicts with the run directory's rung, monthly"),
        "{out}"
    );
    let (ok, out) = run(&dir, &["--resume", &kf]);
    assert!(ok, "{out}");
    assert!(out.contains("monthly rungs"), "{out}");
    println!("GAP-8: the resume kept run.toml's monthly rung and refused --ladder yearly");
}

#[test]
fn gap_12_a_torn_rung_at_the_keyframe_is_refused() {
    let dir = scratch("torn");
    let d = dir.display().to_string();
    let mut h = Host::start(&[
        "run",
        "solo",
        "--years",
        "200",
        "--run-dir",
        &d,
        "--test-stall",
        &format!("mid:{}", 15 * MPY + 43_200),
    ]);
    h.wait_line("STALL");
    h.kill();
    // Tear the keyframe's rung (the end of sim-year 9): one hex digit of its state hash.
    let path = dir.join("run.ladder");
    let text = std::fs::read_to_string(&path).unwrap();
    let tick = (10 * MPY - 1).to_string();
    let torn: String = text
        .lines()
        .map(|l| {
            let f: Vec<&str> = l.split(' ').collect();
            if f.first() == Some(&"rung") && f.get(2) == Some(&tick.as_str()) {
                let mut hash = f[4].to_owned();
                let c = if hash.ends_with('0') { "1" } else { "0" };
                hash.replace_range(hash.len() - 1.., c);
                [&f[..4], &[hash.as_str()], &f[5..]].concat().join(" ")
            } else {
                l.to_owned()
            }
        })
        .map(|l| l + "\n")
        .collect();
    assert_ne!(torn, text, "the keyframe's rung was found");
    std::fs::write(&path, &torn).unwrap();
    let kf = dir.join("keyframes/year-0010.rhkf").display().to_string();
    let (ok, out) = run(&dir, &["--resume", &kf]);
    assert!(
        !ok && out.contains("is torn or is not the keyframe's state"),
        "{out}"
    );
    println!("GAP-12: a torn rung at the keyframe's tick is refused");
}

#[test]
fn replay_of_a_running_writers_directory_says_the_writer_is_running() {
    let dir = scratch("running");
    let d = dir.display().to_string();
    let mut h = Host::start(&[
        "run",
        "solo",
        "--years",
        "200",
        "--run-dir",
        &d,
        "--test-stall",
        &format!("mid:{}", 15 * MPY),
    ]);
    h.wait_line("STALL");
    let out = Command::new(BIN)
        .args(["replay", &d, "--into"])
        .arg(dir.with_extension("replay"))
        .output()
        .unwrap();
    h.kill();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success());
    assert!(text.contains("the run's writer is still running"), "{text}");
    assert!(!text.contains("killed and not resumed"), "{text}");
    println!("replay wording: a running writer's directory is named as such");
}
