# 02 — Design Review of the Draft (Red Team)

*Status: Draft v1.1 (2026-10-15). Reviewed: "REHOBOAM_DEFINITIVE_SYSTEM_DESIGN v2.0", "REHOBOAM_BOOTSTRAP_OPEN_SOURCE_ECOSYSTEM", "REHOBOAM_PRODUCT_VISION_AND_INTERFACES v1.0" and "rehoboam_2". Evidence: research dossiers `research/R01`–`R11`. Section numbers such as R03 §5 refer to those dossiers.*
*v1.1 changes: corrections updated to the v1.1 architecture (scheduled outcomes replace lookahead decrees, pinning is plane B, the burn-in is split into a liveness gate and background certification, hindsight retention replaces pinning of events); section and ADR references fixed.*

The bar for this review is your definition of success (`01_vision_and_intent.md` §2), not whether the drafts are well written. Each finding gives the claim, why it fails, the evidence, and the correction adopted in this plan.

**Finding IDs** are prefixed `DR-` (DR-C = critical, DR-M = major, DR-m = minor) so they don't collide with system IDs such as `M2.demography`.

**Severity.** **Critical**: the project fails or must be rewritten if this is not fixed. **Major**: it costs months, or quietly caps quality. **Minor**: hygiene and clarity.

---

## 0. What the draft gets right (and the plan keeps)

| Kept idea | Why it is right |
|---|---|
| A hard separation where **deterministic systems own truth** and LLMs act as compiler, facade and synthesizer | This matches the strongest finding across R02, R04, R05 and R11: LLMs in the authoritative loop break replay, cost too much at scale (R04 §4: ~2×10¹¹ tokens for 5k actors over 1,000 years), and invite "AI decided" backlash. |
| **Hierarchical level of detail** ("Schrödinger's Simulation") | Well-founded prior art: Sunshine-Hill's alibi generation, Brom's simulation LoD, and Watch Dogs: Legion's Census (R01 §1–2). The *idea* is right, but the implementation in the draft is not (see C6). |
| A **chronicle** of events and a **story sifter** over it | This is exactly the "curationist" architecture James Ryan's thesis argues for (R02 §TL;DR). |
| A **schema-validated world manifest** compiled from natural language | The right instinct. The schema itself has to be redesigned (DR-C2). |
| **Asymmetric minds** and the **covert mimic** | Rich, generalisable drama. Detecting the hive by imperfect tests produces witch-hunts from base rates alone, with nothing scripted (R04 §3, R09 §3). |
| The **Oracle**, played through influence rather than control | Proven niche appeal (King of Dragon Pass, Six Ages; R11 §B), and new in combination with causal graphs and hidden mimics. |
| **Godot** as the 3D client, **DuckDB** for analytics, a **decoupled** client | Confirmed by R06 and R10, with modifications. |

---

## 1. Critical findings

### DR-C1 — Five modalities with no sequencing, on a "Day 1–7" timeline
- **Claim.** The bootstrap plan integrates Flecs, Micropolis, DuckDB, Outlines and Godot in seven days. The roadmap builds all five modalities in five phases.
- **Problem.** Each modality has been the *entire product* of a multi-year precedent:

  | Modality | Closest precedent | Time to ship |
  |---|---|---|
  | Time-Machine | DF Legends / WorldBox | — |
  | Oracle | King of Dragon Pass | — |
  | Street-level | Shadows of Doubt | ~7 years, solo |
  | Diorama | Tiny Glade / Rusty's Retirement | — |
  | Narrative Compiler | Fable Showrunner | — |

  Ambitious solo simulations reach their first paid release after a median of ~6–7 years with *one* modality. URR is still pre-1.0 after 15 years (R11 §G).
- **Correction.** One engine, delivered as a sequence of modalities with phase gates (`01` §6, `11_roadmap.md`). AI agents compress implementation time, but not the design iteration needed for the simulation to become interesting.

