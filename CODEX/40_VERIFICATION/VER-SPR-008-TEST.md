---
id: VER-SPR-008-TEST
title: "Determinism tester evidence for SPR-008"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing, determinism]
related: [SPR-008, VER-SPR-008, VER-SPR-007-TEST, AGT-003-DET, ADR-0006, ADR-0033]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF (round 1, head `29abe04`):** The machinery works and I could not break it, with one hole that matters: **the smoke world never runs two jobs in one level, so thread-count invariance on it is vacuous (DEF-2).** Ladders are byte-identical at 1 to 64 threads, across repeats, debug and release builds, `target-cpu=native`, and `+fma,+avx2`. Resume equals the continuous run at six cut points. My own bisect harness named the exact month, table and row in 66 of 66 random and edge cases (plus 8 more for added and removed rows, cancel-out and multi-table cases). The `include!` scan held against 67 of 69 evasions (the other two were wrong expectations of mine). A hand-edited or regenerated baseline fails the gate, and a planted history change was bisected automatically to its month and table. `bin/check_all.sh` is 187 of 187 on a clean archive. **Open: DEF-1 (low) and DEF-2 (medium), both accepted by the Architect for round 2; GAP-1 goes to BCK-002 A-36.** Cross-machine evidence was not run, as instructed.

