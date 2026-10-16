---
id: GOV-004
title: "Errors and logging in a deterministic engine"
type: reference
status: APPROVED
owner: human
agents: [coder, tester, architect]
tags: [governance, coding, observability]
related: [GOV-003, GOV-006, ADR-0006, ADR-0030]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** No error happens in the dark, as in the director's template. But inside the engine, errors and logs must never change what the simulation does or read the wall clock. The engine's own flight recorder is the World Stream. Diagnostic logs are separate, structured, and outside the tick's hot loop. Services (Python, TypeScript) follow the template's rules directly.

# GOV-004 — Errors and logging

## 1. Laws

1. **No silent failure.** Every error is returned, logged with context, or fails the run.
2. **Logging never changes history.** No branch in authoritative code depends on whether logging is enabled, on log level, or on a log call's result.
3. **No wall clock in plane A.** Diagnostic records carry wall time; simulation records carry sim time only.
4. **The World Stream is the simulation's record** (ADR-0030). Diagnostics explain the *engine*; the stream explains the *world*.

## 2. Error taxonomy (engine)

| Class | Example | Handling |
|---|---|---|
| **Invariant violation** | People not conserved; a column written by a non-owner; a negative stock | Fail the run at once with a crash artifact (§4). Never continue |
| **Determinism leak** | Hash mismatch across thread counts; replay divergence | Stop; run the bisect tool (`docs/13` §4.5); P0 |
| **Input rejection** | A malformed manifest, journal entry or LLM output that fails its schema | Reject at the boundary with a typed error; journal the rejection; the simulation continues unchanged |
| **Resource** | Disk full while sealing a segment | Pause the engine at a tick boundary and preserve the live edge; never write a partial segment |
| **External service** | LLM gateway down | Journaled fallback (`05.3` §8); never block the tick |

## 3. Engine logging (Rust)

- **Structured events only.** Fields: `level`, `wall_time`, `sim_tick`, `timeline_id`, `system`, `event`, plus key-value context. Log through the `tracing` crate, with a JSON sink and a Rerun sink for development.
- **Levels:** `ERROR` (an invariant or leak; always fatal in tests), `WARN` (handled and anomalous), `INFO` (lifecycle: run start, keyframe, gate), `DEBUG`, `TRACE` (development only; never compiled into release builds of hot paths).
- **Hot loop:** no per-entity logging. Aggregate counters only, flushed at `Bookkeeping`.
- **Correlation:** `timeline_id` and `sim_tick` serve as the correlation ID for engine records.

## 4. Crash artifacts

A fatal error writes `artifacts/crash/<timeline>_<tick>.json` with these fields:
- build hash; Registry version; manifest hash; seed;
- the input-journal position;
- the last keyframe and the tick;
- the failing invariant or error;
- the last 1,000 diagnostic records.

With these, any crash replays exactly (ADR-0006).

## 5. Services (Python world compiler, LLM gateway, TypeScript console)

Follow the director's template rules:
- **Logging libraries:** `structlog` (Python), `pino` (TypeScript), JSON.
- **Context:** correlation IDs per request or job.
- **Global handlers:** one top-level exception handler per process.
- **Async safety:** no fire-and-forget tasks.
- **Failure tolerance:** circuit breakers for external APIs.

## 6. Tests

- Any `ERROR` record during a test fails it (GOV-002 law 3).
- A test injects each error class in §2 and checks the handling, including crash-artifact replay.
