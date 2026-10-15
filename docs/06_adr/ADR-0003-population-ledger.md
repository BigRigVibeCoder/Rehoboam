# ADR-0003 — Every person is a ledger row

**Status:** Proposed · **Date:** 2026-10-15

## Context

The draft kept the masses as aggregates ("bitmasks", chunk counts) and hydrated people on demand. Two problems follow:

- Multi-resolution modelling consistency: mapping, temporal coherence, identity stability.
- Inhabitability: every person should have parents, a household and a history.

Measured costs:

| Representation | Cost at 1M people |
|---|---|
| Per-person narrow rows | 19–64 MB |
| Monthly vital and contagion step, NumPy | 70 ms |
| Same step, Numba | 38 ms |
| Same step, Rust | lower |
| Dense cohort tensor (cell × age × sex × occupation × faction × health) | 5.4×10⁹ bins (21 GB) |

## Decision

Every living person (and every creature of a ledgered archetype) is a row with a stable `PersonId` birth serial.

- Age-specific, density-dependent hazards with binomial draws: demography F1.
- Pops (region × class × faith) are *derived aggregates* with their own belief and morale dynamics.
- Off-map polities use compartments (F0).
- Above ~10M people, an "implicit individuals" mode is a future fidelity level. Members are `id = H(cell, cohort, birth_year, k)`, and deaths remove the members with the smallest `H(id, tick, "death")`.

## Alternatives considered

| Option | Why not |
|---|---|
| Cohort-component only | Fast (R03: 2.8 ms per tick in Rust), but disaggregation is ill-posed and there is no genealogy for commoners. Kept as the F0 level. |
| Pops only (Victoria 3 style) | Pops never disaggregate, which is exactly the gap. |

## Consequences

- Hydration only adds detail ("pseudo-disaggregation").
- Genealogy exists for everyone.
- Dead rows are compacted unless pinned or part of a tracked lineage.
- A lineage table grows to roughly 0.5–2 GB per 1,000 years at 1M people.

## Revisit when

- PERF-2 fails at 1M with optimised kernels.
- Worlds above 10M people are needed.

## Evidence

R01 TL;DR 2–3, §5, §R2; R09 §2.
