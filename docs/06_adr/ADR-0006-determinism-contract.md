# ADR-0006 — Determinism contract

**Status:** Accepted (director, 2026-10-16) · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15) \
**Amended by ADR-0034 (2026-10-16, D27):** deliberation consults and pull inputs retired.

## Context

A scrubbable, replayable, branchable history (success criterion S3) and a sound Foresight both require that history be a pure function of its inputs.

Measured hazards:

- Rayon float sums produce different bits at 1, 2 and 4 threads.
- `HashMap` iteration order differs on every run.
- NumPy's `Generator` makes no compatibility guarantee across versions.
- Rust std transcendental functions vary by platform.
- A sequential RNG consumed by hydration creates an observer effect.

## Decision

1. `history = f(engine_build, registry_version, manifest_hash, seed, input_journal)`.
2. **Keyed counter-based RNG** for every draw: `H(seed, branch_salt(t), system, purpose, entity, t, k)`, using Philox or ChaCha8 streams with SplitMix64/xxh3 for scalar hashes. Byte layout: `05.1` §6 item 2.
3. **Vendored samplers**, each checked against golden vectors.
4. **Integer or fixed-point values** for every quantity that crosses a threshold. `libm` only. No FMA contraction and no fast-math.
5. **No hash-map iteration** in authoritative code. Rayon only with fixed chunking and chunk-ordered reductions.
6. **State hash** (xxh3-64, chunked Merkle with dirty tracking) every simulated month in CI and debug builds; every simulated year (configurable) in release builds.
7. **Unsynced guard** on every read-only consumer.
8. **Golden-run CI.** Linux on every PR. Windows and macOS ARM nightly.
9. **Guarantee levels.** Level 1, bit-exact on the same binary, OS and architecture, is required. Level 2, bit-exact across platforms, is a goal; until it holds, keyframes are the fallback.

## Alternatives considered

| Option | Why not |
|---|---|
| Fixed-point everywhere (Photon Quantum style) | Too costly for raster fields. |
| No determinism, with snapshots only | Loses branching, Foresight with common random numbers, and compact replays. |

## Consequences

- Expect weeks of work chasing determinism leaks early on. This is budgeted in P0.
- LLM outputs are journaled, never re-queried, so batch-invariant inference is not needed.

## Revisit when

- Level 2 proves unattainable. Then document a "same platform" scope for shared worlds.

## Evidence

R03 §4, §Recommendations 4; R01 §4; R10 §R4; R04 §4.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-C1, RT-C2, RT-M2, RT-M4). Sources of truth: ADR-0024, ADR-0025, ADR-0026, 05.1 §6, 05.2 §7, 05.9 IF-1, IF-2. No inline corrections were needed. Decision items 2 and 6 are restated precisely here.*

1. **History function.** `history = f(engine_build, registry_version, manifest_hash, seed, input_journal)`. It covers **plane A**. Planes B and C are pure functions of A plus their own versions and inputs (ADR-0024).
2. **Keyed RNG, exact form.** `u = H(world_seed, branch_salt(t), system_id, purpose, entity_id, t, k)`. `H` is Philox-4x64 or ChaCha8 with stream and position, with SplitMix64 or xxh3 for scalar hashes.
3. **Lineage salts (RT-M4).** `branch_salt(t) = H(lineage path of the forks covering t)`. The canonical path is empty. Every fork, Foresight job and rollout extends the path uniquely, so salts never collide. This replaces "`branch_salt = k` for rollout k".
   - **`resample_on_fork`.** Timers and polls drawn before a fork are re-drawn after it only if their purpose carries this Registry flag (vital timers, latent periods). Otherwise they are shared.
   - **What-if forks keep the parent salt** by default, for a clean counterfactual. "Reroll fate" extends it.
   - **Common random numbers survive across branches** because `PersonId`s are genealogical (ADR-0026).
4. **Three plane hashes (ADR-0024).** `state_hash` (A), `observation_hash` (B) and `narrative_hash` (C) are separate. Each is a **chunked Merkle xxh3-64 with dirty tracking**, taken monthly in CI and debug builds and yearly (configurable) in release builds. Golden baselines cover `state_hash` only.
5. **Divergence bisect tool.** Per-table Merkle roots let a tool report the first divergent month, table and row between two runs.
6. **Plane isolation.** A lint forbids plane-A crates from depending on plane-B or plane-C crates. Read-only consumers get immutable frames. The runtime unsynced-guard hash check runs in debug and CI builds only. Observer invariance (DET-6) is tested from P2 by console fuzzing of pins, facts and inspections.
7. **Push inputs only** (ADR-0034).
   - **Push inputs** (player actions and speech acts, Oracle verbs, settings, and LLM translations of a person's own act) apply at their tick. They are the only input class; pull inputs are retired.
   - **Replay hard-fails** on a missing, extra or mismatched input. There is no silent fallback.
8. **No back-dating (ADR-0025).** No event is emitted with a tick earlier than the current instant. `EventId` order = emission order = `(tick, phase, seq)`. Material inputs declare a dependency scope. Timers in that scope are cancelled and re-sampled. The key includes each entity's `epoch`, a derived plane-A counter of the material inputs that have touched it.
9. **Acting in the past forks.** `apply_input` with `tick < now` is refused (`PastInput`). The Timeline Manager forks at that tick first. Seek and drop-in are read-only.
10. **Golden runs.** On every PR, `smoke` runs 1,000 years and the other reference worlds 200 years. All run full length nightly. Hashes are compared every sim-year. Level 1 means bit-exact on the same binary, **OS family** and architecture.
11. **Change classes** (ADR-0022 v1.1). Only a C3 (determinism-affecting) change may move `state_hash`. It is auto-approved if the 32-seed CRN-paired stylized-fact ensemble stays within tolerance. Out-of-tolerance changes, and changes to the determinism machinery itself, go to the director.
12. **Engine upgrades: "frozen past, migrated future"** (DET-10; ADR-0007 v1.1). Archived builds replay the past. The head moves to a new build through a migration keyframe.
