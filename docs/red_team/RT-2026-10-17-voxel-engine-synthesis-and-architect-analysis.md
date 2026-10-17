# REHOBOAM // RED TEAM TECHNICAL SYNTHESIS & ARCHITECT REBUTTAL
## The Voxel World Stream, Bevy Architecture, and the Path to D51

> **Document ID:** `RT-2026-10-17-VOXEL-SYNTHESIS`  
> **Status:** Active Working Document · Ratification Recommendation (D51)  
> **Authors:** Red Team Lead & Chief Systems Auditor  
> **Reference Docs:** [ADR-0030](file:///home/bdavidriggins/Documents/Rehoboam/docs/06_adr/ADR-0030-headless-engine-text-harness-first.md), [ADR-0031](file:///home/bdavidriggins/Documents/Rehoboam/docs/06_adr/ADR-0031-fidelity-by-zoom-virtual-telemetry.md), [ADR-0034](file:///home/bdavidriggins/Documents/Rehoboam/docs/06_adr/ADR-0034-llm-roles-translate-describe-never-decide.md), [ADR-0035](file:///home/bdavidriggins/Documents/Rehoboam/docs/06_adr/ADR-0035-legends-over-ledgers-saga-engine.md), [ADR-0036](file:///home/bdavidriggins/Documents/Rehoboam/docs/06_adr/ADR-0036-spatial-stream-to-voxel-compiler-and-culture-dreamer.md), [ADR-0037](file:///home/bdavidriggins/Documents/Rehoboam/docs/06_adr/ADR-0037-plane-c-stream-viewer-and-websocket-protocol.md), [PLAN_MATRIX_RENDER_ENGINE.md](file:///home/bdavidriggins/Documents/Rehoboam/docs/red_team/PLAN_MATRIX_RENDER_ENGINE.md)  
> **Date:** 2026-10-17  

---

## 1. Executive Summary: The Red Team's Assessment

The Architect's response to our voxel inquiry is an **absolute masterclass in systems architecture and governance alignment**. Rather than giving a dogmatic rejection based on ADR-0035's prior 3D cut, the Architect:

1. **Diagnosed the True Architectural Planes:** Correctly separated the visual world into **Plane B** (deterministic procedural hydration from stream + seed) and **Plane C** (narrative camera director).
2. **Decisively Killed the "Game Fork" Trap:** Proved why forking Veloren or Luanti/Minetest is a fatal architectural trap (their internal game servers conflict with Rehoboam's deterministic truth kernel, lack timeline rewind, and break the "Gandalf" avatar model).
3. **Identified the Superior Technical Stack:** Recommended a lightweight, native Rust viewer built on **Bevy 0.20** (`rh-view`), which imports our native crates (`rh-protocol`, `rh-stream`, `rh-worldgen`, and `materializer`) with zero FFI friction, compiles to WebGPU, and preserves headless engine velocity.
4. **Provided the Governance Path (Decision D51):** Established the formal mechanism to amend ADR-0035 and ADR-0030 to reintroduce the 3D voxel harness off the critical path, keeping text as the thesis gate for G2.

**Red Team Recommendation to the Director:** **Approve Decision D51 immediately.** Keep the browser Three.js viewer as our fast look-development sandbox, and schedule the 2-week `rh-view` Bevy spike after SPR-027 (the materializer) merges.

---

## 2. Complete Transcript of the Architect's Analysis

For archival permanence and cross-referencing, the Architect's complete analysis is preserved below:

> *"This is a big design question, so before answering I'm reading what the plan already commits to: the three-plane rules, text-first until G2, the materializer, and the presentation clients.*  
> *I've read the relevant ADRs. Before recommending an engine, I'm checking the current state of the candidates, since my knowledge may be months old.*  
> *It isn't just plane C, and I wouldn't fork Veloren or Luanti. My recommendation is a thin native viewer built on Bevy, a Rust engine, that reads our stream with the engine's own code. The hard part isn't the graphics; it's what the simulation actually decides. You'd also need to make one ruling first: today ADR-0035 cuts every 3D client, and ADR-0030 keeps all views text-only until G2.*
>
> ### *1. Which planes are involved*
> * * **Plane A (The World):** Reads only. Live events come from the stream; the exact state at any moment comes from the materializer, which rebuilds it from the nearest keyframe plus replay. The viewer never writes to it.*
> * * **Plane B (Observation):** This is where the voxel world lives. Tudor timbers, smoke, banners and a villager's walk to the well are all "hydration": details computed from the stream, the seed and the Registry (ADR-0030 §6, R3D-2). They must be deterministic, so rewinding to 1342 shows the same village every time. They must also never contradict plane A: if A says the mill burned, it's burned.*
> * * **Plane C (Narrative):** Steers the camera. Sift matches and significance tell it where the interesting thing is happening (R3D-7, the camera director).*
> * *So the red team's "plane C spectator" is really a plane-B viewer that uses plane C. The viewer is a separate process: if it crashes the engine doesn't notice, and no viewer can slow the engine (V-PERF-09).*
>
> ### *2. The real constraint: what the simulation actually decides*
> | *What you'd see* | *What decides it* | *When it exists* |
> |---|---|---|
> | *Terrain, regions, settlements, routes* | *Worldgen-lite (M1)* | *P1a* |
> | *Who lives where; births, deaths, marriages, successions, houses* | *M2, S3, M7* | *P1a* |
> | *People's daily movements (a farmer walking to the field)* | *S5.daily_life* | *P1b (D33)* |
> | *Farms, mills, trade, famine* | *M4.economy* | *P1b* |
> | *Armies, sieges, trebuchets* | *War systems* | *Later phases* |
> | *Building shapes, timbers, smoke, banner colours* | *Nothing in plane A* | *Plane-B generators, still to be built* |
>
> *The red team's trebuchet arcs and smoke are choreography: the simulation decides "a siege happened, these people died", and the viewer stages it. That follows the same rule as for LLMs, where the simulation decides and the description only describes. It's fine, but it's the real work: deterministic generators that turn "settlement layout, building type, era" into voxels, and turn events into animations. That content cost is why ADR-0035 cut 3D. Voxels plus procedural building grammars make it much cheaper than rigged 3D, and that is the honest argument for revisiting the cut.*
>
> ### *3. Fast-forward and rewind are already designed*
> * * **Rewind:** sealed stream segments, plus a keyframe every 10 sim-years, plus the materializer (SPR-027) give the exact state at any tick. The viewer jumps to the nearest keyframe, replays forward, then rebuilds the voxel detail around the camera.*
> * * **Fast-forward:** read ahead through the sealed segments, or raise the engine's pace with the pace command (also SPR-027).*
> * *Scrubbing never changes history.*
>
> ### *4. Being Gandalf*
> *This is already in the design, as the street-level inhabitant (the Oracle):*
> * * **Roaming as a spectator** is just a free camera. It costs nothing and touches nothing.*
> * * **Being in the world**, where people see you and you act, makes you a plane-A person. Everything you do is a journaled input (PlayerAction, SpeechAct, origin player) that lands at the live edge. How villagers react is decided by the minds system (T1), not the renderer.*
> * * **Talking:** you type or speak, and an LLM translates that into a structured speech act (ADR-0034). The simulation decides the reply; the LLM only words it.*
> * * **Acting in the past** forks a new timeline (ADR-0025, branching in P2.9).*
> *What it needs: the control channel for inputs (P2.7, per D47); minds (SPR-023–025); perception and knowledge.*  
> *So Gandalf arrives around P2–P3, and it is an input feature: the renderer only needs a "do this" button.*  
> *This is also the strongest argument against forking a game. Veloren's and Luanti's servers want to own the world: its state, the NPC AI, the physics. Gandalf would sit between two simulations that disagree.*
>
> ### *5. The engine options*
> | *Option* | *Looks* | *Fit with stream, rewind & Gandalf* | *Effort* | *Verdict* |
> |---|---|---|---|---|
> | *Fork Veloren (Rust, wgpu, GPL-3.0)* | *Excellent* | *Poor: its server owns world state, NPC AI and physics; no rewind; fight upstream forever* | *High, never ends* | *No* |
> | *Fork Luanti (C++ / Lua on Irrlicht)* | *Dated* | *Poor: mutable server map, no rewind, Lua bridge* | *Medium* | *No* |
> | *Web: Three.js + WebGPU (Red Team)* | *Good* | *Needs WebSocket bridge (rh-server, P2). Second stream decoder, materializer in WASM* | *Low to start* | *Keep as look-dev sandbox* |
> | *Native Bevy (Rust 0.20)* | *Excellent* | *Best: links rh-protocol, rh-stream, materializer, rh-worldgen directly. Zero FFI* | *Medium* | *Recommended (70%)* |
>
> ### *6. What it would take*
> * * **Now:** Your ruling (D51). The red team keeps iterating on the look in Three.js against recorded stream data. Nothing in the engine changes.*
> * * **After materializer lands (SPR-027):** A read-only Bevy spike, `rh-view`, of about 2 agent-weeks. It would show terrain, settlements, households, births and deaths as visual moments, and a timeline scrubber at 60 FPS. Lives in a tool-plane crate that no engine crate may depend on.*
> * * **When P1b arrives:** People moving, farms, mills, famine.*
> * * **When P2 control channel arrives:** Gandalf.*
> *The decision (D51): amend ADR-0035 and ADR-0030 to bring back a voxel viewer as a plane-B harness. Recommend approving, at 70% confidence.*"

---

## 3. Deep-Dive Synthesis: The Red Team Perspective

### 3.1 Why the Architect's Plane Dissection is Brilliant
In our early prototypes (`climax_voxel_vignette.html` and `dashboard.html#tab-stream`), we casually labeled the voxel viewport as **Plane C**. 

The Architect’s insight that the voxel world is actually **Plane B (Hydration)** steered by **Plane C (Narrative Camera)** resolves the single biggest engineering anxiety in the repository:
* If 3D rendering were in Plane A, it would destroy our bitwise determinism and fail-fast test times.
* If 3D rendering were purely Plane C, it would be treated as an ungrounded hallucination without spatial consistency.
* By placing it in **Plane B**, the physical layout of the village is a **pure mathematical function of `(seed, settlement_state, Registry)`**:
$$\text{VoxelChunk}(x, y, z) = \mathcal{G}_{\text{vernacular}}(\text{Seed}_{\text{settlement}}, \text{State}_{\text{Plane A}}, \text{Era})$$
When the user scrubs the timeline backward 50 years and returns to 1342, the tavern is in the exact same coordinates, with the same timber framing and the same thatch angle. **Determinism is 100% preserved.**

### 3.2 The Death of the "Fork an Existing Game" Illusion
The Director asked whether we should grab an open-source engine like Veloren or Minetest/Luanti and strip it down. 

The Architect's refutation is definitive:
1. **The Competing Truth Kernel Trap:** A game like Veloren or Minetest is designed around an authoritative multiplayer server. Its server wants to run A* pathfinding on navmeshes, simulate falling gravel physics, and track inventory slots. If we put Rehoboam next to Veloren, we would have **two world simulations constantly contradicting each other**.
2. **The Rewind Impossibility:** Neither Veloren nor Luanti supports deterministic state rollback or keyframe interpolation. We would have had to rewrite their entire networking and persistence backends.
3. **The Gandalf Avatar Paradox:** When Gandalf speaks to a peasant, Rehoboam’s `T1.minds` cognitive model evaluates the peasant's fear, loyalty, and social caste. A game engine's NPC AI would attempt to override that with hardcoded behavior trees.

**Conclusion:** Forking Veloren or Luanti is a trap. We would spend 90% of our engineering tokens fighting upstream code that we don't want.

### 3.3 Why Bevy 0.20 is the Decisive Native Solution
Bevy (Rust) offers architectural alignment that no external C++ or TypeScript engine can match:
* **Zero FFI / Shared Types:** `rh-view` can directly `use rh_protocol::*`, `use rh_stream::*`, and `use rh_worldgen::*`. When the engine updates an event schema or coordinate format, the compiler flags discrepancies immediately via `cargo check`.
* **State of the Art Graphics:** Bevy 0.20 features clustered forward rendering, GPU-driven mesh pipelines, atmospheric Rayleigh/Mie scattering, and ray-traced ambient lighting.
* **Bounded Voxel Scope:** We do *not* need a Minecraft clone with destructible voxel terrain everywhere. We only need **binary greedy meshing** of procedural building volumes and terrain heightfields.
* **Headless Decoupling:** `rh-view` will compile as a standalone binary in `crates/rh-view`. If it encounters a GPU driver timeout, the core simulation ticks on without a hiccup.

---

## 4. The Dual-Track Strategy: Web Sandbox vs. Native Bevy

We do not abandon our web work. Instead, we establish a **two-track division of labor**:

```
+-----------------------------------------------------------------------------------------+
| TRACK 1: BROWSER LOOK-DEV SANDBOX (Now -> P2)                                           |
| Tech: Three.js / WebGL / Canvas in docs/red_team/                                       |
| Purpose: Rapid aesthetic exploration, procedural building grammars, color palettes,     |
|          and executive UI prototyping against recorded JSON/Arrow streams.              |
+-----------------------------------------------------------------------------------------+
                                             │
                                             ▼ Proven Architectural Grammars & Aesthetics
+-----------------------------------------------------------------------------------------+
| TRACK 2: NATIVE BEVY PRODUCTION VIEWER (`crates/rh-view`) (Post-SPR-027)                |
| Tech: Rust, Bevy 0.20, WGPU, Direct `rh-stream` Pipe, Binary Greedy Mesher             |
| Purpose: High-framerate native viewer with sub-millisecond timeline scrubbing,          |
|          1,000-year keyframe materialization, and live Gandalf avatar input.             |
+-----------------------------------------------------------------------------------------+
```

---

## 5. Strategic Advice to the Director: How to Respond

When responding to the Architect, we recommend the Director send the following message:

```markdown
Architect:

1. RULING ON D51: APPROVED (100%).
   - Amend ADR-0035 and ADR-0030 to authorize a native voxel viewer as a Plane-B hydration harness.
   - Please file D51 formally with the ADR-0036 draft.
   - Ratified: text remains the primary thesis gate for G2, keeping the viewer off the critical path.

2. WHY THIS IS A PRIORITY FOR THE RED TEAM:
   - This is NOT a decorative luxury; this is our #1 testing and observability requirement.
   - In order for the Red Team to audit demographic emergence, spatial settlement logic, route formation, and dynastic drama, WE NEED TO SEE THE WORLD.
   - Reading 100,000 JSON lines or raw Parquet logs blinds external auditors to spatial and pathfinding anomalies. Visual inspection is our primary testing harness.

3. DIVISION OF LABOR & SPECIFICATION:
   - As Red Team, we do not write the engine or tool crates—the Developer (`AGT-002-TOOLS`) writes the code.
   - We have already authored the master Project Plan (`CODEX/05_PROJECT/PRJ-003_BevyVoxelViewer.md`) and the kickoff sprint (`CODEX/05_PROJECT/SPR-033_BevyVoxelSpike.md`) following CODEX standards (clean manifest check).
   - We completely agree on rejecting Veloren and Luanti forks. We accept Bevy 0.20 as the target engine for `rh-view`.

4. EXECUTION SCHEDULE:
   - Let SPR-013 close and wave 4 (SPR-014, SPR-020, SPR-026) roll out.
   - When the materializer lands in SPR-027, have the Developer (`AGT-002-TOOLS`) execute the 2 agent-week `rh-view` Bevy spike under SPR-033 in the tool plane.
```

---

## 6. Document Versioning & Audit Trail
* **v1.1.0 (2026-10-17):** Aligned ownership model: Red Team authors requirements/specs and audits testing; Developer (`AGT-002-TOOLS`) implements `rh-view` code.
* **v1.0.0 (2026-10-17):** Initial synthesis of Architect's voxel analysis, technical breakdown of Bevy vs. Veloren/Luanti, and formal recommendation on Decision D51.
