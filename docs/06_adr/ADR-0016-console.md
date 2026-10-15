# ADR-0016 — Web console served by the sim host

**Status:** Proposed · **Date:** 2026-10-15

## Context

The Time-Machine, the causal inspector, the map and Foresight are dense data visualisation. Godot UI has no mature large-graph or data-visualisation libraries. A Textual TUI can't draw maps or large graphs.

## Decision

- **Stack.** TypeScript plus Vite. The map uses deck.gl with an orthographic view and binary/Arrow attributes. The causal graph uses Sigma.js. A custom canvas timeline. Data arrives as Arrow IPC.
- **Hosting.** Served by `rh-server` on localhost.
- **Packaging.** Tauri on Windows and macOS. Electron or a plain browser on Linux until WebKitGTK WebGL is verified.
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
