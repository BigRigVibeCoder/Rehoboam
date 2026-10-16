# Red Team Design Inbox (Batched for Gate G0)

> **Purpose:** Consolidated design inbox maintained by the Red Team for the Lead Architect.  
> **Rule:** In-flight defect fixes are communicated immediately; all architectural and gameplay design enhancements are batched here for formal incorporation at project planning gates (next gate: **Gate G0** at the conclusion of P0).

---

## Batch 01 (2026-10-16): Five Stress-Tested Architectural Enhancements

All items below have been validated with executable Python prototypes located in `docs/red_team/prototypes/` (`run_all.sh`).

### Item 1: Institutional Circuit Breakers (Anti-Tantrum Spiral)
* **Prototype:** `docs/red_team/prototypes/prototype_1_tantrum_spiral.py`
* **Vulnerability:** Unrestrained stress accumulation in multi-agent court networks causes positive feedback death spirals (50%+ of kingdom rulers dead in 2 ticks from cascading grief/berserk breaks).
* **Architectural Fix:** Courts must feature **Institutional Circuit Breakers**:
  1. *Martial Lock:* Commander/Marshal enforces a physical violence freeze upon sudden death (`violence_locked = true`).
  2. *Legal Inquest:* Shock channels into formal interrogation and suspicion meters, capping raw stress below 80/100 until guilt is established.
  3. *Regency Coalescence:* Factions immediately organize around the strongest legal claimant rather than acting as atomic vigilantes.
* **Target Spec:** `M7.dynasty`, `M14.court_politics`.

---

### Item 2: Causal Motif Grammar for `rh-sifter`
* **Prototype:** `docs/red_team/prototypes/prototype_2_story_sifter_motifs.py`
* **Vulnerability:** Naive keyword searches over 250,000 Parquet event rows drown the player in mundane economic/tavern noise (e.g. 950 peasant cabbage trades) while missing true legends.
* **Architectural Fix:** `rh-sifter` must support a graph-matching motif grammar defining:
  1. *Directed Causal Topology:* Explicit sequence patterns (e.g. `EXECUTION -> VOW -> LATENCY -> ASSASSINATION -> EPISTEMIC_REALIZATION`).
  2. *Temporal Dilation Filter:* Minimum tick latency thresholds (e.g. $\Delta t > 100$ ticks / multi-year grudge).
  3. *Emotional Polarity Inversion:* True sagas require radical emotional shifts (`ArrogantPride -> DelusionalTerror`, `Grief -> ColdHatred`).
* **Target Spec:** `rh-sifter`, `specs/sifting/`.

---

### Item 3: Ground Truth vs. Epistemic Belief Separation (The "Ghost King" Crisis)
* **Prototype:** `docs/red_team/prototypes/prototype_3_epistemic_ghost_king.py`
* **Vulnerability:** If agents directly query Plane A world state, tragic misunderstandings (e.g. Romeo & Juliet, false usurpations, ghost king returns) are mathematically impossible.
* **Architectural Fix:** Agent cognition must be strictly decoupled from Plane A ground truth:
  1. Each agent owns an `EpistemicBeliefStore { entity_id -> (BelievedStatus, Confidence, Timestamp) }`.
  2. Beliefs are mutated solely by arriving `InformationTokens` transmitted physically across the road network ($D / V$).
  3. Enables realistic crises: Duke Valerius crowns himself believing King Alden died at the border, triggering a tragic civil war when Alden returns alive.
* **Target Spec:** Agent Cognition Architecture (`research/R04`, `S2.agents`).

---

### Item 4: Macro-to-Micro Municipal Impulse Bridge (Micropolis 2D Grid)
* **Prototype:** `docs/red_team/prototypes/prototype_4_micropolis_siege.py`
* **Vulnerability:** Running millimeter physics or individual villager digestion during a siege would crush engine frame rates. Conversely, if a downstream 2D grid simulation could cause riots or breaches that feed back into the world state, it would violate engine headless purity.
* **Architectural Fix (Unidirectional Contract):**
  1. *Plane A Simulates & Decides:* The Rust core simulates famine (`M4`) and siege warfare (`M9`), deterministically emitting authoritative events (e.g. `FAMINE_RIOT`, `GATE_BREACHED`).
  2. *Micropolis is Pure Receiver:* The 2D tile engine never feeds back into Plane A. It acts strictly as an impulse consumer, visually rendering cellular fire spread, fleeing citizen sprites, and crumbled walls to match the Parquet chronicle.
  3. *Zero Rust Grid Physics:* The Rust core never knows what a tile or pixel is; the user gets the nostalgic 2D SimCity spectacle purely downstream.
* **Target Spec:** Downstream 2D Settlement Bridge, `rh-events`.

---

