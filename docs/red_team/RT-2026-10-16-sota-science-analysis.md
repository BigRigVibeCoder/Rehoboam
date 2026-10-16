# RED TEAM SOTA SCIENCE ANALYSIS: THREE ALGORITHMIC ENGINES FOR SAGA GENERATION

> **Document ID:** RT-2026-10-16-SOTA-SCIENCE  
> **Author:** Red Team Leader & Chief Antagonist  
> **Source Documents Analyzed:**  
> - `upload/Park2023Generative.pdf` (Park et al., Stanford/DeepMind, UIST 2023)  
> - `research/papers/B2_kreminski_dickinson_wardripfruin_2019_felt.pdf` & `B3_winnow.pdf`  
> - `research/reading_list.md` (§10 "Select the unexpected", Kreminski et al., 2022)  
> - `research/R12_literature_verification.md` (§19 Felt/Winnow verification)  
> - `specs/systems/S6.artifacts.odd.toml` (Influence Profile & Bearer Chain)  
> - `specs/systems/S7.vows.odd.toml` (Vows, Grudges & Quests)  
> **Target Systems:** `T1.minds` (P1a), `rh-sifter` (P2), `S6.artifacts` (P1a/P1b)

---

## 1. VECTOR A: THE DETERMINISTIC "REFLECTION TREE" (PARK ET AL. 2023 ADAPTATION)

### 1.1 The Misconception in R04
In `research/R04` (§118) and `reading_list.md` (C9), the plan previously categorized Park et al. (2023) as an "LLM toy" (25 agents over 2 game days cost thousands of dollars; dormant code; proof that LLM-in-the-loop cannot scale).

**The Red Team Critique:** The Architect threw out the baby with the bathwater.  
The core intellectual contribution of Park et al. is **NOT the LLM**. It is the **Memory Architecture: Stream $\rightarrow$ Reflection $\rightarrow$ Plan/Trait Pyramid** (Park et al., §4.1–§4.3, Figure 7).

```
                      +----------------------------------+
                      |        ROOT IDENTITY TRAIT       |
                      |  "Ruthless Kinslayer & Usurper"  |
                      +----------------------------------+
                                       ▲
                                       │ Deterministic Folding
                      +----------------------------------+
                      |       INTERMEDIATE REFLECTION    |
                      |  "Deep Vengeance Grudge vs House"|
                      |   OpinionKnot: Hostility = 0.95  |
                      +----------------------------------+
                                       ▲
                                       │ Importance Sum Trigger >= 150
          +----------------------------+----------------------------+
          │                            │                            │
+--------------------+       +--------------------+       +--------------------+
|  RAW OBSERVATION   |       |  RAW OBSERVATION   |       |  RAW OBSERVATION   |
| Event #102: Insult |       | Event #314: Spurn  |       | Event #812: Slay   |
| Importance = 15    |       | Importance = 35    |       | Importance = 100   |
+--------------------+       +--------------------+       +--------------------+
```

### 1.2 The Deterministic Rust Adaptation for `T1.minds`
In Rehoboam, an LLM must **never** run in Plane A. However, we can implement Park's Reflection Tree using **pure symbolic integer algebra**:

1. **Leaf Events:** Pure Plane A events entering an agent's perception horizon.
2. **Deterministic Poignancy / Importance ($I \in [1, 100]$):**  
   Defined in `registry_v0.toml` by event verb:
   - Routine trade / harvest: $I = 1$
   - Insult / boundary dispute: $I = 15$
   - Public dishonor / broken oath: $I = 50$
   - Kin execution / murder: $I = 100$
3. **The Reflection Threshold Trigger:**  
   $$\sum_{k=1}^N I_k \ge \Theta_{\text{reflection}} \quad (\text{Park default} = 150)$$
   When the cumulative importance of recent un-reflected perceptions crosses $\Theta_{\text{reflection}}$, `T1.minds` executes a **Deterministic Memory Folding**:
   - The cluster of $N$ events is synthesized into a single compact struct:
     ```rust
     pub struct OpinionKnot {
         pub subject_id: PersonId,
         pub attitude: AttitudeKind,     // Contempt, BloodGrudge, Reverence
         pub weight: Q8_8,               // Fixed-point intensity [0.0, 1.0]
         pub primary_cause_event: EventId, // Root anchor
     }
     ```
   - **The Memory Eviction Superpower:** The raw $N$ events are immediately flushed from Plane A heap RAM. They remain on disk in the Parquet archive, but the live agent's RAM footprint stays strictly $O(1)$!
   - When `weight` crosses a threshold, the OpinionKnot deterministically promotes to an active vow in `S7.vows` (`Vow::AVENGE_KIN`).

---

## 2. VECTOR B: WINNOW & "SELECT THE UNEXPECTED" (KREMINSKI & RYAN)

### 2.1 What Rehoboam Already Has vs. What is Missing
* **What is Already Extracted (`research/R12` §19):**  
  Rehoboam evaluated *Felt* (2019) and *Winnow* (2021). The Architect correctly rejected Winnow's full DSL because it flooded partial matches (99.3 matches/1,000 events without time windows). Rehoboam adapted 6 structural features (anchoring `attr@eN`, multi-stage progression, `guest_right_violated` pattern).
