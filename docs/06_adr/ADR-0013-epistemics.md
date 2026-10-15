# ADR-0013 — Evidence-typed beliefs; channels; hive as a channel

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

## Context

In the draft, beliefs updated only by line of sight. That misses hearsay, which is the engine of drama, and outside the bubble there is no geometry to trace sight lines through anyway. The hive was a special-case boolean.

## Decision

- **Belief records** carry an evidence type (observation, statement, eavesdrop, transference, confabulation, lie, mutation), a source, a channel, log-odds confidence and a lineage.
- **Channels** are typed (witness, testimony, document, crowd, whisper, hive), each with latency, fidelity, trust and reach.
- **Distortion operators** (leveling, sharpening, assimilation) apply to rumours as they spread.
- **Theory of mind** is kept to depth 1–2, and only for flagged propositions.
- **Suspicion** is a Bayesian posterior over each actor's hidden type.
- **The hive** is simply the channel {latency 0, fidelity 1, trust 1, reach all}, stored as one shared belief store.

## Alternatives considered

| Option | Why not |
|---|---|
| Line-of-sight only | Misses hearsay. |
| Full Talk of the Town mutation everywhere | Abandoned historically because the authoring cost was too high. Here it is used only for notable propositions. |

## Consequences

- Who-knows-what drives irony metrics and dialogue grounding.
- "Impossible knowledge" gives the hive an emergent tell.

## Revisit when

- Belief-store memory dominates keyframes. Then compress via semantic roll-ups.

## Evidence

R04 §3, §4.2–4.3; R02 §6.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-M7, RT-M13). Sources of truth: ADR-0024, 05.2 §4.1, 05.3 §4, §6.1–6.2, 05.7 A2. No inline corrections.*

**Changes:**
1. **Evidence types** add `reflection` (05.3 §4.1) and `vision`. A `vision` comes only from Pierce the Veil. It lives only in the Oracle's store, and NPCs cannot verify it.
2. **The Oracle is a plane-A entity with its own belief store.** It learns through the same channels as everyone else: witnessing by co-location, testimony from informants, documents, and `vision`.
3. **Whisper evidence is limited to the Oracle's beliefs.** Grounding checks claims against that store (ADR-0014 v1.1).
4. **Console modes** (05.3 §6.2).
   - **Cultivator:** omniscient views. Oracle verbs are disabled by default. If enabled, they are journaled as `omniscient`, and the timeline is excluded from Oracle-mode statistics.
   - **Oracle:** every view (map, inspector, chronicle, Person pages, Foresight) is filtered by the Oracle's knowledge. Unknown events are absent, not greyed.
   - Switching mode is a journaled input (`ConsoleMode`).
5. **Knowledge at scale (RT-M7).**
   - Per-actor `knowledge` rows exist only for witnessed, faction-internal and secret events, and for testimony hops between named actors.
   - Public, crowd and document knowledge is stored as `diffusion_front` arrival times per (event, region) and evaluated on demand.
   - Pop belief shares use proposition **templates** with garbage collection: shares below ε for longer than τ are dropped.
   - Suspicion posteriors are **sparse**: only pairs above prior + δ, or pairs flagged by an active scheme or investigation.
6. **Planes.** Beliefs and knowledge are plane A. The **facts ledger** (what you were shown) is plane B. Presentation honours it; the simulation never reads it.
7. **Foresight does not leak hidden truth.** In Oracle mode, rollouts fork from a belief-resampled state (ADR-0015 v1.1).
