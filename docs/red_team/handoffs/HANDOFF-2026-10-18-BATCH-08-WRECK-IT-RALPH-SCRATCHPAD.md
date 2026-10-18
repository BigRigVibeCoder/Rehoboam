# RED TEAM WORKING SCRATCHPAD // BATCH 08: "OPERATION WRECK-IT RALPH"
## The Real-World Historical Fidelity, Dynastic Stress & Engine Robustness Matrix

> **Document ID:** `HANDOFF-2026-10-18-BATCH-08-SCRATCHPAD`  
> **Status:** Living Red Team Master Dossier & Strategic Execution Plan  
> **Maintained by:** Red Team Leader & Quality Auditor  
> **Date:** 2026-10-18  
> **Branch Tracking:** `arch/red-team-batch-04`  
> **Target Audience:** The Director & The Architect  

---

## 1. THE DIRECTOR'S NON-NEGOTIABLE 3-TIER PRIORITY INVARIANT

There is **complete alignment** between the Director, the Red Team, and the Architect regarding project priorities:

```
+─────────────────────────────────────────────────────────────────────────────────────────+
| PRIORITY 1: BUILD THE ENGINE TO A MINIMUM VIABLE CORE (PHASE 1A / 1B)                    |
| • Living ledger, bloodlines, births, deaths, and dynastic succession (M7).              |
| • Real demography (M2) and stress/mental breaks (T1.minds).                             |
| • Apache Arrow IPC World Stream sink and bitwise determinism (SPR-013, SPR-026).        |
| • Family Annals (SPR-029) retained in P1: G1a gate requires reading history of 3 seeds. |
| • Status: Active execution. Non-negotiable prerequisite for all visual layers.           |
+─────────────────────────────────────────────────────────────────────────────────────────+
                                             │
                                             ▼
+─────────────────────────────────────────────────────────────────────────────────────────+
| PRIORITY 2: THE PERSONAL 3D VOXEL VIEWER FOR THE DIRECTOR (PLANE C / BEVY D51)          |
| • The visual spectator harness (rh-view / PRJ-003, SPR-033 spike, SPR-034 diorama).     |
| • Native Bevy over Unix domain socket (A.sock / C.sock), tunneled via SSH from P51.     |
| • 0.35m default micro-voxel resolution with measured Level-of-Detail (LOD) fallback.    |
| • Client-side watched history cache: instantaneous (< 16ms) timeline scrubbing.         |
| • Reported target: >= 60 FPS on the Director's ThinkPad P51 (Quadro M2200).             |
| • Sagas, Oracle, and podcast synthesis (P2.x) wait strictly behind the viewer.          |
+─────────────────────────────────────────────────────────────────────────────────────────+
                                             │
                                             ▼
+─────────────────────────────────────────────────────────────────────────────────────────+
| PRIORITY 3: THE 4-EPOCH PROGRESSIVE HISTORICAL EXPANSION                                |
| • Once Priorities 1 & 2 are solid, expand the timeline along authentic historical eras.  |
| • Era is state, not hardcoded branches (ADR-0027): E2 Gunpowder -> E3 Steam -> E4 Modern.│
| • 1,000,000 citizen scaling remains an empirical target measured via G1a & SPR-030.     |
+─────────────────────────────────────────────────────────────────────────────────────────+
```

### 1.1 Contention Rule
Where tiers compete for CI minutes, Architect review/audit time, or GitHub merge slots: **Priority 1 wins unconditionally.**

---

## 2. THE FOUNDATIONAL MANDATE: AUDIENCE OF ONE

1. **Private Personal Product, Not a Commercial SaaS:**
   * Zero enterprise customers, zero sales funnels, zero multi-tenant permissions, and no commercial cloud servers.
   * Rehoboam is an exquisite, bespoke, private world engine built **exclusively for the Director**.
   * Every token, test, and line of code is measured by:
     * Does the living world feel rich, authentic, and historically profound?
     * Does the simulation run whisper-quiet on personal hardware (ThinkPad P51 & `adjutant` VM)?
     * Does history unfold with genuine grit, politics, famine, and dynastic tension without global game clamps?
2. **The Real-World Historical Lens (No Mythical Crutches):**
   * Avoid high-fantasy and mythical tropes (no elves, dwarves, dragons, or magic rings).
   * Anchor strictly in **authentic 11th–15th century feudal human history**: Plantagenet England, Capetian France, the Wars of the Roses, the Holy Roman Empire, the Black Death, and the manorial economy.
   * Real history did not run on magic: it ran on **calories, pathogens, human biology, geography, and dynastic paranoia**.

---

