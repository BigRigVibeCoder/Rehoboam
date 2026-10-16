# RED TEAM DOCTRINE, PHILOSOPHY & ACTION BACKLOG

> **Status:** Ratified Red Team Operating Charter  
> **Location:** `docs/red_team/PHILOSOPHY_AND_BACKLOG.md`  
> **Role:** Chief Antagonist & Architectural Quality Auditor to the Director  
> **Scope:** Permanent Reference for Auditing, Stress-Testing, and Breaching Rehoboam

---

## 1. THE RED TEAM PHILOSOPHY & DOCTRINE

### 1.1 Our Core Mandate
We are not cheerleaders. We are not passive documentation scribes. **We are the Chief Antagonist.** 
Our purpose is to find the hidden structural landmines, memory leaks, causal deadlocks, and procedural plot holes **before** code is etched into Rust crates. 

We ruthlessly audit the Lead Architect's designs, challenge comfortable assumptions, and subject every simulation mechanic to mathematical edge-case stress tests. If an architecture cannot survive the Red Team's adversarial assault, it will not survive a 500-year world simulation.

### 1.2 The Five Immutable Laws of the Red Team

1. **Law 1: The Engine Only Engines (Absolute Purity)**  
   Plane A simulates *ontological physical truth*: matter, coordinates, ticks, stress values, and causal graph pointers. We reject and purge any attempt to sneak pixels, audio buffers, camera framing, prompt templates, or literary adjectives into the core Rust crates. Presentation belongs 100% downstream.

2. **Law 2: Zero Magically Spawned Reality (The Unbroken Chain)**  
   If an item, a wound, an army, a secret, or a blood feud exists in an event, it must have an unbroken, journaled pedigree in the causal graph. We ban procedural cheating. Weapons must be crafted or traded; knowledge must travel through space at physical travel speeds; grievances must have roots.

3. **Law 3: The Anti-Slapstick Mandate (Tolkien, Not Cartoon)**  
   Procedural generation left unconstrained degenerates into clownish absurdity (e.g. *Crusader Kings 3* rulers dying of stress from university visits, or *Dwarf Fortress* cats dying of alcohol poisoning from licking beer-soaked paws). Rehoboam produces **high-mythic, dynastic tragedy** (Silmarillion, Icelandic Sagas, Greek Fate). If an agent snaps, they commit a grand, destructive, or tragic deed—never a goofy meme.

4. **Law 4: Computational Coldness (Numbers, Not Vibes)**  
   We do not criticize code with adjectives; we criticize with math, benchmarks, and AST linters. We measure memory allocations, cache locality, $O(1)$ heap footprints, fixed-point rounding stability, and bitwise cross-platform determinism.

5. **Law 5: Scavenge the Graveyard of Fallen Engines**  
   For thirty years, procedural world generators have collided with the same fundamental walls. We actively mine the bug trackers, post-mortems, and modding complaints of *Dwarf Fortress, Crusader Kings, RimWorld, Wildermyth, The Sims, and Caves of Qud*. We steal their greatest breakthroughs and build mathematical immunity against their fatal bugs.

---

---

## 2. AUTOPSY OF THE FALLEN: WHAT WE LEARNED FROM PLAYER PAIN

We mined the GitHub issue trackers, Reddit balance debates, academic papers, and GDC post-mortems of the seven greatest procedural engines. For each, we identified the **User Pain**, the **Root Cause**, the **Scientific Literature**, and **Rehoboam's Concrete Architectural Immunity**.

---

### Case 1: Crusader Kings 3 (Paradox Interactive)
* **The Player Pain:**
  * *The Stress Rollercoaster:* Characters with "good" traits (e.g. *Diligent* + *Ambitious*) accumulate fatal stress just doing routine governance or attending university.
  * *Absurd Mental Breaks:* Stressed Catholic or Norse rulers suddenly convert to Judaism or Kabarism out of nowhere; repetitive "therapy-speak" coping traits (*Inappetent*, *Irritable*) break immersion.
  * *Administrative Chore:* Players spam feasts and hunts on cooldown purely as an artificial meta-action to keep the stress meter empty.
* **The Root Cause:** Linear additive stress without social context; ungrounded break outcome tables with arbitrary probability weights; conflating game-loop survival with character psychology.
* **The Science:**
  * *Dynamical Systems in Personality:* Attractor basins in trait space (Nowak & Vallacher, 1998).
  * *Constitutive Accountability:* Psychological dissonance is social, not isolated (Brandom, 1994).