### Item 5: Deterministic Hierarchical Vow Arbitration
* **Prototype:** `docs/red_team/prototypes/prototype_5_vow_collision.py`
* **Vulnerability:** When an agent holds contradictory vows (e.g., `DEFEND_KING` vs `AVENGE_FATHER_KILLED_BY_KING`), naive utility AI enters an infinite action oscillation or crash loop.
* **Architectural Fix:** `S7.vows` must codify:
  1. *Strict Priority Tiers:* `BLOOD_OATH (Tier 1) > FEALTY_OATH (Tier 2) > CHIVALRIC_DUTY (Tier 3) > CONTRACT (Tier 4)`.
  2. *Tragic Renunciation Protocol:* When collision occurs, dominant tier executes; subordinate tier triggers formal renunciation with deterministic costs (`Honor -= 40`, `Stress += 60`, `Reputation = 'Oathbreaker'`).
* **Target Spec:** `S7.vows`.

---

## Batch 02 (2026-10-16): SOTA Science Upgrades (For Gate G0)

Full technical analysis in `docs/red_team/RT-2026-10-16-sota-science-analysis.md`.

### Item 6: The Deterministic "Reflection Tree" (Park et al. 2023 Adaptation)
* **Source:** `upload/Park2023Generative.pdf` (§4.1–§4.3, Figure 7).
* **The Insight:** Park et al.'s primary innovation is **not** the LLM, but the hierarchical memory architecture: raw perceptions ($I \in [1, 100]$) fold into abstract reflections when $\sum I \ge 150$.
* **Plane A Implementation:** Implement symbolic reflection trees in `T1.minds` without LLMs. When importance crosses threshold, cluster folds into a compact `OpinionKnot` or `Vow`, and raw events are evicted from RAM to cold Parquet disk, preserving $O(1)$ agent memory footprint across centuries.
* **Target Spec:** `T1.minds` (P1a).

### Item 7: Narrative Unexpectedness Metric $\mathcal{U}$ (Kreminski et al. 2022)
* **Source:** `research/reading_list.md` (§10, B4: Kreminski et al. 2022, "Select the Unexpected").
* **The Insight:** Replace ad-hoc drama heuristics with Bayesian information gain: $\mathcal{U}(e) = -\log_2 P(e \mid S_t)$.
* **Plane A/B Implementation:** Low-probability actions (e.g. an honorable knight slaying their king) yield high bit-values of unexpectedness. Sagas are ranked by maximizing $\sum \mathcal{U}(e_i) \cdot \Delta t^\gamma$, mathematically isolating epic tragedies from mundane peasant events.
* **Target Spec:** `rh-sifter` (P2).

### Item 8: The Relic Corruption Tensor & Attractor Basin
* **Source:** Dynamical systems modeling for `specs/systems/S6.artifacts.odd.toml`.
* **The Insight:** Define `artifact.influence_profile` as a target corruption attractor vector $\vec{\Psi}_{\text{relic}}$ with monthly coupling rate $\kappa$.
* **Plane A Implementation:** Bearer's personality vector $\vec{M}$ experiences exponential decay toward $\vec{\Psi}$ over possession time: $\vec{M}(t+1) = \vec{M}(t) + \kappa \cdot (\vec{\Psi} - \vec{M}(t))$. Creates continuous, emergent Ring-corruption driving vow conflict and mental breaks.
* **Target Spec:** `S6.artifacts` (P1a/P1b).

---

## Batch 03 (2026-10-16): The Red Team Charter & Graveyard Autopsies (For Gate G0)

Full technical dossiers in `docs/red_team/PHILOSOPHY_AND_BACKLOG.md` and `docs/red_team/RT-2026-10-16-bleeding-edge-frontiers.md`.

### Item 9: The Red Team Operating Charter & Action Backlog
* **Source:** `docs/red_team/PHILOSOPHY_AND_BACKLOG.md`.
* **The Insight:** Establishes the 5 Immutable Laws of the Red Team (Engine Only Engines, Zero Spontaneous Reality, Anti-Slapstick, Computational Coldness, Scavenge the Graveyard) and an active 14-item backlog (`RT-ACT-001` through `RT-ACT-014`) tracking audits and probes.

### Item 10: Autopsies of the Fallen & 4 Bleeding-Edge Frontiers
* **Source:** `docs/red_team/RT-2026-10-16-bleeding-edge-frontiers.md`.
* **The Insight:** Synthesizes community complaints and proven scientific solutions across 7 engines:
  1. *Constitutive Social Practices (from Versu / Richard Evans):* Formal situational frames (`Practice::HighInquest`, `Practice::Feast`) providing temporary roles and violence throttling to stop multi-agent court chaos.
  2. *Thematic Mythic Domains (from Caves of Qud):* Dynasties and relics bind to domain tokens (e.g., `[domain.obsidian_falcon]`), automatically styling downstream comic prompts, Azgaar maps, and Watabou architecture.
  3. *Library of Plays 4-Beat Comic Grammar (from Wildermyth):* Compiles Parquet events into structured visual beats (Establishing -> Hook -> Action -> Consequence) with InstantID consistency.
  4. *The Dynastic Nobility Thermostat:* Cadet-branch pruning holding active named actors to a bounded equilibrium of $N \in [150, 400]$, preventing the Dwarf Fortress/CK2 noble explosion.
