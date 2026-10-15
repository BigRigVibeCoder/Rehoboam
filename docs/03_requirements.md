# 03 — Requirements Specification

*Status: Draft v1.0 (2026-10-15). Style: ISO/IEC/IEEE 29148-lite.*

**Reading the tables**
- **Verification method:** **T** = Test (automated), **A** = Analysis, **D** = Demonstration, **I** = Inspection.
- **Phase:** the roadmap phase where the requirement first becomes binding (`11_roadmap.md`).
- **Trace:** links to success claims S1–S4 and design drivers D1–D7 (`01_vision_and_intent.md`).

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
| SIM-1 | The engine shall represent every living person in a world as an individual **ledger row** with a stable `person_id`, up to at least 2M persons per timeline. | T | P1 | SN-5, C6 |
| SIM-2 | The engine shall represent society on a **region graph** (provinces with typed route edges) and physical fields on a **raster**, with resolutions set by the manifest. | T | P1 | C3, M14 |
| SIM-3 | The engine shall implement the **must-have systems library**: M1 Regions, M2 Demography, M3 Agriculture & Climate, M4 Economy, M6 Polity, M7 Succession & Dynasts, M9 Validation (P1); M5 Epidemics/Covert Contagion and M8 Campaign War (P3). | T | P1/P3 | C3 |
| SIM-4 | Every system shall implement a common `SystemModel` interface declaring: state schema, inputs and outputs, clock, fidelity level, conservation invariants and parameter bounds. | I/T | P0 | SN-10, D5 |
| SIM-5 | Vital events shall use **age-specific, density-dependent hazards** with **binomial** draws. Unbounded Poisson death draws are forbidden. | T | P1 | C3 |
| SIM-6 | The engine shall run multiple clocks (year, season, month, day) as pulses on one integer sim-time axis, with **adaptive** daily stepping only for active epidemics, campaigns and sieges. | T | P1 | M1, M20 |
| SIM-7 | Named actors (T1) shall be scheduled by a **discrete-event agenda** keyed `(time, phase, entity_id, seq)` and shall not be polled at a fixed frequency during fast-forward. | T | P1 | M1, M19 |
| SIM-8 | Conservation invariants (people, food, coin, pathogen counts) shall hold at every sync point across tiers. Violations shall fail the run in debug and CI builds. | T | P1 | C6 |
| SIM-9 | Ruling and great houses shall always be tracked as named actors with full genealogy from world start. | T | P1 | M20 |
| SIM-10 | Scenario horizon and resolution shall be declared per world. Modern-urban scenarios beyond 50 years shall be flagged *speculative*. | I | P1 | M15 |
| SIM-11 | The core shall build as a library with no I/O and with four hosts: headless CLI, Python wheel, network server, and an in-process GDExtension (P5+). | I/D | P0 | D6 |

### 2.2 Determinism, replay and timelines (DET)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| DET-1 | History shall be a pure function of `(engine_build, manifest, seed, input_journal)`. | T | P0 | SN-4, D3 |
| DET-2 | All stochastic draws shall use **counter-based keyed RNG**, `u = H(world_seed, system_id, entity_id, tick, purpose, k)`. No global or sequential RNG streams. | T/I | P0 | C5 |
| DET-3 | Authoritative threshold quantities (counts, money, loyalty, timers) shall be integer or fixed-point. Floats are permitted in fields, computed in fixed order with `libm`, and quantised at macro boundaries. | I/T | P0 | C5 |
| DET-4 | The state hash (xxh3-64) shall be identical across 1, 2, 4 and N threads and across repeated runs. It is recorded every sim-month. | T | P0 | MOP-2 |
| DET-5 | **Golden-run CI** shall replay reference worlds and compare state hashes each sim-year, on every merge. | T | P0 | MOE-4 |
| DET-6 | **Observer invariance:** hydration, inspection, rendering, the console and LLM facade activity shall not change any state hash unless a material input is journaled. | T | P5 | D4 |
| DET-7 | Every nondeterministic input shall be appended to the **input journal** with tick and sequence: player actions, accepted LLM outputs, manifest and settings. | T | P0 | DET-1 |
| DET-8 | The engine shall write **permanent keyframes** every 10 sim-years (configurable) and maintain a lazy LRU cache of yearly keyframes near the cursor. | T | P2 | MOP-4 |
| DET-9 | The engine shall support **forking**: a new `timeline_id` from `(parent, tick)`, with the chronicle prefix shared copy-on-write. | T | P2 | SN-4 |
| DET-10 | Saves shall be **versioned**. A newer engine shall load older worlds either bit-identically (pinned model versions) or through an explicit, logged migration. | T | P2 | MOE-4 |
| DET-11 | Read-only consumers (renderer, console, LLM gateway) shall access state only through immutable snapshot frames, under an OpenRA-style "unsynced" guard. | T | P1 | DET-6 |

