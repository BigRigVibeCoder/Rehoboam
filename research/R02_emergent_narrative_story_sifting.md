# R02: Emergent Narrative, Story Sifting, Drama Management and Narrative Significance

*Research dossier for Rehoboam, dated 2026-10-15.*

**How this was checked.** Partway through, the shared web-search budget ran out. arXiv, the ACM DL, AAAI, eScholarship and the conference sites were also unreachable from the sandbox. So most of the checking was done against primary artifacts that *were* reachable: GitHub and GitLab repos (license files, commit histories, source code), the authors' PDFs as hosted in their GitHub Pages repos, PyPI metadata, and the **full text of James Ryan's PhD thesis**, which ships as Markdown inside the Viv repo [1]. Claims carry one of three tags:

- *No tag:* verified this session against the cited artifact.
- **[S]:** only a search-engine snippet was seen, not the paper itself.
- **[U]:** background knowledge from before this session, not re-verified. Treat these as leads, not facts.

---

## TL;DR

1. **Curationism is the right frame. The draft's storage pipeline works against it.** Ryan's thesis argues that raw simulation traces "almost always lack story structure". Stories come from *curation*: sifting followed by narrativization [1]. The draft filters at creation time (`dramatic_weight ≥ 8`) and then deletes. That throws away the causal ancestry that sifting and hindsight depend on. **Verdict: redesign.**
2. **The draft's retention rule contradicts itself.** Only events with weight ≥ 8 are inserted, but the purge removes events with weight < 7. Unless weights are rewritten later, the purge never fires. Meanwhile the low-weight "minor slight" that later starts a war was never stored at all.
3. **The temporal bug is real and documented.** The Winnow paper shows that the same constraint means different things retrospectively and incrementally. Felt checks `?host value communalism` against the state *at the end* of the sequence; Winnow checks it at the time of the event [10]. `paranoia > 80` has the same flaw. Rules must read an **as-of-event snapshot**.
4. **The most important prior art is new: James Ryan's Viv (Sifty LLC, open beta, 2026).** It provides automatic causal bookkeeping (direct causes plus transitive ancestors), per-character memories with salience and forgetting, knowledge relaying, sifting patterns over `caused`/`triggered`/`preceded`, and setting-agnostic "tropes" [2–6]. **It is source-available under a non-commercial license, ships a JavaScript-only runtime, and is sized for games rather than 1M agents.** Treat it as a reference design, or buy a commercial license.
5. **"Cannibalizing" the cited projects is mostly unrealistic** (§Landscape, license table):
   - Felt and Winnow have **no license** (all rights reserved) and have been dormant since 2020 and 2021.
   - DF Storyteller is **AGPL-3.0**, dormant since 2022, and only parses DF 0.44/0.47 legends.
   - Ensemble is **BSD-4-Clause** and dormant since 2019.
   - Ceptre uses the **Anti-Capitalist Software License**, which excludes most for-profit organizations.
   - Bad News has **no public code**.
   - Talk of the Town is MIT but **Python 2** with 18 commits.

   They are reference designs only.
6. **Causality has to be recorded when an event happens, typed, and kept sparse.** Utility-based AI produces "massively distributed causality" that nobody can narrate. Ryan's attempt to extract explanations from Prom Week's utility sums failed [1]. Record *contingency* edges explicitly ("contingent unlocking") and keep weaker edges separately typed.
7. **Use hybrid sifting with no LLM in the detection path.** The tiers are: deterministic incremental pattern automata inside the sim, then batch retrospective Datalog/SQL over the chronicle, then an LLM that only re-ranks, explains and retells the top-K matched subgraphs. Winnow needed **912 ms per event** with a pool of 1,000 partial matches over a 5-character world [10], so naive incremental sifting will not scale. LLM-only sifting over roughly 10⁹–10¹¹ events is infeasible.
8. **Drama management: curate, don't intervene.** Hidden directors "kill the pleasure" of emergent narrative because events stop *really happening* [1]. Apply RimWorld/L4D-style tension curves to *presentation* (auto-pause, camera, arc selection), not to world events. Pressure should be diegetic, declared in the manifest and visible in the causal inspector.
9. **Significance must be retroactive.** Compute it repeatedly as consequences accumulate: rarity, consequence (weighted descendants), centrality, stakes, reversal, irony. Pin the contingency ancestors of any significant event; this is Labov's "narrative pre-construction" [1].
10. **Evaluate each pipeline stage separately.** Ryan's "cover-up" warning applies: human (or LLM) narration hides simulation failures [1]. Use blinded pairwise judgments against a random baseline [11], retelling studies, HANNA's six criteria [38], and LLM judges only for regression triage [39].

---

## Draft claims checked

