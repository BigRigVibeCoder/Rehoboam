//! V-PERF-11: heap allocations per steady-state tick (SPR-009 T-05, ruling 5).
//!
//! This target installs the counting allocator ([`rh_bench::alloc_count::Counting`]) as its
//! `#[global_allocator]`: the only one in the workspace (`bin/lint_arch.py`, global-allocator), in
//! the only target that enables `count-alloc`. It first checks the counter on a pattern whose
//! counts are known exactly, so a broken counter can never report a passing budget.
//!
//! Run: `cargo bench -p rh-bench --features count-alloc --bench alloc_budget`.
//!
//! REF: `docs/09` V-PERF-11; `CODEX/05_PROJECT/SPR-009_Benchmarks.md` T-05, ruling 5; BCK-002 A-32.
#![allow(missing_docs, reason = "a bench binary")]

use std::hint::black_box;
use std::process::ExitCode;

use rh_bench::alloc_count::{Counting, Snapshot, snapshot};

#[global_allocator]
static COUNTING: Counting = Counting;

/// One buffer: allocated, grown twice past its capacity with `reserve_exact`, shrunk once, freed.
/// Explicit calls, so the counts do not depend on `Vec`'s growth policy.
const KNOWN: Snapshot = Snapshot {
    allocations: 1,
    growths: 2,
    shrinks: 1,
    deallocations: 1,
};

fn self_test() -> Snapshot {
    let before = snapshot();
    let mut v: Vec<u64> = Vec::with_capacity(4);
    v.extend_from_slice(black_box(&[1; 4]));
    v.reserve_exact(4);
    v.extend_from_slice(black_box(&[2; 4]));
    v.reserve_exact(8);
    v.truncate(2);
    v.shrink_to_fit();
    drop(black_box(v));
    snapshot().since(before)
}

fn main() -> ExitCode {
    let got = self_test();
    if got != KNOWN {
        println!("counter self-test FAILED: expected {KNOWN:?}, counted {got:?}");
        return ExitCode::FAILURE;
    }
    println!("counter self-test: {got:?} as expected");
    ExitCode::SUCCESS
}
