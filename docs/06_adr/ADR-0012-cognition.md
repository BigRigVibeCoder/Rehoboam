# ADR-0012 — Generic psych core + overlays; event-driven IAUS + HTN

**Status:** Proposed · **Date:** 2026-10-15

## Context

The draft's PsychState mixed timescales, and its vocabulary only made sense in one setting. GOAP at 10–30 Hz for 5k actors costs an estimated 7–75 core-seconds per simulated second.

## Decision

- **Core state.** HEXACO traits, Schwartz values, PAD mood plus up to 8 decaying OCC emotions, needs, opinion-modifier lists, RimWorld-style Thoughts, a belief store, and ACT-R memory activation.
- **Overlays.** Setting-specific stats (Paranoia, Filial Piety and so on) are derived overlays declared in the Registry.
- **Decisions.** Goals come from event-driven IAUS scoring; plans come from bounded HTN; places use Sims-style advertisements.
- **Cadence.** CK3-style polling plus interrupts.
- **Explainability.** The top contributors and margin of each decision are logged as `motivated_by` edges.

## Alternatives considered

| Option | Why not |
|---|---|
| GOAP everywhere | Too costly. |
| LLM agents (Concordia, Smallville) | Cost is about 2×10¹¹ tokens over 1,000 years, and they are non-deterministic. |
| A fixed five-vector psych model | Not generic across settings. |

## Consequences

- Calibration needs sensitivity sweeps.
- Light minds for unnamed persons are seeded from pop distributions.

## Revisit when

- The pivotal-consult evaluation shows LLM choices measurably improve MOE-1 at an acceptable cost.

## Evidence

R04 §1–2, §4.1; R03 §6.