| Draft claim | Verdict | Evidence |
|---|---|---|
| "Datalog story sifter inspired by Kreminski's Felt" | **Right lineage, wrong expectation of a usable engine.** | Felt is a 291-line JavaScript file over DataScript (Datalog), with a single commit (2020-08-14) and no LICENSE [7]. It is retrospective only. Incremental and partial sifting came later in Winnow [9][10]. |
| `tyrant_cleansing_order` when paranoia > 80 | **Temporal bug, and setting-specific.** | Retrospective Felt evaluation checks attribute constraints against end-of-sequence state, while incremental Winnow checks them at event time [10]. The rule also hard-codes a feudal vocabulary (see §Pattern libraries). |
| `dramatic_weight` 1–10 assigned at creation | **Wrong for historiography; fine for agent memory.** | This is exactly Generative Agents' "poignancy" score, rated 1–10 by an LLM at memory creation and combined with recency (0.99 decay) and relevance for *retrieval* [26]. Narrative significance is retroactive: Labov's pre-construction works backward from the "most reportable event" [1]. |
| Purge events with weight < 7 every 100 years | **Destructive and internally inconsistent.** | It breaks causal chains, destroys the base rates that rarity metrics need [11], and never fires on a table that only admits weight ≥ 8. |
| `causes_json` column | **Right instinct, wrong representation.** | Ryan's "causal bookkeeping" is the core technique [1]. Viv stores `causes`, `caused`, `ancestors` and `descendants` per action [6]. A JSON blob cannot be indexed and has no edge types. |
| Cannibalize df-storyteller's schema | **Borrow one idea, not code.** | The Rust DF Storyteller (GitLab) is AGPL-3.0-or-later. Last commit 2022-07-10; it supports DF 0.44.12 and 0.47.04 only; its version checker forces upgrades after about a year [20]. Its `HistoricalEvent` is a sparse struct with ~360 fields. The idea worth taking is **event collections** (`parent_hec_id`, `he_ids`: war → battle → event) [20]. The PyPI "df-storyteller" is an unrelated MIT LLM journaling app from 2026 [21]. |
| BadNews significance heuristics | **No such code exists.** | In Bad News, curation was done by a human "wizard" and actor [1]. Ryan reports that automatic sifting of Talk of the Town was "essentially impossible due to particular deficits in the *Talk of the Town* simulation" [1]. |
| Ensemble/CiF social rules | **Usable as a design reference, with a causality caveat.** | Ensemble is BSD-4-Clause (advertising clause), last commit 2019-12-17 [18]. Ryan found it limiting because every action needs at least two characters [5]. Prom Week's causality was "distributed over potentially thousands of social concerns" [1]. |
| Ceptre linear logic | **The idea is sound; the code cannot be used commercially.** | ACSL v1.4 restricts use to individuals, non-profits, educational institutions and worker co-ops [19]. It is written in Standard ML. Implement resource-consumption semantics natively. |
| LLMs limited to compiler, facade and synthesizer | **Endorse.** | Add one constrained offline job: re-rank and explain sift matches, and propose candidate patterns for human review (cf. Synthesifter [15]). |

---

## Landscape and state of the art

### 1. Story-sifting systems

