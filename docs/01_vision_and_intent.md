# 01 — Vision, Intent and Definition of Success

*Status: Draft v1.0 (2026-10-15). Owner: project lead. Edited by: systems design.*
*Inputs: the four attached draft documents (Product Vision, Definitive System Design v2.0, Open-Source Bootstrap, rehoboam_2), your answers to the intake questions, and research dossiers R01–R11.*

---

## 1. Your intent, restated

This section restates the intent so that you can correct it. Every later document depends on it.

> **Rehoboam is a personal world-engine.** It runs a civilisation from founding through centuries of history. History comes from deterministic simulation, not from a language model making things up. You can watch it as history: fast-forward, rewind, stop at the moments that matter and ask *why*. You can turn it into stories. Over time you can also step inside it: as an ambient painting on your second monitor, as a walker in its streets, and as a mortal oracle nudging its rulers.
>
> It is **a creative engine and a world you inhabit and become invested in.** You keep improving it, raising its fidelity and complexity year after year. You build it **with AI coding agents** doing most of the implementation. **Compute is not the constraint**: start at low fidelity and be able to scale to any amount of compute later. The first thing that must work is **the Time-Machine with its Chronicle**.

These are the elements of your drafts I treat as the **core of the vision**. They are not up for debate unless you say otherwise:

| # | Core element | Where it came from |
|---|---|---|
| V1 | Drama is born from **unforgiving, deterministic systems**: resources, logistics, contagion, death, succession. | Vision §1.1, Design §1.3 |
| V2 | LLMs are used **only where they excel**: interpretation (compiler), facade (dialogue), synthesis (narration). They never decide physics or bookkeeping. | Vision §4, Design §1.3 |
| V3 | **Asymmetric minds.** Fragmented, paranoid, grieving humans collide with a zero-friction collective intelligence, the Hive. | Vision §1.1 |
| V4 | **Scale invariance.** One world can be seen from orbit across 1,000 years or from a cobblestone corner in real time. | Vision §1, Design §1.2 |
| V5 | **Infinite worlds from prompts.** One engine, configured by a declarative world manifest. | Vision §3 |
| V6 | **Five relationships with the world:** Diorama, Time-Machine, Inhabitant, Oracle, Narrative Compiler. | Vision §2 |
| V7 | **Influence, not control.** The player is a mortal catalyst with a finite resource (Foresight), not an RTS god. | Vision §2 (Modality 4) |

## 2. What "success" means

Your answer, verbatim: *"It's a creative engine and a world I want to inhabit and be invested in that I can improve and continue to increase fidelity and complexity."*

That sentence contains four testable claims. The whole plan is organised around them.

| ID | Success claim | What would prove it | What would falsify it |
|---|---|---|---|
| **S1** | **Creative engine.** It produces history and stories you actually want to read. | Over successive runs you consistently rate sifted events and chronicles as "I would read this" (see MOE-1, MOE-2). | Runs read like logs. Every seed tells the same story. Prose hides the fact that the simulation is shallow. |
| **S2** | **Inhabitable.** The world feels like a place with people in it, not a spreadsheet. | Any person you look at has a coherent identity, kin, history and errand. Places persist and change believably over centuries (MOE-3). | People pop in and out of existence. Characters forget what they told you. Towns re-roll between visits. |
| **S3** | **Investment.** You care what happens to *your* world and can keep it. | Worlds persist indefinitely. History is never silently lost. You can branch "what-ifs", replay any moment, and revisit people across centuries (MOE-4). | Saves break on upgrade. Compaction deletes the backstory you cared about. A replay produces a different past. |
| **S4** | **Evolvable.** Fidelity and complexity can rise continuously without rewrites. | New mechanics and higher-fidelity models can be added by AI agents as isolated, tested modules, and old worlds remain loadable (MOE-5). | Every new feature needs a cross-cutting refactor. The engine fossilises at its first fidelity level. |

Commercial success, public reception and store policy were **not** chosen as the bar. They are recorded as constraints to keep open (§6), not as goals. The product-landscape dossier (R11) still matters, because it shows which design choices would close that door later.

## 3. Design drivers derived from success

These drivers rank and break ties for every decision in the architecture and the ADRs.