### DR-C2 — The world DSL is not executable
- **Claim.** An LLM compiles a prompt into a "strict Pydantic JSON manifest" that "parameterizes the identical underlying engine". Outlines gives "100% valid JSON on first pass".
- **Problem.**
  1. `CatalystRule.trigger_event: str`, `consequence_action: str` and `preconditions: Dict[str, Any]` are **free strings with no semantics**. The LLM will produce `FORCE_COUNCIL_SCHISM` or `INITIATE_COVERT_INFRASTRUCTURE_BLACKOUT`, and nothing in the engine implements them. `Dict[str, Any]` also cannot be constrained by any grammar engine (R05 §5).
  2. Constrained decoding guarantees *syntax* only. Truncation, refusals, silently skipped schema keywords and enum-case drift still break outputs (R05 TL;DR 4).
  3. *Semantic* validity is never covered: population against carrying capacity, succession referencing existing houses, dragon energetics against biomass.
  4. Outlines is a Python library and can't ship in a game runtime.
- **Correction.** Introduce a **Mechanics Registry**, a closed vocabulary of implemented systems, archetypes, act classes, roles and tropes with bounded parameters. The pipeline becomes:
  1. The LLM emits a small *intent* object whose enums are generated from the registry.
  2. A deterministic **expander** produces the full manifest.
  3. Schema, referential and semantic validators run.
  4. An LLM repair loop runs at most 3 times, for schema and semantic errors only.
  5. A fast **liveness gate** (200 years × 8 seeds) blocks play. If it fails, a **deterministic calibration** search moves parameters within Registry bounds; the LLM never tunes numbers.
  6. **Certification** (1,000 years × 32 seeds against the stylized facts) runs in the background.

  Every prompt span is recorded in a **mapping table** as exact, approximate or unsupported. Approximate and unsupported concepts come out as **gap reports**, which become your agents' backlog (`05.5`).

### DR-C3 — The macro model cannot produce history
- **Claim.** 512² Laplacian hazard grids plus `ΔPop = Poisson(λb) − Poisson(λd) − casualties` per 16×16 chunk will simulate "1,000 years of demographic shifts, territorial conquest, dynastic lineages and viral contagion".
- **Problem.**
  1. The population model has **no density dependence and no age structure**. Over 1,000 years a 0.1%/yr bias multiplies population by 2.7. With exactly balanced rates, 78 of 1,024 chunks went extinct in R09's simulation.
  2. Poisson deaths can exceed the population.
  3. **There is no economy, polity, war, culture, technology, climate or migration model.** These are the systems that produce "rise and fall".
  4. **A linear Laplacian is the wrong model for contagion and for dragons.** It has no R₀ threshold, no susceptible depletion and no immunity, and relaxes to a static blob. Person-to-person contagion follows mobility networks (R09 §3).
  5. "Dynasts are promoted when a chunk crosses a narrative threshold" is backwards. A succession crisis needs genealogy that exists *before* the crisis (R09 claims table).
- **Correction.** A systems library, prioritised must/should/could (R09 §Recommendations):

  | ID | System |
  |---|---|
  | M1 | Region graph |
  | M2 | Age-structured, density-dependent demography (binomial draws) |
  | M3 | Agriculture and climate shocks |
  | M4 | Minimal economy |
  | M5 | Metapopulation SEIRS with covert compartment, testing and fear |
  | M6 | Structural-demographic polity |
  | M7 | Succession and dynasts |
  | M8 | Campaign war with logistics |
  | M9 | Stylized-fact validation |

  Laplacian diffusion is kept *only* for physical fields such as fire, smoke and flood.

### DR-C4 — The chronicle's retention rule destroys causality, and contradicts itself
- **Claim.** Only events with `dramatic_weight ≥ 8` enter the chronicle. Every 100 years, events with weight < 7 are deleted. Causes are stored in `causes_json`.
- **Problem.**
  1. The rules contradict each other: if only weight ≥ 8 is inserted, the purge never fires (R02 TL;DR 2).
  2. **Significance is retroactive.** The minor slight that starts a war 40 years later was never stored. Labov's "most reportable event" analysis walks backward through exactly those low-weight causes (R02 §4).
  3. Deleting events breaks causal references, destroys the base rates that rarity metrics need, and breaks determinism if the simulation ever reads its own history (R10 §R2).
  4. A JSON blob of causes is untyped and can't be indexed.
  5. The disk saved is trivial: ~2 GB per 250M events (R10 TL;DR 1).
