# 03 — Requirements Specification

*Status: Draft v1.1 (2026-10-15). Style: ISO/IEC/IEEE 29148-lite.*
*v1.1 changes: aligned with the v1.1 architecture (`05.x`) and ADR-0024–0026 (three planes, scheduled outcomes and material inputs, genealogical IDs). New DET-12–15, SIM-12, DATA-8–9, SIFT-8, ORA-7–9, R3D-7–9 and AUD-4. EVO-1 and EVO-3 rewritten. DET-6 binds at P2. Stale wording removed (leases, decrees, pins as retention, pre-batched consults, single burn-in). Phases aligned with the P1a/P1b and P3a/P3b split of `11`; PERF-1 (1M) and PERF-2 follow the re-baselined `08`. §3 now points to the requirement → test matrix in `09` §6.*

**Reading the tables**
- **Verification method:** **T** = Test (automated), **A** = Analysis, **D** = Demonstration, **I** = Inspection.
- **Phase:** the roadmap phase where the requirement first becomes binding (`11_roadmap.md`, which splits P1 into P1a/P1b and P3 into P3a/P3b). A split such as P1b/P3b means the parts named in the text bind at each phase.
- **Trace:** links to stakeholder needs (SN-n), vision elements (V1–V7), success claims S1–S4, design drivers D1–D7 and MOE/MOP (`01_vision_and_intent.md`); design-review findings (DR-, `02`); red-team findings (RT-); ADRs; and research (R0x).

---

## 1. Stakeholder needs

The single stakeholder is you, with AI coding agents as the implementation workforce.

| ID | Need | Trace |
|---|---|---|
| SN-1 | I want to generate worlds and watch centuries of history unfold, stopping at the moments that matter. | S1 |
| SN-2 | I want to understand *why* things happened, by following causes back through time. | S1, D1 |
| SN-3 | I want history to read as story: arcs, betrayals, reversals, irony. | S1, D2 |
| SN-4 | I want my worlds to persist, to replay exactly, and to branch into what-ifs. | S3, D3 |
| SN-5 | I want any person or place I look at to be coherent, persistent and grounded in the simulation. | S2, D4 |
| SN-6 | I want to influence history as a mortal Oracle, without commanding it. | V7 |
| SN-7 | I want to turn histories into chronicles, prose, scripts and audio. | S1 |
| SN-8 | I want to inhabit the world visually: an ambient diorama, then walking its streets and talking to its people. | S2 |
| SN-9 | I want to describe new worlds in natural language and get a valid, playable world. | V5 |
| SN-10 | I want to raise fidelity and complexity continuously without rewrites, using AI agents. | S4, D5, D6 |
| SN-11 | I want to scale compute without limit when I choose to: more fidelity, more forecasts, bigger models. | S4, D7 |

## 2. System requirements

