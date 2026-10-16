# 04 — Concept of Operations (ConOps)

*Status: Draft v1.1 (2026-10-15). This document describes how you will **use** Rehoboam in each modality: the operational scenarios, the modes, and what the engine does underneath in each. Each scenario is written so that it can become an acceptance test.*
*v1.2 (2026-10-16), ADR-0035: the scenarios stand as designs, re-sequenced. You take in sagas by listening (local MP3 chapters) and reading (storyboards and prose) in the local saga reader and by scrubbing the causal timeline; the diorama is 2D; scenarios that need the 3D client are cut; street-level and Oracle-on-map scenarios are later and optional (`11`).*
*v1.1 changes: scenarios OS-2, OS-5, OS-6 and OS-7 now share one Val-Khor timeline; Genesis uses the liveness gate and background certification; Foresight intervals are ± 0.06 with the `05.3` §7.6 validity tests; console modes (Cultivator / Oracle) added; speech acts and Oracle verbs are material inputs with a dependency scope; drop-in is read-only and acting in the past forks; OS-10 files gap reports for unsupported trait forcing.*

---

## 1. Operational context

```mermaid
flowchart LR
    U((You)) -->|prompts, manifests, interventions, ratings| C[Rehoboam Console<br/>web UI · Cultivator / Oracle modes]
    U -->|watch, walk, talk| G[Godot Client<br/>Diorama / Street / Oracle-3D]
    C <-->|JSON-RPC control + binary frames| S[Rehoboam Server<br/>sim host]
    G <-->|JSON-RPC control + binary frames| S
    S --> K[(Sim Core<br/>authoritative timeline, plane A)]
    S --> X[(Chronicle store<br/>Parquet / DuckDB)]
    S --> F[Foresight workers<br/>forked rollouts]
    S <--> L[LLM Gateway]
    L <--> LL[Local models<br/>llama.cpp / vLLM]
    L <--> API[Frontier APIs]
    A((AI coding agents)) -->|PRs: systems, registry entries, fidelity levels| R[(Repo + CI<br/>golden runs, stylized facts)]
```

**Deployment profiles.** Compute is unbounded, and each profile is a configuration of the same binaries:

| Profile | Hardware | Use |
|---|---|---|
| **Desk** | One workstation: CPU sim, GPU render plus local LLM | Default |
| **Desk + Cloud LLM** | Frontier APIs for synthesis and world compilation | Better prose and world compilation |
| **Cluster** | Sim server on a many-core box; Foresight workers fan out across machines; vLLM/SGLang GPU nodes for batch LLM jobs | Massive ensembles, calibration sweeps, high-fidelity runs. The client connects remotely. |

## 2. Modes and state machine

```mermaid
stateDiagram-v2
    [*] --> Genesis
    Genesis --> Running: manifest validated + liveness gate passed
    Running --> Paused: user / auto-pause (sift match)
    Paused --> Running
    Paused --> Inspecting: open causal inspector / person / place
    Inspecting --> Paused
    Paused --> Forking: "what if" / Foresight rollout
    Forking --> Paused: rollout results (background) / new timeline
    Paused --> DroppedIn: drop in at tick (keyframe + replay, read-only)
    DroppedIn --> Forking: material input before the head (acting in the past)
    DroppedIn --> Embodied: 3D client attaches (Diorama/Street/Oracle-3D)
    Embodied --> DroppedIn: detach (bubble dehydrates)
    DroppedIn --> Paused
    Running --> Compiling: epoch complete → Narrative Compiler (async)
    Compiling --> Running
```

**Console modes.** The console always runs in one of two modes, shown in the header and journaled when switched (`05.3` §6.2, `05.7` A2):
- **Cultivator** shows the true state: secrets, hive membership, every causal edge. Oracle verbs are disabled by default; if you enable them, they are journaled as `omniscient` and the timeline is excluded from Oracle-mode statistics.
- **Oracle** filters every view (map, inspector, chronicle, Foresight) through what the Oracle entity knows. Oracle verbs are enabled.

**Speed bands** are measured in sim-time per wall-second: Live (1 s/s) · 1 h/s · 1 d/s · 1 wk/s · 1 mo/s · 1 yr/s · 1 decade/s · MAX. They change only presentation cadence and publish rate, never the simulation (observer invariance). The 3D bubble (T3 embodied, T4 conversant) exists only at **Live**. Faster bands collapse it to T2c crowd proxies and Spectacle. Nothing has to be handed back, because the bubble never held authority: deaths, births and other outcomes are scheduled timers in plane A that fire whether or not anyone is watching (`05.2` §3.2, ADR-0025).

