# 01 — Vision, Intent and Definition of Success

*Status: Draft v1.1 (2026-10-15). Owner: project lead. Edited by: systems design.*
*Inputs: the four attached draft documents (Product Vision, Definitive System Design v2.0, Open-Source Bootstrap, rehoboam_2), your answers to the intake questions, and research dossiers R01–R11.*
*v1.1 changes: MOE-1 gains a precision target and a pause budget; MOE-2 uses a coupling-ablated baseline; MOE-4 is redefined as "frozen past, migrated future"; MOE-6 is reported with Clopper–Pearson bounds; fidelity-level principle relaxed to match `05.1` §3; glossary aligned with the v1.1 architecture (planes, scheduled outcomes, material inputs, genealogical IDs, console modes).*
*v1.2 (2026-10-16): V2, principle 1 and the glossary state the LLM roles of ADR-0034 (D27): LLMs translate in and describe out, never decide; deliberation consults and pull inputs retired.*
*v1.3 (2026-10-16): the saga engine (ADR-0035, ratified by you the same day). New north star; §1, §2, MOE-1–MOE-6 and MOP-1 re-aimed at sagas instead of economic fidelity (IDs kept); the three consumption modes; §6 re-sequenced to the new phases; principle 7 follows ADR-0021's personal-use mode; glossary entries for artifacts, vows, mythic catalysts and the chronicle pipeline.*
*v1.4 (2026-10-16): principles 8 and 9, the red team's Laws 2 and 3, adopted narrowly by you at G0 (`docs/red_team/RT-2026-10-16-G0-dispositions.md` §2).*

---

> **North star.** Rehoboam generates legends: a deterministic world whose dynasties, vows, relics and betrayals become sagas you can listen to, read and scrub through.

## 1. Your intent, restated

This section restates the intent so that you can correct it. Every later document depends on it.

> **Rehoboam is a personal saga engine.** It runs a world from founding through centuries of history, in the manner of the Silmarillion or Dwarf Fortress's legends. Its story is **dynasties, ambitions, vows, relics and betrayals**; famine, plague, war, demography and taxes are the stage lighting and the pressure, not the story. History comes from deterministic simulation, not from a language model making things up. You stop at the moments that matter and ask *why*.
>
> **You are a reader and listener, not a gamer.** You **listen** to your world as an episodic audio chronicle, **read** it as illustrated storyboards and prose, **scrub** its causal timeline, and **glance** at it as a 2D diorama. You keep improving it, raising its depth year after year. You build it **with AI coding agents** doing most of the implementation. **Compute is not the constraint**, but the default world is a kingdom, not a million people. The first thing that must work is **a dynasty that lives, dies and inherits, told as a chronicle** (ADR-0035).

These are the elements of your drafts I treat as the **core of the vision**. They are not up for debate unless you say otherwise:

| # | Core element | Where it came from |
|---|---|---|
| V1 | Drama is born from **unforgiving, deterministic systems**: dynastic ambition, succession, vows, relics and betrayal, under the pressure of famine, plague, war and death. | Vision §1.1, Design §1.3; ADR-0035 |
| V2 | LLMs are used **only where they excel**, in three roles: the **workforce** that builds the engine (coding, research and review agents; build time); **translation in** (whispers, speech and world prompts become typed, journaled inputs); and **description out** (dialogue facade, narration). **They never decide**: no action, outcome or parameter of the simulation (ADR-0034). | Vision §4, Design §1.3; D27 |
| V3 | **Asymmetric minds.** Fragmented, paranoid, grieving humans collide with a zero-friction collective intelligence, the Hive. | Vision §1.1 |
| V4 | **Scale invariance.** One world can be seen from orbit across 1,000 years or from a cobblestone corner in real time. **And one engine runs worlds from a single person to one million** (ADR-0028): a household enduring the trials of Job, or a realm across a millennium. | Vision §1, Design §1.2; director, 2026-10-15 |
| V5 | **Infinite worlds from prompts.** One engine, configured by a declarative world manifest. | Vision §3 |
| V6 | **Relationships with the world**, in order: the **chronicle you listen to and read** (audio, prose, graphic storyboards), the **Time-Machine** you scrub, the **Oracle** who influences, and the **Diorama** you glance at. Walking the streets as an Inhabitant is later and optional (ADR-0035). | Vision §2; ADR-0035 |
| V7 | **Influence, not control.** The player is a mortal catalyst with a finite resource (Foresight), not an RTS god. | Vision §2 (Modality 4) |