### 2.3 Chronicle and data (DATA)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| DATA-1 | Events shall be stored in a typed, append-only store: `event`, `event_role`, `event_snapshot`, `causal_edge`, `knowledge`, `collection`, `sig_component`, `sift_match` (`05.2`). | I/T | P1 | C4 |
| DATA-2 | Causal edges shall be typed: `contingent`, `triggered`, `relayed_knowledge`, `motivated_by`, `enabled`, `probabilistic`, `player`, `director`. | T | P1 | C4 |
| DATA-3 | Events shall capture the predicate values that rules read **at emission time** (`event_snapshot`). Slowly changing facts shall be stored as `valid_from`/`valid_to` intervals. | T | P1 | M4 |
| DATA-4 | No event shall be deleted by a rule fixed at creation. Compaction shall fold unpinned events into summary collections and archive partitions, preserving base-rate counts. | T | P2 | C4 |
| DATA-5 | Commoner-level activity shall be recorded as regional aggregates. Per-person events shall be recorded only for named, pinned or materially involved persons. | A | P1 | R10 |
| DATA-6 | The durable formats shall be Parquet (events, aggregates) and Arrow IPC (keyframes). DuckDB files are caches. | I | P1 | R10 |
| DATA-7 | A **facts ledger** shall persist everything revealed to the player about any person or place, with source event IDs. | T | P5 | MOE-3 |

### 2.4 Story sifting and significance (SIFT)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| SIFT-1 | A declarative **pattern DSL** shall express ordered event clauses, variable bindings over abstract roles and act classes, time windows, `unless` clauses and anchored state guards (`attr@event`). | T | P2 | M4 |
| SIFT-2 | The live sifter shall match **incrementally** in-process, tracking partial matches, so that the engine can auto-pause on completion or on high-suspense partials. | T | P2 | SN-1 |
| SIFT-3 | Every pattern shall have a retrospective SQL twin. CI shall assert identical match sets on recorded chronicles. | T | P2 | R10 |
| SIFT-4 | Pattern completions shall emit `match` events that higher-order patterns can consume. | T | P2 | R02 |
| SIFT-5 | Significance shall be computed retroactively, per era, from components: tellability, rarity, consequence (weighted descendants), centrality, stakes, reversal, irony, suspense. | T | P2 | C4 |
| SIFT-6 | Contingency and motivation ancestors of any event above a significance threshold shall be **pinned**. | T | P2 | C4 |
| SIFT-7 | LLMs shall not participate in detection. They may re-rank, explain and propose patterns offline for human review. | I | P2 | R02 |

### 2.5 Cognition and epistemics (COG)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| COG-1 | Named actors shall hold a generic psychological core: HEXACO traits, Schwartz values, PAD mood, ≤ 8 OCC emotions with decay, needs, opinion-modifier lists per relationship, a belief store and memory references. | T | P3 | M9 |
| COG-2 | Setting-specific psychological stats (e.g., Paranoia, Filial Piety) shall be **derived overlays** defined in the manifest. | T | P3 | M9 |
| COG-3 | Goal selection shall use event-driven utility scoring (IAUS); plan decomposition shall use bounded HTN. Per-tick A*/GOAP is forbidden in fast-forward. | T/A | P3 | M19 |
| COG-4 | Belief records shall carry evidence type, source, channel, confidence (log-odds), acquisition time and distortion lineage. | T | P3 | M8 |
| COG-5 | Information shall propagate through typed **channels** (witness, testimony, document, crowd, whisper, hive), each with latency, fidelity, trust and reach. | T | P3 | M8, m9 |
| COG-6 | The masses shall hold **pop-level** belief shares and morale distributions per (region × class × faith). | T | P3 | R04 |
| COG-7 | Hive mimics shall carry cover-maintenance and exposure-risk utility. Hive-only knowledge used in action shall emit **impossible-knowledge** events. | T | P3 | M16 |
| COG-8 | Optional **pivotal LLM consult** for named actors shall choose only from a simulation-generated legal-action menu. The choice shall be validated and journaled, with a deterministic fallback, and disabled during MAX-speed runs unless pre-batched. | T | P3+ | R04 |

