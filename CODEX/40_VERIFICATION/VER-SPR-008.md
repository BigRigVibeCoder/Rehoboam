---
id: VER-SPR-008
title: "Audit of SPR-008"
type: record
status: DRAFT
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-008, VER-SPR-008-TEST, VER-G0, BCK-002]
created: 2026-10-16
updated: 2026-10-16
version: 0.1.0
---

> **BLUF:** DRAFT; the verdict waits for the Tester's round 2. SPR-008 gives the engine its golden-run evidence:
> - a smoke world of seven dummy systems that exercises every phase, every pulse, timers, commands, causal edges, an `rh-sample` Poisson draw on both of its code paths, and parallel scheduler levels;
> - hash ladders that compare byte for byte across machines, with a yearly PR tier, a monthly nightly tier and a cross-machine procedure;
> - a bisect that names the month, table, row and column of a divergence, and runs automatically when CI's golden check fails;
> - a baseline that can only be regenerated in a C3 change, never typed.
>
> Round 1 found that the thread-count invariance this sprint exists to prove was vacuous (DEF-2): the smoke world never ran two jobs in one level, so a planted thread-identity leak went unseen. It is fixed. My audit found the baseline gate failing open with no base (MAJ-1) and four minors. All are fixed. Three items go to the backlog (A-36 to A-40). Change class C3. The Architect merges (GOV-005 §6).

# VER-SPR-008 — Audit of SPR-008

