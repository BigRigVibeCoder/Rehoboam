# ADR-0001 — Rust core, Python orchestration, TypeScript console

**Status:** Proposed · **Date:** 2026-10-15

## Context

The simulation must:

- be deterministic across runs and thread counts, and ideally across platforms;
- run 1,000 years at 1M people in minutes;
- run headless, as a server, and in-process inside Godot later;
- be built and grown by AI coding agents over years.

The draft mixed Python/NumPy with Flecs, but Flecs has no Python bindings. Measured costs:

| Workload | Python | Rust |
|---|---|---|
| 5k-actor utility AI, per tick | 44.7 ms (pure Python) | 0.81 ms (rayon) |
| Macro tick | NumPy ≈ 20 ms | ≈ 3.5 ms |

Sources: R03 and R01. NumPy's `Generator` also makes no cross-version stream guarantee.

## Decision

- **Rust** for the authoritative core and every system crate.
- **Python** (wheel via PyO3/maturin) for world compilation, the LLM gateway, the narrative compiler, calibration, analytics, and as a prototyping lane.
- **TypeScript** for the console.
- **GDScript plus Rust gdext** for the 3D client.

## Alternatives considered

| Option | Why not |
|---|---|
| C++ with Flecs, godot-cpp and nanobind | Coherent and viable, but we would have to write the Python bindings ourselves, and we would get weaker compiler guardrails for agent-written code. |
| Python with Numba or JAX | Fast enough per kernel, but determinism is fragile, embedding in Godot isn't realistic, and long-term scale is limited. |
| C# | No web export from Godot, and a weaker headless and Python story. |
| Julia, Zig | Embedding is impractical (Julia) or the language is still pre-1.0 (Zig). |

## Consequences

**Positive**
- The strong type system and borrow checker act as a verifier for agent-written code.
- The same crate builds the CLI, server, Python wheel and GDExtension.
- `libm`, `rand_chacha` stream IDs and rayon cover the determinism toolkit.

**Negative**
- Compile times are longer.
- The Python↔Rust boundary needs care around the GIL (release it with `Python::detach` during runs).
- New models are prototyped in Python and then ported. A statistical-equivalence test gates each port.

## Revisit when

- Agent throughput on Rust is measured to be well below throughput on Python, *and* profiling shows Python meets PERF-1/2.

## Evidence

R03 §3, §6, §Recommendations 1; R01 claims table.
