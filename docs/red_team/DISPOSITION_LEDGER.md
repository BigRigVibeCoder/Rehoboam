# RED TEAM FINDINGS & DISPOSITION LEDGER

> **Purpose:** Authoritative living ledger tracking every Red Team finding, architectural proposal, handoff, and decision.  
> **Rule:** Prevents amnesia, prevents re-arguing settled decisions, records exact Architect dispositions and rationales, and defines **Revisit Triggers** for re-testing as the engine evolves.  
> **Maintained by:** Red Team Leader & Quality Auditor  
> **Last Synchronized:** 2026-10-17 (Post-G0 Walking Skeleton & SOTA Stream Engine)

---

## 1. EXECUTIVE DASHBOARD & SUMMARY STATUS

| Category | Count | Status Notes |
| :--- | :---: | :--- |
| **Total Findings Tracked** | **38** | Complete record from project pivot through Phase 1a (RT-001–RT-038) |
| **Adopted into Engine Code / Specs / Nightly** | **17** | Core design, determinism, invariants, demography governor, and manor soak check |
| **Adapted with Lighter / Pragmatic Mechanism** | **3** | Props from stock (RT-009), secret horizon (RT-010), Parquet reverse key (RT-013) |
| **Deferred by Design** | **1** | Acoustic decibel & terrain sound damping (RT-010) |
| **Approved / Intake Records for Future Phases** | **8** | Items 1–3, 5–8, 10 mapped to G0, P1a, P1b, P2 |
| **Branch Look-Dev & Verification Prototypes** | **7** | P6 hydrology, P7 thermostat, P8 sifter, P9 comic, P10 attractor, Three.js look-dev, Valley village |
| **Primary Source Grounded & Calibrated** | **1** | Batch 05 Demography & Inequality Dossier (RT-030) |
| **Investigated / Branch Interim Bridge (127.0.0.1)** | **2** | RT-035 interim stream bridge; RT-036 post-SPR-027 control bridge constraints |
| **Audited / As Designed** | **1** | RT-038 subshell cargo PATH check (fails fast as intended) |
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
| **RT-022** | **Item 9: Red Team Charter & Action Backlog** | Batch 03 | `docs/red_team/` | **PROPOSED & RECORDED** | Formalizes 5 Laws, 14 backlog actions, and graveyard autopsies. Merged in PR #12; Laws 2 & 3 codified into `docs/01 §5` with Architect causal-root clarifications. Director ratified Law 1 (D28). | Active charter. |
| **RT-023** | **Item 10: Four Bleeding-Edge Frontiers** | Batch 03 | `M14`, `S6`, `M7` | **DISPOSED FOR G0** | Versu Social Practices, Caves of Qud Mythic Domains, Wildermyth Comic Grammar, and Nobility Thermostat intake merged in PR #12. | Revisit at Gate G0 planning session. |
| **RT-024** | **Barnes 2014 Priority-Flood Hydrology** | Batch 04 | `M1.topography`, `M1.hydrology` | **VALIDATED (P6)** | $O(N \log N)$ min-heap depression filling guarantees 100% monotonic river routing to sea; eliminates pit traps and river looping. | Integrate into `M1.topography` during P1a. |
| **RT-025** | **Feudal Estate Capacity & Natural Demotion** | Batch 04 | `M7.dynasty`, `M2` | **REBUILT & ADOPTED (P7)** | Rebuilt with zero global controllers. 50 fiefs and age-35 unlanded cadet demotion rule bounds population to ~98 active nobles for 500 yrs. Merged onto `main`. | Revisit at P1a during `M7.dynasty` implementation. |
| **RT-026** | **Two-Stage SQL Causal Sifter & Unexpectedness** | Batch 04 | `rh-sifter`, `05.4` | **REBUILT & ADOPTED (P8)** | Measures both Stage 1 discovery (0.19ms) and Stage 2 DAG traversal (0.30ms) across 100k events with 2k causal trees (0.49ms total). Merged onto `main`. | Revisit at P2 for Parquet DuckDB bindings. |
| **RT-027** | **4-Beat Comic Storyboard Compiler** | Batch 04 | Plane C Graphic Reader | **VALIDATED (P9)** | Compiles Parquet event envelopes into structured 4-beat Wildermyth comic JSON with 100% consistent InstantID visual anchors. Intake merged in PR #20. | Revisit at P5 when Graphic Reader is built. |
| **RT-028** | **The 10 Fallen Bugs Immunity Protocol** | Batch 04 | `GOV-003 §6` | **ADOPTED** | Immunizes engine against DF door storms, item clutter, CDDA nested containers, and RimWorld GC pauses. Ratified by Director into `GOV-003 §6` failure-modes table. | Binding in review. |
| **RT-029** | **Relic Corruption Attractor & Stress Break** | Batch 04 | `S6.artifacts`, `T1.minds` | **VALIDATED & ADOPTED (P10)** | Continuous dynamical attractor $\vec{M}_{t+1} = \vec{M}_t + \kappa(\vec{\Psi} - \vec{M}_t)$ verified in Prototype 10. Architect confirmed: holds up as evidence for Item 8. | Integrate into `S6.artifacts` in P1a. |
| **RT-030** | **Demography & Inequality Science Calibration** | Batch 05 | `M2`, `M4`, `M6`, `V-SF-07`, `V-SF-17` | **CALIBRATED (G1a READY)** | Primary sources deposited in `research/papers/` (`A13b`, `A14`, `A19`, `A20`). Coale-Demeny labels corrected to Levels 3, 5, 7. Runtime Gini clamp retracted under Law 2; M6 Weibull runtime hazard retracted under Law 2; both reclassified as statistical test targets (`V-SF-17`, `V-SF-07`). Formal Calibration Report filed in `RT-2026-10-16-CALIBRATION-REPORT` resolving Notes N13–N17. | Fold into `M2.demography` and `specs/demography/` during Sprint P1.3. |
| **RT-031** | **3D Voxel World Look-Dev Prototypes** | Batch 06 | Plane C (`05.10`, `05.7`) | **LOOK-DEV PROTOTYPE (BRANCH-ONLY)** | D51 chose **Bevy** (`rh-view`, ADR-0036, PRJ-003) as the authoritative Plane C standard. Three.js prototypes serve as branch-level visual look-development and exploratory UX. Measured locked 60 FPS across rural keeps (12k voxels) and imperial metropolis districts (1M census, 17k hydrated voxels, galleons, cathedral, aqueduct, bazaar). | Inform Bevy visual shader/style card implementations for V.1 / V.1b. |
| **RT-032** | **Spatial Stream-to-Voxel World Compiler Engine & LLM Culture Dreamer** | Batch 06 | Plane C (`05.10`, `05.7`, ADR-0031, ADR-0036) | **PROPOSAL RECORD; V.1b STYLE CARDS APPROVED (D55)** | Proposal filed as `RT-2026-10-16-proposal-voxel-compiler-and-culture-dreamer.md`. ADR-0036 is the Bevy viewer ADR. Per D55, V.1b style cards were approved; compiler engine remains a proposal record on red team branch. Measured procedural voxel footprint generation in < 25 ms at 60 FPS with micro-voxel details (0.35m). | Feed style cards into Plane C Bevy viewer when V.1b begins. |
| **RT-033** | **Plane C Simulation Stream Viewer & Live WebSocket Protocol** | Batch 06 | Plane C (`05.10`, `05.7`) | **PROPOSAL RECORD (MAPPED TO WIRE_V1 / SPR-027 / P2.6)** | No ADR-0037 exists. Downstream interface is `wire_v1` (SPR-026) and `control.sock` (SPR-027). Network WebSocket endpoints belong to P2.6 (`rh-server`). Look-dev prototype allowed on branch only. Measured real-time camera controls and fallback modes on local WebSocket bridge. | Adapt local stream bridge after SPR-026 and SPR-027 merge. |
| **RT-034** | **Two Castles War & 100-Person Valley Village Simulation** | Batch 06 | Plane C (`tools/stream_viewer/index.html`) | **VALIDATED & OPERATIONAL** | 100-person valley village ("Val d'Or") positioned between North Sapphire Keep (House of Valois) and South Crimson Citadel (House of Lancaster). Features animated watermill with rotating 8-spoke wheel, wheat fields, timber bridge, 100 procedural animated villagers (farmers, millers, blacksmiths, carriers), 60 garrison troops, and real-time cross-valley parabolic trebuchet counter-battery ballistics with independent wall breach destruction. Verified live at 60 FPS on port 8088/8089. | Live benchmark scenario for walking skeleton C. |
| **RT-035** | **The Potemkin Village Gap & Live Stream Ingestion Hook** | Batch 07 | `bin/stream_server.py`, `rh-stream` | **AUDITED & TESTED (INTERIM LOCAL BRIDGE)** | Audited `stream_server.py` on red team branch; exposed synthetic unseeded Python mock. Added `--run-dir` mode tailing `rh-cli tail --canonical --wait` on `127.0.0.1:8089`. Architect ruled: ad-hoc JSON envelope & client calendar rejected. After SPR-026 merges, read-only bridge forwards `wire_v1` frames unchanged. | Rewire stream bridge to subscribe to A.sock/C.sock upon SPR-026 merge. |
| **RT-036** | **Missing IF-0 Control Channel Bridge (control.sock)** | Batch 07 | `specs/stream/control_v1.md`, SPR-027 | **CONSTRAINED BY RULING (POST-SPR-027)** | Analyzed `control.sock` bridge. Ruled: no control bridge on `main` before P2.6. Local branch bridge permitted only after SPR-027 merges, bound to `127.0.0.1` with explicit `Origin` allow-list, per-run token, and forwarding `control.*` methods verbatim. | Implement token + Origin allow-list when SPR-027 merges. |
| **RT-037** | **The Manor World Demographic Scale Gap (60 vs 1,500)** | Batch 07 | `specs/worldgen/fixture_worlds.md`, D57 | **ADAPTED (SPR-019 0.3.1 T-07 AMENDMENT)** | Measured demographic cohort gap between `hamlet` (N=60) and `twin_vales` (N=1,500). Architect amended SPR-019 0.3.1 T-07: `manor` joins `specs/worldgen/project_population.py` (C1, stage B); nightly soak asserts living count at yrs 50, 100, 200 within projected band; writes `artifacts/manor_cohorts.txt`. | Verify in SPR-019 T-07 nightly soak. |
| **RT-038** | **Pytest Subshell Cargo PATH Discovery Trap** | Batch 07 | `bin/lint_forbidden.py`, `RUN-001` | **AUDITED; NO CHANGE (AS DESIGNED)** | Subshell invocations lacking explicit `~/.cargo/bin` in PATH threw `cargo is not on PATH (RUN-001 §2)`. Architect confirmed gates fail fast by design to enforce exact toolchain selection. | Ensure test environments include `~/.cargo/bin` in PATH. |

