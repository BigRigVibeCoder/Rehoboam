# ARCHITECT MASTER DIRECTIVE: THE HEADLESS SAGA ENGINE
**Document ID:** `ARCHITECT_HANDOFF_GRAPHICS_ROADMAP.md`  
**From:** Director & Chief Antagonist / Red Team  
**To:** Architect Agent (`AGT-001`), Developer Agents (`AGT-002-*`)  
**Status:** FORMAL RATIFICATION, ARCHITECTURAL BOUNDARY & TECHNICAL SPECIFICATION  
**Date:** 2026-10-16  

---

## 1. THE FOUNDATIONAL MANDATE: "THE ENGINE ONLY ENGINES"

We hereby establish an absolute, non-negotiable architectural boundary:
**The simulation engine does one thing: it calculates the world and streams out the Matrix feed (`Channel A` causal events and `Channel C` narrative matches into Arrow/Parquet).**

1. **Zero Media Bloat in Core:** The Rust engine (`rh-core`, `rh-sys-*`, `rh-events`) contains **zero** presentation, web-serving, audio, image-generation, or LLM-inference code. It does not know what an MP3, PNG, HTML tag, or diffusion model is.
2. **CI-Enforced Boundary:** An AST/dependency architecture gate will immediately fail any simulation crate that pulls in web, media, or LLM crates.
3. **Decoupled Downstream Consumption:** Everything the Director reads, sees, or hears is produced by **independent, offline downstream tools** (Python scripts/CLIs) that read the finished `events.parquet` chronicle. The simulation engine never waits for media rendering.
4. **Podcast / RSS Pipeline is Permanently Dead:** All RSS 2.0 generation, local podcast server hosting, and mobile syndication are deleted from the roadmap. Downstream presentation is strictly a local **Web Graphic Novel / Storyboard Reader**.

---

## 2. THE REBALANCED ROADMAP (OFFICIALLY RATIFIED)

| Phase | Title | Core Deliverables |
|---|---|---|
| **P0** | **Foundations** | `SPR-001` through `SPR-010` (Workspace, IDs, Fixed-Point, Determinism CI). |
| **P1a** | **"Bloodlines"** | Dynasties, succession, kinship, named characters with ambition & grudges, chronicle. |
| **P1b** | **"Legends"** | `S6.artifacts` & `S7.vows` as authoritative Plane A state, mythic catalysts, secrets, lean famine/harvest. |
| **P2** | **"Sagas & Audio v0"** | Saga sifter automata (`Dramatis`), keyboard Time-Machine, offline audio reader. G2 thesis gate. |
| **P3** | **"Illustrated Saga"** | Graphic chronicle (storyboard prompts + serverless comic panels), multi-voice audio, web reader. |
| **P4** | **"Deeper Worlds"** | Full rumor/belief networks, epidemic dynamics, war logistics, Oracle mode. |
| **P5** | **"2D Ambient Diorama"** | Bootstrapped 2D tilemap diorama (Kenney CC0, retro map visualizer). |
| **CUT** | **All 3D & VR** | Godot 3D, procedural mesh generation, skeletal animation, and VR bubbles are permanently cut. |

---

## 3. THE "CINEMATIC EVENT ENVELOPE" (`crates/rh-events`)

Downstream comic, novel, and audio tools cannot generate compelling art from dry accounting logs (`Event { actor, target, verb }`). 

To produce cinematic stories without engine bloat, `crates/rh-events` must record the **Cinematic Event Envelope**—a structured ~35-byte integer/enum snapshot embedded in `events.parquet`:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        THE CINEMATIC ENVELOPE (STREAM EVENT)                           │
├──────────────────────────┬─────────────────────────────────────────────────────────────┤
│ 1. THE CAST & VISUAL DNA │ Who is present; deterministic visual appearance tokens.     │
├──────────────────────────┼─────────────────────────────────────────────────────────────┤
│ 2. STAGING & ATMOSPHERE  │ Environment type, weather, time-of-day, dynamic lighting.   │
├──────────────────────────┼─────────────────────────────────────────────────────────────┤
│ 3. THE EMOTIONAL VECTOR  │ Actor & target emotional state deltas (faces & posture).    │
├──────────────────────────┼─────────────────────────────────────────────────────────────┤
│ 4. THE ACTION & PROPS    │ Physical verb, exact artifact used (condition, glow, blood).│
├──────────────────────────┼─────────────────────────────────────────────────────────────┤
│ 5. THE CAUSAL "WHY"      │ Foreign keys to inciting trauma (retaliation, jealousy).    │
└──────────────────────────┴─────────────────────────────────────────────────────────────┘
```

### The 5 Field Specifications:
1. **Cast & Visual DNA (Plane C on-demand):**  
   Appearance is derived dynamically from lineage, age, scars, and carried relics. Children resemble parents; battle wounds and curses update appearance automatically without polluting Plane A state.
2. **Atmospheric Staging:**  
   Every event snapshot captures four environmental integer enums: `TimeOfDay` (e.g., `MoonlessMidnight`), `Lighting` (e.g., `FlickeringTorchlight`), `Weather` (e.g., `HowlingBlizzard`), and `PlaceAcoustic` (e.g., `VaultedStoneHall`).
3. **Emotional Vector Deltas:**  
   Captures actor and target emotional states at the second of execution: `PAD mood` + `OCC emotion` (e.g., `ActorEmotion: ColdFury`, `TargetEmotion: BetrayedHorror`). This gives image models the exact facial expression and audio models the vocal tone.
4. **Physical Props & Artifacts (`S6.artifacts`):**  
   Explicit unique item IDs: `ItemUsed: CursedBladeOfGondor`, `PropStatus: BloodDripping`.
5. **Causal Motive Links (The DAG Edge):**  
   Explicit typed pointers to root causes: `CausalEdge: RETALIATION_FOR(Event #812: Public Execution of Mother)`. Downstream tools know the historical trauma without searching.

---

## 4. WHAT WE STEAL FROM THE GIANTS

### 4.1 From *Crusader Kings 3*: Stress & Mental Breaks (`crates/rh-minds`)
* **The Problem:** Pure utility AI characters behave like cold, calculating chess grandmasters. Sagas need characters to act irrationally out of grief, rage, and pride.
* **The Solution:** Adopt CK3’s **Stress Meter**. When a character's Vow or Kin-Loyalty conflicts with their situation (e.g., forced to bend the knee to their father's murderer), **Stress accumulates**.
* **The Trigger:** High stress forces a **Mental Break**. The character snaps: they abandon rational self-preservation and execute desperate, volatile acts (assassination, suicide pacts, burning dynastic holdings).

