# Red-team proposal: Spatial Stream-to-Voxel Compiler and LLM Culture Dreamer

*Drafted on the red team's branch as "ADR-0036" (prototype P14, RT-032). It is a proposal, not an ADR: `main`'s ADR-0036 is `ADR-0036-voxel-viewer-observation-harness.md` (D51).*

**Status:** Proposed (Red Team Prototype P14, RT-032) · **Date:** 2026-10-16 · **Amends:** ADR-0010, ADR-0018, ADR-0024, ADR-0030, ADR-0031, ADR-0034, ADR-0035 · **Spec:** `05.10` §4, `05.7` §3

> **Architect's intake note (2026-10-17).** This proposal enters `main` as the red team's record, at the director's request. It is not a decision. Its text is kept as written, except for the title line, the prototype link and the elided model names. **Where each part goes in the roadmap** (`docs/11` viewer lane, ADR-0036):
>
> | Part | Where | Conditions |
> |---|---|---|
> | Shape grammar: footprints, framing, roofs, apertures, water, the three voxel resolution tiers, binary greedy meshing, colour jitter | **Viewer lane V.1** (SPR-033 and its follow-ons, after SPR-027) as R3D-2's versioned generators, ported to Bevy in `viewer/rh-view` | The jitter, like every other "random" choice, comes from a keyed hash of `(seed, entity, generator version)`, never from a client RNG. The voxel content is hashed (V-R3D-13). The Three.js prototype stays the red team's look-development sandbox (PRJ-003) |
> | **Plane** | **Plane B** (hydration), steered by plane C | ADR-0036 §1 corrects this proposal's "Plane C". The physical layout is a pure function of the stream, the seed and the Registry |
> | Culture Dreamer: LLM-written style cards | **A new viewer-lane item, V.1b** (proposed), after the V.1 spike | **The director decides (proposed D55).** Style cards are generator data ("describe out", ADR-0034). They are written once, offline, reviewed, and **committed with the world's assets as versioned, hashed files**, never cached in browser storage or fetched at view time, so that every machine renders the same village. Until `S1.culture` gives regions a culture, a card is keyed by era and biome; it is per culture after that. They join SAGA-6's visual style card for the graphic chronicle (P3). A network LLM provider from the client is not accepted: any LLM call is an offline authoring step under D29's budget rule |
> | The "events" in its Context (`SETTLEMENT_SPAWN` and others) | Not adopted | The viewer reads the stream's real records (`canonical_v0.md`, `wire_v1.md`) and the materializer. Settlement records arrive with worldgen-lite (SPR-018) |

---

## Context

Under ADR-0030 ("The Engine Only Engines") and ADR-0035 ("Legends Over Ledgers"), Plane A (the Rust simulation core) maintains strict headless determinism. It models demographic survival, harvest yields, dynastic claims, and military clashes. It emits an immutable Arrow/Parquet world stream (`rh-stream`).

Plane A does **not** store 3D vertices, door hinges, chimney smoke particles, or wall paint colors. Downstream visual spectators (Plane C) receive high-level simulation events such as:
```json
{
  "event": "SETTLEMENT_SPAWN",
  "settlement_id": "burg_8921",
  "tier": "HAMLET",
  "seed": "0x65",
  "population": 48,
  "biome": "subarctic_fjord",
  "wealth": 42
}
```

This raises three architectural questions for Plane C visualization:
1. **Fidelity Gap:** How does Plane C synthesize rich, realistic architectural anatomy (floors, roof pitch, door placement, desire paths, watermills) without hardcoded models or bloating Plane A?
2. **LLM Boundary:** How do we leverage Large Language Models to "dream" rich cultural aesthetics and varied regional architecture without violating ADR-0034 ("LLMs translate in and describe out; they never decide") and without stalling real-time 60 FPS streaming?
3. **Performance & Voxel SOTA:** What is the peak geometric fidelity achievable in the browser for rural keeps and imperial metropolises, and how do we prevent the flat, plastic aesthetic of naive voxel implementations?

---

## Decision

We establish the **Spatial Stream-to-Voxel Compiler Engine** and the **Declarative Culture Dreamer Architecture** as the official Plane C settlement visualizer standard.

### 1. The Tripartite Division of Labor

```
+-----------------------------------------------------------------------------------+
| PLANE A: RUST SIMULATION CORE                                                     |
| Simulates causal state: demographics, harvest yields, stress breaks, and wars.    |
| Emits binary event stream (Arrow / Parquet / WebSocket). Zero 3D code.            |
+-----------------------------------------------------------------------------------+
                                          │
                                          ▼
+-----------------------------------------------------------------------------------+
| LLM CULTURE DREAMER (At World Creation / Biome Intake)                            |
| "Dreams" regional architectural vernaculars into Declarative Style Cards (JSON).  |
| Materials, structural rules, roof pitches, and cultural lore. NEVER coordinates.  |
+-----------------------------------------------------------------------------------+
                                          │
                                          ▼
+-----------------------------------------------------------------------------------+
| PLANE C: PROCEDURAL SPATIAL SHAPE GRAMMAR (Client Side)                           |
| Interprets Style Card & Stream Seed (0x65) via mathematical split grammar.        |
| Compiles watertight 3D voxels deterministically in < 25 ms @ locked 60 FPS.       |
+-----------------------------------------------------------------------------------+
```

