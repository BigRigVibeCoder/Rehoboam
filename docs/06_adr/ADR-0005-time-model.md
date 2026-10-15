# ADR-0005 — Integer sim-time, pulses, event-driven agenda, speed bands

**Status:** Proposed · **Date:** 2026-10-15

## Context

The draft's speed multipliers were off by about five orders of magnitude. At 10,000×, 1,000 years takes 36.5 days. Its agents ran at 10–30 Hz even in fast-forward, and a single `TimeScale` was set per world.

## Decision

- `SimTime = u64` minutes, with a calendar defined in the manifest.
- Pulses run at Year, Season, Month, Day and Hour; inside the 3D bubble there is also a real-time pulse.
- Systems bind to pulses. Adaptive daily stepping happens only for active epidemics, campaigns and sieges.
- Named actors run on a discrete-event agenda keyed by `(time, phase, entity_id, seq)`, with polling intervals in the style of CK3 plus interrupts.
- A fixed phase order within each instant (05.1 §4.2).
- Controls are bands of *simulated time per wall-clock second*, plus **MAX** and **run-until**.
- **Speed never changes the simulation.**

## Alternatives considered

| Option | Why not |
|---|---|
| A fixed global tick | Wastes compute on idle systems and can't reach millennial speed. |
| Float time | Ordering and determinism hazards. |

## Consequences

- At MAX speed, drama finer than a month exists only for named actors and active daily systems. The sifter must accept that granularity.
- The bubble at Live speed runs real-time elaboration and exchanges decrees with the macro layer.

## Revisit when

- Profiling shows pulse overhead dominates. Unlikely.

## Evidence

R03 §5; R01 §R5.
