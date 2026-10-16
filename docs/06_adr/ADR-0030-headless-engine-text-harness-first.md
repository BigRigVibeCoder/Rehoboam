# ADR-0030 — Headless engine and the World Stream: viewers read a buffer, actions land at the tip; text only until G2

**Status:** Accepted (director, 2026-10-16) · **Date:** 2026-10-16 · **Amends:** ADR-0016 (web console moves after G2), ADR-0029 (no graphical view before G2), ADR-0005 (the street bubble follows the viewer's playback rate), `05.7`, `05.9`, `09`, `11` (P1.0, P1.17, P2.7) · **Spec:** `05.10`

## Context

The director wrote:

> "The engine is all that matters. It is constantly streaming its world, like the Matrix. I tap into the stream with my viewer and see it move. The viewer contains all the graphic ability, so I can upgrade it over time, but the stream contains enough fidelity that I see exactly what the actors and events are. Nothing slows the engine's set rate, which I can change. Viewing is a buffer event: the viewer buffers its own section of the stream at the rate I want. Interaction is a tip event: it affects the tip of the stream, and I need to be aware I'm interacting with the tip at its rate of change. For phase 1 there is no GUI."

**This is consistent with the plan's non-negotiables, and with established practice:**
- **Determinism (ADR-0006).** History = f(build, Registry, manifest, seed, input journal). Viewing is not an input; an action is a journaled input with its exact tick.
- **Three planes (ADR-0024).** Viewers read only. The only way in is a journaled material input.
- **Scheduled outcomes, acting in the past forks (ADR-0025).** Actions at the tip are ordinary inputs. An action at a past moment is a fork.
- **Pace never changes outcomes** (`05.1` §4.3, ADR-0005).

**The pattern itself is well established:**
- an append-only, sequenced log with independent consumer cursors (event sourcing; Kafka-style logs);
- deterministic input replay (lockstep game replays store inputs, not states);
- model–view separation in agent-based toolkits (MASON runs headless; GUIs attach and detach);
- "behind live" playback in live streaming, where a viewer can rewind while the live edge moves on, and spectators watch a delayed buffer while players act at the tip.

**Two things the v1 plan got wrong:**
1. **Viewers were assumed to need graphics before G2.** The walking skeleton ended in a web console with a glyph view.
2. **Slow viewers were "skipped to the latest snapshot"** (ADR-0030 v1). That contradicts buffered viewing at your own rate. A viewer must be able to see *everything*, at any pace, by reading a persisted stream.

## Decision

1. **The engine is headless.** It has no rendering, windowing or GPU dependency, and it is complete with no viewer attached.
2. **The engine streams its world** (`05.10`): one ordered, append-only, sequenced stream per timeline with two channels.
   - **A (world):** every authoritative event with its causes, every input at its applied tick, a digest per macro step, and keyframe markers. It is byte-identical for the same inputs at any pace.
   - **C (narrative):** sift matches, significance, suspense.
   - **The chronicle is the stream at rest:** sealed segments are the Parquet chronicle.
3. **Viewing is a buffer event.** Each viewer keeps its own cursor and buffer, reading the live edge from memory or sealed segments from disk. Any number of viewers can watch at any speed, rewind or fall behind.
   - Appending never waits for a reader, and nothing is dropped.
   - Exact state at any tick comes from the **materializer** (nearest keyframe plus deterministic replay), which runs outside the engine's simulation threads.
4. **Interaction is a tip event.** An action lands at the next tick boundary after the tip, or exactly at a future tick you schedule. It is journaled with that tick.
   - Every harness shows the tip, its pace, your cursor and the lag, and confirms the landing date before an action.
   - An action at a past moment is refused unless issued as a **fork**.
5. **Pace is yours.** The pace is a speed band or MAX, issued as a control command and logged in the run log (never the input journal).
   - No viewer can slow the engine.
   - Two play styles:
     - **Watch** (default): an auto-pause pauses *your playback*.
     - **Play**: you stay at the live edge, and an auto-pause pauses the tip so you can act at the moment that matters.
6. **Fidelity contract.** Everything the simulation decides is in channel A, and any state is rebuildable exactly.
   - Street detail (positions, faces, idle movement, conversation text) is plane-B hydration: a deterministic function of the stream, seed and Registry. It is computed on the viewer side or in the materializer, never on the engine's simulation threads.
   - The street bubble therefore follows **your playback rate**, not the engine's (amends ADR-0005 §7).
7. **Text only until G2 (phase 1, P0–P2).**
   - `rh-cli tail` renders the stream as filtered text: by significance, place, person, pattern or regional digest.
   - **`rh-tui`** (ratatui + crossterm, MIT; promoted from optional) adds:
     - the chronicle feed;
     - a query prompt with a causal-ancestor tree;
     - an ASCII map;
     - a keyboard Time-Machine over the stream;
     - vitals and the status line;
     - the G2 rating form.
   - **Deferred until after G2:** the web console (ADR-0016), the glyph client (ADR-0029) and the 3D client. All of them read the same stream.
8. **Requirements and tests.**
   - STR-1 to STR-5 and PERF-5 (`03`).
   - V-STR-01 to 05 and V-PERF-09 (`09`): the engine's throughput with viewers attached, a stalled one included, is ≥ 95% of headless.

## Alternatives considered

| Alternative | Why not |
|---|---|
| Push snapshots to viewers and drop what a slow viewer cannot take (v1) | A viewer could not watch the world at its own pace without missing things |
| Stream full state every tick instead of events + keyframes + replay | At 1M persons that is gigabytes per simulated year; deterministic replay gives exact state for a fraction of the volume |
| Engine and viewer in one process (a game loop) | Rendering competes with the tick, and viewer concerns leak into simulation code |
| Let actions land at the moment being watched | That would be back-dating, which ADR-0025 forbids; it is a fork instead |
| A GUI in phase 1 | Effort spent on views before the thesis is tested; text tests the same loop |

## Consequences

- **The stream is phase 1's main product.** The walking skeleton (P1.0) ends at `rh-cli tail`. WP P1.17 builds World Stream v1: channels, segments, cursors, the materializer and V-STR. P2.7 builds `rh-tui` over the stream.
- **Viewers can be replaced freely.** Any later viewer (web console, glyph client, Godot, a phone app, an LLM narrator) reads the same stream. Graphics upgrades never touch the engine.
- **Auto-pause pauses your playback, not the world, by default.** Interventions at the moment that matters use Play style.
- **Actions at MAX land wherever the tip has got to,** possibly years after what you were watching. The harness shows this before you confirm, and scheduled actions give exact timing.
- **Some spatial debugging capability is lost** until a graphical viewer exists. ASCII maps and Rerun (a separate developer tool) cover most of it.

## Revisit when

- G2 raters cannot judge stories without a graphical timeline or map: bring the web console forward.
- V-STR-02 shows replay-based reconstruction is too slow for interactive seeking: add delta records for hot columns to channel A.

## Evidence

- `05.10` (the World Stream spec); `05.9` IF-0, IF-3, IF-4.
- ADR-0005, ADR-0006, ADR-0024, ADR-0025.
- `08` §1 (event volumes) and §5 (keyframe sizes).
- Ratatui (MIT) in `14` §R.
