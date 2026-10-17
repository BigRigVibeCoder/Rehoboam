---
id: VER-SPR-009
title: "Audit of SPR-009"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-009, VER-SPR-009-TEST, VER-G0, BCK-002, SPR-008]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF: PASS.** SPR-009 gives the engine its speed evidence:
> - benches 7 and 8 (plane hashes, keyframes) at 10k rows, with bench 4 a named, empty slot;
> - a regression gate on instruction counts, run on a deterministic arena so a count does not depend on the checkout's path, with every committed count checked against CI's recomputation within a band;
> - wall-clock trends with criterion and `bin/wall_trend.py`, gating only on `adjutant` after T-04;
> - a clock-free observer interface in `rh-core`, guarded so an observer panic poisons the engine instead of unwinding (DEF-1), and the vitals line computed by the host;
> - a per-tick allocation budget on the smoke world, exact and repeatable at 1 and 4 threads (82 of 82 runs byte-identical).
>
> Three Tester phases and one Architect review found DEF-1 to DEF-6, a CI-red test that read CI's own variables, a PR tier of about 17 min 40 s on a slow runner against V-PERF-04's 15, a concurrency rule inherited from SPR-008 that could drop a push's range (an escape against SPR-008), and a CI matrix that could lose the golden part silently (GAP-19). All are fixed. The PR tier now runs as two parallel jobs: 11 minutes on cold caches, 6–7 minutes warm. T-04 is blocked on the director's machines and re-opens as a follow-up (ruling 6). Change class C2, not the C1 the plan said (§3). Six items go to the backlog (A-32 with the allocation sites, A-41 to A-45). The Architect merges (GOV-005 §6).

# VER-SPR-009 — Audit of SPR-009

