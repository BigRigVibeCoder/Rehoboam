# ADR-0003 — Every person is a ledger row

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

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

Every living person (and every creature of a ledgered archetype) is a row with a stable, genealogical, content-addressed `PersonId` (ADR-0026).

- Age-specific, density-dependent hazards with binomial draws: demography F1.
- Pops (region × class × faith) are *derived aggregates* with their own belief and morale dynamics.
- Off-map (T−1) populations use cohort compartments (F0). On-map demography is always per-person (F1+).
- Above ~5M people (R01 §R2) an "implicit individuals" mode is recommended, and above ~10M it is required. It is a future fidelity level. Members are `id = H(cell, cohort, birth_year, k)`, and deaths remove the members with the smallest `H(id, tick, "death")`.

## Alternatives considered

| Option | Why not |
|---|---|
| Cohort-component only | Fast (R03: 2.8 ms per tick in Rust), but disaggregation is ill-posed and there is no genealogy for commoners. Kept as the F0 level. |
| Pops only (Victoria 3 style) | Pops never disaggregate, which is exactly the gap. |

## Consequences

- Hydration only adds detail ("pseudo-disaggregation").
- Genealogy exists for everyone.
- Dead rows leave the live ledger immediately. Every person ever born stays in the **life-course log** (ADR-0026).
- The life-course log grows to roughly 1.5–2.5 GB per 1,000 years at 1M people (~0.2 GB at 100k).

## Revisit when

- PERF-2 fails at 1M with optimised kernels.
- Worlds above 10M people are needed.

## Evidence

R01 TL;DR 2–3, §5, §R2; R09 §2.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-C3, RT-M6, RT-M14). Sources of truth: ADR-0024, ADR-0026, 05.1 §3, §5, §7, 05.2 §1.*

**Corrected inline:**
- "`PersonId` birth serial" → genealogical, content-addressed `PersonId`. A global birth counter shifts every later ID when one extra birth happens in a branch.
- "Dead rows are compacted unless pinned or part of a tracked lineage" → dead rows leave the ledger at once, and the life-course log keeps everyone. Pins are plane B and never retain rows.
- "A lineage table … 0.5–2 GB" → the life-course log, re-baselined at ~1.5–2.5 GB per 1,000 years at 1M.
- "Off-map polities use compartments (F0)" → off-map (T−1) populations only. On-map demography is always per-person.

**Changes:**
1. **Genealogical identity (ADR-0026).** A child is `H64(mother_id, parity)`. A founder is `H64(seed, "founder", k)`. An immigrant or off-map materialisation is `H64(seed, "immigrant", origin, t, k)`. Collisions are checked at birth and resolved by a deterministic rehash. IDs are never reused. The same person keeps the same ID in every branch.
2. **The ledger holds the living. The life-course log holds everyone ever born.** It is append-only Parquet in plane A: `(id, mother, father, birth, death, cause)` plus interval tables for residence, occupation, faction and marriage. Genealogy no longer depends on pins, so "genealogy for everyone" is true.
3. **Minimal partnering in M2 F1**, so that fathers exist.
4. **Off-map T−1 compartments with a materialisation adapter.** F0 cohort compartments exist only off-map. People crossing the boundary are materialised (immigrants with "unknown parents" provenance) or absorbed by the adapter (05.1 §3).
5. **On-map demography is always per-person.** Variety comes from cadence and richness, not from switching representation. Fidelity switches happen only through a migration keyframe.
6. **Hydration is a view.** It lives in plane B and adds presentation detail only. Pinning never retains rows, creates events or promotes to T1 (ADR-0024).
7. **Open item: implicit individuals.** Its ID rule `H(cell, cohort, birth_year, k)` predates ADR-0026. It must be reconciled with genealogical IDs before that fidelity level is specified.
