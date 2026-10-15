# ADR-0015 — Foresight = Monte-Carlo forked rollouts with common random numbers

**Status:** Proposed · **Date:** 2026-10-15

## Context

The user has unbounded compute and wants the engine to scale with it. The fictional Rehoboam predicts futures. Determinism makes both forking and seeding new futures cheap.

## Decision

1. Foresight forks the canonical timeline at the current tick. Rollout *k* uses `branch_salt = k` for t ≥ fork.
2. N rollouts run on stateless workers that scale horizontally.
3. The output is P ± a Wilson confidence interval for a player-defined prophecy predicate, together with clusters of the paths that fail it.
4. Interventions are evaluated with **common random numbers**: the same salts are run with and without the intervention, and the results are compared pairwise.
5. Calibration is enforced by back-testing (reliability diagram in CI).
6. The canonical timeline is never altered.

## Alternatives considered

| Option | Why not |
|---|---|
| Heuristic or analytic forecasts | Not grounded in the simulation. |
| Independent ensembles for intervention A/B | Much higher variance. |

## Consequences

- This is the main horizontal-scaling workload.
- It is also a core game mechanic, paid for in Foresight.
- Rollouts can run at reduced fidelity in regions the predicate cannot reach.

## Revisit when

- Rollout cost limits interactivity. Then add surrogate models: learned emulators trained on rollouts.

## Evidence

R11 §H; R03 §4 (common random numbers implied by keyed RNG); R10 §R4.
