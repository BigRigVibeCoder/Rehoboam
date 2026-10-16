# RED TEAM FINDINGS & DISPOSITION LEDGER

> **Purpose:** Authoritative living ledger tracking every Red Team finding, architectural proposal, handoff, and decision.  
> **Rule:** Prevents amnesia, prevents re-arguing settled decisions, records exact Architect dispositions and rationales, and defines **Revisit Triggers** for re-testing as the engine evolves.  
> **Maintained by:** Red Team Leader & Quality Auditor

---

## 1. EXECUTIVE DASHBOARD & SUMMARY STATUS

| Category | Count | Status Notes |
| :--- | :---: | :--- |
| **Total Findings Tracked** | **28** | Complete record from project pivot to current sprint |
| **Fully Ratified & Adopted into Code / Specs** | **13** | Integrated via ADR-0035, PR #2, PR #3, PR #9, `S6`, `S7`, `V-SAGA` |
| **Adopted with Lighter / Pragmatic Mechanism** | **2** | Gap 1 (Props) and Gap 5 (Reverse Key) adapted for kingdom scale |
| **Deferred by Design** | **1** | Acoustic decibel & terrain damping propagation (precision without data) |
| **Approved in Principle for Gate G0 Integration** | **7** | Batches 01, 02, and 03 in `docs/red_team/inbox.md` |
| **Validated via Working Prototypes (Batch 04)** | **5** | Hydrology, Thermostat, SQL Sifter, Storyboard, Bug Immunity |
| **Active Revisit Triggers (Hooks)** | **6** | Scheduled prototype re-tests tied to engine phase gates |

---

## 2. MASTER TRACEABILITY MATRIX