### 2.1 Simulation core (SIM)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| SIM-1 | The engine shall represent every living person in a world as an individual **ledger row** with a stable, genealogical `PersonId` (DATA-8). The same engine and binary shall run worlds **from 1 to 1,000,000 persons** (ADR-0028); ID widths and table layouts shall support ≥ 2M persons per timeline; performance budgets apply at 1M (PERF-2). | T | P1a | SN-5, DR-C6, ADR-0026, ADR-0028 |
| SIM-2 | The engine shall represent society on a **region graph** (provinces with typed route edges) and physical fields on a **raster**, with resolutions set by the manifest. | T | P1a | DR-C3, DR-M14 |
| SIM-3 | The engine shall implement the **must-have systems library** at the target fidelity levels of `05.1` §3: `M1.regions` and `M2.demography` at P1a; `M3.agriculture` (with climate), `M4.economy`, `M6.polity`, `M7.dynasty` (succession and dynasts) and `M9.validation` at P1b; `M5.contagion` (including the covert compartment) and `M8.war` (campaign war) at P3a. | T | P1a/P1b/P3a | DR-C3 |
| SIM-4 | Every system shall implement the common `SystemModel` trait, declaring: a descriptor (system ID, fidelity level, model semver); an access spec (read set, **owned** write set, emitted lifecycle commands and event types, declared lags); pulse bindings; a parameter schema with bounds; invariants; and declared aggregates (`05.1` §2). | I/T | P0 | SN-10, D5 |
| SIM-5 | Vital events shall use **age-specific, density-dependent hazards** with **binomial** draws. Unbounded Poisson death draws are forbidden. The time of each sampled outcome within the pulse is scheduled as a timer (DET-13). | T | P1a | DR-C3, ADR-0025 |
| SIM-6 | The engine shall run multiple clocks (year, season, month, day, hour) as pulses on one integer sim-time axis, `SimTime = i64` minutes from the world epoch (negative values allowed). Daily stepping shall be **adaptive**: only for active epidemics, campaigns and sieges. | T | P1a | DR-M1, DR-M20, ADR-0026 |
| SIM-7 | Named actors (T1) shall be scheduled by a **discrete-event agenda** of decisions and timers keyed `(time, phase, entity_id, seq)`, and shall not be polled at a fixed frequency during fast-forward. Decisions due in one instant shall be scored against the start-of-`Agenda` snapshot and applied in keyed permutation order, with at most one re-score per actor (`05.1` §4.3). | T | P1b | DR-M1, DR-M19, RT-M1 |
| SIM-8 | Conservation invariants (people, food, coin, pathogen counts) shall hold at every pulse: between the ledger and the life-course log, and across off-map boundary adapters. Violations shall fail the run in debug and CI builds. | T | P1a | DR-C6, RT-M6 |
| SIM-9 | Ruling houses and the active members of great houses shall be tracked as named actors (T1) from world start. Great-house members more than N degrees from any title, claim or office become **dormant** ordinary persons and are re-promoted only by simulation criteria. Genealogy is complete for everyone through the life-course log (DATA-8). | T | P1b | DR-M20 |
| SIM-10 | Scenario horizon and resolution shall be declared per world. A template sets the starting state, not an era lock. A run shall be flagged *speculative* only when it goes beyond the technological frontier: past the last supported era pack, or a modern-start world forecast beyond 50 years (ADR-0027). | I | P0 | DR-M15, ADR-0027 |
| SIM-11 | The core shall build as a library with no I/O and with four hosts: headless CLI (P0), Python wheel (P1b), network server (P2), and an optional in-process GDExtension host (P5+, not on the critical path; the default 3D client is a separate process, ADR-0018). | I/D | P0–P5 | D6 |
| SIM-12 | The engine shall implement the **should-have systems** at F1 (`05.1` §3): `S1.culture` (religion, ideology and culture: Abrams–Strogatz shares, bounded-confidence opinions, Axelrod traits); `S2.technology` (tech DAG, Bass diffusion, invention rising with connected population, loss below a threshold); `S3.migration`, inside `M2.demography` (radiation flows driven by utility differentials); and `S4.ecology` (Rosenzweig–MacArthur prey and livestock, logistic resources, and apex predators such as dragons as **individuals** with life-history hazards, bounded by energetics). | T | P3a | DR-C3, R09 |
| SIM-13 | **Era-generic mechanisms.** No system shall hard-code an era. Every era-sensitive parameter shall be a Registry function of the era-state vector (technology adoption, energy per capita, urban share, literacy, fiscal capacity, executive constraint, franchise, succession institutionalisation, medical level). Mechanics shall never read the era label, which exists for presentation, narrative and validation only. Modern-capable mechanisms shall be present from the start and dormant until the state activates them. | T/I | P1b | ADR-0027, R12 |
| SIM-15 | **No minimum-N assumptions.** Schema minimums shall be 1 person and 1 region. Every statistic shall declare its minimum sample and report INSUFFICIENT below it. World extinction shall be a valid journaled end state. Scale profiles (micro ≤ 1k, meso, macro) shall only set configuration defaults (cadence, named-mind policy, validation suite, pattern packs), which mechanics read as explicit values. | T | P1a | ADR-0028 |
| SIM-16 | **Agentive catalysts.** A manifest may declare in-world agents with Registry verb sets acting within a catalyst's permission scope (e.g. a wager with stages and `spare_life`). Their verbs shall be journaled material inputs; consequences and every person's response shall be simulated, never forced. | T | P3b | ADR-0028 |
| SIM-14 | **Endogenous institutions and era packs.** Governments, succession laws and institutions shall change in-run along a Registry transition graph (adoption by emulation, reform under fiscal stress, regime change after breakdown), each change being an event with causal edges. Coverage of later eras shall grow by era packs; a world that reaches the engine frontier shall keep running, file a gap report and continue under a newer build through a migration keyframe. | T | P3a / packs | ADR-0027, R12 |

