# RED TEAM OPERATIONAL HANDOFF: ACTIVATING `ADJUTANT` (GCP SPOT)
## Cloud Benchmark, Cross-Machine Determinism & Heavy-Run Verification

> **Document ID:** `HANDOFF-2026-10-17-ADJUTANT-RED-TEAM`  
> **From:** Red Team Leader (ThinkPad P51 Local Session)  
> **To:** Red Team Auditor (Google Cloud VM `adjutant` Antigravity Session)  
> **Date:** 2026-10-17  
> **Status:** Active Execution Briefing  
> **Target Machine:** Google Cloud `c3d-standard-16` (AMD EPYC Genoa Zen 4, 16 vCPU / 8 physical cores, 64 GB DDR5, 100 GB pd-ssd)  
> **Git Tracking:** Branch `arch/red-team-batch-04` (pushed to `origin`, HEAD `8074abe`)  

---

## 1. Executive BLUF: Mission for `adjutant`

Welcome to `adjutant`. You are the Red Team instance running on the Director's reference-class heavy-run cloud instance. 

The local ThinkPad P51 workstation has completed all requirements specifications, dashboard upgrades, and Phase 1a alignment. Your mission on `adjutant` is threefold:

1. **Verify Cross-Machine Bitwise Determinism (`SPR-008` T-02):**  
   Prove that Plane A produces **bitwise identical SHA-256 state hashes** on AMD EPYC Genoa as it does on Intel Core i7 (ThinkPad P51), confirming our strict IEEE-754 float prohibition (`0.000` floats) and fixed-point math (`Q8_8`, `Q16_16`, `Q32_32`).
2. **Establish Server-Class Benchmark Baselines (`SPR-009` T-04):**  
   Run `rh-bench` across the 8 physical Genoa cores to establish our server-class allocation and throughput baselines.
3. **Audit Simulation Emergence via Visual Telemetry:**  
   As the Red Team, **visual observability is our #1 testing priority**. We need to SEE the simulation across rungs 0–4 (solo, household, hamlet, village, kingdom) to audit spatial clustering, settlement growth, demography, and dynastic succession without being blinded by raw JSON dumps.

---

## 2. Bootstrapping Your Session on `adjutant`

All work from the laptop has been committed, cleanly linted, and pushed to GitHub:

```bash
# 1. Navigate to the repository checkout
cd ~/Documents/Rehoboam  # or the directory where Rehoboam is cloned

# 2. Fetch and checkout the active branch
git fetch origin
git checkout arch/red-team-batch-04
git pull origin arch/red-team-batch-04

# 3. Confirm clean working tree and manifest
python3 -I bin/gen_manifest.py --check
# Expected output: MANIFEST: builds cleanly (193 documents, 0 errors)

# 4. Launch the local Red Team HTTP and WebSocket services
python3 -m http.server 8088 &
python3 bin/stream_server.py --port 8089 &
```

Once running, the Red Team Command Dashboard is live at `http://localhost:8088/docs/red_team/dashboard.html`.

---

## 3. Current Monorepo State & Architecture Invariants

| Dimension | Ground Truth Status |
|---|---|
| **Phase 0 Status** | **10/10 Sprints Merged & Closed.** Formal G0 Certification ratified (`VER-G0`). |
| **Monorepo Size** | **14 Crates · 380 Rust files · 48,904 LOC** (160,924 total lines across specs and cleanroom tests). |
| **Fail-Fast Gates** | **149 validation gates** enforced in `bin/check_all.sh`. |
| **Automated Tests** | **673 tests** (421 Rust unit/integration + 252 Python AST reference verifiers). |
| **Kernel Float Ops** | **0.000** (100% fixed-point math). AST purity compiler traps prevent hardware floats. |
| **Phase 1a Progress** | Walking skeletons A, B, and C merged. Wave 4 approved by Director (`SPR-014` lifecycle queues, `SPR-020` event envelope, `SPR-026` World Stream v1). |

---

## 4. Key Decisions & Rulings (Recorded 2026-10-17)

- **D37 (Approved):** Registry effect language v0 (PR #41). Unblocks M7 succession.
- **D46 (Approved):** Population synthesis v0 deferral (PR #67). Iterative proportional fitting deferred to P1b for class/faith/culture.
- **D48 (Approved):** Wave-4 sprint plans (SPR-014, SPR-020, SPR-026).
- **D49 (Approved):** Plane-C deterministic person names (`H(seed, person_id, "name")`). No name strings in Plane A state hash.
- **D50 (Approved):** V-NAR-05 wording amended for annals prologue keyframe rows.
- **CI Skips (Approved & Merged in PR #70):** Docs-only PRs skip golden half of CI; nightly skipped when zero commits landed on `main`.
- **D51 (Ratification In Progress):** Amending ADR-0035 and ADR-0030 to authorize the **Native Bevy 0.20 Voxel Viewer (`rh-view`)** as a Plane-B hydration harness.

---

## 5. The Voxel Viewer Specification (`PRJ-003` & `SPR-033`)

We have authored and committed the formal CODEX blueprints:
* **Master Plan:** [`CODEX/05_PROJECT/PRJ-003_BevyVoxelViewer.md`](file:///home/bdavidriggins/Documents/Rehoboam/CODEX/05_PROJECT/PRJ-003_BevyVoxelViewer.md)
* **Kickoff Sprint:** [`CODEX/05_PROJECT/SPR-033_BevyVoxelSpike.md`](file:///home/bdavidriggins/Documents/Rehoboam/CODEX/05_PROJECT/SPR-033_BevyVoxelSpike.md)

### The Division of Labor:
* **The Developer (`AGT-002-TOOLS`) Writes the Code:** The Developer implements `crates/rh-view` in Bevy 0.20, writes the binary greedy mesher ($<3\text{ ms}$ per 64×64×32 chunk), and connects the Unix domain socket reader to `rh-stream`.
* **The Red Team Audits & Tests:** The Red Team uses `rh-view` as our **primary visual inspection harness**. We audit visual parity, measure simulation non-interference (V-VIEW-01: $\le 0.5\%$ delta), and verify timeline scrubbing latency (V-VIEW-05: $\le 250\text{ ms}$).

---

## 6. Verification Tasks to Execute on `adjutant`

Run these three commands to immediately validate the heavy cloud environment:

```bash
# Task 1: Full PR-Tier Validation Suite (all 149 gates)
bin/check_all.sh
# Expected: artifacts/check_report.md generated with PASS on all tiers.

# Task 2: Golden Ladder Cross-Machine Hash Parity Check
cargo test -p rh-golden -- --nocapture
# Expected: Identical SHA-256 hashes for smoke and solo worlds on AMD as Intel.

# Task 3: Server-Class Benchmark Suite
cargo bench -p rh-bench
# Expected: Record Zen 4 wall-clock baselines for Benches 7 and 8.
```

---

## 7. Shutdown Notice (Cost Management)

Remember that `adjutant` runs at **$0.83/hour** on demand (D6 / RUN-002). 
When finished with your testing session:
1. Ensure all new findings or reports are committed to `arch/red-team-batch-04` and pushed to `origin`.
2. Terminate background daemons (`killall python3`).
3. Notify the Director to stop the instance (`gcloud compute instances stop adjutant`).