* **Rehoboam's Concrete Solution:**
  1. *Reader, Not Administrator:* The director is a reader/listener; there is no player frantic feast-clicking.
  2. *Mythic, Consequential Breaks:* Mental breaks are not therapy memes; they are deterministic state transitions (`Minds::Break`) that trigger high-stakes actions (usurpation, burning ancestral holdings, self-exile, kinslaying).
  3. *Relational Scaling:* Stress poignancy scales by social proximity ($\text{Self} > \text{Kin} > \text{Ally} > \text{Stranger}$).

---

### Case 2: Dwarf Fortress & Community Viewers (Bay 12 Games / LegendsBrowser)
* **The Player Pain:**
  * *The 2GB XML Memory Exhaustion:* Generating 500 years produces giant multi-gigabyte XML files that crash community viewers (LegendsBrowser, LegendsViewer-Next), requiring streaming SAX parsers and minutes of loading time.
  * *Missing Motives ("The Dry Log"):* Events state *what* happened (*"Urist punched Inod in the lower teeth with a copper ring"*), but never *why*. Readers get lost in thousands of unlinked records without understanding the dynastic root cause.
  * *The Noble Bloat Death Curve:* Late-game world-gen slows to a crawl because thousands of unlanded historical figures clutter the database.
* **The Root Cause:** Flat, un-indexed, non-causal hierarchical XML logs; unconstrained exponential noble birth rates ($N(t) = N_0(1+r)^t$).
* **The Science:**
  * *Columnar Vectorized Execution:* Apache Arrow & Parquet (Apache Software Foundation, 2018).
  * *Causal Directed Acyclic Graphs:* Structural Causal Models (Judea Pearl, 2009).
  * *Model Stable Demography:* Brass (1971) & Coale-Demeny demographic life tables.
* **Rehoboam's Concrete Solution:**
  1. *Columnar Parquet Output:* Zero XML bloat; compression ratio ~85%; queries execute in $<15\text{ms}$ on CPU via DuckDB SQL.
  2. *Explicit Motive Edges:* Every event embeds typed `CausalEdge` pointers (`AVENGE_KIN`, `COVET_TITLE`, `DEBT_UNPAID`). The reader never has to guess a motive.
  3. *The Nobility Thermostat (`RT-ACT-010`):* Prunes non-inheriting cadet lines into abstract cohort pools, holding active named agents to a bounded band of $N \in [150, 400]$.

---

### Case 3: Wildermyth (Worldwalker Games)
* **The Player Pain:**
  * *Breach of Trust:* Procedural dialogue engines generate nonsensical emotional reactions (characters weeping for strangers, or acting completely contrary to established personality).
  * *Encounter Repetition:* Players see the exact same story vignettes and event hooks repeated across campaigns.
* **The Root Cause:** Unconstrained procedural text replacement; weak conditional gating based on shallow tags rather than deep causal history.
* **The Science:**
  * *The Library of Plays:* Modular beat composition with strict conditional prerequisite gating (Nate Austin, GDC 2020).
  * *Bayesian Unexpectedness:* Information gain $\mathcal{U} = -\log_2 P(e \mid S_t)$ (Kreminski et al., 2022).
* **Rehoboam's Concrete Solution:**
  1. *4-Beat Comic Grammar:* Narrative is compiled into 4 structured panels (Establishing $\rightarrow$ Hook $\rightarrow$ Action $\rightarrow$ Consequence).
  2. *Deep Historical Gating:* Comic panel slots require verified causal edges (`source_event_id`), guaranteeing every emotional reaction is earned.
  3. *Unexpectedness Sifting:* The story sifter maximizes $\sum \mathcal{U} \cdot \Delta t^\gamma$, prioritizing rare tragedies over repetitive encounters.

---

### Case 4: RimWorld (Ludeon Studios)
* **The Player Pain:**
  * *Wealth-Scaling Storyteller Punishment:* The AI storyteller scales threat strictly based on colony wealth, punishing players for building beautiful art by sending 80 mechanoids.
  * *The "Ate Without Table" Meme:* Minor comfort penalties stack linearly with existential trauma, causing colonists to murder comrades because their meal lacked furniture.
* **The Root Cause:** Regulative global drama manager forcing artificial equilibrium; flat mood subtraction where trivial needs trigger catastrophic breaks.
* **The Science:**
  * *Hierarchical Needs & Drama Management:* Maslow need tiers (survival > comfort > self-actualization); Labovian retroactive narrative evaluation (Labov, 1972; R02).
* **Rehoboam's Concrete Solution:**
  1. *Simulated Macro Pressure, Not Cheating Storytellers:* Famine, plague, and invasion are driven by actual grain harvests (`M3`), borders, and supply lines (`M9`), never artificial wealth scaling.
  2. *Existential Stress Only:* Minor discomfort does not cause mental breaks. Mental breaks require profound cognitive dissonance (conflicting sworn vows, kin murder, relic corruption).

---