### 1.1 The three consumption modes (ADR-0035)

You take in your world in three ways, plus one ambient view. All of them are plane C or plane B: they read the chronicle and never change history.

| Mode | What you do | Delivered as | First phase |
|---|---|---|---|
| **Listen** | Hear your world's saga as episodes | The audio chronicle: local MP3 chapters (narration in P2, a multi-voice drama in P3), played in the local saga reader | P2 |
| **Read** | Flip through the sifted key scenes as illustrated panels, and read the saga as prose | The graphic chronicle (storyboards, then image panels with clickable causal links) and the narrative compiler, in the local Web Graphic Novel / Storyboard Reader, the only presentation surface for sagas | P3 |
| **Scrub** | Move along the causal timeline and click any event through to its causes | The text Time-Machine (P2), then the saga reader's causal timeline (P3) | P2 |
| *Glance* (ambient) | Keep the world on a second screen | The 2D diorama of CC0 tiles and glyphs | P5 |

## 2. What "success" means

Your answer, verbatim: *"It's a creative engine and a world I want to inhabit and be invested in that I can improve and continue to increase fidelity and complexity."*

That sentence contains four testable claims. The whole plan is organised around them.

| ID | Success claim | What would prove it | What would falsify it |
|---|---|---|---|
| **S1** | **Creative engine.** It produces sagas you actually want to listen to and read. | Over successive runs you consistently rate sifted sagas and chronicle episodes as "I want the next one" (see MOE-1, MOE-2). | Runs read like logs. Every seed tells the same story. Vows and relics play out like a script. Prose hides the fact that the simulation is shallow. |
| **S2** | **Inhabitable,** as a reader inhabits a legendarium. The world feels like a place with people in it, not a spreadsheet. | Any person you meet in the chronicle has a coherent identity, kin, history, look and purpose. Houses, relics and places persist and change believably over centuries (MOE-3). | People pop in and out of existence. A hero's face changes between panels. Towns re-roll between visits. |
| **S3** | **Investment.** You care what happens to *your* world and can keep it. | Worlds persist indefinitely. History is never silently lost. You can branch "what-ifs", replay any moment, and revisit people across centuries (MOE-4). | Saves break on upgrade. Compaction deletes the backstory you cared about. A replay produces a different past. |
| **S4** | **Evolvable.** Fidelity and complexity can rise continuously without rewrites. | New mechanics and higher-fidelity models can be added by AI agents as isolated, tested modules (MOE-5), and old worlds remain loadable (MOE-4). | Every new feature needs a cross-cutting refactor. The engine fossilises at its first fidelity level. |

Commercial success, public reception and store policy were **not** chosen as the bar. They are recorded as constraints to keep open (§6), not as goals. The product-landscape dossier (R11) still matters, because it shows which design choices would close that door later.

## 3. Design drivers derived from success

These drivers rank and break ties for every decision in the architecture and the ADRs.

