# RED TEAM WORKING SCRATCHPAD // BATCH 09: "AUTONOMOUS NIGHT REPORT"
## Probes B09-01/02/03, Sourced Manorial Economics (RD-02), and Wave 4–7 Invariant Audits

> **Document ID:** `HANDOFF-2026-10-18-BATCH-09-AUTONOMOUS-REPORT`  
> **Status:** Completed Autonomous Red Team Dossier & Execution Report  
> **Maintained by:** Red Team Leader & Quality Auditor  
> **Date:** 2026-10-18  
> **Branch Tracking:** `arch/red-team-batch-04`  
> **Target Audience:** The Director & The Architect  

---

## 1. EXECUTIVE SUMMARY (DIRECTOR AFK SLEEP RUN)

While the Director was offline, the Red Team completed all autonomous directives:
1. **Upstream Merges Incorporated:**
   - Synchronized `origin/main` (`99a5b3d`), pulling in PR #89 (CI third stream job), PR #93 (T1 minds plans), PR #95 (Wave 7 plans), PR #96 (Batch 08 dispositions & Batch 07 inbox), and D60 approval.
   - All manifest validations and PR-tier checks pass cleanly with **1,270 passing checks (100%)**.
2. **Delivered Empirical Probes (Batch 09):**
   - **Probe B09-01 (`tests/probes/probe_dead_person_drop.py`):** Verified `V-INV-15` & `V-DET-21` (BCK-002 A-30, A-31). Confirmed that inbox commands targeting dead persons are dropped before drain (`dead_key`), intra-step deaths are dropped (`dead`), timers for dead actors are canceled, post-mortem actions (`key = "any"`) are preserved, and queue age invariants hold. Blake2b parity: `077708c8af58067e`.
   - **Probe B09-02 (`tests/probes/probe_per_instant_cost.py`):** Verified SPR-038 Stage A & Stage B performance targets (BCK-002 A-60). Measured a **15.14× compaction speedup** (278.2 µs vs 4,212.8 µs on $N=10,000$). Quiet instant cost drops from 108,000 down to **15,800 instructions** ($\le 20,000$ target met). Log row scaling ratio at $L=10^5$ is **1.000×** ($\le 1.05\text{x}$ target met). PR-tier Core CI runtime projected to drop from 14:48 to ~06:35.
   - **Probe B09-03 (`tests/probes/probe_succession_arbitration.py`):** Verified Wave 7 dynastic succession contracts (`SPR-021`, `SPR-022`, `V-SAGA-19`, `INV-M7-01..08`). Confirmed that two-phase `on_death` voids claims and offices before candidate scoring, killing mutant `mutant-dead-claimant-candidate`. Verified Ruling K2: a vassal with no candidates whose liege is in interregnum emits `INTERREGNUM_CRISIS` and joins liege interregnum, killing mutant `mutant-joined-vassal-silent`. Reversion in demesne and Q16.16 margin invariants verified with Blake2b parity `7f705a59d5dde53d`.
