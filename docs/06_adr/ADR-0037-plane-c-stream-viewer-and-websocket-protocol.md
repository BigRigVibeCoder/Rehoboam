# ADR-0037 — Plane C Simulation Stream Viewer and Live WebSocket Protocol

**Status:** Proposed (Red Team Prototype P15, RT-033) · **Date:** 2026-10-16 · **Amends:** ADR-0010, ADR-0018, ADR-0030, ADR-0036 · **Spec:** `05.10` §5, `05.7` §4

---

## Context

ADR-0030 ("The Engine Only Engines") and ADR-0036 ("Spatial Stream-to-Voxel Compiler") established the boundary between Plane A's headless causal simulation and Plane C's procedural 3D client rendering. However, to complete the vision of Rehoboam as a living world engine, downstream viewers must connect to a continuous, real-time event feed.

Spectators require:
1. **Live Simulation Telemetry:** Continuous delivery of calendar ticks (Year, Season, Month), demographic shifts, agricultural yields, treasury flows, and military casualties.
2. **Bidirectional Control Protocol:** Ability to pause, step, or adjust simulation speed (1x, 2x, 5x, 10x), and trigger tactical world events (e.g. siege artillery volleys, dragon strafes) without compromising Plane A's Single Writer Invariant.
3. **First-Person Exploration:** Ground-level camera perspectives (1.6m eye height) with WASD movement and mouse look, allowing users to walk through hamlets, cross bridges, inspect Sakura gardens, and enter citadel gatehouses.
4. **Zero-Dark-Failure Resiliency:** Viewers must function seamlessly whether connected to a live cluster, a local development daemon, or operating offline in detached playback mode.

---

## Decision

We establish the **Rehoboam Simulation Stream Viewer** and the **RFC-6455 Plane A WebSocket Stream Protocol** as the standard bridge between the simulation core and visual spectator clients.

```
+──────────────────────────────────────────────────────────────────────────────+
|                           PLANE A SIMULATION CORE                            |
|             (Headless Rust Causal Engine / Simulation Daemon)                |
|           • Computes population dynamics, grain stocks, and wars             |
|           • Broadcasts binary JSON frames via RFC-6455 WebSocket             |
+──────────────────────────────────────────────────────────────────────────────+
                                      ▲  │
                  Upstream Commands   │  │  Downstream Event Stream
                 (PAUSE/SPEED/STEP)   │  │  (SIMULATION_TICK / BREACH / DRAGON)
                                      │  ▼
+──────────────────────────────────────────────────────────────────────────────+
|                         PLANE C STREAM VIEWER TOOL                           |
|                   (tools/stream_viewer/index.html)                           |
|                                                                              |
|  [TOP TELEMETRY RIBBON]                                                      |
|  • Connection Status: 🟢 LIVE WS (8089) / 🟡 OFFLINE FEED                   |
|  • Macro Calendar: Year 1068 · Summer (Jul) #1030                            |
|  • Demography: Pop 1,433 | Grain 4,661 bu | Gold 970 oz | Army 120           |
|  • Playback: ⏸️ Pause | 1x | 2x | 5x | 10x | ⏭️ Step                         |
|                                                                              |
|  [VIEWPORT PERSPECTIVES]                                                     |
|  • 🎥 Strategic Orbit Cam: High-angle regional tactical overview             |
|  • 🚶 Ground Walk Cam: 1.6m eye-level WASD walking + 360° mouse look         |
|  • 🐉 Dragon Rider Cam: Mounted chase perspective on flying Crimson Dragon   |
|  • 🏹 Battlement Cam & ☄️ Artillery Cam: First-line siege perspectives       |
|                                                                              |
|  [VOXEL STREAM COMPILER]                                                     |
|  • Real-time wall destructibility from WALL_BREACH_EVENT packets             |
|  • Instant physical rubble tumbling and volumetric explosions               |
|  • Luanti-grade aesthetic fidelity (Willows, Sakura, PBR water, Fireflies)   |
+──────────────────────────────────────────────────────────────────────────────+
```

### 1. Zero-Dependency Stream Daemon (`bin/stream_server.py`)
To prevent virtual environment dependency fractures across development platforms, the WebSocket server is implemented purely in the Python 3 standard library (`asyncio`, `hashlib`, `base64`, `struct`):
- Implements the complete RFC-6455 handshake (`Sec-WebSocket-Accept` SHA-1 key hashing) and frame encoding/decoding.
- Listens by default on `ws://0.0.0.0:8089`.
- Broadcasts simulation ticks at configurable intervals (default: 1.2s per tick).
- Supports concurrent multi-client broadcast with automatic dead-connection pruning.

### 2. Stream Wire Protocol Specification

#### Downstream Packets (Server → Client)
- `SIMULATION_TICK`: Macro state snapshot containing calendar year/season/month, demography population, grain stockpiles, gold treasury, garrison troop counts, and fortress wall integrity.
- `HARVEST_YIELD`: Field crop yields updating agrarian grain reserves.
- `TRADE_CARAVAN`: Inbound merchant arrivals incrementing gold treasury.
- `POPULATION_CENSUS`: Demographic headcount updates.
- `WALL_BREACH_EVENT`: Spatial breach coordinates `(impact_x, impact_y, impact_z)` and count of destroyed stone voxels, instructing Plane C to collapse instanced wall blocks into tumbling physical rubble.
- `DRAGON_COMBAT_ACTION`: Entity strafe event instructing Plane C to spawn volumetric flame boxes and aerial flight pitch.
- `CITADEL_REPAIRED`: Reconstruction event triggering instanced matrix restoration.

#### Upstream Packets (Client → Server)
- `PAUSE` / `RESUME`: Halts or resumes the background simulation loop.
- `SET_SPEED`: Adjusts simulation tick interval (`speed`: seconds per tick).
- `STEP`: Forces a single discrete simulation step immediately.
- `TRIGGER_SIEGE`: Commands the artillery battery to target fortress curtain walls.
- `TRIGGER_DRAGON`: Commands the aerial dragon to execute a fire-breathing strafe.
- `REBUILD_CITADEL`: Restores curtain wall integrity to 100%.

### 3. First-Person Ground Walking Engine
To allow human evaluators to visually audit settlement anatomy at street level:
- Mode toggled via `🚶 Walk (1.6m)` or keyboard shortcut.
- Camera position is decoupled from orbit controls and locked to eye level (`ground_height + 1.4m`).
- Continuous ground elevation tracking prevents clipping into terrain contours or hillocks.
- Full WASD translation with Shift sprint multiplier (12.0 m/s) and mouse drag orientation (-1.3 to +1.3 rad pitch clamping).

### 4. Canonical Deployment & Tooling
- **Production Tool:** `tools/stream_viewer/index.html`
- **Prototype Lab:** `docs/red_team/prototypes/voxel_stream_compiler.html`
- **One-Step Launcher:** `bin/run_stream_viewer.sh` (probes and launches both WebSocket daemon and HTTP server).

---

## Consequences & Invariants

1. **Pure Presentation Decoupling:** Plane C contains zero world simulation logic. If the stream client disconnects, Plane A simulation continues without alteration.
2. **Zero-Dark-Failure Fallback:** If the WebSocket server is unreachable or offline, the stream viewer immediately activates an internal tick simulator so the UI remains interactive and responsive without errors or blank screens.
3. **No External Pip Dependencies:** The streaming server requires zero third-party packages, guaranteeing instant execution on any system with standard Python 3.