1. **D1 — Truth before beauty.** The simulation is the only source of truth. Everything shown, whether prose, voices, panels or dialogue, is a *view* over it and carries provenance back to it. This serves S1, S2 and S3.
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
| MOE-1 | **Story yield.** Auto-paused saga moments (a vow sworn or broken, a relic changing hands, a betrayal, a succession) per simulated century that you rate ≥ 4/5 on "I want to know what happens next". Two companion measures stop yield being bought with noise: **precision**, the fraction of auto-pauses you rate ≥ 3/5, and the **pause budget**, the number of auto-pauses per simulated century. | Blind rating sessions over random seeds, using the rating tool built into the console (TM-6). | Pause budget: ≤ 10 auto-pauses per century at default thresholds. P2: ≥ 3 per century rated ≥ 4, median over 10 seeds, with precision ≥ 0.5. P4: ≥ 6, with precision ≥ 0.7. |
| MOE-2 | **Chronicle quality.** Pairwise preference of the chronicle against the chronicle of a **coupling-ablated** run: the same engine, manifest, seed and sifter, with the cross-system couplings disabled (`05.1` §8: a murder no longer feeds a vow, a famine no longer strains one, a relic no longer pulls on its bearer). | Blind A/B of template-rendered, length-matched chronicles (`09` V-HUM-02). A second, report-only arm turns the saga mechanics off (vows and artifacts; V-SAGA-06). | ≥ 80% preference. *(Proposed, Q21: gate on the lower 80% confidence bound being above 50% over ≥ 40 pairs, and report the point estimate against 80%.)* |
| MOE-3 | **Inhabitation coherence.** The rate of observed contradictions in sampled people, places and relics: identity, kin, location, memory, who bears what. | Automated probes plus manual "interrogate 20 random people" sessions. | 0 hard contradictions. Fewer than 1 soft inconsistency per 20 probes. |
| MOE-4 | **Continuity ("frozen past, migrated future").** A new engine build can load a saved world at its head and continue from a **migration keyframe**. Seeks to times before the migration replay with the archived build that wrote that history (DET-10, `05.2` §7). | Save-compatibility corpus of golden worlds in CI: load at head, continue, then seek to a point before the migration. | 100% of saved worlds load and continue at their head. 100% of pre-migration seeks reproduce the archived build's plane-A hashes. |
| MOE-5 | **Evolvability.** Agent lead time to add a new registry mechanic (e.g. a new relic kind or vow kind) or raise one system's fidelity level, measured from spec to merged with all gates green. | Track work-package cycle times. | A median of 1–3 agent-days for a mechanic. A data-only extension touches only the Registry. A new system touches only its own crate, Registry entries and contract tests, plus contract crates and lifecycle command types (EVO-1, `13` §4.2). |
| MOE-6 | **Bad-run rate.** Fraction of seeds with fewer than **N = 2** completed saga arcs (complete sift matches from ≥ 2 pattern packs, such as a vow arc and a relic chain) per simulated century (R02 §R4). N may be revised at G1b; any change is logged in the decisions log. | Ensemble runs, 32 seeds per manifest. Reported with its exact two-sided 95% (Clopper–Pearson) confidence interval. | Below 10% (point estimate). |

**Why these baselines and bounds.**
- **MOE-1** without a budget could be met by pausing more often. The budget caps pauses at about one per decade, and precision measures how many of them were worth your time. The P2 and P3 precision targets are initial values; changes are made at a gate and logged in the decisions log.
- **MOE-2** compares against a coupling-ablated run, not a random-event chronicle or a "no engine" text. A random baseline does not test the thesis, but it is not trivially easy either. In Behrooz et al. (2015) random spans won 30% of choices, and in Lessard et al. (2026) the random control was the best-liked story. So it runs alongside as a non-gating selection check (V-HUM-11). The ablation tests the actual thesis (V1): coupled systems, not the sifter alone, make the stories.
- **MOE-6** intervals are wide at 32 seeds. 0 bad runs of 32 still has an upper bound of about 11%, and 3 of 32 (9.4%) has an interval of about 2–25%. A borderline result is resolved by adding seeds, not by reading the point estimate alone.

### Measures of performance

| ID | Measure | Target |
|---|---|---|
| MOP-1 | Headless run time for 1,000 years of the default saga world (meso: ≤ 50k people, ≤ 2,000 named actors), monthly resolution (8-core desktop) | P1b: ≤ 2 min. At 100k (≤ 2 min) and 1M (≤ 15 min, provisional) only with the optional scale-up (ADR-0035; `08`). |
| MOP-2 | Determinism: plane-A `state_hash` identical across thread counts and repeated runs (same binary and platform), and unchanged by any viewing (DET-6) | 100%, checked every sim-month in CI. |
| MOP-3 | Timeline scrub latency, chronicle and aggregates (no re-simulation) | < 100 ms. |
| MOP-4 | "Drop in" seek latency (keyframe plus replay) | ≤ 10 s at 100k people with a ±10-year cache. |
| MOP-5 | Stylized-fact validation bands passed (the M9 validation harness, `05.1` §3; bands from R09 §11 and the R09 recommendation "Validation harness (M9)") | ≥ 90% of the gate bands across a 32-seed ensemble. Under ADR-0035 the economic-history bands are reported, not gated (`09` §3.1). |
| MOP-6 | Canon contradictions in generated prose | 0 per 1,000 words for CT0/CT1 claims (R08). |

## 5. Principles (non-negotiable unless you overturn them)