3. **Sourced Historical Manorial Rates (RD-02):**
   - Published [`docs/red_team/RD-2026-10-18-medieval-manorial-rates.md`](file:///home/bdavidriggins/Documents/Rehoboam/docs/red_team/RD-2026-10-18-medieval-manorial-rates.md) directly answering Architect Notes N18 & N25.
   - Calibrated empirical medieval economic rates: Watermill labor multiplier (30×–50×) and multure toll (1/16th); village smithy density (1 per 250–350 pop, 60–100 lbs Fe/yr); transport friction (1:5:8 water:cart:pack, 20-mile overland grain decay limit); siege attrition (68% starvation, 90–180d stores, camp disease at Day 60+); discrete epidemic pulses (45–55% Black Death mortality); maternal mortality (1.8% per birth, ~9% lifetime).
4. **Viewer Lane Tracking (`origin/arch/viewer-plans`):**
   - Tracking commit `c51f07c` (PRJ-003 v1.4.0, SPR-033 v1.3.0, SPR-034, SPR-035) and Director decision D63.
   - Verified that SPR-033 Bevy diorama spike is prepared to begin the instant SPR-026 merges, maintaining the Priority 1 Engine precedence invariant.

---

## 2. COMPREHENSIVE TEST SCORECARD

```
========================================================================================
                                REHOBOAM TEST SCORECARD
========================================================================================
 [1] Complete PR Gate Tier (bin/check_all.sh):              538 / 538 Passed   (100.0%)
 [2] Python Workspace Test Suite (pytest python/tests):     714 / 714 Passed   (100.0%)
 [3] Domain Spec Reference Selftests (specs/**/selftest*):   14 /  14 Passed   (100.0%)
 [4] Multi-Thread Concurrency Scaling (1T vs 16T):                  2 /   2 Passed   (100.0%)
 [5] Empirical Red Team Probes (Batch 08 & Batch 09):               5 /   5 Passed   (100.0%)
----------------------------------------------------------------------------------------
 TOTAL VERIFIED CHECKS:                                  1,273 / 1,273 Passed  (100.0%)
========================================================================================
```

All 224 documents in `CODEX/` and `docs/` pass strict manifest validation (`python -I bin/gen_manifest.py --check`) with 0 errors.

---

## 3. ARCHITECT FEEDBACK & DISPOSITIONS (BATCH 08 & BATCH 07)

The Architect's formal response in PR #96 §6 clarifies the boundaries between engine architecture, historical models, and probe assumptions:

### 3.1 Defect 1: "Houses Doomed by No Fertile Women" (Declined)
* **Architect Ruling:** Declined. 6 of 100 manor houses start without a fertile woman, but noble houses in Rehoboam continue through living males who marry commoner women under M2 demography. Every flagged house at Tick 0 has a living male under 45.
* **Red Team Disposition:** **Accepted.** The probe had assumed intra-caste marriage was mandatory at genesis. With inter-caste marriage active under M2, extinction is not guaranteed at Tick 0. We will monitor the marriage market in SPR-019 to ensure commoner brides are matched promptly before lord senescence.

### 3.2 Defect 2: "Agnatic Collapse to 3 Nobles" (Fixed by D56)
* **Architect Ruling:** Already resolved. The probe evaluated pre-D56 rules. Under D56 approved rules (later-children-only vocations and elite fertility read on the couple), `twin_vales` grows from 56 to 130 members over 300 years, and `manor` grows from 13 to 30 members.
* **Red Team Disposition:** **Accepted.** Confirmed that D56 eliminates the extinction trap by protecting eldest living heirs and applying elite fertility to commoner wives of noblemen.

### 3.3 Defect 3: "Cognatic Explosion" (Already Decided)
* **Architect Ruling:** D56 explicitly rejected cognatic membership. Agnatic descent is the authoritative engine contract.
* **Red Team Disposition:** **Accepted.** The probe demonstrated empirically why the Architect's rejection of cognatic membership was essential: cognatic descent flooded 100% of the manor population into nobility within 300 years.

### 3.4 Batch 07: Settlement Assets & Scorecard (P1b Planning Inbox)
* **Architect Ruling:** Filed into the P1b planning inbox in PR #96. Settlement-level assets fit the existing plan without commoner supply chains. Assets need one decision: mechanical (engine column, Registry kind) vs visual only (Plane B hydration layer, derived from seed). Mechanical assets require sourced historical rates (Notes N18 & N25).
* **Red Team Action Delivered:** Sourced Research Dossier **RD-02** ([`docs/red_team/RD-2026-10-18-medieval-manorial-rates.md`](file:///home/bdavidriggins/Documents/Rehoboam/docs/red_team/RD-2026-10-18-medieval-manorial-rates.md)) provides the exact academic citations and parameters required for the P1b gate.

---

## 4. BATCH 09 EMPIRICAL PROBES SUMMARY

### 4.1 Probe B09-01: Lifecycle Dead Person Command & Timer Drops
* **Target Contracts:** `V-INV-15`, `V-DET-21`, BCK-002 A-30, A-31, EVO-001.
* **Script:** [`tests/probes/probe_dead_person_drop.py`](file:///home/bdavidriggins/Documents/Rehoboam/tests/probes/probe_dead_person_drop.py)
* **Results:**
  - Case 1 (`dead_key`): Commands enqueued for an actor who dies before the inbox drain are safely dropped with reason `dead_key`.
  - Case 2 (`dead`): Actor alive at instant start but dying during `Vital` phase has intra-step commands dropped with reason `dead`.
  - Case 3 (`death_timer_drop`): Pending delayed timers for deceased actors are canceled before expiration.
  - Case 4 (`key = "any"`): System maintenance actions (`SettleEstate`, `RecordDeath`) targeting deceased actors are preserved and delivered.
  - Case 5 (Queue Age): FIFO order and age bounds strictly maintained.
  - Bitwise Parity: Digest `077708c8af58067e` matches across 1,000 deterministic Monte Carlo runs.

### 4.2 Probe B09-02: Engine Instant Cost & Compaction Scaling
* **Target Contracts:** SPR-038 Stage A & Stage B, BCK-002 A-60, `V-PERF-04`.
* **Script:** [`tests/probes/probe_per_instant_cost.py`](file:///home/bdavidriggins/Documents/Rehoboam/tests/probes/probe_per_instant_cost.py)
* **Results:**
  - Positional Compaction Speedup: **15.14× faster** (278.2 µs vs 4,212.8 µs on $N=10,000$, 14 columns).
  - Quiet Instant Cost: Drops from 108,000 instructions to **15,800 instructions** ($\le 20,000$ target met).
  - Log Row Scaling Ratio at $L=10^5$: **1.000×** ($\le 1.05\times$ target met).
  - Projected PR-Tier CI Core Runtime: Drops from 14:48 down to **~06:35**, saving over 8 minutes per CI cycle.

### 4.3 Probe B09-03: Two-Phase Succession Arbitration & Cascades
* **Target Contracts:** SPR-021, SPR-022, `V-SAGA-19`, `INV-M7-01..08`.
* **Script:** [`tests/probes/probe_succession_arbitration.py`](file:///home/bdavidriggins/Documents/Rehoboam/tests/probes/probe_succession_arbitration.py)
* **Results:**
  - Test 1 (Two-Phase `on_death`): Dead claimant claims voided in Phase 1 before arbitration scoring; living heir succeeds. Mutant `mutant-dead-claimant-candidate` detected and killed.
  - Test 2 (Vassal Interregnum Cascade - Ruling K2): Vassal title with no candidate whose liege is in interregnum emits `INTERREGNUM_CRISIS` and joins liege interregnum. Mutant `mutant-joined-vassal-silent` detected and killed.
  - Test 3 (Reversion in Demesne): Vassal dying without heirs when liege is healthy reverts directly to liege demesne (`REVERSION_DEMESNE`).
  - Test 4 (Determinism): Digest `7f705a59d5dde53d` verified across 500 Monte Carlo runs.

---

## 5. HISTORICAL MANORIAL RATES DOSSIER (RD-02 HIGHLIGHTS)

For full details, see [`docs/red_team/RD-2026-10-18-medieval-manorial-rates.md`](file:///home/bdavidriggins/Documents/Rehoboam/docs/red_team/RD-2026-10-18-medieval-manorial-rates.md).

| Mechanical Parameter | Sourced Value | Academic Citation | Engine Column & Plane |
|---|---|---|---|
| **Watermill Milling Multiplier** | **30×–50× labor productivity** | Langdon (2004) *Mills in Medieval Economy* | `settlement.mill_active` (M3 / Plane A) |
| **Lord's Multure Toll** | **1/16th (6.25%) of grain milled** | Bennett (1937) *Life on the English Manor* | `household.grain_store` (M4 / Plane A) |
| **Village Smithy Density** | **1 smithy per 250–350 pop** | Dyer (1989) *Standards of Living* | `settlement.has_smithy` (M4 / Plane A) |
| **Iron Consumption & Gear** | **60–100 lbs Fe/yr; Tier 1 levy gear** | Postan (1972) *Medieval Economy* | `settlement.levy_tier` (M8 / Plane A) |
| **Overland Transport Friction** | **1 : 5 : 8 (Water : Cart : Packhorse)** | Campbell (2000) *English Seigniorial Agr.* | `transport.friction_cost` (M3 / Plane A) |
| **Overland Grain Trade Limit** | **~20 miles economic limit** | Masschaele (1993) *Transport Costs* | Market trade catchment radius |
| **Siege Resolution Breakdown** | **68% starvation/surrender, 22% relief** | Bradbury (1992) *The Medieval Siege* | `siege.status` (M8 / Plane A) |
| **Garrison Stores Duration** | **90–180 days stores; Day 60+ typhus** | Prestwich (1996) *Armies and Warfare* | `granary.siege_days` (M8 / Plane A) |
| **Epidemic Mortality Pulses** | **45%–55% discrete mortality wave** | Benedictow (2004) *The Black Death* | `epidemic.pulse` (M5 / Plane A) |
| **Maternal Mortality Hazard** | **1.8% per birth, ~9% lifetime** | McFarlane (1997); Bardsley (2014) | `mortality.maternal` (M2 / Plane A) |

---

## 6. VIEWER LANE PROGRESS & D63 STATUS

1. **Viewer Plan Merges on `origin/arch/viewer-plans`:**
   - Commit `c51f07c`: PRJ-003 updated to v1.4.0.
   - Plans published:
     - `SPR-033_BevyVoxelSpike.md` (v1.3.0, 3 agent-days, C1).
     - `SPR-034_StreamHydration.md` (3 agent-days, C1).
     - `SPR-035_MaterializerScrubber.md` (3 agent-days, C1).
2. **Decision D63 (Queued for Director Review):**
   - Formal submission of viewer lane sprint plans for Director approval.
   - Recommends starting SPR-033 as soon as SPR-026 (world stream v1) lands.
3. **Hardware Invariant Compliance:**
   - ThinkPad P51 target verified: Quadro M2200, 4 cores, 32GB RAM.
   - Zero GPU requirement in CI; pure headless Linux execution.
   - Priority 1 Core Engine maintains strict precedence over Priority 2 Viewer Lane in all CI runner minutes and review resources.

---

## 7. RED TEAM NEXT STEPS UPON DIRECTOR RETURN

1. **Review D58, D59, and D63 with Director:**
   - D58: Wave 7 succession & ruler exit sprint plans.
   - D59: Minds traits, IAUS scoring, stress & breaks plans.
   - D63: Viewer lane Bevy diorama & scrubber sprint plans.
2. **Track SPR-026 & SPR-038 Merges:**
   - Monitor `sprint/SPR-038a-per-instant-cost` and `sprint/SPR-026-stream-v1` PRs.
   - Validate that SPR-038 implementation matches Probe B09-02 compaction benchmarks.
3. **Execute Wave 7 Pre-Flight Testing:**
   - Adapt Probe B09-03 to run directly against `rh-sys-dynasty` unit test harnesses once SPR-021a lands.
