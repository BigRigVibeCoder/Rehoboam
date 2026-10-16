# 01 — Vision, Intent and Definition of Success

*Status: Draft v1.1 (2026-10-15). Owner: project lead. Edited by: systems design.*
*Inputs: the four attached draft documents (Product Vision, Definitive System Design v2.0, Open-Source Bootstrap, rehoboam_2), your answers to the intake questions, and research dossiers R01–R11.*
*v1.1 changes: MOE-1 gains a precision target and a pause budget; MOE-2 uses a coupling-ablated baseline; MOE-4 is redefined as "frozen past, migrated future"; MOE-6 is reported with Clopper–Pearson bounds; fidelity-level principle relaxed to match `05.1` §3; glossary aligned with the v1.1 architecture (planes, scheduled outcomes, material inputs, genealogical IDs, console modes).*

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
| V4 | **Scale invariance.** One world can be seen from orbit across 1,000 years or from a cobblestone corner in real time. **And one engine runs worlds from a single person to one million** (ADR-0028): a household enduring the trials of Job, or a realm across a millennium. | Vision §1, Design §1.2; director, 2026-10-15 |
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
| **S4** | **Evolvable.** Fidelity and complexity can rise continuously without rewrites. | New mechanics and higher-fidelity models can be added by AI agents as isolated, tested modules (MOE-5), and old worlds remain loadable (MOE-4). | Every new feature needs a cross-cutting refactor. The engine fossilises at its first fidelity level. |

Commercial success, public reception and store policy were **not** chosen as the bar. They are recorded as constraints to keep open (§6), not as goals. The product-landscape dossier (R11) still matters, because it shows which design choices would close that door later.

## 3. Design drivers derived from success

These drivers rank and break ties for every decision in the architecture and the ADRs.

1. **D1 — Truth before beauty.** The simulation is the only source of truth. Everything shown, whether prose, 3D or dialogue, is a *view* over it and carries provenance back to it. This serves S1, S2 and S3.
2. **D2 — Story yield is the north-star metric.** A deterministic simulation that is boring has failed, however fast it is. The sifter and chronicle are built *with* the first simulation, not after it. This serves S1.
3. **D3 — Never lose history, never lie about it.** Event sourcing, deterministic replay, keyframes, branching timelines and versioned saves all follow. Nothing is deleted on a rule fixed at creation time. This serves S3.
4. **D4 — Observer invariance.** Looking at the world, by hydrating a street or opening an inspector, must not change history. Only *material inputs*, which are journaled, may. This serves S2 and S3.
5. **D5 — Fidelity is a dial, not a rewrite.** Every system is a pluggable model with explicit fidelity levels. Each level is validated against the stylized facts, and neighbouring levels agree on conserved quantities and declared targets. More compute buys more fidelity through configuration. This serves S4.
6. **D6 — Agent-buildable by construction.** Small modules, typed contracts, golden-run tests and spec-first work packages, because AI agents are the workforce. This serves S4.
7. **D7 — Scale out with ensembles, not distribution.** One authoritative process per timeline. Unbounded compute goes into *more timelines* (Monte-Carlo foresight, calibration sweeps, what-if branches), more model fidelity and bigger LLMs, never into a sharded world. R10 records why the SpatialOS approach failed. This serves S4.

## 4. Measures of effectiveness (MOE) and performance (MOP)

**MOEs** measure whether the success claims hold. **MOPs** measure engineering progress toward them. Targets marked "P1/P2/…" are phase gates; see `11_roadmap.md`.

### Measures of effectiveness