1. **D1 — Truth before beauty.** The simulation is the only source of truth. Everything shown, whether prose, 3D or dialogue, is a *view* over it and carries provenance back to it. This serves S1, S2 and S3.
2. **D2 — Story yield is the north-star metric.** A deterministic simulation that is boring has failed, however fast it is. The sifter and chronicle are built *with* the first simulation, not after it. This serves S1.
3. **D3 — Never lose history, never lie about it.** Event sourcing, deterministic replay, keyframes, branching timelines and versioned saves all follow. Nothing is deleted on a rule fixed at creation time. This serves S3.
4. **D4 — Observer invariance.** Looking at the world, by hydrating a street or opening an inspector, must not change history. Only *material actions*, which are logged, may. This serves S2 and S3.
5. **D5 — Fidelity is a dial, not a rewrite.** Every system is a pluggable model with explicit fidelity levels and aggregate-consistency contracts. More compute buys more fidelity through configuration. This serves S4.
6. **D6 — Agent-buildable by construction.** Small modules, typed contracts, golden-run tests and spec-first work packages, because AI agents are the workforce. This serves S4.
7. **D7 — Scale out with ensembles, not distribution.** One authoritative process per timeline. Unbounded compute goes into *more timelines* (Monte-Carlo foresight, calibration sweeps, what-if branches), more model fidelity and bigger LLMs, never into a sharded world. R10 records why the SpatialOS approach failed. This serves S4.

## 4. Measures of effectiveness (MOE) and performance (MOP)

**MOEs** measure whether the success claims hold. **MOPs** measure engineering progress toward them. Targets marked "P1/P2/…" are phase gates; see `11_roadmap.md`.

### Measures of effectiveness

| ID | Measure | Method | Target |
|---|---|---|---|
| MOE-1 | **Story yield.** Auto-paused events per simulated century that you rate ≥ 4/5 on "I want to know what happens next". | Blind rating sessions over random seeds, using the rating tool built into the console. | P2: ≥ 3 per century, median over 10 seeds. P3: ≥ 6. |
| MOE-2 | **Chronicle quality.** Pairwise preference of the chronicle against a random-baseline chronicle of the same run. | Blind A/B, as in StU [R02]. | ≥ 80% preference. |
| MOE-3 | **Inhabitation coherence.** The rate of observed contradictions in sampled people and places: identity, kin, location, memory. | Automated probes plus manual "interrogate 20 random people" sessions. | 0 hard contradictions. Fewer than 1 soft inconsistency per 20 probes. |
| MOE-4 | **Continuity.** Fraction of saved worlds that load and replay bit-identically after an engine upgrade, or migrate with a declared, logged divergence. | Golden-world corpus in CI. | 100% load. 100% either bit-identical or migrated by an explicit migration. |
| MOE-5 | **Evolvability.** Agent lead time to add a new registry mechanic (e.g., "drought-resistant crop") or raise one system's fidelity level, measured from spec to merged with all gates green. | Track work-package cycle times. | A median of 1–3 agent-days for a mechanic. No change touches more than one system crate plus the registry. |
| MOE-6 | **Bad-run rate.** Fraction of seeds with fewer than N completed story arcs per century (R02 §R4). | Ensemble runs, 32 seeds per manifest. | Below 10%. |

### Measures of performance

| ID | Measure | Target |
|---|---|---|
| MOP-1 | Headless macro run time for 1,000 years at 100k people, monthly resolution (8-core desktop) | P1: ≤ 2 min. P3: ≤ 10 min at 1M people. |
| MOP-2 | Determinism: state hash identical across thread counts and repeated runs (same binary and platform) | 100%, checked every sim-month in CI. |
| MOP-3 | Timeline scrub latency, chronicle and aggregates (no re-simulation) | < 100 ms. |
| MOP-4 | "Drop in" seek latency (keyframe plus replay) | ≤ 10 s at 100k people with a ±10-year cache. |
| MOP-5 | Stylized-fact validation bands passed (R09 §M9) | ≥ 90% of bands across a 32-seed ensemble. |
| MOP-6 | Canon contradictions in generated prose | 0 per 1,000 words for T0/T1 claims (R08). |

## 5. Principles (non-negotiable unless you overturn them)

1. **The simulation decides; language models describe.** An LLM output can only enter the world as a *typed, validated, journaled input*: a manifest, an argument frame, a choice from a legal-action menu, or a speech act. It can never be a free-form mutation.
2. **Every claim has provenance.** Any sentence of narration, any line of NPC dialogue and any chart can be traced back to event IDs.
3. **Determinism is a contract, tested continuously.** History is a pure function of `(engine version, world manifest, seed, input journal)`.
4. **Pressure is diegetic and declared.** There is no hidden "director" spawning events in the simulation. Pacing lives in presentation: auto-pause, camera choice and arc selection (R02 §R4). Any opt-in storyteller is labelled and logged as a cause.
5. **Never delete history by fixed rule.** Information is tiered, summarised and archived, but never destroyed by a weight assigned before significance is known.
6. **Each fidelity level must explain the level below it.** A higher-fidelity model must reproduce the aggregate statistics of the lower one within tolerance, or document why it does not.
7. **Licence hygiene by default.** No GPL, AGPL or non-commercial code or weights in the engine. Ideas are reimplemented, not copied. This keeps open the option of publishing or selling later.

