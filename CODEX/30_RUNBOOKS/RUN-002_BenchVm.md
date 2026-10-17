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
version: 1.2.1
---

> **BLUF:** The director's Google Cloud VM `adjutant` is our reference-class machine: 8 cores and 64 GB, like the reference desktop in `docs/08`. It runs the wall-clock benchmarks, a cross-machine determinism check (AMD, against the laptop's Intel), and later the heavy runs (100k to 1M persons, ensembles). It costs about $0.83 per hour while running, so it runs **only on demand** (D6): the Architect asks for a session in advance, the director starts the VM, and it shuts itself down after 30 idle minutes. The director manages the spend. The first session is in SPR-009; regular use starts in P1b.

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

## 3. Sessions through GitHub Actions (set up in SPR-009)
1. **Self-hosted runner.** Register a GitHub Actions runner on the VM for this repository, as a service under an unprivileged user, with the label `adjutant`. Self-hosted runners use no GitHub Actions minutes, which matters on the Free plan (GOV-008 §1).
   - Safe because the repository is private. **If the repository ever becomes public, remove the runner first**: a public repository lets anyone's pull request run code on it.
2. **Jobs wait for the VM.** Workflows for the VM (`runs-on: [self-hosted, adjutant]`) are started by hand (`workflow_dispatch`) or by the Architect. They wait in GitHub's queue until the VM is running.
3. **Idle shutdown.** A small systemd timer on the VM powers it off after 30 minutes with no runner job and no interactive login. Any login blocks the shutdown, an idle SSH or IAP session included: log out when done. `IDLE_MINUTES` overrides the 30; a value that is not a positive integer is logged and ignored. This is the backstop that keeps a forgotten VM from running up the bill.
4. **Results** go back as workflow artifacts: the bench report, the hash ladders and the check report.

### 3.1 Setting it up (SPR-009 T-04; the director, once)
The workflow is `.github/workflows/adjutant.yml` (`workflow_dispatch` only, never pull requests). On the VM, after RUN-001 §1–5 (which installs valgrind and `iai-callgrind-runner`):
```bash
# 1. The runner: Settings → Actions → Runners → New self-hosted runner (Linux x64) gives the download
#    and a registration token. Register it under an unprivileged user with the label adjutant, then
#    install it as a service:
./config.sh --url https://github.com/BigRigVibeCoder/Rehoboam --labels adjutant --unattended --token <token>
sudo ./svc.sh install && sudo ./svc.sh start
# 2. The idle shutdown (§3 item 3), from a checkout of this repository:
sudo install -m 0755 infra/adjutant/idle-shutdown.sh /usr/local/sbin/rehoboam-idle-shutdown
sudo install -m 0644 infra/adjutant/rehoboam-idle-shutdown.service infra/adjutant/rehoboam-idle-shutdown.timer /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now rehoboam-idle-shutdown.timer
```
A session: start the VM, then run the `adjutant` workflow (Actions → adjutant → Run workflow). It runs `bin/check_all.sh` (the AMD cross-check) and `bin/bench_session.sh adjutant` (5 separate wall-clock runs of benches 7 and 8 at 100k and 1M, with the run conditions and the run-to-run spread). It then compares the session with `crates/rh-bench/baselines/wall_adjutant_v1.txt` once that exists, and uploads everything. The VM powers itself off 30 minutes after the job ends.

## 4. When the VM is needed
The Architect asks in advance, saying what the session is for and roughly how long it takes (BCK-002 A-12).

| When | What for | About |
|---|---|---|
| SPR-009 | Wall-clock baselines; the VM's and the laptop's ratios to the `08` reference | 1–2 h, once |
| Before each gate from G0 | Cross-machine determinism on AMD; wall-clock trend | 1 h |
| P1b onward | 100k-person, 1,000-year runs; liveness gate and certification ensembles | Several hours per session |
| P3b | 1M runs, Foresight batches | To be planned then |

## 5. Benchmark rules on a VM
- Record the machine type, CPU model (`lscpu`) and kernel in every bench report; a resized or migrated VM gets a new baseline, never a silent comparison.
- At least 5 repeated runs per wall-clock bench; report the median and spread.
- Wall-clock results gate only on this machine, and only after SPR-009 has measured its run-to-run spread. The per-PR gate stays on instruction counts (GOV-002 tier 10).
