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
version: 1.2.0
---

> **BLUF (round 3, head `6a87616`; section 10): DEF-3, GAP-7 and GAP-8 are fixed and the merge of main resolved correctly. `bin/check_all.sh` is 226 of 226 on a clean archive with an absolute `CARGO_TARGET_DIR`. The rewritten baseline equals my own recomputation byte for byte, and my three leaks are still caught on the new world. No open defect.** Round 2 follows.
>
> **BLUF (round 2, head `8e17816`; section 7):** DEF-1 and DEF-2 are fixed and I could not reproduce either; my own thread-identity, HashMap-order and wall-clock leaks are all caught now. GAP-3, GAP-4 and GAP-6 hold, MAJ-1 fails closed. **One new defect: DEF-3 (low-medium), `check_all.sh` fails with an absolute `CARGO_TARGET_DIR`; with one line patched the tier is 210 of 210.** Details in section 7. Round 1 follows.
>
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

## 7. Round 2 (head `8e17816`; the branch head when I committed was `cac67de`, a merge of main that I did not test)

Probes ran on a `git archive` copy of `8e17816` with a fresh `CARGO_TARGET_DIR`. The Developer's merge `cac67de` adds main's changes (manifest examples, the `05.1` changelog); I did not re-run the tier on it.

| Item | Verdict | Basis |
|---|---|---|
| DEF-2 (vacuous thread invariance) | **fixed** | Section 7.1 |
| DEF-1 (resume header) | **fixed** | Section 7.2 |
| MAJ-1 (baseline gate fails closed) | **pass** | Section 7.3 |
| GAP-3 (`\|\| true`) | **fixed**, with a lint evasion (GAP-7) | Section 7.4 |
| GAP-4 (strict ladder bytes) | **fixed** | 8 of 8 malformed ladders refused (section 7.4) |
| GAP-6 (nightly cut = baseline) | **fixed** | Section 7.4 |
| MIN-1 PTRS, MIN-2 inject line, MIN-3 includes | **pass** | Section 7.5 |
| Bisect after the changes | **pass** | 33 of 33 (section 7.6) |
| `bin/check_all.sh` | **FAIL as shipped (DEF-3); 210 of 210 with one line patched** | Section 7.7 |

### 7.1 DEF-2
- **Real parallelism.** Counters in a throwaway copy (30 years, 4 threads): 287,782 one-job levels and 21,900 two-job levels; `parallel_dispatch` 21,900; 43,801 distinct thread ids (one at 1 thread). Before: 0 and 1. The runner's own summary agrees (100 years: 73,000 of 1,032,358 levels, 7%, at most 2 jobs in a level) and is the same at 1 thread, since it counts jobs per level, not threads used.
- **`--require-parallel`.** 5: passes. 8 and 99: exit 1, "PARALLEL LEVELS BELOW THE FLOOR: 14600 of 206472 levels (7%) ... the floor is 8%". 0: passes.
- **My leaks, planted in X1's Hour step** (which now shares levels with X7), 30 years monthly at threads 1, 2, 4, 4 × 3 repeats, 11 comparisons each, written by me and not the Developer's `--leak-thread`:

| Leak | Round 1 | Round 2 |
|---|---|---|
| none (control) | 0 of 11 | 0 of 11 |
| `HashMap` iteration order | 11 of 11 | 11 of 11 (the PR-shaped 1-vs-4 run ends differently: one side fails an invariant) |
| wall clock | 11 of 11 | 11 of 11; PR-shaped: rung 1 differs |
| thread identity (`ThreadId` hashed) | **0 of 11** | **9 of 11**; the 2 equal are the 1-thread repeats, which must be equal. PR-shaped (yearly, 100 years, 1 vs 4): first differing rung 6, `weather.hours` |

- The Developer's `--leak-thread` also fails 1-vs-4 at rung 1 (`market.ticks`) and is stable across two 4-thread repeats. Its ladder carries a `leak thread` header line, but `check_baseline.py` accepts such a ladder as a baseline when the recomputation is the same file (it is not in the real gate, whose recomputation has no leak). Defence in depth: refuse a baseline with an `inject` or `leak` line.
- Limit: no level holds more than 2 jobs, so only 2 threads are ever busy at once.