### 2. The LLM Culture Dreamer Contract

In accordance with ADR-0034:
1. **No LLM in the Hot Loop:** LLMs are **never** called during tick step simulation or frame rendering. They never output raw 3D mesh arrays or voxel coordinate blobs $(x,y,z)$.
2. **Declarative Output:** LLMs synthesize only structured **Architectural Vernacular Style Cards** adhering to the typed JSON schema:
   - `foundation`: quarried stone type (`cobble`, `stonebrick`, `travertine`)
   - `timber`: framing wood type (`wood`, `woodDark`, `charred_pine`)
   - `infill`: wall material (`plaster`, `plasterWhite`, `adobe`, `wattle_daub`)
   - `roof`: roofing system (`thatch`, `sod`, `terracotta_tile`, `slate`)
   - `chimney`: masonry stack type (`cobble`, `stonebrick`, `brick`)
   - `amenities`: economic and cultural props (`hasMill`, `flowerBoxes`, `hearth`)
3. **Pluggable Multi-Provider Studio:** The client supports direct integration with **OpenRouter.ai** (enabling any model; the model names are elided in this copy), local Ollama instances, or offline deterministic synthesizers.

### 3. Deterministic Culture Caching Doctrine

To eliminate latency, prevent hallucination drift across runs, and achieve zero recurring token cost:
- All LLM-generated Style Cards are keyed by `hash(provider + model + culture_prompt)` and cached in persistent client storage (`localStorage` / IndexedDB).
- When a settlement or region in Plane A emits a known culture tag or stream seed, Plane C performs a **Cache Lookup (0.0 ms)**.
- Re-querying an LLM for an already-dreamed culture is strictly prohibited unless explicitly requested by the Director via the Admin Studio.

### 4. Peak SOTA Micro-Voxel Standards

To transcend naive block-building aesthetics, the Spatial Grammar supports three discrete resolution tiers:
- **Standard (1.0m):** Macro footprint validation (~5,000 voxels, < 5 ms compile).
- **Enhanced (0.65m):** Detailed settlement overview (~18,000 voxels, < 12 ms compile).
- **👑 Peak SOTA Micro-Voxel (0.35m–0.38m):** High-fidelity architectural realization (~28,000–75,000 voxels, < 35 ms compile, locked 60 FPS):
  1. **Sub-Voxel Framing:** Exposed Tudor timber post-and-beam frameworks featuring 3D diagonal tension cross-braces (*St. Andrew's cross* timbers) and corbel overhangs.
  2. **Multi-Tier Stepped Thatch Roofs:** 3-tier overlapping straw layers with natural eaves sag, woven straw ridge crests, and rake verge boards.
  3. **Living Apertures:** Recessed window frames with leaded mullions, warm amber glowing lantern cores, and windowsill planter boxes with vibrant red/yellow floral micro-voxels.
  4. **Interior Living Hearth:** Open entryways revealing internal stone fireplaces with glowing orange ember voxels and chimney mantles.
  5. **Hydrological Sluice & Foam Spray:** Deep navy river channels transitioning to sparkling turquoise shallows and wet mossy banks, with animated white foam spray particles churning around the 12-paddle millwheel.
  6. **Per-Voxel Procedural HSL Color Jitter:** GPU-instanced colors (`instancedMesh.setColorAt`) inject organic micro-noise ($\pm 6\%$ lightness/hue) into quarried stone, thatch straw, and timber grain, eliminating flat plastic rendering.

---

## Verification & Prototype Evidence

The complete architecture has been implemented and verified in Prototype 14:
- Reference Implementation: `docs/red_team/prototypes/voxel_stream_compiler.html` (on the red team's branch `arch/red-team-batch-04`; not on `main`)
- Telemetry Benchmark:
  - Voxel Count: 28,985 micro-voxels (Anglo-Saxon Shire)
  - Compile Latency: 28.30 ms
  - GPU Batches: 13 draw calls
  - Framerate: 60.0 FPS locked
  - Cache Performance: 19 hits / 2 misses (~24,800 saved tokens)
  - Custom Culture Dreaming: Verified with OpenRouter API integration and prompt chips (Gothic Vampire, Feudal Japanese, High Elven).

---

## Consequences

- **Zero Plane A Bloat:** Plane A remains 100% headless, fast, and pure Rust.
- **Zero Simulation Stalls:** World streaming is completely decoupled from LLM inference latency.
- **Deterministic Aesthetics:** Cached Style Cards guarantee that revisiting a province always renders identical cultural architecture.
- **Cinema-Grade Web Spectator:** WebGL/Three.js easily renders rich, living settlements in sub-second load times without desktop native C++ build hurdles.
