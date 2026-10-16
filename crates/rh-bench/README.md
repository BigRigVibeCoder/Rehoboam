# rh-bench: benchmarks and the regression gate (SPR-009)

A tool-plane crate: it measures plane-A code from outside and is never a dependency of it.

| Target | What it measures | Run by | Gate |
|---|---|---|---|
| `benches/plane_hashes.rs` (bench 7) | `state_hash`, observation and narrative hashes, full and dirty-chunk, at 10k rows | `bin/perf_gate.py`, under callgrind (iai-callgrind) | V-PERF-04: more than 10% more instructions than `baselines/instructions_v1.txt` fails the PR |
| `benches/keyframe_write_read.rs` (bench 8) | keyframe write (with its read-back), read and load, at 10k rows | as above | as above |
| `benches/wall.rs` | benches 7 and 8 at 100k and 1M rows, wall clock (criterion) | `bin/bench_session.sh`, `.github/workflows/adjutant.yml` | trends; an alert above 10%, gating only on `adjutant` once T-04 has measured its spread |
| `benches/alloc_budget.rs` | heap allocations per steady-state tick (V-PERF-11), under the counting allocator | `cargo bench -p rh-bench --features count-alloc --bench alloc_budget` | V-PERF-11 (T-05) |

Bench 4 (`raster_8x512`) is a named, empty slot until P1a (SPR-009 ruling 1): a `slot` line in the baseline.

## What the instruction counts model (SPR-009 R-7)

The instruction-count targets run on a deterministic bump arena (`src/alloc_count.rs`, `Arena`) as their global allocator.
- **Why:** with glibc's malloc, the counts of one commit moved by up to 13% between two checkouts. The path strings the bench process holds change the heap the measured code allocates from, and with it buffer alignment and the malloc and memcpy paths taken.
- **How:** in the arena every block is 64-byte aligned and nothing is reused, so the counts depend on the code, not on where the repository is.
- **What this means:** **instruction counts measure computation, with allocation at a constant cost.** Allocations themselves are measured separately, by V-PERF-11's counting allocator (`Counting`, the `alloc_budget` target).
- **Containment:** the two allocators are separate bench targets, and one binary never holds both (a compile error). `bin/lint_arch.py` keeps both features, `count-alloc` and `arena-alloc`, and the only `#[global_allocator]` inside this crate's bench targets.

## Baselines

They are regenerated, never typed (SPR-009 ruling 3).
- **Commands:** `python -I bin/perf_gate.py record` writes `baselines/instructions_v1.txt`; `bin/wall_trend.py record` writes a machine's trend file.
- **Equality:** a changed baseline must equal CI's recomputation, toolchain lines exactly and each count within max(64 instructions, 1e-5) (R-8). The tool's path still moves a count by up to 48 instructions in 66 million.
- **Where it may change:** only in a PR titled `[perf-baseline]` whose body has a `perf-baseline-reason:` line.
- **Toolchain:** the toolchain lines (rustc, LLVM, host, valgrind, libc, iai-callgrind, the CPU's BMI2) must match, or the gate reports "toolchain changed: re-baseline required".