Tested head: `29abe041d2547f93f3c49fdf7b1f0a03df192247` (PR #28, on main `c8acb54`). Probes ran on `git archive` copies with a fresh `CARGO_TARGET_DIR` per copy; the repository's working tree was only read. The probe plan was written from the contracts (SPR-008 v1.2.0 with its nine rulings, `docs/13` §4.2–4.5, ADR-0006, ADR-0033 §3, `05.1` §2, §4.2, §6, `09` V-DET-01, 02 and 11, BCK-002 A-27) before the branch was opened. The Architect's answers on `include!($file)`, the `[C3]` title and the cancel-within-a-year case were applied. Mutants of the engine (a perturbation hook of my own, an event-only change, planted leaks, and counters that log every dispatched job) were built in throwaway copies and are not in the repository. Tester model: set by the Architect in GOV-005 §4.2; not recorded here by instruction.

## 1. Verdicts (round 1)

| Task | Verdict | Basis |
|---|---|---|
| T-00 `include!` scan (A-27) | **PASS** | 67 of 69 probes as expected, the other 2 my own mistakes (§2.1); the counted `LINT-ALLOW` in `real_specs.rs` is gone; 10 literal includes followed in the workspace |
| T-01 smoke world | **PASS with DEF-2** | An independent counter shows all 12 phases, all 5 pulses, two bindings of one system in a phase, timers fired, cancelled and re-sampled, commands sent and applied, causal events, an invariant per system and `rh-sample` draws (§2.2). But no level ever holds two jobs |
| T-02 ladders, V-DET-01/02 | **PASS with DEF-2, DEF-1** | Byte-identical across threads, repeats and builds (§2.3). The thread-count part cannot fail on this world |
| T-03 bisect, V-DET-11 | **PASS** | 66 of 66 cases, plus 8 shape cases, plus the automatic CI path (§2.5) |
| T-04 baselines | **PASS with GAP-2** | Hand edit and non-C3 regeneration fail; the `[C3]` twin passes; a changed root fails at its rung (§2.4) |
| Deviations 1 to 8 (Architect's list) | see §2.6 | 1 holds (DEF-1, GAP-1); 2 holds; 3 to 8 verified |
| `bin/check_all.sh` | **PASS** | 187 of 187 on a clean archive of `29abe04`; PR CI run 38931650691: the `check_all.sh` step took 10 min 56 s, whole job 11 min 30 s (warm cache) |
| Cross-machine check | **not run** | As instructed (needs the director's laptop and `adjutant`); `bin/cross_machine.sh` itself ran here against my own ladder and reported `VERDICT: equal` |

## 2. Results

### 2.1 T-00: the `include!` scan
69 cases through `lint_forbidden.py --scan`, each with a control in both directions.

| Group | Cases | Result |
|---|---|---|
| Literal, in-repo, clean (also raw string, nested 2 and 3 deep, `../` staying inside, empty file, 200,000-line file, a symlink to an in-repo file, the same include twice) | 12 | pass, as expected |
| Forbidden API inside the included file (`HashMap`, `Instant::now`, `mul_add`, `f64`, `unsafe`, `thread_rng`; also `#[path]` and `HashMap` inside a nested file) | 9 | FAIL, as expected, reported at the included file's own line |
| Not a plain literal: `concat!`, `concat!(env!(..))`, `env!`, `stringify!`, a `macro_rules` metavariable, a byte string, `\x`/`\u{}` escapes in the literal, a line-continuation literal, a trailing comma, two arguments, two literals | 13 | FAIL, as expected |
| Outside the repo: absolute path, `../` out of the repository, `sub/../../..` out, a symlink to a file outside (bad and clean content), a symlinked directory component, a symlink loop | 8 | FAIL, as expected |
| Missing file, a directory, a cycle `a→b→a`, a self-include, case variant, trailing space, NUL in the path | 7 | FAIL, as expected |
| Token forms: `include !` with comments, `include!{}`, `include![]`, `std::include!`, `::core::include!`, in `fn`, in `#[cfg(test)]` | 7 | all followed and scanned |
| Must not be followed: in a line comment, a block comment, a doc comment, a string; `include_str!` and `include_bytes!` of a `.rs`; a `.toml` | 7 | pass, as expected |
| Escapes: a reasoned `LINT-ALLOW` inside the included file is accepted; the old escape on the `include!` line now covers nothing and FAILS | 2 | as expected |
| BOM file with a violation; float in an included file | 2 | FAIL, as expected |
| Non-UTF-8 included file | 1 | exit **2** ("cannot read the code to scan"), not exit 1: still a refusal; my expectation was wrong |
| `--scan` of a `tests/` file that includes a float file | 1 | exit 1: `--scan` never applies the `tests/` float exemption by design; my expectation was wrong |

Workspace mode: `lint_forbidden` reports `10 literal include! followed`; `grep LINT-ALLOW crates` finds only `rh-sample`'s. A planted dirty skeleton and a stale smoke skeleton are both red in `check_all.sh` (controls pass).

### 2.2 T-01: what the smoke world exercises (independent counter)
I added counters to a copy of `rh-core` and `rh-golden` (every dispatched job, `schedule`, `cancel`, `submit`, `emit`, `rng`, `stream`, timer fire, re-sample call, invariant evaluation) and ran 100 years.

| Ruling 3 item | Observed |
|---|---|
| Every phase | all 12 (Inputs, Environment, Production, Exchange, Vital, Activity, Contagion, Polity, Military, Agenda, Culture, Bookkeeping) |
| Every pulse | Hour 876,000; Day 36,500 per binding; Month 1,200; Season 400; Year 100 |
| Two bindings of one system in a phase | X5 binds Month and Year in Polity |
| `same_instant` and `lagged` reads | X6 reads `person.alive` same-instant and `estate.heirs` lagged (from the code; the schedule order and the ladders agree) |
| Timers | 5,051 scheduled, 4,559 fired, 491 cancelled, 8 re-sample calls (8 returned a time), 112 material inputs applied |
| Commands | 5,944 sent (`Settle`) and 5,944 delivered to the owner's inbox |
| Causal events | `CENSUS_NOTE` with a draft-local causal edge, 1,200 |
| Invariants | evaluated for all six systems, every instant |
| `rh-sample` draw | `stream yield_noise` 1,600 (Poisson) |

**What it does not exercise: parallel jobs.** `level_jobs 1` was recorded 1,032,358 times and nothing else; `parallel_dispatch` stayed at 0 and a single thread id appeared, at `--threads 4`. See DEF-2. The world is also small (tables of 1, 4 and 64 rows), so no table has a second Merkle chunk (1,024 rows).

### 2.3 T-02: ladders
| Probe | Result |
|---|---|
| 50 years monthly at 1, 2, 3, 4, 7, 8, 16 and 64 threads × 3 repeats (24 runs), `TZ=Asia/Tokyo`, `LANG=C.UTF-8`, another working directory | all 24 `cmp`-equal to the 1-thread ladder |
| 1,000 years yearly at 1, 2, 4, 8 threads | equal to each other and to the committed baseline |
| 1,000 years monthly (12,000 rungs, 11 MB) at 1 and 4 threads | equal; 60 s per run |
| Builds: debug; release; release `target-cpu=native`; release `opt-level=1 +fma,+avx2` (50 years monthly, 1 and 4 threads, run from `/var` with `--repo`) | all equal to release at 1 thread |
| No wall time or build id in a ladder | header is world, seed, years, rung, hash layout; no path, date, host or SHA anywhere in 11 MB |
| `months` field (deviation 2): a one-bit change to a table at month 5, undone at month 6 | for 9 of 15 tables the year-end state hash is equal and the `months` field differs, so a yearly ladder catches it; the same at 4 threads |
| An event-only change (one `CENSUS_NOTE` payload changed in month 5, no table touched) | `rh-core/events.meta` differs from rung 1 onward, in every later rung, at 1 and 4 threads; the bisect names month 5 and `events.meta` |
| Horizon in history (deviation 1) | the 50-year and the 1,000-year ladders differ at rung 1 (`engine.inputs`, `engine.meta`): a ladder of fewer years is **not** a prefix. See GAP-1 |
| `ladder compare`: identical; a changed month-hash or table root at rungs 1, 2, 500, 1,000; truncated; one extra rung; a `failed` ending; a different rung kind, seed, years, world or hash layout; swapped arguments | exit 0 or 1 as appropriate, the first differing rung and the table named, headers named; swapping arguments gives the same verdict |
| `ladder compare` on an unknown version, an empty file, garbage, uppercase hex, a missing file or a directory | exit 2 with the reason (a version mismatch is refused, not compared) |
| `ladder compare` on CRLF line ends and on a missing final newline | exit **0** "EQUAL". See GAP-4 |
| `bin/cross_machine.sh` against my own monthly ladder | `VERDICT: equal`; writes the CPU, toolchain and both verdicts |

Planted leaks (a copy of the weather system, 50 years monthly, threads 1, 2, 4, 4 × 3 repeats, 11 comparisons each):

| Leak | Detected |
|---|---|
| none (control) | 0 of 11 differ |
| `HashMap` iteration order | 11 of 11 |
| wall clock (`SystemTime` nanoseconds) | 11 of 11; also red in the PR-shaped 1,000-year yearly run (1 vs 4 threads, and against the baseline) |
| thread identity (`thread::current().id()` hashed) | **0 of 11** (DEF-2) |
| core count (`available_parallelism() > 4`) | 0 of 11; this machine has 4, so it can only show on another machine: the cross-machine check's job |

### 2.4 T-04: baseline integrity (`bin/check_baseline.py`, driven by a real git base)
| Scenario | Expected | Actual |
|---|---|---|
| Unchanged baseline, recomputation equal, base = HEAD | pass | `BASELINE OK` |
| Hand edit: one hex digit; a space; no final newline; CRLF; two roots swapped; version `v2`; last rung dropped; empty file | fail | 8 of 8 `BASELINE MISMATCH`, each with the first differing line |
| Baseline deleted | fail | `NO BASELINE` |
| Regenerated baseline (a real changed history, recomputed at 1 and 4 threads), title without `[C3]` | fail | `BASELINE CHANGED in a PR not marked [C3]` |
| Same, title `[C3] SPR-009` | pass | `BASELINE OK ... regenerated in a [C3] PR` |
| Title variants: `[c3]`, `C3`, `[C3 ]`, `[ C3]`, `[C3`, Cyrillic `[С3]`, fullwidth `［C3］`, empty | fail | 8 of 8 fail |
| Title `no [C3] here` | (should not pass) | **passes**: the check is a substring match. GAP-2 |
| Regenerated baseline but one of the two recomputations is the old history | fail | `RECOMPUTATIONS DIFFER` |
| Invalid base ref | fail closed | exit 2 |
| Baseline new at the base (absent there) | counts as changed | fails without `[C3]` |
| No base ref (push or manual dispatch) | counts as unchanged | a regenerated baseline passes if it equals the recomputation; by design |
| `check_all.sh` controls (hand-edited, non-C3, C3 twin, one changed root) | as stated | all PASS inside the 187 |

The `--inject` hook (deviation 6): it is a command-line flag only (`rh-cli run smoke --inject M:TABLE:ROW`); no environment variable, script or workflow passes it (grepped `bin/`, `.github/`, the Rust sources). `regen_baseline.sh` and every golden gate run without it. A ladder written with it carries no marker, but the gate recomputes without it, so such a ladder cannot be a baseline.

### 2.5 T-03: the bisect (V-DET-11), my own harness
The candidate was a build of the same source whose perturbation hook I wrote (any XOR mask, appended or removed rows, several edits in one run), driven through `rh-cli bisect --baseline <binary> --candidate <binary>`; the baseline binary is the untouched build. Seeds are mine (80081016).

| Probe | Result |
|---|---|
| 11 edge months (1, 2, 12, 13, 119, 120, 121, 240, 241, 719, 720 of 720; 120 and 121 straddle the year-10 keyframe) with random table, row and mask | 11 of 11 named exactly |
| 40 random (month, table of 15, row, mask) | 40 of 40 |
| The last row of each of the 15 tables at a random month | 15 of 15 |
| Row appended to a 1-row table; row removed from a 4-row table; a 65th row in a 64-row table | month, table and the row that is absent on one side named, with `absent` for the missing value |
| Two tables changed in one month; two rows of one table | the first table by name (`chronicle.census`), the first row (3 before 9) |
| A change at month 5 undone at month 6 (state equal again after 2 months, and one of 7 months) | month 5 named, though the year-end states are equal (the `months` field puts the divergence in the yearly rung) |
| An event-only change | month 5, `rh-core/events.meta` |
| Identical builds | `verdict: no divergence over the horizon`, exit 0 |
| Automatic CI path: a planted model change (`NoteTone % 4` to `% 5`) on a copy with two commits; a real `bin/check_all.sh` run | fails at "Golden baseline: ..." (every earlier gate passed, so the fail-fast order does not skip the golden gates); `golden_bisect_ci.sh` then built the base revision in a worktree and wrote `bisect_report.txt`: year 1, month 1 (0000-01), table `rh-core/events.meta`, chunk 0, row 0, byte 18, `e2` vs `05` |
| Keyframe skip-ahead | the bisect resumed from the year-0 keyframe for year 4, and from the nearest earlier permanent keyframe elsewhere; a corrupt or truncated keyframe is refused (§2.6) |

Not exercised: month 0 (the perturbation hook acts at month ends), an empty world table (none exists), and a second Merkle chunk (no table has more than 64 rows). SPR-007's tests cover the chunk logic itself.

### 2.6 The Architect's list of deviations
| # | Claim | Verdict |
|---|---|---|
| 1 | Inputs journaled up front; resume = continuous; header records the horizon | Resume equals continuous at cut years 0, 10, 30, 50, 90 and 100 (a 100-year run, monthly, 1 and 3 threads; every resumed rung equal and the header equal). Corrupt, truncated and empty keyframes, a missing keyframe or journal, and another seed are all refused (exit 2). **But** the horizon in the header comes from `--years`, not from the journal: DEF-1. The up-front design itself: GAP-1 |
| 2 | The `months` field catches a one-bit change absorbed before year end | Holds, and is stable across thread counts (§2.3) |
| 3 | Synthetic inputs are `OracleMove` rows from `Producer::System` | Verified in the journal: 112 rows per 100 years, kind `oracle_move`, scope entity, producer system; the 100-year journal is a strict prefix of the 200-year one |
| 4 | The ignored 1,000-year test was removed; the PR golden run stands in | Accepted: the golden gate runs 1,000 years headless on every PR (61 s) |
| 5 | New tool crate `rh-golden` | `plane = "tool"`, `kind = "tool"`. `lint_arch` goes red for: a system crate depending on it (two rules), `rh-core` depending on it, relabelling it plane A (metadata rule), and relabelling `rh-sys-smoke` to plane B or kind machinery. No plane-A crate depends on it. It uses no hash map, clock or environment variable that could reach a ladder (grepped) |
| 6 | `--inject` never reaches a baseline | §2.4 |
| 7 | CI timeout 20 to 30 minutes | Warm-cache CI run: job 11 min 30 s, inside the 15-minute PR budget (V-PERF-04). My local full run, with other work competing for 4 cores, took 17 min. A cold cache was not measured |
| 8 | Baseline gate | §2.4 |

### 2.7 Gate mutations
| Probe | Result |
|---|---|
| Delete each of the 22 new `run` lines of `check_all.sh` (golden, smoke-world, skeleton, include, `rh-golden`, `rh-sys-smoke` lines) | 22 of 22 turn the inventory test red |
| Comment out a gate line | red |
| Append `\|\| true` to a gate line | **survives** the inventory test. See GAP-3 |
| `ci.yml`, `nightly.yml`, `golden_nightly.sh`, `golden_bisect_ci.sh`, `regen_baseline.sh` | read in full; the commands were run by hand where possible (`golden_bisect_ci.sh`, `cross_machine.sh`); the workflows were not run on a runner and not mutated |

## 3. Defects

### DEF-1 (low): a resumed ladder's header misstates its history *(accepted; round-2 fix: the header records the journal's identity, and `--resume` refuses a `--years` past the journal's horizon)*
`run --resume` writes `--years` into the header, but the history is fixed by the journal.
```
rh-cli run smoke --years 100 --ladder monthly --keyframes kf --journal c100.sqlite --out c100.ladder
rh-cli run smoke --years 200 --ladder monthly --out c200.ladder --journal c200.sqlite
rh-cli run smoke --years 100 --ladder monthly --resume kf/year-0050.rhkf --journal c200.sqlite --out x1.ladder
```
The last command exits 0 and writes a header `years 100`; its rung 1200 equals the 200-year run's rung 1200 and differs from the 100-year run's. The reverse (`--years 200` with the 100-year journal) also exits 0 and runs 100 years with no inputs. The anchor check cannot catch either, because the 100-year journal is a strict prefix of the 200-year one (112 of 224 rows identical). Impact: low, since `ladder compare` against the real ladder still reports a difference; the header is what lies.

### DEF-2 (medium): thread-count invariance on the smoke world is vacuous *(accepted; round-2 fix: sibling bindings in a level every month, the run reports levels with two or more jobs and a gate fails when there are none, a planted thread-identity leak must fail the 1-versus-N comparison)*
Counters in a throwaway copy: in 100 years at `--threads 4` the engine formed 1,032,358 levels, **every one with one job**. `dispatch()` always took its single-thread branch, `parallel_dispatch` stayed 0 and one thread id appeared. Each phase holds the bindings of a single system, so `--threads` changes nothing. Consequences:
- the V-DET-01 gates (PR tier 1 vs N; nightly 1, 2, 4, N × 3) are green by construction, and the G0 criterion is not tested by them;
- a planted thread-identity leak went undetected (§2.3), although the `HashMap` and clock leaks are caught;
- the engine's parallel dispatch and merge are covered only by SPR-005's stub world (`rh-core/tests/determinism.rs`), which has sibling systems in a level.

Ruling 3 never requires siblings in one level, so the Developer met its letter; the purpose is not met. The Developer's own small-scale test (`smoke.rs`, "equal at 1, 2, 4 and 8 threads") has the same hole.

## 4. Contract gaps

| Gap | Recommendation | Confidence |
|---|---|---|
| **GAP-1** (medium): the journal's future inputs are in `state_hash`. With journaling up front, `engine.inputs` holds every not-yet-due input, so the state at year 1 depends on the horizon: a 50-year ladder is not a prefix of a 1,000-year one, and a keyframe taken live (inputs arriving over time) would not equal one resumed from a complete journal. | **Disposition: P0 accepts up-front journaling; the Architect files BCK-002 A-36** (the journal records each row's acceptance cursor and resume submits a row when the engine reaches it). I had suggested excluding the not-yet-due part from the hash; the Architect prefers keeping it hashed, since a keyframe must carry the inputs. Until A-36, every shorter PR horizon needs its own baseline | 0.6 (mine); 0.8 (Architect's) |
| **GAP-2** (low): the `[C3]` marker. (a) It is a substring test, so `no [C3] here` passes. (b) It is read from the event payload when CI starts: `ci.yml` uses the default `pull_request` types (opened, synchronize, reopened), so editing the title afterwards does not re-run the gate, and a re-run of the same run reuses the old payload. A PR can pass with `[C3]` and be renamed before merge, or be fixed by adding `[C3]` only with a new push (from GitHub's documented behaviour; not run on a runner). (c) The gate compares the working tree with the base *tip*, so a branch behind a merged C3 PR counts as "changed" (fails closed). (d) Nothing makes edits to the gate itself (`bin/check_baseline.py`, the golden lines, `rh-golden`) require `[C3]`, though `docs/13` §4.2 calls the determinism machinery C3. | The Architect's merge checklist reads the title at merge time; match `\[C3\]` at the start of the title; add `edited` to the `pull_request` types only if the 11-minute cost is acceptable; add the gate files to the path set that demands `[C3]` when a detector lands | 0.8 |
| **GAP-3** (low, inherited from SPR-001): `check_all.sh` exits with the status of its last command, not `$failed`, so a gate line ending in `\|\| true` stays green and the inventory test does not see it | `exit` on `[ -z "$failed" ] && [ $status -eq 0 ]`; make the inventory test refuse `\|\| true` and `\|\| :` on a `run` line | 0.85 |
| **GAP-4** (low): `ladder compare` accepts CRLF line ends and a missing final newline as "EQUAL", though ruling 4 says ladders compare byte for byte and the format says `\n` and a final newline | Parse strictly (refuse both), or have the PR-tier comparison use `cmp` as the baseline gate does | 0.8 |
| **GAP-5** (low): for an event-only divergence the report names a byte of `events.meta` (the running hash), not the event | Keep a per-month event digest list in the dump so the report can name the first differing event | 0.5 |
| **GAP-6** (info): the nightly compares its 12 monthly ladders with each other but not with the committed baseline; on a 4-core runner N equals 4, so the matrix runs 1, 2, 4, 4; a leak that depends on the machine (core count) shows only in the cross-machine check | Compare a yearly ladder cut from the monthly one with the baseline nightly; keep the cross-machine check scheduled | 0.5 |

## 5. Observations
- OBS-1: the bisect report names the two binaries by absolute path; it is deterministic for one machine only.
- OBS-2: `cross_machine.sh` writes `cpu:` without a space; cosmetic.
- OBS-3: the smoke world has 22 tables (15 world, 7 engine-held) of at most 64 rows; the chunk step of the bisect (SPR-007) is not exercised end to end by it.
- OBS-4: a non-UTF-8 included file makes `lint_forbidden` exit 2 rather than 1; both refuse.
- OBS-5: `golden_bisect_ci.sh` runs only when a check-report row named `Golden ...` failed. In my planted run that was the case; a history change that tripped an earlier gate (a unit test with a golden value) would stop `check_all.sh` before the golden rows and produce no bisect report. None of the existing tests did.

## 6. Not run
- The cross-machine check (the director's laptop and `adjutant`); level 2 across platforms; Windows and macOS ARM.
- The workflows on a runner: `ci.yml` and `nightly.yml` were read, not executed or mutated; GitHub's title-edit behaviour is from its documentation. A cold-cache PR run was not timed.
- The full nightly (`golden_nightly.sh`) end to end: I ran its parts (1,000 years monthly at 1 and 4 threads, the bisect cases) but not the 12-run loop or its 64-case V-DET-11 test.
- Month 0, an empty table and a table over 1,024 rows through the bisect (§2.5).
- Round 2 (the Developer's fixes for DEF-1 and DEF-2); I will re-run the three leaks, the parallel-dispatch counters, DEF-1 and the baseline-gate changes on the head I am sent.