### 7.2 DEF-1
| Command | Result |
|---|---|
| 100-year keyframe, 200-year journal, `--years 100` | exit 2: "--years 100 disagrees with the journal's horizon: the journal holds 224 inputs, not the 112 of a 100-year history ... stop earlier with --until" |
| 100-year journal, `--years 200` | exit 2: "goes past the journal's last input ... Pass --extend" |
| `--years 200 --extend` | runs; the header says `years 200` and `journal 112 <head>`, so it cannot pass for a real 200-year ladder (`journal 224 ...`) |
| 100-year journal, `--years 50 --extend` | exit 2 |
| honest resume at 50 | header equal to the continuous run's, all 600 resumed rungs equal |
| `--until 60`; `--until 120` (past the horizon); `--until 0`; `--until -3` | 720 rungs, header keeps `years 100`; exit 2; 0 rungs, exit 0; exit 2 |
| header journal line | `journal <count> <chain head>`; changing the hash, the count, or deleting the line each give "the headers differ in `journal`" |

### 7.3 MAJ-1: the baseline gate, driven by real git history
- A pull request with no base: exit 2 (fails closed). A bad base ref or a bad `before` sha: exit 2.
- Titles for a changed baseline: pass for `x [C3]`, `[C3]`, `x [C3] [perf]`, `x [perf] [C3]`, trailing spaces; fail for `docs: explain the [C3] rule`, `[C3] x`, `x [C3] y`, `[c3]`, Cyrillic `[С3]`, `[C3 ]`, `[[C3]]` and the empty title.
- Push ranges: `C0..C1` (a regeneration commit with no tag) fails; `C0..C2` (plus a docs commit whose subject has `[C3]` in the middle) fails; `C0..C3` (a commit ending in `[C3]`) passes; `C1..C4` (no baseline change in range) passes; a `[C3]` only in a commit body does not count.
- `ci.yml`: `types: [opened, synchronize, reopened, edited]`, `fetch-depth: 0` on a push, the base fetched for a pull request.
- Residual (observations): a push to a new branch (`before` all zeros) reports "C3 rule not checked: no base" and passes; and `concurrency: cancel-in-progress` cancels a superseded push run, so the next run's range (`before` = the cancelled head) no longer includes the cancelled push's baseline change. Both matter only for pushes that bypass pull requests; recommendation in GAP-8.

### 7.4 GAP-3, GAP-4, GAP-6
- **GAP-3.** `check_all_selftest.py verdict` runs the real `run()` and verdict code on a planted chain: a failed gate hidden by `|| true` still gives `FAIL at "planted failing gate"`. The new lint refuses `|| true`, `|| :` and `||true`, on a continuation line too. **It does not see** `|| exit 0`, `|| /bin/true`, `|| echo ok`, `|| { :; }`, `; true`, a subshell wrap, or `if ...; then :; fi`; of these only `|| exit 0` defeats the verdict (it ends the script with status 0 before the verdict). Reproduced on a copy of `check_all.sh` whose first gate was `false || exit 0`: exit code 0, `FAIL planted failing first gate` in the report, no verdict line. **GAP-7.**
- **GAP-4.** `ladder compare` now refuses (exit 2, with the line): CRLF, a lone CR, no final newline, an extra blank line, a trailing space, a tab, a BOM.
- **GAP-6.** `ladder cut-yearly` turns a monthly ladder into a yearly one. The 100-year cut is byte-equal to a real yearly run; the 1,000-year monthly run's cut is **byte-equal to the committed baseline**; `golden_nightly.sh` does `compare` and `cmp` of that cut. A yearly input is refused (exit 2).

### 7.5 MIN-1, MIN-2, MIN-3
- **MIN-1.** `SHOCK_MEAN = [5, 5, 5, 12]`; `rh-sample`'s `PTRS_THRESHOLD` is 10, so region 4 draws through PTRS and the others through inversion, inside the golden run.
- **MIN-2.** `--inject 30:person.alive:3` writes an `inject 30:person.alive:3` header line, and the bisect still names the month, table and row through `--candidate-arg`. (See the note in 7.1 on baselines that carry it.)
- **MIN-3.** The 69 round-1 `include!` probes: the same 67 as expected and the same 2 wrong expectations. New probes, 24: a symlinked include resolves from the link's directory (clean-beside-link passes, dirty-beside-link fails, an inner file only beside the target fails); `use core::include as inc;`, `use std::include as inc;`, `use std::{include as inc};`, `use ::core::{self, include as inc};`, `pub use ... as inc;` and `as _` are all refused; a `macro_rules` that passes a literal to `include!` is refused, one with the literal inside its body is followed; `include_str!` and `include_bytes!` with an in-repo literal pass, and a `concat!`, `env!`, absolute, `../` or missing path is refused. Two differences from my expectations: a renamed `include_str` (`use core::include_str as is;`) is not caught, which is harmless since it can only produce a string (OBS-6); and `use include::X;` for a user module named `include` is refused (a conservative false positive, OBS-7).

