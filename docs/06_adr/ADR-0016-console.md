# ADR-0016 — Web console served by the sim host

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

## Context

The Time-Machine, the causal inspector, the map and Foresight are dense data visualisation. Godot UI has no mature large-graph or data-visualisation libraries. A Textual TUI can't draw maps or large graphs.

## Decision

- **Stack.** TypeScript plus Vite. The map uses deck.gl with an orthographic view and binary/Arrow attributes. The causal graph uses Sigma.js. A custom canvas timeline. Data arrives as Arrow IPC.
- **Hosting.** Served by `rh-server` on localhost.
- **Packaging.** Browser first. Tauri 2 is optional on Windows and macOS. On Linux, a browser or Electron until WebKitGTK WebGL performance is verified.
- **Debugging.** Rerun 0.39 for spatial and time-series data, and a Textual TUI for headless runs.

## Alternatives considered

| Option | Why not |
|---|---|
| Godot for all UI | Weak for data visualisation. |
| Electron everywhere | Heavier, but acceptable. It remains the Linux fallback. |

## Consequences

- Two client code bases, the console and Godot, sharing the same protocol.
- The console is the first-class product surface for P2 and P3.

## Revisit when

- Godot ships data-visualisation tooling, or the 3D client needs an embedded console.

## Evidence

R10 §6, §R6; R11 §Recommendations 2.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-M13). Sources of truth: 05.7 Part A, 05.3 §6.2, 05.9 IF-3, ADR-0025.*

**Corrected inline:**
- Packaging: the console is **browser first**. Tauri is optional, not the Windows and macOS default.

**Changes:**
1. **Console modes**, shown permanently in the header bar:
   - **Cultivator.** The true plane-A state: secrets, hive membership, every causal edge. Oracle verbs are disabled by default.
   - **Oracle.** Every screen is filtered through the Oracle's belief store. Unknown events are absent, not greyed.
   - Switching uses `console.set_mode` and is journaled (`ConsoleMode`).
2. **Interaction contracts.**
   - **Scrub** reads only the chronicle and regional aggregates, in < 100 ms (MOP-3).
   - **Drop in** is keyframe + replay in ≤ 10 s (MOP-4), and is **read-only**.
   - **Acting in the past forks.** Any material input at t < head forks a new timeline from that tick. The canonical timeline is never rewritten. The fork appears as a branch in the Time-Machine lanes.
3. **Looking never changes plane A.** Displayed facts go to the plane-B facts ledger. Chronicle queries use read-only DuckDB views through query templates; clients send no arbitrary SQL.
4. **Genesis** shows the mapping table, the liveness and calibration reports, certification status and gap reports (ADR-0010 v1.1).
5. **Foresight screen.** Rollouts start from a belief-resampled state in Oracle mode and from the true state in Cultivator mode. Intervention A/B uses common random numbers (ADR-0015 v1.1).
