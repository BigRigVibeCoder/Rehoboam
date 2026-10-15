# R04: Agent Cognition, Social Simulation, Epistemics and Persuasion for Rehoboam

*Research dossier, 2026-10-15. Scope: mind architectures for the masses, named actors and conversing NPCs; belief and rumor models; hive-mind infiltration and detection; LLM agent societies; persuasion as a mechanic; prompt-injection defense.*

> **Evidence legend.** In the Sources list, **[V]** means the analyst checked the item in this session against the repo, release page, code or a search-result abstract. **[R]** means a well-known publication cited from the analyst's prior knowledge and not re-fetched: the shared web-search budget ran out partway through, and arxiv.org, ACM, AAAI and Wikipedia were unreachable from the fetch tool. Treat [R] numbers and quotes as "check before you rely on them". Paragraphs headed **Inference** contain the analyst's own reasoning and estimates, not sourced facts.

---

## TL;DR

1. **Reject GOAP at 10–30 Hz for 5k actors.** In a fast-forwarded world "Hz" means nothing. Use **event-driven utility selection (IAUS-style)** for goals, **small HTN libraries** for plans, and level-of-detail scheduling: months in the time machine, 1–5 Hz near the camera. Shipped planners re-plan only on events, and their plans are short [48][50].
2. **PsychState mixes timescales.** Despair is a mood, Hubris and Paranoia are trait-plus-state, Filial Piety is a value, Corruption is a setting stat. Split it into a core (HEXACO traits, Schwartz values, PAD mood with a few decaying OCC emotions, needs, CK3-style decaying opinions, a belief store, event memory) and put setting stats on top as overlays.
3. **Reject line-of-sight-only beliefs.** Most of what people know is hearsay. Adopt Talk of the Town's evidence types [44] and use co-presence witness sets, not raycasts, outside the 3D bubble.
4. **The hive is a channel {latency 0, fidelity 1, trust 1, reach all}.** As one shared belief store it is *cheaper* than individual minds, and it creates a free detection tell: **impossible knowledge**.
5. **Base rates dominate detection.** At 1% prevalence, a 95%/95% test has a PPV of about 16%. If Paranoia sets the accusation criterion, witch-hunts emerge.
6. **Keep LLMs out of the inner loop.** At OASIS's ~3.4k input tokens per agent-step [17], 5k actors stepped monthly for 1,000 years is ~2×10¹¹ tokens, and no surveyed system is replay-deterministic. Consult an LLM only at rare in-focus decisions, from a legal-action menu, with outputs logged as inputs.
7. **Concordia** (v2.4.0, 2026-03-06, Apache-2.0, unsupported) [1][3] lets an **LLM Game Master decide outcomes**, the opposite of Rehoboam's thesis. Borrow its patterns, not its runtime.
8. **Smallville retrieval is wrong for 1,000 years.** It needs an LLM importance call per memory [5], the code computes recency as `0.99^rank` [6][7], and embeddings are non-deterministic. Use **ACT-R activation** over a structured event graph.
9. **Whisper Counsel: parse, don't inject.** Turn the text into a typed, logged **ArgumentFrame**, ground its claims, and score it **deterministically** (ELM route × value congruence × evidence × trust × habituation × cap × seeded resistance). The LLM never outputs a number.
10. **Prompt injection can only be bounded.** Adaptive attacks break published defenses [33], and judges fall to single tokens [36]. Make the worst case a capped, logged delta, and let meta-instructions raise the target's Paranoia.
11. **Facade leaks are the default.** GPT-4o scores **46%** on TimeChara "future" items [26]. Scope context to the NPC's beliefs, verify claims after generation, and extract speech acts (with sincerity) back into the sim.

---

## Draft claims checked

| Draft claim | Verdict | Evidence / reasoning |
|---|---|---|
| Stratum 2 uses utility AI/GOAP at 10–30 Hz for 1k–5k actors | **Rework** | F.E.A.R.-style GOAP re-plans when goals become invalid, not every tick, and its plans are short [48][49][50]. A* over world states for 5k actors at 30 Hz costs roughly 7–75 core-seconds per second (**Inference**: 50–500 node expansions at about 1 µs each). A utility sweep is affordable (about 7.5 ms for 5k actors × 30 options × 5 considerations), but re-evaluating at 30 Hz is pointless at month granularity. |
| PsychState = ⟨Despair, Hubris, Paranoia, FilialPiety, Corruption⟩ | **Partially accept; restructure** | These are useful *setting-facing* stats, but they mix traits, states and values. Dwarf Fortress separates facets (0–100) from beliefs/values (about −50 to +50), and derives needs and stress from them [56][57]. ALMA separates personality, mood and emotion [63]. |
| Epistemic registry Belief(subject, predicate, confidence) | **Accept and extend** | Add object/value, source agent, evidence type, acquisition time, last reinforcement and distortion lineage, following Talk of the Town [44]. |
| Beliefs are updated only by line of sight | **Reject** | Rumor, testimony, documents and inference are the core of a drama engine [44][45][46]. Raycasting for 5k actors in fast-forward has no geometry to work with; use co-presence witness sets. |
| Emotional decay | **Accept** | Use exponential decay toward a personality baseline. This is standard in ALMA, GAMYGDALA and FAtiMA [61][62][63]. RimWorld adds per-thought duration, stack limits and stacking multipliers [58]. |
| Hive is a zero-latency shared sensory buffer | **Accept as parameterization** | Generalize it as a channel {latency 0, fidelity 1, trust 1}. Implemented as a shared store it is O(1) per event. |
| Mimics keep personality and ties but have utility repointed to sabotage | **Accept with fix** | A pure repoint changes the distribution of a mimic's actions and makes it statistically detectable. Add a *cover-maintenance* term and an exposure-risk term. |
| U_hive = Σbiomass + Σinfiltrated − λΣdrones lost | **Partially accept** | The objective is linear and undiscounted, so it drives greedy infiltration until exposure. Add −μ·P(exposure), discounting, and diminishing returns on infiltration. |
| The masses exist only as aggregates | **Accept** | This matches pop-based strategy games (Victoria 3 pops [R]) and compartment rumor models [45][47]. It needs a deterministic *promotion* (hydration) protocol. |
| Concordia as GM arbitration and associative-memory scaffolding | **Reject for runtime** | Its GM is an LLM that resolves actions, and its memory needs an embedder [3]. No determinism claim; unsupported. Useful as a reference and offline testbed [1]. |
| Smallville score (recency + importance + relevance) for long-term recall | **Reject as-is** | Importance needs an LLM rating per memory [5]. The code decays recency by rank (`0.99^i`) with `gw=[0.5,3,2]` [6][7], while the paper says 0.995 per game-hour [5]. Tested over 2 game days, not centuries. |
| Whisper Counsel text is "injected into the target's cognitive deliberation loop" | **Reject literally; keep the fantasy** | LLM targets are trivially persuadable [37][38], and judges and parsers can be injected [33][36]. Convert the text into typed, bounded modifiers. |
| Pierce the Veil reveals mimics | **Accept as an oracle power** | In-world NPC tests should be imperfect, with known error rates, so that deduction drama exists. |
| Any NPC is talkable via a local small LLM, limited to what it witnessed | **Accept with guards** | Point-in-time leakage is common even in frontier models [26], and persona drift appears within a few turns [29]. Use belief-scoped context, post-hoc verification and speech-act extraction. |