### 2.6 LLM platform (LLM)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| LLM-1 | All model calls shall go through one **gateway** with typed job kinds, schemas, budgets, retries and full request/response logging. | I/T | P2 | D6 |
| LLM-2 | Any LLM output that affects state shall be journaled as an input. Replays shall never re-query a model. | T | P2 | DET-1 |
| LLM-3 | Structured outputs shall use grammar-constrained decoding (provider structured outputs, llguidance or XGrammar) with enums generated from live registries, followed by semantic validation and at most 3 repair rounds. | T | P2 | C2 |
| LLM-4 | Interactive local inference shall target TTFT < 0.5 s for a cached persona prefix on the reference GPU. | T | P5 | R05 |
| LLM-5 | NPC dialogue context shall contain only the speaker's persona and *belief-scoped* facts, never world truth. Replies shall pass a post-hoc claim check against the speaker's beliefs. | T | P5 | M6 |
| LLM-6 | Player free text that influences state shall be parsed into closed-schema frames by a tool-less model. The parser's output shall never contain numeric state deltas. | T | P3 | M7 |
| LLM-7 | A red-team corpus (injections, master-key tokens, persuasion-taxonomy prompts) shall run in CI against every LLM job that consumes player text. | T | P3 | R04 |
| LLM-8 | Each job type shall define a local and a cloud model path. The world must remain fully usable offline, with degraded narration. | D | P4 | R05 |

### 2.7 World compiler (WC)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| WC-1 | The **Mechanics Registry** shall be the closed vocabulary for manifests: systems, archetypes, act classes, roles, tropes, goods, techs, laws and parameter bounds. It is versioned. | I/T | P1 | C2 |
| WC-2 | Hand-authored manifests (YAML/TOML) shall be first-class. The LLM compiler is an optional front end that emits the same format. | D | P1 | R11 |
| WC-3 | The compile pipeline shall run in order: intent (LLM) → deterministic expander → schema validation → semantic validation → 200-year burn-in against stylized facts → repair loop → hashed manifest. | T | P4 | C2 |
| WC-4 | Concepts not expressible in the registry shall produce a structured **gap report** rather than a silent approximation. | T | P4 | S4 |
| WC-5 | Catalysts shall be initial conditions and probabilities, never forced outcomes. | I | P1 | R09 |

### 2.8 Time-Machine console (TM)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| TM-1 | Time controls shall be bands of **sim-time per wall-second** (Live, 1 h/s, 1 d/s, 1 wk/s, 1 mo/s, 1 yr/s, 1 decade/s, MAX) plus **run-until** (date, predicate, or next match above a salience threshold). | D/T | P2 | M1 |
| TM-2 | Auto-pause shall trigger on pattern completion, or on open partial matches whose completion is high-stakes (suspense), using thresholds you set. | T | P2 | SN-1 |
| TM-3 | The timeline shall scrub over chronicle and aggregates without re-simulation (MOP-3). "Drop in" shall seek through keyframe plus replay (MOP-4). | T | P2 | SN-4 |
| TM-4 | The **causal inspector** shall show, for any event, its typed ancestors and descendants, the anchored state values it was evaluated on, and its significance components. | D | P2 | SN-2 |
| TM-5 | The map shall show regions, polities, routes, fields and armies at any tick, with level-of-detail aggregation. | D | P2 | SN-1 |
| TM-6 | The console shall include an in-built **rating tool** that records your judgement of each auto-paused event (MOE-1). | D | P2 | MOE-1 |
| TM-7 | Any person shall be inspectable: ledger row, genealogy, life events and pinned facts. | D | P2 | SN-5 |

### 2.9 Oracle and Foresight (ORA)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| ORA-1 | The Oracle shall be an in-world entity with location, travel time and mortality. Its verbs shall be gated by co-location and Foresight. | T | P3 | V7 |
| ORA-2 | **Whisper Counsel** shall follow parse → ground → deterministic score → bounded Thought modifier → shown parse (`05.3` §6). | T | P3 | M7 |
| ORA-3 | **Pierce the Veil** shall reveal hidden state to the player only. Acting on it in-world requires evidence-backed counsel. | T | P3 | R04 |
| ORA-4 | **Plant Omen** shall act on pop-level beliefs through a Bayesian-persuasion credibility model that decays with overuse. | T | P3 | R04 |
| ORA-5 | **Foresight** shall run N forked Monte-Carlo rollouts with branch-salted RNG and report outcome probabilities with confidence intervals for player-defined predicates ("prophecies"). | T/A | P3 | D7 |
| ORA-6 | All Oracle actions shall be journaled inputs and appear in the chronicle as `player` causal edges. | T | P3 | DET-7 |

### 2.10 Narrative compiler (NAR)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| NAR-1 | Forms shall be offered in escalating order: Annals (templated, no LLM), Chronicle, Saga, In-world Documents, Novella/Novel (behind a story-shape gate), Screenplay (Fountain), and Audio-drama script. | D | P4 | M13 |
| NAR-2 | The pipeline shall run in order: digest → selection → discourse plan → bible → beats → scene drafting → scene QA → critique → book pass → exporters, each producing a versioned artifact with provenance. | I/T | P4 | R08 |
| NAR-3 | Canon policy tiers T0–T3 shall be enforced. T0/T1 contradictions shall block publication (MOP-6). | T | P4 | R08 |
| NAR-4 | Style shall be controlled by **style cards** with public-domain exemplars. Living or estate-protected author names shall not be used as style controls. | I | P4 | M13 |
| NAR-5 | Every sentence of output shall be traceable to the event IDs it narrates (sidecar provenance). | T | P4 | D1 |