* **What is STILL MISSING (Kreminski et al. 2022, B4):**  
  The missing paper from `reading_list.md` line 38 is **"Select the Unexpected"**.  
  Currently, `rh-sifter` uses static heuristics. But true drama requires mathematical **Unexpectedness ($\mathcal{U}$)**.

### 2.2 Mathematical Formulation of Narrative Unexpectedness
Let an event $e$ occur in global world state $S_t$.  
In procedural simulation, the transition probability $P(e \mid S_t)$ is computable from the utility action weights:

$$\mathcal{U}(e) = -\log_2 P(e \mid S_t)$$

1. **Mundane Event:** A peasant purchases bread during peace:  
   $P(e \mid S_t) \approx 0.95 \implies \mathcal{U}(e) = -\log_2(0.95) \approx 0.07 \text{ bits}$ (Sifter ignores).
2. **Expected Tragedy:** An executioner executes a condemned criminal:  
   $P(e \mid S_t) \approx 0.80 \implies \mathcal{U}(e) \approx 0.32 \text{ bits}$ (Low priority).
3. **True Mythic Drama:** A sworn knight with 95 Honor and 90 Fealty executes his own King:  
   $P(e \mid S_t) \approx 0.001 \implies \mathcal{U}(e) = -\log_2(0.001) \approx 9.96 \text{ bits}$ (Massive spike).

**Actionable Sifter Rule:** A causal chain $\mathcal{C} = [e_1, e_2, \dots, e_k]$ is elevated to a **Headline Saga** if:
$$\text{SagaScore}(\mathcal{C}) = \sum_{i=1}^k \mathcal{U}(e_i) \cdot \Delta t(e_i, e_{i-1})^{\gamma} \ge \text{Threshold}$$
where $\gamma > 0$ rewards long-latency vendettas and patience.

---

## 3. VECTOR C: THE RELIC CORRUPTION TENSOR ($\vec{\Psi}_{\text{relic}}$)

### 3.1 The Gap in S6.artifacts
In [`specs/systems/S6.artifacts.odd.toml`](file:///home/bdavidriggins/Documents/Rehoboam/specs/systems/S6.artifacts.odd.toml), line 48 declares:
`column = "artifact.influence_profile"`  
`meaning = "Fixed-point influence profile: how the artifact bends its bearer's mind"`

Currently, the spec lacks an analytical equation for how this influence acts. Without a formula, implementers will resort to binary hacks (`if holding_relic { go_crazy() }`).

### 3.2 The Dynamical Attractor Model
We model the bearer's mind as a state vector $\vec{M} \in [0, 1]^4$ in `T1.minds`:
$$\vec{M} = \begin{bmatrix} \text{Honor} \\ \text{Ambition} \\ \text{Paranoia} \\ \text{Sanity} \end{bmatrix}$$

A relic in `S6.artifacts` carries a static target **Corruption Attractor** $\vec{\Psi} \in [0, 1]^4$ and an **Attractor Strength** $\kappa \in \mathbb{Q}_{8.8}$:
$$\vec{\Psi}_{\text{relic}} = \begin{bmatrix} 0.05 \text{ (Extinguish Honor)} \\ 0.95 \text{ (Maximise Ambition)} \\ 0.90 \text{ (Paranoid Dread)} \\ 0.10 \text{ (Erode Sanity)} \end{bmatrix}, \quad \kappa = 0.02 \text{ / month}$$

Each monthly tick, while the artifact is borne, $\vec{M}$ experiences continuous exponential decay toward $\vec{\Psi}$:
$$\vec{M}(t+1) = \vec{M}(t) + \kappa \cdot (\vec{\Psi}_{\text{relic}} - \vec{M}(t))$$

### 3.3 Coupling to Stress & Mental Breaks
As $\vec{M}(t)$ is pulled toward $\vec{\Psi}$, it begins to conflict with the bearer's active sworn vows:
$$\text{DissonanceStress} = \|\vec{M}(t) - \vec{V}_{\text{vow}}\| \cdot Q_{\text{weight}}$$
1. **Months 1–12:** Bearer notices minor paranoia. Stress is manageable.
2. **Year 3:** Ambition exceeds Honor. Conflict with `Vow(Fealty)` generates $+8$ stress per tick.
3. **Year 5:** Bearer hits 100/100 stress $\implies$ **Deterministic Mental Break**: The bearer slays their liege or flees with the relic into the wilderness.

**Result:** Pure Tolkien-grade Ring corruption emergent from a two-variable fixed-point differential equation.

---

## 4. RECOMMENDATIONS FOR THE SPRINT & SPEC PIPELINE

1. **For `specs/systems/S6.artifacts.odd.toml`:**  
   Update the definition of `influence_profile` to explicitly name the Corruption Attractor Vector $\vec{\Psi}$ and monthly coupling constant $\kappa$.
2. **For `T1.minds` (Pre-P1a Spec):**  
   Incorporate the Symbolic Reflection Tree with importance threshold $\Theta = 150$ and deterministic memory eviction to preserve $O(1)$ agent heap memory.
3. **For `rh-sifter` (P2):**  
   Implement the Bayesian Unexpectedness Metric $\mathcal{U}(e) = -\log_2 P(e \mid S_t)$ to rank sagas without heuristics.
