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

/// A zero-filled buffer (`vec![0; n]`), which `Vec` takes from `alloc_zeroed`, then freed: an
/// uncounted `alloc_zeroed` would under-count engine code that zero-fills (VER-SPR-009-TEST).
const KNOWN_ZEROED: Snapshot = Snapshot {
    allocations: 1,
    growths: 0,
    shrinks: 0,
    deallocations: 1,
};

fn zeroed_test() -> Snapshot {
    let before = snapshot();
    let z: Vec<u64> = vec![0; black_box(64)];
    drop(black_box(z));
    snapshot().since(before)
}

fn main() -> ExitCode {
    let got = self_test();
    let zeroed = zeroed_test();
    if got != KNOWN || zeroed != KNOWN_ZEROED {
        println!(
            "counter self-test FAILED: expected {KNOWN:?} and {KNOWN_ZEROED:?} (zeroed), counted {got:?} and {zeroed:?}"
        );
        return ExitCode::FAILURE;
    }
    println!("counter self-test: {got:?} and {zeroed:?} (zeroed) as expected");
    ExitCode::SUCCESS
}
