# R01: Simulation LoD, consistent "hydration" of individuals, and multi-resolution modeling for Rehoboam

*Research dossier, 2026-10-15. Scope: prior art on simulation level of detail (LoD) and on-demand materialization, the multi-resolution modeling (MRM) consistency problem, the supporting techniques, a check of the draft's claims, and a recommended tier design.*

**Evidence labels used throughout.**
- **[V]**: confirmed this session from a primary or near-primary source (search-result abstract, GDC Vault listing, official docs, GitHub or PyPI metadata).
- **[X]**: measured this session with local benchmarks. Setup: 4-vCPU Xeon @ 2.1 GHz container, Python 3.13, NumPy 2.5.4, SciPy 1.18.1, numba 0.68.0, SQLite 3.45.1. A modern desktop CPU is likely 1.5–2.5x faster single-threaded.
- **[R]**: recalled from prior knowledge and **not re-verified this session**. The shared web-search quota ran out mid-session, and the sandbox's fetch path was blocked for most hosts (gameaipro.com, arxiv.org, rand.org, epicgames.com). Treat [R] items as leads to confirm, not as settled facts.
- **Inference:** marks my own analysis.

---

## TL;DR

1. **The core idea is sound but not novel.** "Schrödinger LoD" is close to Sunshine-Hill's alibi generation and LOD Trader [1–4], Brom's simulation LoD [5,6] and Ubisoft's Census "uprez" model [8–10]. Borrow those designs.
2. **Correction: store people as rows, not cohorts.** Every person can be a row in a struct-of-arrays at 19–64 B each, ≤64 MB for 1M [X]. One monthly demography and contagion step over 1M rows took 70 ms in NumPy and 38 ms in numba, or 8–14 minutes per 1,000 years [X]. The proposed cohort tensor has about 5.4 billion bins for 1M people [X].
3. **Per-person rows remove the hardest MRM problem.** Hydration then only adds *detail* on top of existing facts. In MRM terms this is "pseudo-disaggregation" [R].
4. **Use counter-based or hash RNG for *everything*.** With a sequential stream, hydrating one NPC changes the world's future, so the camera path becomes a hidden input. Keyed hashing hydrated 200 entities × 12 fields in 0.43 ms, order-independently [X].
5. **"Compressed bitmasks": misleading.** Use narrow-dtype columns, with bitsets only for flags.
6. **SQLite spatial index: reject for live use.** Updating 5k movers took 378 ms per tick, against 55 µs per query for an in-memory grid hash [X].
7. **100 m / 50–200 is an interaction radius, not a visibility radius.** A human is about 17 px tall at 100 m and a dragon 16–33 px at 1–2 km [X]. Shipped games separate the two: AC Unity ran 40 full AIs inside a 10,000-NPC crowd [13,14]. Use 4–5 rings.
8. **Python plus Flecs: there is no maintained Python binding** [56,57]. Pure-Python utility scoring for 5k actors ran at 43.7 ms per tick; NumPy at 6 ms [X]. Go vectorized and event-driven at 0.2–2 decisions/s, not 30 Hz.
9. **Pinning is the most important policy.** Despawn-on-look-away is the most criticized failure [19]. Census and Dwarf Fortress pin on engagement or notability [9,25].
10. **Reconcile time with ownership transfer plus lookahead decrees**, with material bubble events event-sourced back into the macro sim [R, HLA/PDES; Inference].

---

## Draft claims checked

