# ADR-0020 — Single writer per timeline; scale out with timelines

**Status:** Proposed · **Date:** 2026-10-15

## Context

Compute is unbounded. Distributing one world across server processes (the SpatialOS approach) caused consistency, latency and cost failures, and SpatialOS's Unity and Unreal GDKs were archived.

## Decision

- One authoritative process and writer thread per timeline, with rayon data-parallelism inside it.
- Horizontal scale goes to independent timelines:
  - Foresight rollouts.
  - Calibration ensembles and sensitivity sweeps.
  - What-if branches.
  - Burn-in seeds.
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