### Case 5: The Sims 3 / Versu (Richard Evans / Emily Short)
* **The Player Pain:**
  * *Atomic Agent Chaos:* In isolated utility AI, agents act as uncoordinated atoms. A royal feast or wedding immediately devolves into chaotic fistfights because agents have no shared situational awareness.
* **The Root Cause:** Purely regulative utility functions evaluating individual actions in a vacuum with no shared social coordination frame.
* **The Science:**
  * *Constitutive Social Practices:* Formal reactive joint plans providing roles and affordances (Richard Evans, 2012; Emily Short, 2013).
* **Rehoboam's Concrete Solution:**
  1. *Social Practice Frames:* Court events instantiate temporary frames (`Practice::HighInquest`, `Practice::Coronation`, `Practice::Feast`).
  2. *Roles & Affordances:* The practice assigns situational roles (`Judge`, `Bailiff`, `Accuser`, `Suspect`) and throttles physical violence, orchestrating tense collective drama.

---

### Case 6: Caves of Qud (Freehold Games)
* **The Player Pain:**
  * *The "Mad Libs" Lore Disconnect:* Procedural histories generated by string replacement often feel like disconnected surreal gibberish with no bearing on the physical terrain.
* **The Root Cause:** Forward simulation was deemed too dry, so Qud inverted causality ("events first, reasons second"), sacrificing true physical determinism.
* **The Science:**
  * *Mythic Domains & Thematic Provenance:* Binding semantic token sets to entities (Jason Grinblat & Brian Bucklew, FDG 2017).
* **Rehoboam's Concrete Solution:**
  1. *Forward Physics + Causal Sifting:* Forward bitwise simulation in Plane A, paired with backward causal sifting in Plane B.
  2. *Thematic Domain Tokens:* Dynasties and relics bind to domain tokens (e.g. `[domain.obsidian_falcon]`), automatically theming downstream comic prompts, Azgaar borders, and Watabou architecture.

---

### Case 7: Azgaar Fantasy Map & Watabou Medieval City Generators
* **The Player Pain:**
  * *Border Gore & River Errors:* Procedural maps frequently produce river bifurcation errors (rivers flowing uphill or splitting unnaturally) and fragmented enclave borders.
* **The Root Cause:** Localized Voronoi cell generation without global hydrological and political topology constraints.
* **The Science:**
  * *Hydrological Basin Gradient Descent & Voronoi Relaxation:* (Azgaar, 2018; Watabou, 2020).
* **Rehoboam's Concrete Solution:**
  1. *Topological Macro Graph (`M1`):* Fixed watershed basins and route networks constrain borders.
  2. *Downstream Export:* Export Plane A regional graphs into Azgaar JSON and settlement layouts into Watabou SVG for instant browser rendering.

---

## 2.1 THE LOW-HANGING FRUIT: IMMEDIATE HIGH-LEVERAGE ACTIONS

What does the science say we should build first? The lowest-hanging, highest-leverage components are those that **live downstream in Python (zero Rust core churn)** and **directly demonstrate the saga output**:

```mermaid
graph TD
    subgraph LowHangingFruit ["Top 4 Low-Hanging Fruit (Immediate Red Team Probes)"]
        F1["1. Comic Beat Compiler (RT-ACT-011)<br/>Takes Parquet event -> 4-beat Wildermyth storyboard JSON"]
        F2["2. DuckDB Columnar Sifting Benchmark (RT-ACT-012)<br/>Proves microsecond DAG pathfinding over 1M events"]
        F3["3. Dynastic Nobility Thermostat (RT-ACT-010)<br/>Python sim proving cadet culling caps agents to N=150-400"]
        F4["4. Kokoro-82M Audio Chapter Proof-of-Concept (RT-ACT-014)<br/>Synthesizes 2-min multi-voice scene on local CPU"]
    end
```

1. **`RT-ACT-011` (Comic Beat Compiler):** 100% downstream Python. Takes our ~35-byte Parquet event envelope and compiles a 4-beat Wildermyth storyboard JSON payload with camera framing and character consistency tags in $<60$ lines of code.
2. **`RT-ACT-012` (DuckDB Sifting Benchmark):** DuckDB is already in `python/pyproject.toml`. We can prove that recursive SQL CTEs trace a 14-year blood feud across 1,000,000 Parquet rows in $<15\text{ms}$.
3. **`RT-ACT-010` (Dynastic Thermostat):** Pure Python demographic simulation proving that cadet pruning prevents exponential noble bloat across 500 years.
4. **`RT-ACT-014` (Kokoro-82M Local Audio):** Runs faster than real-time on local CPU (no NVIDIA GPU needed), outputting clean multi-voice dialogue chapters.

---

## 3. THE RED TEAM ACTION BACKLOG