| Draft claim | Verdict | Evidence |
|---|---|---|
| Stratum 1 = 512×512 NumPy grids + Poisson chunk demographics for 1M people | **Revise.** Keep the grids for fields (resources, terrain, contagion pressure). Replace Poisson chunk *people* with per-person rows. | 1M people × 8 narrow columns = 19 MB; monthly step 70 ms (NumPy) and 38 ms (numba, 4 cores) [X]. A dense cohort tensor would have 5.37e9 bins (21 GB as int32), mean occupancy 0.0002 [X]. Aggregate-only people make consistent disaggregation stochastic and fragile (the MRM literature's mapping and temporal inconsistency) [27,29]. |
| "Entities exist as compressed bitmasks" | **Partly wrong.** | Bitsets suit flags and set queries (pyroaring 1.2.0, MIT [V]). Identity needs typed fields (age, cell, household, faction, health, birth tick). Narrow dtypes already give about 20–64 B/person [X], so compression buys nothing at 1M. |
| SQLite spatial index for hydration queries | **Reject for live use; keep for persistence.** | R*Tree query 0.49 ms vs. grid hash 0.055 ms vs. cKDTree 0.031 ms. Moving 5k actors: 378 ms per tick. 1M build: 9.3 s. Float32 coordinate storage produced 4 boundary mismatches in 143 queries [X; float32 storage per SQLite docs, 54, R]. |
| 100 m radius, 50–200 hydrated entities | **Adequate for interaction, inadequate for vistas.** | Outdoor density of 0.005/m² gives about 157 people in 100 m; a busy market at 0.02/m² gives about 628, with about 5 crossings per second at the edge [X, flux model]. In a synthetic clustered town, town-centred 100 m queries returned a median of 322 and a p95 of 2,498 residents [X]. AC Unity: 40 full AIs, 120 hi-res, up to 10k crowd [13–15]. City Sample: rigged near, VAT far [21,22]. A human is about 17 px tall at 100 m [X]. |
| Python/Flecs ECS at 10–30 Hz for 1–5k named actors | **Flecs-from-Python: no. Python at 10–30 Hz per entity: no. Vectorized Python at event-driven rates: yes.** | Flecs is C/C++, active, 8.7k stars [55]. No maintained Python binding [56,57]. esper 3.9 (Sep 2026) is a viable pure-Python ECS [58]: trivial iteration over 5k entities costs 0.25 ms, but logic bodies dominate. Pure-Python scoring: 43.7 ms per tick; NumPy: 6 ms [X]. GOAP planning (F.E.A.R.-style A* over actions [60, R]) per actor at 10 Hz for 5k actors is not credible in Python. |
| Utility AI/GOAP for Stratum 2 | **Keep utility AI; restrict GOAP.** | Plan only on salient events and cache or share plans per role (Inference). |
| Felt-style Datalog sifter | Out of scope here (see the sifter dossier). | `mkremins/felt` is JavaScript with 23 stars [53, V]; treat it as a reference design, not a dependency. |

---

## Landscape / state of the art

### 1. Simulation-LoD research lineage

| Work | Date | Licence / availability | Maturity | Fit for Rehoboam | Notes |
|---|---|---|---|---|---|
| Sunshine-Hill, *Perceptually Driven Simulation* (PhD) [1]; with Badler, *Alibi Generation* (AIIDE) [2] | 2010–11 | Open access | Research | **Very high** | Retroactively elaborates an agent's behaviour so it survives prolonged scrutiny; cheap pedestrians stay perceptually and statistically indistinguishable from the full simulation [V]. |
| *LOD Trader*, Game AI Pro ch. 14 [3]; MIG 2013 [4] | 2013 | Free PDF; ACM | Shipped-adjacent | **High** | Per-frame optimization of every agent's LoD to minimize the probability of a "Break in Realism" under a CPU budget, in tens of µs. The author calls it approximate [V]. |
| Brom, Šerý, Poch, *Simulation LoD for Virtual Humans* (IVA, LNAI 4722) [5]; SOFSEM 2006 [6] | 2006–07 | Author PDFs | Prototype (IVE) | **High** | Gradual, multi-degree simplification of space (hierarchical areas) and behaviour where unobserved [V]. |
| Navarro et al., dynamic LoD for urban ABM (AAMAS) [31]; Morvan's multi-level ABM survey [32]; TCD thesis [7] | 2011–16 | Mixed | Research | Medium | Aggregates agents into mesoscopic groups and back; taxonomies of micro/meso/macro coupling [R; 7 V]. |

**Sunshine-Hill's key lesson.** Consistency is judged *perceptually and statistically*, not by exact equivalence. A low-detail agent may have no specific destination. When the player starts scrutinizing it, the system samples a destination or errand *conditioned on everything the player has already seen it do*: its "alibi" [1,2]. That is exactly what Rehoboam's crowd-to-hydrated promotion needs.

**The LOD Trader's lesson.** A fixed radius is the naive policy. Better is to allocate a fixed number of detail "slots" each frame to the agents where a break in realism is most likely to be noticed [3,4]. Inference: implement the radius first, then add a greedy BIR-weighted allocator in v2.

### 2. Shipped population systems

| System | Year | Scale (verified where marked) | LoD / identity model | Lesson for Rehoboam |
|---|---|---|---|---|
| *Watch Dogs: Legion*, **Census** [8–10] | 2020; GDC 2021 | "Millions" of playable Londoners [10]; an *active collection of hundreds*, with unengaged citizens cycled out [9] [V] | A procedural layer generates consistent demographic profiles; a simulation layer makes characters persistent and generates schedules and relationships when "uprezzed". Hocking: a relational database inferring job, income and neighbourhood from each other [V]. | **Closest analogue to hydration.** Generate lazily, constrain by the database, pin on engagement, evict the rest. |
| *Shadows of Doubt* [11,12] | 2023–24 | "100s" of citizens, all simulated [V] | All persistent. Routines precomputed before each day (≤10–15 s); high AI update rate only near the player [V]. | Full persistence works at hundreds, not 1M. Daily precompute is cheap and deterministic. |
| *AC Unity* "AI Recycling" [13–15] | 2014; GDC 2015 | 40 full AIs and 120 high-res models while showing up to 10,000 crowd NPCs; 5,000 in some missions [V] | Unnoticed pooled swaps between low- and high-resolution NPCs [V]. | Interaction and render LoD are separate budgets, about 100× apart. |
| *Hitman: Absolution* [16,17] | 2012 | 1,200-character crowds at 30 fps on PS3/360; about 500 on screen [V] | Every crowd member can be interacted with and influenced [V]. | Dense crowds can be interactive if per-agent state is tiny. |
| *GTA V* (RAGE) [18] | 2013 | No authoritative numbers | Script-created peds report `POPTYPE_MISSION` (7), ambient peds 4–6 [V, community]. Ambient peds are recycled; mission entities persist [R]. | The two-class "ambient vs. pinned" identity is the industry norm. |
| *Cyberpunk 2077* [19] | 2020– | n/a | Crowd NPCs spawn and despawn near the player; at launch, looking away could replace them [V-anecdotal]. | Object permanence is the most criticized failure: never dehydrate in view. |
| Unreal **Mass** / City Sample [20–24] | 2021– | About 35k pedestrians and 20k traffic vehicles (fan wiki); 38,146 drivable cars [V, mixed] | Per-LoD representation: high-res actor, low-res actor, vertex-animated ISM (experimental), or none [V]. Near: rigged MetaHumans; far: VAT meshes [V]. Separate simulation-LoD tick rates [R]. | **Simulation LoD ≠ representation LoD.** |
| *Dwarf Fortress* [25,26] | 2006–; Steam 2022 | World-gen tracks a small percentage as historical figures; the rest is abstract counts; soft cap on histfigs [V] | Nobles, villains and megabeasts are histfigs. In play, units become histfigs when notable (a kill, an artifact). Rules exist so that "people you encounter don't suddenly disappear… as they are offloaded" [V]. | **The two-population model Rehoboam needs.** |
| *Mount & Blade II* [45] | 2020– | Battle size up to about 1,000 agents [R] | Parties hold troop *rosters* (counts plus wounded); heroes are persistent. Scenes spawn agents from rosters with edge reinforcement waves and write casualties back [R]. | A clean macro↔scene contract; **reinforcements = boundary flux.** |
| *No Man's Sky* [42] / *Elite* Stellar Forge [43] | 2014– | 2^64 seeds; about 400 billion systems [R] | Regenerated from seed on each visit; player edits stored as deltas; Elite pins real catalog stars [R]. | Seed plus delta overlay for unpinned detail. |
| *Victoria 3* pops [44] | 2022– | n/a | Pops are aggregates by culture, religion and profession per state; named characters are a separate class [R]. | Pops never disaggregate, which is exactly the gap Rehoboam must fill. |
| RimWorld world pawns | 2018– | n/a | Off-map pawns are GC'd unless important (kin, leaders, referenced by "tales") [R, `WorldPawnGC`; no URL]. | **Pin anything the event log references.** |
| Cities: Skylines II | 2023– | 100k+ individual citizens on Unity DOTS [R] | Heavy CPU use at launch [R]. | 100k+ agents needs compiled ECS code. |

### 3. Multi-resolution modeling (defense and science)

| Work | Date | Contribution | Fit |
|---|---|---|---|
| Davis & Hillestad (WSC 1993); Davis & Bigelow, *Experiments in MRM*, RAND MR-1004 [27]; *Motivated Metamodels*, MR-1570 [28] | 1993–2003 | Model families with explicit aggregation relations; integrated hierarchical variable resolution. Consistency means agreement *on the outputs the application cares about*, not state equality. Low-resolution models are calibrated against high-resolution runs [R]. | **High** (defines the criteria; Stratum-1 calibration) |
| Reynolds, Natrajan & Srinivasan, "Consistency maintenance in multiresolution simulation" (ACM TOMACS 7(3)); Natrajan's UNIFY / Multiple Representation Entity (MRE) [29] | 1997–2000 | Instead of switching between aggregate and disaggregate, an MRE maintains **all representations concurrently**. Attribute Dependency Graphs and mapping functions keep them consistent; an interaction resolver handles concurrent interactions. Names the failure modes of classical aggregation-disaggregation [R]. | **Very high** |
| "Pseudo-disaggregation" (1990s CGF practice) [R] | 1990s | Generate high-resolution entities *for interaction or display only*, without replacing the aggregate as the authority. | **Very high**: this is what most hydration should be |
| Hybrid ABM–compartmental epidemic models (Bobashev et al., WSC 2007) [33] | 2007 | Switch individual ↔ compartment representations depending on prevalence [R]. | High (contagion stratum) |
| AgentTorch, "limits of agency" (8.4M-agent NYC, LLM archetypes per cohort) [62] | 2024 | Behaviour computed per *archetype* and broadcast to individuals [R]. | Medium: a cohort-level decision pattern, LLM-free variant applicable |

I could not run fresh 2025–26 MRM searches. To my knowledge [R], the defense MRM line has not produced a new canonical framework since UNIFY/MRE. Recent work appears under "hybrid simulation", "multi-level ABM" and co-simulation (HLA/FMI).

**The classic failure modes of aggregation-disaggregation** (Reynolds/Natrajan, [29] [R]), mapped to Rehoboam:

| MRM failure | Rehoboam manifestation | Mitigation |
|---|---|---|
| Mapping inconsistency | Hydrated people don't sum to the macro counts, e.g. a hydrated plague victim is still counted alive in the cell. | Per-person rows are the single source of truth; hydration never invents people. |
| Temporal inconsistency | The bubble is at 14:03 on day 12 while the macro sim is at month 3 boundary; the two disagree on who is alive. | Ownership transfer plus lookahead decrees (see Recommendations). |
| Chain disaggregation | Talking to a smith hydrates his wife, his guild and his debtors, then their relations. | Cap the hydration depth: relations are hydrated as *stubs* (ID plus pinned facts) and fully hydrated only when encountered. |
| Thrashing | The camera oscillates at the bubble edge, so entities flicker in and out. | Hysteresis (hydrate at R_in, dehydrate at R_out > R_in), a minimum residence time, and no dehydration in view. |
| Transition latency | A hydration spike when a crowded gate comes into range. | Budgeted hydration (N per frame); prefetch along the camera velocity vector; T2 crowd proxies cover the gap. |

**Consistency criteria that matter** (Inference, synthesized from [1,27,29]):

1. **Identity stability.** The same ID always yields the same immutable traits, regardless of when, where or in what order it is hydrated.
2. **Conservation (mapping consistency).** Macro counts equal the sum of rows (people, deaths, infections, goods) at every sync point.
3. **Observational non-contradiction.** Anything the player saw or was told is canon (a facts ledger).
4. **Temporal consistency.** One authority per entity per interval; no entity is in two places at once.
5. **Distributional fidelity.** Hydrated samples are statistically indistinguishable from what full simulation would produce (Sunshine-Hill's criterion).
6. **Observer independence of the macro future.** Hydration must not perturb unrelated macro outcomes; only *material* bubble events may. This is what keeps the Time-Machine reproducible.

### 4. Deterministic identity: counter-based and hash RNG

| Option | Version/date | Licence | Stability | Fit | Notes |
|---|---|---|---|---|---|
| Philox / Threefry (Random123; Salmon et al., SC11) [34,35] | 2011; repo active 2026 [V] | BSD-style [R] | Crush-resistant [R] | **High** | A counter-based design: output = f(key, counter); no state to carry. |
| NumPy `Philox` BitGenerator [36] | NumPy 2.5.4 (latest on PyPI [V]) | BSD-3 | Raw bit streams are stable; **`Generator` distribution methods are not guaranteed stable across NumPy versions** (NEP 19 [37], R) | **High** | Works keyed per entity: `Philox(key=world_seed, counter=[id,…])` was order-independent in testing, but costs about 19 µs per entity to construct [X]. |
| Squares (Widynski) [38]; JAX Threefry key-splitting [41] | 2020; stable | Paper; Apache-2.0 | Counter-based [R] | Reference | Squares is easy to port to numba; JAX is the reference design for key splitting. |
| SplitMix64 / noise-based hash RNG (Eiserloh GDC 2017 "Squirrel noise" [39]; Jarzynski & Olano JCGT 2020 [40]) | 2014–2020 | Public domain / paper | Hash quality is adequate for gameplay [R] | **Very high** | Vectorized hydration of 200 entities × 12 fields took 0.43 ms; 1M entities × 12 fields took 0.78 s [X]. |

**Design implication (Inference).** Derive every stochastic decision from `u = H(world_seed, process_id, entity_id, tick, k)`. Then:

- the sim is order-independent and parallel-safe;
- hydration consumes no shared stream, so the observer effect disappears;
- replay needs only the seed plus the log of player and material bubble events.

Turn uniforms into variates with your own inverse-CDF or ratio code, not NumPy `Generator` methods, so saved worlds survive NumPy upgrades (NEP 19 [37], R). Note also that NumPy's SIMD-dispatched `exp`/`log` can differ in the last ULP across CPUs [R]. If cross-machine bit-exact replay matters, keep decision thresholds in integer or fixed-point arithmetic, or accept per-machine determinism only.

### 5. Cohorts vs. individuals

| Representation | Memory at 1M | Disaggregation | When to use |
|---|---|---|---|
| Dense cohort tensor (cell × 20 ages × 2 sexes × 16 occupations × 8 factions × 4 health states) | 5.37e9 bins, 21 GB [X] | Ill-posed | Never at this granularity |
| Sparse cohort table (only non-empty bins) | About 1M entries, roughly one person each [X, from occupancy] | Ill-posed | Pointless: it degenerates to individuals |
| Compartments per *settlement* or district (e.g. 2,000 settlements × age × sex × SEIR) | About 1 MB | Requires a sampling rule | Off-map polities; contagion fast-path; > 5M populations |
| **Per-person struct-of-arrays rows** | 19–64 MB [X] | Trivial (rows *are* individuals) | **Recommended for the 1M simulated population** |

Lifetime population matters over 1,000 years. A stable 1M population with a pre-modern crude birth rate of 35–40‰ means about 35–40M people ever lived, or about 2.2–2.6 GB at 64 B each [X, arithmetic]. Dead rows must be compacted into aggregates unless pinned (see Recommendations).

The initial population comes from manifest marginals via iterative proportional fitting or updating: Beckman et al. 1996 [63, R]; PopulationSim [48, R]. This is the established way to disaggregate marginals into individuals once, at world birth.

### 6. Constraint-consistent backstory and lazy history

- **Alibi generation** [1,2]: retroactive elaboration, conditioned on what was observed.
- **Caves of Qud** (Grinblat & Bucklew, FDG 2017) [51, R]: generate events first, then *rationalize* causes. Good for flavor backstory if the rationalization is constrained to ledger facts.
- **Talk of the Town** (James Ryan) [52, R]: pre-simulated town history gives characters knowledge, misremembering and lies. That is the substrate the facade LLM's "only what the character knows" rule requires.
- **Answer-set programming** (Smith & Mateas 2011) [R]: too slow and unpredictable per NPC at runtime; use it offline to validate manifests.

**Inference: the lazy-history recipe.** An unnamed person's biography is a *view* over facts that already exist:

1. Their row: birth tick, birth cell, household, parents if tracked, faction, health history bits, and hive bit.
2. Their cell's or settlement's event log since birth: famines, raids, plagues, edicts.
3. Their occupation's template.

Flavor such as anecdotes, grudges and favourite tavern is sampled from H(seed, id, "bio", k), conditioned on items 1–3. Whatever is *revealed* to the player is written to the facts ledger and pins the entity.

### 7. Daily schedules and "real economic errands"

| System | Version/date | Licence | Maturity | Fit | Notes |
|---|---|---|---|---|---|
| MATSim [46] | Repo active Oct 2026 [V] | GPL-2.0 [R] | Mature research (Java) | **Concepts only** | Each agent has a daily plan (activity chain with times, locations and modes). Co-evolutionary replanning; queue-based mobility simulation [R]. The GPL licence makes embedding unattractive. |
| ActivitySim [47] (+ PopulationSim [48]) | Repo active Oct 2026; BSD-3 [V] | BSD-3 | Production in US MPOs | **Structure to copy** | Hierarchy: coordinated daily activity pattern (mandatory / non-mandatory / home) → tours → stops → destination and time-of-day choice [R]. Too heavy to run per tick; borrow the hierarchy. |
| The Sims: needs + smart-object "advertisements" (Forbus & Wright) [50] | 2000– | Paper | Shipped | **High (venue level)** | Objects advertise need satisfaction; agents pick by utility [R]. |
| Ultima VII schedules; Oblivion/Skyrim Radiant AI "packages" [49] | 1992; 2006–11 | n/a | Shipped | Medium | Time-window plus location plus procedure packages [R]. |
| *Shadows of Doubt* daily precompute [11,12]; *WD:Legion* Census schedules [8] | 2020–24 | n/a | Shipped | **High** | Schedule is a function of profile and generated at uprez [V]. |

---

## Recommendations for Rehoboam

### R1. Tier model ("rings", not strata)

| Tier | What lives there | Count | Clock | Representation | Authority |
|---|---|---|---|---|---|
| **T−1 Off-map** | Foreign polities, hinterlands | Compartments | Seasonal | None | Macro |
| **T0 Ledger** | *Every* living person as a row (id, birth tick, sex, cell, household, occupation, faction, health/SEIR, hive bit, wealth band, pinned bit, schedule-template id) | 1M | Monthly; adaptive weekly or daily for active epidemics or wars | Fields and heatmaps only | Macro |
| **T1 Agenda** | Named and pinned actors: leaders, hive nodes once revealed, the Oracle's targets, anyone in the facts ledger | 1–5k | **Event-driven** discrete-event agenda (next decision time per actor, 0.2–2 decisions/s at most); vectorized utility scoring | Far-field proxy if spectacle-class | Macro, plus T1 for its own decisions |
| **T2 Crowd proxy** | Statistically sampled real T0 people who are outdoors in the visible area this hour | 200–5,000 visible | 5–10 Hz flow-field/steering, in batches | VAT or MultiMesh impostors; no dialogue | Read-only view of T0 |
| **T3 Embodied** | Hydrated within the interaction radius: full components, navmesh, perception, barks | 50–200 (hard cap) | 10–20 Hz sim, 60 fps render | Rigged kitbash meshes | **Bubble owns** them between sync points |
| **T4 Conversant** | 1–3 NPCs in active SLM dialogue | 1–3 | Turn-based | Close-up | Bubble, facts ledger |
| **Spectacle** (orthogonal) | Dragons, armies, fires, processions: T1 actors with a world-scale visual footprint | Tens | T1 clock, interpolated | Impostor or particles out to 2–5 km | T1 |

Ring radii (starting values, tune per vista; Inference from [13,21] and the pixel-size calculation [X]):

- T3 hydrate at 80 m, dehydrate at 130 m, plus at least 10 s out of view.
- T2 crowd out to 300–500 m. A human is about 6 px tall at 300 m.
- Spectacle and skyline out to the draw distance.

This directly answers the vista case. The keep with circling dragons is *Spectacle plus T2*, not hydration. A city intersection is T3 (≤200) within a T2 crowd. The Living Diorama mostly needs T2 plus Spectacle, and hydrates only when the camera dwells.

### R2. Identity model

- `person_id` is a u64 **birth serial** assigned by the macro sim in deterministic order: births processed sorted by (tick, mother_id). Founders are IDs 0..N₀−1 from IPF synthesis. Never reuse an ID.
- Immutable traits come from `H(world_seed, person_id, field)`: sex at birth if not already set by the birth process, face, voice, body, temperament (Big-Five-like), name (culture table plus H), handedness and so on. These are never stored and always recomputable.
- Mutable macro state is the T0 row. Pinned detail lives in a facts ledger table (SQLite/DuckDB): `(person_id, fact_type, value, source_event_id, revealed_at)`.
- **Hive membership is a T0 bit set by the contagion process**, never generated at hydration. A reveal reads the bit; the SLM must never be the source of the infection status.
- *If* the population ever exceeds about 5M, switch T0 cells to "implicit individuals": counts per cohort, where member k has `id = H(cell, cohort, birth_year, k)`, and deaths remove the members with the smallest `H(id, tick, "death")`. That keeps identities stable without rows. It is more complex, so don't build it until it is needed.

### R3. Promotion and demotion (pinning policy)

**Promote T0 → pinned (T1 ledger membership) when any of these holds:**

- The person is spoken to (T4).
- They are in T3 and in view for at least N seconds within M minutes (as in WD:Legion's engagement-based retention [9]).
- They are the subject or object of a *material* event: death, crime, transfer of significant goods or office, a reveal.
- The sifter references them in a matched pattern.
- They hold an office.
- The Oracle targets them.
- They are first-degree kin or a close associate *referenced in dialogue*, hydrated as a stub only.

**Demote:**

- T3 → (T1 if pinned, else T0) when outside R_out, out of view for 10 s or more, and not engaged.
- T1 → T0 never for the living (pins are sticky). Dead pinned persons are kept while referenced by the event graph or by living pinned kin, then compacted, RimWorld-style [R].
- Unpinned dead are compacted into cell- and decade-level aggregates (counts by cause), keeping only lineage links for pinned lineages.

### R4. What is persisted on dehydration

| State | Unpinned | Pinned |
|---|---|---|
| Traits (face, voice, temperament, name) | Not stored (recomputed from H) | Not stored (recomputed from H) |
| T0 row delta (cell, health, alive, wealth band) | Written back | Written back |
| Exact position and animation | Dropped (snapped to cell plus anchor) | Dropped |
| Revealed backstory, relationships, dialogue summary, player reputation, inventory deltas, schedule overrides | n/a (pin is triggered first) | Facts ledger |
| Unrevealed generated flavor | Dropped; it regenerates identically from H | Dropped unless revealed |

### R5. Time reconciliation

1. **Ownership.** At hydration, the bubble takes ownership of the person's fast state. During a macro step the macro sim *excludes owned rows from its stochastic draws*. Instead it issues **lookahead decrees** for them: for example, "dies of plague at a sampled time t inside this step" or "migrates on day d". The bubble enacts each decree at time t, or the decree fires on dehydration if the person leaves first. Because the decree comes from the keyed RNG, the outcome is the same whether or not the person was observed. That keeps criterion 6 (observer independence) for everything *except* material bubble events.
2. **Feedback.** Material bubble events (the player's actions, NPC deaths, thefts, reveals) are appended to an event log with an origin tag. They are applied to T0 at the next sync point and become *inputs* for replay (event sourcing). Cosmetic interactions are never fed back.
3. **Catch-up.** A pinned T1 actor that has not been evaluated since tick t₀ is advanced on demand. Use closed-form or summarized updates for slow variables (ageing, wealth drift) and replay its agenda only for salient events. This is the same lazy trick T0 cells can use for between-step queries.
4. **Clock ratios.** Street-level and Diorama run at an accelerated but bounded scale. For example, Skyrim's default timescale is 20:1 [R]; 30–60:1 is reasonable. At those scales a monthly macro step happens every 12–24 real hours. A district-level daily sub-step therefore runs around the bubble (cheap: ≪70 ms), so births, deaths and market prices change visibly. The Time-Machine **forbids T3**: it collapses the bubble (dehydrates everything) before fast-forwarding and re-hydrates at pause.

### R6. Schedules and errands (three levels)

1. **T0: a schedule template per person.** About 30–60 templates keyed by occupation × age band × sex × season × faction. Each is a small Markov chain of activity blocks (sleep, work, market, worship, tavern, patrol). From these, compute **hourly per-cell presence and origin-destination flows** with bincount. These flows drive T2 crowds and contagion contact rates. Inference: this is ActivitySim's day-pattern → tours hierarchy [47], collapsed.
2. **T1/T3: concrete errands driven by economy state.** Household stocks below threshold generate a purchase errand at a market cell that actually has stock. Debts generate a visit to the creditor; summons generate a visit to the court. The errand is generated with keys H(id, day, "errand") from real T0 and T1 economic state, so it is a "real economic errand".
3. **Venue level: Sims-style smart-object advertisements** [50] choose the concrete interaction once the person arrives.
4. **Crowd → embodied promotion uses an alibi.** A T2 proxy is a real `person_id` sampled *without replacement* from T0 people whose template places them outdoors in that area at that hour. On promotion, its errand is sampled conditioned on its observed heading and position [1,2], so it doesn't turn around unexpectedly.

### R7. Boundary flux

With isotropic walkers, the inward crossing rate equals the density × mean speed / π, times the perimeter. That gives about 1.3 entries per second for an ordinary street (0.005/m²) and about 5 per second for a busy market (0.02/m²) at R = 100 m, with a mean residence time of about 2 minutes [X]. Hydration therefore needs a **budget of about 5–10 per second**, with prefetch along the camera velocity vector.

- Entrants come from T2 proxies crossing R_in (they already have identities), not from new spawns.
- Exits return through dehydration once outside R_out and not in view.
- Use Bannerlord-style "reinforcement" only for scripted mass events [R].

### R8. Implementation notes

- **Spatial index:** a NumPy CSR grid hash with 50–100 m cells for T0 and T2 (rebuild per macro step); a dict-of-cells for movers in T1 and T3. A k-d tree is optional. SQLite or DuckDB serves *only* the ledger, snapshots and analytics.
- **Stratum 2 runtime:** NumPy/numba structured arrays plus an event agenda (heapq). Use esper [58] only if an ECS API is wanted for T3 in Python. Drop Flecs, or move T1 and T3 into Godot via a GDExtension. A `godot-flecs` prototype exists but is negligible (1 star) [V-GitHub search], so plan on writing that binding yourself.
- **v2:** a LOD Trader-style allocator [3,4] that hands out the 200 T3 slots and the T4 dialogue slots by BIR risk: on-screen size, gaze, narrative salience from the sifter, and pinned status.

---

## Risks & unknowns

1. **Source verification gap.** Web search hit its quota and most primary hosts were unreachable from the sandbox. MRM papers [27–33], RNG papers [34,37–41] and several game facts (*NMS*, *Elite*, *Victoria 3*, *Bannerlord*, *RimWorld*, *CS2*) are [R]. Re-verify them before citing externally, especially the TOMACS DOI and the MRM failure-mode list attributed to Reynolds/Natrajan.
2. **Observer effect.** Even with keyed RNG, material bubble events change the future. The Time-Machine's "same seed → same history" holds only *given the same event log*. Product-wise, decide whether that is a feature ("your wandering changed history") or a bug.
3. **Float determinism across machines** (NumPy SIMD transcendental functions) [R]. Saved-world sharing across PCs may diverge. Mitigate with integer thresholds in decision-critical paths.
4. **Chain hydration blow-up** through dense kin and social graphs. The stub-hydration cap is essential, and graph depth must be capped per frame.
5. **Benchmarks are toy-scale.** The 70/38 ms macro step omits markets, politics, migration logic and hive spread. Expect 3–10× growth. Benchmark again once the real Stratum-1 processes exist.
6. **Python latency jitter (GC, GIL)** alongside an SLM on the same consumer PC. T3 at 10–20 Hz must not share a process with LLM token streaming.
7. **Godot rendering cost** of 200 *unique* kitbashed characters plus thousands of VAT impostors. The draw-call and skinning budget is unverified. Godot's visibility ranges and MultiMesh [59, R] are the likely tools, but this needs a spike.
8. **Statistical fidelity of T2 proxies.** If flows come from coarse templates, a trained eye will see "the same crowd". *Cyberpunk* players estimated a duplicate rate of about 10–20% [19, anecdotal]. Unique identities per proxy, enforced by sampling without replacement, mitigate this.
9. **Dead-row compaction vs. 1,000-year genealogy.** The novels in modality 5 may want ancestors of pinned characters. Lineage links must survive compaction for every pinned lineage, which can grow large in later centuries.

---

## Sources

1. https://repository.upenn.edu/edissertations/435 — Sunshine-Hill, *Perceptually Driven Simulation*, PhD thesis, UPenn 2011 (alibi generation, LOD Trader, Marketplace) [V].
2. https://ojs.aaai.org/index.php/AIIDE/article/view/12389 — Sunshine-Hill & Badler, "Perceptually Realistic Behavior through Alibi Generation", AIIDE 2010 [V].
3. http://www.gameaipro.com/GameAIPro/GameAIPro_Chapter14_Phenomenal_AI_Level-of-Detail_Control_with_the_LOD_Trader.pdf — Game AI Pro (2013) ch. 14, LOD Trader and Break-in-Realism [V exists; not fetched].
4. https://doi.org/10.1145/2522628.2541250 — Sunshine-Hill, "Managing Simulation Level-of-Detail with the LOD Trader", MIG 2013 [V].
5. https://artemis.ms.mff.cuni.cz/main/papers/IVE_IVA07.pdf — Brom, Šerý, Poch, "Simulation Level of Detail for Virtual Humans", IVA 2007, LNAI 4722 [V].
6. https://artemis.ms.mff.cuni.cz/main/papers/IVE-LOD-2006.pdf — Šerý, Poch, Šafrata, Brom, IVE LoD paper, SOFSEM 2006 [V].
7. https://scss.tcd.ie/publications/theses/diss/2016/TCD-SCSS-DISSERTATION-2016-055.pdf — TCD 2016, "A Multi-tiered Level of Detail System for Game AI" [V exists].
8. https://www.gdcvault.com/play/1027018/Census-The-Systemic-Backbone-Behind — Dragert, GDC 2021, Census talk listing [V].
9. https://www.gamedeveloper.com/design/how-watch-dogs-legion-s-play-as-anyone-simulation-works — Uprez on demand; active set of hundreds cycles out unengaged citizens [V].
10. https://news.ubisoft.com/en-us/article/4po3S9Pwp1YcgBmGPmQxAh/watch-dogs-legion-the-tools-that-built-london — Ubisoft on Census: "millions" of playable Londoners [V].
11. https://colepowered.com/shadows-of-doubt-devblog-8-simulating-a-city/ — *Shadows of Doubt*: hundreds of simulated citizens; per-day routine precompute [V].
12. https://colepowered.itch.io/shadows/devlog/78044/shadows-of-doubt-devblog-15-moving-in-the-citizens — Update rate high only near the player [V].
13. https://gdcvault.com/play/1022411/Massive-Crowd-on-Assassin-s — Cournoyer, GDC 2015, AC Unity AI recycling [V].
14. https://archive.org/details/GDC2015Cournoyer — Talk abstract: 40 full AIs, 120 high-res, 10,000 crowd NPCs [V].
15. https://www.techradar.com/news/gaming/inside-assassin-s-creed-unity-ubisoft-s-leap-of-faith-1268435/2 — 5,000 AI in some missions [V].
16. https://gdcvault.com/play/1016443/Crowds-in-Hitman — Fauerby, GDC 2012: 1,200-character crowds at 30 fps [V].
17. https://www.gamedeveloper.com/business/gdc-europe-2012-adds-i-hitman-i-bigpoint-breakdancing-talks — About 500 on-screen characters [V].
18. https://forum.cfx.re/t/knowing-whether-a-ped-or-vehicle-was-created-by-a-script/1503416 — GTA V population types (mission = 7 vs. ambient 4–6) [V, community].
19. https://steamcommunity.com/app/1091500/discussions/0/4635988522932735208 — *Cyberpunk 2077* despawn-on-look-away reports [V, anecdotal].
20. https://dev.epicgames.com/documentation/unreal-engine/overview-of-mass-gameplay-in-unreal-engine — Mass representation LoD: high-res actor, low-res actor, ISM, none [V].
21. https://dev.epicgames.com/documentation/unreal-engine/city-sample-project-unreal-engine-demonstration — City Sample: rigged MetaHumans near, VAT meshes far [V].
22. https://vrealmatic.com/unreal-engine/city-sample/crowd — Community breakdown of City Sample crowd processors [V, community].
23. https://www.pcgamer.com/unreal-engine-5-city-sample-matrix/ — 38,146 drivable cars [V].
24. https://matrix.fandom.com/wiki/The_Matrix_Awakens — About 35k pedestrians and about 20k Mass-simulated vehicles [V, low reliability].
25. https://dwarffortresswiki.org/index.php/Historical_figure — DF histfigs vs. abstract population; consistency rule on offloading [V].
26. https://steamcommunity.com/app/975370/discussions/0/3878219832279670171/ — DF Steam discussion; developer remark on units becoming histfigs (paraphrased via search; exact thread unconfirmed) [V-partial].
27. https://www.rand.org/pubs/monograph_reports/MR1004.html — Davis & Bigelow, *Experiments in Multiresolution Modeling*, RAND 1998 [R].
28. https://www.rand.org/pubs/monograph_reports/MR1570.html — Davis & Bigelow, *Motivated Metamodels*, RAND 2003 [R].
29. https://doi.org/10.1145/259207.259235 — Reynolds, Natrajan, Srinivasan, "Consistency maintenance in multiresolution simulation", ACM TOMACS 1997 (UNIFY/MRE) [R; DOI unverified].
30. (no URL) Natrajan, PhD thesis, University of Virginia, c. 2000, *Consistency Maintenance in Concurrent Representations* [R].
31. https://www.ifaamas.org/Proceedings/aamas2011/ — Navarro, Flacher, Corruble, dynamic LoD for urban ABM, AAMAS 2011 [R].
32. https://arxiv.org/abs/1205.0561 — Morvan, multi-level ABM survey [R].
33. https://www.informs-sim.org/ — WSC archive; Bobashev et al. 2007 hybrid ABM/equation epidemic model [R].
34. https://www.thesalmons.org/john/random123/papers/random123sc11.pdf — Salmon et al., "Parallel Random Numbers: As Easy as 1, 2, 3", SC11 [R].
35. https://github.com/DEShawResearch/random123 — Random123 reference implementation (active, updated Oct 2026) [V].
36. https://numpy.org/doc/stable/reference/random/bit_generators/philox.html — NumPy Philox (key/counter use tested locally) [R doc; X behaviour].
37. https://numpy.org/neps/nep-0019-rng-policy.html — NEP 19 RNG stream-compatibility policy [R].
38. https://arxiv.org/abs/2004.06278 — Widynski, "Squares: A Fast Counter-Based RNG" [R].
39. https://www.gdcvault.com/play/1024365/Math-for-Game-Programmers-Noise — Eiserloh, noise-based RNG, GDC 2017 [R].
40. https://jcgt.org/published/0009/03/02/ — Jarzynski & Olano, "Hash Functions for GPU Rendering", JCGT 2020 [R].
41. https://jax.readthedocs.io/en/latest/jep/263-prng.html — JAX Threefry PRNG design [R].
42. https://www.gdcvault.com/play/1024265/Continuous-World-Generation-in-No — McKendrick, *No Man's Sky* world generation, GDC 2017 [R].
43. https://elite-dangerous.fandom.com/wiki/Stellar_Forge — *Elite Dangerous* Stellar Forge [R, wiki].
44. https://vic3.paradoxwikis.com/Pops — *Victoria 3* pops [R, wiki].
45. https://moddocs.bannerlord.com/ — Bannerlord modding docs (parties, rosters, missions) [R].
46. https://github.com/matsim-org/matsim-libs — MATSim (active; licence GPL-2.0 [R]); book: https://www.ubiquitypress.com/site/books/10.5334/baw/ [R].
47. https://github.com/ActivitySim/activitysim — ActivitySim, BSD-3, active Oct 2026 [V].
48. https://github.com/ActivitySim/populationsim — PopulationSim synthetic-population tool [R].
49. https://ck.uesp.net/wiki/Package — Skyrim AI packages (Creation Kit) [R].
50. https://www.qrg.northwestern.edu/papers/Files/Programming_Objects_in_The_Sims.pdf — Forbus & Wright, smart objects in *The Sims* [R].
51. (DOI unverified) Grinblat & Bucklew, "Subverting Historical Cause & Effect: Generation of Mythic Biographies in Caves of Qud", FDG 2017 [R].
52. https://escholarship.org/uc/item/1340j5h2 — James Ryan, *Curating Simulated Storyworlds*, PhD thesis 2018 [R].
53. https://github.com/mkremins/felt — Felt story sifter (JavaScript, small project) [V].
54. https://www.sqlite.org/rtree.html — SQLite R*Tree (32-bit float coordinates) [R doc; X behaviour].
55. https://github.com/SanderMertens/flecs — Flecs C/C++ ECS (active, about 8.7k stars) [V].
56. https://pypi.org/project/flecs/ — PyPI `flecs` 0.0.2 (2025-07-23), "minimal example" third-party bindings [V].
57. https://github.com/pixelflux/pyflecs — `pyflecs`, last push 2022-02-18, 0 stars [V].
58. https://pypi.org/project/esper/ — esper 3.9 (2026-09-20), pure-Python ECS [V].
59. https://docs.godotengine.org/en/stable/tutorials/3d/visibility_ranges.html — Godot visibility ranges (HLOD) [R].
60. https://www.gamedevs.org/uploads/three-states-plan-ai-of-fear.pdf — Orkin, GOAP in F.E.A.R., GDC 2006 [R].
61. https://en.wikipedia.org/wiki/High_Level_Architecture — HLA ownership and time management (IEEE 1516) background [R].
62. https://arxiv.org/abs/2409.10568 — AgentTorch, "On the limits of agency in agent-based models" (LLM archetypes per cohort) [R].
63. https://doi.org/10.1016/0965-8564(96)00004-3 — Beckman, Baggerly, McKay, "Creating synthetic baseline populations", 1996 (IPF) [R].
