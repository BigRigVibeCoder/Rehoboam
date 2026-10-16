# ADR-0002 — State model: SoA tables + pinned `bevy_ecs` for named actors + SocialGraph store

**Status:** Accepted (director, 2026-10-16) · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

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

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-C1, RT-M11). Sources of truth: ADR-0024, ADR-0026, 05.1 §2, §5, §6.*

No inline corrections. The decision stands, with these clarifications:

1. **"Pinned" here means version-pinned.** The title's "pinned `bevy_ecs`" pins the crate release. It is unrelated to **pinning a person**, which is a plane-B presentation guarantee and never promotes anyone to T1 (ADR-0024).
2. **T1 membership** comes only from simulation criteria (office, great-house rank, material involvement, Registry-declared notability) or a journaled input (ADR-0024 rule 3).
3. **No presentation components on authoritative entities.** Authoritative iteration over `bevy_ecs` entities is sorted by `PersonId`, never by archetype order (ADR-0024 rule 4; 05.1 §6.5).
4. **The ledger holds the living only.** Everyone ever born is in the append-only **life-course log** (Parquet, plane A; ADR-0026).
5. **Single-writer column ownership.** Every SoA column and every SocialGraph relation has exactly one owning system, declared in the Registry. Other systems read through **contract crates** (`rh-contracts`) and change state only through **lifecycle command queues** (05.1 §2; ADR-0022 v1.1). `rh-minds` never writes another system's tables; verb effects are typed commands.