## 3. ADJUTANT CLOUD HOST: HARDWARE PROFILE & COMPUTE BOTTLENECK

Probed directly from `/proc/cpuinfo`, `free -h`, and `lscpu --caches` on 2026-10-18:

| Hardware Component | Adjutant Specification | Technical Reality & Simulation Impact |
| :--- | :--- | :--- |
| **CPU Architecture** | **AMD EPYC 9B14 "Genoa" (Zen 4)** | 16 vCPUs (8 physical cores, 16 threads @ 5.2 GHz BogoMIPS). |
| **Vector Extensions** | **Native AVX-512** (`avx512_vnni`, `avx512bw`) | SIMD evaluation of 16–32 agent life states per clock cycle. |
| **L3 Cache** | **32 MiB Unified Cache** | Blazing-fast L3 cache locality across Rayon worker threads. |
| **L2 Cache** | **8 MiB** (1 MiB dedicated per core) | Dedicated instruction & data cache per worker thread. |
| **RAM** | **62 GiB DDR5 Server Memory** | 57 GiB available. Ample capacity for all state tables and sifting. |
| **RAM Disk (`/dev/shm`)**| **32 GiB tmpfs** | Zero-latency ring buffer for **ephemeral live viewer streaming**. |
| **Persistent Storage**| **PCIe NVMe SSD** | Mandatory for authoritative runs; **`fsync` required for crash recovery**. |

### 3.1 The Compute Bottleneck (Architect Correction)
* **Memory was never the ceiling; time per simulated instant is.**
* **Measured Baseline Today:**
  - `solo` world: ~92,000 instructions per instant.
  - `kingdom_10k` world: 1–4 ms per quiet sim-day (before SPR-038).
* **Entity Footprint:** A person is not 72 bytes; each citizen spans 9–17 eight-byte columnar tables, plus life-course interval logs, household graphs, and mental state models (`T1.minds`).
* **Scaling Path:** Sub-millisecond ticks at 1,000,000 citizens is an unverified hypothesis. The empirical standard is **G1a**: simulate `kingdom_10k` across 1,000 sim-years in $\le 60\text{ seconds}$, with SPR-030 measuring core scaling. 1M remains a post-G2 scaling target governed by measured benchmarks.

---

## 4. THE 4-EPOCH PROGRESSIVE ROADMAP (ADR-0027 REALIZED)

Eras remain state vectors (ADR-0027), preserving 100% backward compatibility:

