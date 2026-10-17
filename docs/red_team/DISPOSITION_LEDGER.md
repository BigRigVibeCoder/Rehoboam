# RED TEAM FINDINGS & DISPOSITION LEDGER

> **Purpose:** Authoritative living ledger tracking every Red Team finding, architectural proposal, handoff, and decision.  
> **Rule:** Prevents amnesia, prevents re-arguing settled decisions, records exact Architect dispositions and rationales, and defines **Revisit Triggers** for re-testing as the engine evolves.  
> **Maintained by:** Red Team Leader & Quality Auditor  
> **Last Synchronized:** 2026-10-16 (Gate G0 Director Rulings & Batch 04 Intake)

---

## 1. EXECUTIVE DASHBOARD & SUMMARY STATUS

| Category | Count | Status Notes |
| :--- | :---: | :--- |
| **Total Findings Tracked** | **32** | Complete record from project pivot through Gate G0 rulings |
| **Fully Ratified & Adopted into Code / Specs** | **18** | Integrated via ADR-0035, ADR-0036, ADR-0037, PRs #2, #3, #9, #12, #20, `S6`, `S7`, `GOV-003 §6`, RT-031–RT-033 |
| **Adopted with Lighter / Pragmatic Mechanism** | **2** | Gap 1 (Props from stock) and Gap 5 (Parquet reverse key) adapted for kingdom scale |
| **Deferred by Design** | **1** | Acoustic decibel & terrain damping propagation (precision without data) |
| **Approved / Disposed for Gate G0 Integration** | **8** | Batches 01, 02, 03, and 04 in `docs/handoffs/` and `docs/decisions-log.md` |
| **Validated via Working Prototypes (11/11 Passing)** | **11** | Stream Viewer (P15), Compiler (P14), Megacity (P13), Hydrology (P6), Thermostat (P7), Sifter (P8), Comic (P9), Attractor (P10), P1–P5 |
| **Primary Source Grounded & Calibrated (G1a Ready)** | **1** | Batch 05 Demography & Inequality Dossier (`A13b`, `A14`, `A19`, `A20` verified; Notes N13–N17 resolved) |
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
| **RT-008** | **PRNG Stream Independence (`V-DET-20`)** | Review 1 | `rh-ids`, tests | **ADOPTED** | Statistical PRNG covariance tests ensuring split streams don't correlate. Coded and merged in `SPR-002`. | Verified in `VER-SPR-002`. Closed. |
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
| **RT-022** | **Item 9: Red Team Charter & Action Backlog** | Batch 03 | `docs/red_team/` | **RATIFIED & MERGED** | Formalizes 5 Laws, 14 backlog actions, and graveyard autopsies. Merged in PR #12; Laws 2 & 3 codified into `docs/01 §5` with Architect causal-root clarifications. | Active charter. |
| **RT-023** | **Item 10: Four Bleeding-Edge Frontiers** | Batch 03 | `M14`, `S6`, `M7` | **DISPOSED FOR G0** | Versu Social Practices, Caves of Qud Mythic Domains, Wildermyth Comic Grammar, and Nobility Thermostat intake merged in PR #12. | Revisit at Gate G0 planning session. |
| **RT-024** | **Barnes 2014 Priority-Flood Hydrology** | Batch 04 | `M1.topography`, `M1.hydrology` | **VALIDATED (P6)** | $O(N \log N)$ min-heap depression filling guarantees 100% monotonic river routing to sea; eliminates pit traps and river looping. | Integrate into `M1.topography` during P1a. |
| **RT-025** | **Feudal Estate Capacity & Natural Demotion** | Batch 04 | `M7.dynasty`, `M2` | **REBUILT & ADOPTED (P7)** | Rebuilt with zero global controllers. 50 fiefs and age-35 unlanded cadet demotion rule bounds population to ~98 active nobles for 500 yrs. Merged onto `main`. | Revisit at P1a during `M7.dynasty` implementation. |
| **RT-026** | **Two-Stage SQL Causal Sifter & Unexpectedness** | Batch 04 | `rh-sifter`, `05.4` | **REBUILT & ADOPTED (P8)** | Measures both Stage 1 discovery (0.19ms) and Stage 2 DAG traversal (0.30ms) across 100k events with 2k causal trees (0.49ms total). Merged onto `main`. | Revisit at P2 for Parquet DuckDB bindings. |
| **RT-027** | **4-Beat Comic Storyboard Compiler** | Batch 04 | Plane C Graphic Reader | **VALIDATED (P9)** | Compiles Parquet event envelopes into structured 4-beat Wildermyth comic JSON with 100% consistent InstantID visual anchors. Intake merged in PR #20. | Revisit at P5 when Graphic Reader is built. |
| **RT-028** | **The 10 Fallen Bugs Immunity Protocol** | Batch 04 | `GOV-003 §6` | **ADOPTED** | Immunizes engine against DF door storms, item clutter, CDDA nested containers, and RimWorld GC pauses. Ratified by Director into `GOV-003 §6` failure-modes table. | Binding in review. |
| **RT-029** | **Relic Corruption Attractor & Stress Break** | Batch 04 | `S6.artifacts`, `T1.minds` | **VALIDATED & ADOPTED (P10)** | Continuous dynamical attractor $\vec{M}_{t+1} = \vec{M}_t + \kappa(\vec{\Psi} - \vec{M}_t)$ verified in Prototype 10. Architect confirmed: holds up as evidence for Item 8. | Integrate into `S6.artifacts` in P1a. |
| **RT-030** | **Demography & Inequality Science Calibration** | Batch 05 | `M2`, `M4`, `M6`, `V-SF-07`, `V-SF-17` | **CALIBRATED (G1a READY)** | Primary sources deposited in `research/papers/` (`A13b`, `A14`, `A19`, `A20`). Coale-Demeny labels corrected to Levels 3, 5, 7. Runtime Gini clamp retracted under Law 2; M6 Weibull runtime hazard retracted under Law 2; both reclassified as statistical test targets (`V-SF-17`, `V-SF-07`). Formal Calibration Report filed in `RT-2026-10-16-CALIBRATION-REPORT` resolving Notes N13–N17. | Fold into `M2.demography` and `specs/demography/` during Sprint P1.3. |
| **RT-031** | **3D Voxel World Engine Standard for Plane C** | Batch 06 | Plane C (`05.10`, `05.7`) | **ADOPTED AS STANDARD** | Replaces legacy 2D sprite ripper tools with Web-Native Three.js 3D Voxel Engine. Proved locked 60 FPS across rural keeps (12k voxels) and imperial metropolis districts (1M census, 17k hydrated voxels, galleons, cathedral, aqueduct, bazaar). Leverages ADR-0031 Hydration Bubble, Greedy Meshing, and Octree LOD. | Primary Plane C visual spectator. Wire to `rh-stream` in SPR-013. |
| **RT-032** | **Spatial Stream-to-Voxel World Compiler Engine & LLM Culture Dreamer** | Batch 06 | Plane C (`05.10`, `05.7`, ADR-0031, ADR-0036) | **RATIFIED & ADOPTED (P14, ADR-0036)** | Eliminates handcoded voxel coordinates. Reads raw Plane A simulation event streams (Arrow/WebSocket) and uses procedural spatial grammar to synthesize 3D voxel footprints in < 25 ms at locked 60 FPS. Features **👑 Peak SOTA Micro-Voxel Fidelity (0.35m)** with Tudor timber diagonal cross-braces, 3-tier stepped thatch roofs, leaded lantern windows with floral planter boxes, interior hearths, and per-voxel HSL color jittering. Integrates pluggable **Admin & LLM Studio (OpenRouter / Claude-3.5 / Gemini)** and **Deterministic Culture Caching** (zero-lag 0.0 ms replay). Proven in `voxel_stream_compiler.html`. | Core bridge between Plane A simulation stream and Plane C voxel visualizer. |
| **RT-033** | **Plane C Simulation Stream Viewer & Live WebSocket Protocol** | Batch 06 | Plane C (`05.10`, `05.7`, ADR-0037) | **RATIFIED & ADOPTED (P15, ADR-0037)** | Zero-dependency RFC-6455 WebSocket streaming daemon (`bin/stream_server.py`) broadcasting live Plane A simulation packets (Year, Season, Demography, Harvests, Wall Breaches, Dragon Actions) on port 8089. Features live top telemetry ribbon, bidirectional control commands (Pause, 1x/2x/5x/10x speed, Step tick, trigger siege/dragon), First-Person Ground Walking Camera (1.6m eye height, WASD + mouse look, terrain collision), Luanti-grade micro-voxel aesthetics (Weeping Willow drooping vines, multi-toned Sakura cherry blossoms, PBR specular water, drifting fireflies), and zero-dark-failure offline fallback. Verified live in `tools/stream_viewer/index.html` and `bin/run_stream_viewer.sh`. | Production viewer for live Phase 1 simulation streams. |