| ID | Measure | Method | Target |
|---|---|---|---|
| MOE-1 | **Story yield.** Auto-paused events per simulated century that you rate ≥ 4/5 on "I want to know what happens next". Two companion measures stop yield being bought with noise: **precision**, the fraction of auto-pauses you rate ≥ 3/5, and the **pause budget**, the number of auto-pauses per simulated century. | Blind rating sessions over random seeds, using the rating tool built into the console (TM-6). | Pause budget: ≤ 10 auto-pauses per century at default thresholds. P2: ≥ 3 per century rated ≥ 4, median over 10 seeds, with precision ≥ 0.5. P3: ≥ 6, with precision ≥ 0.7. |
| MOE-2 | **Chronicle quality.** Pairwise preference of the chronicle against the chronicle of a **coupling-ablated** run: the same engine, manifest, seed and sifter, with the cross-system couplings disabled (`05.1` §8, e.g. famine no longer feeds unrest). | Blind A/B of template-rendered, length-matched chronicles (`09` V-HUM-02). | ≥ 80% preference. *(Proposed, Q21: gate on the lower 80% confidence bound being above 50% over ≥ 40 pairs, and report the point estimate against 80%.)* |
| MOE-3 | **Inhabitation coherence.** The rate of observed contradictions in sampled people and places: identity, kin, location, memory. | Automated probes plus manual "interrogate 20 random people" sessions. | 0 hard contradictions. Fewer than 1 soft inconsistency per 20 probes. |
| MOE-4 | **Continuity ("frozen past, migrated future").** A new engine build can load a saved world at its head and continue from a **migration keyframe**. Seeks to times before the migration replay with the archived build that wrote that history (DET-10, `05.2` §7). | Save-compatibility corpus of golden worlds in CI: load at head, continue, then seek to a point before the migration. | 100% of saved worlds load and continue at their head. 100% of pre-migration seeks reproduce the archived build's plane-A hashes. |
| MOE-5 | **Evolvability.** Agent lead time to add a new registry mechanic (e.g., "drought-resistant crop") or raise one system's fidelity level, measured from spec to merged with all gates green. | Track work-package cycle times. | A median of 1–3 agent-days for a mechanic. A data-only extension touches only the Registry. A new system touches only its own crate, Registry entries and contract tests, plus contract crates and lifecycle command types (EVO-1, `13` §4.2). |
| MOE-6 | **Bad-run rate.** Fraction of seeds with fewer than **N = 2** completed story arcs (complete sift matches from ≥ 2 pattern packs) per simulated century (R02 §R4). N may be revised at G1b; any change is logged in the decisions log. | Ensemble runs, 32 seeds per manifest. Reported with its exact two-sided 95% (Clopper–Pearson) confidence interval. | Below 10% (point estimate). |

**Why these baselines and bounds.**
- **MOE-1** without a budget could be met by pausing more often. The budget caps pauses at about one per decade, and precision measures how many of them were worth your time. The P2 and P3 precision targets are initial values; changes are made at a gate and logged in the decisions log.
- **MOE-2** compares against a coupling-ablated run, not a random-event chronicle or a "no engine" text. A random baseline does not test the thesis, but it is not trivially easy either. In Behrooz et al. (2015) random spans won 30% of choices, and in Lessard et al. (2026) the random control was the best-liked story. So it runs alongside as a non-gating selection check (V-HUM-11). The ablation tests the actual thesis (V1): coupled systems, not the sifter alone, make the stories.
- **MOE-6** intervals are wide at 32 seeds. 0 bad runs of 32 still has an upper bound of about 11%, and 3 of 32 (9.4%) has an interval of about 2–25%. A borderline result is resolved by adding seeds, not by reading the point estimate alone.

### Measures of performance

| ID | Measure | Target |
|---|---|---|
| MOP-1 | Headless macro run time for 1,000 years at 100k people, monthly resolution (8-core desktop) | P1: ≤ 2 min. P3: ≤ 15 min at 1M people (provisional; set from P1 measurements, `08`). |
| MOP-2 | Determinism: plane-A `state_hash` identical across thread counts and repeated runs (same binary and platform), and unchanged by any viewing (DET-6) | 100%, checked every sim-month in CI. |
| MOP-3 | Timeline scrub latency, chronicle and aggregates (no re-simulation) | < 100 ms. |
| MOP-4 | "Drop in" seek latency (keyframe plus replay) | ≤ 10 s at 100k people with a ±10-year cache. |
| MOP-5 | Stylized-fact validation bands passed (the M9 validation harness, `05.1` §3; bands from R09 §11 and the R09 recommendation "Validation harness (M9)") | ≥ 90% of bands across a 32-seed ensemble. |
| MOP-6 | Canon contradictions in generated prose | 0 per 1,000 words for CT0/CT1 claims (R08). |

## 5. Principles (non-negotiable unless you overturn them)

1. **The simulation decides; language models describe.** An LLM output can only enter the world as a *typed, validated, journaled input*: a manifest, an argument frame, a choice from a legal-action menu (a pull input), or a speech act. It can never be a free-form mutation.
2. **Every claim has provenance.** Any sentence of narration, any line of NPC dialogue and any chart can be traced back to event IDs.
3. **Determinism is a contract, tested continuously.** History is a pure function of `(engine build, registry version, world manifest, seed, input journal)`.
4. **Pressure is diegetic and declared.** There is no hidden "director" spawning events in the simulation. Pacing lives in presentation: auto-pause, camera choice and arc selection (R02 §R4). Any opt-in storyteller is labelled and logged as a cause.
5. **Never delete history by fixed rule.** Information is tiered, summarised and archived, but never destroyed by a weight assigned before significance is known.
6. **Each fidelity level must stand on its own evidence.** Every level is validated against the stylized facts. Neighbouring levels must agree on conserved quantities and declared targets, and the tested mixes are published as a supported-combinations matrix (`05.1` §3).
7. **Licence hygiene by default.** No GPL, AGPL or non-commercial code or weights in the engine. Ideas are reimplemented, not copied. This keeps open the option of publishing or selling later.

