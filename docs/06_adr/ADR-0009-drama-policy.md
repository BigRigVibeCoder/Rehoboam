# ADR-0009 — Curation, not intervention

**Status:** Proposed · **Date:** 2026-10-15

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