PR #34 merges with a merge commit: the branch carries a merge of `main` (`5d77e25`), as PR #28 did. The sprint's SHAs stay valid on `main`.

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh` | Phase 1: PASS (194 checks) on clean archives of `874a302`, `8d6c51c` and `46f0f59` (Tester). `c29787c`: PASS (265) locally, FAIL on CI (pytest read `RH_EVENT`). `ded5f7f`: PASS (271, both parts) locally, 8 min 57 s. `69ce702` (final): PASS (272, both parts) locally | Tester phases 2 and 3: PASS on clean archives of `c29787c`, `ded5f7f` (core 218, golden 53) and `a4b78cb`; the final delta Architect-verified (499 tests; the GAP-19 and GAP-21 controls) |
| CI on GitHub-hosted runners | `c29787c`: FAIL (run 38972047368), 17 min 22 s, stopped at check 243 of 265; a full run would take about 17 min 40 s. `ded5f7f`: **PASS** (run 38977023421): two parallel jobs on cold per-part caches, `core` 9 min 55 s and `golden` 10 min 54 s; the run's wall time is about 11 minutes, inside the 12-minute acceptance and V-PERF-04's 15. `1aec52d` (warm caches, run 38980955766): `core` 5 min 4 s, `golden` 6 min 28 s | PR #34 |
| Engine speed | No regression. 300 smoke years, `main` (`c5b197a`) vs `c29787c`, same machine, two rounds: 14.0/13.9 s vs 14.0/13.6 s at 1 thread; 42.5/40.2 s vs 41.7/41.0 s at 4 threads; ladders equal. CI's slower times on `c29787c` were the runner (GitHub-hosted runners vary about 2×) and test load | Architect |
| Change class | C2 (§3) | — |

## 2. Tester verdicts
| Phase | Head | Result |
|---|---|---|
| 1 | `874a302`, re-verified on `8d6c51c` and `46f0f59` (report `58debbd`) | PASS on T-01, T-02a (the 10% edge, real slowdowns, path independence within 42 instructions of the 64 band), R-7, R-8, R-9, the clock-free observer and observer invariance (a hostile observer at 1/2/4/8 threads). DEF-1 to DEF-4 found; DEF-1, DEF-2 and DEF-4 fixed and re-verified in phase 1; DEF-3 open at `46f0f59`. 35 mutants: 18 red / 17 survived at `874a302`, 27 red / 8 survived at `46f0f59` (Y1–Y6 CI workflow edits, C2, R4). 12 contract gaps. T-03 and the T-05 smoke parts not testable before SPR-008 merged |
| 2 | `c29787c` (report `1aec52d`) | **PASS** on the merge of main (smoke ladders at 1/2/4/8 threads, and with `--vitals` at 1/4/8, all byte-identical to the unchanged baseline; a hostile observer on smoke at 1/2/4/8 threads, attach and detach, and a thread hammering `Timing::vitals()` change nothing; the panic matrix, 9 kinds × 1/2/4/8 threads, poisons the engine with columns intact, refuses the keyframe in 36 of 36, and names the observer and the failing instant, with the same text and hash at every thread count). T-03 PASS with DEF-6. T-05 PASS: 82 of 82 runs byte-identical to the budget (pinned to 1, 2 and 3 cores, under load, other cwd and env); `vec![0;64]`, `Vec::with_capacity`, `Box`, `String`, push growth, a thread spawned in a step, `shrink_to_fit` and an allocation in a timer's `fire` all fail OVER BUDGET. Phase-1 fixes PASS. V-PERF-04 FAIL on `c29787c` (about 17 min 40 s), fixed in `ded5f7f`. DEF-5 (the CI-environment test; the Architect's review 1) fixed in `ded5f7f` and re-verified: 487 tests pass under every event shape. 28 mutants of the new code: 15 red, 13 green (GAP-13, GAP-15, G3, D4); the 10 older survivors (C2, R4, Y1–Y8) still survive |
| 3 | `ded5f7f` and `a4b78cb` (report `ebbd7f4`) | **PASS** on both, both parts side by side as CI runs them: `ded5f7f` core 218 checks in 511 s and golden 53 in 568 s; `a4b78cb` 476 s and 536 s. The split: 218 + 53 = 271 checks against 265 at `c29787c`, zero overlap, every earlier gate present (renamed where merged), 6 new; each part's report says which part it ran and has a SKIP row for the other; an unknown `--part` exits 2; the inventory tests catch an unrun by-name entry, a stray `run` line and a bogus part name. Fixes verified: DEF-5 (487 tests under every event shape), DEF-6, GAP-13, GAP-15, GAP-18 (pytest is check 20 of 218), the PR tag-group rule, the unknown-event refusal, the allocation budget's toolchain lines (exit 3), the observer text without the worker, lint_arch under a `target/` ancestor. 25 mutants on `a4b78cb`: 19 red by a test, 1 by clippy, 5 green (V10b, V11, T4, C2, R4). New: GAP-19 (medium), GAP-20, GAP-21, GAP-22 |

## 3. Rulings (Architect)
| # | Question | Ruling |
|---|---|---|
| 1–6 | Pre-sprint: bench 4, the instruction-count gate, baselines never typed, timing never in plane A, the allocation budget measured first, order and machines | SPR-009 (Architect rulings, 2026-10-16) |
| 7 | criterion for wall-clock trends | Approved, `=0.8.2`, default features off, `rh-bench` dev-dependency only |
| 8 | Our own counting allocator | Approved; `rh-bench`'s `src/alloc_count.rs` on the reviewed unsafe allow-list, behind `count-alloc`, gated by lint_arch |
| R-7 | Instruction counts depended on the checkout's path (up to 13%) | A deterministic bump arena behind `arena-alloc`; counts measure computation with allocation at a constant cost |
| R-8 | Ruling 3 amended | Integrity within max(64 instructions, 1e-5 relative) per count, every difference printed; toolchain lines and allocation counts exact |
| R-9 | zstd's run-time BMI2 path | BMI2 on the toolchain lines; a host without it reports "re-baseline required" |
| DEF-1 | An observer panic | A fatal `SimError::Observer`: the engine is poisoned, the crash artifact names the observer, nothing unwinds out of `advance_until`. Accepted deviation: `snapshot` and `state_hash` still compute on a poisoned engine, while `write_keyframe` refuses (18 of 18 notice kinds and thread counts, Tester); the question for every poison is BCK-002 A-42 |
| Phase 1 | Dispositions | Fixed before merge: DEF-3, GAP-1 (with SPR-008's `edited` trigger), GAP-2 (never fail open: a push range needs a `[perf-baseline]` commit with a `Reason:` line; no context prints "not checked"), GAP-6, GAP-9, OBS-1, C2, GAP-7, GAP-8, GAP-10 (README). Backlog: GAP-3 and GAP-4 (A-43), GAP-5 (A-44), Y1–Y6 (A-41), GAP-11 (T-04, the director's session), GAP-12 (phase 2) |
| Review of `c29787c` | The Architect's review | §4 |
| Class | The plan said C1, "planes touched: none" | C2. The sprint adds a public interface to `rh-core` (`Observer`, `Notice`, `set_observer`) and `SimError::Observer`, and changes `run_level` and `run_instant`. History is unchanged: the smoke ladders are byte-equal to `main`'s, with and without an observer. C2 needs a Tester pass and this audit, which it has |

## 4. Defects and gaps: disposition
| Item | Disposition |
|---|---|
| **DEF-1** (medium; Tester phase 1) a panicking `JobStart`, `JobEnd` or `LevelMerge` observer unwound out of `advance_until` with the level's columns dropped, no artifact and no poison | Fixed (`8d6c51c`): every notice goes through `catch_unwind`; a panic is `SimError::Observer`; columns return to the tables before the merge. Re-verified: 9 notice kinds × 1/2/4/8 threads |
| DEF-2 (medium-low) nothing pinned a notice's time, phase or worker | Fixed (`46f0f59`): notices compared with an independently derived expectation; the Tester's mutants are red |
| DEF-3 (low) `wall_trend.py check` passed a NaN median | Fixed (`c29787c`): non-finite and non-positive medians and negative spreads refused; a spread of 0 allowed |
| DEF-4 (low) the allocator self-test could not see `alloc_zeroed` | Fixed (`46f0f59`): `vec![0; 64]` in the known pattern |
| OBS-1 (Tester) an `InstantStart` crash artifact named the previous instant | A real bug: the current instant was set only inside the phase loop. Fixed (`c29787c`) at the top of `run_instant`; state-neutral (not in the snapshot, `state_hash` or keyframes); tested at all 9 kinds, 1 and 4 threads |
| **Review 1** (blocker) `test_perf_gate.py` read CI's `RH_EVENT`; CI red on `c29787c` | Fixed (`ded5f7f`): `python/tests/conftest.py` clears every `RH_*` and `GITHUB_*` variable before each test; `check_all` runs pytest again under hostile CI values. The second time this class reached CI (SPR-008's `test_check_baseline` was the first), so the fix is repository-wide |
| **Review 2** (major; V-PERF-04) the PR tier took about 17 min 40 s on a slow runner | Fixed (`ded5f7f`): the debug workspace run leaves out the binaries the by-name gates run (an inventory test proves each runs once); `bin/check_all.sh --part core|golden`, two parallel CI jobs with a verdict per part; the vitals observer rides on the N-thread golden run. Acceptance: ≤ 12 min of CI wall time. Measured: about 11 min on cold caches (`core` 9:55, `golden` 10:54) |
| **Review 3** (major; an escape against SPR-008) GitHub keeps one pending run per concurrency group, so a third quick push to `main` dropped the second push's range from the `[C3]` and `[perf-baseline]` push rules | Fixed (`ded5f7f`): a push gets its own group, its SHA. Recorded in `VER-SPR-008` as an escape found later |
| Review 4–8 (minor) lint_arch skipped every file under a checkout path containing `target`; the PR tag accepted mid-title; an unknown Actions event treated as local; no toolchain lines on the allocation budget; the observer error text named the worker | Fixed (`ded5f7f`), each with a control or test |
| Review 9–11 (nits) vitals semantics, C-library allocations, the PR body | README notes; PR body updated |
| Mutants surviving at `46f0f59` | C2 already red on the named regression; R4 immaterial (an equal-size `realloc`); Y1–Y6 (CI workflow edits) are BCK-002 A-41 |
| DEF-6 (low; Tester phase 2) the vitals pace truncated to whole years | Fixed (`a4b78cb`): one exact ratio; `--years 1/2/3` print 15.7/15.7/15.9 (Tester) |
| GAP-13, GAP-15, G3, D4 (Tester phase 2) 13 green mutants: the vitals timing arithmetic, four branches of the allocation comparison, the no-base push, 0 samples | Fixed (`a4b78cb`): fake-clock tests of `Timing` at 1 and 2 workers; one test per allocation field; the no-base push and 0-samples tests. All 13 mutants now fail (Tester phase 3) |
| GAP-17, GAP-18 | A docs line beside `[C3]` (never squash these PRs); the core part runs ruff, mypy and pytest before the Rust builds |
| **GAP-19** (medium; Tester phase 3) nothing ties `ci.yml` to the parts: a matrix of `[core]` alone passes every test, and the golden part would leave CI silently | Fixed (`69ce702`): `python/tests/test_ci_workflow.py` parses the CI matrix and the `--part` argument against `check_all.sh`'s parts. Controls: `[core]`, `[core, core]`, a hard-coded `--part core`, `CHECK_PART: core` and a bare `check_all.sh` all fail; Architect-verified (a `[core]` matrix and `CHECK_PART: core` each turn the test red). The first piece of A-41 |
| GAP-21, GAP-22, T4 (Tester phase 3) lint_arch skipped any `target` component below the crate; the fake clock started at 0, so an origin error survived (V10b); the DEF-1 test panicked only at job index 0, so a worker number in the error text survived | Fixed (`69ce702`): lint_arch skips only the crate's top-level `target/` (fixture `global_allocator_src_target`, a FAIL gate; Architect-verified); the scripted clock starts at 3,600,000 ms, and V10b now fails; a DEF-1 test panics at job index 1 and compares the tick and text at 1 and 4 threads, so a worker number in the text fails |
| GAP-20 (Tester phase 3) the inventory counts a filtered must-fail control as coverage of a whole test binary | Fixed (`69ce702`): a filtered or must-fail by-name gate covers nothing in the inventory; a test pins the three cases |
| GAP-14, GAP-16, Y7, Y8 | Backlog: the serial fraction's semantics with A-32 (it depends on thread count and includes spawn cost); GOV-006 §2 signals (A-45); Y7 and Y8 with A-41 |
| GAP-6, GAP-9 residuals | Accepted: a `]` inside a string in the same attribute; a huge `IDLE_MINUTES` integer |

**Escapes found later:** none so far.

## 5. Acceptance criteria
| Task | Criterion | Met |
|---|---|---|
| T-01 | Benches 7 and 8; bench 4 a named slot; baselines recorded | Yes |
| T-02 | V-PERF-04 positive control on gate (a); two runs on one commit give identical counts | Yes |
| T-03 | Vitals printed by `rh-cli run smoke` | Yes; the N-thread golden run carries the observer and equals the baseline |
| T-04 | Machine ratios and wall-clock baselines | **Blocked: the director's machines** (ruling 6). Procedure and scripts done |
| T-05 | A fixture that allocates in its tick fails; two runs report identical counts | Yes: 82 of 82 runs byte-identical (pinned to 1–3 cores, under load, other cwd and env); eight allocating fixtures fail OVER BUDGET (Tester phase 2) |

## 6. Verdict
| Field | Value |
|---|---|
| Verdict | **PASS** (2026-10-17) |
| Merge route | The Architect merges (GOV-005 §6), with a merge commit |
| Carried forward | T-04 (the director's session); BCK-002 A-32 (the allocation sites are its work list), A-41 (a ci.yml structural lint), A-42 (snapshot and `state_hash` on a poisoned engine), A-43 (wall-clock gate preconditions: machine, spread, trend-file integrity), A-44 (plane-A file-metadata and process lint, in SPR-011 T-00), A-45 (the vitals' GOV-006 §2 signals and serial-fraction semantics). SPR-011 also takes A-37 and the 20-minute timeout |