| ID | Finding / Proposal | Origin Batch | Target Spec | Disposition Status | Architect Ruling & Technical Rationale | Revisit Trigger & Evolution Hook |
| :--- | :--- | :---: | :--- | :---: | :--- | :--- |
| **RT-001** | **"Hero vs. Wheat" Saga Pivot** | Batch 00 | ADR-0035, `11` | **ADOPTED** | The director is a reader/listener, not a gamer. 40% dynasties/vows/relics, 30% sifting/chronicle, 30% environmental pressure. | Revisit at Gate G2 (Thesis Gate). |
| **RT-002** | **"The Engine Only Engines" Boundary** | Batch 00 | ADR-0030, `05.10` | **ADOPTED** | Hard rule: Rust core contains zero presentation, media, or LLM code. Emits only Parquet world stream; inputs only IF-0. | Permanent invariant. Checked by `V-INSP-05`. |
| **RT-003** | **Kill Custom 3D & Podcast Infrastructure** | Batch 00 | ADR-0018, ADR-0019 | **ADOPTED** | Cut all custom Godot 3D and mobile RSS feeds. Output is offline MP3 chapters and Web Graphic Novel Reader. | Closed. Never revisit. |
| **RT-004** | **Cinematic Event Envelope Split** | Batch 00 | ADR-0035, `05.11` | **ADOPTED** | Engine stores simulated facts (cast, coordinates, weather, emotion, artifact, causal edges). Downstream tools infer adjectives. | Revisit at P2 when `rh-sifter` compiles first saga. |
| **RT-005** | **CK3-Style Stress & Mental Breaks** | Batch 00 | ADR-0035, `T1.minds` | **ADOPTED** | Fixed-point `Q8_8` (0–100) stress meter. Crossing threshold triggers deterministic state transition break from Registry table. | Revisit at P1a when `rh-minds` crate is initialized. |
| **RT-006** | **Subsystem Isolation Tests (`V-SF-22`)** | Review 1 | `docs/09`, tests | **ADOPTED** | Multi-system integration tests must allow isolating individual systems to test causal feedback. Merged in PR #3. | Checked by CI in `bin/check_all.sh`. |
| **RT-007** | **Coupling Sweeps $\alpha \in [0, 1]$ (`V-SF-23`)** | Review 1 | `docs/09`, tests | **ADOPTED** | Parametric sensitivity sweeps to test system coupling stability. Merged in PR #3. | Run during phase certification gates. |
| **RT-008** | **PRNG Stream Independence (`V-DET-20`)** | Review 1 | `rh-ids`, tests | **ADOPTED** | Statistical PRNG covariance tests ensuring split streams don't correlate. Coded in `SPR-002`. | **Active Revisit:** Audit `SPR-002` once merged (`RT-ACT-007`). |
| **RT-009** | **Gap 1: Spontaneous Props Ban** | Batch 01 | SAGA-12, `S6` | **LIGHTENED** | *Architect ruling:* Unique artifacts require unbroken bearer chains. Ordinary props (poison phial, dagger) drawn from simulated stock at point of purchase. | Revisit at P1a during `S6.artifacts` implementation. |
| **RT-010** | **Gap 2: Information Horizon (Secrets v0)** | Batch 01 | SAGA-4, `05.3` | **LIGHTENED / DEFERRED** | *Architect ruling:* Secrets travel by road network and scheduled discovery. Acoustic decibel sound damping deferred as "precision without data". | Revisit at P1b when road travel speed is benchmarked. |
| **RT-011** | **Gap 3: Instant Dynastic Succession** | Batch 01 | SAGA-13, `M7` | **ADOPTED** | When a ruler dies, M7 settles succession in the same instant (heir, claims, proximity, treasury) or declares crisis. No annual lag. | Revisit at P1a during `M7.dynasty` sprint. |
| **RT-012** | **Gap 4: Step-by-Step Fugitive Flight** | Batch 01 | SAGA-14, `S3` | **ADOPTED** | Fugitives move step-by-step along the road graph toward a border, temple, or ally, with garrison interception hazards. No teleportation. | Revisit at P1b during `S3.migration` sprint. |
| **RT-013** | **Gap 5: Legends Reverse Key on Mutated State** | Batch 01 | SAGA-15, `05.2` | **LIGHTENED** | *Architect ruling:* Mutated entities store cause `EventId` in state (`Ruin { cause: EventId }`). World map queries answered by Parquet index, not on-tile columns. | Revisit at P2 during DuckDB index benchmark. |
| **RT-014** | **Item 1: Court Circuit Breakers (Anti-Tantrum)** | Batch 01 | `M14`, `T1.minds` | **APPROVED FOR G0** | *Architect read:* Good fit. Stress needs damping or one death cascades through court. Fits M7 and minds. | Revisit at Gate G0 for formal spec folding. |
| **RT-015** | **Item 2: Sifter Causal Motif Grammar** | Batch 01 | `rh-sifter` | **APPROVED FOR G0** | *Architect read:* Mostly already expressible in story-pattern language; emotion snapshots make reversals checkable. | Revisit at P2 (`RT-ACT-002` prototype benchmark). |
| **RT-016** | **Item 3: Beliefs Separate from Ground Truth** | Batch 01 | `S2.agents`, `05.3` | **APPROVED FOR G0** | *Architect read:* Already the design: characters decide from beliefs, news travels by road. "Ghost King" justifies pulling earlier. | Revisit at Gate G0 / P1b planning. |
| **RT-017** | **Item 4: Micropolis Unidirectional Impulse Bridge** | Batch 01 | 2D Client, `05.7` | **RESOLVED & ALIGNED** | *Architect critique:* If grid simulates riots, it's simulation, not display. *Red Team resolution:* 100% unidirectional. Rust decides breaches; grid only animates. | Revisit at P5 when 2D diorama is built. |
| **RT-018** | **Item 5: Strict Vow Priority Tiers** | Batch 01 | `S7.vows` | **APPROVED FOR G0** | *Architect read:* Good. Tiers should come from culture data in `registry_v0.toml`, not hardcoded in Rust enums. Red Team agreed. | Revisit at Gate G0 / P1a `S7.vows` spec freeze. |
| **RT-019** | **Item 6: Symbolic Reflection Tree (Park et al.)** | Batch 02 | `T1.minds` | **APPROVED FOR G0** | *Architect read:* Good fit for R-11 (bounded memory). Refined with relational scaling, evidence chain, bitwise order, and fixed-point math. | Revisit at Gate G0 for formal `T1.minds` incorporation. |
| **RT-020** | **Item 7: Bayesian Unexpectedness Metric ($\mathcal{U}$)** | Batch 02 | `rh-sifter` | **APPROVED FOR G0** | *Architect read:* Useful formula to make rarity score in `05.4` §3 exact ($\mathcal{U} = -\log_2 P$). | Revisit at P2 when ranking algorithm is coded. |
| **RT-021** | **Item 8: Relic Corruption Attractor Tensor** | Batch 02 | `S6.artifacts` | **APPROVED FOR G0** | *Architect read:* Good. Simple, bounded, fixed-point friendly, slots straight into S6 coupling fields. | Revisit at Gate G0 / P1a for `S6.artifacts` coupling. |
| **RT-022** | **Item 9: Red Team Charter & Action Backlog** | Batch 03 | `docs/red_team/` | **PENDING G0** | Formalizes 5 Laws, 14 backlog actions, and graveyard autopsies. Filed in PR `arch/red-team-batch-03`. | Revisit when PR is merged by Architect. |
| **RT-023** | **Item 10: Four Bleeding-Edge Frontiers** | Batch 03 | `M14`, `S6`, `M7` | **PENDING G0** | Versu Social Practices, Caves of Qud Mythic Domains, Wildermyth Comic Grammar, and Nobility Thermostat. | Revisit at Gate G0 planning session. |
| **RT-024** | **Barnes 2014 Priority-Flood Hydrology** | Batch 04 | `M1.topography`, `M1.hydrology` | **VALIDATED (P6)** | $O(N \log N)$ min-heap depression filling guarantees 100% monotonic river routing to sea; eliminates pit traps and river looping. | Integrate into `M1.topography` during P1a. |
| **RT-025** | **The Feudal Nobility Thermostat** | Batch 04 | `M7.dynasty`, `M2` | **VALIDATED (P7)** | Cadet pruning and downward mobility clamps active named historical figures to $[150, 350]$ across 500 years; prevents DF/CK2 FPS collapse. | Revisit at P1a during `M7.dynasty` implementation. |
| **RT-026** | **Recursive SQL CTE Causal DAG Sifter** | Batch 04 | `rh-sifter`, `05.4` | **VALIDATED (P8)** | Traverses 30-year regicide DAG across 100,000 events in 0.197ms; computes 52.5 bits Kreminski surprise, filtering 99.995% noise. | Revisit at P2 for Parquet DuckDB bindings. |
| **RT-027** | **4-Beat Comic Storyboard Compiler** | Batch 04 | Plane C Graphic Reader | **VALIDATED (P9)** | Compiles Parquet event envelopes into structured 4-beat Wildermyth comic JSON with 100% consistent InstantID visual anchors. | Revisit at P5 when Graphic Reader is built. |
| **RT-028** | **The 10 Fallen Bugs Immunity Protocol** | Batch 04 | Engine Invariants, `05.10` | **PROPOSED G0** | Immunizes engine against DF door storms, item clutter, CDDA nested containers, and RimWorld GC pauses. | Codify in `GOV-008` & `CLAUDE.md` engineering invariants. |

