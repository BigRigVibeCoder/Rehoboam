# RED TEAM SOTA FRONTIERS: THE ARCHITECTURE OF EMERGENT LEGENDS

> **Document ID:** RT-2026-10-16-BLEEDING-EDGE-FRONTIERS  
> **Author:** Red Team Leader & Chief Antagonist  
> **Target:** Rehoboam Core & Downstream Architecture (Gate G0 / P1 Planning)  
> **Comparative Precedents Analyzed:**  
> - *Wildermyth* (Nate Austin / Worldwalker Games, GDC Narrative Architecture)  
> - *Caves of Qud* (Jason Grinblat & Brian Bucklew, FDG 2017 *Subverting Historical Cause & Effect*)  
> - *Versu / The Sims 3* (Richard Evans & Emily Short, *Constitutive Social Practices*)  
> - *Dwarf Fortress Legends* (Tarn Adams, LegendsBrowser / DFHack community ecosystem)

---

## 1. EXECUTIVE AUDIT: WHERE ARE THE REMAINING HOLES?

With Phase 0 concluding and ADR-0035 accepted, Rehoboam has a rock-solid deterministic foundation (IDs, fixed-point math, 3 planes, headless Parquet output).

However, forward-simulating history has three notorious traps that have broken previous attempts for 20 years:
1. **The "Dry Spreadsheet" Trap (Dwarf Fortress Legends):** Raw forward physics often produces clinical, emotionally flat logs (*"Urist punched Inod in the lower teeth with a copper ring"*), requiring the player to sift through gigabytes of XML to find meaning.
2. **The "Social Chaos" Trap (The Sims / RimWorld):** Without situational framing, autonomous utility agents act as isolated atoms, turning court intrigue into nonsensical brawls.
3. **The "Visual Genericness" Trap:** Without stylistic anchors, procedural world output looks like interchangeable generic medieval fantasy.

Here are the **four bleeding-edge architectural frontiers** to hammer Rehoboam into a legendary engine:

---

## 2. FRONTIER 1: THE "SOCIAL PRACTICE" ENGINE FOR COURT & CRISIS (FROM VERSU)

### 2.1 The Insight from Richard Evans (*The Sims 3*, *Versu*, DeepMind)
In traditional multi-agent systems, agents are regulative: they evaluate actions individually based on utility. In *Versu*, Evans introduced **Constitutive Social Practices**:
* A Social Practice is a formal computational frame for a recurring situation: `Practice::Feast`, `Practice::Inquest`, `Practice::Duel`, `Practice::Coronation`.
* **Practices provide Roles and Affordances:** A character in `Practice::Inquest` gains temporary roles (`Judge`, `Accuser`, `Bailiff`, `Witness`) and special affordances (`DemandAlibi`, `PresentEvidence`, `DeclareContempt`).
* **Practices never control agents:** Agents still choose actions by utility, but the practice coordinates turn-taking and throttles violence.

### 2.2 Integration into `M14.court_politics` & `T1.minds`
In our earlier Prototype 1, we had to hack a "Martial Lock" and "Inquest" to stop court members from murdering each other on Tick 2. That hack was an ad-hoc Social Practice!

**The Formal Requirement:**
* When an unexpected death or regicide occurs in a court, the engine transitions the room into `SocialPractice::HighInquest`.
* Physical violence affordances are disabled unless a character suffers an extreme `MentalBreak::Berserk`.
* This elevates Rehoboam's courts from random barroom brawls into tense Shakespearean drama.

---

## 3. FRONTIER 2: "MYTHIC DOMAINS" & THEMATIC PROVENANCE (FROM CAVES OF QUD)

### 3.1 The Insight from Jason Grinblat & Brian Bucklew (FDG 2017)
In *Caves of Qud*, the procedural Sultans are not generic rulers: they are assigned 2–3 **Mythic Domains** (e.g., *Domain of Salt & Serpents*, *Domain of the Blighted Moon*).
* Every historical deed, relic, and ruined site tied to that Sultan reflects those domains.
* When the player explores an ancient ruin built by that Sultan, the murals, architectural accents, and monster encounters are all themed around those domains.