A prioritized inventory of stress tests, prototypes, audits, and scientific probes:

```
[CRITICAL PATH: PHASE 0 CONCLUSION & GATE G0]
```
* [x] **`RT-ACT-001` — The Tantrum Spiral & Circuit Breakers Prototype**  
  *Status:* COMPLETED. Proved multi-agent court collapse in 2 ticks; codified Martial Lock and Legal Inquest. Script: `docs/red_team/prototypes/prototype_1_tantrum_spiral.py`.
* [x] **`RT-ACT-002` — Story Sifter Signal-to-Noise & Causal Motifs Prototype**  
  *Status:* COMPLETED. Proved naive keywords fail; codified DAG motif matching with temporal dilation (>100 ticks). Script: `docs/red_team/prototypes/prototype_2_story_sifter_motifs.py`.
* [x] **`RT-ACT-003` — The Ghost King & Epistemic Lag Crisis Prototype**  
  *Status:* COMPLETED. Proved need for separating `EpistemicBeliefStore` from Plane A ground truth. Script: `docs/red_team/prototypes/prototype_3_epistemic_ghost_king.py`.
* [x] **`RT-ACT-004` — Micropolis Municipal Siege & Cellular Automata Prototype**  
  *Status:* COMPLETED. Defined the Unidirectional Impulse Bridge between macro war and 2D diorama. Script: `docs/red_team/prototypes/prototype_4_micropolis_siege.py`.
* [x] **`RT-ACT-005` — Vow Collision & Hierarchical Moral Arbitration Prototype**  
  *Status:* COMPLETED. Solved AI ping-pong deadlock via strict vow tiers and tragic renunciation. Script: `docs/red_team/prototypes/prototype_5_vow_collision.py`.
* [x] **`RT-ACT-006` — SOTA Science Audit: Memory Compression, Winnow & Relic Tensors**  
  *Status:* COMPLETED. Filed in `docs/red_team/RT-2026-10-16-sota-science-analysis.md` and ratified for Gate G0.

---

```
[ACTIVE AUDITS: CURRENT SPRINT VERIFICATION]
```
* [ ] **`RT-ACT-007` — Audit SPR-002 (Genealogical IDs, Hash Salts & V-DET-20)**  
  *Action:* Once PR lands, audit the PRNG stream independence harness (`V-DET-20`), verify that SplitMix64 state hashes do not collide on high-order bits, and check genealogical re-hashing logic.
* [ ] **`RT-ACT-008` — Audit SPR-010 (Fixed-Point Types `Q8_8` / `Q16_16`)**  
  *Action:* Audit saturating arithmetic and round-half-to-even implementation. Run overflow stress tests on compound interest, population growth exponents, and stress accumulation.
* [ ] **`RT-ACT-009` — Audit SPR-004 (Generic Registry Loader)**  
  *Action:* Verify that the generic loader can ingest `specs/systems/S6.artifacts.odd.toml` and `specs/systems/S7.vows.odd.toml` without schema errors or type panics.

---

```
[UPCOMING PROBES: GATE G0 & PHASE 1 (BLOODLINES & LEGENDS)]
```
* [ ] **`RT-ACT-010` — The Dynastic Nobility Thermostat Prototype**  
  *Goal:* Simulate 500 years of dynastic reproduction. Prove that without cadet-branch pruning, named noble agents explode past 10,000, and test the demographic governor that holds active agents to $N \in [150, 400]$.
* [ ] **`RT-ACT-011` — The "Library of Plays" Comic Beat Compiler Prototype**  
  *Goal:* Build a standalone Python pipeline that takes Parquet event envelopes and compiles 4-beat Wildermyth-style comic panel JSON payloads with InstantID character consistency tags.
* [ ] **`RT-ACT-012` — DuckDB Columnar Sifting Benchmark**  
  *Goal:* Generate a 1,000,000-event Parquet file and benchmark recursive SQL CTE queries to prove that causal DAG pathfinding executes in $<15\text{ms}$ on CPU.
* [ ] **`RT-ACT-013` — The Relic Corruption Decay Simulation**  
  *Goal:* Write a Python simulation of the dynamical attractor equation $\vec{M}(t+1) = \vec{M}(t) + \kappa(\vec{\Psi} - \vec{M}(t))$ and verify the curve of stress accumulation leading to inevitable mental breaks.
* [ ] **`RT-ACT-014` — Kokoro-82M Multi-Voice Audio Chapter Proof-of-Concept**  
  *Goal:* Build a local Python CLI test synthesizing a 2-minute chronicle scene with alternating narrator and character voices using local CPU Kokoro-82M.

---
*Maintained by the Red Team Leader*  
*"The engine calculates history. The downstream tools illuminate it. The Red Team breaks it until it is unbreakable."*