### 2.2 Determinism, replay and timelines (DET)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| DET-1 | History shall be a pure function of `(engine_build, registry_version, manifest_hash, seed, input_journal)`. | T | P0 | SN-4, D3 |
| DET-2 | All stochastic draws shall use **counter-based keyed RNG**, `u = H(world_seed, branch_salt(t), system_id, purpose, entity_id, t, k)`, where `k` indexes draws within one (purpose, entity, instant) and `branch_salt(t)` hashes the fork lineage covering `t`. The lineage is empty on the canonical timeline. Every Foresight rollout extends it uniquely. What-if forks keep the parent salt unless "reroll fate" is chosen. Purposes flagged `resample_on_fork` in the Registry are re-drawn after a fork. No global or sequential RNG streams. | T/I | P0 | DR-C5, RT-M4 |
| DET-3 | Authoritative threshold quantities (counts, money, loyalty, timers) shall be integer or fixed-point. Floats are permitted in fields, computed in fixed order with `libm`, and quantised at macro boundaries. | I/T | P0 | DR-C5 |
| DET-4 | The state hash (chunked Merkle xxh3-64) shall be identical across 1, 2, 4 and N threads and across repeated runs. It is recorded every sim-month in CI and debug builds and every sim-year (configurable) in release builds. Per-table Merkle roots shall support a **divergence-bisect tool** that reports the first divergent month, table and row. | T | P0 | MOP-2 |
| DET-5 | **Golden-run CI** shall replay reference worlds and compare state hashes each sim-year: on every PR (`smoke` for 1,000 years, the other reference worlds for 200 years) and at full length nightly. Golden baselines cover plane A only. | T | P0 | MOE-4, ADR-0024 |
| DET-6 | **Observer invariance:** viewing, pinning, inspecting, scrubbing, drop-in, rendering, hydration, console use and LLM facade activity shall not change `state_hash` unless a material input is journaled. Binding from P2 through console fuzzing; extended at P5 (passive crowds) and P6 (live hydration). | T | P2 | D4, ADR-0024, RT-C1 |
| DET-7 | Every nondeterministic input shall be appended to the **input journal** with tick, sequence, class (push or pull), dependency scope and producer (IF-2): manifest, settings, console mode, player actions and speech acts, Oracle verbs, accepted LLM outputs, and deliberation answers including journaled fallbacks. | T | P0 | DET-1 |
| DET-8 | The engine shall write **permanent keyframes** every 10 sim-years (configurable) and maintain a lazy LRU cache of yearly keyframes near the cursor. Each keyframe records the engine build that wrote it. | T | P2 | MOP-4 |
| DET-9 | The engine shall support **forking**: a new `timeline_id` from `(parent, tick)` that extends the salt lineage, with chronicle segments before the fork shared by reference. | T | P2 | SN-4 |
| DET-10 | Saves shall be versioned under **"frozen past, migrated future"**. Each world shall record the content-addressed engine build of every segment of its history, and those builds shall be archived. A new engine build shall **never re-simulate archived history**: the past is read from the chronicle or replayed with its archived build, and the head is migrated forward through an explicit, logged **migration keyframe**. Keyframes shall not be thinned below what an archived build can regenerate. | T | P2 | MOE-4, `05.2` §7 |
| DET-11 | Read-only consumers (renderer, console, LLM gateway, Python bindings) shall access state only through immutable snapshot frames, under an OpenRA-style "unsynced" guard whose runtime hash check runs in debug and CI builds. | T | P1b | DET-6 |
| DET-12 | **Three planes.** All state shall belong to exactly one plane: **A** authoritative (ledger, life-course log, regions, polities, markets, named actors, beliefs, agenda and timers, chronicle events; `state_hash`); **B** observation (pins, facts ledger, hydrated presentation, view caches; `observation_hash`); or **C** narrative (sift matches, significance, retention and visibility tiers, ratings, chronicles and prose; `narrative_hash`). Plane-A code shall never read B or C, enforced by a crate-dependency lint. Only journaled material inputs enter A. Nothing in B or C may change which events A emits, the EventIds it assigns, the rows it retains, the actors it schedules or promotes to T1, or any A value. In particular, pinning never promotes a person to T1. | T/I | P0/P2 | ADR-0024, RT-C1, D4 |
| DET-13 | **No back-dating.** No event shall be emitted with a tick earlier than the current instant. `EventId` order shall equal emission order `(tick, phase, seq)`. An outcome sampled for a later instant (death, birth, illness onset, migration, marriage) shall be a plane-A **timer** that fires at its own tick in every mode, watched or not. | T | P1a | ADR-0025, RT-C2 |
| DET-14 | **Acting in the past forks.** A material input at a tick before the timeline head shall create a fork at that tick and apply there; the canonical timeline is never rewritten. Passive drop-in (seek, inspect, watch) shall be read-only. Each material input shall declare a dependency scope (person, household, settlement or region); pending timers in that scope are cancelled and re-sampled in the `Inputs` phase, and at equal ticks inputs apply before timers. | T | P2 | ADR-0025, IF-1 |
| DET-15 | **Pull inputs.** A deliberation-consult answer shall be a pull input keyed `(actor, decision_type, t, menu_hash)` and applied at `t + D`, where D is a fixed sim-time delay per decision type. If no answer has arrived by `t + D`, the simulation stalls; a configurable timeout applies the deterministic fallback, and the fallback is journaled. Replay shall **hard-fail** on a missing, extra or mismatched pull input. | T | P3b | `05.3` §8, RT-M2 |

