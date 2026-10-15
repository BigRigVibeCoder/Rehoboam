# ADR-0002 — State model: SoA tables + pinned `bevy_ecs` for named actors + SocialGraph store

**Status:** Proposed · **Date:** 2026-10-15

## Context

Most world state is dense and homogeneous: rasters, the region graph, the ledger and pops. That data is best held as struct-of-arrays (SoA) columns. It also maps onto Arrow, which is the keyframe and analytics format.

Named actors are heterogeneous (humans, dragons, hive nodes, armies) and benefit from ECS composition, change detection and observers.

Many-to-many weighted relations (trust, suspects, knows) fragment archetype ECS storage, and Bevy has no many-to-many relations in 0.20.

## Decision

1. **`rh-core` tables.** Custom typed SoA column store for rasters, the region graph (CSR), the ledger, pops, polities, markets and armies, laid out to be Arrow-compatible.
2. **Named actors (T1).** `bevy_ecs` used standalone and **pinned to one release** behind an `rh-minds` facade.
   - Single-threaded executor for authoritative systems.
   - Ambiguity checks and the 0.20 `shuffle_seed` enabled in CI.
   - One-to-many relations (ChildOf, FealtyTo) use Bevy's relationships.
3. **`SocialGraph` store.** Sorted CSR / `BTreeMap` adjacency holding weighted, provenance-tagged many-to-many relations.

## Alternatives considered

| Option | Why not |
|---|---|
| Flecs via flecs_ecs (Rust) | Self-declared alpha. Flecs remains the best C/C++ option for relations. |
| hecs | Minimal and stable, but we would have to write change detection and observers. Kept as a fallback behind the facade. |
| An ECS for everything | 1M ledger rows as entities give no benefit and complicate Arrow snapshots. |

## Consequences

- Bevy releases churn every 4–5 months. The version pin and facade contain that. Upgrades are deliberate and run golden-run CI.
- Two storage idioms exist. The facade hides `bevy_ecs` from system crates other than `rh-minds`.

## Revisit when

- Bevy ships many-to-many relations, or flecs_ecs reaches 1.0.
- The facade's overhead shows up in profiles.

## Evidence

R03 §1, §Recommendations 2; R01 §R8.