- **Correction.**
  - A typed, append-only event store with a `causal_edge(kind ∈ {contingent, triggered, relayed_knowledge, motivated_by, enabled, …})` table.
  - Significance is a set of components recomputed each era.
  - Ancestors of significant events are marked **retained-visible** by a bounded backward walk (hindsight retention, `05.4` §3).
  - Named-actor events are never physically compacted. Low-salience events are hidden by a per-timeline visibility sidecar, and nothing is deleted by rule.
  - Typed Parquet costs ~11 B/event, so a 1,000-year chronicle of named actors is ~3 GB (R10).

### DR-C5 — Determinism and replay were never designed
- **Claim.** A "scrubbable timeline slider" spans 1,000 years, and you can "zoom in to inspect the exact causal event graph".
- **Problem.** Scrubbing a stochastic simulation needs keyframes, a deterministic re-simulation and an input journal. None are in the draft.
  - NumPy's `Generator` carries a "No Compatibility Guarantee" across versions.
  - Rayon's float reductions give different bits at different thread counts, and HashMap iteration order changes every run (measured in R03 §4).
  - A sequential RNG consumed by hydration makes **the camera path a hidden input**: watching changes history (R01 TL;DR 4).
- **Correction.** A determinism contract (`05.1` §6, ADR-0006):
  - integer sim-time;
  - counter-based keyed RNG for every draw;
  - fixed-point or integer values for anything that crosses a threshold;
  - libm only, no fast-math or FMA;
  - ordered iteration and fixed-chunk reductions;
  - separate state, observation and narrative hashes, monthly in CI (ADR-0024);
  - golden-run CI;
  - an input journal;
  - keyframes every 10 sim-years plus a lazy cache.

  Scrubbing reads the chronicle and aggregates and never re-simulates. Re-simulation happens only when you "drop in".

### DR-C6 — Hydration has no consistency model
- **Claim.** Unobserved entities "exist as compressed bitmasks and demographic aggregates". On observation the engine "collapses the probability wave" and kitbashes people within 100 m.
- **Problem.** This is the classic multi-resolution-modelling (MRM) consistency failure (R01 §3):
  - *Mapping inconsistency*: hydrated people don't sum to the aggregate counts.
  - *Temporal inconsistency*: the bubble's clock and the macro clock disagree.
  - *Chain disaggregation*: talking to a smith hydrates his family, then his guild.
  - *Thrashing* at the bubble edge.

  On top of that, there is no **identity model** (is the blacksmith you met yesterday the same person today?) and no **pinning policy**. Despawn-on-look-away is the most criticised failure in shipped games, e.g. Cyberpunk 2077 (R01 §2).
- **Correction.**
  - **Every person is a narrow ledger row** (19–64 B, ≤ 64 MB at 1M people; R01 §5). Hydration then only adds *detail* on top of existing facts ("pseudo-disaggregation").
  - Person IDs are **genealogical** ("this mother's n-th child"), and every person ever born is kept in a life-course log (ADR-0026).
  - Immutable traits come from `H(world_seed, person_id, field)`.
  - Pinning is engagement-based and sticky. It lives in the observation plane: it never changes history and never promotes anyone to a named actor (ADR-0024).
  - **Authority never moves to the bubble.** Deaths, births and other outcomes are scheduled timers for everyone, watched or not, and the bubble only displays them (ADR-0025). This supersedes the v1.0 plan's leases and lookahead decrees.
  - Relations are hydrated as stubs, with a depth cap.
  - Hysteresis rings: hydrate at 80 m, dehydrate at 130 m, and never in view.

  Together these give the six consistency criteria in `05.2` §3.6.

---

## 2. Major findings

