---
id: GOV-008
title: "Infrastructure and operations: a local Linux laptop first, the cloud later"
type: reference
status: APPROVED
owner: human
agents: [all]
tags: [governance, infrastructure]
related: [GOV-007, ADR-0030, ADR-0021, RUN-001, RUN-002]
created: 2026-10-16
updated: 2026-10-16
version: 1.2.0
---

> **BLUF:** Development and play happen on the director's Linux laptop, a ThinkPad P51 with 4 cores and 32 GB (D5, D6; set up by RUN-001). There is no deployed service: the engine runs headless on localhost and viewers attach to it. CI's PR tier runs on GitHub-hosted Linux runners; nightly and benchmark runs use the director's Google Cloud VM `adjutant` (8 cores, 64 GB; RUN-002), our reference-class machine. Coding agents run in cloud sessions. Performance targets stay quoted on the reference desktop of `docs/08`; the laptop is a development machine, not the yardstick. The move to the cloud comes after local testing is complete, on x86-64 Linux instances, and is planned in this document when it happens.

# GOV-008 — Infrastructure and operations

## 1. Decisions in force

| Topic | Decision | Source |
|---|---|---|
| Operating system | Linux (development, play, CI) | D5 |
| Development machine | ThinkPad P51 laptop: i7-7820HQ (4 cores / 8 threads, 2017), 32 GB, Quadro M2200 4 GB (Maxwell), 512 GB NVMe with 153 GB free, Ubuntu 24.04. Setup: RUN-001 | D6; Q-INFRA-3 |
| Benchmark and heavy-run machine | Google Cloud VM `adjutant`: `c3d-standard-16` (8 AMD Genoa cores, 64 GB DDR5), 100 GB SSD, Ubuntu 24.04; about $0.83/h while running. Runs only on a schedule or on demand and stops itself. Setup and rules: RUN-002 | Director, 2026-10-16 |
| Performance yardstick | The reference desktop of `docs/08` (8 fast cores, 64 GB). `adjutant` is the closest match and carries the wall-clock benchmarks; SPR-009 measures its ratio to the reference. The laptop is roughly 3× slower than the reference (an estimate). Targets are not lowered to fit either machine | `08` |
| Coding agents | Cloud sessions, not the laptop. They cannot reach the VM; they use it only through GitHub Actions | Q-INFRA-2 (director) |
| GPU | None needed in phase 1 (no GUI, no LLMs). The M2200 is not used for LLMs: 4 GB is too small and driver 580 is the last branch for Maxwell. LLM work (P3 onward) goes to cloud APIs or a later machine | D6 |
| Cloud | `adjutant` from SPR-008 for nightly runs and benchmarks. The full move (play and development in the cloud) comes after local testing is complete, on x86-64 Linux. Monthly ceiling: D6 (default $50 through P1a) | D6 |
| CI | GitHub Actions. **PR tier:** GitHub-hosted Ubuntu runners. **Nightly and benchmarks:** a self-hosted runner on `adjutant` (from SPR-008; no Actions minutes used). Cross-machine determinism compares the laptop (Intel), `adjutant` (AMD) and a hosted runner. No runner on the laptop: it would compete with development and its timings are noisy | This document; RUN-002 |
| GitHub plan | **Free** (director, Q-INFRA-1). A private repository on Free has no protected branches or required checks, and about 2,000 Actions minutes a month. So: the Architect enforces "no green `check_report.md`, no merge" by hand (`docs/13` §4.4); the PR tier stays lean; nightly work runs on `adjutant`. Revisit if a merge ever lands red | Q-INFRA-1 |
| Repository | One monorepo: `BigRigVibeCoder/Rehoboam` (private while it holds copyrighted papers) | — |
| Network exposure | localhost only in phase 1; no service listens on a public interface | ADR-0030 |
| Cross-platform matrix | Optional; Windows and macOS are not targets | D5 |

## 2. Repository layout (planned at P0.1)

```
Cargo.toml (workspace)   crates/rh-*          Rust engine and systems
python/                  rh-py, compiler, gateway, tools
console/                 (after G2) TypeScript web console
specs/                   executable contracts
worlds/                  reference manifests and golden baselines (changed only by approved C3 merges; `docs/13` §4.4)
docs/ research/ CODEX/   plan, evidence, operations
bin/                     check_all.sh, gen_manifest.py, developer scripts
artifacts/               reports, crash artifacts (git-ignored)
```

## 3. Data on disk

| Data | Location | Retention |
|---|---|---|
| World Stream sealed segments (the chronicle) | `runs/<world>/<timeline>/stream/` | Permanent for kept worlds; disposable for test runs |
| Keyframes | `runs/<world>/<timeline>/keyframes/` | 10-year permanent; yearly LRU (`08`) |
| Input journal | `runs/<world>/<timeline>/journal.sqlite` | Permanent: with the seed it *is* the world |
| Check and crash artifacts | `artifacts/` | Git-ignored; kept for 30 days |

**Backup rule:** a world is preserved by its manifest, seed, journal and engine build; everything else can be regenerated (ADR-0031 §1).

## 4. Toolchain (pinned at P0.1)

Rust stable (pinned via `rust-toolchain.toml`); Python 3.12 in a `uv` virtual environment with a committed `uv.lock` (Ubuntu 24.04 blocks `pip` into the system Python); DuckDB ≥ 1.5; `cargo-deny`, `cargo-nextest`, `cargo-mutants`, `cargo-fuzz`; `valgrind` for instruction-count benchmarks (GPL, run only as an outside tool, which ADR-0021 allows). Installation: RUN-001.

## 5. When the cloud move happens

Before migrating, write an adaptation table here covering:
- instance type;
- storage for streams and keyframes;
- CI runners;
- secrets for LLM APIs;
- cost ceiling;
- cross-machine determinism check (V-DET);
- re-measuring the `08` budgets on reference-class hardware, if the cloud machine is the first one that matches it.

The move is a C2 change, and it is approved by the director.