### 2.3 Chronicle and data (DATA)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| DATA-1 | Events shall be stored in a typed, append-only store whose tables are assigned to planes (`05.2` §4, `specs/chronicle/schema.sql`). Plane A: `event`, `event_role`, `event_snapshot`, `causal_edge`, `knowledge`, `diffusion_front`, `collection`, `aggregate_event`, `state_interval`, `life_course`, `regional_aggregate`. Plane B: `facts_ledger`. Plane C: `sig_component`, `sift_match`, `visibility`. Tables land with the work packages that produce them. | I/T | P1a–P3a | DR-C4, ADR-0024 |
| DATA-2 | Causal edges shall be typed: `contingent`, `triggered`, `relayed_knowledge`, `motivated_by`, `enabled`, `probabilistic`, `player`, `director`. | T | P1b | DR-C4 |
| DATA-3 | Events shall capture the predicate values that rules read **at emission time** (`event_snapshot`). Slowly changing facts shall be stored as `valid_from`/`valid_to` intervals. | T | P1b | DR-M4 |
| DATA-4 | No event shall be deleted by a rule fixed at creation. Named-actor events shall **never be physically compacted**. Low-salience events are hidden by a per-timeline **visibility sidecar** (plane C), not moved or rewritten. Rarity base rates are kept as counts per (pattern, era). Hard deletion is an explicit, opt-in disk-pressure action on cold archives only, and never touches data that plane A reads or that the archived engine build cannot regenerate. | T | P2 | DR-C4, ADR-0007, RT-M3 |
| DATA-5 | An event row shall be created only by plane-A criteria: a tellable act class, a named (T1) participant, rarity above a threshold, or a material input. Pins never create event rows. Commoner life shall be recorded as regional aggregates, `aggregate_event`s and the life-course log. | A/T | P1b | R10, ADR-0024 |
| DATA-6 | The durable formats shall be Parquet (events, aggregates, life-course log) and Arrow IPC (keyframes), with the input journal in SQLite (WAL). DuckDB files are caches. | I | P0 | R10 |
| DATA-7 | A **facts ledger** (plane B) shall persist everything revealed to you about any person or place, with source event IDs. Every later presentation shall honour it. Plane A never reads it. | T | P2 | MOE-3, ADR-0024 |
| DATA-8 | **Life-course log.** Every person ever born shall be recorded in an append-only, plane-A life-course log: identity, parents, birth, death and cause, plus interval tables for residence, occupation, faction and marriage. The live ledger shall hold the living only; dead rows leave at once through the lifecycle queue. `PersonId`s shall be **genealogical and content-addressed**: `H64(mother_id, parity)` for births, `H64(seed, "founder", k)` for founders and `H64(seed, "immigrant", origin, t, k)` for immigrants and off-map materialisation, with a deterministic rehash on collision. IDs are never reused. | T | P1a | ADR-0026, RT-C3, RT-M14 |
| DATA-9 | **Knowledge fronts.** Knowledge of public, crowd and document events among the population shall be stored as **diffusion fronts** (arrival tick per event and region, with channel and fidelity), not as per-person rows. Per-actor `knowledge` rows are kept only for witnessed, faction-internal and secret events and for testimony hops between named actors. `knows(a, e, t)` shall evaluate as `row(a, e) OR (public(e) AND arrival(region(a, t), e) ≤ t)`. | T/A | P3a | `05.2` §4.1, RT-M7 |

### 2.3a World Stream (STR; ADR-0030, `05.10`)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| STR-1 | The engine shall stream its world as one ordered, append-only, sequenced stream per timeline: channel A (every authoritative event with causes, every input at its applied tick, a digest per macro step, keyframe markers) and channel C (sift annotations). Channel A shall be byte-identical for the same build, Registry, manifest, seed and journal, at any pace and thread count. | T | P1.0 (v0), P1b (v1) | Director (2026-10-16) |
| STR-2 | **Fidelity.** Every plane-A state change shall be either a channel-A record or reproducible by deterministic replay from a keyframe. The materializer shall rebuild the exact state (equal `state_hash`) at any tick. | T | P1b | Director (2026-10-16) |
| STR-3 | **Viewing is a buffer event.** Viewers shall read the stream with their own cursors, at any playback rate, live or behind, from memory or sealed segments, with no gaps or duplicates. No reader shall slow the engine or change the stream. | T | P1b | Director (2026-10-16) |
| STR-4 | **Interaction is a tip event.** A material input shall land at the next tick boundary after the tip, or at a scheduled future tick, and be journaled with that tick. An input aimed at a past tick shall be refused unless issued as a fork. Every harness shall show the tip, its pace, the viewer's cursor and the lag, and confirm the landing date before an action. | T | P2 | Director (2026-10-16); ADR-0025 |
| STR-5 | **Pace is the director's setting.** The engine's pace (speed band or MAX) shall be set only by control commands, logged in the run log and never in the input journal. In Watch style an auto-pause pauses the viewer's playback; in Play style it pauses the tip. | T | P2 | Director (2026-10-16) |

