---
id: VER-SPR-009-TEST
title: "Determinism tester evidence for SPR-009"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing, determinism]
related: [SPR-009, VER-SPR-009, VER-SPR-006-TEST, VER-SPR-007-TEST, AGT-003-DET, GOV-006]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** The gate does what SPR-009 and the Architect's rulings say, and after the Developer's two fix commits every defect but one is closed. `bin/check_all.sh` is green on clean archives of all three heads I saw: `874a302` (194 of 194, 542 s), `8d6c51c` (194, 437 s) and `46f0f59` (194, 386 s). The instruction-count gate passes 10.00% and fails 10.01%; a real measured +4.55% and +7.46% slowdown pass and +10.92% and +12.92% fail; a 30% speed-up passes and never rewrites the baseline; runs in one checkout, under `taskset`, with ASLR off, and from checkout paths of 2, 81, 83 and 106 characters agree within 42 instructions (the band is 64). The R-8 band, the `[perf-baseline]` title and reason rules, R-7 (arena containment, exclusion, exhaustion) and R-9 behave as ruled. A hostile observer (clock reads, id-dependent sleeps, allocation, thread spawns, attach and detach mid-run) changes no monthly `state_hash` at 1, 2, 4 and 8 threads, and no clock API is reachable from plane A by name or through `unsafe`. **Four defects:** DEF-1 (medium) a panicking observer corrupted the engine silently, fixed in `8d6c51c` and re-verified; DEF-2 (medium-low) nothing pinned the content of an observer notice, fixed in `46f0f59` and re-verified; DEF-4 (low) the allocator self-test could not see `alloc_zeroed`, fixed in `46f0f59` and re-verified; **DEF-3 (low) `wall_trend.py check` passes a `nan` median, still open on `46f0f59`.** **Mutation:** of 35 mutants of the gate, the observer and the CI workflow, 18 turned the checks red at `874a302` and 17 survived; at `46f0f59` 27 are red and 8 survive (the six CI-workflow edits, one meta-level expectation in a control, one immaterial boundary in the allocator). **Twelve contract gaps**, none blocking; GAP-1 (the workflow does not re-run when a PR title is edited) is the one I would fix before merge. T-03 (vitals) and T-05's per-tick baseline are out of scope at these heads by the Architect's instruction; T-04's machines were not available.

