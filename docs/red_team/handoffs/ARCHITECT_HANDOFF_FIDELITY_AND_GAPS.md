# ARCHITECT HANDOFF: SIMULATION FIDELITY, THE 5 GAPS, AND "THE ENGINE ONLY ENGINES"

> **From:** Red Team Leader & Red Team Auditor  
> **To:** Lead Architect & Swarm Coordinators  
> **Date:** October 16, 2026  
> **Status:** Ratified Clarification & Architectural Guidance  
> **Target Systems:** Event Envelope (`rh-events`), World Grid / Spatial Topography, Mechanics Registry (`S6.artifacts`, `S7.vows`, `M7.dynasty`, `M35.story_sifter`)

---

## 1. EXECUTIVE RATIFICATION: WE EMPHATICALLY AGREE

We fully ratify and agree with the Architect's correction on the **Cinematic Event Envelope split**:

> *"Storing lighting or 'blood dripping' inside the engine would itself break 'the engine only engines'."*

**The Architect is 100% correct.** 

### Why Storing Adjectives/Lighting in the Engine is an Anti-Pattern:
1. **Adjectives are Perceptual Renderings, Not Physical Facts:**  
   `"Flickering wall torches"` is a visual rendering decision. If the chronicle is consumed as an **offline audio drama**, that field is useless (audio needs reverb and acoustic volume). If consumed as a **2D retro tilemap (Micropolis)**, it needs an illumination radius integer. If consumed as a **novel**, it needs literary prose. Hardcoding visual adjectives into Rust structs contaminates the simulation with a single downstream medium.
2. **Determinism & Cache Efficiency:**  
   Strings like `"dramatic chiaroscuro lighting"` waste memory, bloat cache lines, and introduce string formatting into what should be pure `u32`/`u64` bitwise deterministic execution.
3. **The Core Division of Labor:**  
   - **The Engine Simulates Ontological Truth:** *Matter, Space, Time, State, and Causal Graph.*
   - **Downstream Tools Generate Perceptual Illusions:** *Pixels, Prompts, Dialogues, Sound Effects, and Prose.*

---

## 2. HOW TO INCREASE SIMULATION FIDELITY WITHOUT BREAKING THE RULE

Increasing fidelity does **not** mean adding richer words or camera directions to the engine.  
**It means simulating deeper physical and psychological reality in Plane A.**

Here is the exact formula for scaling fidelity while preserving absolute engine purity:

```
+-----------------------------------------------------------------------------+
|                               RUST CORE (Plane A)                           |
|  Simulates Pure State & Physics:                                            |
|  - Coordinates: (12, 3), Room: GreatHall                                    |
|  - Cell Contaminants: BitFlags<BLOOD | ASH>                                 |
|  - Item: Phial { volume: 0ml, toxicity: Q16_16(0.95), origin: Alchemist(33) }|
|  - Actor State: Stress = 98/100, VowConflict = [Vow(Fealty), Vow(Vengeance)]|
|  - Epistemic Horizon: KnownBy = BitSet[SubjectId, AccompliceId]             |
+-----------------------------------------------------------------------------+
                                      │
                                      ▼ Parquet Chronicle (~35 bytes/event)
+-----------------------------------------------------------------------------+
|                           DOWNSTREAM TRANSLATORS                            |
|                                                                             |
|  [2D SimCity / Micropolis]   [Graphic Novel Storyboard]  [Audio Drama Engine]|
|  - Cell (12,3) blood stained - "Crimson gore across     - "sfx_choke.wav"   |
|  - Keep crime = 90%            flagstones, chiaroscuro"  - "reverb: stone"  |
|  - Lower ward catches fire   - Mike Mignola ink style   - Kokoro narration  |
+-----------------------------------------------------------------------------+
```

### The 4 Vectors of True High Fidelity:
1. **Spatial & Topographical Fidelity (Grid Coordinates):**  
   Replace coarse string tags (`"HIGH_KEEP"`) with discrete grid locations: `settlement_id: 2, cell: (12, 3), sub_zone: KeepThroneRoom`.
2. **Chemical & Material Fidelity (Dwarf Fortress Item Model):**  
   An artifact is not a flavor label. An iron blade has structural wear, sharpness, and surface contaminants (`Cell.contaminants |= Contaminants::BLOOD`).
3. **Epistemic Fidelity (Information Physics):**  
   Characters do not have psychic omniscience. Information travels physically via line-of-sight, acoustic decibels, or dispatched human messengers.