| # | Draft claim | Problem | Evidence | Correction |
|---|---|---|---|---|
| DR-M1 | ">5,000 ticks/s headless"; speed presets "1×, 100×, 10,000×" simulate 1,000 years "in minutes"; Stratum 2 runs at 10–30 Hz | At 10,000×, 1,000 years take **36.5 days**; doing it in 60 s needs 5.3×10⁸×. Measured per tick: NumPy ≈ 50 ticks/s, Rust ≈ 290 ticks/s for the described workload. 30 Hz agents over 1,000 years = 9.5×10¹¹ ticks. | R03 §5–6 | Integer sim-time. Multi-rate pulses (year/season/month/day). Event-driven named actors. Time controls as *sim-time per wall-second* bands plus MAX plus run-until (`05.1` §4.4). |
| DR-M2 | "Flecs C99/C++/Python bindings"; "Flecs / Python ECS" | **No maintained Python binding exists** (PyPI `flecs` 0.0.2 is a prototype). Two runtimes cannot both own authoritative state. Pure-Python utility AI took 44.7 ms per tick for 5k actors; Rust took 0.81 ms. | R03 §1, R01 claims table | Rust core. Named actors in `bevy_ecs` (pinned version) behind a facade. Population in SoA tables. Python for tooling (ADR-0001/0002). |
| DR-M3 | "Cannibalize" Micropolis, Ceptre, Felt, df-storyteller, Ensemble, Talk of the Town, BadNews, unified-planning, Concordia | Licence and maintenance problems throughout: **Micropolis GPLv3 + EA terms**; **Ceptre Anti-Capitalist licence**; **Felt/Winnow unlicensed**; **df-storyteller AGPL, dormant**; Ensemble BSD-4; Talk of the Town is Python 2; **Bad News has no code**; unified-planning costs 11.9 ms per validation and its Fast Downward wrapper is GPL; **Concordia's LLM game master decides outcomes**, the opposite of the thesis. | R02 §1, R03, R04 | Reference designs only. Reimplement ideas cleanly. Licence allow-list (ADR-0021). |
| DR-M4 | Sifter rules such as `psychology(Sovereign,"paranoia",P), P > 80` | **Temporal bug**: the rule reads *current* state, not state *at the event*. Results depend on when sifting runs, which breaks determinism. The vocabulary is also feudal-specific. | R02 §6, R10 §3 | Rules are anchored (`paranoia@e1`). Captured snapshots at emission time, SCD2 interval tables, and patterns over *abstract roles and act classes* (`05.4`). |
| DR-M5 | Qwen 2.5 3B / Llama 3.2 3B / Llama 3.3 70B; Ollama/vLLM; "50+ concurrent conversations at >100 tok/s" | The models are 2024-era and superseded by Qwen3.5/3.6/3.8 and Gemma 4 (Apache-2.0). **vLLM has no native Windows support** and takes 92% of VRAM by default. The player speaks to one NPC at a time. | R05 §1–4 | A provider-agnostic gateway. llama.cpp for interactive local use; vLLM/SGLang on Linux GPU servers for batch work; frontier APIs for synthesis. One foreground slot plus 2–4 background slots (ADR-0011). |
| DR-M6 | "The SLM knows *only* what that character has witnessed" | Not achievable by prompting: GPT-4o leaks "future" knowledge on 54% of TimeChara probes, and small models do worse. Persona drift appears within ~8 turns. | R04 §6, R05 §6 | **Plan, then realise.** The simulation chooses a speech-act plan over belief IDs (reveal/conceal/lie/deflect), the LLM realises it as prose, and a post-hoc claim check runs against the speaker's belief store. World truth never enters the prompt. |
| DR-M7 | Whisper Counsel "injects high-priority text into the target's cognitive context" | Targets are numeric utility minds, so there is nothing for text to land in. LLM targets are trivially persuadable, LLM judges have position and verbosity bias and "master-key" token vulnerabilities, and prompt injection can only be bounded, not prevented. | R04 §5 | **Parse, don't inject.** Text becomes a typed `ArgumentFrame`, grounded against what the player actually knows, and scored *deterministically* (elaboration-likelihood route × value-based argumentation congruence × evidence × trust × habituation × cap × seeded resistance). The parse is shown to the player. |
| DR-M8 | Beliefs update by line of sight only | Most of what people know is hearsay. Outside the 3D bubble there is no geometry to ray-cast. | R04 §3 | Evidence-typed beliefs, after Ryan et al. 2015: observation, statement, eavesdrop, confabulation, lie… Co-presence witness sets. Information channels with latency, fidelity and trust. **The hive is one such channel**: {latency 0, fidelity 1, trust 1, reach all}. |
| DR-M9 | PsychState = ⟨Despair, Hubris, Paranoia, FilialPiety, Corruption⟩ | Mixes timescales and kinds: mood, trait plus state, value, setting stat. Not generic across worlds. `relic_corruption` is used in a rule but is not in the vector. | R04 §2 | Generic core: HEXACO traits, Schwartz values, PAD mood plus OCC emotions, needs, opinion modifiers, beliefs and memories. Setting stats are **derived overlays** shown in the UI. |
| DR-M10 | "Godot procedural mesh generation and terrain heightmaps" cover the 3D world | **Nothing generates roads, plots, buildings, settlements or interiors** from a 512² grid. This is the largest unaddressed content problem. | R06 §4 | A seed-hierarchy content pipeline: Azgaar-style macro (MIT), ward or tensor-field streets, plots, then grammar and WFC kit buildings, with lazy interiors and versioned generators (`05.7`). |
| DR-M11 | 100 m hydration radius, 50–200 entities | 100 m is an *interaction* radius, not a *visibility* radius. A human is 17 px tall at 100 m. The draft's own vistas (ramparts with dragons, river bends) need 1–2 km. Godot's skeletal animation lags at ~100 rigged characters. | R01 §R1, R06 §5 | Ring LoD: embodied ≤ 80/130 m; crowd proxies (VAT) to 300–500 m; impostors to 2 km; "Spectacle" class for dragons and armies. VAT is mandatory, not polish. |
| DR-M12 | JSON-RPC 2.0 WebSocket stream of entity JSON diffs; SQLite spatial index for hydration | JSON-RPC has no sequence, baseline or resync semantics, and becomes per-entity GDScript dictionaries. A SQLite R*Tree updates 5k movers in 378 ms per tick, against an in-memory grid query of 55 µs. | R10 §5, R01 claims | JSON-RPC for *control only*. Binary columnar delta frames for state (≈ 9× smaller). Interest management. In-memory spatial hash. |
| DR-M13 | Novels "styled after Frank Herbert, J.R.R. Tolkien, or Peter F. Hamilton", produced by "a frontier reasoning model" | All three are under copyright, Hamilton is living, and the Tolkien estate litigates. In-context imitation of a named author loses to expert readers. No model writes a coherent 100k-word novel in one call (output cap 128K tokens). | R08 §6, R11 | A hierarchical pipeline: digest → discourse plan → bible → beats → parallel scenes → claim checks → critique → book pass. **Style cards** with public-domain exemplars. Chronicle/Saga by default; novel only past a story-shape gate. |
| DR-M14 | 16×16 chunks as the demographic unit | 1,024 chunks of ~98 people at 100k. That is too small for stable demography and arbitrary with respect to geography: politics, markets and armies follow provinces and routes. | R09 claims | A **region graph** (300–2,000 provinces) for society. The raster stays for terrain, climate and physical hazards. |
| DR-M15 | A modern 1M-person megacity in the "Millenary" time machine | There is no defensible 1,000-year model of a modern city. | R09 §7 | Horizons are per scenario. Modern cities run 1–50 years (nominally 1–30) at day/week resolution, or are explicitly labelled speculative with a declared technology and institution trajectory. |
| DR-M16 | Hive utility `U = Σbiomass + Σinfiltrated − λΣdrones` | Linear and undiscounted, so it drives greedy infiltration until exposure. A pure utility repoint makes mimics statistically detectable. Testing base rates are not modelled. | R04 §4.3, R09 §3 | Add exposure risk, discounting and a concave infiltration term, plus a cover-maintenance term for mimics. Tells: impossible knowledge, behavioural drift and imperfect tests. Positive predictive value (PPV) collapse at low prevalence produces witch-hunts. |
| DR-M17 | "Generative procedural audio synthesizing real-time soundscapes" | Runtime neural audio costs seconds of latency and competes for the GPU. An always-open Godot audio device stops the PC sleeping. Many "open" TTS models are non-commercial (F5, Fish S2, Higgs v3, XTTS), and espeak-ng is GPL. | R07 | Systemic sample-based layers driven by telemetry (RTPC-style). Generative audio offline only. Licence-vetted TTS (Chatterbox, Pocket, Qwen3-TTS, MOSS). Designed voices only. |
| DR-M18 | The 3D Diorama runs as a desktop wallpaper while you work | Running a live 100k-agent simulation plus a 3D renderer all day breaks the power budget. Lively disables its pause rules for application wallpapers. Wallpaper Engine may not allow application wallpapers on its Workshop. | R06 §7, R11 §F | The renderer throttles itself (30 FPS cap, 0 FPS when covered, a "photograph" mode on battery). The simulation runs in its own process, or the diorama shows **replay** of computed history. |
| DR-M19 | GOAP at 10–30 Hz for 5k actors | A* over world states for 5k actors at 30 Hz costs ~7–75 core-seconds per second. Shipped planners re-plan on events, over short plans. | R04 §1 | Event-driven utility scoring (IAUS) for goals, small HTN libraries for plans, CK3-style polling cadence. |
| DR-M20 | Foresight regenerates "1 point per 50 ticks"; `TimeScale` is a single enum per world | Tick length varies with resolution, so "per tick" means nothing. One time scale can't serve yearly demography and daily epidemics together. | R03 §5 | Every rate is per sim-time unit. Multiple clocks per world. |