```
                                  THE REHOBOAM PROGRESSIVE TIMELINE
                                  (Governed by Empirical Scaling)
  
   EPOCH 1: THE FEUDAL FOUNDATION (1000–1500 CE) — [ACTIVE BUILD: TODAY]
   ┌──────────────────────────────────────────────────────────────────────────────────────┐
   │ • Scope: Manors, keeps, bloodlines (M7), peasant demography (M2), stress breaks (T1). │
   │ • Mechanics: Food/wheat harvests (M3/M4), castle sieges & wall breaches (M8).         │
   │ • Scale: 60 to 10,000 living humans. Target: 1,000 years in 60s (G1a).               │
   │ • Personal Experience: The gritty, intimate saga of a noble dynasty holding a keep.  │
   └──────────────────────────────────────────────────────────────────────────────────────┘
                                              │
                                              ▼
   EPOCH 2: THE AGE OF GUNPOWDER & SAIL (1500–1750 CE) — [ERA PACK E2]
   ┌──────────────────────────────────────────────────────────────────────────────────────┐
   │ • Adds: Matchlocks, cannons, trace italienne star forts replacing vertical keeps.     │
   │ • Mechanics: Caravels and galleons on oceanic trade routes; early parliaments (M6).   │
   │ • Scale: 10,000 to 50,000 living humans.                                              │
   │ • Personal Experience: Stone castles shatter under cannon fire; merchant fleets rise. │
   └──────────────────────────────────────────────────────────────────────────────────────┘
                                              │
                                              ▼
   EPOCH 3: STEAM, COAL & THE INDUSTRIAL LEAP (1750–1900 CE) — [ERA PACK E3]
   ┌──────────────────────────────────────────────────────────────────────────────────────┐
   │ • Adds: Coal mining, steam engines, textile factories, iron railroads.                │
   │ • Mechanics: Malthusian escape (sanitation collapses death rates; 10x population).    │
   │ • Scale: 100,000 to 1,000,000+ living humans (contingent on measured scaling).        │
   │ • Personal Experience: Smoke stacks rise over the river; trains haul iron ore.        │
   └──────────────────────────────────────────────────────────────────────────────────────┘
                                              │
                                              ▼
   EPOCH 4: THE INDUSTRIAL CRUCIBLE & MASS MOBILIZATION (1900–1945 CE) — [ERA PACK E4]
   ┌──────────────────────────────────────────────────────────────────────────────────────┐
   │ • Adds: Telegraph/radio communications, mass conscription, artillery, trench systems. │
   │ • Mechanics: National mobilization, war bonds, factory supply chains.                 │
   │ • Scale: Continental empires; multi-district metropolises.                            │
   │ • Personal Experience: The epic death of the old feudal order under industrial war.   │
   └──────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 5. THE REAL-WORLD HISTORICAL FIDELITY SCORECARD (P1b INBOX)

The following 5 pillars represent core historical design proposals. Per the Architect's directive, they are remanded to the **P1b Planning Gate Inbox** for formal dispositions without disrupting active Priority 1 sprints:

| Historical Reality | Authentic Medieval Mechanic | Current Rehoboam State | P1b Planning Gate Disposition |
| :--- | :--- | :--- | :--- |
| **Pillar 1: Sieges & Starvation** | Castles are food/water banks. 90% of sieges won by starvation, well poisoning, and dysentery; wall breaches are rare. Sapping collapses stone towers. | Prototypes show trebuchet ballistics; `M8.war` has siege clocks. | **P1b Task:** Add granary depletion countdown and camp disease hazards to `M8`. |
| **Pillar 2: Plagues & Epidemics** | Black Death wipes out 30–50% of population in 18 months; childbed fever kills 20% of mothers, causing remarriage and half-sibling wars. | `M2` uses static Coale-Demeny curves; `M5.epidemics` (SEIRS) is deferred. | **P1b Task:** Implement discrete mortality pulse events that kill across demographic ranks. |
| **Pillar 3: The Manorial Economy** | Land and grain are the only wealth. 1:5:30 transport cost ratio (water 30x cheaper than land). Famine is hyper-local. Crop rotation failure = starvation. | `M3` (agri) and `M4` (economy) designed; Phase 1a tracks ages/births/deaths. | **P1b Task:** Bind military levy capacity directly to regional caloric grain surplus. |
| **Pillar 4: Cadet Feuds & Ambition** | Primogeniture leaves younger brothers landless. Unlanded cadet sons become mercenaries, pretenders, and assassins, driving dynastic wars. | Nobility Thermostat demotes cadets at age 35. `T1.minds` models stress & breaks. | **P1b Task:** Ensure demoted cadet lines generate rival claim factions in `M7`/`T1`. |
| **Pillar 5: Battle Panic & The Rout** | Armies rarely fight to the death. 5% casualties in the clash; 40% die during panic and rout. Levies break when their lord falls. | `M8` uses Helmbold/Lanchester combat attrition. | **P1b Task:** Enforce morale shock thresholds causing rapid formation routs. |

---

## 6. PERSONAL 3D VOXEL VIEWER ARCHITECTURE (`rh-view`)

* **Rendering Engine:** Native Bevy (D51) with direct Vulkan/OpenGL metal access on the Director's ThinkPad P51 (NVIDIA Quadro M2200). Browser/WebGPU is deferred to V.3.
* **IPC Transport:** `wire_v1` frames streamed over a Unix domain socket directly on Adjutant (`/tmp/rh_stream.sock`), tunneled securely via SSH to the Director's laptop.
* **Voxel Resolution:** Default **0.35m micro-voxels**, backed by a measured Level-of-Detail (LOD) octree fallback for distant landscape tiles.
* **Camera Modes:** Free spectator / floating camera in V.1; first-person ground walking with world interaction in V.3.
* **The Client-Side Scrub Cache:**
  - The viewer maintains a local ring cache of previously ingested frames.
  - **Scrubbing within watched history is instantaneous ($< 16\text{ ms}$)**.
  - Cold jumps into unwatched historical eras query the Plane A replay materializer.
* **Target Metrics:** $\ge 60\text{ FPS}$ on a 64×64 diorama with 50 actors, and $\le 250\text{ ms}$ scrub latency. Governed as **reported targets** measured on the Director's ThinkPad P51 (CI and Adjutant have no GPUs).

---

## 7. EMPIRICAL TEST BATTERY RESULTS (1,237 / 1,237 PASS)

Subjected to exhaustive multi-tier verification on 2026-10-18:

```
========================================================================================
                                REHOBOAM TEST SCORECARD