### 7.6 Bisect after the changes
33 cases through `rh-cli bisect --candidate-arg --inject M:T:R` (the Developer's hook, my seeds 20261016): month 1, 12, 13, 119, 120, 121, 479 and 480 of 480, 22 random months and tables, and the three new market tables; all 33 named the exact month, table and row.

### 7.7 DEF-3 (low-medium): `check_all.sh` fails with an absolute `CARGO_TARGET_DIR`
`bin/check_all.sh:278` builds `ladder="$PWD/$GOLDEN/smoke_yearly_t1.ladder"`. The script's header says built binaries live under `$CARGO_TARGET_DIR` when it is set, and GOV-002's verification protocol (a fresh target directory per copy) sets it to an absolute path; `$PWD/<absolute>` is `<repo>/<absolute>`, which does not exist.
```
CARGO_TARGET_DIR=/abs/path bin/check_all.sh
FAIL  Golden baseline positive control: a push that changes the baseline with no [C3] commit must FAIL (VER-SPR-008 MAJ-1)
check_baseline: <repo>/<abs path>/golden/smoke_yearly_t1.ladder: No such file or directory
```
The control fails for the wrong reason, and the tier stops at 181 of 187 gates. It passes on CI only because `target/` is relative there. With line 278 changed to `ladder="$(realpath "$GOLDEN")/smoke_yearly_t1.ladder"` in my copy, the tier is **210 of 210** (a clean archive, 18 minutes under load). The expected-text match (`fail:BASELINE CHANGED ...`) did its job: a control that fails for the wrong reason is caught, as designed.

## 8. Round 2: gaps and observations
| Item | Recommendation | Confidence |
|---|---|---|
| **GAP-7** (low): the `\|\| true` lint is textual; `\|\| exit 0` ends `check_all.sh` with status 0 and no verdict | Compute the verdict in an `EXIT` trap (a non-empty `$failed` forces exit 1), so no spelling can skip it; extend the lint to `exit 0`, `/bin/true`, `echo` | 0.8 |
| **GAP-8** (low): the push rule is not checked for a new branch, and a cancelled push run drops its range | Set `cancel-in-progress` only for pull requests; for a push with an all-zero `before`, compare with the default branch | 0.6 |
| **GAP-2** (remainder, backlog) | as recorded in section 4 | |
| **GAP-5** (backlog) | as recorded in section 4 | |
- OBS-6: a renamed `include_str!` is not caught (data only). OBS-7: `use include::X;` (a user module named `include`) is flagged.
- OBS-8: `check_baseline.py` does not refuse a baseline whose header has an `inject` or `leak thread` line; the recomputation in the real gate never has one, so it fails there.
- OBS-9: the full tier took 1,073 s wall under load on 4 cores (about 18 minutes); `ci.yml` has a 30-minute timeout and the round-1 warm-cache run took 11.5 minutes. The round-2 CI time was not measured.

## 9. Not run in round 2
- The merge `cac67de`, and CI on a runner; the cross-machine check; the full nightly loop (12 monthly ladders and its 64 bisect cases): I ran its parts (1,000 years monthly at 1 thread, the cut against the baseline, the bisect cases).
- `golden_bisect_ci.sh` after MIN-4 (the bisect on a push): read, not run.

## 10. Round 3 (head `6a87616`)

A clean `git archive` of `6a87616`, a fresh absolute `CARGO_TARGET_DIR` per copy. The head adds the pytest isolation fix, DEF-3, a merge of main (`070d6ac`, four conflicts resolved), the smoke world under the new `[[emits]]` lint (X3 emits `X_DEATH`, so the history changed and the baseline was regenerated in a `[C3]` commit), GAP-7 and GAP-8.

| Item | Verdict | Basis |
|---|---|---|
| DEF-3 (absolute `CARGO_TARGET_DIR`) | **fixed** | The full tier ran with an absolute target directory: **226 of 226**. `check_all_selftest.py abs-target` passes, and with `--plant-pwd-join` (the old join) it fails for the wrong reason, as a control should |
| GAP-7 (`EXIT`-trap verdict) | **fixed** | Section 10.1 |
| GAP-8 (no cancel on pushes) | **fixed** | `ci.yml`: `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`; the push trigger is main only, so the new-branch case cannot arise |
| pytest under CI's `RH_*` variables | **pass** | 447 of 447 plain, with the pull-request variables, with the push variables, and with hostile values (`RH_EVENT=bogus`, garbage shas, `GITHUB_*`, a nonexistent `CARGO_TARGET_DIR`) |
| Merge resolutions | **pass** | Section 10.2 |
| New baseline | **confirmed** | Section 10.3 |
| Leaks on the new world | **caught** | Section 10.4 |
| `bin/check_all.sh` | **PASS, 226 of 226** | Clean archive, absolute target directory; 1,192 s wall under load on 4 cores |

### 10.1 GAP-7: the verdict cannot be skipped
Each variant was planted into a copy of `check_all.sh` and run for real:

| Planted | Exit | Verdict line |
|---|---|---|
| first gate `false \|\| exit 0` | 1 | `FAIL at "planted failing first gate"` |
| second gate `false \|\| exit 0` | 1 | `FAIL at "planted failing second gate"`; the lint also refuses it |
| a passing gate then `&& exit 0` (an early exit) | 1 | `FAIL (the chain exited early, status 0, after 2 checks: later gates unrun)` |
| `\|\| { exit 0; }` | 1 | `FAIL at "planted brace exit"` (the lint does not see it; the trap does) |
| `bash -c "exit 0" && exit 0` | 1 | `FAIL (the chain exited early ...)` |
| `; true` | 1 | `FAIL` |
| `kill -9 $$` after a failed gate | 137 | none (a signal, non-zero) |
| **`trap - EXIT; exit 0`** | **0** | none |

The last one removes the trap on purpose. It needs a visible, deliberate edit and no spelling of `|| ...` reaches it; I record it as the accepted residual. The self-test (`verdict --or-true`, `--or-exit`, `--early-exit`) and the lint with its planted controls are in the tier.

### 10.2 The merge of main
- `lint_odd.py`, three modes. Real specs: `PASS: 6 system specs linted`. Smoke `--specs`: `PASS: 7`. Controls: an overlay id without the X prefix fails; a smoke spec missing from the overlay fails; `--specs` without `--overlay` fails; a real spec (M2) dropped into the smoke directory fails ("not in the overlay"); an overlay that claims a real id fails ("already in the Registry"); `--specs` together with `--extra-specs` over the same ids fails ("already has a spec"); `--extra-specs` with a duplicate of a real id fails; **X3 emitting M2's real verb `DEATH` fails** ("the Registry gives verb DEATH to M2.demography" and "[[emits]] does not declare X_DEATH"). `--extra-specs` with the smoke set alone fails on column ownership (`person.alive` is also M2's), which is the single-writer rule doing its job when the two sets are mixed. The `--extra-specs` fixtures (frozen, draft, overlay for `[[emits]]` and `[[applies]]`) are in the tier and pass.
- `gen_skeletons --check`: real `6 up to date`, smoke `7 up to date`; the stale fixture fails; a hand-edited smoke skeleton (mine) fails; a missing smoke skeleton (mine, X7) fails with `MISSING`; a hand-edited real skeleton (mine) fails.
- `real_specs.rs`: eight literal `include!` calls, no `LINT-ALLOW`; `lint_forbidden` reports `0 violations; 13 literal include! followed`.