## 6. Scope and sequencing

The five modalities are not five features of one release. Each one alone has been the *entire product* of a multi-year precedent (R11 §G). They share one engine and are delivered in sequence:

| Order | Modality | Delivered as | Why this position |
|---|---|---|---|
| 1 | **Millenary Time-Machine + Chronicle** (2D console) | The first playable | Cheapest test of the core thesis (S1). Exercises determinism, the sifter and the causal graph. No 3D or LLM dependency. |
| 2 | **Itinerant Catalyst (thin)**: Oracle verbs in the console | An extension of 1 | Adds agency and "Foresight" (Monte-Carlo forecasts) using the same engine. Proves the persuasion and epistemics models. |
| 3 | **Narrative Compiler**: chronicle, then saga, then novella | A batch pipeline | Depends on a rich chronicle. Batch and offline, so its risk is isolated. |
| 4 | **Living Diorama**: a lo-fi glyph view of the live or replayed world (Dwarf Fortress style; ADR-0029) | The first inhabitable view, in the console; also a desktop wallpaper | Cheap, works in every era via glyph packs, and lets you watch the world months earlier than a 3D client would. No T3 hydration yet: crowd shading, glyphs and Spectacle only. The embodied bubble arrives with 5. A 3D "painted miniature" client is an optional later art phase. |
| 5 | **Street-Level Inhabitant**: hydration, then grounded dialogue | An extension of 4 | The highest-risk modality: an LLM facade, TTS, the hydration bubble. |
| 6 | **Oracle on the travel map**: avatar plus strategic travel map (lo-fi first; 3D in the optional art phase 2) | An extension of 2 and 5 | Combines everything. |

Out of scope until you decide otherwise (see `12_open_questions.md`): multiplayer and shared worlds, mobile, web distribution, and a commercial launch with store-policy compliance work.

**Worlds are not stuck in time (ADR-0027).** Eras are *state*, not templates. A world that starts agrarian can pass through early-modern, industrial and modern conditions in one continuous run, because every era-sensitive mechanism is a function of technology and institutions, and institutions themselves change. Coverage grows through **era packs**. A world that reaches the edge of what the engine can model waits there, files a gap report, and continues when the next pack ships.

**Horizons** (R09 TL;DR 11, re-scoped by ADR-0027). A template sets the *starting* state, not an era lock. *Speculative* now means beyond the technological frontier: running past the last era pack, or forecasting a modern-start world beyond about 50 years. Passing *through* eras the engine supports is not speculative. A modern-start megacity still runs at day or week resolution for 1–50 years by default.

## 7. Thesis risks we accept, and how we will know early

| Thesis | Our position | Early test |
|---|---|---|
| Unforgiving physics makes drama. | **Necessary but not sufficient.** Emergent behaviour is not narrative (Walsh, Ryan; R02 §3). Drama needs (a) systems with *contingent unlocking*: actions that create follow-up opportunities, (b) knowledge asymmetry, and (c) curation. | P2 gate: MOE-1 on 10 seeds (rated by you); MOE-6 on 32 seeds (automated). If it fails, fix the simulation and sifter before any 3D. |
| Grounding makes generated fiction authentic. | **Half true.** Grounding fixes continuity and plot holes, not prose quality (R08). | P4 gate: zero canon contradictions plus your blind rating against an ungrounded baseline. |
| Local small LLMs can voice any citizen within their knowledge. | **Unproven.** Frontier models leak "future knowledge" 54% of the time on TimeChara (R04/R05). It needs architecture: plan-then-realise, belief-scoped context and claim checks. | P6 spike (WP P6.4): RP-200, a 200-scenario role-play eval run against our own belief store. |
| 1M entities with individual drama. | **Feasible in the macro model** as per-person ledger rows (R01: 19–64 MB, tens of ms per month). Individual *cognition* stays limited to 1–5k named actors. | P1/P3 MOP-1 benchmarks. |

## 8. Glossary

Definitions follow the architecture set (`05.0`–`05.9`) and ADR-0024 to ADR-0026. Where this glossary and a 05.x document disagree, the 05.x document wins.

