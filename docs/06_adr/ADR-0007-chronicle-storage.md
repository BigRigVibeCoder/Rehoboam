# ADR-0007 — Typed Parquet chronicle, causal edges, no-delete tiering

**Status:** Proposed · **Date:** 2026-10-15

## Context

The draft's chronicle had four problems:

- `context_json`/`causes_json` blobs in SQLite cost 83–114 B per event.
- Events were inserted at weight ≥ 8 but purged below weight 7, a contradiction.
- Weights were fixed at creation.
- It had no as-of state.

Typed Parquet costs about 11 B per event, so a 1,000-year chronicle of named actors is roughly 3 GB. DuckDB is 60× faster than SQLite on a year-sized window but has a ~20 ms floor and allows a single writer. Kùzu and CozoDB are dead, and DDlog is archived.

## Decision

1. Normalized tables, append-only, partitioned by timeline and sim-year, in Parquet: `event`, `event_role`, `event_snapshot` (anchored state), `causal_edge` (typed), `knowledge`, `collection`, `aggregate_event`, `state_interval`, `sig_component`, `sift_match`, `facts_ledger`.
2. Journal, state hashes and keyframe index in SQLite (WAL).
3. Keyframes in Arrow IPC with zstd.
4. DuckDB only as **read-only views** for retrospective queries and the console.
5. **Nothing is deleted by rule.** Significance is recomputed per era, causal ancestors are pinned, and unpinned events are folded into collections and moved to archive partitions.

## Alternatives considered

| Option | Why not |
|---|---|
| SQLite as primary | Slow scans and large rows. |
| DuckDB as the writer | Single-writer constraint, and the per-tick latency floor. |
| Graph database (LadybugDB, the Kùzu fork) | Unnecessary for bounded-window patterns, and its long-term maintenance is uncertain. |
| Bitemporal store (XTDB) | Heavy JVM dependency. Our history is append-only per timeline, so we don't need it. |

## Consequences

- One writer thread in the simulation host.
- DuckDB 2.0 changes the storage format, but Parquet is the system of record, so that does not matter.
- Commoners are recorded as aggregates only, unless they are promoted.

## Revisit when

- Recursive or aggregate sifting needs incremental view maintenance. Then embed differential-dataflow or dbsp.

## Evidence

R10 TL;DR, §1–3, §R1–R2; R02 §R1, §R5.