### 4.2 From *Wildermyth*: The 4-Panel Comic Framing
* The downstream comic tool (`rehoboam-comic`) structures sifted sagas into classic 4-panel visual storyboards:
  - **Panel 1 (The Staging):** Wide shot, lighting, atmosphere, setting the scene.
  - **Panel 2 (The Confrontation):** Medium shot, tension, drawn weapon, whispered accusation.
  - **Panel 3 (The Climax):** Close-up dramatic action, poison, strike, betrayal.
  - **Panel 4 (The Consequence):** Aftermath, dying realization, escape into the blizzard.

### 4.3 Beyond *Dwarf Fortress*: True Causal Lineage
* Dwarf Fortress exports massive unlinked XML event logs; players must act like forensic historians to guess why a king was murdered.
* Rehoboam's competitive advantage is the **Explicit Causal DAG**. Every tragedy points directly to its origin event.

---

## 5. DOWNSTREAM PRESENTATION & REUSE ARCHITECTURE

### 5.1 The Graphic Chronicle Pipeline (P3)
- **Character Consistency via Reference Conditioning:**  
  When an actor becomes a saga protagonist, a **Panel 0: Canonical Character Sheet** portrait is generated once. All action panels pass Panel 0 into reference conditioning (InstantID, PuLID, or IP-Adapter).
- **Stylistic Armor:**  
  Strictly ban photorealism. Mandate descriptive dark fantasy graphic novel styling:
  > *"Dark fantasy graphic novel style, heavy ink cross-hatching, high-contrast chiaroscuro, muted watercolor wash, bold angular silhouettes, gritty textured paper."*
- **Serverless Cloud APIs:**  
  Rendered asynchronously via serverless diffusion APIs (Fal.ai, Replicate) at **~$0.01 per panel** (~$0.10/chapter). Zero local GPU load.

### 5.2 2D Map & Settlement Bootstrap (P1b / P2 / P5)
- **Azgaar’s Fantasy Map Generator (Open Source, JS/SVG):** Export the simulation’s region graph into Azgaar JSON for an instant, interactive browser world map with political borders and troop routes.
- **Watabou’s Medieval City Generator (Open Source):** Procedurally generates 2D vector town/castle layouts from world seeds.
- **Kenney.nl CC0 Asset Packs:** Public-domain medieval/fantasy tilesets for 2D settlement diorama rendering.

### 5.3 Audio Drama Pipeline (P2 / P3)
- **Kokoro-82M (Apache-2.0):** Ultra-fast, broadcast-quality narration running locally on CPU.
- **Offline Audio Exporter:** Python script reads the chronicle, synthesizes voice tracks, mixes ambient soundscapes, and saves local MP3 chapters playable directly in the Web Graphic Novel Reader.

---

## 6. IMMEDIATE WORK PACKAGES

1. **Merge PR #2:** Finalize defect fixes (DEF-1 through DEF-5) and licence ledger updates.
2. **Execute Active Sprints:**
   - `SPR-002`: Genealogical IDs (`H64`) & SplitMix64 RNG stream independence.
   - `SPR-004`: Mechanics Registry loader.
   - `SPR-010`: Fixed-Point numeric types (`Q8_8`, `Q16_16`, integer cents).
3. **Draft ODD-R Specs for `S6.artifacts` & `S7.vows`:** Machine-checked specs defining artifact corruption fields, bearer vectors, and vow-completion failure modes.

*The engine calculates history. The downstream tools illuminate it.*
