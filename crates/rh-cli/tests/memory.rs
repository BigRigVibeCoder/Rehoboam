//! GAP-1 (SPR-026; the Architect's ruling on VER-SPR-026-TEST): `LiveEdge::bytes()` counts the
//! seal queue as well as the window, so it accounts for what the live edge holds.
//!
//! `solo` runs 1,500 sim-years at full speed: three times headless (in parallel; each process's
//! `VmHWM` is its own), then with `--socket` under (b), where the seal thread falls behind and its
//! queue holds most of the run. The host's peak resident memory (`VmHWM`, polled from
//! `/proc/<pid>/status` while it runs) rises over the **highest of the three headless** runs by at
//! most 2 × the peak `LiveEdge::bytes()` the host reports, plus a stated allowance of 2 MiB for
//! the allocator's residency. The maximum of three takes out the run-to-run noise of headless
//! `VmHWM` (about 1.9 to 2.9 MB between identical runs: VER-SPR-026-TEST GAP-8, the Architect's
//! ruling). The RSS bound is 2 × peak `bytes()` + 2 MiB, and exact accounting of window and queue
//! is guaranteed deterministically by `rh-stream`'s accounting test
//! (`deterministic_accounting_recounts_window_and_seal_queue_by_allocated`).
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a tool-crate test that runs rh-cli as child processes; helpers fail loudly"
)]

mod support;

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::Duration;

use support::{BIN, scratch};

const YEARS: &str = "1500";
const ALLOWANCE: u64 = 2 << 20;

/// `VmHWM` of `pid` in bytes, `None` once the process is gone.
fn vm_hwm(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status
        .lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
        .map(|kib| kib * 1024)
}

/// Runs `rh-cli run solo` with `extra`; (its highest `VmHWM` seen, its stdout lines).
fn measured(name: &str, extra: &[&str]) -> (u64, Vec<String>) {
    let dir = scratch(name);
    let mut child = Command::new(BIN)
        .args(["run", "solo", "--years", YEARS, "--run-dir"])
        .arg(&dir)
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id();
    let stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        BufReader::new(stdout)
            .lines()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    });
    let mut peak = 0;
    while child.try_wait().unwrap().is_none() {
        if let Some(v) = vm_hwm(pid) {
            peak = peak.max(v);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let lines = reader.join().unwrap();
    assert!(child.wait().unwrap().success(), "{name}: {lines:?}");
    (peak, lines)
}

#[test]
fn the_hosts_memory_over_headless_is_within_twice_the_live_edge_bytes() {
    let runs: Vec<_> = (0..3)
        .map(|i| std::thread::spawn(move || measured(&format!("hwm-headless-{i}"), &[]).0))
        .collect();
    let all: Vec<u64> = runs.into_iter().map(|h| h.join().unwrap()).collect();
    let headless = all.iter().copied().max().unwrap();
    // A linger of one second: the host is still alive, its high-water mark final, after the run.
    let (live, lines) = measured("hwm-live", &["--socket", "--linger", "1"]);
    let bytes: u64 = lines
        .iter()
        .find_map(|l| l.split("peak LiveEdge::bytes() ").nth(1))
        .and_then(|rest| rest.split(';').next())
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or_else(|| panic!("no peak LiveEdge::bytes() in {lines:?}"));
    let rise = live.saturating_sub(headless);
    let bound = 2 * bytes + ALLOWANCE;
    #[allow(clippy::cast_precision_loss, reason = "a printed ratio")]
    let ratio = rise as f64 / bytes as f64;
    println!(
        "GAP-1: solo {YEARS} years at full speed: VmHWM headless {headless} B (the highest of \
         {all:?}), with --socket \
         {live} B, rise {rise} B = {ratio:.2} x peak LiveEdge::bytes() {bytes} B; bound 2 x bytes \
         + {ALLOWANCE} B = {bound} B"
    );
    assert!(
        rise <= bound,
        "the host rose {rise} B over headless, over 2 x peak LiveEdge::bytes() {bytes} B + \
         {ALLOWANCE} B: the live edge holds memory bytes() does not count"
    );
}
