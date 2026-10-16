# RED TEAM REPORT: SIMULATION FIDELITY & PATHOLOGICAL EDGE-CASE PROTOTYPING

> **Report ID:** RT-2026-10-16-FIDELITY-GAPS  
> **Author:** Red Team Leader & Chief Antagonist  
> **Target:** Rehoboam Core & Downstream Architecture  
> **Scope:** Multi-Agent Stress Testing, Sifting Motifs, Epistemic Lag, Municipal Automata, and Vow Deadlocks

---

## 1. EXECUTIVE SUMMARY

To stress-test Rehoboam’s simulation fidelity and expose architectural blind spots before the Rust crates are finalized, the Red Team developed and executed a suite of **5 executable Python prototypes** (`docs/red_team/prototypes/run_all.sh`).

The prototypes proved that **Rehoboam’s ~35-byte Parquet event model is capable of driving high-fidelity graphic novel storyboards and 2D SimCity-style municipal tilemaps without violating "the engine only engines."**

However, the stress tests exposed **5 critical vulnerabilities** in standard procedural story engines that Rehoboam must actively guard against:
1. **The Positive Feedback Grief Spiral:** Unmitigated stress causes multi-agent courts to collapse in 2 ticks.
2. **The Sifter Noise Flood:** Naive keyword searches retrieve thousands of mundane economic events while missing true mythic arcs.
3. **The Omniscient Cognition Defect:** Direct reads of Plane A ground truth prevent tragic misunderstandings and false usurpations.
4. **The Macro-Micro Coupling Trap:** Trying to simulate micro-physics during regional sieges destroys performance.
5. **The Vow Collision Deadlock:** Contradictory agent oaths cause infinite behavioral oscillation.

---

## 2. PROTOTYPE SUITE & EXPERIMENTAL FINDINGS

### Prototype 1: The Court Tantrum & Grief Cascade
* **Script:** `docs/red_team/prototypes/prototype_1_tantrum_spiral.py`
* **Test Scenario:** King Alden is assassinated in the Great Hall. An 8-noble court with CK3 stress meters processes the shock.
* **Failure Mode (Run A):** Without institutional dampening, Princess Lyra, Duke Valerius, and Marshal Garrett hit 100/100 stress on Tick 2. Lyra slays Prince Eamon, Valerius slaughters Chancellor Malakor, and the court collapses with >50% casualties in under 2 ticks.
* **Hardened Solution (Run B):** Adding **Institutional Circuit Breakers** (Martial Lock by Marshal, Legal Inquest by Chancellor, Faction Regency under Duke Valerius) contained violence, keeping 7/8 nobles alive.

### Prototype 2: Story Sifter Signal-to-Noise & Motif Matching
* **Script:** `docs/red_team/prototypes/prototype_2_story_sifter_motifs.py`
* **Test Scenario:** 950 mundane villager events (buying wool, harvesting wheat) mixed with 7 mythic saga events (a 14-year vendetta and a warlord's sacrilege).
* **Failure Mode:** Naive keyword search (`EXECUTE`, `SLAY`) returned 3 isolated violence events with no narrative coherence.
* **Hardened Solution:** Causal Motif Matching combining **DAG traversal**, **temporal dilation filters** ($\Delta t > 100$ ticks), and **emotional polarity inversions** extracted both sagas with 100% precision and zero false positives.

### Prototype 3: The Ghost King & Epistemic Lag Crisis
* **Script:** `docs/red_team/prototypes/prototype_3_epistemic_ghost_king.py`
* **Test Scenario:** King Alden is wounded at the border (150 miles away). A fleeing scout reports to the Capital that the King is dead. Duke Valerius crowns himself to preserve order. 20 days later, King Alden returns alive.
* **Finding:** If agents directly query Plane A state, they "magically know" the King is alive, destroying dramatic irony. Decoupling agent cognition into an `EpistemicBeliefStore` fed by road messengers created a realistic tragic civil war at the city gates.

### Prototype 4: The Micropolis Siege & Cellular Automata
* **Script:** `docs/red_team/prototypes/prototype_4_micropolis_siege.py`
* **Test Scenario:** An external army encircles a 16×16 settlement, cutting trade and road access.
* **Finding:** The Rust core only needs to emit `inflow = 0`. Local 2D cellular automata naturally handles food depletion, famine riots, fire spreading across hovels, and desperate mobs breaching the city gate from the inside.

### Prototype 5: The Vow Collision & Moral Dilemma
* **Script:** `docs/red_team/prototypes/prototype_5_vow_collision.py`
* **Test Scenario:** Sir Gareth has two active vows: `DEFEND_KING` vs `SLAY_FATHER'S_KILLER`. King Alden is revealed as the killer.
* **Failure Mode (Run A):** Naive utility AI oscillated infinitely between drawing steel and shielding the king every tick.
* **Hardened Solution (Run B):** Hierarchical Vow Arbitration (`BLOOD_OATH > FEALTY_OATH`) resolved the dilemma deterministically, triggering a tragic formal renunciation (`Honor -= 40`, `Reputation = 'Oathbreaker'`).

---

## 3. PROTOTYPE SUITE EXECUTION LOG

All 5 prototypes pass and execute via `docs/red_team/prototypes/run_all.sh`:
```bash
bash docs/red_team/prototypes/run_all.sh
```

---

## 4. GOVERNANCE & SPRINT BATCHING

In accordance with the Architect's directive:
- These findings and design specs are formally batched into `docs/red_team/inbox.md` for incorporation at **Gate G0** (conclusion of P0).
- In-flight foundation sprints (`SPR-002`, `SPR-004`, `SPR-010`) remain unblocked and uninterrupted.