---

## 3. Minor findings

| # | Item | Fix |
|---|---|---|
| DR-m1 | "Compressed bitmasks" is a misleading description of identity storage (R01) | Use narrow-dtype columns, with bitsets only for flags. |
| DR-m2 | `AssetKey = Hash(id) mod VariantCount` reshuffles almost every entity when one variant is added (R06 §8) | Eligible-set rendezvous hashing with semantic filters. |
| DR-m3 | PixiJS offered as an alternative to the 3D client (R06) | PixiJS is 2D. Use it, or deck.gl, only for the console map. |
| DR-m4 | Flecs explorer as the debug console (R03) | Its REST server is "not for production" and only applies if Flecs is used. Use Rerun for scrubbable debugging. |
| DR-m5 | Equations in the .docx are corrupted (`lpha`, `eta`, `abla`, `ext`): LaTeX escapes were lost | Keep specs in Markdown with checked math. Machine-readable specs live in `specs/`. |
| DR-m6 | The Datalog sample mixes SQL comment syntax (`-- is covert mimic`) into Prolog | Use a proper pattern DSL with a reference parser (`specs/sifting`). |
| DR-m7 | "Free-tier LLM" as a dependency (R05) | Free tiers are rate-limited and volatile. Cheap paid tiers cost fractions of a cent per compile. Keep a local fallback. |
| DR-m8 | The name "Rehoboam" collides with Westworld's predictive AI (R11 §H) | Fine for a personal project, and thematically apt for Foresight. Clear the trademark before any public release. |
| DR-m9 | `FactionArchetype.is_hive_mind` / `covert_infiltration` are boolean special cases | Generalise to an information-channel topology plus hidden-state mechanics in the registry. |
| DR-m10 | `macro_dimensions` fixed per world; 512 vs 256 inconsistent between the examples | Raster resolution is a fidelity parameter, independent of the region graph. |

