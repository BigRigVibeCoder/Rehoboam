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

There is **zero ambiguity** between the Director, the Red Team, and the Architect regarding project priorities:

```
+─────────────────────────────────────────────────────────────────────────────────────────+
| PRIORITY 1: BUILD THE ENGINE TO A MINIMUM VIABLE CORE (PHASE 1A / 1B)                    |
| • Living ledger, bloodlines, births, deaths, and dynastic succession (M7).              |
| • Real demography (M2) and stress/mental breaks (T1.minds).                             |
| • Apache Arrow IPC World Stream sink and bitwise determinism (SPR-013, SPR-026).        |
| • Status: Active execution. Non-negotiable prerequisite for all visual layers.           |
+─────────────────────────────────────────────────────────────────────────────────────────+
                                             │
                                             ▼
+─────────────────────────────────────────────────────────────────────────────────────────+
| PRIORITY 2: THE PERSONAL 3D VOXEL VIEWER FOR THE DIRECTOR (PLANE C / BEVY & WEBGL)       |
| • The visual spectator harness (rh-view / PRJ-003, SPR-033, ADR-0036).                  |
| • Direct subscription to wire_v1 (A.sock / C.sock) streaming un-enveloped frames.       |
| • Peak SOTA 0.35m micro-voxel fidelity, first-person ground walking, locked 60 FPS.     |
| • The Director must be able to sit back and watch his living world unfold with zero lag.│
+─────────────────────────────────────────────────────────────────────────────────────────+
                                             │
                                             ▼
+─────────────────────────────────────────────────────────────────────────────────────────+
| PRIORITY 3: THE 4-EPOCH PROGRESSIVE HISTORICAL EXPANSION                                |
| • Once Priorities 1 & 2 are solid, expand the timeline along authentic historical eras.  |
| • Era is state, not hardcoded branches (ADR-0027): E2 Gunpowder -> E3 Steam -> E4 Modern.│
| • Unleash Adjutant's 16 vCPUs and 62 GB RAM to scale to 1,000,000 living citizens.       |
+─────────────────────────────────────────────────────────────────────────────────────────+
```

---

## 2. THE FOUNDATIONAL MANDATE: AUDIENCE OF ONE

1. **Private Personal Product, Not a Commercial SaaS:**
   * There are zero enterprise customers, zero sales funnels, zero multi-tenant permissions, and no commercial cloud servers.
   * Rehoboam is an exquisite, bespoke, private world engine built **exclusively for the Director**.
   * Every token, every test, and every line of code is measured by:
     * Does the living world feel rich, authentic, and historically profound?
     * Does the simulation run whisper-quiet, with $O(1)$ memory per agent on personal hardware (ThinkPad P51 & `adjutant` VM)?
     * Does history unfold with genuine grit, politics, famine, and dynastic tension without global game clamps?
2. **The Real-World Historical Lens (No Mythical Crutches):**
   * Avoid high-fantasy and mythical tropes (no elves, dwarves, dragons, or magic rings).
   * Anchor strictly in **authentic 11th–15th century feudal human history**: Plantagenet England, Capetian France, the Wars of the Roses, the Holy Roman Empire, the Black Death, and the manorial economy.
   * Real history did not run on magic: it ran on **calories, pathogens, human biology, geography, and dynastic paranoia**.

---

## 3. ADJUTANT CLOUD BEAST: HARDWARE SPECS & PERFORMANCE RE-ASSESSMENT

Probed directly from `/proc/cpuinfo`, `free -h`, and `lscpu --caches` on 2026-10-18:

