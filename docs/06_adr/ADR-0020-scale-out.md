# ADR-0020 — Single writer per timeline; scale out with timelines

**Status:** Accepted (director, 2026-10-16) · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15) \
**Amended by ADR-0034 (2026-10-16, D27):** deliberation consults and pull inputs retired.

## Context

Compute is unbounded. Distributing one world across server processes (the SpatialOS approach) caused consistency, latency and cost failures, and SpatialOS's Unity and Unreal GDKs were archived.

## Decision

- One authoritative process and writer thread per timeline, with rayon data-parallelism inside it.
- Horizontal scale goes to independent timelines:
  - Foresight rollouts.
  - Calibration ensembles and sensitivity sweeps.
  - What-if branches.
  - Liveness-gate, calibration and certification seeds.
- Extra compute also buys model fidelity (F2/F3 levels, larger regions and populations) and larger LLMs.
- Remote clients attach to cluster-hosted timelines.

## Alternatives considered

| Option | Why not |
|---|---|
| A sharded world with entity hand-off | Complexity with no benefit at our scale. |
| GPU simulation | 8 stencil layers take 0.65 ms on 4 CPU threads; GPU float results are not bit-identical across vendors. |

## Consequences

- Single-timeline speed is bounded by one machine's cores. The 1M-person budget fits (PERF-2).
- Beyond about 10M people, implicit individuals are needed (ADR-0003).

## Revisit when

- A single timeline needs more than about 64 cores to meet its budget.

## Evidence

R10 §5 (SpatialOS); R03 §6; R01.

## Amendment v1.1 (2026-10-15)

*Checked against ADR-0010 v1.1, ADR-0015 v1.1, 05.0 §4, 05.1 §6.9 and 05.2 §7. The decision stands.*

**Corrected inline:**
- "Burn-in seeds" → the liveness gate, calibration repair and background certification runs (ADR-0010 v1.1).

**Changes:**
1. **Every fork is a timeline with a unique salt lineage.** Rollouts, what-ifs and ensemble seeds never collide, however many workers run them. What-if A/B keeps the parent salt. Rollouts extend it and re-draw `resample_on_fork` timers.
2. **Rollout workers** run the same binary as separate processes. Each is stateless apart from its keyframe input. Parquet segments before the fork are shared by reference.
3. **No rollout waits on an LLM.** No simulation step calls a model (ADR-0034), so this holds by construction.
4. **Implicit individuals** are recommended above ~5M people and required above ~10M (ADR-0003).
5. **"Single writer" has two levels.** There is one writer per timeline (this ADR), and inside it one owning system per column (ADR-0022 v1.1).
