---
id: GOV-006
title: "Observability: what the engine, the stream and the tools must make visible"
type: reference
status: APPROVED
owner: human
agents: [all]
tags: [governance, observability, stream]
related: [GOV-004, ADR-0030, ADR-0031]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** Three things must always be visible without a debugger: what the world did (the World Stream), what the engine did (structured diagnostics and vitals), and whether it is still correct (hashes, invariants, check reports). Logging rules are in GOV-004; this document says what must be observable and where.

# GOV-006 — Observability

## 1. The three views

| View | Question it answers | Source | Phase |
|---|---|---|---|
| **World** | What happened, to whom, why? | World Stream channel A and C; `rh-cli tail`; virtual telemetry (`where`, `who`) | P1.0 |
| **Engine** | How fast, where is time spent, what is it doing? | Vitals line (tip, pace, ticks/s, plane hashes); per-system timings; Rerun timeline in development | P0.7 |
| **Correctness** | Is it still right? | Hash ladders; invariant counters; `artifacts/check_report.md`; certification reports; VER documents | P0 |

## 2. Required signals

- **Per tick (in memory, sampled into the step digest):**
  - sim time;
  - plane hashes (A every month in CI and debug, yearly in release);
  - invariant counters;
  - per-system time;
  - events emitted.
- **Per run:**
  - build, Registry version, manifest hash and seed;
  - wall time;
  - peak RSS;
  - stream size per channel;
  - keyframes written.
- **Per check run:** the report from `bin/check_all.sh` (GOV-002 §6).

## 3. Rules

- Observability never changes history (GOV-004 law 2).
- Every new system adds its timing and invariant counters, its stream record kinds and its `--brief` (ADR-0033) in the same sprint that adds the system.
- Dashboards come later and are harnesses (ADR-0030); in phase 1 everything is text.
