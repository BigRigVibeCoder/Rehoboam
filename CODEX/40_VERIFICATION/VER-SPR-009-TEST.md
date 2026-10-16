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