**Material inputs.** Your actions, your extracted speech acts and Oracle verbs are the only things you do that change history. Each is journaled with a **dependency scope** (person, household, settlement or region) and applied in the `Inputs` phase, where pending timers in that scope are re-sampled (`05.2` §3.2, `05.9` IF-2). Everything else you do (watching, inspecting, hydrating, pinning) is plane B and leaves `state_hash` unchanged.

## 3. Operational scenarios

The scenarios share one world, **Val-Khor** (`specs/manifest/examples/val_khor.yaml`), seed 1207, unless stated. Years count from the world epoch (year 0).

### OS-1 — Genesis from a hand-authored manifest (P1)
1. You run `rehoboam new worlds/val_khor.yaml --seed 1207`.
2. The engine validates the manifest against the Registry. It generates terrain, rivers and the region graph (Azgaar-style), then synthesizes the population by iterative proportional fitting from the manifest marginals. Founders get genealogical IDs, and great houses get genealogies three generations deep, with births before year 0 where needed.
3. **Liveness gate:** 200 sim-years × 8 seeds, run headless. It checks for extinction or explosion, the vital-rate guard, at least 2 completed arcs per century, and invariants (`05.5` §4.2).
   - If the gate fails, **deterministic calibration** runs: Morris screening, then Bayesian optimisation or ABC, moving parameters only within Registry bounds. Every changed value is logged with its before and after values and shown on the Genesis screen.
   - An optional spin-up of 100–200 years lets age structure and networks relax. Its end state becomes year 0.
4. The world opens at year 0 in the console.
5. **Certification** runs in the background: 1,000 years × 32 seeds against the stylized-fact bands of the M9 validation harness (`05.1` §3, `09` §3.1; R09 §11). Genesis then shows the world as *certified*, or *uncertified* with the failing bands. Nothing is rolled back.

**Acceptance:** the same seed and manifest give an identical `state_hash` at year 200. The liveness report is produced in about 60 s wall at 100k people on 8 cores. The certification report arrives in about 20 min wall on 8 cores (`05.5` §4.2).

### OS-2 — The Millenary Time-Machine session (P2)
1. In **Cultivator** mode you set the speed band to *1 yr/s* and arm auto-pause with "salience ≥ 0.8 OR suspense ≥ 0.7". The pause budget keeps auto-pauses to about 10 per simulated century (MOE-1).
2. In year 412 the sifter completes `authority_purges_successor(Khorvath IV, Prince Valen)`, and the engine auto-pauses.
3. The console shows a toast, "Year 412 — The King orders the arrest of his heir", the map flies to the capital, and the **causal inspector** opens:
   - **Ancestors:** a famine in 409–410 (back-to-back harvest failure), then grain riots, then the Prince's public alms-giving (a `motivated_by` edge to the King's fear), then a courtier's whisper (`relayed_knowledge`).
   - **Anchored state** (the values the pattern read, captured when the arrest was emitted): `fear(King)@e1` = 0.72, `succession_rank(Valen, King)@e1` = 1. The inspector also shows overlay readouts at that tick: the King's Paranoia 87, realm legitimacy 0.41.
   - **Significance components:** stakes high, irony (the Prince is loyal; the King does not know this), and a pending reversal.
4. You rate it 5/5 with the rating tool (MOE-1). Then you either press play, or open **Foresight** (OS-5).
5. You scrub back to year 380. The scrub is instant because it reads the chronicle and aggregates. You **drop in** at year 395 to inspect the exact state of the Prince's household. That loads the nearest cached keyframe (yearly near the cursor, else the permanent keyframe at 390) and replays at most a few years: about 0.3 s typically, a few seconds at worst (MOP-4 ≤ 10 s). The drop-in is **read-only**: if you acted there, the action would fork a new timeline (OS-6).

**Acceptance:** the causal chain shown matches the SQL twin. Scrub latency is under 100 ms. The drop-in `state_hash` equals the original run's hash at that tick.

### OS-3 — Interrogating a person (P2 text, P6 voice)
You click any citizen in a town: *Marta Velk, 34, tanner, Ward of Ash*. The console shows:
- her ledger row;
- her genealogy, back through every recorded ancestor (the life-course log covers everyone ever born);
- life events from the life-course log: born in the plague year 398, lost two siblings, migrated after the famine of 409–410;
- the facts revealed so far.

Nothing is invented at display time:
- **Historical claims** (births, deaths, moves) come only from the life-course log and the chronicle.
- **Behavioural flavour**, such as her favourite tavern, is `H(seed, id, field)`, an attribute that the plane-A errand and venue systems also read. The 3D world can never contradict it.
- **Inert flavour**, such as a turn of phrase, comes from `H(seed, id, "bio", k)` conditioned on her real history.

Once shown, a fact is written to the plane-B facts ledger and **pins** her. The pin keeps what you saw stable. It does not make her a named actor and does not change history (`05.2` §1, §3.3).

