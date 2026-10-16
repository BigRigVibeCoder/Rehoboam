# ADR-0007 — Typed Parquet chronicle, causal edges, no-delete tiering

**Status:** Accepted (director, 2026-10-16) · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

## Context

The draft's chronicle had four problems:

- `context_json`/`causes_json` blobs in SQLite cost 83–114 B per event.
- Events were inserted at weight ≥ 8 but purged below weight 7, a contradiction.
- Weights were fixed at creation.
- It had no as-of state.

Typed Parquet costs about 11 B per event, so a 1,000-year chronicle of named actors is roughly 3 GB. DuckDB is 60× faster than SQLite on a year-sized window but has a ~20 ms floor and allows a single writer. Kùzu and CozoDB are dead, and DDlog is archived.

## Decision

1. Normalized tables, append-only, partitioned by timeline and sim-year, in Parquet: `event`, `event_role`, `event_snapshot` (anchored state), `causal_edge` (typed), `knowledge`, `collection`, `aggregate_event`, `state_interval`, `sig_component`, `sift_match`, `facts_ledger`.
2. Journal, plane hashes (`state_hash`, `observation_hash`, `narrative_hash`) and keyframe index in SQLite (WAL).
3. Keyframes in Arrow IPC with zstd.
4. DuckDB only as **read-only views** for retrospective queries and the console.
5. **Nothing is deleted by rule.** Significance is recomputed per era. Named-actor events are never physically compacted, moved or rewritten. Low-salience events are hidden by a per-timeline **visibility sidecar** (plane C), and hindsight retention marks causal ancestors retained-visible there.

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
- Commoners are recorded as regional aggregates, `aggregate_event`s and the life-course log. A person gets `event` rows only by plane-A criteria, never by being pinned.

## Revisit when

- Recursive or aggregate sifting needs incremental view maintenance. Then embed differential-dataflow or dbsp.

## Evidence

R10 TL;DR, §1–3, §R1–R2; R02 §R1, §R5.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-C1, RT-M3, RT-M7, RT-M14, RT-m7). Sources of truth: ADR-0024, ADR-0026, 05.2 §4, §6–8, 05.4 §3.*

**Corrected inline:**
- Decision 5 described **physical compaction**: ancestors pinned, unpinned events folded and moved to archive partitions. That conflicts with forks, which share Parquet segments by reference, and it made plane-A storage depend on pins. Named-actor events are now never physically compacted.
- Decision 2: "state hashes" → the three **plane hashes**.
- Consequences: commoners were "aggregates only, unless promoted". Everyone ever born is now in the life-course log.

**Changes:**
1. **Tables by plane.**

   | Plane | Tables |
   |---|---|
   | A | `event`, `event_role`, `event_snapshot`, `causal_edge`, `knowledge`, `diffusion_front`, `collection`, `aggregate_event`, `state_interval`, `life_course` (+ interval tables), `regional_aggregate` |
   | B | `facts_ledger` |
   | C | `sig_component`, `sift_match`, `visibility` |
   | Canonical record and ledgers | `input_journal`; `state_hash`, `observation_hash`, `narrative_hash`; `keyframe` index |

2. **Event rows by plane-A criteria only:** a tellable act class, a named (T1) participant, rarity above a threshold, or any material input. Pins never create event rows.
3. **Visibility sidecar (plane C).** It holds a per-timeline tier for each named event. It hides low-salience events without moving or rewriting them, so it stays compatible with forks that share segments. Rarity base rates are kept as counts per (pattern, era).
4. **Bounded hindsight retention (RT-m7).** When an event's combined significance crosses θ, walk backward and mark events retained-visible. Follow `contingent` edges plus the single top `motivated_by` edge per node. Stop at Registry-declared root event kinds or at depth 12. This never changes plane A.
5. **Hard deletion** is an explicit, opt-in disk-pressure action on cold archives only. It never touches data plane A reads, or data the archived engine build cannot regenerate (RT-M3).
6. **Life-course log (ADR-0026).** Plane A, append-only Parquet, one entry per person ever born. It sits in the warm tier with events, knowledge and diffusion fronts.
7. **Knowledge and diffusion fronts (RT-M7).** Per-actor `knowledge` rows are kept only for witnessed, faction-internal and secret events, and for testimony hops between named actors. Public, crowd and document knowledge is stored as `diffusion_front` arrival times per (event, region): `knows(a, e, t) = row(a, e) OR (public(e) AND arrival(region(a, t), e) ≤ t)`.
8. **Forks.** A fork shares the Parquet segments before its fork tick by reference. Journal inputs after the fork belong to the child. Visibility is per timeline.
9. **"Frozen past, migrated future" (DET-10).**
   - Each world records the content-addressed engine build of every segment of history. Those builds are archived.
   - An upgrade writes a **migration keyframe** at the head. The head then runs on the new build.
   - History before it replays with the archived build, or is read from the chronicle.
   - Keyframes are never thinned below what an archived build can regenerate.
10. **Keyframes.** Arrow IPC with zstd-3, tagged with `engine_build` and schema version. Permanent every 10 sim-years, plus a yearly LRU near the cursor. Cold segments are recompressed per century (zstd-19).
11. **Volume.** The ~3 GB in Context is R10's estimate for named-actor events alone. 05.2 §8 now estimates 5–15 GB for events and roles at 1M persons, and tens of GB in total (`08` §6).
