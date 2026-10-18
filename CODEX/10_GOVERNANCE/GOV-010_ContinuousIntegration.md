---
id: GOV-010
title: "Continuous integration: the PR tier, the nightly tier and the rules CI enforces"
type: reference
status: APPROVED
owner: human
agents: [all]
tags: [governance, testing, infrastructure, workflow]
related: [GOV-002, GOV-005, GOV-008, GOV-009, GOV-011]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF:** CI runs the same `bin/check_all.sh` that agents run locally. On every pull-request event it runs as three parallel jobs (`core`, `golden`, `stream`), and each part must finish within 12 minutes (15 is the hard limit). Nightly, advisory and benchmark workflows run apart from PRs. CI reports but cannot block a merge on GitHub Free, so "no green report, no merge" is kept by the Architect (GOV-009 §6). Written on the director's instruction of 2026-10-18 from the workflows as they stand at `main` `3d26090`.

# GOV-010 — Continuous integration

## 1. Workflows (`.github/workflows/`)

| Workflow | Runs on | What it does | When it is red |
|---|---|---|---|
| `ci.yml`: the PR tier | PR `opened`, `synchronize`, `reopened` and `edited`; every push to `main`; manual dispatch | `bin/check_all.sh --part <p>` for `core`, `golden` and `stream` in parallel on `ubuntu-24.04`, each with a 20-minute timeout. Uploads `check-report-<part>` (the report, the licence ledger, ladders, perf counts, any bisect report) | The PR may not merge (GOV-009 §6). A red push run on `main` freezes C1–C3 merges |
| `nightly.yml` | 03:17 UTC daily; manual | The `#[ignore]` tier (V-DET-20 full sweep, sampler statistics) and the golden nightly (`bin/golden_nightly.sh`: monthly ladders at 1/2/4/N threads × 3, V-SCALE, V-INV-02, V-DET-11 × 64). Skipped when `main` has not moved since the last successful nightly (director, 2026-10-17) | Freeze C1–C3 merges until the cause is found (`docs/13` §4.4). A C3 sampler change needs it green before its merge (GOV-005 §6) |
| `advisories.yml` | 05:17 UTC daily; manual | `cargo deny check advisories` | Open a `DEF-` (or an `EVO-` to replace the crate). Does not block unrelated merges |
| `adjutant.yml` | Manual only | On the director's on-demand VM: `check_all` on AMD, wall-clock trends and instruction counts (RUN-002). **Never triggered by a PR** | Ask the director to start the VM first; the job waits in the queue until it is running |

- **Branch pushes alone do not run CI.** Open a draft PR (GOV-009 §4).
- **Concurrency:**
  - a newer push to a PR cancels that PR's older run;
  - push runs on `main` are never cancelled, so each one checks its own `before..after` range for the `[C3]` and `[perf-baseline]` rules.
- **Planned, not yet on `main`:** the viewer workspace gets its own workflow, scheduled twice a week (`43 4 * * 1,4`), in SPR-033 (PR #100). The two-week age rule there covers the viewer's direct dependency pins only.

## 2. The three parts and the scope rules

| Part | Holds |
|---|---|
| `core` | Lints, debug builds, workspace and by-name tests, Python (ruff, mypy, pytest), spec references and self-tests. **Always runs in full** |
| `golden` | The release build, golden runs with the vitals observer, baselines, ladders, the perf gates (valgrind), release-mode M2 and guard gates. A golden mismatch triggers the divergence bisect (`bin/golden_bisect_ci.sh`), whose report is attached |
| `stream` | The stream, serve, tail, sift, replay, rungs and twin gates, on their own run directories, in release with overflow checks |

- **Docs-only changes** (every changed file under `docs/` or `CODEX/`, or a `*.md` file) skip the golden and stream parts' work and report green in seconds. `bin/ci_scope.py` decides, and it fails closed: anything it cannot classify runs everything.
- **Path-scoped gates** (BCK-002 A-64): a few slow pure-Python references in `core` run only when one of their declared inputs changed. They run under `bin/ci_scope.py trace`, which fails a gate that reads an undeclared file (`SCOPE INCOMPLETE`).

## 3. Running it locally

- **`bin/check_all.sh`** runs all three parts in order; `--part core|golden|stream` runs one. It writes `artifacts/check_report.md` with the commit and the count of uncommitted files. **No report, no pass** (GOV-002 §6). Commit before running, so the report names a real commit.
- **Setup:** RUN-001 (the Rust toolchain from `rust-toolchain.toml`, `cargo-nextest`, `cargo-deny`, `valgrind`, `iai-callgrind-runner` 0.16.1, `uv` 0.11.32, then `uv sync --frozen --project python`). A missing tool fails the first check by name; nothing is skipped silently.
- **The pre-push hook** (`bin/install_hooks.sh`) runs the whole chain before every push. A WIP push to your own branch may use `--no-verify` (GOV-009 §3); nothing else may.
- **Disk:** one cargo `target/` takes 8–11 GB.
  - Run one cargo-building `check_all` at a time per machine.
  - Keep at least 5 GB free.
  - Delete `target/` directories you no longer need.
  - Point a second worktree at the first one's `CARGO_TARGET_DIR` only if they never build at the same time.

## 4. The rules CI enforces on history

| Rule | Checked by | How to satisfy it |
|---|---|---|
| A golden baseline under `worlds/` changes only in a C3 change | `bin/check_baseline.py` | The PR title's trailing tag group holds `[C3]`; on `main`, a commit in the pushed range whose subject ends with `[C3]` |
| A performance baseline changes only with a stated reason | `bin/perf_gate.py integrity` | `[perf-baseline]` in the PR title and a `perf-baseline-reason: <why>` line in the body; the commit's subject ends `[perf-baseline]` and its body has a `Reason:` line |
| Every gate is in the chain and in the inventory, in exactly one part | `bin/check_all_selftest.py inventory`, `python/tests/test_check_all_inventory.py` | Add a gate in the sprint that creates it: in `check_all.sh` (in its part), in `REQUIRED_GATES`, with its planted control under `tests/lint_fixtures/` that must FAIL, and its line in `CLAUDE.md` |
| A gate cannot be hidden | `bin/check_all_selftest.py verdict` | Never `\|\| true` a gate |
| The workflows keep their guarantees | `python/tests/test_ci_workflow.py` | Change the workflow and that test in the same commit |

## 5. Time and minutes

- **Budget:** each part at most **12 minutes** on a hosted runner; **15** is the hard limit (V-PERF-04); the job times out at 20. A full PR-tier run costs about 25–35 runner-minutes (for example SPR-017: core 7:21, golden 14:37, stream 5:17).
- **Over budget:** make the gate faster, or move gates between parts under the same names, with the inventory in the same commit. Never drop a gate.
- **Minutes are scarce** (GitHub Free; the PR tier ran 462 times in the week to 2026-10-18):
  - push in batches, not after every small edit;
  - use a docs-only PR for records;
  - do not re-run green jobs.
  - P1 work (D62) wins any contention for minutes.

## 6. Reading a result

- **Find the runs** for the PR's head: the check runs named `PR tier (bin/check_all.sh --part <p>)` (`gh pr checks <N>`). Download `check-report-<part>` (`gh run download <run-id>`) for per-gate times and the failing gate's tail.
- **A failure is a finding, not a flake.** The engine is deterministic, so a gate that passes and fails on the same commit is a defect to file. Re-run a job only when it died before any gate ran (runner loss, a download error), and at most once.
- **A failure that is also red on `main`** is not the PR's. Report it once in the PR and to the Architect, and port the fix if one exists. Never weaken the gate.