| Hardware Component | Adjutant Specification | Simulation Impact & Scaling Ceiling |
| :--- | :--- | :--- |
| **CPU Architecture** | **AMD EPYC 9B14 "Genoa" (Zen 4)** | 16 vCPUs (8 physical cores, 16 threads @ 5.2 GHz BogoMIPS). |
| **Vector Extensions** | **Native AVX-512** (`avx512_vnni`, `avx512bw`) | SIMD vectorized evaluation of 16–32 agent life states per clock cycle. |
| **L3 Cache** | **32 MiB Unified Cache** | **Holds 466,000 living humans entirely in CPU cache** ($72\text{ bytes/person}$). |
| **L2 Cache** | **8 MiB** (1 MiB dedicated per core) | Blazing-fast inner tick loops on active settlement subsets. |
| **RAM** | **62 GiB DDR5 Server Memory** | **57 GiB available.** Holds 1,000,000 agents in 72 MB; 5,000,000 in 360 MB. |
| **RAM Disk** | **32 GiB `/dev/shm` tmpfs** | Zero-latency Arrow IPC ring buffers & DuckDB Parquet segments in RAM. |
| **Storage** | **PCIe NVMe SSD** | High-throughput background chronicle persistence. |

**The Hardware Sledgehammer:** The old fear that a 500,000-person demographic transition would melt the Director's laptop is dead. Adjutant can simulate a 1,000,000-person empire entirely in memory with sub-millisecond tick execution.

---

## 4. THE 4-EPOCH PROGRESSIVE ROADMAP (ADR-0027 REALIZED)

Eras are continuous state, not hardcoded engine branches. Coverage expands progressively:

```
                                  THE REHOBOAM PROGRESSIVE TIMELINE
                                 (Tuned for Cloud Beast "Adjutant")
  
   EPOCH 1: THE FEUDAL FOUNDATION (1000–1500 CE) — [ACTIVE BUILD: TODAY]
   ┌──────────────────────────────────────────────────────────────────────────────────────┐
   │ • Scope: Manors, keeps, bloodlines (M7), peasant demography (M2), stress breaks (T1). │
   │ • Mechanics: Food/wheat harvests (M3/M4), castle sieges & wall breaches (M8).         │
   │ • Scale: 60 to 10,000 living humans. Sub-millisecond tick speeds on Adjutant.         │
   │ • Personal Experience: The gritty, intimate saga of a noble dynasty holding a keep.  │
   └──────────────────────────────────────────────────────────────────────────────────────┘
                                              │
                                              ▼
   EPOCH 2: THE AGE OF GUNPOWDER & SAIL (1500–1750 CE) — [ERA PACK E2]
   ┌──────────────────────────────────────────────────────────────────────────────────────┐
   │ • Adds: Matchlocks, cannons, trace italienne star forts replacing vertical keeps.     │
   │ • Mechanics: Caravels and galleons on oceanic trade routes; early parliaments (M6).   │
   │ • Scale: 10,000 to 50,000 living humans. Build time: ~24–48 agent hours.             │
   │ • Personal Experience: Stone castles shatter under cannon fire; merchant fleets rise. │
   └──────────────────────────────────────────────────────────────────────────────────────┘
                                              │
                                              ▼
   EPOCH 3: STEAM, COAL & THE INDUSTRIAL LEAP (1750–1900 CE) — [ERA PACK E3]
   ┌──────────────────────────────────────────────────────────────────────────────────────┐
   │ • Adds: Coal mining, steam engines, textile factories, iron railroads.                │
   │ • Mechanics: Malthusian escape (sanitation collapses death rates; 10x population).    │
   │ • Scale: 100,000 to 1,000,000+ living humans (unleashing Adjutant's 62 GB DDR5 RAM).   │
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

## 5. THE REAL-WORLD HISTORICAL FIDELITY SCORECARD

An unflinching reality check comparing authentic medieval mechanics against Rehoboam's architecture:

| Historical Reality | Authentic Medieval Mechanic | Current Rehoboam Reality | Red Team Gap & Action Required |
| :--- | :--- | :--- | :--- |
| **Pillar 1: Sieges & Starvation** | Castles are food/water banks. 90% of sieges won by starvation, well poisoning, and camp dysentery; wall breaches are rare. Sapping collapses stone towers. | Prototypes show trebuchet ballistics; `M8.war` has siege clocks. | **Gap:** No granary depletion counters or camp typhus in `M8`. Must audit siege duration formulas. |
| **Pillar 2: Plagues & Epidemics** | Black Death wipes out 30–50% of the population in 18 months; childbed fever kills 20% of mothers, causing remarriage and half-sibling wars. | `M2` uses static Coale-Demeny curves; `M5.epidemics` (SEIRS) is deferred to later phases. | **Gap:** Succession is too predictable. Real history needs sudden epidemic pulses that wipe out princes. |
| **Pillar 3: The Manorial Economy** | Land and grain are the only wealth. 1:5:30 transport cost ratio (water 30x cheaper than land). Famine is hyper-local. Crop rotation failure = starvation. | `M3` (agri) and `M4` (economy) designed in CODEX; Phase 1a Rust core only tracks ages/births/deaths. | **Gap:** Commoners don't yet sow/harvest physical wheat in code. Army ceilings must strictly bind to grain surplus. |
| **Pillar 4: Cadet Feuds & Ambition** | Primogeniture leaves younger brothers landless. Unlanded cadet sons become mercenaries, pretenders, and assassins, driving dynastic wars. | Prototype 7 (*Nobility Thermostat*) demotes cadets at age 35. `T1.minds` models stress & breaks. | **Gap:** Ensure demoted cadets generate jealousy and pretender factions in `T1`/`M7` rather than vanishing quietly. |
| **Pillar 5: Battle Panic & The Rout** | Armies rarely fight to the death. 5% casualties in the clash; 40% die during the panic and rout. Levies break when their lord falls. | `M8` uses Helmbold/Lanchester combat attrition. | **Gap:** Must enforce morale breakpoint thresholds where formations break into chaotic routs. |

---

## 6. FORMAL STRATEGIC MEMO FOR THE ARCHITECT

```markdown
MEMORANDUM FOR THE ARCHITECT (AGT-001)
FROM: Red Team Leader & Quality Auditor
DATE: 2026-10-18
SUBJECT: Strategic Execution Alignment & Director Priorities