### 2.4 Story sifting and significance (SIFT)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| SIFT-1 | A declarative **pattern DSL** shall express ordered event clauses, variable bindings over abstract roles and act classes, time windows, `unless` clauses and anchored state guards (`attr@event`). Attributes are declared and typed in the Registry; a bare or undeclared read is a compile error. | T | P2 | DR-M4 |
| SIFT-2 | The live sifter shall match **incrementally** in-process, tracking partial matches, so that the engine can auto-pause on completion or on high-suspense partials. | T | P2 | SN-1 |
| SIFT-3 | Every pattern shall have a retrospective SQL twin. CI shall assert identical match sets on recorded chronicles. | T | P2 | R10 |
| SIFT-4 | Pattern completions shall emit `match` events, in a separate `MatchId` space (plane C), that higher-order patterns can consume. Editing a pattern never renumbers simulation events. | T | P2 | R02, ADR-0024 |
| SIFT-5 | Significance (plane C) shall be computed retroactively, per era, from components: tellability, rarity, consequence (weighted descendants), centrality, stakes, reversal, irony, suspense, kernel-ness. | T | P2 | DR-C4 |
| SIFT-6 | **Hindsight retention.** When an event's significance crosses θ, its ancestors shall be marked *retained-visible* in the visibility sidecar (plane C), following `contingent` edges plus the single top `motivated_by` edge per node, and stopping at Registry-declared root event kinds or at depth 12. Retention never changes plane A. | T | P2 | DR-C4, RT-m7 |
| SIFT-7 | LLMs shall not participate in detection. They may re-rank, explain and propose patterns offline for human review. A proposed pattern is run against its SQL twin and its precision@K is reported before it can be accepted. | I | P2 | R02 |
| SIFT-8 | **Match semantics and budgets.** Every pattern shall declare `selection = latest \| earliest \| all` (default `latest`), `on_complete = clear_binding \| consume \| keep` (default `clear_binding`) and an optional `dedupe = {key, bucket_days}`. Pruning shall be declarative only (windows, `unless`, death tolerance and these semantics), so the SQL twin can express it. Each pattern shall declare a firing-rate budget per century per 100k persons. CI shall fail a pattern above 3× its budget on the reference ensembles, and shall report precision@K against your labelled story-worthy set. Twin and budget tests run on clustered fixture chronicles. | T | P2 | RT-C4, `05.4` §2.3 |

### 2.5 Cognition and epistemics (COG)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| COG-1 | Named actors shall hold a generic psychological core: HEXACO traits, Schwartz values, PAD mood, ≤ 8 OCC emotions with decay, needs, opinion-modifier lists per relationship, a belief store and memory references. | T | P3a | DR-M9 |
| COG-2 | Setting-specific psychological stats (e.g., Paranoia, Filial Piety) shall be **derived overlays** defined in the manifest. | T | P3a | DR-M9 |
| COG-3 | Goal selection shall use event-driven utility scoring (IAUS); plan decomposition shall use bounded HTN. Per-tick A*/GOAP is forbidden in fast-forward. | T/A | P3a | DR-M19 |
| COG-4 | Belief records shall carry evidence type, source, channel, confidence (log-odds), acquisition time and distortion lineage. | T | P3a | DR-M8 |
| COG-5 | Information shall propagate through typed **channels** (witness, testimony, document, crowd, whisper, hive), each with latency, fidelity, trust and reach. | T | P3a | DR-M8, DR-m9 |
| COG-6 | The masses shall hold **pop-level** belief shares and morale distributions per (region × class × faith). | T | P3a | R04 |
| COG-7 | Hive mimics shall carry cover-maintenance and exposure-risk utility. Hive-only knowledge used in action shall emit **impossible-knowledge** events. | T | P3a | DR-M16 |
| COG-8 | An optional **deliberation consult** (off by default) shall let an LLM choose a named actor's action only from a simulation-generated legal-action menu. The answer is a pull input (DET-15): validated, journaled and applied at `t + D`. Foresight rollouts and MAX-speed runs use the declared fallback policy and are labelled as such. | T | P3b | R04, RT-M2 |

### 2.6 LLM platform (LLM)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| LLM-1 | All model calls shall go through one **gateway** with typed job kinds, schemas, budgets, retries and full request/response logging. | I/T | P3b | D6 |
| LLM-2 | Any LLM output that affects state shall be journaled as an input. Replays shall never re-query a model. | T | P3b | DET-1 |
| LLM-3 | Structured outputs shall use grammar-constrained decoding (provider structured outputs, llguidance or XGrammar) with enums generated from live registries, followed by semantic validation and at most 3 repair rounds. | T | P3b | DR-C2 |
| LLM-4 | Interactive local inference shall target TTFT < 0.5 s for a cached persona prefix on the reference GPU. | T | P6 | R05 |
| LLM-5 | NPC dialogue shall be **planned by the simulation** (`SpeechPlan`) and only realised by the model. The context shall contain only the speaker's persona and *belief-scoped* facts, never world truth. Replies shall pass a post-hoc claim check against the speaker's beliefs. | T | P6 | DR-M6 |
| LLM-6 | Player free text that influences state shall be parsed into closed-schema frames by a tool-less model. The parser's output shall never contain numeric state deltas. | T | P3b | DR-M7 |
| LLM-7 | A red-team corpus (injections, master-key tokens, persuasion-taxonomy prompts) shall run in CI against every LLM job that consumes player text. | T | P3b | R04 |
| LLM-8 | Each job type shall define a local and a cloud model path. The world must remain fully usable offline, with degraded narration. | D | P4 | R05 |