### 8.1 State, identity and time
- **Planes (A, B, C).** The three partitions of state, each with its own hash (ADR-0024).
  - **Plane A, authoritative:** ledger, life-course log, named actors, beliefs, agenda and timers, chronicle events (`state_hash`).
  - **Plane B, observation:** pins, the facts ledger, hydrated presentation (`observation_hash`).
  - **Plane C, narrative:** sift matches, significance, visibility tiers, ratings, prose (`narrative_hash`).
  - Plane A never reads B or C. The only way into A is a journaled material input.
- **Ledger (T0).** The plane-A table of every *living* person or creature in the macro simulation, as narrow typed rows. Dead rows leave at once.
- **Life-course log.** The append-only plane-A record of every person ever born: parents, birth, death, cause, and intervals of residence, occupation, faction and marriage (ADR-0026). It makes genealogy complete for everyone.
- **Genealogical ID.** A `PersonId` derived from ancestry, not from a global counter. A child is `H64(mother_id, parity)`, a founder `H64(seed, "founder", k)`, an immigrant `H64(seed, "immigrant", origin, t, k)`. The same person has the same ID in every branch.
- **SimTime.** Signed 64-bit minutes (`i64`) from the world epoch, so founders can be born before year 0.
- **Era.** One simulated century, unless stated otherwise. Significance is recomputed per era.
- **Chronicle.** The append-only, typed, causal event store. Its events are plane A.
- **Anchored state / anchored attribute.** The value of a Registry-declared attribute captured in `event_snapshot` when an event is emitted. Patterns read it as `attr(args)@eN`, never as current state, so a sift result never depends on when sifting runs.
- **Facts ledger.** The plane-B record of what you have been shown about a person or place. Every later presentation must honour it. The simulation never reads it.

### 8.2 Representation tiers
These are distinct from the canon tiers CT0–CT3 (§8.4).
- **T−1 (off-map).** Aggregate compartments for populations outside the map. People who cross in or out are materialised or absorbed by an adapter.
- **Pop.** An aggregate per region × class × faith holding belief shares, morale, fear and omen credibility for the masses.
- **Named actor (T1).** A person with a full mind: rulers, active great-house members, officers, clergy, office-holding hive nodes, dragons. Promotion happens only by simulation criteria (office, great-house rank, material involvement, Registry notability) or a journaled input. **Pinning never promotes to T1.**
- **Light mind.** The mind of an unnamed person, computed on demand as a pure function of plane A (keyed traits, pop belief shares, life-course log). It is never stored, and being observed never makes the person T1.
- **T2c (crowd proxy).** A real living person, sampled without replacement from those whose schedule puts them outdoors at that place and hour, drawn as a VAT crowd instance. A plane-B view.
- **Embodied (T3) / Conversant (T4).** Persons presented in the 3D bubble, or in live dialogue. Plane-B views that exist only at Live speed.
- **Spectacle.** Things with a world-scale footprint (dragons, armies, processions, fires), drawn out to the horizon at any speed band.
- **Hydration.** Presenting real persons near the camera or avatar at T3/T4 detail. It is a view computed from plane A. It never takes authority, never invents people and never changes history.
- **Pinning.** Keeping what you have been shown of a person stable, through the facts ledger (plane B). Pins come from engagement and are sticky. They never change plane A: no T1 promotion, no events, no retained rows, no change to scheduling.

### 8.3 Inputs, determinism and history
- **Input journal.** The canonical record of every nondeterministic input (manifest, settings, player actions, speech acts, Oracle verbs, LLM choices), with its tick and scope. Together with the build, Registry version, manifest and seed it defines a history. Replay reads it and never regenerates it.
- **Material input.** A journaled input that changes plane A: a player action, a player speech act or an Oracle verb. It declares a **dependency scope** (person, household, settlement or region). Pending timers in that scope are cancelled and re-sampled.
- **Push / pull input.** A push input starts outside the simulation (you, the Oracle, settings) and applies at its tick. A **pull input** answers a request the simulation emitted, such as a deliberation consult. It applies at t + D, and replay hard-fails if it is missing, extra or different.
- **Scheduled outcome.** A state-changing outcome (death, birth, illness onset, migration, marriage) whose time is sampled in advance for everyone, watched or not, and held as a plane-A timer that fires at its own tick (ADR-0025). Nothing is ever back-dated.
- **Timeline / branch.** One history. A fork creates a new timeline from a tick of a parent. **Acting in the past forks:** a material input before the timeline head always creates a fork. Dropping in to watch or inspect is read-only.
- **Golden run.** A reference world replayed in CI with its plane-A hash compared, sim-year by sim-year, against a recorded baseline. An unexplained mismatch blocks the merge.
- **Frozen past, migrated future.** How engine upgrades keep worlds alive: a new build continues from a migration keyframe at the head, and history before it replays with the archived build that wrote it (MOE-4).
- **Fidelity level.** A versioned implementation tier of a system model (F0 coarse to F3 fine). Each level is validated against the stylized facts. Neighbouring levels must agree on conserved quantities and declared targets.
- **Burn-in.** The headless runs that check a newly compiled world, in two stages:
  - the **liveness gate** (200 years × 8 seeds, about a minute at 100k; it blocks play): no extinction or explosion, enough completed arcs;
  - **certification** (1,000 years × 32 seeds against the stylized-fact bands; it runs in the background and labels the world certified or uncertified).