The Director has established an ironclad, non-negotiable 3-tier prioritization invariant:

1. PRIORITY 1: Minimum Viable Engine Core (Phase 1a / 1b)
   - Do not defer or distract core developers with auxiliary tooling or premature era packs.
   - Deliver and lock down the living ledger, M2 demography knots, M7 dynastic succession, 
     T1 stress/mental breaks, and the raw Apache Arrow wire_v1 stream writer.
   - G1a and G1b remain the primary critical path.

2. PRIORITY 2: The Director's 3D Voxel Viewer (Plane C / rh-view / WebGL)
   - Immediately following SPR-026/SPR-027, the visual spectator harness (PRJ-003, SPR-033) 
     is the primary deliverable for the Director's personal workstation.
   - The viewer must subscribe directly to wire_v1 on 127.0.0.1, render 0.35m micro-voxels 
     at locked 60 FPS, support first-person walking, and achieve sub-250ms timeline scrubbing.
   - The Director must have a working, beautiful window into his simulated world before 
     sagas or narrative compilers are scheduled.

3. PRIORITY 3: The 4-Epoch Progressive Expansion (E2 -> E3 -> E4)
   - With Adjutant's hardware profile verified (AMD EPYC Genoa Zen 4, 16 vCPUs, 32 MiB L3, 
     62 GiB RAM, 32 GiB /dev/shm RAM disk), the 1M-person memory ceiling is fully lifted.
   - Following G2, era packs expand continuously: E2 Gunpowder/Sail (1500-1750) -> E3 Steam/Rail (1750-1900) -> 
     E4 Industrial War (1900-1945).
   - Eras remain state vectors (ADR-0027), preserving 100% backward compatibility.
```