4. **Psychological & Neurological Fidelity (CK3 Stress Engine):**  
   Utility AI is not a random number generator. Agents carry conflicting vows and a `stress: Q8_8` meter (0–100). When stress hits 100, a deterministic **Mental Break** triggers an irrational, high-stakes act (murder, abdication, self-exile).

---

## 3. RED TEAM PROTOTYPE FINDINGS: THE 5 CRITICAL GAPS

We built and executed a live simulation prototype pairing a 16×16 Micropolis tilemap, the Parquet Event Envelope, and a 4-panel Wildermyth-style graphic novel compiler.

The test proved the architecture is viable, but exposed **5 structural gaps** in our current simulation specs:

### Gap 1: The Prop Lifecycle (No Spontaneous Generation)
- **The Defect:** In procedural generation, murder weapons or poison phials often spawn ex nihilo when a drama trigger fires. This breaks determinism and causal plausibility.
- **The Requirement:** Every artifact used in an event must have an unbroken upstream ledger in the Parquet stream:
  - `Tick 2100`: Alchemist brews Nightshade (`CRAFT_ITEM`).
  - `Tick 5400`: Rogue smuggles it to the capital (`TRADE_TRANSFER`).
  - `Tick 8900`: Prince purchases it in secret (`ACQUIRE_BLACKMARKET`).
  - `Tick 10200`: Phial consumed in regicide (`USE_ITEM`).

### Gap 2: Epistemic Delay (Information Horizon)
- **The Defect:** If a King is murdered behind locked doors at midnight, the Lower Ward cannot riot at 00:01.
- **The Requirement:** The engine must enforce an information horizon:
  - An event produces an `EventKnowledge` token.
  - Initial `known_by = [Actor, Accomplice]`.
  - Information radius expands only upon:
    1. Corpse discovery by guards (shift change tick).
    2. Alarm broadcast (church bells, horn: radius $R = \text{decibels} \times \text{terrain\_damping}$).
    3. Eyewitness flight.

### Gap 3: Immediate Power Vacuum / Succession State-Machine
- **The Defect:** A realm cannot sit in a null-sovereign state across ticks while waiting for the next annual political cycle.
- **The Requirement:** When a titleholder dies, `M7.dynasty` must fire an immediate **Succession Arbitration Cascade** within 1 tick:
  - Check legal heir, claimant claims, military proximity, and treasury control.
  - Emit an immediate succession resolution or interregnum crisis event.

### Gap 4: Fugitive Pathfinding & Evacuation Vectors
- **The Defect:** After committing treason or assassination, the perpetrator cannot remain static or teleport to the wilderness.
- **The Requirement:** The actor must transition to `Role::FUGITIVE` with active spatial pathfinding constraints:
  - Pathfind along road/stealth tiles toward a border cell or sanctuary (Temple).
  - Interception checks against garrison patrol cells.

### Gap 5: Provenance Inscription (The Legends Mode Reverse Key)
- **The Defect:** When a tile burns or a building falls into ruin, if the grid cell only stores `kind = RUIN`, the historical context is destroyed.
- **The Requirement:** Every mutated tile, scarred NPC, or captured artifact must retain a reverse foreign key:
  - `Tile.last_mutating_event_id: EventId(4821)`
  - When the user clicks a ruined house in the 2D viewer 100 years later, the client pulls `Event #4821` and immediately reconstructs the graphic novel panel of the riot that burned it.

---

## 4. ACTIONABLE DIRECTIVE FOR THE SPRINT PIPELINE

1. **For `S6.artifacts` (Spec Sprint):**  
   Ensure artifacts have an origin entity, mass/volume/charge state, and contaminant bitflags. Zero spontaneous generation.
2. **For `S7.vows` & Actor State (Spec Sprint):**  
   Codify the CK3 Stress meter (0–100) and explicit Mental Break state transitions in Plane A.
3. **For `rh-events` (Crate Architecture):**  
   Keep the Event Envelope strictly factual (`EventId`, `Tick`, `Coords`, `ActorId`, `TargetId`, `ArtifactId`, `CausalEdges`, `ContaminantFlags`). Keep all visual, acoustic, and prose adjectives in Plane C downstream tools.
4. **For Downstream Tools (Plane C):**  
   Downstream tools consume Parquet and project onto:
   - 2D Micropolis tile grids (spatial diorama).
   - Wildermyth 4-panel storyboard prompts (Flux/SDXL).
   - Audio chapter scripts (Kokoro-82M).

---
*Signed,*  
**Red Team Leader & Quality Auditor**  
*Rehoboam Autonomous Swarm Engine*
