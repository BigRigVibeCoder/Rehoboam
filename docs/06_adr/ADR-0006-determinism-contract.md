# ADR-0006 — Determinism contract

**Status:** Proposed · **Date:** 2026-10-15

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
2. **Keyed counter-based RNG** for every draw: `H(seed, branch_salt(t), system, purpose, entity, t, k)`, using Philox or ChaCha8 streams with SplitMix64/xxh3 for scalar hashes.
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