---

## 3. ACTIVE REVISIT TRIGGER REGISTRY (EVOLUTION HOOKS)

These are scheduled re-entry points where the Red Team must un-pause an item, run verification code, or challenge new pull requests:

```
+-----------------------------------------------------------------------------------------+
| TRIGGER 1: SPRINT MERGE CLEARANCE (SPR-007, SPR-008, SPR-009)                           |
| Target: Active Phase 0 PRs                                                              |
| Actions:                                                                                |
| - Verify SPR-007 PR #25 plane hashes, keyframes, and input journal IPC negative length.  |
| - Stand by for laptop adjutant session on SPR-008 PR #28 (T-02 cross-machine golden run).|
| - Audit SPR-009 wall-clock performance benches (Benches 7 & 8) and Gate G0 inventory.   |
+-----------------------------------------------------------------------------------------+
                                           │
                                           ▼
+-----------------------------------------------------------------------------------------+
| TRIGGER 2: GATE G0 RATIFICATION MILESTONE (CONCLUSION OF PHASE 0)                       |
| Target: CODEX, Roadmap, and System Specifications                                       |
| Actions:                                                                                |
| - Audit VER-G0 formal gate evaluation report.                                           |
| - Confirm Laws 2 & 3 text in docs/01 §5 with Architect causal-root clarifications.     |
| - Confirm GOV-003 §6 ten-bug failure-modes table.                                       |
| - Verify Benches 7 & 8 instruction baselines (Bench 4 deferred to P1a field sprint).    |
+-----------------------------------------------------------------------------------------+
                                           │
                                           ▼
+-----------------------------------------------------------------------------------------+
| TRIGGER 3: PHASE 1A LAUNCH (BLOODLINES & MINDS COMPILATION)                             |
| Target: Live Rust Simulation Runs (Rungs 0-2: Solo, Household, Hamlet)                  |
| Actions:                                                                                |
| - Run live Rust simulation of Court Tantrum Cascade with live minds crate.              |
| - Run RT-ACT-010 (The Nobility Thermostat): verify bounded noble demographic governor.  |
| - Verify O(1) heap memory bound per agent under stress.                                 |
| - Calibrate M2 commoner infant mortality against primary Coale-Demeny tables.           |
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