---

## Landscape / state of the art

### 1. Game-AI decision architectures

| Architecture | Exemplars | Cost per decision | Authoring cost | Emergence quality | Fit for 1–5k named actors |
|---|---|---|---|---|---|
| **Utility AI / IAUS** (Dave Mark) | Guild Wars 2: Heart of Thorns (GDC 2015, "Building a Better Centaur") [51] | O(actions × considerations × targets) curve evaluations; vectorizable | Low per behavior: a modular, data-driven design lets designers build an NPC AI "in as little as 7 minutes" (author's claim) [51] | Good reactive variety, weak at multi-step plans | **Primary choice for goal and stratagem selection** |
| **GOAP** | F.E.A.R. (2005) [48]; later Monolith titles such as Shadow of Mordor, and Crystal Dynamics' Tomb Raider [49] | A* search over symbolic state, so the most expensive option | Medium; designers lose direct control and need dedicated debugging and reporting [49] | Novel action sequences, but in practice plans are short [50] | Optional, only for rare multi-step schemes; never per tick |
| **HTN** | Killzone 2/3, Transformers: Fall of Cybertron [50][53]; Horizon Zero Dawn (HTN plus utility, extending the Killzone planner) [52] | Forward decomposition with bounded branching; cheaper and more predictable than GOAP | Medium to high: designers author methods | Authored-looking but controllable | **Primary choice for plan decomposition** ("assassinate rival" → subtasks) |
| **Behavior trees** | Halo 2 [54] and most AAA titles | Very cheap per tick | High for broad repertoires | Low emergence and brittle across 1,000 years of context | Good for execution and animation in the 3D bubble |
| **Sims "advertisements"** | The Sims: objects advertise motive deltas weighted by current needs; selection is random among the top options [55] | O(nearby objects × motives) | Very low, since the content lives in objects | Strong emergence from needs | **Use for places and institutions** (the temple advertises relief from Despair) |
| **CK3 character AI** | Traits carry AI personality weights (boldness, compassion, greed, honor, rationality, sociability, vengefulness, zeal, energy); decisions are weighted and polled at intervals; opinion is a sum of decaying modifiers [59] | Cheap, and polled monthly or yearly per decision | Script-heavy but scalable | Proven drama at roughly a 10k-character scale | **The closest analog** to the 1k–5k named stratum |
| **Dwarf Fortress** | Facets 0–100, beliefs about −50 to +50, needs derived from facets and beliefs, thoughts → stress → breakdowns [56][57] | Cheap scalar updates | Data-heavy, and it pays off | Famously emergent | Model for the core psychology |
| **RimWorld** | A priority think-tree of job givers; mood = sum of thoughts, each with duration, stack limit and stacking multiplier [58]; mental-break thresholds [R] | Cheap | Moderate | High drama with a small cast | Model for "thoughts as timed modifiers", which is also how a whisper should land |

**Lessons.** Jacopin's analytics of F.E.A.R. (GOAP), Killzone 3 and Transformers 3 (HTN) found the planner "is called more often by the game over the years" [50]: shipped planners react to events over small domains. For the time machine, CK3's pattern fits best: per-decision polling in months, plus event interrupts. **Inference (cost):** 5,000 actors × 30 options × 5 considerations is ~750k curve evaluations, about 7.5 ms on one core. That is negligible monthly and affordable at a few Hz near the player.

### 2. Psychological models usable in simulations

| Model | What it gives | Maturity / tooling | Notes for Rehoboam |
|---|---|---|---|
| Big Five / **HEXACO** [64] | 5 or 6 stable traits. HEXACO adds Honesty-Humility. | Strong psychometric base | Low H-H is the natural substrate for Corruption and Hubris. Prefer HEXACO. |
| **Schwartz values** (10 basic, refined to 19) [65] | Motivational priorities arranged in a circle of compatible and conflicting values | Strong cross-cultural base | Filial Piety maps onto Tradition, Conformity-rules and Benevolence-caring. Keep it as a named overlay value. |
| **OCC appraisal** [R] | 22 emotion types from event, agent and object appraisals | Classic and widely implemented | Use a subset of about 8: fear, hope, anger, gratitude, shame, pride, distress, joy. |
| **PAD + ALMA** [63] | Mood as a point in pleasure-arousal-dominance space, a default mood derived from the Big Five, emotions that push mood, and decay | Published model | Despair ≈ persistent low-P, low-D mood. Paranoia's state part ≈ fear plus suspicion of agents. |
| **GAMYGDALA** [62] | Emotion engine designed for games: goals and beliefs with likelihoods → OCC emotions with decay | Research code | A lightweight reference for appraisal. |
| **FAtiMA Toolkit** [61] | Appraisal, emotional decision-making and social-importance dynamics; C#; Apache-2.0 | 74 stars; maintenance unclear | Worth reading the code; do not adopt it as a dependency. |
| **Needs** (The Sims, DF, Humanoid Agents) [55][57][20] | Decaying meters that drive utility | Standard | Humanoid Agents' defaults are fullness, social, health, fun and energy [20]. |
| **CK3 opinion** [59] | Opinion = Σ modifiers, each with its own magnitude and decay or expiry | Shipped at scale | Store a modifier list per relationship edge, not a single scalar. |
| **CiF / Ensemble** [60] | Social exchanges, volition and trigger rules (Prom Week lineage); BSD-4-clause JavaScript | Low activity | A good *rule vocabulary* for social moves; too slow for 5k actors as a rules engine. |
| **ACT-R base-level activation** [66] | B = ln Σ tⱼ^(−d), a power-law forgetting curve | Robust cognitive-science result | A deterministic, cheap memory score. |

**Proposed core (setting-agnostic) plus overlays.** Core per actor: Traits (6 HEXACO, int8) · Values (8–10 Schwartz higher-order or basic values, int8) · Mood (PAD, int16) plus active Emotions (≤8 OCC slots, each with intensity and decay) · Needs (5–7 meters) · Relationships (edge → {opinion modifiers[], trust, familiarity, kinship type}) · Beliefs (sparse store) · Memories (event references with activation).
Overlays are *derived or added* stats defined per setting:

- **Filial Piety** = a value overlay (Tradition plus Benevolence toward kin) and a kin-edge weight.
- **Corruption** = a setting stat accumulated by corrupt acts, gated by low Honesty-Humility.
- **Hubris** = trait (low H-H, high dominance) plus a mood state raised by success streaks.
- **Paranoia** = trait (low Agreeableness, high anxiety) plus state (fear and suspicion beliefs). It also *sets the decision criterion for accusations*.
- **Despair** = a mood-region read-out (P < θ for longer than τ).

The UI shows the five draft vectors; the simulation runs on the core.

### 3. Epistemics

**Belief models.** Ryan et al. (EXAG 2015, "Toward Characters Who Observe, Tell, Misremember, and Lie") gave each character a mental model of other people. Each belief facet carries evidence typed as observation, reflection, statement, eavesdropping, transference (misattributing a feature to someone else), confabulation, lie, mutation (memory corruption) or forgetting, and characters spread knowledge in conversation according to salience [44][R]. This is the closest published template to Rehoboam's facade requirement: it is symbolic, cheap and lies are first-class. The released Talk of the Town repo (MIT) no longer shows those modules on master; master has `mind.py` with a single memory scalar, `personality.py` and others [44]. **Port the paper, not the repo.**

**Rumor and distortion.** Classic rumor transmission shows *leveling* (details drop), *sharpening* (salient details are exaggerated) and *assimilation* (content bends toward the listener's expectations) [45]. False news spreads farther, faster and deeper, attributed in part to novelty [46]. Opinion dynamics with bounded confidence [47] give polarization in aggregate. All three can be implemented as **deterministic transforms on structured propositions**: drop an attribute, scale a magnitude, or substitute an actor consistent with a stereotype, each with a seeded probability.

**Theory of mind.** Symbolic and shallow works. Rehoboam needs depth-1 beliefs ("X believes P") and occasionally depth 2 ("X believes Y suspects me"), stored only for a few dramatic propositions such as mimic suspicion, affairs and plots. LLM theory of mind is brittle under trivial task changes [68], so **do not delegate ToM to the facade**. Bayesian inverse planning [69] is a good model for NPC suspicion: infer hidden type from observed actions under each type's policy.

**Social deduction lessons** [R]: *Among Us* (visual tasks give perfect-specificity clearance; votes cost something), *Werewolf* (information asymmetry), *The Thing* (2002; NPC trust/fear meters, largely scripted infections), *Shadows of Doubt* (simulated citizens, inspectable evidence). Deduction is fun when evidence is discrete and inspectable, tests have known error rates, and false accusations cost something.

**Base rates (Inference, standard Bayes).** Take a test with sensitivity 0.95 and specificity 0.95. At 1% prevalence, PPV = 0.0095 / (0.0095 + 0.0495) ≈ **16%**. At 10% prevalence it is ≈ 68%. In-world purges based on a "good test" therefore mostly kill humans early in an infection, which is a strong, emergent and tragic arc. Paranoia should shift the signal-detection criterion: more accusations, more hits, many more false alarms.

**Hive mind as topology.** Every information flow is an edge type with these parameters:

| Channel | Latency | Fidelity (per hop) | Trust weight | Reach | Notes |
|---|---|---|---|---|---|
| Witnessing (co-presence) | 0 | High, dropping with distance and stress | Self ≈ 1 | Place × time | Raycasts only in the hydrated bubble |
| Testimony / gossip | Hours to days | 0.6–0.9, plus distortion operators | Relationship trust | Social-graph neighbors | Driven by named actors' "tell" actions |
| Documents / decrees | Days to weeks | ≈1 for content, low for interpretation | Institutional legitimacy | Literate subset | Forgery is a lie on an artifact |
| Sermon / market crowd | Days | Low | Faction affinity | Pop aggregates | Compartment flows |
| Oracle whisper | 0 | 1 for content, gated by parsing | Trust in the "inner voice" | One target | Section 4.4 |
| **Hive** | **0** | **1** | **1** | **All drones and mimics** | One shared store; no per-member copies |

Mimics belong to two channels: they hold the hive store plus a personal "cover" store. If a decision draws on hive-only knowledge, it emits an **impossible-knowledge** event that observers with depth-1 ToM can flag ("how did she know the granary burned?").

### 4. LLM agent societies, 2023–2026

| System | Date / version | License | Scale | What it taught | Fit |
|---|---|---|---|---|---|
| Generative Agents (Smallville) [5][6] | 2023 | Apache-2.0; research code | 25 agents, 2 game days | "Thousands of dollars" for the study [8]. Memory stream, reflection (importance-sum trigger of 150 [6]) and planning yield believable days. | Ideas only |
| Affordable Generative Agents [8] | TMLR 2024 | Apache-2.0 | Smallville | Learned "lifestyle policies" replace repeated LLM calls; social memory compresses dialogue | Supports the hybrid design |
| Lyfe Agents [19] | Oct 2023 | n/a | Small groups | Option-action hierarchy, asynchronous self-monitoring, Summarize-and-Forget memory; **10–100× cheaper** | Supports the hybrid design |
| Humanoid Agents [20] | EMNLP Demo 2023 | Apache-2.0 | Small | Adds System-1 needs, emotion and closeness to generative agents | Confirms "needs and emotions outside the LLM" |
| 1,000 People (Park et al.) [9][10][11] | Nov 2024 | Code MIT; **interview data restricted** | 1,052 people | Normalized GSS accuracy **0.85** (68.85% / 81.25% self-consistency), driven by **2-hour interviews** [9][10] | Lesson: fidelity comes from rich per-person grounding. Rehoboam's equivalent is event history, not prompts. |
| Project Sid / PIANO (Altera) [12][13] | Oct 2024 | **No code; no license** | 10 to 1,000+ | Concurrent modules with a coherence controller; reported roles, taxes and religion spread. The authors call the results "preliminary". | Unverifiable; treat claims as anecdotes |
| AgentSociety [14][15] | Feb 2025, then v2 | Apache-2.0 **except a `commercial/` folder** | 10k+ agents, ~5M interactions. A later version reports 30k agents faster than wall-clock on 24 A800 GPUs. | Ray-distributed. v2 makes agents stateless Ray tasks with JSONL/DuckDB *replay logs*. | Its log-replay pattern is relevant; its compute is not |
| OASIS (CAMEL-AI) [16][17] | Nov 2024; active (camel-ai 0.2.90 update dated Aug 2026) | Apache-2.0 | Up to 1M agents | 23 actions plus a recommender. Reference run: 100 agents × 1 step = 335.6k input / 16.75k output tokens. Supports an activation probability per step. | Masses analog: most agents are inactive most steps |
| Concordia [1][2][3][4] | v2.0 (Jul 2025, entity-component, GM as entity, checkpointing) → v2.4.0 (2026-03-06: async engine, simulation server) | Apache-2.0; "not an officially supported Google product" | Tens of agents | LLM GM arbitration. Prefabs, PuppetAct and a dramaturgic GM (v2.3). **No release in about 7 months.** | Reference and offline testbed |
| AI Town [18] | Ongoing | MIT | Tens | Convex backend; Ollama `llama3` by default | Prototype UI only |
| YuLan-OneSim [21] | May 2025 | Apache-2.0 | Claims 100k; no benchmarks | Last dated news June 2025 | Weak signal |
| AgentTorch / LLM archetypes [22][23] | v0.6.0 | **AGPL-3.0** (commercial licence on request) | Millions, GPU-tensorized | Query an LLM once per *archetype*, not per agent | **Good idea for the masses; incompatible licence** |

**Cross-cutting lessons.**

- **Cost.** Smallville cost thousands of dollars for 50 agent-days [5][8]. **Inference:** at ~3.4k input tokens per agent-step [17], 5k actors stepped monthly for 1,000 years is 6×10⁷ agent-steps, ~2×10¹¹ tokens, or about eight months at an optimistic 10k tokens/s.
- **Drift.** Persona drift is measurable within about 8 dialogue rounds [29].
- **Determinism.** No surveyed system claims it. AgentSociety v2's log replay [15] is the right idea. Temperature 0 is not enough: batching gave 18 distinct completions from 1,000 identical prompts, and 1 with batch-invariant kernels [24][25]. **Log every LLM output that touches state; never re-query on replay.**

**Hybrid pattern (recommended).** The simulation enumerates legal actions, each with an ID, precondition-checked and with a utility estimate. The LLM receives a scoped context plus that menu and returns `{action_id, rationale}` under a JSON grammar. The simulation validates the answer and falls back to the utility argmax if it is invalid. The tuple (model, weights hash, prompt hash, output) is written to the event log as an input event. This is the "action-selector" pattern from prompt-injection design-pattern work [32], and a weaker relative of CaMeL's control/data-flow separation [31].

### 5. Persuasion and argumentation as a mechanic

**Computational models** [R]:
- **Elaboration Likelihood Model** [40]: a central route (argument quality matters) when motivation and ability are high, and a peripheral route (source cues, emotion, repetition) otherwise.
- **Dung argumentation frameworks** [41] (attack graphs with grounded and preferred semantics) and **value-based argumentation frameworks** [42]. In VAFs, whether an argument is accepted depends on the *audience's value ordering*, so the same counsel moves a filial son and not a hubristic general.
- **Bayesian persuasion** [43]: a sender commits to a signal structure, and persuasive power is bounded by the receiver's model of that signal. This is the right model for Omens: overuse destroys their credibility.
- **Reactance** (Brehm) [R]: pressure on autonomy backfires.

**Games** [R]:

| Game | Mechanic | Lesson |
|---|---|---|
| Disco Elysium [71] | Skill-voiced inner monologue; visible white/red checks; white checks can be retried after the skill changes | Whisper Counsel *is* a Disco Elysium inner voice. Show the check. |
| Griftlands [72] | Negotiation as a card battle against "resolve" | Typed appeals with explicit numbers are satisfying |
| Pentiment [73] | Persuasion shaped by backstory and earlier choices; delayed consequences | Value-congruent appeals; consequences that land later |
| King of Dragon Pass [74] | Advisors with personality biases give counsel that can be wrong | The inverse of the Oracle mode; counsel quality depends on who is speaking |
| Suck Up! [75], 1001 Nights, Dead Meat | LLM-judged free-text persuasion or interrogation | Fun, but players quickly learn to game the LLM |

**LLM-as-judge biases.** Judges show position, verbosity and self-enhancement bias [34], and one study catalogues 12 bias types including authority and bandwagon [35]. Single "master-key" tokens such as `:` or "Thought process:" can flip LLM reward models and judges [36]. LLMs are also over-persuadable *as targets*: multi-turn persuasion flips correct beliefs [37], and a 40-technique persuasion taxonomy jailbreaks frontier models at high rates [38]. LLMs are themselves strong persuaders of humans [39]. **Implication:** neither the target's mind nor the quality score may be an LLM's free judgment.

**Prompt-injection state of the art.** Researchers have proposed structural defenses:
- Six design patterns [32]: action-selector, plan-then-execute, map-reduce, dual LLM, code-then-execute and context-minimization.
- CaMeL [31]: privileged and quarantined LLMs plus capabilities. Its repo is a "research artifact" with no maintenance plan.

Adaptive attacks have bypassed a dozen recent defenses at high success rates [33]. The only robust stance is **bounded blast radius**.

### 6. Grounded NPC dialogue

| Benchmark | Date | License | Finding |
|---|---|---|---|
| TimeChara [26] | ACL Findings 2024 | Code MIT; **dataset non-commercial** | Point-in-time role-play (Harry Potter), 10,895 test items. Averages: GPT-4o 64.5%, o1 81.8%. GPT-4o scores **46.0% on "future"** items against 90% on past-presence items: characters leak knowledge they should not yet have. Proposed fix: "Narrative-Experts" decomposition. |
| RoleBench / RoleLLM [27] | 2023 | **Not stated** | 100 roles and 168k samples; role-conditioned tuning lets a LLaMA model approach GPT-4 role-play |
| CharacterEval [28] | 2024 | MIT | Chinese; 77 characters and 1,785 dialogues; 13 metrics |
| Persona drift [29] | 2024 | n/a | Instruction and persona stability decays within about 8 rounds |

**Techniques.**

1. **Context scoping.** The prompt contains only the NPC's persona card, its belief records above an activation threshold for the topic, relationship summaries and emotional state. *World truth never enters the prompt.*
2. **Post-hoc verification.** Decompose the reply into atomic claims, as in FActScore [30], and check each against the speaker's belief store. Unsupported claims either become hedges ("I've heard…, but I don't know") or are permitted as **lies**, but only if the sim's deception utility chose to lie.
3. **Speech-act extraction.** Under grammar-constrained decoding, classify the reply into Searle-style acts with a typed payload: INFORM, ASK, ACCUSE, THREATEN, PROMISE, REQUEST, CONFESS. The payload is (subject, predicate, object, evidence references). Sincerity = whether the content matches the speaker's beliefs. Listeners update through the testimony channel. PROMISE and THREATEN create commitments only if the sim validates them as legal and consistent with the speaker's utility.

Mantella, a Skyrim/Fallout LLM-dialogue mod (AGPL-3.0), shows that hobbyist local-LLM NPCs ship, but without this rigor [67].

---

## Recommendations for Rehoboam

### 4.1 Three-tier mind architecture

| Tier | Population | State | Decision | Cadence | LLM |
|---|---|---|---|---|---|
| **T0 Masses** | 100k–1M as ~10³–10⁴ **pops** (place × culture × class × faith) | Distributions (mean and variance or small histograms) of needs satisfaction, morale, fear, faction loyalty, and **belief shares** per tracked proposition | Difference equations: needs → unrest; compartment rumor flows (ignorant → spreader → stifler); bounded-confidence drift between pops | Monthly, plus event interrupts | None at runtime. Optionally calibrate response curves offline by querying an LLM once per archetype (the AgentTorch idea, reimplemented to avoid AGPL) |
| **T1 Named actors** | 1k–5k (hive drones, mostly as T0-like squads) | The core from §2, about 1–4 KB per actor plus beliefs | **Event-driven IAUS** chooses a goal or stratagem; **HTN** decomposes it; Sims-style advertisements from places and institutions | Time machine: decision points at events plus CK3-style polling (1–12 months per decision type). Street level: 1–5 Hz re-evaluation within the bubble; BT execution at frame rate | None by default |
| **T2 Hydrated / conversing** | 1–20 at once | T1 state plus a dialogue buffer | T1 decides; the **LLM voices** the decision and proposes speech acts that the sim validates | Real time | Local small model; outputs logged if they produce events |
| **T2+ Pivotal consult** (optional) | Top ~50 actors *in focus* | n/a | LLM picks from a sim-generated legal menu; validated; logged | Rare (succession, betrayal, war) | Off during fast-forward |

**Promotion.** When the camera or the plot needs an individual from a pop, sample their traits from the pop's distributions using a counter-based RNG keyed by (world seed, pop ID, index). Seed beliefs from the pop's belief shares, and synthesize a plausible event history from the pop's recent events. **Demotion** folds the actor's deltas back into the pop's moments.

### 4.2 Epistemics model

- **Belief record:** `{subj, pred, obj/value, conf (int8 log-odds), evidence_type, source_agent, channel, t_acquired, t_last, event_ref, lineage_ptr}`.
- **Update rule:** add log-odds weighted by `trust(source) × fidelity(channel) × skepticism(Paranoia, in-group/out-group)`. Contradictions trigger cognitive-dissonance appraisal (anger or shame).
- **Memory:** an event graph is the ground truth. Each actor holds references, scored by ACT-R activation [66] plus emotional intensity at encoding. Below threshold, a memory becomes fuzzy and eligible for *mutation* or *transference* [44]. Every N years, roll episodes up into semantic beliefs (Lyfe's Summarize-and-Forget, done symbolically [19]). For the facade, retrieve by structured keys (entity, place, topic, time) first and use embeddings only as an optional re-ranker. That keeps retrieval deterministic.
- **Channels:** use the topology table in §3. Hive = {0, 1, 1, all}.
- **ToM:** store nested beliefs only for flagged propositions. Suspicion is a Bayesian posterior over each actor's type (human or mimic), updated by likelihoods of observed actions under each type's policy [69].

### 4.3 Hive, mimics and detection

- **U_hive** = Σ w_b·biomass + Σ w_i·g(infiltrated) − λ·Σ drones lost − μ·E[exposure cost], discounted at γ, where g is concave. Mimic utility = cover-maintenance (act as the old self) + covert objectives, gated by local risk (witness density).
- **Tells**, each with its own sensitivity and specificity:
  - impossible-knowledge events;
  - behavioral drift, measured as the KL divergence between a mimic's action distribution and its pre-capture baseline;
  - physical tests (authorable, imperfect).
- NPC investigators run the Bayes update. Paranoia lowers their accusation criterion.
- **Pierce the Veil** is a perfect oracle *for the player only*. Converting that knowledge into NPC action requires evidence-backed Whisper Counsel, which closes the loop.

### 4.4 Whisper Counsel pipeline

1. **Pre-filter (deterministic):** enforce a length cap (about 500 characters), strip markup, and normalize text.
2. **Parse (LLM, temperature 0, JSON grammar, logged):** player text goes in as *quoted data*. The output is an `ArgumentFrame {appeals[type ∈ {evidence, value(v), fear, hope, authority, kinship, threat, promise, social-proof}], target_vectors[], requested_action?, claims[(s,p,o)], references[entity/event names]}`. A second, independent classifier flags *meta-instructions* addressed to the system or game. If the two passes disagree, take the weaker reading.
3. **Ground:** resolve references against **the player's knowledge base**, meaning what the Oracle has actually seen. Each claim becomes *verified*, *known-false* (a lie) or *unknown* (a bluff). Then check it against the **target's** beliefs: consistent, contradicting or novel.
4. **Score (no LLM):**
   - P(central route) = σ(a·Openness + b·Involvement − c·Despair − d·Stress).
   - Central effect = evidence strength × VAF congruence (the target's rank for the value invoked) [42].
   - Peripheral effect = trust in the inner voice × emotional fit with the current OCC state.
   - Raw Δ = route-weighted sum.
   - Multiply by habituation 1/(1 + k·n_recent_same_vector).
   - Cap at c_max·(1 − e^(−Foresight/F₀)).
   - Apply **resistance** r ~ Beta(α, β), with α and β set by Hubris, Agreeableness and Paranoia and drawn from the deterministic RNG stream (seed, target, whisper#).
   - **Reactance flip**: if the requested action violates top values or the target's autonomy needs, Δ changes sign.
5. **Apply:** add a RimWorld-style **Thought** (vector modifier with a half-life, stack limit and stacking multiplier) [58]. Verified claims become beliefs with source = "inner voice". A caught lie (the target already holds a contradicting belief with high confidence) damages trust in the voice.
6. **Show the parse to the player**, Disco Elysium-style: "Heard as an appeal to Filial Piety, citing the granary fire. She already doubted the fire." Transparency turns guessing into play and shows the player that injection achieves nothing.

### 4.5 Omens

Pops are Bayesian receivers with a per-pop *omen credibility* [43]. Omens shift morale through compartment flows. Credibility decays with frequency and with failed portents, and priests (T1 actors) *interpret* omens according to their faction's utility.

### 4.6 Prompt-injection defense checklist

- The LLM never emits numbers, verdicts or state. It emits *labels* from a closed schema.
- Player text is never concatenated with the instructions that define outcomes. Use the dual-LLM / action-selector separation [32]: the parser model has no tools and no state.
- Effects are bounded per whisper and per target-season (caps and habituation), so the worst-case attack equals a perfect argument, which is already capped.
- Make the response diegetic: detected meta-instructions add Paranoia and an "uncanny voice" memory, so attacks are penalized in-world.
- Use length-normalized, categorical scoring, which avoids verbosity bias in judges [34][35].
- Run a red-team corpus in CI: injection strings, master-key tokens [36] and persuasion-taxonomy prompts [38]. Track the frame distribution after model upgrades.
- Log the ArgumentFrame as the canonical input, so replay never touches the LLM.

### 4.7 Determinism

- Fixed-point psychological variables.
- Stable ID-ordered iteration.
- Counter-based RNG streams keyed by (seed, entity, tick, purpose).
- No dependence on hash-map iteration order.
- All external inputs logged: player actions, ArgumentFrames, LLM choices, extracted speech acts.
- Flavor-only dialogue does not need to be logged for determinism; anything that becomes an event does.

### 4.8 What to borrow and what not to

- **Borrow:** CK3 opinions and cadence, DF's psych split, RimWorld Thoughts, Sims advertisements, Talk of the Town evidence, Concordia's entity-component pattern, Lyfe/AGA "policies replace calls", OASIS activation probability.
- **Do not borrow:** Concordia as runtime, LLM-scored importance, LLM-as-judge for persuasion strength, per-tick GOAP.

---

## Risks & unknowns

- **Small-model quality.** Parsing accuracy and knowledge-boundary fidelity of 1–8B local models are unmeasured for this schema. Build an eval set early (TimeChara-style point-in-time probes over Rehoboam's own event graph), keeping in mind that the TimeChara dataset itself is non-commercial [26].
- **Opacity versus exploitability.** Fully transparent persuasion invites min-maxing; opaque persuasion feels arbitrary. Needs playtesting.
- **Residual injection risk.** Any LLM parser can be steered into choosing the *most favorable legal label*. The caps make this bounded but not zero [33].
- **Blandness.** Deterministic sims often generate uneventful history. The narrative compiler will probably need story sifting (Ryan; Kreminski et al., not re-verified here) to *find* drama rather than force it.
- **Calibration.** Psych parameters, decay constants and channel fidelities have no ground truth. Plan sensitivity sweeps across 1,000-year runs; watch for value-distribution collapse and runaway polarization.
- **Representation.** Filial Piety and similar culture-coded values risk caricature, and LLMs flatten identity groups [R]. Ground the facade voice in actor state, not in a stereotype prompt.
- **Licensing.**
  - AgentTorch [22] and Mantella [67] are AGPL-3.0.
  - The TimeChara dataset is non-commercial [26].
  - RoleBench has no stated license [27].
  - Project Sid has no code or license [13].
  - AgentSociety excludes a `commercial/` folder from Apache-2.0 [15].
  - CaMeL is unmaintained [31].
- **Project health.** Concordia has had no release since March 2026 [2]; YuLan-OneSim's last dated news is June 2025 [21]; the Talk of the Town repo lacks the published knowledge system [44].
- **Verification gap.** Many [R] items were not re-fetched. Re-verify the numbers in [33]–[39] and the CK3 and RimWorld internals before citing them externally.

---

## Sources

[V] = checked this session; [R] = recalled, not re-fetched.

1. https://github.com/google-deepmind/concordia/releases [V]: Concordia release notes from v2.0.0 (Jul 2025, entity-component, GM as entity) to v2.4.0 (2026-03-06, async engine).
2. https://pypi.org/project/gdm-concordia/ [V]: confirms 2.4.0 (Mar 6, 2026) is the latest release.
3. https://github.com/google-deepmind/concordia [V]: Apache-2.0; LLM GM resolves actions; "not an officially supported Google product"; tech report arXiv:2312.03664.
4. https://arxiv.org/abs/2507.08892 [R]: Concordia 2025 design paper ("Multi-Actor Generative AI as a Game Engine"); ID taken from the README.
5. https://arxiv.org/abs/2304.03442 [R, partly V]: Generative Agents (Park et al. 2023); recency 0.995 per game-hour, importance 1–10 (search snippet).
6. https://github.com/joonspk-research/generative_agents/blob/main/reverie/backend_server/persona/memory_structures/scratch.py [V]: `recency_decay=0.99`, equal weights, `importance_trigger_max=150`, `vision_r=4`.
7. https://github.com/joonspk-research/generative_agents/blob/main/reverie/backend_server/persona/cognitive_modules/retrieve.py [V]: `gw=[0.5,3,2]`, min-max normalization, recency decayed by rank.
8. https://arxiv.org/abs/2402.02053 [V]: Affordable Generative Agents (TMLR 2024); states the Stanford Town study cost "thousands of dollars".
9. https://arxiv.org/abs/2411.10109 [V]: Generative Agent Simulations of 1,000 People; 1,052 participants, normalized accuracy 0.85.
10. https://hai.stanford.edu/policy/simulating-human-behavior-with-ai-agents [V]: Stanford HAI brief; two-hour interviews.
11. https://github.com/joonspk-research/genagents [V]: MIT code; interview agent bank available only through restricted access.
12. https://arxiv.org/abs/2411.00114 [V]: Project Sid / PIANO paper.
13. https://github.com/altera-al/project-sid [V]: PDF and video only; no code, no license; results called "preliminary".
14. https://arxiv.org/abs/2502.08691 [V]: AgentSociety; 10k+ agents, ~5M interactions; later 30k agents on 24 A800s.
15. https://github.com/tsinghua-fib-lab/agentsociety [V]: Apache-2.0 except `commercial/`; v2 stateless Ray tasks, JSONL/DuckDB replay.
16. https://arxiv.org/abs/2411.11581 [V]: OASIS; up to 1M agents; herd effects and polarization.
17. https://github.com/camel-ai/oasis [V]: Apache-2.0; 23 actions; 100 agents × 1 step = 335.6k input / 16.75k output tokens.
18. https://github.com/a16z-infra/ai-town [V]: MIT; Convex backend; Ollama/llama3 default.
19. https://arxiv.org/abs/2310.02172 [V]: Lyfe Agents; option-action, Summarize-and-Forget, 10–100× cheaper.
20. https://aclanthology.org/2023.emnlp-demo.15 [V]: Humanoid Agents (needs, emotion, closeness); repo is Apache-2.0.
21. https://github.com/RUC-GSAI/YuLan-OneSim [V]: claims 100k agents with no benchmarks; last news June 2025.
22. https://github.com/AgentTorch/AgentTorch [V]: Large Population Models; AGPL-3.0; v0.6.0.
23. https://arxiv.org/abs/2409.10568 [R]: Chopra et al., "On the limits of agency in agent-based models" (LLM archetypes).
24. https://github.com/thinking-machines-lab/batch_invariant_ops [V]: 18 vs. 1 unique completions out of 1,000 at temperature 0.
25. https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/ [R]: batch invariance as the root cause of nondeterminism.
26. https://github.com/ahnjaewoo/timechara [V]: TimeChara leaderboard (GPT-4o future 46.0%); dataset non-commercial.
27. https://github.com/InteractiveNLP-Team/RoleLLM-public [V]: RoleBench (100 roles, 168k samples); no license stated.
28. https://github.com/morecry/CharacterEval [V]: Chinese role-play benchmark; MIT.
29. https://arxiv.org/abs/2402.10962 [R]: Li et al. 2024; persona/instruction drift in dialogs.
30. https://arxiv.org/abs/2305.14251 [R]: FActScore; atomic-claim verification.
31. https://github.com/google-research/camel-prompt-injection [V]: CaMeL (arXiv:2503.18813); research artifact, no maintenance planned.
32. https://arxiv.org/abs/2506.08837 [R]: Beurer-Kellner et al. 2025; six prompt-injection design patterns.
33. https://arxiv.org/abs/2510.09023 [R]: Nasr et al. 2025, "The Attacker Moves Second"; adaptive attacks break defenses.
34. https://arxiv.org/abs/2306.05685 [R]: Zheng et al. 2023; LLM-as-judge position, verbosity and self-enhancement bias.
35. https://arxiv.org/abs/2410.02736 [R]: Ye et al., "Justice or Prejudice?"; 12 judge biases.
36. https://arxiv.org/abs/2507.08794 [R]: "One Token to Fool LLM-as-a-Judge"; master-key tokens.
37. https://arxiv.org/abs/2312.09085 [R]: "The Earth is Flat because…"; LLM beliefs flipped by persuasion.
38. https://arxiv.org/abs/2401.06373 [R]: Zeng et al. 2024; persuasion taxonomy used for jailbreaks.
39. https://arxiv.org/abs/2403.14380 [R]: Salvi et al.; LLM persuasiveness in debate.
40. https://doi.org/10.1016/S0065-2601(08)60214-2 [R]: Petty & Cacioppo 1986, Elaboration Likelihood Model.
41. https://doi.org/10.1016/0004-3702(94)00041-X [R]: Dung 1995, abstract argumentation frameworks.
42. https://doi.org/10.1093/logcom/13.3.429 [R]: Bench-Capon 2003, value-based argumentation frameworks.
43. https://doi.org/10.1257/aer.101.6.2590 [R]: Kamenica & Gentzkow 2011, Bayesian persuasion.
44. https://github.com/james-owen-ryan/talktown [V repo / R paper]: Talk of the Town (MIT). The knowledge system is described in Ryan et al., EXAG 2015, and is absent from master.
45. https://doi.org/10.1093/poq/10.4.501 [R]: Allport & Postman, rumor leveling, sharpening and assimilation.
46. https://doi.org/10.1126/science.aap9559 [R]: Vosoughi, Roy & Aral 2018; false news spreads faster.
47. https://www.jasss.org/5/3/2.html [R]: Hegselmann & Krause 2002, bounded-confidence opinion dynamics.
48. https://alumni.media.mit.edu/~jorkin/gdc2006_orkin_jeff_fear.pdf [R]: Orkin, "Three States and a Plan" (F.E.A.R. GOAP).
49. https://gdcvault.com/play/1022019/Goal-0riented-Action-Planning-Ten [V]: GDC 2015 "GOAP: Ten Years Old and No Fear!" (Shadow of Mordor, Tomb Raider).
50. https://ojs.aaai.org/index.php/AIIDE/article/view/12728 [V]: Jacopin 2014; planning analytics for F.E.A.R., Killzone 3 and Transformers 3.
51. https://gameai.com/iaus.php [V]: Dave Mark's IAUS; GDC 2015 "Building a Better Centaur".
52. https://www.guerrilla-games.com/read/the-ai-of-horizon-zero-dawn [V]: Horizon Zero Dawn HTN plus utility, extending Killzone's planner.
53. http://www.gameaipro.com/GameAIPro/GameAIPro_Chapter12_Exploring_HTN_Planners_through_Example.pdf [R]: Humphreys, HTN planners (Transformers).
54. https://www.gamedeveloper.com/programming/gdc-2005-proceeding-handling-complexity-in-the-i-halo-2-i-ai [R]: Isla, Halo 2 behavior trees.
55. https://gameindustrylibrary.com/documents/gmtk-the-genius-ai-behind-the-sims [V]: Sims advertisements weighted by motives.
56. https://dwarffortresswiki.org/index.php/Facet [V]: DF facets on a 0–100 scale.
57. https://dwarffortresswiki.org/Thoughts_and_preferences [V]: DF beliefs, needs, thoughts and stress.
58. https://rimworldwiki.com/wiki/Thoughts/Memory_Misc_(Ideology) [V]: RimWorld thoughts with duration, stack limit and stacking multiplier.
59. https://ck3.paradoxwikis.com/Traits_modding [R]: CK3 AI personality values and opinion modding.
60. https://github.com/ensemble-engine/ensemble [V]: Ensemble (CiF successor); BSD-4-clause.
61. https://github.com/GAIPS-INESC-ID/FAtiMA-Toolkit [V]: FAtiMA Toolkit (C#, Apache-2.0); maintenance unclear.
62. https://doi.org/10.1109/TAFFC.2013.24 [R]: GAMYGDALA, an emotion engine for games.
63. https://doi.org/10.1145/1082473.1082478 [R]: Gebhard 2005, ALMA layered affect model.
64. https://doi.org/10.1177/1088868306294907 [R]: Ashton & Lee 2007, HEXACO.
65. https://doi.org/10.1037/a0029393 [R]: Schwartz et al. 2012, refined theory of 19 values.
66. https://doi.org/10.1111/j.1467-9280.1991.tb00174.x [R]: Anderson & Schooler 1991, memory activation and power-law forgetting.
67. https://github.com/art-from-the-machine/Mantella [V]: LLM NPC dialogue mod for Skyrim and Fallout 4; AGPL-3.0.
68. https://arxiv.org/abs/2302.08399 [R]: Ullman 2023; LLM theory of mind fails on trivial alterations.
69. https://doi.org/10.1038/s41562-017-0064 [R]: Baker et al. 2017, Bayesian theory of mind.
70. (reserved; story sifting, Ryan and Kreminski et al., not re-verified)
71. https://store.steampowered.com/app/632470/ [R]: Disco Elysium.
72. https://store.steampowered.com/app/601840/ [R]: Griftlands.
73. https://store.steampowered.com/app/1205520/ [R]: Pentiment.
74. https://store.steampowered.com/app/352310/ [R]: King of Dragon Pass.
75. https://www.suckupgame.com/ [R]: Suck Up! (LLM persuasion game).