### 2.7 World compiler (WC)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| WC-1 | The **Mechanics Registry** shall be the closed, versioned vocabulary for manifests: systems (with owned columns), archetypes, act classes, roles, verbs, typed attributes (with owning plane), goods, technologies, governments, succession laws, institutions, channels, overlays, pattern packs and parameter bounds, plus the effect language for data-only extensions (`05.5` §2.1). | I/T | P0 | DR-C2, ADR-0010 |
| WC-2 | Hand-authored manifests (YAML/TOML) shall be first-class. The LLM compiler is an optional front end that emits the same format. Flavour text is stored in a hashed flavour asset that no simulation crate may read. | D | P1b | R11, ADR-0010 |
| WC-3 | The compile pipeline shall run in order: intent and mapping table (LLM) → deterministic expander → schema, referential and semantic validation (LLM repair ≤ 3 rounds, for schema and semantic errors only) → **fast liveness gate** (200 y × 8 seeds, blocking; bad-run rate ≤ 1/8) → deterministic calibration repair within Registry bounds if the gate fails → optional spin-up → hashed manifest and flavour asset → background **certification** (1,000 y × 32 seeds against the stylized-fact bands). The LLM never tunes numbers. | T | P4 | DR-C2, RT-M9 |
| WC-4 | Every content-bearing prompt span shall be mapped as exact, approximate or unsupported. Approximate and unsupported mappings shall produce a structured **gap report** (`data_only` or `needs_code`), shown on Genesis, rather than a silent approximation. | T | P4 | S4, RT-M10 |
| WC-5 | Catalysts shall be initial conditions and probabilities, never forced outcomes. | I | P1b | R09 |

### 2.8 Time-Machine console (TM)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| TM-1 | Time controls shall be bands of **sim-time per wall-second** (Live, 1 h/s, 1 d/s, 1 wk/s, 1 mo/s, 1 yr/s, 1 decade/s, MAX) plus **run-until** (date, predicate, or next match above a salience threshold). | D/T | P2 | DR-M1 |
| TM-2 | Auto-pause shall trigger on pattern completion, or on open partial matches whose completion is high-stakes (suspense), using thresholds you set. A rate limiter shall cap suspense auto-pauses (default ≤ 1 per simulated decade). | T | P2 | SN-1 |
| TM-3 | The timeline shall scrub over chronicle and aggregates without re-simulation (MOP-3). "Drop in" shall seek through keyframe plus replay (MOP-4) and is read-only (DET-14). | T | P2 | SN-4 |
| TM-4 | The **causal inspector** shall show, for any event, its typed ancestors and descendants, the anchored state values it was evaluated on, and its significance components. In Oracle mode it shows only what the Oracle knows (ORA-9). | D | P2 | SN-2 |
| TM-5 | The map shall show regions, polities, routes, fields and armies at any tick, with level-of-detail aggregation. | D | P2 | SN-1 |
| TM-6 | The console shall include an in-built **rating tool** that records your judgement of each auto-paused event (MOE-1). | D | P2 | MOE-1 |
| TM-7 | Any person, living or dead, shall be inspectable: ledger row (if living), life-course record, genealogy, life events and the facts already shown to you (facts ledger). Inspection writes only to plane B. | D | P2 | SN-5, ADR-0026 |