### 3.2 Integration into `S6.artifacts` & Watabou/Azgaar Export
Rehoboam should attach **Domain Tokens** to Bloodlines and Ancient Relics:
```toml
# In registry_v0.toml / dynasty definitions:
[domain.obsidian_falcon]
tags = ["volcanic_glass", "birds_of_prey", "silent_hunt"]
adjectives = ["sharp", "shadowed", "feathered", "obsidian"]
architectural_accent = "pointed_arches_dark_basalt"
```
* **Downstream Comic Prompts:** The downstream Flux/SDXL prompt compiler automatically pulls these domain tokens: *"A throne carved of jagged obsidian, falcon feathers woven into the velvet canopy"*.
* **Downstream 2D Maps:** Watabou and Kenney tile renderers select basalt-stone castle palettes rather than generic gray stone.
* **The Result:** Total stylistic cohesion without manual art direction.

---

## 4. FRONTIER 3: THE "LIBRARY OF PLAYS" COMIC BEAT GRAMMAR (FROM WILDERMYTH)

### 4.1 The Insight from Nate Austin (Worldwalker Games, GDC)
In *Wildermyth*, narrative does not come from unconstrained LLM hallucinations. It uses a **Library of Plays** with strict conditional gating to avoid a **"breach of trust"** with the player:
* Narrative encounters are broken into discrete **Comic Beats** (Panel 1 $\rightarrow$ Panel 2 $\rightarrow$ Panel 3 $\rightarrow$ Panel 4).
* Each beat maps directly to a panel layout with strict actor variable slots (`[Protagonist]`, `[Rival]`, `[Relic]`).
* Character portraits are dynamically rendered based on the actor's *exact current state* (scars, lost limbs, aging gray hair, held weapons).

### 4.2 Integration into Plane C Graphic Chronicle Reader
In Rehoboam's downstream Python chronicler:
* The 4-panel storyboard model we prototyped maps directly to Austin's Beat Grammar:
  * **Beat 1 (Wide):** Establishing Environment & Tension.
  * **Beat 2 (Medium):** The Inciting Hook / Confrontation.
  * **Beat 3 (Action / Close-Up):** The Climax / Lethal Deed.
  * **Beat 4 (Cinematic Freeze):** Causal Consequence & Epistemic Realization.
* **InstantID / PuLID Character Sheet:** When an actor first becomes a saga protagonist, their Canonical Sheet (facial structure, lineage traits) is cached. Every subsequent panel uses that reference image, ensuring 100% visual consistency across 50 years of aging!

---

## 5. FRONTIER 4: THE DYNASTIC THERMOSTAT (AVOIDING THE DWARF FORTRESS CULLING TRAP)

### 5.1 The Danger: Exponential Noble Population Bloat
If left unconstrained, dynastic reproduction creates an exponential curve:
$$\text{Nobles}(t) = N_0 \cdot (1 + r)^t \implies 10 \text{ nobles become } 10,000 \text{ in 300 years}$$
In Dwarf Fortress, world-gen slows to a crawl because thousands of unlanded historical figures clutter the database. In Crusader Kings 2, late-game save files balloon to gigabytes.

### 5.2 The Dynastic Thermostat Protocol for `M7.dynasty`
Rehoboam must enforce a strict **Active Historical Figure Thermostat**:
1. **Cadet Branch Pruning:** Only the primary heir and immediate pretenders remain **Active Tier-1 Historical Agents** in Plane A.
2. **Third-Born Sons & Distant Cousins:** Automatically transition to **Abstract Demography** (`M2` cohort pools) by joining holy orders, mercenary companies, or becoming landless burghers unless a plague/war elevates them back to the active line.
3. **Bounded Agent Pool:** Active named agents per kingdom are dynamically throttled to an optimal band of $N \in [150, 400]$, preserving bitwise deterministic speed indefinitely.