### 2.11 3D clients (R3D)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| R3D-1 | The renderer shall be a separate process consuming binary snapshot and delta frames, plus JSON-RPC control. | I | P5 | M12 |
| R3D-2 | Street-level content (terrain detail, roads, plots, buildings, interiors) shall be generated deterministically from a seed hierarchy plus the simulation snapshot, by versioned generators. Simulation events persist as overrides. | T | P5 | M10 |
| R3D-3 | Crowd rendering shall use LoD tiers: rigged ≤ 80 m, VAT MultiMesh to 400 m, impostors to 2 km, aggregates beyond. Spectacle-class entities shall be visible to draw distance. | D | P5 | M11 |
| R3D-4 | Hydration rings shall use hysteresis (hydrate at 80 m, dehydrate at 130 m plus ≥ 10 s out of view), never dehydrate an entity in view, and stay within a budget of ~5–10 hydrations per second. | T | P6 | C6 |
| R3D-5 | Wallpaper mode shall cap at 30 FPS, drop to 0 FPS when covered or under a fullscreen application, switch to "photograph" mode on battery, and stop audio streams when paused. | D | P5 | M18 |
| R3D-6 | Art direction shall follow the "painted miniature" bible: palette atlases, a LUT per world and era, and tilt-shift in Diorama mode. All imported assets shall be re-shaded. | I | P5 | R06 |

### 2.12 Audio (AUD)

| ID | Requirement | Ver. | Phase | Trace |
|---|---|---|---|---|
| AUD-1 | Ambience shall be systemic: layered beds plus density beds plus capped, aggregated one-shots driven by an `AudioTelemetry` struct at ~10 Hz. There shall be no runtime neural audio generation. | D | P5 | M17 |
| AUD-2 | NPC speech shall use licence-cleared TTS (Chatterbox family or Pocket TTS class) and a designed-voice bank. Real voices shall not be cloned. | I | P6 | R07 |
| AUD-3 | Audio drama shall be rendered line by line with an ASR word-error-rate (WER) gate, speaker-similarity checks, mastering to −16 LUFS / −1 dBTP, chapters, and a private RSS feed. | T | P4+ | R07 |

### 2.13 Non-functional: performance, evolvability, observability, safety, licensing

| ID | Requirement | Ver. | Phase |
|---|---|---|---|
| PERF-1 | MOP-1: 1,000 years at 100k persons, monthly resolution, in ≤ 2 min on an 8-core desktop (P1). At 1M persons in ≤ 10 min (P3). | T |  P1/P3 |
| PERF-2 | The macro-step budget at 1M persons (Rust, 8 cores) shall be ≤ 15 ms per sim-month, excluding active daily sub-steps. | T | P3 |
| PERF-3 | The live sifter shall add ≤ 10% to the run time at MAX speed. | T | P2 |
| PERF-4 | Benchmarks shall run in CI with regression alarms above 10%. | T | P0 |
| EVO-1 | A new system or fidelity level shall be addable as an isolated crate plus registry entries, with no edits to other systems' code (interfaces via the event bus and shared state tables). | I | P1 |
| EVO-2 | Every system crate shall ship with property tests (invariants), a calibration notebook, and golden fixtures. | I | P1 |
| EVO-3 | Each fidelity level F(n+1) shall reproduce F(n)'s declared aggregate statistics within declared tolerance on the reference manifests, or carry a documented exemption. | T | P3 |
| OBS-1 | Per-system timing, invariant counters and sampled agents shall be logged to Rerun on a `sim_tick` timeline. A Textual TUI shall cover headless runs. | D | P1 |
| SEC-1 | The LLM worst case shall be bounded. No single player input may move any state variable by more than the documented cap. Meta-instructions are penalised in-world. | T | P3 |
| LIC-1 | Dependencies, model weights and generated assets shall be on an allow-list with recorded licences and provenance. GPL, AGPL and non-commercial items are excluded from shipped artifacts. | I | P0 |

## 3. Verification matrix

Each requirement maps to at least one verification activity defined in `09_verification_and_validation.md`:

| Verification activity | Requirements covered |
|---|---|
| Golden-run suite | DET-* |
| Stylized-fact harness | SIM-3/5, MOP-5 |
| Invariant property tests | SIM-8, EVO-2 |
| Sifter twin tests | SIFT-3 |
| Narrative QA pipeline | NAR-3, NAR-5 |
| Role-play eval | LLM-5 |
| Red-team corpus | LLM-7, SEC-1 |
| Performance CI | PERF-* |
| Rating sessions | MOE-1/2 |
| Inspection checklists | ADR conformance, LIC-1 |