1. **The simulation decides; language models describe.** An LLM output can only enter the world as a *typed, validated, journaled input* that translates a person's own act: a manifest, an argument frame or a speech act. It can never be a free-form mutation, and it never chooses an action for anyone in the world (ADR-0034).
2. **Every claim has provenance.** Any sentence of narration, any line of NPC dialogue and any chart can be traced back to event IDs.
3. **Determinism is a contract, tested continuously.** History is a pure function of `(engine build, registry version, world manifest, seed, input journal)`.
4. **Pressure is diegetic and declared.** There is no hidden "director" spawning events in the simulation. Pacing lives in presentation: auto-pause, camera choice and arc selection (R02 §R4). Any opt-in storyteller is labelled and logged as a cause.
5. **Never delete history by fixed rule.** Information is tiered, summarised and archived, but never destroyed by a weight assigned before significance is known.
6. **Each fidelity level must stand on its own evidence.** Every level is validated against the stylized facts. Neighbouring levels must agree on conserved quantities and declared targets, and the tested mixes are published as a supported-combinations matrix (`05.1` §3).
7. **Licences in personal-use mode** (D19, ADR-0021 as amended). Rehoboam is never distributed, so any licence may be used, except one whose terms forbid the use itself. Unlicensed code is still not copied, and a licence ledger records what is used.
8. **No spontaneous reality** (the red team's Law 2, adopted narrowly at G0). Every plane-A entity and every change to plane-A state, a title, a prop or a status included, originates from a simulated in-world event. There is no god-mode spawning and no global count controller. *Reading:*
   - World creation is the root of the causal graph and seeds the initial state.
   - "A simulated in-world event" means a system's mechanics, a timer that fires, or a journaled material input. A routine column update therefore needs no event record of its own.
   - An ordinary prop drawn from aggregate stock is such a draw (the gap-1 ruling, D28).
   - A count, such as the number of nobles or the population, is an outcome that tests check, never a target the engine enforces.
9. **Tragic, not slapstick** (the red team's Law 3, adopted narrowly at G0). A mental break resolves only into a mythic, consequential act, such as usurpation, exile, a feud or a broken vow, never into a slapstick or meme break. *Reading:*
   - This is a rule for authoring. The Registry's break acts and stress sources are written for the saga register, and plane C tells events in that register.
   - No rule in plane A filters, suppresses or rerolls an outcome for its tone (principle 4, ADR-0009).
   - The listed acts are examples; each break act fits the actor's station.

## 6. Scope and sequencing

The modalities are not features of one release. Each one alone has been the *entire product* of a multi-year precedent (R11 §G). They share one engine and are delivered in sequence. **ADR-0035 re-orders them for a reader and listener:** the saga and its chronicle come first, interactive clients last.

| Order | Phase | What you get | Why this position |
|---|---|---|---|
| 1 | **P1a Bloodlines** | Dynasties that live, die and inherit, with named minds, told as a templated family history | The substrate every saga needs: people, kin, succession, ambition |
| 2 | **P1b Legends** | Artifacts, vows, grudges and quests, mythic catalysts and secrets, under low-fidelity famine and unrest, in a kingdom of 10k–50k | The saga mechanics, in plane A, where they change what people do |
| 3 | **P2 Sagas & audio v0** | The sifter with saga pattern packs, the keyboard Time-Machine (**scrub**), the templated chronicle and the **offline audio exporter** with reader v0 (**listen**) | The cheapest test of the thesis (S1), now on sagas: G2 |
| 4 | **P3 The illustrated saga** | Prose sagas, a multi-voice audio drama, the **graphic chronicle** (**read**) and the saga reader with the causal timeline | Depends on a rich chronicle. Batch and offline, plane C, so its risk is isolated |
| 5 | **P4 Deeper worlds** | Full epistemics, plague, war, markets, culture, the Oracle with Foresight and whispers, the world compiler | Deeper pressure and agency once sagas are proven |
| 6 | **P5 2D diorama** | A glyph and CC0-tile view, a settlement growth view and a travel map (**glance**) | Ambient, cheap, and never needed to enjoy the saga |
| Later | Optional | 1M scale-up, M4 F2, era packs, the lo-fi street-level inhabitant | On your call |
| Cut | — | Every custom-3D, Godot and VR view | ADR-0035: you are not a gamer, and 3D was the largest content cost in the plan |

**The engine only engines** (ADR-0035 §6). The engine's only output is the World Stream and its only input is the journal. Every way of listening, reading, scrubbing or glancing is a separate tool downstream of it.

Out of scope until you decide otherwise (see `12_open_questions.md`): multiplayer and shared worlds, mobile apps, web distribution, and a commercial launch.

**Worlds are not stuck in time (ADR-0027).** Eras are *state*, not templates. A world that starts agrarian can pass through early-modern, industrial and modern conditions in one continuous run, because every era-sensitive mechanism is a function of technology and institutions, and institutions themselves change. Coverage grows through **era packs**. A world that reaches the edge of what the engine can model waits there, files a gap report, and continues when the next pack ships.

**Horizons** (R09 TL;DR 11, re-scoped by ADR-0027). A template sets the *starting* state, not an era lock. *Speculative* now means beyond the technological frontier: running past the last era pack, or forecasting a modern-start world beyond about 50 years. Passing *through* eras the engine supports is not speculative. A modern-start megacity still runs at day or week resolution for 1–50 years by default.

## 7. Thesis risks we accept, and how we will know early

| Thesis | Our position | Early test |
|---|---|---|
| Unforgiving physics makes drama. | **Necessary but not sufficient.** Emergent behaviour is not narrative (Walsh, Ryan; R02 §3). Drama needs (a) systems with *contingent unlocking*: actions that create follow-up opportunities, (b) knowledge asymmetry (secrets v0), and (c) curation. Vows and relics are contingent unlocking made explicit. | P2 gate: MOE-1 on 10 seeds (rated by you); MOE-6 on 32 seeds (automated); the saga-mechanics ablation reported. If it fails, fix the simulation and sifter before any illustrated work (P3). |
| Vows and relics make sagas. | **Plausible, untested.** They give actors persistent goals that override routine, which is what saga plots are made of. The risk is that they read as scripted (R-31). | G1b: V-SAGA-10 (every vow kind ends in more than one way). G2: the saga-mechanics ablation (V-SAGA-06, reported). |
| Grounding makes generated fiction authentic. | **Half true.** Grounding fixes continuity and plot holes, not prose quality (R08). | P3 gate (G3): zero canon contradictions plus your blind rating against an ungrounded baseline, and a season you listen to and read. |
| Generated panels can keep a character recognisable. | **Unproven.** Image models drift between panels. Deterministic visual DNA, a canonical character sheet and reference conditioning should help (`05.11`). | P3: the consistency report (V-SAGA-08) before it becomes a gate (R-32). |
| Local small LLMs can voice any citizen within their knowledge. | **Unproven.** Frontier models leak "future knowledge" 54% of the time on TimeChara (R04/R05). It needs architecture: plan-then-realise, belief-scoped context and claim checks. | Later and optional (WP P6.4): RP-200, a 200-scenario role-play eval run against our own belief store. |
| 1M entities with individual drama. | **Feasible in the macro model** as per-person ledger rows (R01: 19–64 MB, tens of ms per month). Individual *cognition* stays limited to 1–5k named actors. **Not needed for sagas:** the default world is a kingdom of 10k–50k with 200–2,000 named actors (ADR-0035). | P1b MOP-1 at meso; 1M only with the optional scale-up (P3.10). |

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
- **Input journal.** The canonical record of every nondeterministic input (manifest, settings, player actions, speech acts, Oracle verbs, translated LLM inputs), with its tick and scope. Together with the build, Registry version, manifest and seed it defines a history. Replay reads it and never regenerates it.
- **Material input.** A journaled input that changes plane A: a player action, a player speech act or an Oracle verb. It declares a **dependency scope** (person, household, settlement or region). Pending timers in that scope are cancelled and re-sampled.
- **Push input.** A journaled input that starts outside the simulation (you, the Oracle, settings, or an LLM translation of your own act) and applies at its tick. It is the only input class; replay hard-fails if one is missing, extra or different. Pull inputs are retired (ADR-0034).
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
- **Catalyst.** A manifest initial condition that sets up drama, such as dynastic tension or a contested artefact. It names a polity, a place or a role (or a declared cast member), and never forces an outcome.
- **Mythic catalyst.** A scenario seed (ADR-0035, extending ADR-0028 §3): an entity that awakens in a given year, a relic that passes to a named role, a prophecy announced to named actors. It is an initial condition or a scheduled, journaled material input; every consequence is simulated.
- **Overlay.** A setting-specific readout (Paranoia, Filial Piety, Despair…) derived from the generic psychological core by a Registry formula. The UI and the sifter can read it; the simulation runs on the core.

### 8.6 Minds and the Oracle
- **Thought.** A timed modifier on a mind: vector deltas, half-life, stacking rules and source. Whispers and omens land as Thoughts.
- **ArgumentFrame.** The typed parse of a whisper: appeals, target vectors, requested action, claims, references and a meta-instruction flag. It is grounded and scored deterministically. The whisper's text never reaches a mind.
- **Deliberation consult.** Retired by ADR-0034 (D27). It was an optional LLM choice from a legal-action menu for a named actor's decision. Named actors now decide only through the simulation.
- **Oracle.** Your in-world persona: a plane-A entity with its own belief store, fed by the same channels as everyone else. Its verbs are material inputs.
- **Cultivator mode / Oracle mode.** The console's two modes. **Cultivator** shows the true state, secrets included, with Oracle verbs disabled by default. **Oracle** filters every view, including Foresight, through what the Oracle knows. Switching is journaled.
- **Foresight (mechanic).** Monte-Carlo rollouts of forked timelines that estimate the probabilities of future outcomes. It is the Oracle's power and the engine's namesake. In Oracle mode, rollouts start from a belief-resampled state, so they never leak hidden truth.

### 8.7 Sagas and the chronicle pipeline (ADR-0035)
- **Saga.** A connected run of sifted arcs around a house, a vow or a relic: what the engine is for.
- **Artifact.** A unique, singular object in plane A (`S6.artifacts`): one bearer or one location at a time, a fixed-point pull on its bearer's mind (sent to the minds system as lifecycle commands), and a rumour that draws ambitious actors.
- **Vow, grudge, quest.** A persistent directional goal in plane A (`S7.vows`), such as `AVENGE_KIN(father, murderer)`. It adds a bounded priority to its vower's agenda, never to a survival decision, and ends fulfilled, broken or void (the vower died).
- **Secrets v0.** Minimal epistemics: who knows about a hidden act. Betrayal needs it.
- **Audio chronicle.** The chronicle as narrated episodes (P2), then as a multi-voice drama (P3), written by an offline exporter as local MP3 chapters and played in the local saga reader. Plane C.
- **Graphic chronicle.** The sifter's climaxes as four-panel storyboard scripts (staging, confrontation, climax, consequence) and then illustrated panels, each linked to its events. Plane C.
- **Cinematic event envelope.** The five groups of facts a chronicle-worthy event carries for the downstream tools: cast, staging, emotional vector, action and props, and the causal "why". Only simulated facts are stored in plane A (roles, place, weather and emotion snapshots, the instrument, typed motive edges); the rest is derived in plane C (`05.2` §4.1a).
- **Stress and mental break.** A named actor's fixed-point stress rises when a vow, kin loyalty or a value conflicts with the situation; above a threshold a mental break is a scheduled timer, and the actor commits a volatile act from the Registry break table (minds v0, P1a).
- **Visual DNA.** A named actor's look, `H(world_seed, person_id, "visual")` unpacked into locked tokens from the Registry's visual vocabulary. A pure plane-C function of plane-A facts (genealogy, age, scars, artifacts borne, regalia, health); it never consumes a plane-A random draw. Children inherit their parents' tokens with a keyed variation.
- **Information horizon.** Secrets v0's rule: a hidden act is known first only by its actor, accomplices and witnesses, and spreads only through scheduled outcomes (discovery, alarm, flight, news at travel speed, testimony). Nobody reacts before they know.
- **Legends reverse key.** Damage or transformation state stores its cause event inside the value (`Ruin { cause }`, `Scar { cause, site }`), so every change answers "why?".
- **Ontological truth and perception.** The engine simulates matter, space, time, state and the causal graph; downstream tools generate pixels, prompts, dialogue, sound and prose (ADR-0035 §5a).
- **The engine only engines.** Engine crates hold no presentation, media or LLM code. The engine's only output is the World Stream and its only input is the journal; every presentation tool is a separate consumer downstream (ADR-0035 §6).