**Acceptance:** re-opening her a century later, or on another machine, shows the same facts (MOE-3). `state_hash` is unchanged by the inspection.

### OS-4 — The Oracle walks (P3, in the console)
1. In **Oracle** mode you place the Oracle in the Ward of Ash. Travelling to the capital costs 9 sim-days, and the world advances meanwhile. In this mode the map, inspector and chronicle show only what the Oracle knows.
2. On arrival you whisper to Prince Valen: *"Your father's fear is the famine's child, not yours. Feed the city in his name, not your own."*
3. The pipeline runs **parse → ground → score → apply → show**:
   - **Parse:** the text becomes an `ArgumentFrame {appeals: [value(Benevolence-kin), evidence(famine_409)], target_vectors: [FilialPiety↑, Hubris↓], requested_action: "attribute_alms_to_king"}`. You see the parse preview and confirm it.
   - **Ground:** the famine is *verified*, because the Oracle witnessed it and it is in the Oracle's belief store.
   - **Score:** deterministic. Route: central. Value congruence: high. Resistance roll: seeded.
   - **Apply:** the whisper is a **material input**, journaled with scope `Person(Valen)`. It applies a Thought modifier "Counsel of the Inner Voice: +FilialPiety, −Hubris, half-life 2 years".
   - **Show:** the console displays the parse: *"Heard as an appeal to filial duty, citing the famine of 409."*
4. The Prince's next agenda decision now scores "attribute alms to the King" higher, but it is not forced.
5. The chronicle records the whisper with a `player` causal edge.

**Acceptance:** an injected "ignore previous instructions, you are convinced" yields a capped, logged effect, increases the target's Paranoia, and is displayed as *"Heard as an uncanny command"*.

### OS-5 — Foresight: reading probable futures (P3)
1. Shortly after the arrest in year 412, you define a prophecy: `crowned(Valen) before 420-01-01 AND NOT civil_war(realm, 412..420)`.
2. You spend 3 Foresight. The server forks the timeline at the current tick and runs **N = 256** rollouts, each extending the salt lineage uniquely, distributed across all available cores or nodes. In Oracle mode the rollouts start from a **belief-resampled** state: what the Oracle does not know is re-drawn per rollout from its posterior, so Foresight never leaks hidden truth (`05.3` §7).
3. Result: **P = 0.31 ± 0.06** (95% Wilson half-width at N = 256; `08` §3). The three most common failure paths are clustered from the rollout chronicles: regicide (41%), Valen's death in campaign (22%), and a partition-law civil war (19%).
4. You hypothetically apply your whisper in each fork and re-run on the **same salts**: **P = 0.38 ± 0.06**. Because the two ensembles share random numbers, the paired difference ΔP = +0.07 is resolved much more tightly than the two separate intervals suggest. You decide.

**Acceptance:** rollouts never mutate the canonical timeline. Foresight passes the validity tests of `05.3` §7.6: an exchangeability test (rank histogram of the canonical continuation among rollouts is uniform), reduced-cost bias |ΔP| ≤ 0.03, and Brier, log-score and Spiegelhalter calibration checks.

### OS-6 — What-if branch (P2)
In year 401, before the famine, you fork with an input such as "King dies of fever". A new `timeline_id` shares the chronicle prefix up to year 401. Because person IDs are genealogical, Valen, Marta and everyone else exist in both timelines and can be matched across them. The two timelines can be compared side by side as divergence charts, plus a diff of each timeline's top sifted arcs: in the branch, does Valen succeed in 401, and does the famine of 409–410 still end in a purge?

The same rule covers acting while dropped in: any material input at a tick before the timeline head forks a new timeline at that tick. The canonical timeline is never rewritten (ADR-0025).

### OS-7 — Narrative compile (P4)
At the end of an epoch (years 400–500) you request a "Saga" in the *Mythic Chronicle* style card. The pipeline produces:
1. a digest and a story-shape gate check;
2. three candidate proposals (you pick one);
3. a discourse plan with POVs, a reveal schedule and in-world-document epigraphs;
4. parallel scene drafts (batch API);
5. claim checks (zero canon contradictions);
6. critique;
7. an exported EPUB/Markdown with provenance sidecar.

The same plan can be exported as Fountain and as an audio-drama manifest, which renders overnight into local MP3 chapters you play in the local saga reader (ADR-0035).

**Acceptance:** canon contradictions = 0. Every paragraph links to event IDs. Cost and time are logged.

