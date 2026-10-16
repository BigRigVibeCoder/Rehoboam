//! `rh-bench`: the benchmark harness of SPR-009: the bench list, the fixtures benches 7 and 8 run
//! on.
//!
//! Tool plane. It measures plane-A code from outside and is never a dependency of a plane-A
//! crate. The instruction-count benches are in `benches/` (iai-callgrind, valgrind as an outside
//! tool); `bin/perf_gate.py` runs them, and compares their counts with the committed baseline
//! `baselines/instructions_v1.txt` (V-PERF-04).
//!
//! The benches of `docs/08` §9 that P0 owns: 7 (`benches/plane_hashes.rs`) and 8
//! (`benches/keyframe_write_read.rs`) are built and gated; 4 (`raster_8x512`) is a named, empty
//! slot by the director's ruling at G0 (SPR-009 ruling 1), recorded in the baseline as a `slot`
//! line (`bin/perf_gate.py`, `SLOTS`): it measures stencil fields, which do not exist before the
//! first field system (P1a).
//!
//! REF: `docs/08_performance_and_capacity.md` §9; `docs/09` V-PERF-04, V-PERF-11;
//! `CODEX/05_PROJECT/SPR-009_Benchmarks.md` T-01, T-02, rulings 1 and 2.
#![forbid(unsafe_code)]

pub mod fixture;
