//! V-PERF-09's ingredients (SPR-026 T-05, R23; the gate is SPR-030's on the 10k world): the host's
//! live heap bytes, counted with SPR-009's counting allocator rather than measured as RSS, while
//! viewers read the live edge of a streaming run, set against the terms of the bound
//! `LiveEdge::bytes()` (peak) + V x 1 MiB (one frame per viewer) + an allowance.
//!
//! Each world runs twice in this process under the counting allocator: headless, then with a live
//! edge (D42's option (b)) read by V = 2 in-process viewers that do what a serving thread does
//! (`LiveEdge::read` into a frame of at most 1 MiB): one follows the edge, one is stalled (it takes
//! one frame and never reads again, holding it). A sampler records the peak `LiveEdge::bytes()`;
//! the allocator records the peak live bytes of each run. Every term is printed; nothing is gated.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a bench target: it fails loudly"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use rh_bench::alloc_count::{Counting, peak_bytes, reset_peak};
use rh_golden::run::{LiveHost, run_live};
use rh_stream::live::{LiveEdge, Policy};

#[global_allocator]
static COUNTING: Counting = Counting;

/// One frame's body cap (the 1 MiB payload less the RECORDS sub-header).
const FRAME: usize = 1024 * 1024 - 20;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn args(world: &str, dir: &Path) -> rh_golden::run::RunArgs {
    let _ = std::fs::remove_dir_all(dir);
    let words: Vec<String> = ["--years", "200", "--run-dir"]
        .iter()
        .map(|s| (*s).to_owned())
        .chain([dir.display().to_string()])
        .collect();
    let mut a = rh_golden::cli::parse_run(&words, &repo()).unwrap();
    world.clone_into(&mut a.world);
    a
}

fn viewers(edge: &LiveEdge, stop: &Arc<AtomicBool>, peak: &Arc<AtomicUsize>) {
    // The edge follower: copies frames from its cursor, as a serving thread does.
    let (e, s) = (edge.clone(), Arc::clone(stop));
    std::thread::spawn(move || {
        let mut c = 0;
        while !s.load(Ordering::Relaxed) {
            match e.read(c, FRAME) {
                Ok(l) if l.count > 0 => c += l.count,
                Ok(_) => std::thread::sleep(Duration::from_millis(1)),
                Err(b) => c = b.s.map_or(0, |s| s + 1),
            }
        }
    });
    // The stalled viewer: one frame taken and held; it never reads again.
    let (e, s) = (edge.clone(), Arc::clone(stop));
    std::thread::spawn(move || {
        let held = loop {
            if let Ok(l) = e.read(0, FRAME)
                && l.count > 0
            {
                break l;
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        while !s.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(5));
        }
        drop(held);
    });
    // The sampler of LiveEdge::bytes().
    let (e, s, p) = (edge.clone(), Arc::clone(stop), Arc::clone(peak));
    std::thread::spawn(move || {
        while !s.load(Ordering::Relaxed) {
            p.fetch_max(e.bytes(), Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(1));
        }
    });
}

fn main() {
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("live_edge");
    std::fs::create_dir_all(&tmp).unwrap();
    for world in ["solo", "hamlet"] {
        reset_peak();
        let before = rh_bench::alloc_count::live_bytes();
        run_live(&args(world, &tmp.join(format!("{world}-headless"))), None).unwrap();
        let headless = peak_bytes().saturating_sub(before);
        reset_peak();
        let before = rh_bench::alloc_count::live_bytes();
        let stop = Arc::new(AtomicBool::new(false));
        let edge_peak = Arc::new(AtomicUsize::new(0));
        let (s, p) = (Arc::clone(&stop), Arc::clone(&edge_peak));
        let live = LiveHost {
            policy: Policy::YearEnd,
            clock: None,
            nonce: None,
            on_edge: Box::new(move |edge, _, _| {
                viewers(&edge, &s, &p);
                Ok(())
            }),
        };
        run_live(&args(world, &tmp.join(format!("{world}-live"))), Some(live)).unwrap();
        stop.store(true, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(20));
        let with = peak_bytes().saturating_sub(before);
        let edge = edge_peak.load(Ordering::Relaxed);
        let v = 2;
        println!(
            "V-PERF-09 ingredients {world}: host live bytes peak {with} with the live edge and {v} \
             viewers, {headless} headless (difference {}); terms: peak LiveEdge::bytes() {edge}, \
             V x frame cap {v} x {FRAME} = {}; the difference is within LiveEdge::bytes() + V x 1 MiB: \
             {} (no gate: SPR-030's V-PERF-09)",
            with.saturating_sub(headless),
            v * FRAME,
            with.saturating_sub(headless) <= edge + v * FRAME
        );
    }
}
