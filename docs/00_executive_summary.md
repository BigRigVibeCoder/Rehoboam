# 00 — Executive Summary

*Status: Draft v1.1 (2026-10-15). Rehoboam planning baseline. Read this first. Every claim here links to the document that backs it.*
*v1.1 changes: reflects the red-team revision of the architecture (`05.0`–`05.9`, ADR-0024 to ADR-0026): three state planes, scheduled outcomes and material inputs, genealogical identity, the Oracle as an in-world entity, deliberation consults, the revised world compiler, and the Registry effect language. Roadmap durations now defer to `11`.*

---

## What we are building

**Rehoboam** is a personal world engine. A deterministic simulation runs a civilisation for centuries, from 100k people up to a target of 1M. You watch its history unfold, stop where it matters and ask *why*. You turn that history into chronicles and stories. In later phases you step inside it: as an ambient diorama, as a walker in its streets, and as a mortal Oracle who influences its rulers without commanding them.

**Success**, by your definition, means four things (`01` §2):
- **S1, a creative engine:** it produces history worth reading.
- **S2, an inhabitable world:** every person and place is coherent and persists.
- **S3, investment:** worlds persist, replay exactly and can branch.
- **S4, evolvable:** fidelity and complexity can keep rising without rewrites, with **AI agents as the workforce** and **no compute ceiling**.

## Verdict on the draft

The **vision is strong and the core separation is right**: deterministic systems own the truth, and LLMs act only as compiler, facade and synthesizer. The **engineering plan would not have reached the vision**. Six critical defects (`02` §1):

1. **Five modalities with no sequencing, on a "Day 1–7" plan.** Each modality alone has been a multi-year product somewhere else.
2. **A world DSL with no semantics.** `consequence_action: str` and `Dict[str, Any]` hold names the engine cannot execute. "100% valid JSON" guarantees only syntax.
3. **A macro model that cannot produce history.** Undamped Poisson demography, no economy, polity, war or climate, and a linear Laplacian used for contagion and dragons.
4. **A chronicle that deletes its own causality.** Events with weight ≥ 8 are inserted, events with weight < 7 are purged, weights are fixed at creation, and causes are kept as JSON blobs.
5. **No determinism design.** A scrubbable 1,000-year timeline is impossible without one, and hydration would change history.
6. **Hydration without a consistency model.** There is no identity scheme, no pinning and no time reconciliation.

Many further major defects are listed in `02` §2:
- Speed presets that are 10⁵× off.
- Flecs Python bindings that don't exist.
- A "cannibalisation" list of GPL, AGPL, non-commercial and dormant code.
- Sifter rules that read current state instead of the state at the time of the event.
- 2024-era LLM choices.
- Dialogue grounding by prompting alone, which leaks 54% of the time even with frontier models.
- Whisper Counsel that is open to prompt injection.
- No street-level content pipeline.
- Named-author styling.

A second, internal red-team review of the v1.0 plan found that *observing* the world could still change it. The v1.1 architecture closes that and the related gaps; the key ideas below are the revised design.

## The redesigned architecture (`05.0`)

```
 Console (TS: Time-Machine · Causal Inspector · Oracle · Foresight;      Godot 4 client (Diorama · Street · Oracle-3D)
          Cultivator / Oracle modes)
                     │  JSON-RPC control + binary columnar state frames  │
 ┌───────────────────┴──────────────────────────────────────────────────┴───────────────────┐
 │ rh-server (Rust): Timeline Manager · Live Sifter · Chronicle Writer · Keyframes · Foresight │
 │   rh-core + system crates (M1 regions · M2 demography · M3 agri/climate · M4 economy ·      │
 │   M5 contagion/hive · M6 polity(SDT) · M7 dynasty · M8 war · S* culture/tech/ecology)       │
 │   rh-minds (psych core · IAUS+HTN · beliefs/channels · persuasion)                          │
 │   Plane A authoritative · Plane B observation · Plane C narrative — three separate hashes  │
 └──────────┬───────────────────────────────┬───────────────────────────────┬────────────────┘
   Parquet chronicle · Arrow keyframes   Rollout workers (forked timelines)   LLM Gateway (Python)
   SQLite journal · DuckDB views         Foresight · ensembles · certification llama.cpp · vLLM · frontier
                                                                               World & Narrative Compilers
```

