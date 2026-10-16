# ADR-0030 — Headless engine; every view is a harness; text-only until G2

**Status:** Proposed (director's direction, 2026-10-16) · **Date:** 2026-10-16 · **Amends:** ADR-0016 (web console moves after G2), ADR-0029 (no graphical view before G2), `05.7`, `05.9`, `09`, `11` (P1.0, P2.7)

## Context

The director wrote:

> *"For phase 1, we should be text only interface? The world should still exist in the engine but my viewing and interaction is completely secondary. The world engine should churn ... like the Matrix. When I interact I am using a separate harness. My meaty needs should not interfere with the simulation at all. Why have graphics stuff bottlenecking the simulation?"*

**Most of this already holds.**
- **Watching is not an input.** History = f(engine build, Registry version, manifest hash, seed, input journal) (ADR-0006).
- **Plane A never reads plane B (observation) or plane C (narrative)** (ADR-0024).
- **Observer invariance (DET-6) is binding from P2** (V-DET-06–08). The world is run with and without a fuzzer poking at it, and the state hash must be identical.
- **Outcomes fire whether anyone is watching or not** (ADR-0025).
- **Clients decide nothing** (ADR-0029 §4).

**This is standard practice, and stricter than most games.**
- Agent-based toolkits such as MASON keep the model separate from its visualisation: the model runs headless, and a GUI attaches and detaches at will. Deterministic game replays and server-authoritative online games follow the same pattern.
- Many games simulate in more detail near the player, so history depends on where the player looks. Rehoboam does not. Detail near a viewer (hydration) lives in plane B and cannot feed back.

**Two things do not hold yet.**
1. **Performance isolation.** The plan proves that watching cannot *change* history, but not that it cannot *slow* it. A viewer costs engine time in three places:
   - hydration of plane-B detail inside the engine process;
   - encoding and sending IF-4 frames;
   - synchronous control-plane queries against live state.

   If any of these runs on the tick thread, or holds a lock the tick needs, a slow viewer throttles the world.
2. **The first views are graphical.** The walking skeleton (P1.0) ends in the web console with a glyph view. P2.7 builds a deck.gl map and a Sigma.js graph. ADR-0029 aims for a Diorama during P3. That is effort spent on views before the engine has proved itself at G2.

## Decision

### 1. The engine is a headless process
- `rh-server` and `rh-cli run` have no rendering, windowing or GPU dependency.
- The engine runs at full speed with nothing attached, and it is complete without a viewer.

### 2. Every view and every interaction is a harness
- A harness is a separate process, or at minimum a separate thread pool, that uses only the published interfaces:
  - the frame stream (IF-4);
  - the chronicle (Parquet through DuckDB);
  - the control plane (IF-3);
  - the Python bindings (IF-5).
- Harnesses include `rh-tui`, `rh-cli`, notebooks, LLM agents, and later the web console, the glyph client and Godot.
- Any number of harnesses can attach, detach, or run on another machine.

### 3. There are two doors and no others
- **Read door.**
  - Immutable snapshots (`Arc<SnapshotFrame>`) are published at tick boundaries.
  - "Now" queries read the latest published snapshot. History queries read the chronicle.
  - A read never takes a lock the tick loop needs.
- **Write door.**
  - Material inputs (`player.act`, Oracle verbs, director edits) enter the input journal and take effect at the next tick boundary, identically on replay (ADR-0025). Acting in the past forks.
  - Non-material commands (pins, inspections) touch plane B only.

### 4. The tick loop never waits for a harness
- **Publication is fire-and-forget** into a bounded ring of snapshots.
  - A slow consumer skips to the latest snapshot: its deltas are coalesced into a full frame.
  - A slow consumer never back-pressures the engine, and a stalled one costs at most the ring's memory.
- **Hydration runs on its own worker pool** with a CPU budget (`observer_cpu_share`: default 25% of cores in interactive mode, 0 in batch mode).
  - When the budget runs out, hydration degrades first: fewer named people, coarser LoD.
  - The tick rate never degrades.
- **Rendering never happens in the engine process.**
- **Batch mode** (certification, Foresight rollouts, liveness gates) runs with no harness and no hydration at all.

### 5. Text only until G2
In phase 1 (P0–P2, up to the thesis gate) every harness is text.

**`rh-tui`** (ratatui + crossterm, MIT) is promoted from optional to the phase-1 harness. It shares the Rust frame decoder. Its panels:

| Panel | Contents |
|---|---|
| **Chronicle feed** | The live event stream (the "Matrix rain"), filterable by significance, place, person or pattern, with auto-pause marks |
| **Query prompt** | DuckDB SQL plus canned queries: who, where, and *why* (a causal-ancestor tree walk) |
| **Map** | An ASCII/CP437 map of regions and settlements, drawn from the same IF-4 frames and Registry glyph keys that the glyph client will use later |
| **Time-Machine** | A keyboard timeline: speed bands, run-until, read-only seek, fork |
| **Vitals** | Ticks/s, plane hashes, invariant counters, firing rates |
| **Rating form** | The G2 rating tool, in text |

**`rh-cli`** covers scripted runs, exports and the G2 protocol tooling.

**Deferred until after G2:**
- the web console (ADR-0016);
- the deck.gl map and the Sigma.js causal graph;
- the glyph client (ADR-0029).

They are built against the same interfaces, so nothing in the engine changes when they arrive. ADR-0029's early Diorama in P3 is dropped. The lo-fi Diorama stays in P5.

### 6. A new performance requirement
**PERF-5, harness isolation:**
- the engine's throughput with harnesses attached is ≥ 95% of its throughput with none;
- a stalled harness causes no engine slowdown and no memory growth beyond the snapshot ring;
- the state hash is unchanged (V-PERF-09).

## Alternatives considered

| Alternative | Why not |
|---|---|
| Keep the web console and glyph view in P1–P2 | They are effort on views before the thesis is tested. A text harness tests the same loop: world → chronicle → sifter → auto-pause → why |
| Plain logs and SQL only, no TUI | Too thin to *watch* centuries pass or to judge auto-pause; G2 needs a usable Time-Machine |
| Engine and viewer in one process (a typical game loop) | Rendering competes with the tick, and viewer state drifts into simulation code. It is the bottleneck the director wants to avoid |

## Consequences

- **The phase-1 harness is text-based.** P1.0's skeleton ends at `rh-tui`. P2.7 becomes "Time-Machine harness v1 (text)".
- **Spatial debugging is weaker** without the deck.gl map. The ASCII map and Rerun (a separate developer tool) cover most of it.
- **Causal graphs are shown as text trees in P2.** That suffices for "why did this happen?" but not for browsing a large graph, which waits for the web console after G2.
- **The schedule is roughly neutral overall.** P2 should shrink by about 1–2 weeks, and the web console work, about 3–4 weeks, moves after G2 alongside P5. The risk moves out of the thesis path.
- **The ambient experience arrives later.** The living-wallpaper Diorama comes in P5 as before; the P3 preview is dropped.
- **Observer invariance gains a performance half.** Watching can change neither what happens nor how fast it happens.

## Revisit when

- The G2 raters cannot judge stories without a graphical timeline or map. In that case, bring the web console forward.
- `rh-tui` cannot meet MOP-3 (scrub < 100 ms) over a terminal.

## Evidence

- `05.9` IF-3, IF-4 and IF-5.
- ADR-0006, ADR-0024, ADR-0025, ADR-0029.
- `09` V-DET-06–08 (observer invariance) and V-PERF.
- Ratatui (MIT) is already in the reuse inventory (`14` §R).