### 10.3 The new baseline
I recomputed it from a build of my own: `rh-cli run smoke --years 1000 --ladder yearly --require-parallel 5` at 1 and at 4 threads (10,323,515 levels, 730,000 with two or more jobs, 7%). The two ladders are byte-equal, and **equal to the committed baseline byte for byte** (`journal 1136 d9c497e0...`, final state hash `bef936ec026341a0`). The change from round 2 is the X_DEATH event kind and the overlay's own verbs, which is what the commit says.

### 10.4 Leaks on the new world
The same three leaks as round 2, planted again by me (X1 Hour step, probability about 1 in 4,000 per step), 30 years monthly at threads 1, 2, 4, 4 × 3: none 0 of 11; `HashMap` order 11 of 11; wall clock 11 of 11; thread identity 9 of 11 (the two equal ones are the 1-thread repeats). PR-shaped (100 years yearly, 1 vs 4 threads): all three differ. Counters: 21,900 parallel dispatches in 30 years at 4 threads, 43,801 thread ids seen. The Developer's `--leak-thread` also fails 1-vs-4 at rung 1.

### 10.5 Status
No open defect. Open gaps, all backlog: GAP-2 (the remainder), GAP-5, and the accepted residuals: `trap - EXIT` (10.1), the renamed `include_str!` and the `use include::X;` false positive (OBS-6, OBS-7), and `check_baseline.py` accepting a baseline whose header carries `inject` or `leak thread` when the recomputation is the same file (OBS-8). Not run in round 3: CI on a runner, the cross-machine check, the full nightly loop, and the bisect cases (unchanged code; the round-2 33 cases stand).