---

## 3. ACTIVE REVISIT TRIGGER REGISTRY (EVOLUTION HOOKS)

These are scheduled re-entry points where the Red Team must un-pause an item, run verification code, or challenge new pull requests:

```
+-----------------------------------------------------------------------------------------+
| TRIGGER 1: SPRINT MERGE CLEARANCE (SPR-001–SPR-010, SPR-011, SPR-012, SPR-013)          |
| Target: Active Phase 0 & Phase 1a PRs                                                   |
| Actions:                                                                                |
| - Verify SPR-007 plane hashes, keyframes, and input journal IPC negative length.        |
| - Verify SPR-008 T-02 cross-machine golden run and SPR-009 wall-clock performance.      |
| - Audit SPR-011 engine prerequisites and SPR-012 solo living/death walking skeleton B.  |
| - Clear Wave 4 sprint plans: SPR-014 (mortality queues), SPR-020 (envelope), SPR-026.   |
+-----------------------------------------------------------------------------------------+
                                           │
                                           ▼
+-----------------------------------------------------------------------------------------+
| TRIGGER 2: GATE G0 RATIFICATION MILESTONE (PASSED — VER-G0)                             |
| Target: CODEX, Roadmap, and System Specifications                                       |
| Actions:                                                                                |
| - Audit VER-G0 formal gate evaluation report.                                           |
| - Confirm Laws 2 & 3 text in docs/01 §5 with Architect causal-root clarifications.     |
| - Confirm GOV-003 §6 ten-bug failure-modes table.                                       |
| - Verify Benches 7 & 8 instruction baselines.                                           |
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
