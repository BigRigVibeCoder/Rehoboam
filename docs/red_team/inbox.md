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
* **Vulnerability:** Running millimeter physics or individual villager digestion during a siege would crush engine frame rates.
* **Architectural Fix:** A clean **Macro-to-Micro Impulse Protocol**:
  1. Strategic army encirclement downsamples to a simple contract: `ExternalInflow = 0` and `RoadAccess[Gate] = Blocked`.
  2. Local 2D grid layers (Micropolis cellular automata) handle the municipal decay naturally: granary drops $\rightarrow$ famine riot $\rightarrow$ cellular fire spread $\rightarrow$ internal gate breach.
  3. Damaged tiles store reverse foreign keys: `Tile.last_mutation_event_id = 4821`.
* **Target Spec:** Downstream 2D Settlement Bridge, `rh-events`.

---

### Item 5: Deterministic Hierarchical Vow Arbitration
* **Prototype:** `docs/red_team/prototypes/prototype_5_vow_collision.py`
* **Vulnerability:** When an agent holds contradictory vows (e.g., `DEFEND_KING` vs `AVENGE_FATHER_KILLED_BY_KING`), naive utility AI enters an infinite action oscillation or crash loop.
* **Architectural Fix:** `S7.vows` must codify:
  1. *Strict Priority Tiers:* `BLOOD_OATH (Tier 1) > FEALTY_OATH (Tier 2) > CHIVALRIC_DUTY (Tier 3) > CONTRACT (Tier 4)`.
  2. *Tragic Renunciation Protocol:* When collision occurs, dominant tier executes; subordinate tier triggers formal renunciation with deterministic costs (`Honor -= 40`, `Stress += 60`, `Reputation = 'Oathbreaker'`).
* **Target Spec:** `S7.vows`.