---

## 4. Where the draft is not ambitious enough

Your answers removed the compute constraint and made fidelity and longevity the goal. That opens capabilities the draft never considered:

1. **Foresight as real forecasting.** The engine is deterministic and fast, so it can fork the current timeline and run hundreds of Monte-Carlo futures, giving *calibrated probabilities* of outcomes ("the heir is crowned before 1250: 31%"). An Oracle intervention can be evaluated by how it shifts that distribution. This is literally what the fictional Rehoboam does (R11 §H). It is the best use of unbounded compute, because it scales out across processes without distributing the world.
2. **Branching timelines as a first-class feature.** "What if the king had died in 1201?" is a fork with a new input. The chronicle shares the prefix copy-on-write (R10 §R4).
3. **Everyone has a genealogy.** With per-person ledger rows and a life-course log of everyone ever born (ADR-0026), every citizen has parents, a household and a birth tick. You can trace any blacksmith's ancestry through the centuries, and DF-style "historical figures" emerge naturally.
4. **Causal provenance as the UI's centrepiece.** "Click any event, see why it happened" is the most differentiated feature (R11 §Recommendations). Every narrated sentence links back to events.
5. **In-world documents as a narrative layer.** Chronicles, decrees, propaganda and unreliable myths are generated *from* the true causal record. This is Caves of Qud's technique run in the honest direction (R02 §2), and it gives Herbert-style epigraphs without Herbert's text (R08 §4).
6. **Gap reports drive evolution.** When a prompt asks for something the registry can't express, the compiler produces a structured gap report that becomes an agent work package. The engine grows toward what you ask of it. This is the mechanism behind success claim S4.
7. **The validation harness as a fidelity guard.** Stylized facts are checked automatically: Zipf city sizes, regicide rates, secular cycles of 200–300 years, Black Death–scale mortality (R09 §11). Every fidelity level must pass them on its own, so "more complex" never silently means "less plausible".