## 6. Scope and sequencing

The five modalities are not five features of one release. Each one alone has been the *entire product* of a multi-year precedent (R11 §G). They share one engine and are delivered in sequence:

| Order | Modality | Delivered as | Why this position |
|---|---|---|---|
| 1 | **Millenary Time-Machine + Chronicle** (2D console) | The first playable | Cheapest test of the core thesis (S1). Exercises determinism, the sifter and the causal graph. No 3D or LLM dependency. |
| 2 | **Itinerant Catalyst (thin)**: Oracle verbs in the console | An extension of 1 | Adds agency and "Foresight" (Monte-Carlo forecasts) using the same engine. Proves the persuasion and epistemics models. |
| 3 | **Narrative Compiler**: chronicle, then saga, then novella | A batch pipeline | Depends on a rich chronicle. Batch and offline, so its risk is isolated. |
| 4 | **Living Diorama**: 3D render of the live or replayed world | The first 3D client | First consumer of the rendering and street-content pipeline. Hydration is passive (no dialogue). |
| 5 | **Street-Level Inhabitant**: hydration, then grounded dialogue | An extension of 4 | The highest-risk modality: an LLM facade, TTS, the hydration bubble. |
| 6 | **Oracle in 3D**: avatar plus strategic travel map | An extension of 2 and 5 | Combines everything. |

Out of scope until you decide otherwise (see `12_open_questions.md`): multiplayer and shared worlds, mobile, web distribution, and a commercial launch with store-policy compliance work.

**Scenario horizons are part of the world, not of the engine** (R09 §7). Pre-modern and fantasy worlds run 50–1,000+ years. A modern megacity runs at day or week resolution for 1–30 years, unless the manifest declares an explicit technology and institutional trajectory, in which case the run is labelled *speculative*.

## 7. Thesis risks we accept, and how we will know early

| Thesis | Our position | Early test |
|---|---|---|
| Unforgiving physics makes drama. | **Necessary but not sufficient.** Emergent behaviour is not narrative (Walsh, Ryan; R02 §3). Drama needs (a) systems with *contingent unlocking*: actions that create follow-up opportunities, (b) knowledge asymmetry, and (c) curation. | P2 gate: MOE-1 and MOE-6 on 10 seeds. If it fails, fix the simulation and sifter before any 3D. |
| Grounding makes generated fiction authentic. | **Half true.** Grounding fixes continuity and plot holes, not prose quality (R08). | P4 gate: zero canon contradictions plus your blind rating against an ungrounded baseline. |
| Local small LLMs can voice any citizen within their knowledge. | **Unproven.** Frontier models leak "future knowledge" 54% of the time on TimeChara (R04/R05). It needs architecture: plan-then-realise, belief-scoped context and claim checks. | P5 spike: a 200-scenario role-play eval run against our own belief store. |
| 1M entities with individual drama. | **Feasible in the macro model** as per-person ledger rows (R01: 19–64 MB, tens of ms per month). Individual *cognition* stays limited to 1–5k named actors. | P1/P3 MOP-1 benchmarks. |

## 8. Glossary

- **Ledger (T0).** The per-person table holding every living human or creature in the macro simulation, as narrow typed rows.
- **Named actor (T1).** A person promoted to full cognition: rulers, great houses, hive nodes, pinned persons.
- **Embodied (T3) / Conversant (T4).** Persons hydrated into the 3D bubble, or in live dialogue.
- **Pinning.** Making a person's detail persistent once it has been observed or has mattered. Pins are sticky.
- **Chronicle.** The append-only, typed, causal event store.
- **Sifter.** Pattern-matching over the chronicle that finds story structure, incrementally and retrospectively.
- **Salience / significance.** A *retroactive* score of an event's narrative importance, recomputed as consequences accumulate.
- **Manifest.** The declarative world configuration: registry mechanics plus bounded parameters plus initial conditions.
- **Mechanics Registry.** The closed vocabulary of implemented systems, archetypes, act classes, roles and tropes that manifests may reference.
- **Input journal.** Every nondeterministic input (player action, LLM decision, manifest), timestamped. Together with the seed it defines a history.
- **Timeline / branch.** One history. A fork creates a new timeline from a tick of a parent.
- **Foresight (mechanic).** Monte-Carlo rollouts of forked timelines that estimate the probabilities of future outcomes. It is the Oracle's power and the engine's namesake.
- **Fidelity level.** A versioned implementation tier of a system model (F0 coarse to F3 fine). Each level has consistency contracts against the level below.
