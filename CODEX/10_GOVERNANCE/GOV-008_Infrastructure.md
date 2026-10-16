---
id: GOV-008
title: "Infrastructure and operations: a local Linux desktop first, the cloud later"
type: reference
status: APPROVED
owner: human
agents: [all]
tags: [governance, infrastructure]
related: [GOV-007, ADR-0030, ADR-0021]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** Development and play happen on the director's 8-core Linux desktop (D5, D6). There is no deployed service: the engine runs headless on localhost and viewers attach to it. CI runs on Linux. The move to the cloud comes after local testing is complete, on x86-64 Linux instances, and is planned in this document when it happens.

# GOV-008 — Infrastructure and operations

## 1. Decisions in force

| Topic | Decision | Source |
|---|---|---|
| Operating system | Linux (development, play, CI) | D5 |
| Machine | 8-core desktop; RAM and GPU to be stated before P3 (default 32–64 GB) | D6 |
| Cloud | After local testing is complete; x86-64 Linux, to match the desktop. Budget to be set then | D6 |
| Repository | One monorepo: `BigRigVibeCoder/Rehoboam` (private while it holds copyrighted papers) | — |
| Network exposure | localhost only in phase 1; no service listens on a public interface | ADR-0030 |
| Cross-platform matrix | Optional; Windows and macOS are not targets | D5 |

## 2. Repository layout (planned at P0.1)

```
Cargo.toml (workspace)   crates/rh-*          Rust engine and systems
python/                  rh-py, compiler, gateway, tools
console/                 (after G2) TypeScript web console
specs/                   executable contracts
worlds/                  reference manifests and golden baselines (managed by the merge queue)
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

Rust stable (pinned via `rust-toolchain.toml`); Python ≥ 3.11 with a lockfile; DuckDB ≥ 1.5; `cargo-deny`, `cargo-nextest`, `cargo-mutants`, `cargo-fuzz`.

## 5. When the cloud move happens

Before migrating, write an adaptation table here covering:
- instance type;
- storage for streams and keyframes;
- CI runners;
- secrets for LLM APIs;
- cost ceiling;
- cross-machine determinism check (V-DET).

The move is a C2 change, and it is approved by the director.
