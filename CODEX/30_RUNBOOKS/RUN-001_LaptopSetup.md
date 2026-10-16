---
id: RUN-001
title: "Set up the development laptop (Ubuntu 24.04, ThinkPad P51)"
type: how-to
status: APPROVED
owner: architect
agents: [all]
tags: [infrastructure, workflow]
related: [GOV-008, SPR-001, GOV-002]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** Six steps take the director's laptop from a clean Ubuntu 24.04 to running `bin/check_all.sh` green: system packages, Rust, the Cargo tools, Python through `uv` (Ubuntu blocks `pip` into the system Python), a first check run, and benchmark hygiene. Phase 1 needs no GPU. About 20–30 minutes, mostly compiling the Cargo tools.

# RUN-001 — Laptop setup

## 0. The machine (D6, 2026-10-16)

| Part | Spec | Consequence |
|---|---|---|
| CPU | Intel i7-7820HQ, 4 cores / 8 threads, 2.9–3.9 GHz (2017) | Thread ladder runs at 1/2/4/8. Roughly 3× slower than the reference desktop in `08` (an estimate; SPR-009 measures it) |
| RAM | 32 GB (about 18 GiB free in normal use) | Enough for P0–P2 and single 1M runs; not for many parallel 1M rollouts |
| GPU | Quadro M2200, 4 GB, Maxwell; driver 580 | Not needed in phase 1. Driver 580 is the last branch for Maxwell, and 4 GB is too small for the planned local LLMs; LLM work goes to the cloud or a later machine |
| Disk | 512 GB NVMe | Keep ≥ 100 GB free: Rust build directories grow to 10–30 GB; a 100k world is 6–18 GB per 1,000 years |

## 1. System packages
```bash
sudo apt update
sudo apt install -y build-essential pkg-config git curl valgrind python-is-python3
```
`valgrind` is for instruction-count benchmarks (§6). It is GPL and runs only as an outside tool, which ADR-0021 allows.

## 2. Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup component add rustfmt clippy
```
SPR-001 pins the exact Rust version in `rust-toolchain.toml`; `rustup` installs it automatically on first build.

## 3. Cargo tools (SPR-001 needs the first two)
```bash
cargo install --locked cargo-nextest cargo-deny
# Later sprints: cargo install --locked cargo-mutants   (nightly mutation testing)
#                cargo install --locked cargo-fuzz       (needs the nightly toolchain)
```

## 4. Python
```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
cd ~/Rehoboam                      # your clone
uv venv .venv --python 3.12
uv pip install --python .venv/bin/python pydantic pyyaml duckdb
```
SPR-001 replaces the last line with `pyproject.toml` and a committed `uv.lock`.

## 5. First check run
```bash
bin/check_all.sh                   # finds .venv/bin/python automatically
less artifacts/check_report.md
```
Expect `Verdict: PASS (12 checks)`. If a check fails, the report shows the first failure's output; file it as a `DEF-` only after the run is reproducible.

## 6. Benchmark hygiene (laptop-specific)
- **Plug in to AC power** and select the performance profile: `powerprofilesctl set performance`.
- **Close heavy applications** during benchmark runs; keep the vents clear. Sustained all-core loads throttle a mobile CPU.
- **Wall-clock timings on this laptop vary by more than 10% between runs.** The per-PR performance gate therefore uses **instruction counts** (deterministic, via `iai-callgrind`), and wall-clock benchmarks run nightly with repeated samples (GOV-002 tier 10, SPR-009).
