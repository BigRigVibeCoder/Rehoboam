# VISION PIVOT: THE "HERO VS. WHEAT" RECKONING
**Document ID:** `VISION_PIVOT.md`  
**Author:** Director & Chief Antagonist / Red Team Lead  
**Audience:** Architect Agent (`AGT-001`), Developer Agents (`AGT-002-*`), System Designers  
**Status:** PROPOSED DIRECTIVE FOR SPRINT PLANNING & ROADMAP REBALANCING  
**Date:** 2026-10-16  

---

## 1. THE DIRECTOR'S CONFESSION: "I AM NOT AN ECONOMIST"

We fell into a seductive trap over the last two weeks: **academic over-intellectualization.**

We collected 30+ peer-reviewed papers on medieval Italian grain price elasticity (Davenant, Allen 2001), demographic logit tables (Brass 1971), and structural-demographic political stress indices (Turchin). We designed an engine that can mathematically prove whether an agrarian civilization survives a 14th-century drought.

**And the Director realized something crucial:**  
> *"I think I'm gonna get bored with 'does this world survive a drought' simulation pretty fast... Nobody remembers the Lord of the Rings because Gondor had a mathematically sound wheat distribution."*

### The Core Realization
- **Wheat, demography, and taxes are not the story.** They are merely the **stage lighting, pressure cookers, and obstacles**.
- A harvest failure is only interesting if it drives a desperate Duke to steal an ancient cursed relic, or causes a young prince to betray his father's kingdom for the love of a rebel leader.
- If Rehoboam only simulates demographic weather, it is a failure of creative intent.
- **Rehoboam must be a Silmarillion / Dwarf Fortress Saga Engine, not an Excel simulator.**

---

## 2. WHO IS THE PLAYER? (THE ACTUAL CONSUMPTION MODEL)

The design documents spent months debating Godot 3D clients, VR bubbles, first-person street walks, and WASD controls. 