Commit SHAs are as reported during the sprint. The branch is rebased onto `main` before the merge and is rebase-merged, so the SHAs on `main` differ. Each commit keeps its subject line, so `git log --grep` finds it.

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh` | Round 1: PASS (187 checks), Developer and Tester (clean archive of `29abe04`). Round 2: PASS (210 checks), Developer at `8e17816`; the Tester's clean archive stopped at 181 of 187 with an absolute `CARGO_TARGET_DIR` (DEF-3), and passed 210 of 210 with that line patched. Final: PASS (226 checks) at `6a87616`, with relative and absolute target dirs | Tester round 3: *pending* |
| CI on GitHub-hosted runners | Round 1: PASS at `29abe04` (run 38931650691, 11 min 28 s, under the 15-minute budget). Round 2: no run on `8e17816`, because PR #28 conflicted with main in `05.1`'s changelog; then FAIL at `cac67de` (run 38955663828), with every golden gate passing and only pytest failing: a test read CI's own `RH_*` variables. Final: **PASS at `6a87616`** (run 38962222586, 7 min 36 s) | PR #28 |
| Change class | C3: creates the golden corpus. The baseline was regenerated in `[C3]` commits only, the last one after every change to the smoke world | — |

## 2. Tester verdicts
| Round | Head | Result |
|---|---|---|
| 1 | `29abe04` (report `058ff35`) | T-00 PASS; T-01 PASS with DEF-2; T-02 PASS with DEF-1 and DEF-2; T-03 PASS; T-04 PASS with GAP-2. Evidence: `include!` evasions 67 of 69 refused (the 2 others were the Tester's own wrong expectations); 66 of 66 random and edge bisect cases plus 8 shape cases named exactly; CI's automatic bisect on a planted change named month 1, `events.meta`, row 0, byte 18; resume equal to continuous at 6 cuts; ladders equal across 1–64 threads and debug, release, native and fma builds; planted HashMap-order and wall-clock leaks caught 11 of 11; the baseline gate holds; `--inject` cannot reach a baseline |
| 2 | `8e17816` (report `b62a795`) | **DEF-2 fixed:** real parallel dispatch, with 21,900 two-job levels in 30 years at 4 threads and 43,801 distinct thread ids (before: 0 and 1); `--require-parallel 5` passes, and 8 and 99 fail. The Tester's own leaks: thread identity caught 9 of 11 (the other 2 are the 1-thread repeats, correctly equal), HashMap order 11 of 11, wall clock 11 of 11. **DEF-1 fixed:** a mismatched horizon exits 2 in both directions. MAJ-1, GAP-4, GAP-6 and MIN-1 to MIN-3 pass; the bisect names 33 of 33 on the new world; the nightly's yearly cut equals the committed baseline. GAP-3 is fixed, with GAP-7 (`\|\| exit 0`) found. New: DEF-3 (an absolute `CARGO_TARGET_DIR` broke a baseline control) and GAP-8 (a cancelled push run dropped its range) |
| 3 | `6a87616` | *pending* |

## 3. Rulings (Architect)
| # | Question | Ruling |
|---|---|---|
| Pre-sprint 1–9 | The first system crate, dummy systems, coverage, the runner and ladder, tiers, baseline integrity, bisect v0, cross-machine, order | SPR-008 (Architect rulings, 2026-10-16) |
| T-01 | A stream accessor for `rh-sample` draws | `StepCtx::stream(purpose, entity)`, the same key as `rng`, positioned at draw 0. A pair is handed out as a stream at most once per step and never alongside `rng` draws for it; `SimError::StreamReuse` otherwise. A timer pass is one step for the rule (`05.1` §2 rule 6) |
| Done report | Deviations 1–8 | Accepted. Inputs journaled up front (consistent for P0; the contract fix is BCK-002 A-36). A per-rung `months` digest. Synthetic inputs as `OracleMove` rows from `Producer::System`. The PR-tier golden run as T-01's 1,000-year acceptance. A tool crate `rh-golden`. The `--inject` hook. Column equals table for world tables. A 30-minute CI timeout (A-38 restores 20) |
| Audit | MAJ-1, MIN-1 to MIN-4 | Below |
| Round 1 | DEF-1, DEF-2, GAP-1 to GAP-6 | Below |
| Round 2 | Merge route | The branch carries merge commits from `main` (the Developer's session refused a force push, and the merge was content-identical to the rebase), so PR #28 merges with a merge commit, a one-off against GOV-005's rebase-merge, recorded here |
| Round 2 | The smoke world under SPR-011 ruling 11's `[[emits]]` lint (main) | The smoke overlay declares the world's own verbs, and X3 emits `X_DEATH`, not M2's `DEATH`: dummy systems emit dummy kinds. The history changed, so the baseline was regenerated in the final `[C3]` commit |

## 4. Defects and gaps: disposition
| Item | Disposition |
|---|---|
| **MAJ-1** (Architect audit) the baseline gate's C3 rule failed open with no base: a push to main, `workflow_dispatch` and every local run counted a changed baseline as "unchanged" | Fixed (`4a55942`, rebased): a PR event with no base exits 2. On a push, the range `before..after` is checked, and a changed baseline needs a commit subject ending in `[C3]` within it. A local run with no base reports "C3 rule not checked". `[C3]` counts only in the title's trailing tag group, and `edited` re-runs the gate. `regen_baseline.sh` moved out of `CLAUDE.md`'s checks. Controls both ways, plus 22 unit tests over real git repositories |
| MIN-1 the golden run covered only Poisson's inversion path | Fixed: one region at λ = 12, so PTRS runs too. The README lists the sampler paths covered (binomial, normal, gamma and beta are covered by V-DET-09's vectors) |
| MIN-2 an injected ladder looked clean | Fixed: an `inject M:TABLE:ROW` header line; `compare` reports `Difference::Inject` |
| MIN-3 the include lint resolved symlinks differently from rustc | Fixed: includes resolve from the file as reached. Also refused: a renamed include macro (`use core::include as inc`), and `include_str!`/`include_bytes!` other than one literal naming a file inside the repository. Fixtures both ways |
| MIN-4 the auto-bisect could not run on a push (depth-1 checkout) and its grep matched unrelated rows | Fixed: on a push the base is `before`; only the `Golden run`, `Golden ladders` and `Golden baseline` FAIL rows trigger it |
| **DEF-1** (low) a resumed ladder took `years` from the flag, not from the journal | Fixed (`6bfce56`, rebased): the header carries `journal <inputs> <head chain>`. `--resume` refuses a horizon that disagrees with the journal, or one past its last input unless `--extend` is given. `--until` stops early without changing the history |
| **DEF-2** (medium; a G0 criterion) the smoke world never formed a two-job level, so thread-count invariance was vacuous | Fixed (`93b100c`): a seventh system, X7 `smoke_market`, shares the Environment level with X1 and the Activity level with X4 every day, and exchanges commands with X4. That gives 730 two-job levels a year (7% of 10.3 M levels over 1,000 years). `run smoke` prints a `dispatch:` line, and `--require-parallel 5` fails a run with no parallel level or under 5%; it runs in check_all, the nightly and `regen_baseline`. A planted `--leak-thread` control fails the 1-versus-N comparison at rung 1 |
| GAP-1 queued future inputs are in `state_hash`, so the hashes depend on the horizon | P0 accepted (inputs journaled up front). Contract fix: BCK-002 A-36, the journal records each row's acceptance cursor |
| GAP-2 `[C3]` substring match; a title edit was not re-checked; gate files editable without `[C3]` | First two fixed with MAJ-1. The third is BCK-002 A-39 |
| GAP-3 `check_all.sh` exited with the last command's status, so a `\|\| true` gate stayed green | Fixed (`ea2be7d`): the verdict tests `$failed`; `bin/check_all_selftest.py` runs the real `run()` on a planted chain; the inventory refuses `\|\| true` and `\|\| :` on `run` lines |
| GAP-4 `ladder compare` accepted CRLF and a missing final newline | Fixed (`ea2be7d`): strict parse, with controls |
| GAP-5 an event-only divergence named an `events.meta` byte | BCK-002 A-40 |
| GAP-6 the nightly did not compare with the baseline | Fixed (`ea2be7d`): `ladder cut-yearly`, and the nightly compares its cut with the committed baseline |
| Found by check_all in round 2 | `def1_resume` reused keyframes cached by an earlier run. Fixed: the cache is keyed to the run |
| **DEF-3** (low-medium; Tester round 2) `check_all.sh` joined `$PWD/$GOLDEN`, which broke with an absolute `CARGO_TARGET_DIR` | Fixed: `GOLDEN` is absolute once. `check_all_selftest.py abs-target` runs the real baseline controls with an absolute target, and a planted old join fails |
| GAP-7 (Tester round 2) `\|\| exit 0` evaded the lint and ended the tier green with no verdict | Fixed (`211f0e6`): the verdict runs in an `EXIT` trap and passes only when the chain reached its end; the lint refuses `\|\| exit [N]`; controls fail as they should |
| GAP-8 (Tester round 2) a cancelled push run on main dropped its baseline range | Fixed (`35b21f2`): `cancel-in-progress` only for pull requests |
| Found by CI in round 2 | `test_check_baseline` read CI's own `RH_*` variables, so it passed locally and failed on CI | Fixed (`750ce32`): an autouse fixture clears them, and one test sets them on purpose |

**Escapes found later:** none so far.

## 5. Acceptance criteria
| Task | Criterion | Met |
|---|---|---|
| T-00 | `lint_forbidden` follows a literal in-repository `include!`; the `real_specs` escape is removed | Yes (round 1; MIN-3 in round 2) |
| T-01 | The smoke world exercises every phase and pulse, timers, commands, causal edges and an `rh-sample` draw; 1,000 years headless | Yes; parallel levels since DEF-2 |
| T-02 | V-DET-01/02: identical plane-A hashes at 1, 2, 4 and N threads, 3 repeats, every month for 1,000 years | PR tier yearly at 1 and N; nightly monthly at 1/2/4/N × 3. *Round-2 confirmation pending* |
| T-03 | V-DET-11: the bisect names the injected month, table and row | Yes (66 of 66 plus 8 shapes) |
| T-04 | A hand-edited baseline fails; a regenerated one outside a C3 change fails | Yes, on every event since MAJ-1 |

## 6. Verdict
| Field | Value |
|---|---|
| Verdict | *pending the Tester's round 2* |
| Merge route | The Architect merges (GOV-005 §6) |
| Carried forward | BCK-002 A-32 (a persistent worker pool; DEF-2 measured the cost of a spawn per level), A-36 (journal acceptance cursor), A-37 (`stream()` guard scope and its cost), A-38 (bisect in its own job), A-39 (gate edits need `[C3]`), A-40 (the bisect names the event). Not run: the cross-machine check (the director's session) |