### 2.9 Oracle and Foresight (ORA)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| ORA-1 | The Oracle shall be an in-world entity with location, travel time and mortality. Its verbs shall be gated by co-location and Foresight. | T | P3b | V7 |
| ORA-2 | **Whisper Counsel** shall follow parse → ground (against the Oracle's belief store) → deterministic score → bounded Thought modifier → shown parse (`05.3` §6). | T | P3b | DR-M7 |
| ORA-3 | **Pierce the Veil** shall add a `vision` belief about hidden state to the Oracle's belief store. NPCs cannot verify a vision; acting on it in-world requires evidence-backed counsel. | T | P3b | R04 |
| ORA-4 | **Plant Omen** shall act on pop-level beliefs through a Bayesian-persuasion credibility model that decays with overuse. | T | P3b | R04 |
| ORA-5 | **Foresight** shall run N forked Monte-Carlo rollouts with branch-salted RNG and report outcome probabilities with Wilson intervals for player-defined predicates ("prophecies"). Its forecasts shall pass the validity tests of `09` §3.3: exchangeability, reduced-cost bias ≤ 0.03, and calibration scoring. | T/A | P3b | D7, RT-M5 |
| ORA-6 | All Oracle actions shall be journaled inputs and appear in the chronicle as `player` causal edges. | T | P3b | DET-7 |
| ORA-7 | **Oracle-3D.** The Oracle shall be playable in the Godot client: a strategic travel map with time compression while travelling, and drop-in to scenes for conversations. Oracle verbs in 3D shall share the console's backend (`oracle.*`), so their effects and journal records are identical. | D | P7 | SN-6, `05.7` B7 |
| ORA-8 | **Forbidden Ward** (fantasy registries only) shall apply a physical-field effect at a coordinate for a duration in sim-time. It is journaled like every Oracle verb, and a Registry without the Ward shall reject it at validation. | T | P3b | `05.3` §6.1 |
| ORA-9 | **Oracle knowledge and console modes.** The Oracle shall be a plane-A entity with its own belief store, fed by the same channels as everyone else (witness by co-location, testimony, documents, and `vision` from Pierce). The console shall run in one of two journaled modes. **Cultivator** shows omniscient views; Oracle verbs are disabled, or, if enabled, journaled as `omniscient` and excluded from Oracle-mode statistics. **Oracle** filters every view (map, inspector, chronicle, person pages, Foresight) by the Oracle's knowledge. In Oracle mode, Foresight shall fork from a **belief-resampled** state: hidden attributes the Oracle does not know are re-drawn per rollout from its posterior. | T | P3b | RT-M13, `05.3` §6.2 |

### 2.10 Narrative compiler (NAR)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| NAR-1 | Forms shall be offered in escalating order: Annals (templated, no LLM), Chronicle, Saga, In-world Documents, Novella/Novel (behind a story-shape gate), Screenplay (Fountain), and Audio-drama script. Annals and the templated Chronicle bind at P2 (WP P2.8); the rest at P4. | D | P2/P4 | DR-M13 |
| NAR-2 | The pipeline shall run in order: digest → selection → discourse plan → bible → beats → scene drafting → scene QA → critique → book pass → exporters, each producing a versioned artifact with provenance. | I/T | P4 | R08 |
| NAR-3 | Canon policy tiers CT0–CT3 shall be enforced. CT0/CT1 contradictions shall block publication (MOP-6). | T | P4 | R08 |
| NAR-4 | Style shall be controlled by **style cards** with public-domain exemplars. Living or estate-protected author names shall not be used as style controls. | I | P4 | DR-M13 |
| NAR-5 | Every sentence of output shall be traceable to the event IDs it narrates (sidecar provenance). | T | P4 | D1 |

### 2.11 3D clients (R3D)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| R3D-1 | The renderer shall be a separate process consuming binary snapshot and delta frames, plus JSON-RPC control. | I | P5 | DR-M12 |
| R3D-2 | Street-level content (terrain detail, roads, plots, buildings, interiors) shall be generated deterministically from a seed hierarchy plus the simulation snapshot, by versioned generators. Simulation events persist as overrides. | T | P5 | DR-M10 |
| R3D-3 | Crowd rendering shall use LoD tiers: embodied T3 (rigged) inside the hydration ring, at Live speed only, from P6; T2c crowd proxies as VAT MultiMesh to 400 m; impostors to 2 km; aggregates beyond. Spectacle-class entities (T1 actors and SoA objects with a world-scale footprint, such as dragons and armies) shall be visible to draw distance at any speed band. | D | P5 | DR-M11 |
| R3D-4 | Hydration rings shall exist only at Live speed and use hysteresis (hydrate at 80 m, dehydrate at 130 m plus ≥ 10 s out of view), never dehydrate an entity in view, and stay within a budget of ~5–10 hydrations per second. | T | P6 | DR-C6 |
| R3D-5 | Wallpaper mode shall cap at 30 FPS, drop to 0 FPS when covered or under a fullscreen application, switch to "photograph" mode on battery, and stop audio streams when paused. | D | P5 | DR-M18 |
| R3D-6 | Art direction shall follow the "painted miniature" bible: palette atlases, a LUT per world and era, and tilt-shift in Diorama mode. All imported assets shall be re-shaded. | I | P5 | R06 |
| R3D-7 | A **camera director** (Diorama) shall choose among vantages pre-computed per settlement and region, scored by salience of nearby live events × visual interest × novelty. It changes vantage on a slow cadence with long dissolves, and favours stable shots in wallpaper mode (`05.7` B5). | D | P5 | SN-8, `05.7` B5 |
| R3D-8 | **Replay mode.** The Diorama shall render a computed history from the chronicle, keyframes and journal without running simulation work, so it can run on laptops. Replay is read-only. | D/T | P5 | DR-M18 |
| R3D-9 | **Investigation templates.** Street-Level shall offer scenario templates that give a purpose for walking around (e.g. *investigate the mimic*, *find the plotter*). They are built on simulation state and material inputs only, never on scripted outcomes. | D | P6 | R11, `05.7` B7 |
| R3D-10 | **Lo-fi glyph client first** (ADR-0029). Art phase 1 shall be a glyph view inside the console (deck.gl instanced glyph layer, CP437 or small-tileset glyph packs keyed by Registry IDs) serving the Diorama (including a Lively web-wallpaper mode), Street-level (with `@` movement via `player.act` and a dialogue panel), the Oracle travel map and the Time-Machine map. It shall render ≥ 10k cells at 60 FPS on a laptop iGPU and make no state decisions (no FOV, pathing or randomness). R3D-1, R3D-3, R3D-5 and R3D-6 apply to the optional 3D art phase 2. | T/I | P5 | Director 2026-10-15, ADR-0029 |

### 2.12 Audio (AUD)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| AUD-1 | Ambience shall be systemic: layered beds plus density beds plus capped, aggregated one-shots driven by an `AudioTelemetry` struct at ~10 Hz. There shall be no runtime neural audio generation. | D | P5 | DR-M17 |
| AUD-2 | NPC speech shall use licence-cleared TTS (Chatterbox family or Pocket TTS class) and a designed-voice bank. Real voices shall not be cloned. | I | P6 | R07 |
| AUD-3 | Audio drama shall be rendered line by line with an ASR word-error-rate (WER) gate, speaker-similarity checks, mastering to −16 LUFS / −1 dBTP, chapters, and a private RSS feed. | T | P4+ | R07 |
| AUD-4 | **Conversational voice latency**, from the end of your speech to the start of NPC audio, shall be ≤ 2 s at p95 on the reference machine (`08`). | T | P6 | `05.8` §2, `11` G6 |

### 2.13 Non-functional: performance, evolvability, observability, safety, licensing

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| PERF-1 | MOP-1: 1,000 years at 100k persons, monthly resolution, in ≤ 2 min on an 8-core desktop (P1b). At 1M persons in ≤ 15 min (P3b; provisional, confirmed or revised at G3b, `08` §1.4). | T | P1b/P3b | MOP-1, RT-M8 |
| PERF-2 | The macro-step budget at 1M persons on the reference desktop shall be set at G1b from P1 measurements (WP P1.14) and shall hold from P3b. Provisional values: ≤ 60 ms per normal sim-month and ≤ 130 ms per crisis sim-month, with serial work ≤ 20% of tick wall time (`08` §1). | T | P1b/P3b | MOP-1, RT-M8, `08` §1 |
| PERF-3 | The live sifter shall add ≤ 10% to the run time at MAX speed at 100k persons (P2), and ≤ 20% at 1M persons (P3b). | T | P2/P3b | SN-1, R-06 |
| PERF-4 | Benchmarks shall run in CI with regression alarms above 10%. A speed-up never re-baselines a gate automatically. | T | P0 | D6 |
| PERF-5 | **Harness isolation (ADR-0030).** The engine shall run headless and complete without any harness. With harnesses attached (frame subscribers, live hydration, queries at the maximum rate, and one deliberately stalled consumer), engine throughput shall be ≥ 95% of the no-harness run, memory shall grow by no more than the in-memory live-edge segment, and `state_hash` shall be unchanged. The tick loop never waits for a harness. | T | P2 | Director (2026-10-16) |
| EVO-1 | **Ownership and change scope.** Every column and relation shall have exactly one owning system, declared in the Registry with units and meaning. Other systems shall read it only through **contract crates** (types and read accessors, no implementation), and shall cause cross-system effects only through typed **lifecycle command queues** owned by `rh-core` and through events; systems never call each other. The scheduler shall derive system order within each phase by **topological sort** of the declared read and write sets (ties by system ID); a dependency against phase order must be declared as a **lag**. Every change shall carry a **change class** (C0–C3, `13` §4.2), detected by CI: C0–C2 changes must leave `state_hash` unchanged; a C3 (determinism-affecting) change is auto-approved if the 32-seed CRN-paired stylized-fact ensemble is equivalent within tolerance, and goes to you if it is not or if it changes the determinism machinery. A data-only Registry extension in the effect language (`05.5` §2.1) shall need no Rust change. A new system or fidelity level shall need its own crate, Registry entries and contract tests; edits elsewhere are limited to contract crates (new read accessors) and, where needed, new lifecycle command types. | I/T | P0 | S4, D5, RT-M11, RT-M12 |
| EVO-2 | Every system crate shall ship with property tests (invariants), a calibration notebook, and golden fixtures. | I | P1a | D6 |
| EVO-3 | **Fidelity levels.** Each fidelity level shall be validated against the stylized-fact bands on its own. Between F(n) and F(n+1), consistency shall be required only for **conserved quantities** and the system's **declared targets**, not for every aggregate. A published **supported-combinations matrix** shall list the tested fidelity mixes. Systems that mix levels across a boundary shall declare lift and project adapters with conservation and distribution tests. Fidelity switches happen only through a migration keyframe. The per-level band check binds from P1b; the F(n)/F(n+1) contract binds with the first second level (WP P5.0, `M4.economy` F2). | T | P1b/P5 | D5, RT-m2, RT-M6 |
| OBS-1 | Per-system timing, invariant counters and sampled agents shall be logged to Rerun on a `sim_tick` timeline. A Textual TUI shall cover headless runs. | D | P1a | D6 |
| SEC-1 | The LLM worst case shall be bounded. No single player input may move any state variable by more than the documented cap. Meta-instructions are penalised in-world. Oracle leverage (ΔP per Foresight spent) shall stay within declared bands, and no strategy of the adversarial player bot may exceed the cap envelope (`05.3` §6.3). | T | P3b | DR-M7, RT-m3 |
| LIC-1 | Dependencies, model weights and generated assets shall be on an allow-list with recorded licences and provenance. GPL, AGPL, non-commercial and territorially restricted items are excluded from shipped artifacts. | I | P0 | ADR-0021 |

## 3. Verification traceability

Every requirement above maps to at least one test ID. The full requirement → test matrix is in `09_verification_and_validation.md` §6; the suite definitions are in `09` §2–§3 and the statistical conventions in `09` §1.2. Each work package lists the requirement IDs it covers (`13` §2), and its acceptance gates cite the matching test IDs.