Let's strip away the gamer fantasies:
1. **The Director is not a gamer:** He is not going to sit at a desk with a gamepad walking around a procedural street corner inspecting cobblestones.
2. **The Director is a reader, a world-builder, and a lore cultivator:** He wants to put on headphones while working, listen to a rich episodic audio chronicle of his world's history, flip through generated graphic-novel storyboards, and scrub an interactive causal timeline.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              THE THREE CONSUMPTION MODES                               │
├──────────────────────────┬───────────────────────────┬────────────────────────────────┤
│ 1. THE SAGA PODCAST      │ 2. THE GRAPHIC CHRONICLE  │ 3. THE 2D AMBIENT DIORAMA      │
│    (Audio Drama / RSS)   │    (Storyboard / Comic)   │    (SimCity / DF Matrix View)  │
├──────────────────────────┼───────────────────────────┼────────────────────────────────┤
│ Multi-voice TTS          │ Key sifted scenes turned  │ Zero custom 3D rigging.        │
│ (Qwen3-TTS / Chatterbox) │ into visual novel panels  │ Bootstrapped open-source 2D    │
│ narrating generational   │ via local/cloud image     │ tile engine (Kenney CC0 tiles, │
│ triumphs & betrayals     │ generation (Flux/SDXL).   │ deck.gl glyphs, or retro       │
│ directly to your phone.  │ Clickable causal links.   │ SimCity/Micropolis grid).      │
└──────────────────────────┴───────────────────────────┴────────────────────────────────┘
```

---

## 3. THE MISSING PIECES: TURNING "WEATHER" INTO "FRODO"

Why doesn't the current architecture produce *The Lord of the Rings* or *A Song of Ice and Fire*?  
Because `M2 Demography` and `M4 Economy` model **passive aggregate drift**. Sagas require **active, directional, high-stakes vectors.**

The Architect must introduce three first-class mechanics into `Plane A`:

### 3.1 Artifacts as First-Class Physical Entities (`rh-artifacts`)
Currently, items are abstract trade goods (grain, cloth, iron). Sagas are driven by **Unique Singular Entities**:
- The Ring of Domination, the Broken Blade, the Crown of the First King, the Necrotic Grimoire.
- An Artifact has:
  - **Physical Location / Bearer:** (Held by Frodo in Coordinates `(X, Y)`).
  - **Psychological Corruption Field:** Modifies the bearer's `rh-minds` vector every tick (increasing Paranoia, Hubris, or Despair; decaying Filial Piety).
  - **Attraction Vector:** Other named actors with high Ambition perceive its rumor and generate goals to acquire it.

### 3.2 Vows, Grudges, and Quests (Directional Intent)
In our current design, characters react to immediate daily needs (eat, sleep, work). Frodo does not walk to Mount Doom to satisfy a daily utility need. **He is bound by an existential Vow.**
- A **Vow / Quest** is a persistent, non-routine agenda item that overrides daily life:
  $$\text{ActionPriority} = w_{\text{routine}} \cdot \text{Utility}_{\text{daily}} + w_{\text{quest}} \cdot \text{VowProgress}$$
- When a father is murdered, the prince takes a Vow: `AVENGE_KIN(Father, Murderer)`. That vow does not expire until blood is drawn or the prince dies.

### 3.3 Catalysts as Scenario Seeds
Instead of waiting 300 years for random mutations to spark a war, the world manifest must support **Mythic Catalysts**:
- *"In the Year 100, an ancient entity awakens in the deep caverns beneath the northern mountains."*
- *"A cursed relic of supreme power is inherited by a humble farmer's nephew."*
- The simulation takes that catalyst, and the unforgiving deterministic mechanics figure out who betrays whom to get it.

---

## 4. WHAT WE CAN BOOTSTRAP (NO 8-YEAR SPRITE ART TRAPS)

The Director is not spending 8 years drawing pixel art or modeling 3D tavern mugs. We will ruthlessly bootstrap existing open-source and free assets:

### 4.1 The Visuals (The "Dwarf Fortress / SimCity" 2D View)
- **Kenney.nl CC0 Asset Packs:** Tens of thousands of professionally drawn, public-domain 2D tiles (Medieval, Fantasy, Town, Castle, Roguelike). 100% free, commercial-friendly, zero legal risk.
- **`rot-js` & `deck.gl` Glyph Rendering:** High-performance web canvas or GPU-accelerated glyph rendering. Crisp, modern, beautiful retro aesthetics (like *Caves of Qud* or *Songs of Syx*) with dynamic lighting shaders.
- **The SimCity Grid Model:** Simple cellular automata for village growth. Buildings zone and evolve based on proximity to water and roads, not complex BIM architectural modeling.

### 4.2 The Narrative Output (The "Graphic Novel & Audio Drama")
- **The Sifter Dispatches to the Prompt Engine:** When the sifter detects a peak narrative climax (e.g., *Prince Eamon poisons the King's goblet at the Solstice Banquet*), it packages the scene into a structured storyboard prompt:
  - Characters present, emotional states, lighting, anchored setting.
- **Image Generation Pipeline:** Dispatched to local (SDXL/Flux) or cloud generation APIs to generate 3–5 dramatic visual panels.
- **Voice Synthesis:** Dispatched to Chatterbox or Qwen3-TTS to generate multi-character voice lines and narrator voice-over.
- **Result:** An episodic story feed delivered to a web dashboard or RSS feed. You read/listen to your world's history like a prestige HBO drama.

---

## 5. IMPACT ON CURRENT SPRINTS & ROADMAP

We do **not** throw away the foundation we just built. The core Rust architecture (`SPR-001` through `SPR-010`), Three-Plane state separation, and determinism are still 100% necessary. A good saga generator requires rock-solid causal tracking.

However, the **priorities** must shift:

```
OLD EMPHASIS                                NEW EMPHASIS
────────────                                ────────────
80% Macroeconomic Demography                30% Environmental Pressure (Famine, Plague, War)
20% Character Psychology                    40% Dynastic Ambition, Vows, Betrayals, Relics
0%  Quests & Heroic Arcs                    30% Story Sifting, Illustrated Chronicle, TTS
```

### Directives for the Architect (`AGT-001`):
1. **Keep P0 Core Intact:** Finish `SPR-002` (Genealogical IDs), `SPR-010` (Fixed-Point Math), and `SPR-003` (SoA tables). The clockwork foundation must exist.
2. **Elevate Artifacts and Vows into Plane A:** Design `rh-artifacts` and `rh-vows` so items and epic goals exist in the authoritative simulation state.
3. **Focus Presentation on the "Chronicle / Comic" Pipeline:** Prioritize the narrative output pipeline (text -> storyboard prompts -> TTS audio) over complex interactive GUI clients.
4. **Bootstrap Visuals from CC0:** Ban custom 3D asset creation. Use Kenney CC0 2D tiles and deck.gl/glyph layers for all visual views.

*The goal is not to simulate an agrarian economy. The goal is to generate legends.*