========================================================================================
 [1] Complete PR Gate Tier (bin/check_all.sh):              538 / 538 Passed   (100.0%)
 [2] Python Workspace Test Suite (pytest python/tests):     681 / 681 Passed   (100.0%)
 [3] Domain Spec Reference Selftests (specs/**/selftest*):   14 /  14 Passed   (100.0%)
 [4] Multi-Thread Concurrency Scaling (1T vs 16T):                  2 /   2 Passed   (100.0%)
 [5] Empirical Red Team Probes (Batch 08):                          2 /   2 Passed   (100.0%)
----------------------------------------------------------------------------------------
 TOTAL VERIFIED CHECKS:                                  1,237 / 1,237 Passed  (100.0%)
========================================================================================
```

### 7.1 Concurrency & Determinism Highlights
* **`smoke` World (1,000 Sim-Years @ 1T vs 16T):** 10,323,633 dispatch levels; final state hash `8e723abdf1262be8` matched bit-for-bit across 1 thread and 16 threads.
* **`hamlet` World (200 Sim-Years @ 1T vs 8T):** 2,460 dispatch levels; final state hash `b87575277fb8b3f3` matched bit-for-bit.
* **Crash Recovery:** `kill -9` during segment sealing recovers byte-identical resealing.
* **Virtual Telemetry:** Random access seeking anywhere across 60 sim-years clocks at $27.9\,\mu\text{s}$ per person-day.

---

## 8. EMPIRICAL SIMULATION & GAMEPLAY DEFECTS (PROBE B08-01 FINDINGS)

While the engine software executed with zero errors, Probe B08-01 discovered **three critical simulation/demographic defects** that directly threaten long-run world viability:

### Defect 1: The "Dead-on-Arrival" Genesis Trap (6% of World Seeds)
* **Finding:** In 50 Monte Carlo seeds of population synthesis on `The Manor of Ash` ($N=400$, 2 noble houses), **6.0% of generated seeds (3 of 50)** spawn a noble house with **ZERO fertile women** (no adult bloodline daughters and no in-married wives under age 45).
* **Impact:** The house is biologically doomed at Tick 0. Upon the aging lord's death, the house goes extinct immediately with zero opportunity for player intervention.
* **Required Fix:** Synthesizer must enforce a hard constraint: every landed noble house at Tick 0 must start with $\ge 1$ living fertile woman (age 16–35) or fertile wife.

### Defect 2: The 300-Year Dynastic Collapse Trap (Agnatic Primogeniture Bleed)
* **Finding:** Under historical Agnatic Primogeniture, noble counts collapse from 20 genesis nobles down to **3.0 total (1.5 per house)** at Year 300, hitting a low of **2.6 total (1.3 per house)** at Year 323.
* **Compounding Cause (Holy Orders Celibacy Drain):** 
  * Sweeping the holy orders vocation drain rate $v$ from 0% to 30%:
    - At $v = 0.0\%$: 31.3 nobles survive (healthy).
    - At $v = 5.0\%$: 18.1 nobles survive.
    - At $v = 10.0\%$: 10.2 nobles survive.
    - At $v \ge 20.0\%$: **3.0 nobles survive (CRITICAL EXTINCTION RISK).**
  * Draining 20% of younger siblings into monasteries in small settlements drains the reproductive pool faster than replacement.
* **Required Fix:** Scale holy orders drain to 5–10% in settlements where $N < 1,000$, or allow vocation entry only when a house has $\ge 3$ living sons.

### Defect 3: Unbounded Cognatic Explosion vs. Bounded Succession
* **Finding:** Under pure Cognatic succession (both sons and daughters inherit and transmit nobility), noble populations explode from 20 to **458 nobles (100% of the manor)** by Year 300, completely diluting commoner labor.
* **Required Fix:** Implement **Bounded Succession** (nobility tied to recognized physical seats/fiefs), which was the only rule that maintained a stable 20–22 nobles across 300 years (~11 per house).

---

## 9. DISPOSITIONS ON ARCHITECT DECISIONS

1. **Decision A (Viewer Start Time): Approved Option (2).**
   - Commission viewer plans now.
   - Start the **SPR-033 Bevy spike immediately upon SPR-026 merge** (wire framing).
   - Scrubber implementation (SPR-035) waits for SPR-027 (materializer).
   - Delivers visual diorama 3–4 days earlier with zero risk to engine core.
2. **Decision B (Viewer Performance Targets): Approved as Reported Targets on ThinkPad P51.**
   - $\ge 60\text{ FPS}$ and $\le 250\text{ ms}$ scrub times remain reported targets measured on the Director's P51 laptop.
   - Zero GPU gates in headless CI or Adjutant.
3. **P1 Contention Rule Invariant:**
   - Active P1 work (wave 4, SPR-015/016/017, wave 6–8, SPR-038, T1.minds, SPR-029 family annals) holds absolute priority over viewer lanes in all resource contention.