---

## 3. ACTIVE REVISIT TRIGGER REGISTRY (EVOLUTION HOOKS)

These are scheduled re-entry points where the Red Team must un-pause an item, run verification code, or challenge new pull requests:

```
+-----------------------------------------------------------------------------------------+
| TRIGGER 1: SPRINT MERGE CLEARANCE (SPR-002, SPR-004, SPR-010)                           |
| Target: In-Flight Rust Crates                                                           |
| Actions:                                                                                |
| - Audit SPR-002 PRNG covariance and genealogical PersonId re-hashing (RT-ACT-007).      |
| - Audit SPR-010 fixed-point arithmetic for overflow traps on stress surges (RT-ACT-008).|
| - Audit SPR-004 generic Registry loader against S6.artifacts & S7.vows (RT-ACT-009).    |
+-----------------------------------------------------------------------------------------+
                                           │
                                           ▼
+-----------------------------------------------------------------------------------------+
| TRIGGER 2: GATE G0 PLANNING MILESTONE (CONCLUSION OF PHASE 0)                           |
| Target: CODEX, Roadmap, and System Specifications                                       |
| Actions:                                                                                |
| - Audit G0 dispositions record for Batches 01, 02, and 03.                              |
| - Ensure S6.artifacts includes Corruption Attractor equations.                           |
| - Ensure S7.vows includes Cultural Priority Tiers from registry_v0.toml.                |
| - Ensure T1.minds spec includes the Symbolic Reflection Tree and evidence chains.       |
+-----------------------------------------------------------------------------------------+
                                           │
                                           ▼
+-----------------------------------------------------------------------------------------+
| TRIGGER 3: PHASE 1A LAUNCH (BLOODLINES & MINDS COMPILATION)                             |
| Target: Live Rust Simulation Runs                                                       |
| Actions:                                                                                |
| - Run live Rust simulation of Court Tantrum Cascade with live minds crate.              |
| - Run RT-ACT-010 (The Nobility Thermostat): measure named noble population over 500 yrs.|
| - Verify O(1) heap memory bound per agent under stress.                                 |
+-----------------------------------------------------------------------------------------+
                                           │
                                           ▼
+-----------------------------------------------------------------------------------------+
| TRIGGER 4: PHASE 2 LAUNCH (SAGAS & SIFTER IMPLEMENTATION)                               |
| Target: rh-sifter & DuckDB Parquet Storage                                              |
| Actions:                                                                                |
| - Execute RT-ACT-012: Benchmark DuckDB recursive CTEs over 1,000,000 live Parquet rows. |
| - Verify that Bayesian Unexpectedness U(e) filters out 99% of peasant trade events.     |
| - Execute RT-ACT-011: Compile live 4-beat Wildermyth storyboard JSON payloads.          |
+-----------------------------------------------------------------------------------------+
```

---

## 4. PERMANENT ANTI-REPETITION RULES

To prevent circular debates and respect the Architect's time and sprint velocity, the Red Team adheres strictly to these three rules:

1. **The Settled Compromise Rule:** Once an item is marked **ADOPTED** or **LIGHTENED** (such as Gap 1 ordinary props being drawn from stock, or Gap 5 Parquet indexing rather than on-tile columns), **we do not re-open the design**. We accept the Architect's scale compromise unless an active revisit benchmark proves a fatal failure.
2. **The Branch & PR Protocol:** Direct pushes to `main` are banned. Every new probe or finding is filed as a numbered batch on an `arch/red-team-batch-XX` branch with a pull request, allowing the Architect to sequence merges between sprint clearances.
3. **The Evidence-First Mandate:** No defect or architectural hole is presented to the Architect without an accompanying executable Python prototype, mathematical formulation, or ast-linter test fixture.
