# RED TEAM WORKING SCRATCHPAD // BATCH 08: "OPERATION WRECK-IT RALPH"
## The Real-World Historical Fidelity, Dynastic Stress & Engine Robustness Matrix

> **Document ID:** `HANDOFF-2026-10-18-BATCH-08-SCRATCHPAD`  
> **Status:** Living Red Team Working Dossier (In Progress)  
> **Maintained by:** Red Team Leader & Quality Auditor  
> **Date:** 2026-10-18  
> **Branch Tracking:** `arch/red-team-batch-04`  
> **Target Audience:** The Director (Personal Private World Engine)  

---

## 1. THE FOUNDATIONAL MANDATE: AUDIENCE OF ONE

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

## 2. THE "WRECK-IT RALPH" ATTACK MATRIX

These are the four operational assault vectors under active execution:

```
+-----------------------------------------------------------------------------------------+
| VECTOR 1: DYNASTIC DRAMA & SURVIVAL (THE WORLD CANNOT DIE OUT)                           |
| Target: M2.demography, M7.dynasty, specs/worldgen/project_population.py                 |
| The Risk: The Director runs a 200-year feudal simulation (Manor / Twin Vales), and by   |
| year 80 every noble house collapses to zero living heirs due to aggressive mortality.   |
| The Attack: Sweep 100 seeds across 300+ years. Check extinction rates under strict       |
| primogeniture and agnatic succession. Measure noble cohort survival.                    |
+-----------------------------------------------------------------------------------------+
                                           │
                                           ▼
+-----------------------------------------------------------------------------------------+
| VECTOR 2: BIT-FOR-BIT CROSS-MACHINE DETERMINISM                                         |
| Target: PRNG streams (rh-ids), floating-point leaks, thread race conditions              |
| The Risk: A 500-year soak run on the cloud Genoa VM (`adjutant`) diverges when re-run    |
| on the Director's ThinkPad P51.                                                         |
| The Attack: Assert 100% bitwise parity on golden event hashes and state digests across   |
| 1, 2, 4, and N worker threads. Hunt down any unseeded standard library randoms.         |
+-----------------------------------------------------------------------------------------+
                                           │
                                           ▼
+-----------------------------------------------------------------------------------------+
| VECTOR 3: SPECTATOR STREAMING & ZERO-LAG SCRUBBING                                      |
| Target: SPR-026 wire_v1, crates/rh-protocol, bin/stream_server.py, 3D Voxel Viewer      |
| The Risk: The socket blocks the simulation engine, drops frames, or stutters during     |
| 50-year timeline scrubbing in the viewer.                                               |
| The Attack: Stress-test slow consumers, un-enveloped raw wire_v1 frames, and            |
| keyframe seek latencies (< 250 ms for a 20-year scrub).                                 |
+-----------------------------------------------------------------------------------------+
                                           │
                                           ▼
+-----------------------------------------------------------------------------------------+
| VECTOR 4: HARDWARE RESOURCE BOUNDS (LAPTOP PRESERVATION)                                |
| Target: Memory footprint, Arrow IPC buffers, Parquet segment rollover                   |
| The Risk: A 1,000-year simulation run leaks memory or clutters disk, freezing hardware. |
| The Attack: Enforce O(1) memory per agent (dead rows purged immediately to lifecycle    |
| queues) and bounded Parquet/Arrow streaming footprints.                                 |
+-----------------------------------------------------------------------------------------+
```

---

## 3. THE REAL-WORLD HISTORICAL FIDELITY SCORECARD

An unflinching reality check comparing authentic medieval mechanics against Rehoboam's architecture:

| Historical Reality | Authentic Medieval Mechanic | Current Rehoboam Reality | Red Team Gap & Action Required |
| :--- | :--- | :--- | :--- |
| **Pillar 1: Sieges & Starvation** | Castles are food/water banks. 90% of sieges won by starvation, well poisoning, and camp dysentery; wall breaches are rare. Sapping collapses stone towers. | Prototypes show trebuchet ballistics; `M8.war` has siege clocks. | **Gap:** No granary depletion counters or camp typhus in `M8`. Must audit siege duration formulas. |
| **Pillar 2: Plagues & Epidemics** | Black Death wipes out 30–50% of the population in 18 months; childbed fever kills 20% of mothers, causing remarriage and half-sibling wars. | `M2` uses static Coale-Demeny curves; `M5.epidemics` (SEIRS) is deferred to later phases. | **Gap:** Succession is too predictable. Real history needs sudden epidemic pulses that wipe out princes. |
| **Pillar 3: The Manorial Economy** | Land and grain are the only wealth. 1:5:30 transport cost ratio (water 30x cheaper than land). Famine is hyper-local. Crop rotation failure = starvation. | `M3` (agri) and `M4` (economy) designed in CODEX; Phase 1a Rust core only tracks ages/births/deaths. | **Gap:** Commoners don't yet sow/harvest physical wheat in code. Army ceilings must strictly bind to grain surplus. |
| **Pillar 4: Cadet Feuds & Ambition** | Primogeniture leaves younger brothers landless. Unlanded cadet sons become mercenaries, pretenders, and assassins, driving dynastic wars. | Prototype 7 (*Nobility Thermostat*) demotes cadets at age 35. `T1.minds` models stress & breaks. | **Gap:** Ensure demoted cadets generate jealousy and pretender factions in `T1`/`M7` rather than vanishing quietly. |
| **Pillar 5: Battle Panic & The Rout** | Armies rarely fight to the death. 5% casualties in the clash; 40% die during the panic and rout. Levies break when their lord falls. | `M8` uses Helmbold/Lanchester combat attrition. | **Gap:** Must enforce morale breakpoint thresholds where formations break into chaotic routs. |

---

## 4. IMMEDIATE EXECUTION BACKLOG

1. **Probe B08-01: Dynastic Longevity Sweep (`probe_dynastic_survival.py`)**
   * Run 100 seeds of `manor` and `twin_vales` through `specs/worldgen/project_population.py` and `selftest_population_synthesis.py`.
   * Measure: How many noble houses survive past Year 100? Year 200? Year 300?
   * Pinpoint the exact tipping point where strict agnatic primogeniture causes total dynastic extinction.
2. **Probe B08-02: Live Wire v1 Stream Ingestion (`probe_wire_v1_feed.py`)**
   * Build the interim subscriber for `A.sock` directly consuming raw `wire_v1` frames (PR #87 Directive 2).
   * Verify that canonical simulation records stream into the viewer with zero custom JSON envelope bloat and zero client-side calendar math.
3. **Probe B08-03: Famine & Army Ceiling Stress Audit**
   * Inspect the coupling between `M3.agriculture` yield and `M8` army mobilization.
   * Verify that pulling 30% of male commoners into war causes an immediate food deficit in the home region.