- **Stylized facts.** Empirical regularities of real history that a plausible run must reproduce within a numeric band: child mortality, secular-cycle periods, ruler-murder rates, city rank-size slope, war-severity tails and others (`09` §3.1, R09 §11). The M9 validation harness checks them.

### 8.4 Narrative
- **Sifter.** Pattern matching over the chronicle that finds story structure, incrementally (live) and retrospectively.
- **SQL twin.** The DuckDB query equivalent to a sifting pattern. CI asserts that the live matcher and its twin produce identical match sets.
- **MatchId.** The plane-C ID space for sift matches. It is separate from EventIds, so editing a pattern never renumbers simulation events.
- **Salience / significance.** A *retroactive* score of an event's narrative importance, recomputed each era as consequences accumulate. Plane C.
- **Story-shape gate.** The Narrative Compiler's first check (stage 0) that an epoch has novel-shaped material: protagonists in ≥ 5 backbone events and a mostly connected causal graph. An epoch that fails is routed to the Saga or Chronicle form (`05.4` §6).
- **Canon tiers (CT0–CT3).** What generated prose may assert: CT0 canon (the immutable record), CT1 derived facts (ages, distances, seasons), CT2 licensed embellishment, CT3 forbidden invention (`05.4` §6.1).
- **Style card.** A versioned description of a prose style (density, POV, register, ban list) with 2–4 US public-domain exemplars. It replaces "write like <named author>".

### 8.5 World compiler and Registry
- **Manifest.** The declarative world configuration: Registry mechanics plus bounded parameters plus initial conditions.
- **Mechanics Registry.** The closed vocabulary of implemented systems, archetypes, act classes, roles, verbs, attributes and tropes that manifests may reference. Its small typed **effect language** lets data-only extensions land without code (`05.5` §2.1).
- **Intent / expander.** The LLM writes a small **intent**: a template choice, bounded overrides and flavour seeds. A deterministic **expander** turns it into the full manifest. No LLM writes the bulk of a world.
- **Mapping table.** Emitted alongside the intent. Every content-bearing span of the prompt is mapped to Registry IDs as *exact*, *approximate* or *unsupported*, and the table is shown on the Genesis screen.
- **Gap report.** A structured record of something you asked for that the Registry cannot express. Every approximate or unsupported mapping files one. A `data_only` gap closes with a Registry edit; a `needs_code` gap becomes an agent work package.
- **Catalyst.** A manifest initial condition that sets up drama, such as dynastic tension or a contested artefact. It names a polity and a role, never a person, and never forces an outcome.
- **Overlay.** A setting-specific readout (Paranoia, Filial Piety, Despair…) derived from the generic psychological core by a Registry formula. The UI and the sifter can read it; the simulation runs on the core.

### 8.6 Minds and the Oracle
- **Thought.** A timed modifier on a mind: vector deltas, half-life, stacking rules and source. Whispers and omens land as Thoughts.
- **ArgumentFrame.** The typed parse of a whisper: appeals, target vectors, requested action, claims, references and a meta-instruction flag. It is grounded and scored deterministically. The whisper's text never reaches a mind.
- **Deliberation consult.** An optional LLM choice from a legal-action menu for a named actor's decision, applied as a pull input after a fixed delay D. Off by default. Foresight rollouts use the deterministic fallback.
- **Oracle.** Your in-world persona: a plane-A entity with its own belief store, fed by the same channels as everyone else. Its verbs are material inputs.
- **Cultivator mode / Oracle mode.** The console's two modes. **Cultivator** shows the true state, secrets included, with Oracle verbs disabled by default. **Oracle** filters every view, including Foresight, through what the Oracle knows. Switching is journaled.
- **Foresight (mechanic).** Monte-Carlo rollouts of forked timelines that estimate the probabilities of future outcomes. It is the Oracle's power and the engine's namesake. In Oracle mode, rollouts start from a belief-resampled state, so they never leak hidden truth.
