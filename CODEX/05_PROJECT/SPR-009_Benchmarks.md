---
id: SPR-009
title: "Benchmark harness and regression gate"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001, RUN-001, GOV-008]
created: 2026-10-16
updated: 2026-10-16
version: 1.7.0
---

> **BLUF:** Measure speed from the start: criterion benches, a 10% regression gate on instruction counts (stable on shared runners and laptops), wall-clock trends on the on-demand VM, serial-fraction instrumentation and the vitals line. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-009 — Benchmark harness and regression gate

- **Roadmap item:** `docs/11` P0.7 · **Phase:** P0 (gate G0)
- **Depends on:** SPR-007 (benches 7 and 8 measure its plane hashes and keyframes) and SPR-008 (T-03 and T-05 run on its `smoke` world). T-04 also needs `adjutant` and the director's laptop (RUN-001, RUN-002). Architect, 2026-10-16: it may start once SPR-007 merges, alongside SPR-008, and merges after it · **Plan approved by the director 2026-10-16** (P0 batch)
- **Binding contracts:** `docs/08` §9; GOV-006
- **Requirements covered:** PERF-4

## Architect rulings, 2026-10-16 (binding; written before the sprint starts)
1. **Bench 4 is out of this sprint** (director's ruling at G0, 2026-10-16). Bench 4 (`raster_8x512`, stencil fields) measures field code that does not exist until the first field system (M1 or M3, P1a), so its baseline moves to that sprint, and the G0 criterion reads "benches 7 and 8" (`docs/11` v1.4). Build the harness and benches 7 and 8, and leave bench 4 as a named, empty slot.
2. **The instruction-count gate (T-02a).**
   - `iai-callgrind`, exactly pinned, as a dev-dependency of the bench targets only. It never enters a plane-A crate's normal dependency graph, so `PLANE_A_EXTERNAL_DEPS` does not apply.
   - valgrind is installed in CI from the runner's package manager.
   - Every baseline file records the valgrind and toolchain versions. When either changes, the gate reports "toolchain changed: re-baseline required" instead of a false pass or fail.
   - Benches run at sizes that fit the 15-minute PR budget (bench 7: for example 10k rows, full and dirty-chunk; bench 8: for example 10k rows, write and read). The 100k and 1M sizes of `08` §9 are wall-clock trends (T-02b, T-04).
3. **Baselines are regenerated, never typed** (as SPR-008 ruling 6).
   - A committed instruction-count or allocation baseline must equal the tool's output.
   - It may change only in a PR whose title carries `[perf-baseline]` and whose body gives the reason.
   - A speed-up never re-baselines automatically.
   - Controls both ways.
4. **Timing never happens inside plane A** (T-03). `Instant` and `SystemTime` are banned in plane-A crates (GOV-003 §2).
   - `rh-core` exposes an observer interface that receives phase and level boundary notifications, carrying sim time, phase and binding but no clock.
   - The host (`rh-cli`, tool plane) reads the clock in its observer and computes per-system times, the serial fraction and the vitals line (GOV-006 §1).
   - The observer cannot touch plane-A state (the unsynced guard applies). V-DET-07 style: a run with the observer attached hashes equal to one without.
5. **The allocation budget is measured first (T-05).**
   - The counting allocator is enabled only in bench builds.
   - The P0 engine allocates per tick today (per-level buffers, scoped threads), so a budget of 0 would fail on day one. Record the measured per-tick counts on the smoke world as the baseline budget, with the allocation sites listed, and fail on any increase.
   - Driving the count to 0 (pooled buffers, a persistent thread pool) is BCK-002 A-32, due before benches 1 and 2 (P1a).
6. **Order and machines.**
   - This sprint starts once SPR-007 merges and merges after SPR-008, whose `rh-cli run smoke` it uses for T-03 and T-05.
   - T-04 (the `adjutant` and laptop baselines) needs the director's machines (RUN-001, RUN-002). Build the procedure and scripts. The sprint closes with T-04 marked "blocked: director's machines" if they are not available, and the Architect re-opens it as a follow-up.

## Architect rulings during the sprint, 2026-10-16 (binding)
7. **criterion approved** for the wall-clock trends (T-02b), not a std-only runner (70%).
   - Pinned `=0.8.2`, `default-features = false`, features `cargo_bench_support` only (no rayon, plotters or HTML reports). It is a dev-dependency of `rh-bench` and used by the `wall` bench target only.
   - It adds 28 packages to `Cargo.lock`, among them clap, ciborium, regex, itertools, tinytemplate, walkdir, and Windows-only shims that never build on Linux. `cargo deny` stays green, and the licence ledger lists the tree.
   - Reporting (RUN-002 §5): `bin/wall_trend.py` takes the median and spread of criterion's own samples (at least 5), not its bootstrap summary, records the run conditions, and alerts when a median is more than 10% above the committed trend file.
8. **Our own counting allocator approved** (75%). `rh-bench` is on GOV-003 §2's reviewed unsafe allow-list for one module, `src/alloc_count.rs`.
   - The allow-list is `UNSAFE_ALLOW_LIST` in `bin/lint_arch.py`.
   - The crate's own `[lints]` equal the workspace's but for `unsafe_code = "deny"`.
   - The module is compiled only behind `count-alloc`, which only the `alloc_budget` bench target requires.
   - `lint_arch` gates this (rules unsafe-allow, global-allocator and count-alloc), with planted violations.
9. **R-7, the deterministic arena: approved** (85%).
   - **Why:** instruction counts depended on the checkout's path, by up to 13% between two directories (measured), because the bench process's path strings change glibc malloc's state before the measured code.
   - **What:** the instruction-count targets run on a bump arena. Every block is 64-byte aligned in one 1 GiB reservation, and nothing is reused. A 10k-row bench takes 16–32 MiB.
   - **Where it lives:** in the allow-listed `src/alloc_count.rs`, behind `arena-alloc`. It never shares a binary with `count-alloc` (a compile error), and lint_arch covers both features, with controls both ways.
   - **On exhaustion:** it aborts with a message naming the request and the reservation.
   - **Modelling:** instruction counts measure computation, with allocation at a constant cost; V-PERF-11 counts allocations (`crates/rh-bench/README.md`).
10. **R-8, ruling 3 amended: a determinism band** (75%).
    - **Band:** a committed instruction-count baseline equals CI's recomputation within max(64 instructions, 1e-5 relative) per count, with every difference printed.
    - **Exact as before:** toolchain lines, allocation counts (V-PERF-11), the `[perf-baseline]` title and reason, and "a speed-up never re-baselines".
    - **Controls:** a count typed 65 off fails; one 40 off passes and is printed.
11. **R-9, BMI2 on the toolchain lines** (70%). zstd chooses its BMI2 path at run time, so a host without BMI2 reports "toolchain changed: re-baseline required".

## Deviations (Developer)
- **Instruction counts run on a deterministic arena, not on glibc's malloc** (R-7). Instruction counts measure computation, with allocation at a constant cost; V-PERF-11 counts allocations.
- **Baseline integrity is equality within a band, not byte for byte** (R-8). The executable's path still moves a count by up to 48 instructions in 66 million across checkouts.

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C1 |
| Planes touched (ADR-0024) | none (tooling) |
| Owned columns written | none |
| RNG purposes | none |
| Golden-hash effect | none |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-PERF-04 (a deliberate 15% slowdown, measured in instructions, fails CI; a speed-up never re-baselines automatically); V-PERF-11 (allocation budget per tick) |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: criterion benches
- **Deliverable:** Benches 7 and 8 of `08` §9 against the smoke world; bench 4 as a named, empty slot (ruling 1)
- **Acceptance:** Baselines recorded
- **Status:** [ ] not started

### T-02: Regression gate
- **Deliverable:** Two gates. (a) **PR gate on instruction counts** (`iai-callgrind`, valgrind as an outside tool): more than 10% more instructions fails; deterministic to well under 1%, so it does not flake on a hosted runner or a laptop. (b) **Wall-clock benches** (criterion) run in each `adjutant` session (on demand, RUN-002 §4), are tracked as trends, and alert above 10%; they gate only on `adjutant`, once T-04 shows its spread is well under 10%. No automatic re-baseline
- **Acceptance:** V-PERF-04 positive control on gate (a); a run of gate (a) twice on the same commit gives identical counts
- **Status:** [ ] not started

### T-03: Serial fraction and vitals
- **Deliverable:** Per-system timings; the vitals line of GOV-006 §1
- **Acceptance:** Vitals printed by `rh-cli run smoke`
- **Status:** [ ] not started

### T-04: Machine ratios and wall-clock baselines
- **Deliverable:** The `adjutant` runner and its idle shutdown (RUN-002 §3). The benches of T-01 run on `adjutant` in one VM session (RUN-002 §4–5) and on the laptop (RUN-001 §6). `adjutant` becomes the wall-clock baseline machine; its run-to-run spread decides whether a wall-clock gate is possible there. Both machines' ratios to the `08` reference desktop are recorded in `08` as [M]
- **Acceptance:** Ratios, spreads and run conditions (machine type, CPU model, power profile, temperature) are in `VER-SPR-009`
- **Status:** [ ] not started

### T-05: Allocation budget per tick (RT-2026-10-16)
- **Deliverable:** A counting global allocator, enabled only in bench builds, that reports heap allocations per steady-state tick of the `smoke` world; buffer growth on capacity doubling counted separately; the budget recorded per bench (default 0 for P0 machinery) and gated like instruction counts
- **Acceptance:** V-PERF-11 positive control: a fixture system that allocates in its tick fails the gate; two runs on the same commit report identical counts
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-009` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-009` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