**The key ideas**
- **Three planes of state** (ADR-0024). Plane A is authoritative, plane B is what you observed (pins, the facts ledger, the 3D bubble), plane C is narrative (sift matches, significance, prose). A never reads B or C, and each plane has its own hash. The only way into A is a **journaled material input**. So **watching never changes history**, by construction (`05.2` §0).
- **Scheduled outcomes for everyone** (ADR-0025). Deaths, births, migrations and marriages are sampled in advance as timers for every person, watched or not, and fire at their own tick. Nothing is back-dated. Your actions, speech acts and Oracle verbs are **material inputs** with a dependency scope. Dropping into the past is read-only; **acting in the past forks** a new timeline (`05.2` §3.2).
- **Every person is a ledger row**, and **every person ever born is in the life-course log** (ADR-0026). IDs are **genealogical** ("this mother's n-th child"), so the same person exists in every branch and common random numbers survive across forks. Hydration adds detail, never invents people, and never takes authority. Pinning keeps what you were shown stable; it never makes anyone a named actor (`05.2` §1, §3).
- **Determinism contract.** Keyed counter-based RNG, fixed-point thresholds, golden-run CI, and a divergence bisect tool. History = f(build, registry, manifest, seed, journal). Engine upgrades follow **"frozen past, migrated future"**: a new build continues from a migration keyframe, and older history replays with the archived build (ADR-0006, `05.1` §6, `05.2` §7).
- **Society on a region graph**, with a research-backed systems library: structural-demographic cycles, density-dependent demography, metapopulation SEIRS with a covert compartment and testing base rates, campaign logistics and succession hazards. Each system has a **fidelity ladder** F0→F3, and each level is validated against the stylized facts. Systems own their columns as single writers and talk through contract crates and lifecycle command queues. This is the mechanism for S4 (`05.1`).
- **The chronicle never forgets.** Typed causal edges, anchored state snapshots, retroactive significance with hindsight retention, and tiered storage that hides low-salience events rather than deleting them (`05.2`, `05.4`).
- **Incremental story sifter.** A setting-agnostic pattern DSL compiled to automata, with declared match semantics and an SQL twin for every pattern. LLMs only re-rank and explain. **No hidden director** (`05.4`).
- **The Mechanics Registry is the language.** It includes a small, typed **effect language**, so many new laws, overlays and verbs land as data rather than code. The world compiler runs: LLM *intent* plus a **mapping table** (every approximation visible) → deterministic expander → validators → a fast **liveness gate** (200 years × 8 seeds) → **deterministic calibration** if it fails (the LLM never tunes numbers) → play, with **certification** (1,000 years × 32 seeds) in the background. **Gap reports** turn your imagination into the agents' backlog (`05.5`).
- **Minds:** a generic psychological core with setting overlays, event-driven utility AI plus HTN, evidence-typed beliefs, and channels, of which **the hive is one**. **Whisper Counsel is parsed, not injected**, and scored deterministically. Optional LLM **deliberation consults** choose from a legal-action menu and return as journaled pull inputs after a fixed delay (`05.3`).
- **The Oracle is an in-world entity** with its own belief store. The console has two modes: **Cultivator** (omniscient, verbs off) and **Oracle** (every view, including Foresight, filtered by what the Oracle knows) (`05.3` §6.2, `05.7`).
- **Foresight is real forecasting.** It runs Monte-Carlo rollouts on forked timelines and uses common random numbers to compare interventions. This is the best use of your unbounded compute, and it is literally what the fictional Rehoboam does (ADR-0015, `05.3` §7).
- **Narrative compiler.** A hierarchical, canon-gated pipeline with zero tolerated contradictions, style cards built on public-domain exemplars, and roughly $28–35 per novel (`05.4` §6).

## Stack in one line (`07`)
Rust (core) · Python (LLM, compilers, calibration) · TypeScript + deck.gl + Sigma.js (console) · Parquet/Arrow/DuckDB/SQLite · llama.cpp + vLLM/SGLang + frontier APIs · Godot 4.7+ with Rust gdext · Chatterbox/Pocket/Qwen3-TTS · Rerun · marimo.

**What we reuse from open source**, and what we build, is inventoried with licences and effort estimates in `14_reuse_inventory.md`.

## Roadmap (`11`)

*The authoritative schedule, with current phase splits and durations, is `11_roadmap.md`. The figures below are indicative.*

| Phase | Outcome | Gate |
|---|---|---|
| **P0** (≈ 4–6 wk) | Determinism harness, Registry, CI | Identical hashes across thread counts |
| **P1** (≈ 16–24 wk, split into sub-phases in `11`) | 1,000 years of headless history | ≥ 90% stylized facts; ≤ 2 min/1,000 years at 100k |
| **P2** (≈ 10–12 wk + a 3–5 wk G2 rating window) | Sifter + Time-Machine console | **Thesis gate:** ≥ 3 events per century that *you* rate ≥ 4/5, within a pause budget |
| P3 (split into sub-phases in `11`) | Hive, minds, Oracle-2D, Foresight, 1M scale | Emergent witch-hunt spiral; Foresight validity tests |
| P4 | Narrative and world compilers | 0 contradictions; gap-report loop proven |
| P5–P7 | Diorama → Street-level → Oracle-3D | Spikes SPK-1 to SPK-3 de-risk them during P3 and P4 |

**If the thesis gate (G2) fails, no 3D work starts.** Simulation and sifting are iterated until stories emerge.

## Top risks (`10`)
1. Emergent history turns out boring (R-01).
2. Scope creep across modalities (R-02).
3. Determinism leaks (R-03).
4. Architectural decay from agent-written code (R-04).
5. Macro calibration (R-05).

Every risk has an early-warning indicator and a mitigation. Agent governance (single-writer ownership, change classes, merge queue) is in `13`.

## What I need from you (`12`)
★ Approve the core ADRs. ★ Choose the first reference world (recommended: feudal, dragons added in P3). ★ Your OS. ★ Your actual compute and cloud budget. Then commit to rating sessions at each gate, because your ratings *are* the north-star metric. Rulings go in `decisions-log.md`.

## Evidence base
Eleven research dossiers (`research/R01`–`R11`, ~69k words, Oct 2026) covering:
- LoD and multi-resolution modelling;
- emergent narrative and sifting;
- simulation-core technology;
- cognition and persuasion;
- local LLMs and structured generation;
- engine, rendering and content;
- audio and TTS;
- long-form generation;
- macro-history science;
- data infrastructure;
- product precedents.

Machine-readable contracts in `specs/` are validated by running them:
- a manifest schema with semantic validators, including a negative test that rejects the draft's failure modes;
- a chronicle DDL plus SQL-twin equivalence against an incremental matcher, with declared match semantics, on a clustered 200k-event fixture;
- a pattern linter that rejects the draft's unanchored rule.