| System | Date / version | License | Maturity | Fit for Rehoboam | Notes |
|---|---|---|---|---|---|
| Talk of the Town (Ryan) [17] | 2016; JSON export 2021; LICENSE added 2024 | MIT | 18 commits, Python 2 | Low | `StoryRecognizer` runs 8 hand-coded *current-state* queries (love triangles, rivalries) [17]. That is the same snapshot bug. |
| Bad News (Samuel, Ryan, Summerville 2016) [1] | Performance piece | No public code | n/a | Lessons only | Human wizard plus actor. Lesson: "augment but do not contradict" the simulation [1]. |
| Hennepin + Sheldon County (Ryan 2017–18) [1] | Thesis | Not public | Prototype | High (ideas) | "A few million transpired character actions" per county. Sifting patterns produce **nuggets** tagged with **narration moves**, plus a show bible [1]. Described in future tense ("once the project is released"); release not verified. |
| Felt [7][8] | ICIDS 2019; repo 2020 | **None** | 1 commit | Ideas | Sifting patterns double as action preconditions. DataScript's immutable database enables snapshot queries [8]. |
| Winnow [9][10] | AIIDE 2021 | **None** | 22 commits, last 2021-10 | Ideas | Incremental and partial matches; `unless-event` negation; pruning strategies [10]. |
| Synthesifter / StU [11][15] | 2020 / ICIDS 2022 | **None** (repos) [16] | Research | Heuristic | StU ranks matches by statistical unlikelihood. Raters preferred its pick in 38/45 pairwise judgments (84.4%), but with **only 3 raters** [11]. |
| Gossamer [13] | CoG 2023 | **None** | 6 commits | Knowledge model | Four gossip phases: witness, reflection, propagation, decay. Each character's memory is a DataScript database queried with Felt [13]. |
| Centrifuge (Johnson-Bey) [24] | 2021 | **None** | Dormant | Authoring UX | Node-graph editor that compiles to Talk of the Town sifting queries. |
| Neighborly (Johnson-Bey) [23] | v2.5.0 (2024-03) | MIT | **Unmaintained** (notice dated 2026-04-07) | Low | ECS settlement simulation; life events as story material. |
| Step-based sifters (Horswill) [25] | 2022 | n/a | Small | Ideas | Logic-programming sifting for Talk of the Town and City of Gangsters saves. |
| **Viv** (Ryan / Sifty) [2–6] | Open beta; 130 commits 2026-04 to 2026-09; components at 0.x | **Source-available, non-commercial** | 1000+ tests; JS runtime (C# planned) | **Best reference** | Causal bookkeeping, knowledge manager, sifting patterns, tropes, plans. Determinism is possible through a host-supplied `adapter.rng` [6]. Government-funded research counts as commercial [2]. |
| Recent academic work [S] | 2022–2026 | Unknown | Papers | Track | Leong et al., automated sifting (IJCAI-22) [45]. *Awash*: a prospective sifter feeding a drama manager (ICIDS 2023) [43]. Lyu, Millard and Gibbins: composable patterns with an incremental sifter on an alien-invasion simulation [42]. Lessard et al., "Narrative Gold Digging: Sifting and Narrating…" (FDG 2026) [44]. |

**LLM and sifting (2023–2026).** The evidence is thin and mostly [S]:

- A short paper tested ChatGPT as a sifter over the log of the Charade romance simulation [46]. An evolutionary (non-LLM) sifter was published on the same log [47].
- StoryBox (arXiv, Oct 2025) refines multi-agent simulation logs into long-form stories with an LLM [48].
- Peng et al. (2024) used GPT-4 to turn game logs into narrative node-graphs [49].
- Kreminski's own recent LLM work is about *authoring* and *storylets* rather than sifting. It includes Dramamancer (UIST 2025; MIT repo 2026), Drama Llama, and "Can LLMs Generate Good Stories? … Narrative Planning Perspective" (CoG 2025) [15].
- Most relevant to the Compiler role: PWIM uses a **local sentence-embedding model** to map free-text intents onto simulation actions [14].
- GitHub's `emergent-narrative` topic shows a 2025–26 hobbyist wave converging on Rehoboam's architecture, e.g. "deterministic rules + LLM narration" and "AI showrunner that discovers episodes". All have 0–94 stars [41]. This shows convergence, not solved problems.

**Inference:** nobody has published LLM sifting at scale with rigorous evaluation. LLMs are a plausible re-ranker or explainer on top of symbolic candidates, not a detector.

### 2. Precedents for drama management and curation

| System | Mechanism | Intervenes in the world? | Lesson for Rehoboam |
|---|---|---|---|
| **RimWorld** storytellers [30] | `OnOffCycle` threat scheduler (Cassandra: 4.6 days on / 6.0 days off, 1–2 big threats). "Adaptation days" fall when colonists die (28 days per colonist lost), which lowers threat points. A population-intent curve applies. | **Yes:** raids are spawned. | It is accepted because it is *named, chosen and diegetically framed* ("Cassandra Classic"). Copy the transparency, not the spawning. |
| **Left 4 Dead** AI Director [32] | Per-survivor intensity (decay 30 s), five impulse categories, tempo cycle `BUILD_UP → SUSTAIN_PEAK → PEAK_FADE → RELAX`, driven by maximum instantaneous intensity. | **Yes** (spawning). | Evidence is community reverse engineering of the 2026 binary, so treat it as secondary. The tension FSM is a good model for **presentation pacing**. |
| **CK3** [31] | `story_cycle`: persistent scripted storylines with `effect_group`s that fire at day/month/year intervals with `chance` and `trigger`. `secret` types: `on_discover`, `on_expose`, `is_shunned`, `is_criminal`. | Partly: authored storylets. | Production-scale **contingent unlocking** plus **who-knows** modeling. Hive infection is structurally a CK3 secret. |
| **Façade** (Mateas & Stern) [U][1] | Beat manager sequences beats (preconditions, tension effects) toward an Aristotelian arc. | **Yes.** | Ryan argues that intervention policies can be recast as **curation policies**: search over *observed* sequences [1]. |
| **Riedl** experience management; Weyhrauch; Thue's PaSSAGE [U][1] | Planning- or search-based drama managers; player modeling. | **Yes.** | Relevant only to the player-facing modalities (3 and 4). |
| **Wildermyth** [U] | Authored vignettes cast onto procedural heroes by role requirements; personality-variant dialogue. | Authored layer on top of the simulation. | Model for the **Synthesizer**: an authored *discourse* layer over simulated *story*. |
| **Caves of Qud** [8][1] | Random biography events, then sifting patterns that *retroactively justify* them, inventing new facts when needed (Grinblat & Bucklew, PCG 2017). | Rewrites history. | Never use this for ground truth. It is excellent for **in-world unreliable chroniclers** (propaganda, hive-faction myths) layered over the true chronicle. |
| **Dwarf Fortress legends** [20][22][1] | XML legends, event collections, no general causal links. Humans curate: Oilfurnace comics, Legends Viewer browsing, Dwarf Grandpa auto-biographies [8]. | No. | DF is the "cornucopian simulation" paragon, but stories arise through *mental curation* by players [1]. LegendsViewer (GPL-3, 2012–2024) has been rebooted as LegendsViewer-Next (MIT, v1.3.0, .NET 10/Vue 3, active 2026) for the Steam release [22]. |
| **Versu** (Evans & Short 2014) [13][1] | Social practices; role evaluations ("bad brother") with reasons, which can be gossiped. Only one evaluation per character–character–role triple. | No. | Its evaluation and gossip model fits "who knows what". Its UI showed every action as a button, which overwhelmed players [14]. |

### 3. Theory: why logs are boring and what makes them compelling

- **Emergent behavior is not narrative.** Walsh: a simulation run "does not in itself produce emergent narrative but instead emergent behavior … that we as observers may be inclined to narrativize" [1] (quoting Walsh 2011). Aylett (1999) coined the academic usage, followed by Louchart and Aylett's programme and FearNot! [1].
- **Ryan's catalogue of pains** [1]:
  - *Simulation pains:* boringness, granularity extremes, low modularity, lack of abstraction, modeling gaps, causality issues.
  - *Curation pains:* no telling, no curation, poor curation, poor presentation, failure to "mount" the story in a media experience.
  - Wardrip-Fruin's "Tale-Spin effect" belongs here: rich internal processes that the audience never sees [1]. It directly threatens the hive-mind faction's covert minds.
- **What reliably works:**
  - *Tellability.* Labov's reportable events: death and injury are always reportable [1]. Schank's absolute interests [1][11].
  - *Unrealized projections.* Marie-Laure Ryan notes that deception, broken promises, violated interdictions and mistaken interpretation make better tales [1]. These map directly onto infiltrator and hive mechanics.
  - *Kernels versus satellites* (Barthes and Chatman) [1].
  - *Contingency.* Powell's three causal kinds are possibility, probability and contingency; only contingency reads as "X caused Y" [1].
  - *Storyline interlocking.* Two independently emergent revenge-arson chains collided in Hennepin [1].
- **Drama manager or not?** Ryan's position is "fiercely emergentist … strictly non-interventionist": intervention "kills the pleasure" because events no longer actually happen [1]. Instead, *bad runs are authoring bugs*. Build a "cornucopian simulation" that reliably produces intrigue, using **contingent unlocking**: actions that unlock specific follow-up actions, as in Klein's 1970s murder-mystery story registers [1].

### 4. Narrative significance metrics

| Metric | Operationalization | When computed | Notes |
|---|---|---|---|
| Inherent tellability | Prior per act class (death, betrayal, coronation, infection) | At creation | Schank/Labov [1]. The only part the draft's weight captures. |
| Rarity / surprise | StU: mean likelihood of a match's properties (event types, tags, traits) across matches of the same pattern [11] | Retroactive, per pattern | Fails on *correlated* rarities (a "villain kicks puppy" pair scores as doubly rare) [11]. Condition on actor traits. Needs **retained base-rate counts**. |
| Consequence | Significance-weighted count of causal descendants, i.e. PageRank on the reversed causal DAG | **Retroactive, recomputed per era** | This is the "minor slight → war" metric. |
| Actor centrality | Degree and betweenness in the kin, ally and faction graph; "notable figure" status | Periodic | DF's "historical figure" promotion is the precedent [20]. |
| Stakes | Change in an actor's basic needs (life, health, wealth, love, membership, ego) | At creation, from a state diff | Elson's story intention graph: tellable content *affects* basic needs [1]. |
| Reversal | Sign change in status or affect within a window (Lehnert's "loss", "fleeting success") [U] | Retroactive, windowed | |
| Irony | Knowledge asymmetry: an audience-visible truth not known to an affected character | Retroactive, from the knowledge table | Requires a witness and knowledge model (see §6). The core metric for the hive faction. |
| Suspense | Open partial matches (Winnow) whose completion would be high-stakes [10] | Live | Drives Time-Machine auto-pause *before* the climax. |
| Kernel-ness | Lies on a contingency path to a high-significance anchor | Retroactive | Barthes/Chatman kernels [1]. |

**Hindsight policy.** Significance S(e) is a function of time, S(e, t). Store the components, not a frozen scalar. When an anchor event exceeds a threshold, walk *contingency* edges backward until reaching an event that "needs no explanation". That is Labov's pre-construction, operationalized by Ryan as backward chaining from the "most reportable event" [1]. **Pin** everything on that path.

### 5. Setting-agnostic pattern libraries

- **Role-based typologies.**
  - Propp: 31 functions and 7 spheres of action.
  - Polti: 36 situations, each a role tuple such as persecutor, suppliant and power-in-authority [1][U].
  - Lehnert's plot units (1981) [U]: affect states +, −, M (mental) linked by motivation, actualization, termination and equivalence. They are setting-neutral by construction, and complex units (retaliation, double-cross, honored or denied request) are subgraph patterns.
  - Rumelhart's story grammars [1]. Ryan notes that nested sifting patterns *are* story grammars used for retrieval [1].
  - Viv's **tropes** are named relational patterns over roles (`rivalry: @hero.opinion[@villain] < -50`) [4].
- **TV Tropes** content is reported as CC BY-NC-SA 3.0 (non-commercial) [40]. Use the trope *names and concepts* as an index; do not ingest the text.
- **How to make patterns portable.** Write patterns over an **abstract layer** that the world-manifest compiler maps from concrete verbs:
  - *Act classes:* harm, aid, promise, betray, coerce, conceal, reveal, infect/convert, succeed-to, flee.
  - *Abstract roles:* authority, successor, kin, ally, rival, subordinate, member(faction).
  - *Affect:* Lehnert-style.
  - Example: `authority_purges_successor(A, S)` holds when a coerce or harm event by A targets S, `succession_rank(S, A) = 1` **at the event**, and the event has a `motivated_by` edge to a fear-type state of A **at the event**. It matches king and heir, mayor and deputy, or hive-queen and rival node.
  - Example: `trust_then_betrayal(A, B)` is an aid or promise event (A→B), followed later by a harm or deceive event (A→B), with `trust(B, A) @e1 > θ` and, optionally, B learning of the second event. It covers fealty-then-poisoning and colleague-shares-badge-then-infects.
  - *Composition:* when a pattern completes, emit a `match` event into the chronicle so that higher-level patterns can consume it [10]. Lyu et al.'s composable building blocks take the same approach [42].

### 6. Event ontology and causality

Two facts from Ryan's thesis underpin the design.

**Contingency edges in Hennepin are recorded when** [1]:
- (a) an action casts a past action in a role (gossip *about* X), or
- (b) an action's effects queue a follow-up action that is later performed.

**Viv generalizes this to five edge sources** [3]:
- (1) action roles,
- (2) reactions,
- (3) knowledge relaying,
- (4) inscriptions on items,
- (5) host-forced causes.

**Knowledge in Viv** works like this: everyone present forms a memory with *per-character* salience and associations. Hearers form post-hoc memories via `@hearer`, and memories fade below a threshold [3]. Hennepin dropped memory fallibility because Talk of the Town's mutation model imposed a "massive authorial burden" [1].

Viv's `ActionView` stores `causes`, `caused`, `ancestors`, `descendants`, `relayedActions`, `initiator`, `partners`, `recipients`, `bystanders`, `present` and `importance` [6]. At game scale that is fine. **Storing full ancestor and descendant sets is O(n²) at Rehoboam scale (inference).** Store direct edges and compute closures on demand.

**The temporal bug, precisely.** `tyrant_cleansing_order(Sovereign, Heir) :- paranoia(Sovereign) > 80, …` evaluated during a later sift (or a Time-Machine scrub) reads paranoia *now*. If the king has died or calmed down, true instances are missed. If his paranoia rose later, false instances appear. The result also depends on when sifting runs, which breaks determinism. `mimic_sabotage_observed(Infiltrator, Sibling)` additionally needs the *witness set at that event* and the sibling's *knowledge*, not omniscient truth.

There are three fixes:
- (i) Every rule emits an `event_snapshot` of the predicates it read.
- (ii) Traits get a valid-time history (`valid_from`, `valid_to`), so retrospective queries use `AS OF e.tick`. Felt's DataScript snapshots illustrate the idea [8].
- (iii) The pattern DSL forbids un-anchored state reads (`paranoia@e1`, never bare `paranoia`).

### 7. Simulation-to-story pipelines and Showrunner

- **SHOW-1** (Maas, Carey, Wheeler, Saatchi, Billington, Shamash; Fable Studio, July 2023) [29]. From recollection, its content is [U]:
  - A South Park–style town simulation supplies characters' daily events as context.
  - An LLM "showrunner" chains prompts into scenes, then image and voice generation.
  - The paper discusses the "slot machine effect" and the "10,000 bowls of oatmeal" problem.
  - It reports **no rigorous user evaluation**.
- **Fable's public code is dormant.** SAGA (MIT) and the Thistle Gulch bridge (**non-commercial** license) were both last updated 2024-05-22 [28]. Reports of a 2025 consumer Showrunner launch are [U].
- **What transfers:** the simulation is the *context provider*; an LLM writes *discourse* over it; a human or agent selects. This matches Rehoboam's Synthesizer.
- **What did not transfer:** grounding guarantees. Ryan's Sheldon County design is more disciplined. Its pipeline is: sift into nuggets, then narration moves, then grammar-based scripts with precondition tags, then title and narration proposals plus a show bible across episodes [1]. Rehoboam's Synthesizer should keep that skeleton and swap the grammar for an LLM constrained to the nugget.
- **Generative Agents** (Apache-2.0, dormant since 2023-08) [26] and **Project Sid** (a technical report only) [27] show LLM-agent societies. They are not significance or sifting systems.

### 8. Engine options for sifting and chronicle queries

| Engine | Version / date | License | Status | Fit |
|---|---|---|---|---|
| DuckDB [37] | v1.5.6 | MIT | Active | **Chronicle store.** Recursive CTEs incl. `USING KEY` handle ancestor/descendant walks; 1.5 deprecates the `UNION` form. |
| Soufflé [35] | 2.5; commit 2026-10 | UPL-1.0 | Active | Batch retrospective Datalog. |
| Feldera / DBSP [34] | v0.366.0 | MIT (open-source edition) plus enterprise | Very active | Incremental SQL if you need it; open-core caveat. |
| differential-dataflow / Ascent / Nemo / Scallop | 2026 commits | MIT, MIT, Apache/MIT, MIT | Active | Embeddable Rust options. |
| DDlog [33] | Last commit 2022-11 | MIT | **Archived 2026-07-13** | Avoid. |
| DataScript [36] | 1.8.1 | EPL-1.0 | Active | Felt and Winnow's backend; JS/Clojure only. |

---

## Recommendations for Rehoboam

### R1. Event ontology (normalized, append-only)

```
event(event_id, tick, era, verb, act_class[], location, magnitude,
      rule_id, rule_version, rng_stream, visibility{public|faction|secret})
event_role(event_id, entity_id, concrete_role, abstract_role)   -- initiator/target/partner/bystander/instrument
event_snapshot(event_id, entity_id, predicate, value)           -- exactly what the rule read (paranoia=87)
causal_edge(cause_id, effect_id, kind, weight, evidence)
   kind ∈ {contingent, triggered(queued follow-up), relayed_knowledge,
           motivated_by(state/affect), enabled(precondition), probabilistic,
           player, director}
knowledge(entity_id, event_id, acquired_tick, via_event_id, fidelity, variant_id, salience)
collection(collection_id, parent_id, type, members)             -- DF-style episode/war/plague
sig_component(event_id, component, value, computed_at_tick)     -- retroactive significance
sift_match(match_id, pattern_id, pattern_version, bindings, event_ids, status, opened_tick, closed_tick)
```

Notes on the schema:
- **Aggregate mass events.** A famine killing 40,000 people is one event with counts and distributions, not 40,000 edges.
- **Per-agent memory only for notable events.** Use `knowledge` rows only for notable events and notable figures. Crowd knowledge should be diffusion fields (inference, needed for 1M agents).
- **Two salience layers.** Global narrative significance serves the narrator. Per-character salience serves the Facade, so NPC dialogue stays within what that NPC knows. Viv models both levels (`importance` plus per-character `saliences`) [6].

### R2. Causality model

- **Contingency.** Emit contingency edges *only* from explicit unlocking: an action referencing a past action, a queued follow-up, a knowledge relay, or a player or Oracle action. That is Ryan and Viv's bookkeeping rule set [1][3].
- **Utility decisions.** Log the top contributors plus the margin over the runner-up as `motivated_by` edges, but **do not** treat them as contingency. Ryan's Prom Week experiment shows that utility sums rarely give readable causes [1].
- **Authoring rule.** Author **contingent unlocking** generously (succession crises unlock purges; an observed infection unlocks a reveal or a cover-up). It is how a simulation becomes narratable without intervention [1].

### R3. Sifting: a three-tier hybrid

1. **Live (in the simulation, deterministic, Rust).** Write a Winnow-like DSL and compile it to partial-match automata, indexed by `(act_class, abstract_role binding)`. Prune matches that are inactive for K events, matches whose participants died (unless the pattern tolerates death), and use "advance directly" once roles are bound [10]. Feed matches back into NPC behavior where wanted; Ryan calls this feedback curation [1]. Budget: microseconds per notable event, measured on synthetic 1M-agent traces before committing.
2. **Retrospective (batch, per era or on demand).** Run Datalog or recursive SQL over the chronicle (DuckDB or Soufflé) for hindsight patterns and significance recomputation.
3. **LLM (offline, non-authoritative).** Re-rank and explain the top-K match subgraphs, propose new patterns as code for human review, and narrate. It never writes to the chronicle.

Version patterns and sifter builds per world, so replays are reproducible.

### R4. Drama-management policy

- **No hidden director inside the simulation.** Pacing lives in *presentation*: Time-Machine auto-pause thresholds use suspense and significance, the Diorama camera picks salient threads, and the Narrative Compiler selects arcs. These are intervention policies recast as curation [1].
- **Pressure must be diegetic and declared.** Climate cycles, succession law and hive growth rates go in the manifest, use seeded RNG, and appear in the causal inspector.
- **Player actions are causes too.** Oracle and Foresight actions are `player` edges. Foresight effectively *is* a drama-management budget, which keeps intervention honest.
- **Optional storyteller modes only, and labeled.** RimWorld-style modes may exist for the game modalities (3 and 4) only if they are named and opt-in, logged as `director` edges, and off in "pure" runs.
- **Treat bad runs as bugs.** Track a "bad-run rate": the fraction of seeds with fewer than N completed arcs per century [1].

### R5. Retention and compaction

- **T0, raw.** Keep a ring buffer of recent ticks plus **deterministic replay**: keyframes every K years, inputs and seeds, and a pinned engine version. Detail can then be *regenerated* rather than stored (inference; it requires bit-exact determinism across platforms).
- **T1, chronicle.** Keep every event that passes a cheap notability gate: a tellable act class, a notable participant, or rarity. Keep all its direct edges.
- **Recompute S(e, t) each era**, using all metrics in §4.
- **Pin events.** Pin the contingency and `motivated_by` ancestors of any event with S ≥ θ, all events in completed matches, and secret/reveal pairs. Pinned events are never deleted.
- **Compaction.** Fold unpinned old events into `collection` summary nodes with counts and distributions, so rarity base rates survive [11]. Rewire edges through the summary node. Never purge by a weight fixed at creation time.

### R6. Narrative evaluation

1. **Sifter.** Run blinded pairwise preference against a random baseline, as StU did [11], but with 30 or more raters. Also measure precision@K against human-labeled "story-worthy" sets and run retelling studies (Kreminski et al. 2019) [15].
2. **Synthesizer fidelity.** Extract claims from the output and check them against the chronicle. Report contradiction and invented-cause rates ("augment but do not contradict") [1].
3. **Prose.** Use HANNA's six criteria: relevance, coherence, empathy, surprise, engagement, complexity [38]. LLM-as-judge is for regression triage only: pairwise plus rubric, swapped positions, length control and a judge from a different model family (EQ-Bench v3 practice) [39].
4. **Structural metrics per seed.** Track arc count, causal depth, protagonist continuity, reversals, irony instances and bad-run rate.

---

## Risks and unknowns

- **Scale is unproven everywhere.** The largest documented chronicle is Hennepin's "few million actions" [1]. Winnow was benchmarked on 5 characters [10]. Rehoboam's 10⁹–10¹¹ raw events is 3–5 orders of magnitude beyond any published sifter (inference).
- **Causal hygiene versus emergent richness.** Too few contingency edges make the world unnarratable; too many recreate the dense lattice [1].
- **Knowledge-model cost at 1M agents.** Talk of the Town-style belief mutation was abandoned for its authoring cost [1].
- **Licensing.** Viv is non-commercial. Felt, Winnow, StU, Gossamer and WAWLT have no license. Ceptre uses ACSL. DF Storyteller is AGPL [2][7][9][16][19][20].
- **LLM "cover-up".** Fluent prose hides simulation gaps and invents causes [1]. Gate every synthesizer claim on chronicle evidence.
- **Unverified this session** (the [U] items): SHOW-1's content and Showrunner's commercial status, Façade, Riedl and Wildermyth specifics, details of Lehnert, Polti and Propp, and the post-2023 LLM-sifting papers [42–49] beyond their titles and venues.

---

## Sources

1. https://github.com/siftystudio/viv/blob/main/docs/.llm/curating_simulated_storyworlds.md: Ryan, *Curating Simulated Storyworlds* (2018), full text read. Canonical copy: https://escholarship.org/uc/item/1340j5h2
2. https://github.com/siftystudio/viv: Viv README and LICENSE.txt. Open beta; non-commercial; Sifty LLC.
3. https://github.com/siftystudio/viv/blob/main/docs/reference/language/20-runtime-model.mdx: causal-bookkeeping rules and knowledge manager.
4. https://github.com/siftystudio/viv/tree/main/docs/reference/language: sifting patterns (§16) and tropes (§14).
5. https://github.com/siftystudio/viv/blob/main/docs/background/history-of-viv.md: World, Talk of the Town, Hennepin, Esper and Viv; Ensemble limitations.
6. https://github.com/siftystudio/viv/blob/main/runtimes/js/src/adapter/types.ts: `ActionView` fields; seedable `rng`.
7. https://github.com/mkremins/felt: 1 commit (2020); no LICENSE; DataScript.
8. https://github.com/mkremins/mkremins.github.io/blob/master/publications/Felt_SimpleStorySifter.pdf: Felt paper (ICIDS 2019); descriptions of Qud, Dwarf Grandpa and Sheldon.
9. https://github.com/mkremins/winnow: 22 commits; last 2021-10-05; no LICENSE.
10. https://github.com/mkremins/mkremins.github.io/blob/master/publications/Winnow_AIIDE2021.pdf: incremental sifting; 912 ms/event benchmark; temporal semantics; pruning.
11. https://github.com/mkremins/mkremins.github.io/blob/master/publications/StU_ICIDS2022.pdf: "Select the Unexpected"; 38/45 agreement with 3 raters; correlated-rarity limitation.
12. https://github.com/mkremins/mkremins.github.io/blob/master/publications/AuthoringSifters_TAP.pdf: authoring burden; sifting heuristics; a sifting-based drama manager proposed.
13. https://github.com/mkremins/mkremins.github.io/blob/master/publications/Gossamer_CoG2023.pdf: gossip phases; comparison with Versu, Kismet and Talk of the Town.
14. https://github.com/mkremins/mkremins.github.io/blob/master/publications/PWIM_FDG2024.pdf: embedding-based mapping of text intents onto simulation actions.
15. https://github.com/mkremins/mkremins.github.io/blob/master/_data/publications.json: Kreminski bibliography (Synthesifter, retellings evaluation, Dramamancer, CoG 2025).
16. https://github.com/mkremins/statistical-sifting, https://github.com/ItsProbablyFine/WAWLT, https://github.com/mkremins/gossamer: no LICENSE files.
17. https://github.com/james-owen-ryan/talktown: MIT (2024); Python 2; `drama.py` StoryRecognizer.
18. https://github.com/ensemble-engine/ensemble: BSD-4-Clause (UC); last commit 2019-12-17.
19. https://github.com/chrisamaphone/interactive-lp: Ceptre; Anti-Capitalist Software License v1.4.
20. https://gitlab.com/df_storyteller/df-storyteller: AGPL-3.0+; Rust and Postgres; last commit 2022-07-10; DF 0.44/0.47 only.
21. https://pypi.org/project/df-storyteller/ and https://github.com/Been012/df-storyteller: unrelated 2026 MIT LLM companion app.
22. https://github.com/Kromtec/LegendsViewer (GPL-3, ends 2024) and https://github.com/Kromtec/LegendsViewer-Next (MIT, v1.3.0, active).
23. https://github.com/ShiJbey/neighborly: MIT; unmaintained since 2026-04-07.
24. https://github.com/ShiJbey/centrifuge: visual sifting-pattern editor (2021); no license.
25. https://github.com/ianhorswill/TotT and https://github.com/ianhorswill/CogSifter: Step-based sifters.
26. https://github.com/joonspk-research/generative_agents: poignancy prompt (1–10) and retrieval weights; Apache-2.0.
27. https://github.com/altera-al/project-sid: technical report only (arXiv 2411.00114).
28. https://github.com/fablestudio/fable-saga and https://github.com/fablestudio/thistle-gulch: MIT and non-commercial respectively; both dormant since 2024-05.
29. https://fablestudio.github.io/showrunner-agents/: SHOW-1 project page (title, authors and date confirmed through citation lists; text not re-read).
30. https://github.com/RimWorld-zh/RimWorld-Core/blob/master/Core/Defs/Storyteller/Storytellers.xml: mirror of the storyteller defs (Ludeon data; version unknown).
31. https://github.com/jesec/ck3-mod-base (`common/story_cycles/_story_cycles.info`, `common/secret_types/_secret_types.info`): CK3 script docs mirror.
32. https://github.com/zeljkovranjes/l4d2-director-system-research: reverse-engineered AI Director tempo and intensity (secondary).
33. https://github.com/vmware/differential-datalog: archived 2026-07-13.
34. https://github.com/feldera/feldera: DBSP incremental SQL; MIT open-source edition.
35. https://github.com/souffle-lang/souffle: UPL-1.0; release 2.5.
36. https://github.com/tonsky/datascript: EPL-1.0; 1.8.1.
37. https://github.com/duckdb/duckdb-web/blob/main/docs/preview/sql/query_syntax/with.md: recursive CTE `USING KEY`. Releases: https://github.com/duckdb/duckdb (v1.5.6 tag).
38. https://github.com/dig-team/hanna-benchmark-asg: HANNA (COLING 2022 / TACL 2024); 6 criteria; 19,008 annotations.
39. https://github.com/EQ-bench/creative-writing-bench: rubric plus pairwise Glicko LLM judging, with bias mitigations.
40. https://github.com/bocoup/stereotropes-data-public: states that TV Tropes data is CC BY-NC-SA 3.0.
41. https://github.com/topics/story-sifting and https://github.com/topics/emergent-narrative: ecosystem snapshot, October 2026.
42. https://eprints.soton.ac.uk/502632/: Lyu, Millard, Gibbins, composable story-sifting patterns [S].
43. https://dl.acm.org/doi/10.1007/978-3-031-47655-6_12: *Awash*, prospective sifting feeding a drama manager [S].
44. https://researchr.org/publication/LessardFPK26: Lessard et al., "Narrative Gold Digging" (FDG 2026) [S].
45. https://www.ijcai.org/proceedings/2022/0686.pdf: Leong, Porteous, Thangarajah, automated sifting (IJCAI-22) [S].
46. https://www.researchgate.net/publication/379381055_Using_ChatGPT_for_Story_Sifting_in_Narrative_Generation: ChatGPT as sifter [S].
47. https://www.researchgate.net/publication/379414785_Evolutionary_Story_Sifting_over_the_Log_of_a_Social_Simulation: evolutionary sifting on Charade [S].
48. https://arxiv.org/abs/2510.11618: StoryBox, multi-agent simulation logs refined into long-form stories (Oct 2025) [S].
49. https://arxiv.org/abs/2404.17027: Player-driven emergence; GPT-4 turns game logs into narrative graphs [S].
50. https://emshort.blog/2019/05/21/curating-simulated-storyworlds-james-ryan/: Emily Short's chapter-by-chapter commentary on Ryan's thesis [S].
