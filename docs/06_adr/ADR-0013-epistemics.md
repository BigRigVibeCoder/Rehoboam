# ADR-0013 — Evidence-typed beliefs; channels; hive as a channel

**Status:** Proposed · **Date:** 2026-10-15

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