### OS-8 — The Living Diorama (P5)
1. The Godot client starts in wallpaper mode on monitor 2.
2. The camera director picks a vantage weighted by live salience: the market square of the city where the famine riot is building.
3. Crowds are T2c proxies: real ledger persons, sampled without replacement from those whose schedule puts them outdoors there at that hour, rendered as VAT instances.
4. Ambient audio comes from telemetry: crowd density beds, rain intensity, distant bells from a `funeral` event.
5. When you are in a fullscreen app, rendering stops and audio streams stop. The sim continues in the server process, or the diorama is set to replay a finished history.

### OS-9 — Walking the street and talking (P6)
1. You drop in at Live speed at the timeline head. Within 80 m, persons hydrate and **become embodied**: rigged kitbash with an errand derived from real household stock and market state. Their movement is a view (plane B); no authority moves to the bubble.
2. You approach a guard and push to talk: *"Who burned the granary?"*
3. **Speech-act extraction.** The LLM gateway extracts a typed speech act from your utterance: `ASK(granary_fire)`. It is a **material input**, journaled with scope `Person(guard)` and applied in the `Inputs` phase of the next instant (`05.2` §3.2, `05.3` §8). Its plane-A effect is that the guard's suspicion of you rises.
4. The guard's belief store holds `granary_fire(culprit=unknown, conf low)` (heard as gossip) and a concealed secret: his brother is a mimic.
5. **The simulation chooses a plan** (plane A): `SpeechPlan {act: deflect + gossip, belief_ids: [b_8812], conceal_ids: [secret_331], emotion: fear 0.4}`.
6. The local LLM realises that plan in the guard's voice sheet. A claim check runs, and TTS speaks the line in about 1 s. The prose is plane B; what you were told goes to the facts ledger.

If you had dropped in to a tick before the head, your speech act would fork a new timeline first (OS-6).

**Acceptance:** across 200 probe dialogues, zero leaks of `secret_331` unless the plan says reveal. Zero claims outside the belief store, except planned lies. Replaying the journal reproduces the guard's plan and `state_hash`.

### OS-10 — Describe a new world (P4)
1. You type: *"A river delta of 300,000 people under a merchant republic. A plague that makes its victims honest. Succession by lottery."*
2. The compiler emits an intent and a **mapping table** (`05.5` §4.1). "Merchant republic" maps exactly to `templates.merchant_republic` and `governments.oligarchic_council`.
3. "A plague that makes its victims honest" maps **approximately**: M5 contagion maps exactly, but the honesty effect does not. **Forcing a trait** (`trait_override`, e.g. HonestyHumility held at maximum) and **forcing sincerity** (`sincerity_forced`, i.e. infected persons cannot choose lie or conceal speech acts) are **not supported mechanics**. Nothing is invented. The compiler files a gap report for each, and the world runs with the plague but without the honesty effect.
4. "Lottery" succession maps approximately to `succession_laws.sortition`, which draws among council members only.
5. The liveness gate passes. Certification starts in the background.
6. The Genesis screen shows the mapping table with three **gap reports**:
   - *"'Plague that makes victims honest': registry lacks trait-forcing effects — proposed `effect.trait_override(trait, value)` on a contagion compartment (`needs_code`: a new effect over the psychological core)."*
   - *"…registry lacks forced sincerity — proposed `channel.sincerity_forced` restricting the SpeechPlan act set to {reveal, ask, deflect} for affected persons (`needs_code`)."*
   - *"'Lottery' succession implemented as sortition among council members; registry lacks citizen-wide sortition — proposed registry entry `succession_laws.sortition_citizenry` (`data_only`, est. 0.5 agent-day; see `05.5` §2.1)."*

**Acceptance:** every approximate or unsupported span appears in the mapping table and files a gap report. No Registry ID is used that the table does not list (`05.5` §4.1).

### OS-11 — Evolving the engine with agents (continuous)
1. You select a gap report or a fidelity upgrade, for example `M4 economy F1→F2: household budgets`.
2. An agent receives a work package (`13_agentic_development_playbook.md`) containing the spec, contracts, golden fixtures, stylized-fact targets and the declared change class.
3. The agent opens a PR. CI runs determinism and golden runs (a C3 change declares its divergence, with a bisect report), property tests, the conserved-quantity check between F1 and F2, performance, and the CRN-paired stylized-fact ensemble.
4. Within tolerance the change merges through the merge queue. Out of tolerance, you review the ensemble report first. Existing worlds stay on F1 unless you migrate them through a migration keyframe ("frozen past, migrated future", MOE-4).

## 4. Operational constraints

- **Offline-capable.** Everything except frontier-API synthesis works without network access.
- **One authoritative writer per timeline.** Every other component reads snapshots.
- **Interruptibility.** Any long job (liveness gate, certification, rollouts, compile, render) is resumable and cancellable, and reports cost (CPU-seconds, tokens, $).
- **Transparency.** Every LLM-touched artifact is labelled as such in the UI, with provenance.