> **Phase 2 BLUF (§7, head `c29787c`):** the merge of main, T-03 vitals and T-05 hold, and the phase-1 fixes are in, but **`c29787c` is red on GitHub** (DEF-5: one pytest test reads `RH_EVENT`, which the workflow sets; it passes on a developer's machine and fails on all three CI event shapes) and its PR tier needs about 17 min 40 s, **over the 15-minute criterion** (V-PERF-04). The Developer's `ded5f7f` (two parallel jobs, 11 min) turns CI green; I re-ran only its pytest. One more defect (DEF-6, low), six contract gaps, and 13 of 28 new mutants survive, most of them in the vitals timing logic and the allocation gate's branches.

Tested commit: `874a30271da9c22a12e14775a011379e9f02263f` on `sprint/SPR-009-benchmarks` (PR #34), five SPR-009 commits (`d549bf8`, `671dbe6`, `b35ac48`, `82bdddf`, `874a302`) on top of `4ac588d`. During the run the Developer pushed two commits: `8d6c51c` (the DEF-1 fix, four files in `rh-core` and `rh-snapshot`) and `46f0f59` (DEF-2 and DEF-4 tests and the survivors of my first mutation round, in `observer.rs`, `test_perf_gate.py` and `alloc_budget.rs`; no gate code changed). Each defect was re-verified on the commit that fixes it, `check_all.sh` was run whole on all three heads, and the mutants that had survived were re-run on `46f0f59` (§4). This file is committed on top of `46f0f59`; every other result is from `874a302`, which the two later commits change only in the observer and in tests. The probe plan was written from the contracts before the branch was opened (SPR-009 v1.5.0 with its six rulings, `docs/08` §9, `docs/09` V-PERF-04, V-PERF-11, V-DET-07, GOV-003 §2 and §5, GOV-006, `docs/13` §4.2–4.5); the Architect's later rulings R-7, R-8, R-9, 7 and 8 and deviation 3 were applied when the head arrived. All probing ran on `git archive` copies of the SHA, each with its own `CARGO_TARGET_DIR`; the repository tree was only read. Tools: valgrind 3.22.0, cargo-nextest 0.9.148, cargo-deny 0.20.2, iai-callgrind-runner 0.16.1, rustc 1.97.0 (pinned), glibc 2.39, a 4-core Xeon 2.1 GHz with BMI2, AVX2 and AVX-512. Probes are in the scratchpad (`probe_plan.md`, `mutate.py`, `mkperf.py`, `tester_obs.rs`, `tester_alloc.rs`); nothing but this file is committed. The Tester runs on a model different from the Developer's (GOV-005 §4.2); which one is not recorded here by instruction.

## 1. Verdicts

| Gate | Verdict | Basis |
|---|---|---|
| `bin/check_all.sh` | **PASS** | 194 of 194 on a clean archive of `874a302` with a fresh target directory (542 s, of which the instruction-count gate 131 s with a cold build), 194 of 194 on `8d6c51c` (437 s) and 194 of 194 on `46f0f59` (386 s); the new tests run inside the existing nextest and pytest checks, so the count of checks does not change. Every SPR-009 gate has a positive control in the run. |
| T-01 benches 7 and 8 (ruling 1, 2; deviation 3) | **PASS** | 10 cases: bench 7 has `state`, `observation` and `narrative` full and dirty-chunk, plus a dirty-column case; bench 8 has write, read and load. Bench 4 is a named `slot` line with no number. PR tier is 10k rows; the wall-clock bench defaults to 100k and 1M rows, as `docs/08` §9 asks. |
| T-02a instruction-count gate (V-PERF-04) | **PASS** | §2.1. |
| Baseline integrity (ruling 3, R-8) | **PASS** | §2.2. Gaps GAP-1, GAP-2. |
| R-9 BMI2 on the toolchain lines | **PASS** | A different `bmi2`, valgrind, rustc, llvm, libc, host or iai-callgrind line, or a missing line, gives exit 3 and "toolchain changed: re-baseline required", even with a 2× regression beside it. At `874a302` a mutant that ignored `bmi2` left every gate green (P5); `46f0f59` adds a test and the mutant is red. |
| R-7 arena | **PASS** | §2.4: unreachable from non-bench builds, exclusive with `count-alloc`, exhaustion aborts with a message naming the request. GAP-6, GAP-8. |
| Ruling 4 observer: invariance | **PASS** | §2.3: hostile observer, chopped attach and detach. |
| Ruling 4 observer: clock-free, cannot write state | **PASS; GAP-5** | §2.3. |
| Ruling 4 observer: panics | **PASS after DEF-1** (fixed in `8d6c51c`) | §3. |
| Ruling 4 observer: notice content | **PASS after DEF-2** (fixed in `46f0f59`) | §3. |
| T-02b wall-clock trends | **PASS with DEF-3** | §2.5. GAP-3, GAP-4, GAP-7. |
| T-05 (part): counting allocator, self-test, lint containment, allow-list | **PASS after DEF-4** (fixed in `46f0f59`) | §2.4. |
| T-04 procedure and scripts | **PASS (read and exercised); machines not run** | §2.6. No `[M]` ratio is written into `docs/08`. |
| Mutation of the gate, observer and CI | **8 of 35 survive** at `46f0f59` (17 at `874a302`) | §4. |
| T-03 vitals; T-05 per-tick baseline and the fixture-allocates gate | **Not testable at this head** | Wait for SPR-008, by the Architect's instruction. |

## 2. Results (condensed)

### 2.1 The gate (V-PERF-04, T-02a)

| Probe | Expected | Actual | Verdict |
|---|---|---|---|
| `state_full` baseline 2,311,755 scaled to +9.00, +9.99, +10.00, +10.01, +15, +5% | pass, pass, pass, **fail**, fail, pass | exactly so (`now × 100 > base × 110`); +10.00% passes | PASS |
| +12% on each of the 10 cases in turn | each fails | 10 of 10 fail | PASS |
| Real slowdown: extra hashed rows in bench 7 (`slowdown-control` feature with 4, 7, 9, 11% extra rows) | small passes, large fails | +4.55% **pass**, +7.46% **pass**, +10.92% **fail**, +12.92% **fail** (the committed control is about +15%) | PASS |
| All 10 cases 30% faster | pass; baseline untouched | pass, "faster (baseline kept)"; the planted baseline's checksum is unchanged (also a control in `check_all`) | PASS |
| A bench missing from the measured file, or from the baseline; a `slot` line missing | fail | fail, fail, fail | PASS |
| Toolchain: rustc, llvm, host, valgrind, libc, iai-callgrind, `bmi2` each changed; `bmi2` line missing; changed plus 2× counts | exit 3, "re-baseline required" | 9 of 9 | PASS |
| Two runs, one checkout | identical | identical (also a control) | PASS |
| One checkout under `taskset -c 1`; `setarch -R` (no ASLR); a 60 kB environment variable | identical | identical (iai-callgrind clears the benchmark's environment, so the last probe cannot move a count) | PASS |
| Same commit from checkouts with paths of 2, 81, 83 and 106 characters | agree within the band | the largest spread is 42 instructions (`keyframe_write_read/write`: 66,396,014 committed, 66,396,015, 66,396,047, 66,396,056); `load` spreads 9, `read` 27; bench 7 is identical everywhere | PASS (margin 42 of 64) |
| `bin/perf_gate.py record` in a fresh checkout, then `integrity` against the committed file as base | within the band, passes with tag and reason | passes; two instruction counts differ from the committed file by ≤ 33 | PASS |
| `iai-callgrind-runner` or `valgrind` absent from `PATH`; `cargo` absent | a named error | exit 2 naming the missing tool (`check_all` names valgrind and the runner up front) | PASS |
| An arena that runs out (`PR_ROWS` set to 40M) | abort with a message | "rh-bench arena exhausted: a request of 320000000 bytes does not fit the 1 GiB reservation…", then SIGABRT; the gate reports exit 2 | PASS |

### 2.2 Baseline integrity (ruling 3, R-8)

The base is a baseline that differs from the tool's output; `RH_PR_TITLE` and `RH_PR_BODY` are set as CI sets them.

| Case | Expected | Actual |
|---|---|---|
| Regenerated, `[perf-baseline]` in the title, reason line | pass | pass |
| Regenerated, no tag; tag only in the body; `[PERF-BASELINE]`; no reason; empty reason; reason as `> quoted` text | fail | fail ×6 (a reason key in capitals passes by design; text inside a code fence also passes) |
| One count typed +40, +64 (small bench) and +663 (66M bench), with tag and reason | pass, every difference printed | pass; "within band" printed |
| +65 and −65 (small bench), +664 and −664 (66M bench) | fail | fail; "OUTSIDE BAND … regenerated with `perf_gate.py record`, never typed" |
| Typed toolchain line (`valgrind`, `bmi2`); a bench removed or added; the slot removed | fail | fail |
| Comment-only edit, reordered lines, CRLF | pass if equal to the tool's output | pass (parsed content compared) |
| Title with `$(…)`, backticks and quotes | no execution | none (`/tmp/pwned` not created; `ci.yml` passes the title through `env`, never interpolated) |
| `pull_request` event with an empty title | error | exit 2 "the workflow must pass it" |
| Baseline unchanged from the base | the regression check governs | printed so |

### 2.3 The observer (ruling 4)

| Probe | Expected | Actual | Verdict |
|---|---|---|---|
| Hostile observer on the life world (64 rows, six years, three material inputs): reads `Instant`, sleeps 0–400 µs by sim time, worker and call count, allocates ~4 kB per notice, spawns a thread every 997 notices | monthly `state_hash` equal to the unobserved run | equal at 1, 2, 4 and 8 threads, two repeats each | PASS |
| Attach and detach every few instants, 3 threads, four years | equal | equal | PASS |
| `Observer::notify(&self, &Notice)`; a `Notice` holds copies and `&'static str`; the engine keeps the observer outside `EngineState` | no path to plane-A state | confirmed from the types; the observer test's protocol checks pass | PASS |
| Clock names planted in `rh-core` (workspace mode and `--scan`): `Instant::now`, `SystemTime`, `UNIX_EPOCH`, `use std::time as t`, `use std as s`, a type alias, a glob, grouped and aliased imports, `<std::time::Instant>::now`, spaced paths, a macro body, `chrono::Utc::now`, `quanta`, `tokio::time` | each refused | refused. `clock_gettime` (extern or `libc`), `_rdtsc` and `asm!("rdtsc")` are refused by the `unsafe` rule; `_rdtsc` does not compile under `forbid(unsafe_code)` on 1.97 either | PASS |
| A clock crate added to `rh-core` (`libc`) | refused | `plane-A-external` ×2 (not on the reviewed list; not pinned exactly) | PASS |
| Clocks without a clock name: `std::fs::metadata(..).modified()`, reading `/proc/uptime`, `Command::new("date")`, `env::var("EPOCHREALTIME")`, `chrono::Local` through an alias | refused | **not refused** | GAP-5 |

### 2.4 Allocation and containment (T-05 part, R-7)

| Probe | Result |
|---|---|
| My own bench target, `Counting` as the global allocator: `Box` = 1; `Vec::with_capacity(10)` = 1; zero-sized and empty vectors = 0; 1,000 pushes = 1 allocation + 8 growths; `String::from`, `format!` = 1; `vec![0; 4096]` = 1; `shrink_to_fit` = 1 shrink; 4 scoped threads × 100 boxes = 418 (worker threads are counted); a bare thread spawn = 3 | exact and equal in three processes |
| `cargo tree -i rh-bench`: nothing depends on it; `iai-callgrind`, `criterion`, `rh-bench` in the normal graph of each plane-A crate and of `rh-cli` | none |
| Release `rh-cli` built in the target directory that had just built the benches | no allocator symbols (`nm`), no arena message (`strings`) |
| `cargo check --workspace --all-features` | refused with "count-alloc and arena-alloc are for separate bench targets" (GAP-8) |
| `#[global_allocator]` outside `rh-bench`'s benches; a `[bin]` or `[test]` target requiring either feature; a crate on the allow-list drifting from the workspace lints; unsafe outside `alloc_count.rs` | all refused by the planted-violation controls in the run (mutants R5 and R6 turn them red); a `[lib]` or `[example]` target requiring the feature has no control (the code handles it) |
| `#[cfg_attr(not(test), global_allocator)] static G: System = System;` in `rh-core` | **not refused** by `lint_arch` or `lint_forbidden` (GAP-6; harmless alone because a counting allocator needs `unsafe`) |

### 2.5 Wall-clock trends (T-02b)

`bin/bench_session.sh adjutant 5` with `RH_WALL_ROWS=2000` on this sandbox: five criterion runs, the combine step, the conditions file, and the instruction counts, all produced; the worst run-to-run spread was 24.45% (a noisy, shared sandbox; the procedure works and the figure is what it is meant to show). `check` against a planted file: a median 11.00% above the trend is an ALERT that `--gate` fails; another CPU model gives exit 3 "machine changed". Refusals: unknown machine, fewer than 5 runs, a non-number, a laptop off AC or without the performance profile (all exit 2). **A trend or measured file with `bench x nan 1.00 5` passes as "ok (+nan%)"**, also with `--gate`: DEF-3. `inf` correctly alerts.

### 2.6 T-04 (read and exercised, no machines)

- `infra/adjutant/idle-shutdown.sh` with a fake `pgrep`, `who`, `systemctl` and `logger`: a marker 31 min old powers off; 29 min does not; a runner process or a login refreshes the marker; no marker creates it; `IDLE_MINUTES=60` is honoured. A non-numeric `IDLE_MINUTES` makes `find` fail and the VM never shuts down (GAP-9).
- `adjutant.yml` is `workflow_dispatch` only, so a pull request cannot reach the self-hosted runner; inputs go through `env`.
- `docs/08` records no `[M]` ratio for `adjutant` or the laptop; both still say "measured in SPR-009". Correct: nothing was measured.

## 3. Defects

Reproductions run from the root of a `git archive` copy of `874a302`.

**DEF-1 (medium; fixed in `8d6c51c`): a panicking observer corrupts the engine silently.**
The `JobStart`, `JobEnd` and `LevelMerge` notices run outside `guard()` (`crates/rh-core/src/engine.rs`, the `run` closure in `run_level`; `dispatch`'s comment says `guard` catches every panic so a worker never unwinds). Repro: in `crates/rh-snapshot/tests`, an `Observer` whose `notify` panics on the N-th (N = 2000) `JobStart`, `JobEnd` or `LevelMerge`; life world, 64 rows, 1 and 4 threads; `catch_unwind` around `advance_until`. Observed in all six cases: the panic unwinds out of `advance_until`; the level's owned columns were taken out of the tables and dropped; `crash_artifact()` is `None`; the engine is not poisoned; `state_hash(&e.snapshot())` returns a value without error (a world without those columns); after detaching the observer, the next `advance_until` fails with "X2.estate accesses estate.heirs, which the world does not have" and writes an artifact naming `X2.estate`, not the observer. A `PhaseStart` panic is harmless: state is intact and the hashes equal the reference. `observe.rs` documents that a panic "reaches the host", but SPR-006 DEF-1 and DEF-4 made "every call out of the engine is guarded" the rule, and GOV-004 law 2 says observability never changes history. A host that catches the panic and writes a keyframe persists a corrupt state. Recommendation: wrap every `notify` in the same `catch_unwind` and return a fatal `SimError::Observer` (poisoned engine, artifact naming the observer), or restore the columns before unwinding. Confidence: high on the repro, medium on severity (it needs a buggy host). **Re-verified on `8d6c51c`:** `SimError::Observer` is raised for every one of the nine notice kinds (`InstantStart` to `InstantEnd`), at 1, 2, 4 and 8 threads. In all 36 runs nothing unwinds out of `advance_until`; the world has the same columns and row counts as a fresh world; the engine refuses a further `advance_until`; the crash artifact names the system `observer`; and the error text and the `state_hash` after the failure are identical at every thread count. (Observation OBS-1: for a panic at `InstantStart` the artifact's tick is the previous instant, 59,040, while the error text and the other eight kinds say 60,480; a replay of that artifact would start one instant early. Low; Architect to decide whether the artifact tick should be the failing instant.)

**DEF-2 (medium-low; fixed in `46f0f59`): nothing pins the content of an observer notice.**
Ruling 4: notices carry sim time, phase and binding. `tests/observer.rs` checks only the open and close protocol and that the multiset of notices is the same at 1, 2 and 4 threads. Repro: apply together to `engine.rs` (a) in `Engine::notify` `at,` → `at: at.min(SimTime::EPOCH),`; (b) in `run_level`'s `notice` closure `at: t,` → `at: SimTime::from_minutes(t.minutes() - 1),`; (c) `phase: Some(phase),` → `phase: None,` and `worker: k % workers,` → `worker: (k % workers).saturating_sub(k % workers),`. `cargo nextest run --workspace --locked`: 295 of 295 pass (302 of 302 on `8d6c51c` with the same edits, so the fix did not touch it). Each of (a), (b) and (c) alone leaves the reduced SPR-009 chain of `check_all` green (mutants O3b, O3c, O4, R1b). The vitals line (T-03) will read these fields. Recommendation: in `tests/observer.rs` compare the recorded notices with an independently derived expectation (every `at` equals the engine's instant; phase notices carry `Some(phase)` in `Phase::ALL` order; job notices carry their level's `at` and phase; `worker == index % min(threads, jobs)`). Confidence: high. Sent to the Architect during the run. **Re-verified on `46f0f59`:** the new test `def_2_every_notice_carries_the_engine_s_instant_phase_and_worker` fails on my mutant (a) (engine-thread notices at the epoch) and the reduced chain is red on (b) job notices a minute early, (c) `phase: None` and `worker` always 0.

**DEF-3 (low; open on `46f0f59`): the wall-clock check passes a `nan`.**
`bin/wall_trend.py` `parse` accepts `float("nan")`; `now * 100 > base * 110` is false for NaN, so `check --gate` prints "ok (+nan%)" and exits 0. Repro: a trend and a measured file whose `bench wall/a/1000` line has median `nan` (either one). A NaN can come from a damaged `sample.json` (Python's `json` accepts `NaN`). Recommendation: refuse non-finite or non-positive medians and spreads in `parse`. Confidence: high. Re-run on `46f0f59`: still "ok (+nan%)", exit 0.

**DEF-4 (low; fixed in `46f0f59`): the counting allocator's self-test cannot see `alloc_zeroed`.**
`src/alloc_count.rs`: delete `ALLOCATIONS.fetch_add(1, …)` from `alloc_zeroed`. `cargo bench -p rh-bench --features count-alloc --bench alloc_budget` still prints "counter self-test … as expected", and `check_all` (reduced chain) stays green (mutant R3). With the mutant my probe counts `vec![0u8; 4096]` as 0 allocations (1 on the correct code). The engine's own tables use `vec![0; n]`, which compiles to `alloc_zeroed`, so a budget measured with this mutant would under-count and pass. Ruling 5 says "a broken counter can never report a passing budget". Recommendation: add a `vec![0u8; n]` to the known pattern (and `KNOWN.allocations` 2). Confidence: high. **Re-verified on `46f0f59`:** `alloc_budget` now also counts `vec![0; 64]` (1 allocation, 1 free); mutant R3 turns the self-test check red.

## 4. Mutation matrix

The mutants were applied one at a time to a clean archive, and each run used the SPR-009 chain of `check_all.sh` (the 29 checks that name SPR-009, V-PERF, the baseline, the observer and T-02b, plus the Python tests, which in `check_all` include `test_check_all_inventory.py`), generated from the script by `mkperf.py`. The unmutated chain is green (3 min 12 s with a cold build; about 45 s warm). Mutants that failed to compile or tripped clippy for an unrelated reason (R1, O3, O5, O5b) were rewritten and are not counted. For the one defect claim built on a survivor (DEF-2) I also ran the full workspace test suite. "Red" names the first failing check.

| # | Mutation | First red check |
|---|---|---|
| P1 | limit 10% → 14% | pytest |
| P2 | `>` → `>=` (exactly 10% fails) | pytest |
| P3 | comparison in the wrong direction | speed-up control |
| P4 | bench missing from either file ignored | pytest |
| P5 | `bmi2` ignored in the toolchain diff | **none: green** |
| P6 | the `[perf-baseline]` title check off | untagged control |
| P7 | the reason check off | no-reason control |
| P8 | band 64 → 640 | typed-baseline control |
| P9 | integrity never checks | typed-baseline control |
| P10 | a slot difference no longer fails | **none: green** |
| P11 | a speed-up rewrites the baseline | speed-up control |
| P12 | a `pull_request` run without `RH_PR_TITLE` returns "no context" | **none: green** |
| P13 | a *new* baseline file returns OK before any check | **none: green** |
| C1 | the 15% slowdown control deleted | pytest (inventory) |
| C2 | the slowdown control expects any failure, not `REGRESSION …state_full` | **none: green** |
| C3 | observer-invariance line deleted | pytest (inventory) |
| C4 | allocator self-test line deleted | pytest (inventory) |
| C5 | integrity line deleted | pytest (inventory) |
| C6 | repeatability line deleted | pytest (inventory) |
| R1b | notice `worker` always 0 | **none: green** |
| R2 | `PhaseEnd` never sent | observer invariance |
| R3 | `alloc_zeroed` not counted | **none: green** (DEF-4) |
| R4 | growth test `>` → `>=` | **none: green** (equal-size `realloc`; immaterial) |
| R5 | `#[global_allocator]` allowed in any file | lint_arch control |
| R6 | the unsafe allow-list admits any `rh-bench` file | lint_arch control |
| O1 | `InstantEnd` never sent | observer invariance |
| O3b | engine-thread notices carry `at = EPOCH` | **none: green** (DEF-2) |
| O3c | job notices one minute early | **none: green** (DEF-2) |
| O4 | job notices carry `phase = None` | **none: green** (DEF-2) |
| Y1 | `ci.yml`: valgrind install step removed | **none: green** |
| Y2 | `ci.yml`: `fetch-depth: 0` removed | **none: green** |
| Y3 | `ci.yml`: `RH_PR_TITLE` read from `pull_request.number` | **none: green** |
| Y4 | `ci.yml`: `continue-on-error: true` on the check step | **none: green** |
| Y5 | `ci.yml`: `iai-callgrind-runner` dropped from the install list | **none: green** |
| Y6 | `ci.yml`: `timeout-minutes` 20 → 5 | **none: green** |

18 red, 17 survived at `874a302` (P5, P10, P12, P13, C2, R1b, R3, R4, O3b, O3c, O4, Y1–Y6). The Python gate logic is pinned well; what survives is the observer's notice content, a few branches of the integrity check, the allocator's rarely used paths and the whole CI workflow. **Re-run on `46f0f59`** (the 11 survivors that touch code, in the same harness; O3b by hand because the notice builder moved in `8d6c51c`): P5, P10, P12, P13 are red in pytest (`test_a_bmi2_change_asks_for_a_re_baseline`, `test_bench_4_s_empty_slot_must_match`, `test_a_pull_request_run_without_a_title_is_an_error`, `test_a_new_baseline_is_checked_both_ways`); R3 is red in the allocator self-test; R1b, O3c and O4 are red in the observer test; O3b is red in `def_2_every_notice_carries_…`. C2 and R4 still survive, and so do Y1–Y6 (the workflow is unchanged). The 18 mutants that were red before were not re-run: the commits since change tests only, apart from the observer panic handling. That makes 27 red and 8 surviving mutants.

Y1–Y6 cannot be seen by a local run of `check_all`; the first three would turn CI red the first time they matter (a missing valgrind, no history for the merge base, a changed baseline with no tag found), which is a late signal.

## 5. Contract gaps

| # | Gap | Recommendation | Confidence |
|---|---|---|---|
| GAP-1 | `ci.yml` triggers on `pull_request` with GitHub's default types, which do not include `edited`. The tag and reason are read from the title and body, so editing them re-runs nothing: a PR that goes green with `[perf-baseline]` can have the tag removed before merge, and a red one needs a push, not an edit, to turn green. | `types: [opened, synchronize, reopened, edited]`. | High |
| GAP-2 | The tag and reason are enforced only when `RH_PR_TITLE` exists. On `push` or `workflow_dispatch` (or locally) a changed baseline passes, and the success line still says "in a [perf-baseline] pull request with a reason". Merges come from PRs, and the plan has no merge queue (`docs/13` §4.4), so the Architect's hand check is the backstop. | Print "tag and reason not checked" instead of the success wording; on `push` to `main`, compare with the parent commit. | Medium |
| GAP-3 | T-02b says wall-clock gates "only on `adjutant`, once T-04 shows its spread is well under 10%". `wall_trend.py check --gate` consults neither the machine nor the recorded spread; the `adjutant` workflow's `gate` input is a manual boolean. | Refuse `--gate` unless the trend's `machine` is `adjutant` and every case's recorded run-to-run spread is below a stated figure (the director's call; I suggest 5%). | Medium |
| GAP-4 | Ruling 3 covers instruction-count and allocation baselines only. `wall_trend.py record` writes the wall-clock trend file from any summary, and nothing checks that file in a PR. | Extend `integrity` (tag and reason) to `wall_*_v1.txt`; require the summary to be a `combine` of ≥ 5 runs. | Medium |
| GAP-5 | `lint_forbidden` is textual. A wall clock without a clock name passes: `fs::Metadata::modified()/accessed()/created()`, `/proc/uptime`, `Command::new("date")`, `env::var("EPOCHREALTIME")`, a `chrono::Local` alias (the crate itself is gated by `lint_arch`). Raw clocks (`clock_gettime`, `rdtsc`) are blocked, but only because they need `unsafe`. | Ban those three metadata methods and `std::process::Command` in plane-A crates; list the crates allowed to do file I/O. | Medium |
| GAP-6 | `#[cfg_attr(…, global_allocator)]` evades the `global-allocator` rule (a regex on `#[global_allocator]`). Alone it is harmless, because a counting `GlobalAlloc` needs `unsafe`, but `System` can be installed. | Match `global_allocator` anywhere inside `#[…]`. | Medium |
| GAP-7 | The wall-clock bench has 8 of the 10 instruction-count cases (no observation or narrative dirty-chunk) and its dirty-chunk case touches one column, not all four. T-04 says the benches of T-01 run on `adjutant`. | Add the two cases or say in the README that wall-clock covers a subset. | Low |
| GAP-8 | `cargo check --workspace --all-features` fails (`count-alloc` and `arena-alloc` are exclusive). Any tool that enables all features (`cargo hack`, `cargo doc --all-features`, rust-analyzer's all-features mode) breaks. | Document it in `crates/rh-bench/README.md`, or make `arena-alloc` win when both are set. | Low |
| GAP-9 | `idle-shutdown.sh`: any `who` output, even a forgotten SSH session, keeps the VM up for ever; a bad `IDLE_MINUTES` makes `find` fail inside `$(…)` and the VM never shuts down (fail-open on cost). | `set -e` is not enough: test the number, and say in RUN-002 that a login blocks shutdown. | Low |
| GAP-10 | R-8's margin is 42 of 64 instructions across the paths I tried (the Developer measured up to 48). The relative term lets a typed count 663 off pass on the 66M-instruction bench, as ruled. | Keep the figure in `crates/rh-bench/README.md`; revisit the band if a CI run ever exceeds 48. | Medium |
| GAP-11 | The toolchain lines record `bmi2` but not the other CPU features glibc and zstd select code paths by (AVX2, ERMS, FSRM), nor the installed runner's version (only the library's, from `Cargo.lock`). iai-callgrind clears the benchmark's environment, so `GLIBC_TUNABLES` could not be used to emulate another CPU; I could not test it. | Run `bin/perf_gate.py measure` on the laptop and on `adjutant` in the T-04 sessions and compare with the CI baseline before trusting "stable on shared runners and laptops". | Medium |
| GAP-12 | T-05's acceptance ("a fixture system that allocates in its tick fails the gate") has no gate to run at this head: the counting allocator is not attached to any engine measure, and the budget file does not exist. Out of scope until SPR-008. | Re-test when the per-tick bench lands; include a fixture that uses `vec![0; n]` (DEF-4 is fixed for the self-test, but the gate itself is not built). | High |

## 6. What was not run

- **T-03 vitals** (fields against GOV-006 §1–2, the serial fraction, a stalled consumer, peak RSS): not at this head.
- **T-05 per-tick budget on the smoke world**, and the fixture-allocates-per-tick positive control (GAP-12).
- **T-04 on the director's machines:** no `adjutant`, no laptop, no AMD run, no machine ratios, no idle shutdown on a real VM, no self-hosted runner. The session script ran only on this 4-core sandbox, with a 2,000-row smoke size.
- **A second CPU or a second glibc:** counts were compared across checkout paths, not across machines. BMI2 reporting was exercised by editing the baseline, not by running on a CPU without it (GAP-11). The hosted runner's own count was not compared with this sandbox's; the Architect reports CI green on the same head.
- **The full `check_all.sh` under each mutant.** The reduced chain judged them. I ran the whole workspace test suite on the DEF-2 mutants (at `874a302` and at `8d6c51c`) and the whole `check_all.sh` on the three unmutated heads only.
- **CI behaviour on GitHub:** `ci.yml` and `adjutant.yml` were read and parsed, not run (Y1–Y6 are therefore mutants that no local run can judge); the PR title and body of #34 were not read.
- **Windows and macOS.** Linux x86-64 only.
- **Nightly tiers** (`--run-ignored only` suites) were not run; they do not cover SPR-009.
- **A kill during a baseline write:** `record` writes the file in one `write_text`; I did not interrupt it.

## 7. Phase 2: the merge of main, T-03, T-05, the phase-1 fixes, V-PERF-04 (head `c29787c`)

Tested commit: `c29787cef3f93adbbfeb826ddf4a197b94472f61` (`feat(SPR-009): T-03 vitals, T-05 the smoke-world allocation budget, VER-SPR-009-TEST phase-1 fixes [perf-baseline]`), on `5d77e25` (main at `3bd2262`, SPR-008, merged in). Method: a `git archive` of the commit given a git history (its merge base with `origin/main` is `3bd2262`, as in CI), a cold target directory, this 4-core sandbox. While I worked the Developer pushed `ded5f7f` (the review of `c29787c`); I read its CI result and re-ran its pytest, nothing more (§7.7). This file is committed on top of `ded5f7f`.

### 7.1 Verdicts

| Item | Verdict | Basis |
|---|---|---|
| `bin/check_all.sh` on a clean archive | **PASS** | 265 of 265, 889 s (14 min 49 s) from a cold target directory. |
| **GitHub CI on `c29787c`** | **FAIL: DEF-5** | Run 38972047368, job 117212500585: `FAIL Python: unit tests (pytest)` at check 243 of 265, after 16 min 36 s. |
| 1. The merge of main | **PASS** | §7.2. |
| 2. T-03 vitals | **PASS with DEF-6, GAP-13, GAP-14** | §7.3. |
| 3. T-05 allocation budget | **PASS** | §7.4. 82 of 82 repeat runs identical; every allocating fixture fails; integrity covers the file. GAP-15. |
| 4. The phase-1 fixes | **PASS** | §7.5. DEF-3, GAP-2, GAP-6, GAP-9, OBS-1, C2 closed; residuals noted. |
| 5. V-PERF-04, per-PR CI ≤ 15 min | **FAIL on `c29787c`** (about 17 min 40 s); **plausibly met on `ded5f7f`** (11 min, warm cache) | §7.6. |
| 6. Keyframe refusal on a poisoned engine | **PASS** | 18 of 18 (9 notice kinds, 1 and 4 threads) on `8d6c51c`; 36 of 36 (1, 2, 4 and 8 threads) on the merged engine with the smoke world. |
| T-04 | **NOT TESTABLE** (the director's machines) | As ruled. |

### 7.2 The merge of main

- **`engine.rs`.** Against main (`3bd2262`) the diff is the observer (`Engine::observer`, `notify`, `observe`, `run_passes`, `JobStart`/`JobEnd`/`LevelMerge` notices) and three extractions: `binding_pass`, `DispatchStats::record` and `check_owned_columns`. I read each against main's inline code; the logic is the same. `StepCtx::stream` (`system.rs` adds only `SimError::Observer`), the parallel dispatch (`dispatch`, job `k` on worker `k % threads`) and `DispatchStats` are intact.
- **The merge itself** was resolved by hand in four files (`ci.yml`, `CLAUDE.md`, `engine.rs`, `test_check_all_inventory.py`). Against git's own three-way merge of `58debbd` and `3bd2262`, what differs is the removal of conflict markers and the union of both sides' lines; nothing of main's was dropped.
- **The smoke golden ladder.** `rh-cli run smoke --years 1000 --ladder yearly` at 1, 2, 4 and 8 threads, and with `--vitals` at 1, 4 and 8: all seven ladders are byte for byte (`cmp`) equal to `worlds/smoke/baselines/smoke_yearly_1000y.ladder`. The baseline file is identical to main at `3bd2262` and at the current `c5b197a`.
- **Observer invariance on the merged engine with the smoke world** (20,000 instants, 40 state hashes, 1,660 parallel levels; probe in `crates/rh-bench/tests`, not committed): the hostile observer (clock reads, sleeps that depend on sim time, worker and a counter, allocation, a spawned thread, atomics) at 1, 2, 4 and 8 threads equals the unobserved 1-thread reference, and so does the unobserved 4-thread run. Attach and detach every 7 instants at 4 threads: equal. The `Timing` observer with a second thread calling `vitals()` in a tight loop (4.0 M, 8.5 M and 9.5 M reads) at 1, 4 and 8 threads: equal. A consumer cannot hold `Timing`'s lock (`vitals()` clones and returns), so a stalled consumer cannot stall the engine either.
- **Panic matrix on the smoke world** at an instant that holds all nine notice kinds (minute 360), 9 kinds × 1, 2, 4, 8 threads: nothing unwinds; the engine is poisoned; every column is back in the tables; `write_keyframe` is refused; the artifact's system is `observer` and its tick is 360, the failing instant, for all nine kinds (OBS-1); the error text and the state hash are the same at every thread count; a further `advance_until` errors.
- **`ci.yml`:** the `edited` trigger, the PR-only `cancel-in-progress` expression, `fetch-depth` full history, the bisect step on `failure()` and the SPR-008 ladder uploads are intact, with SPR-009's valgrind, `iai-callgrind-runner@0.16.1`, `RH_PR_BODY` and perf-artifact steps added.

### 7.3 T-03 vitals

`rh-cli run smoke --years 1000 --threads T --vitals`, T = 1, 4, 8, against GOV-006 §1–2.

| Field | Result |
|---|---|
| tip, pace, ticks/s, plane hashes A, B, C (GOV-006 §1) | All present in one line. The state hash (`bef936ec026341a0`) equals the last ladder rung at 1, 4 and 8 threads; the observation and narrative hashes agree across thread counts. |
| Per-system time (GOV-006 §2) | Printed per system as a share and in ms. At 1 thread the seven systems sum to 11.9 s of a 68.5 s run. |
| Serial fraction | 90.7% at 1 thread, 47.8% at 4, 48.2% at 8 (§7.3, GAP-14). |
| The ladder with `--vitals` equals the one without | **Yes**, at 1, 4 and 8 threads; also equal to the baseline. |
| A slow or stalled consumer | Cannot change the run (§7.2). |
| Edge cases | `--years 0` prints no vitals line and exits 0; `--years 1` prints one. |

- **DEF-6 (low): the pace is truncated to whole years before the division.** `Vitals::line` computes `per_second(sim_minutes / MINUTES_PER_YEAR, wall)`. A 1-year run (77 ms, 12.9 sim-years/s) prints `pace 0.0`; 2 years (141 ms) prints `7.0` for a true 14.2; 3 years prints `8.5` for 12.9; 10 years `14.8`. At 1,000 years the error is 0.1%. Fix: divide minutes by wall first, then scale (`sim_minutes * 10 * NANOS / (MINUTES_PER_YEAR * wall)`).
- **GAP-14: what "serial fraction" measures.** It is the wall share not inside a level with two or more jobs (`jobs >= 2`, not `workers >= 2`). At 1 thread nothing runs in parallel, yet 9.3% of the time is booked as parallel; at 4 threads each parallel level also pays its thread spawn (the run is 2.3 times slower than at 1 thread: 6.3 against 14.5 sim-years/s; BCK-002 A-32), and that sits inside the "parallel" share. The figure depends on the thread count, so it is not the Amdahl `s` of `docs/08` §1.3 yet. Say what it measures before P1a builds its ≤ 20% gate on it.
- **GAP-13: the timing logic has no test beyond the format of the line.** `check_all` runs the vitals at 1 thread and greps for the line's shape; the one unit test uses invented numbers. Six of seven mutants of `Timing::notify` and `Vitals::line` survive (§7.8): parallel levels counted from one job, per-system time halved, the tip stuck at the first instant, `JobStart` always on worker 0 (invisible at 1 thread), instants not counted, the pace scaled wrongly. Only the inverted serial share is caught. Feed `Timing` a scripted notice sequence with a fake clock (an `Instant`-free injection point) and assert the numbers.
- **GAP-16 (info): GOV-006 §2 lists signals the vitals do not carry:** invariant counters, events emitted, peak RSS, stream size per channel, keyframes written. T-03's acceptance asks only for the vitals line and per-system timings, so this is not a defect; a later sprint owns them.

### 7.4 T-05 allocation budget

- **Repeatability.** `perf_gate.py alloc-measure` (4-thread measurement included) was run 82 times and every output is identical byte for byte to `crates/rh-bench/baselines/allocations_v1.txt`: 21 on all four cores, 12 each pinned to 1, 2 and 3 cores (`taskset`), 12 with six busy loops competing, 6 pinned to one core and competing, 4 with `RUST_BACKTRACE=full MALLOC_ARENA_MAX=1 RUST_MIN_STACK=8M`, 3 from another working directory. Thread spawns and scheduling do not leak into the count.
- **Positive controls** (the control wraps one system; all fail unless noted): `vec![0; 64]` after X1's step (the Architect's X1) **OVER BUDGET**, +8,761 allocations at 1 thread, +2 on the most in a tick; `Vec::with_capacity(8)`; `Box::new(7_u64)`; `String::from(..)`; a `Vec` pushed 100 times (growths 856 → 44,661); a thread spawned inside the step (+43,805); `with_capacity(100)` then `shrink_to_fit`. All fail at 1 and 4 threads. An allocation in `fire` (the timer path) of X3, the only smoke system with timers, fails with +42; in any other system that path never runs in the window, so it passes, which is right.
- **Integrity.** With the budget file as `--exact`: a regenerated file (= the tool's output) that differs from the base passes only in a PR titled `[perf-baseline]` with a `perf-baseline-reason:` line (untagged and tagged-without-reason both fail); a hand-typed growth, window line or shrink count fails "does not equal the tool's output"; the instruction-count path refuses the file's format.
- **GAP-15: four branches of `alloc_compare` have no test** (mutants A2–A5 survive): `growths` and `max_per_tick` not gated, a changed window line not noticed, a changed tick count not noticed. The controls only raise `allocations`. Add one case per field to `python/tests`.
- Shrinks (730 at 4 threads, 0 at 1) are reported and not gated: a shrink is a realloc to a smaller size and the regrowth it needs is counted as a growth. Informational.

### 7.5 The phase-1 fixes

| Fix | Result |
|---|---|
| DEF-3 | `wall_trend.py check --gate` refuses (exit 2, "not a rehoboam/rh-bench/wall/v1 line") `nan`, `NaN`, `inf`, `-inf`, `1e999`, a median of 0 or below, a negative spread, and 0 samples, in the measured file and in the trend file. A spread of 0 is accepted. The check_all control plants a NaN. |
| GAP-2, in scratch repositories, push range | Tag at the end of the subject (`… [perf-baseline]`, `… (#29) [perf-baseline]`, `… [perf-baseline] [C1]`) plus a `Reason:` line **passes**; a reason without the tag, a tag without the reason, an empty `Reason:` and an upper-case `[PERF-BASELINE]` **fail**; a typed edit 10,000,000 off fails whatever the tag; an unchanged baseline passes. A push with no `RH_PUSH_BEFORE`/`AFTER` is an error (exit 2). A run with no context prints "equals the tool's output; tag and reason not checked here", never the success wording. A `pull_request` run without a title is an error. |
| GAP-6 | `cfg_attr(feature = "x", global_allocator)`, nested parentheses, a multi-line attribute, `# [ global_allocator ]`, a path-qualified `#[::core::prelude::v1::global_allocator]` and a macro-generated one are all flagged. **Residual:** a `]` inside a string or comment earlier in the same attribute (`cfg_attr(feature = "a]b", global_allocator)`) is not. Low: the attribute must be hand-crafted to evade. |
| GAP-9 | Invalid `IDLE_MINUTES` (`abc`, `0`, `00`, `-5`, `1.5`, `1e2`, `5 `) is logged and 30 is used; empty or unset gives 30; a runner job or any `who` output keeps the VM up. **Residual:** `99999999999999999999` is a "positive integer" and `find -mmin` never matches it, so the VM never powers off; `007` is rejected with the message "not a positive integer". Low. |
| OBS-1 | Artifact tick = the failing instant at all 9 kinds, 1, 2, 4, 8 threads (§7.2). |
| C2 | The 15% slowdown control requires `REGRESSION plane_hashes/state_full/rows_10k` in its output and passes in the run. Loosening the expectation to "any failure" is not caught by anything: a meta-level survivor. |

### 7.6 V-PERF-04: does the per-PR tier fit 15 minutes?

| Measurement | Time |
|---|---|
| `check_all.sh` on a clean archive, cold target, 4 cores, 265 checks | **889 s = 14 min 49 s** |
| of which the checks that name SPR-009 (gate, repeatability, integrity controls, wall smoke, allocation, vitals, observer) | about 196 s (22%) |
| the slowest: golden N threads 136 s; workspace nextest 135 s; V-DET-10 release build 73 s; V-PERF-04 gate 71 s; vitals gate 68 s; golden 1 thread 55 s | |
| **GitHub, `c29787c`** (run 38972047368, one job): job 21:50:38 → 22:08:00 | **17 min 22 s**, stopped at check 243 of 265 |
| of which the `check_all` step | 16 min 36 s; set-up before it 43 s |
| the 22 checks it did not reach take 18 s locally | a passing run needs about **17 min 40 s** |
| GitHub, earlier PR runs without SPR-009's steps (run level, queue included): #211 15 min 09 s, #213 14 min 34 s; main pushes #212 7 min 34 s, #214 10 min 22 s | |
| **GitHub, `ded5f7f`** (run 38977023421), two parallel jobs: golden part 10 min 54 s, core part 9 min 55 s | **11 min 02 s** to completion, green |

**The tier does not fit on `c29787c`:** 17 min 40 s against 15. The pre-SPR-009 PR runs were already at 14.5 to 15 min; SPR-009 adds about 3 min. I cannot confirm the 13 min 49 s cold-cache figure for main; my own cold run on 4 cores is 14 min 49 s, and the hosted job (with its restored cache) was slower. Two further points: pytest, a 9-second check, runs at position 243, so a Python failure costs the whole 16-minute run before it shows (move cheap checks first); and `ded5f7f`'s split into two parallel jobs does reach 11 minutes, but its run restored a rust-cache in 1 s, so I have no cold-cache figure for it.

### 7.7 Defects of this phase

**DEF-5 (high, blocks the merge; fixed in `ded5f7f`, pytest re-verified): CI is red on `c29787c`.** `python/tests/test_perf_gate.py::test_a_pull_request_run_without_a_title_is_an_error` sets `GITHUB_EVENT_NAME=push` and expects "without RH_PUSH_BEFORE", but `change_context()` reads `RH_EVENT` first and `ci.yml` exports `RH_EVENT` for the whole job. Repro from the root of the archive, with the venv: `RH_EVENT=pull_request RH_PR_TITLE="x [perf-baseline]" python -m pytest -c python/pyproject.toml --rootdir=. -q` → 1 failed, 479 passed ("Actual message: a pull_request run without RH_PR_TITLE"); with `RH_EVENT=push` or `RH_EVENT=local` (push, `workflow_dispatch`) the same test fails with "DID NOT RAISE GateError". So the Python check fails on all three event shapes and passes only with `RH_EVENT` unset, which is how a developer runs `check_all`. The commit message says "check_all: PASS, 265 checks"; true locally, false in CI. It is the mistake SPR-008 fixed in `750ce32` for `test_check_baseline`. Confidence: high. **On `ded5f7f`:** `python/tests/conftest.py` clears every `RH_*` and `GITHUB_*` variable before each test; the 487 tests pass with no variables, as a pull request, as a push, as `workflow_dispatch`, and with every variable set at once.

**DEF-6 (low, open): the vitals pace is truncated to whole years** (§7.3).

### 7.8 Mutation matrix of this phase

Method as in §4, on `c29787c`, with the release build of `rh-cli` added to the chain (41 checks) and, for the `rh-golden` mutants, the crate's unit tests. The unmutated chain is green. The eight survivors of §4 were re-run unchanged in effect (C2, R4, Y1–Y6): all eight still survive, and two new workflow mutants (Y7 `RH_PR_BODY` not passed, Y8 `edited` removed) survive with them. Twenty-eight mutants of the code new in this commit:

| # | Mutation | Result |
|---|---|---|
| V1 | a level counts as parallel from one job (`jobs >= 1`) | **green** |
| V2 | per-system time halved | **green** |
| V3 | the tip stuck at the first instant | **green** |
| V4 | `JobStart` always recorded on worker 0 | **green** (the gate runs 1 thread) |
| V5 | serial share inverted | red: `rh-golden` unit test |
| V6 | instants not counted | **green** |
| V7 | pace scaled by half a year | **green** |
| A1 | an equal count counts as over budget (`>` → `>=`) | red: allocation budget |
| A2 | `growths` not gated | **green** |
| A3 | `max_per_tick` not gated | **green** |
| A4 | a changed window line not noticed | **green** |
| A5 | a changed tick count not noticed | **green** |
| A6 | exact integrity accepts any file | red: integrity control |
| A7 | the 4-thread window dropped | red: allocation budget |
| A8 | the `vec![0; n]` control deleted | red: pytest (inventory) |
| A9 | the allocation repeatability check deleted | red: pytest (inventory) |
| G1 | push: a tag **or** a reason suffices | red: pytest |
| G2 | push: the tag anywhere in the subject | red: pytest |
| G3 | the all-zero `before` (no base) push no longer skipped | **green** |
| G4 | push: tag and reason unchecked | red: push control |
| G5 | a local run prints the success wording | red: local control |
| D1 | wall: the finite check removed | red: NaN control |
| D2 | wall: a median of 0 allowed | red: pytest |
| D3 | wall: a negative spread allowed | red: pytest |
| D4 | wall: 0 samples allowed | **green** |
| L1 | the `global_allocator` regex reverted to the old one | red: `cfg_attr` control |
| K1 | OBS-1: the instant not recorded before `InstantStart` | red: observer test |
| I1 | idle shutdown: no `IDLE_MINUTES` validation | red: pytest |

15 red, 13 green. The survivors are the vitals timing logic (GAP-13), four branches of the allocation comparison (GAP-15), the no-base push, and 0 samples. Together with the 10 older and workflow survivors, 23 mutants of 38 run in this phase survive.

### 7.9 New contract gaps

| # | Gap | Recommendation | Confidence |
|---|---|---|---|
| GAP-13 | The timing logic of `Timing` has no test; six of seven mutants survive. | A fake-clock test of scripted notice sequences, at one and several workers. | High |
| GAP-14 | "Serial fraction" is thread-count dependent and includes spawn cost. | Name it in the line and in `docs/08` §9; define the P1a gate on single-thread work shares. | Medium |
| GAP-15 | `alloc_compare`'s growth, max-per-tick, window and tick branches are untested. | One unit test per field. | High |
| GAP-16 | Signals of GOV-006 §2 (counters, events, RSS, stream size, keyframes) are not in the vitals. | Backlog item for the sprint that owns observability. | Medium |
| GAP-17 | The push rule is satisfied by one tagged commit anywhere in `before..after`, and `[perf-baseline]` must end the subject, so GitHub's default squash subject `title (#N)` fails the push to main (the repo's own merges write `(#N) [tag]`). | Say so in `worlds/smoke/README.md` and RUN-001, beside the `[C3]` text; or accept the tag anywhere as the PR check does. | Medium |
| GAP-18 | `check_all` runs the cheap Python checks last (pytest at 243 of 265). | Run lint and pytest before the builds. | High |

### 7.10 What was not run in this phase

- Anything of `ded5f7f` except its pytest and the CI result of run 38977023421: its `--part` split, its re-recorded allocation baseline with toolchain lines, the typo control, the push-concurrency change and the unknown-event refusal are unverified by me.
- A cold-cache hosted run: the only hosted timings are those above.
- The nightly golden tier, the cross-machine procedure, peak RSS, a second CPU or glibc, and T-04.
- The full `check_all` under each mutant (a 41-check chain judged them).