* **Target Specs:** `M14.court_politics`, `M7.dynasty`, `S6.artifacts`, Downstream Narrative Compiler.

---

## Batch 04 (2026-10-16): The 10 Fallen Bugs Autopsy & 4 New Live Prototypes (For Gate G0)

Full technical analysis in `docs/red_team/RT-2026-10-16-simulation-bug-autopsy-and-code-theft.md`. All 4 items validated with executable prototypes in `docs/red_team/prototypes/` (`run_all.sh` 9/9 passing).

### Item 11: Priority-Flood Monotonic Hydrology & Natural Borders
* **Prototype:** `docs/red_team/prototypes/prototype_6_priority_flood_hydrology.py`
* **Vulnerability:** World-generators (Azgaar, DF) suffer from depression pit sinks, lake overflows, and river looping where rivers flow uphill or terminate in dry fields.
* **Architectural Fix:** Adopt Barnes, Lehman, & Mulla (2014) Priority-Flood algorithm. Uses an $O(N \log N)$ min-heap priority queue flooding inward from the ocean boundary, raising all depression sinks to spill elevation and computing monotonic D8 flow accumulation. 100% of river trunks reach the ocean; rivers form immutable natural borders for polities.
* **Target Spec:** `M1.topography`, `M1.hydrology`.

### Item 12: The Feudal Nobility Thermostat (Bounded Active Figures)
* **Prototype:** `docs/red_team/prototypes/prototype_7_nobility_thermostat.py`
* **Vulnerability:** Unconstrained noble reproduction causes exponential explosion ($N_0(1+r)^t$), ballooning to 2,500+ nobles by Year 208 and 900,000+ by Year 500, causing catastrophic CPU tick lag and save-file bloat.
* **Architectural Fix:** Enforce feudal seat capacity:
  1. Only primary heirs inherit landed seats (Tier 1 Active).
  2. Spares / 2nd-born act as active rivals/claimants.
  3. 3rd-born and distant cadet lines undergo downward social mobility (Holy Orders, Mercenary companies, or M2 burgher cohort pools).
  4. Bounds active historical nobles permanently to $[150, 350]$ across 500 simulated years.
* **Target Spec:** `M7.dynasty`, `M2.demography`.

### Item 13: Sub-Millisecond Recursive SQL CTE Causal DAG Sifter
* **Prototype:** `docs/red_team/prototypes/prototype_8_causal_sifter_sql.py`
* **Vulnerability:** Felt/Winnow graph isomorphism algorithms suffer from exponential combinatorial search times $O(V^k)$ over $10^5$ events.
* **Architectural Fix:** Columnar Parquet/SQLite events with indexed `cause_event_id` foreign keys. Backward causal DAG traversal executes via recursive SQL Common Table Expressions (CTEs) in **0.197 ms** across 100,000 events, isolating the 52.5-bit Kreminski unexpectedness revenge saga while filtering 99.995% of background peasant noise.
* **Target Spec:** `rh-sifter`, `specs/sifting/`.

### Item 14: 4-Beat Wildermyth Comic Storyboard Compiler
* **Prototype:** `docs/red_team/prototypes/prototype_9_comic_beat_compiler.py`
* **Vulnerability:** Raw simulation event logs are emotionally dry; unconstrained LLM generation hallucinates slapstick and loses visual character continuity.
* **Architectural Fix:** Compiles sifted Parquet causal envelopes into an authoritative 4-beat comic layout (Establishing Wide $\rightarrow$ Inciting Hook $\rightarrow$ Action Climax $\rightarrow$ Tragic Consequence) with structured InstantID visual anchor prompts ensuring 100% character face/costume consistency across decades.
* **Target Spec:** Downstream Graphic Novel Reader, Plane C.

### Item 15: The 10 Fallen Bugs Immunity Protocol
* **Vulnerability:** Dwarf Fortress pet-door pathing storms, item clutter, CDDA nested container recursion freezes, and RimWorld GC pauses.
* **Architectural Fix:** Codify 10 permanent engineering invariants in `GOV-008` & `CLAUDE.md`: zero unowned map debris, flat non-recursive item slots (`[Option<ItemId>; 6]`), zero heap allocation in tick loops, monthly climate zones, and static road graph Dijkstra caching.
* **Target Spec:** `GOV-008`, `CLAUDE.md`.


