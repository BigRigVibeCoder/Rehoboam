# ADR-0009 — Curation, not intervention

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

## Context

Emergent behaviour is not narrative. Hidden directors "kill the pleasure" because events stop really happening. RimWorld's storyteller works because it is named, chosen and framed in the world.

## Decision

- No hidden director inside the simulation.
- Pacing lives in presentation: auto-pause thresholds, camera choice and arc selection.
- Pressure is declared in the manifest and shown in the causal inspector.
- Optional storyteller modes are labelled, opt-in and logged as `director` causal edges, and are off in "pure" runs.
- Bad runs count as bugs to fix through contingent unlocking and system coupling.

## Alternatives considered

| Option | Why not |
|---|---|
| A Façade/L4D-style director spawning events | Breaks the thesis that drama comes from the simulation. |
| Pure emergence with no curation | Boring logs. |

## Consequences

- Narratability is designed into the Registry: verbs declare which past events they can reference.
- The bad-run rate (MOE-6) is a first-class metric.

## Revisit when

- MOE-1 and MOE-6 stay unmet after repeated rounds of simulation fixes. Then consider labelled storyteller modes for the game modalities only.

## Evidence

R02 §2–3, §R4.

## Amendment v1.1 (2026-10-15)

*Checked against ADR-0024 and 05.4 §2.3, §3–4. No inline corrections. The decision stands. These points make it consistent with the three planes:*

1. **Curation lives in planes B and C.** Auto-pause thresholds, camera choice, arc selection and the rating-trained salience model read plane C. None of them writes plane A.
2. **Storyteller modes** (game modalities only, opt-in, off in "pure" runs) are declared plane-A components. They read plane A only. Their effects are typed commands, and their events carry origin `director` and `director` causal edges. They never read sift matches, significance or ratings.
3. **Feedback curation.** A pattern whose matches change simulation behaviour is promoted into the Registry rule set and becomes authoritative (ADR-0008 v1.1).
4. **Bad runs are also caught at compile time.** The liveness gate requires ≥ 2 completed arcs per century and a bad-run rate ≤ 1/8 across its seeds (ADR-0010 v1.1).
