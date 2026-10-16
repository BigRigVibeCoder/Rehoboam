---
id: RUN-002
title: "The benchmark and heavy-run VM (adjutant, Google Cloud)"
type: how-to
status: APPROVED
owner: architect
agents: [all]
tags: [infrastructure, workflow]
related: [GOV-008, RUN-001, SPR-008, SPR-009]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** The director's Google Cloud VM `adjutant` is our reference-class machine: 8 cores and 64 GB, like the reference desktop in `docs/08`. It runs the wall-clock benchmarks, the nightly cross-machine determinism check (AMD, against the laptop's Intel), and later the heavy runs (100k to 1M persons, ensembles). It costs about $0.83 per hour while running, so it runs only on a schedule or on demand and stops itself afterwards. Nothing in P0 needs it before SPR-008.

# RUN-002 — The benchmark and heavy-run VM

## 0. The machine (director, 2026-10-16)

| Part | Spec | Consequence |
|---|---|---|
| Type | `c3d-standard-16` in `us-central1-a`: 16 vCPU = **8 physical cores** (AMD EPYC Genoa, Zen 4), 64 GB DDR5 | The closest machine we have to the `08` reference desktop (8 fast cores, 64 GB). Its per-core clock is lower (about 3.3–3.7 GHz against about 4.5), so SPR-009 measures the ratio rather than assuming it |
| Disk | 100 GB `pd-ssd` | Enough through P1a. A 100k world is 6–18 GB per 1,000 years; grow the disk before 1M runs or ensembles |
| OS | Ubuntu 24.04, the same as the laptop | One runbook for both (RUN-001 §1–4) |
| CPU vendor | AMD (the laptop is Intel; GitHub's hosted runners vary) | A stronger cross-machine determinism check than two Intel machines (SPR-008) |
| Cost | About $0.83/h while running; the persistent disk is billed whether or not the VM runs | A nightly hour is about $25 a month; see §4 |
| Reach | Over the director's Tailscale network, or GCP IAP | Cloud agent sessions **cannot** reach it. Agents use it only through GitHub Actions (§3) |

Connection details (project, addresses, scripts) stay in the director's own notes, not in this repository.

## 1. Start and stop (director)
```bash
gcloud compute instances start adjutant --zone=us-central1-a --project=<project>
gcloud compute instances stop  adjutant --zone=us-central1-a --project=<project>
```
Always stop it when idle. §3 adds an automatic start and stop for nightly runs.

## 2. One-time setup on the VM
Run RUN-001 §1–5 on the VM (system packages, Rust, Cargo tools, Python through `uv`, a first `bin/check_all.sh`). Skip RUN-001 §6: a VM has no power profile, but keep its benchmark rules (§5 below).

## 3. Nightly runs through GitHub Actions (set up in SPR-008)
1. **Self-hosted runner.** Register a GitHub Actions runner on the VM for this repository, as a service under an unprivileged user, with the label `adjutant`. Self-hosted runners use no GitHub Actions minutes, which matters on the Free plan (GOV-008 §1).
   - Safe because the repository is private. **If the repository ever becomes public, remove the runner first**: a public repository lets anyone's pull request run code on it.
2. **Automatic start and stop.** A Compute Engine **instance schedule** starts the VM shortly before the nightly workflow and stops it two hours later. This needs no keys stored in GitHub.
   - The nightly workflow (`runs-on: [self-hosted, adjutant]`) waits in GitHub's queue until the runner comes online.
   - The schedule's stop time is the backstop: a hung job cannot run up the bill past it.
3. **Results** go back as workflow artifacts: the bench report, the hash ladders and the check report.

## 4. Cost discipline
| Use | Hours a month | About |
|---|---|---|
| Nightly runs (one hour a night) | 30 | $25 |
| Benchmark sessions on demand | 5–10 | $4–8 |
| Persistent disk (100 GB SSD) | always | Billed whether or not the VM runs; check your billing page |

The monthly ceiling is the director's (`decisions_pending.md`, D6). Default: **$50 a month through P1a**. The Architect reports actual spend at each gate.

## 5. Benchmark rules on a VM
- Record the machine type, CPU model (`lscpu`) and kernel in every bench report; a resized or migrated VM gets a new baseline, never a silent comparison.
- At least 5 repeated runs per wall-clock bench; report the median and spread.
- Wall-clock results gate only on this machine, and only after SPR-009 has measured its run-to-run spread. The per-PR gate stays on instruction counts (GOV-002 tier 10).
