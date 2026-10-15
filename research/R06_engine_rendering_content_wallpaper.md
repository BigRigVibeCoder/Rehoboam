# R06: Engine, Rendering, Art Direction, Procedural World Content and Desktop-Wallpaper Hosting

*Research dossier for Rehoboam, as of 2026-10-15.*

**How this was verified.** The shared web-search quota ran out early in the session, and page fetching could only reach GitHub-hosted material. Engine facts were therefore checked against primary sources on GitHub: the source of the Godot and Bevy websites (release posts and feature lists), the Godot docs source, the Godot class reference, and project READMEs and LICENSE files. Claims tagged **(R)** come from the analyst's recall of sources up to mid-2026 and were **not** re-verified here. Treat them as leads to confirm. Statements labelled *Inference* are the analyst's own judgement.

---

## TL;DR

1. **Engine: stay on Godot. Verdict: GO.** Godot 4.7.2 (released 2026-08-18) is the current stable version, and 4.8 is in development (dev7, 2026-09-29) [1]. Godot is MIT-licensed and charges no royalty. It can be embedded as a wallpaper (`--wid`, native window handles), and 4.6 added LibGodot [4][16][17]. The custom crowd system should be written in GDExtension (Rust gdext 0.5.x or C++) [25].
2. **Godot's weakest area is the one Rehoboam needs most: animated crowds.** Skeletal animation runs on the CPU. About 100 rigged characters can already cause lag, and reaching 1,000 needs vertex-animation textures (VAT) [18][19][20]. A tiered crowd LOD (rig → VAT MultiMesh → impostor) is **mandatory**, not polish.
3. **Large worlds are not a real problem.** The client only renders a local slice, so a floating origin is enough. Double-precision builds mean recompiling the editor and export templates and rebuilding every GDExtension. Godot's own docs say they are unnecessary for an on-foot area under 8 km [9].
4. **The alternatives lose for this team.** Unreal 5.x is the only engine with ready-made crowds, traffic and PCG (Mass, City Sample), but it means C++ and Blueprint costs, a 5% royalty above $1M [73] (R), and an AAA-class runtime that is wrong for an always-on wallpaper. Bevy 0.20 (2026-10-08) still has no official editor [30][32]. Unity 6's licensing is calm again, but its DOTS animation story is weak (R). Stride 4.4 and O3DE 2605 are niche [36][37].
5. **A stylized look is required by the scope, not a matter of taste.** Use a "painted miniature" style: low-poly meshes, a shared palette atlas, a colour grade (LUT) per world or era, and tilt-shift depth of field in Diorama mode. Godot 4.7's AreaLight3D and HDR output support the neon-megacity variant [3].
6. **The biggest gap in the draft is street-level content.** Proposed pipeline, driven by a hierarchy of seeds:
   - a macro layer in the style of Azgaar's generator (MIT, so it can be ported) [39–41];
   - a settlement plan, using a Watabou-style ward plan for medieval towns and tensor-field streets for modern cities [42][43];
   - plots;
   - buildings assembled from kit pieces by a grammar or wave-function-collapse (WFC) solver onto a GridMap [3][44];
   - lazy interiors, built only when someone enters.
7. **Characters:** use CC0 bases (Quaternius Universal Animation Library, Kenney) [60][61], Mixamo, and paid Synty kits (R), all on **one** humanoid skeleton profile [12]. The draft's `hash mod N` assignment must become semantically filtered, stable rendezvous hashing.
8. **AI 3D generation is useful offline for the long tail of props.** It is not good for layered rigged characters or for kit pieces that must snap to a grid. Licensing traps:
   - Every open Hunyuan3D licence excludes the EU, the UK and South Korea, **including outputs** [45].
   - TRELLIS.2 is MIT-licensed, but it depends on NVIDIA nvdiffrast, which is licensed for research and evaluation only [47][48].
   - HY-Motion carries the same Tencent territory clause [55].
9. **Generative world models are not viable for a sim-consistent wallpaper in 2026.** HY-World 1.5 runs at 480p and needs 28–72 GB of VRAM [57]. Matrix-Game 4.0, released today, reaches about 38 FPS at 864×480 on an A100 [58]. Its architecture is the one to watch: the engine stays authoritative and a video model only re-renders low-poly frames.
10. **Wallpaper hosting:** build our own Windows WorkerW host, and also ship a Lively Application-Wallpaper package. Lively's pause rules are **disabled for application wallpapers**, so Rehoboam must throttle itself [63]. macOS and Linux are best-effort.
11. **IPC:** keeping the sim and the renderer in separate processes is right (crash isolation, and the renderer can pause while the sim runs). Sending bulk entity state as JSON-RPC is wrong. Use binary, interest-managed snapshots that carry verbs and waypoints, and keep JSON-RPC as the control plane.
12. **The "100 m hydration radius" contradicts the vistas** described for the Diorama. Replace it with screen-space LOD hydration that reaches out to 1–2 km.

---

## Draft claims checked

| Claim in the draft | Verdict | Evidence |
|---|---|---|
| A Godot 4 client is suitable | **Supported, with caveats** | 4.7.2 is stable. 4.8 adds mip-level texture streaming (merged 2026-08-27) [1][5][6]. Crowd animation is the bottleneck [19][20]. |
| "Godot procedural mesh generation and terrain heightmaps" cover the world | **Partial: a major gap** | Terrain3D (MIT, v1.0.2) handles heightmaps from 64 m up to 65.5 km [23]. Nothing in the draft generates roads, buildings, settlements or interiors (see §4). |
| Python FastAPI → WebSocket JSON-RPC 2.0 stream | **Keep the split, change the payload** | No in-process Python is viable: godot-python's Godot 4 branch is "totally incompatible" and under refactor [26], and py4godot is "early… treat as a demo" [27]. JSON for 10³–10⁴ entities per tick is wasteful (*Inference*, §8). |
| PixiJS canvas as the alternative | **Not an alternative for 3D modalities** | PixiJS v8.22 is a 2D renderer [70]. It suits a 2D map or Observer view. The web-3D option is three.js r186 with WebGPU [71]. |
| AssetKey = hash(entity_id) mod variants | **Deterministic but fragile** | Adding a variant reshuffles almost every entity, and there are no semantic constraints. MultiMesh custom-data channels are scarce: OpenVAT reserves them [14][22]. |
| Universal rig plus a blend tree driven by verbs | **Feasible near the camera only** | SkeletonProfileHumanoid supports retargeting [12][15]. AnimationTree is CPU-expensive per character [19]. |
| 100 m hydration radius | **Incompatible with the described vantages** | River bends, ramparts and intersections are seen at 100 m to 2 km. Hydration needs screen-space tiers (§5.3). |
| "Visual immersion of modern 3D engines" | **Hype risk** | Godot has no virtual geometry or GPU-driven pipeline in release notes up to 4.8-dev7; it relies on mesh LOD, HLOD and occlusion culling [3][4][13]. Realism costs content (Manor Lords, R). |
| Large-world support needs special handling | **Mostly a non-issue** | Godot's docs say an on-foot area under 8,192 m is fine in single precision, and double precision needs custom builds [9]. |
| Godot web export as a fallback | **Limited** | WebGL2 through the Compatibility renderer only, no WebGPU, C# cannot export to the web, and single-threaded export is the default [10][11]. |
| A game executable can be the wallpaper | **Yes on Windows** | Lively runs Godot and Unity executables [62][63]. Windows 11 24H2 forced Lively to rewrite its core [65]. |
| Kitbashed low-poly characters with a CorruptionDecal | **Feasible, but implement the decal as a shader mask** | Decal nodes project in world space (*Inference*). MultiMesh ignores blend shapes [14]. |

---

## Landscape and state of the art

### 1. Godot as of October 2026

| Version | Date | Relevant contents |
|---|---|---|
| 4.5 | 2025-09-15 | Stencil buffer; AccessKit screen reader [72]; shader baker (R) |
| 4.6 | 2026-01-26 | Jolt is the default 3D physics; new IK framework (TwoBoneIK3D, FABRIK3D, CCDIK3D, SplineIK3D); SSR overhaul with a half-resolution mode; octahedral reflection probes; **D3D12 is the default on Windows**; **LibGodot** embedding; Tracy and Perfetto profiling [4] |
| 4.7 | 2026-06-18 | HDR output on Windows, macOS and Linux (Wayland); **AreaLight3D** rectangular lights (for neon signs and windows); **MeshLibrary editor** for GridMap; Asset Store; more than 1,600 pull requests [2][3] |
| 4.7.2 | 2026-08-18 | Current stable maintenance release [1][8] |
| 4.8-dev | dev1 2026-07-06 to dev7 2026-09-29 | Mip-level **texture streaming** with a budget (built for "large, open worlds"); multi-bounce AO; directional lightmap specular; decals in the Compatibility renderer; contact shadows; animated decals [5][6][7] |

**Large worlds and terrain.** Double precision requires custom engine builds, rebuilt GDExtensions, and accepted rendering caveats: triplanar materials and world-space shaders jitter [9]. Rehoboam renders at most a few square kilometres around the camera, so shifting the origin per chunk is enough (*Inference*).

| Terrain option | Version | License | Status | Notes |
|---|---|---|---|---|
| Terrain3D | v1.0.2 (19 May, 2026 inferred) | MIT | Active, about 4.4k stars | C++ GDExtension. Terrains from 64 m to 65.5 km, 10 LODs, 32 textures, holes, and a foliage instancer with shadow impostors. Imports heightmaps from World Machine and other tools. Supports Godot 4.4–4.6+; 4.7 was untested at the RC stage [23]. |
| HTerrain (Zylann) | master targets 4.6+ | (see repo) | **Maintenance mode**: bug fixes only [24] | Not for a new project. |
| Custom chunk mesher | n/a | ours | n/a | A procedural upsample of the 512² macro grid, built with ArrayMesh. Needed anyway for deterministic streaming (*Inference*). |

**Crowds, animation and navigation.**

- MultiMesh draws thousands of instances in one draw call, with some limits [14]:
  - culling happens for the whole MultiMesh, not per instance;
  - blend shapes are ignored;
  - the per-object light limit applies to the MultiMesh as a whole;
  - the `buffer` (PackedFloat32Array) gives a fast path for bulk updates.
- MultiMesh has no skinning. Godot's own answer is vertex animation in shaders ("thousands of fish") [18].
- Community data points:
  - The GodotCon Amsterdam 2026 talk abstract says about 100 rigged AnimationPlayers "can already lag a lot" and that 1,000 needed a recently merged PR plus VAT [20].
  - Forum reports call AnimationTree expensive, with about 200 units reachable only after disabling off-screen players [19].
  - GLES3 skinning was far slower than Vulkan (Godot 4.2) [21].
- **OpenVAT-for-Godot** (MIT, Godot 4.5+) provides a VAT MultiMesh with a per-instance animation track. It has **no blending**, requires fewer than 8,192 vertices and fewer than 4,096 total frames, and reserves `custom_data` and `instance_color` [22].
- Retargeting goes through BoneMap and SkeletonProfileHumanoid at import [12], and RetargetModifier3D handles it at runtime [15].
- A motion-matching plugin exists for the player avatar only: GuilhermeGSousa's, with about 250 stars [29].
- NavigationServer is irrelevant for sim-driven pedestrians, because the sim supplies the paths (*Inference*).

**Rendering performance.** Forward+ runs on Vulkan or D3D12, with Mobile and Compatibility (GL) as alternatives. Large-scene tools are automatic mesh LOD, visibility ranges (HLOD) and occlusion culling [13], plus texture streaming in 4.8 [5]. None of the release notes up to 4.8-dev7 mention GPU-driven culling or virtual geometry, so do not plan for Nanite-style density [3][4][7].

**Web export.** Web builds use WebGL2 through Compatibility only, with no WebGPU. Single-threaded export is the default, and C# projects cannot export to the web [10][11].

**Bindings.** GDScript for glue; C++ GDExtension (godot-cpp) is mature. Rust's **gdext** is at v0.5.5 (Aug 2026): self-described as "usable", with occasional breaking changes, an `api-4-7` level, experimental Wasm, and an MPL-2.0 licence that lets game code stay closed [25]. C# works on desktop. Python has no viable in-process option [26][27].

**Shipped 3D titles and limits.** Godot showcase entries include *Of Life and Land* (a 3D settlement and ecology sim, 2024), *PVKK*, Blender Studio's *DOGWALK* (2025), *Buckshot Roulette* and *Cruelty Squad* [28]. No AAA-scale seamless open world has shipped in Godot; *Road to Vostok* (R) is the most-watched large-map project.

### 2. Alternatives for a small team

| Option | Version and date | License and cost | Maturity | Fit for Rehoboam |
|---|---|---|---|---|
| **Godot** | 4.7.2 (Aug 2026); 4.8-dev [1] | MIT, $0 | Production for small and mid 3D | **Best fit.** Small runtime, wallpaper-friendly, GDExtension for hot paths. |
| **Bevy** | 0.20 (2026-10-08); 0.19 (2026-06-19) [30][31][34] | MIT or Apache, $0 | Fast-improving renderer: 1.6M-cube scene went from 21 to 53 FPS in 0.19, and a `bevy_city` example renders 55k entities [31]. **No official editor**: Jackdaw is a community prototype and the editor working group has not started [32]. A breaking release every 3–4 months. | Attractive if the sim moves to Rust and the ECS is shared. Costly for authoring content without an editor. |
| **Custom wgpu renderer** | n/a | $0, plus person-years | n/a | *Tiny Glade* used "Bevy ECS and Bevy App alongside a **custom renderer**" [33]. The draft's "wgpu" detail is unverified. It worked for an expert graphics programmer on a **small-scene** game; it is a poor trade for a 100k-entity sim. |
| **Fyrox** | 1.0.0 (29 Mar, 2026 inferred) [35] | MIT | Rust engine with an editor, small community | A long-shot Rust alternative. |
| **Unreal 5.6/5.7** (R) | 5.7 (Nov 2025); 5.8 status unverified | Free under $1M per product, then 5% royalty [73] (R) | **Mass Entity, MassCrowd, ZoneGraph; PCG** (production-ready in 5.7, R); MetaHumans, whose 2025 licence change allows other engines (R). City Sample: about 35k simulated pedestrians and about 45k parked cars in a Houdini-built city [75] (R). | The strongest crowd tech, but it needs C++ and a 30–100 GB toolchain, and it was designed for consoles running at full load. It is a poor wallpaper citizen and overkill for stylized art. |
| **Unity 6.x** (R) | Official samples use Unity 6.2 with **Entities 1.4** [38] | Runtime fee cancelled Sept 2024; Personal free under $200k [74] (R) | DOTS is mature for simulation, but there is no first-party ECS skinned animation (R). | A viable second choice with a large asset store. Licence-trust risk and a C#-only stack. |
| **Stride** | 4.4.0 (2026-10-09) [36] | MIT | Vulkan and D3D12 overhaul; the editor runs on Linux only via Proton or Wine | Niche. Not recommended. |
| **O3DE** | 2605.0 (2026-05-27) [37] | Apache-2.0 | Heavy, with a small community | Not recommended. |

### 3. Art direction feasibility

| Reference | Engine | Look | Why it scales | Lesson |
|---|---|---|---|---|
| *Tiny Glade* | Bevy ECS with a custom renderer [33] | Soft painterly diorama | Procedural architecture driven by user strokes | Charm comes from procedural *rules*, not asset count |
| *Townscaper* | Unity (R) | Toy town | WFC plus marching cubes on an irregular grid [44] | Tiny kit, infinite variety |
| *Bad North* | Unity (R) | Minimal islands | WFC level generation [44] | Flat palette hides low poly counts |
| *Dorfromantik* | Unity (R) | Hex diorama, tilt-shift | Tiles | Reads well at wallpaper distance |
| *Sable* | Unity (R) | Moebius-style line art | Flat shading | Shader-driven identity |
| *Hades* | Proprietary (R) | Hand-painted 2D | Large art staff | **Not reproducible by a solo team** |
| *Manor Lords* | UE4 (R) | Realistic medieval | About 7 years of solo work plus bought photoscans (R) | Realism costs years, and crowds stay in the hundreds |
| *Cities: Skylines II* | Unity (R) | Realistic | Launch performance hurt by citizen meshes without LODs (R) | LOD discipline for characters is non-negotiable |

**Why stylized is required (*Inference*):**
1. **Content volume.** Each of the 2–4 eras needs buildings, garments and props; one artist makes roughly 10× more palette-atlas low-poly pieces than PBR pieces.
2. **Heterogeneous sources.** CC0, paid and AI-generated meshes only cohere after one shared material, palette and outline pass.
3. **Performance.** Thousands of instances must fit a background budget.
4. **The Diorama read.** Tilt-shift depth of field (`CameraAttributesPractical` near/far blur) plus a high camera reads as a miniature and hides LOD transitions.
5. **The uncanny valley.** At crowd scale, avoid realistic faces altogether.

### 4. Procedural world content, from macro grid to street

**4.1 Terrain, biomes and rivers.** Upsample the 512² macro heights with seeded fBm noise and cheap per-chunk erosion. **Carve rivers from the macro river graph** rather than re-deriving them, or the macro and street layers will disagree. Keep biomes as the macro attribute, scatter vegetation with Poisson-disc sampling seeded per chunk, and feed the result to Terrain3D or our own clipmap.

**4.2 Azgaar's Fantasy Map Generator** (MIT, 2017–2024, about 6.1k stars, actively refactored to TypeScript as "FMG 2.0") [39]. The verified pipeline [40][41]:
1. A jittered square grid feeds a **Voronoi** graph.
2. Heightmap templates, then temperature and precipitation.
3. A re-graph into the "pack" structure.
4. **Rivers by flux accumulation**, with depression resolving (a river forms at flux ≥ 30).
5. Biomes, then goods.
6. Cultures, which expand from their centres.
7. Burgs, spaced with a quadtree.
8. **States, grown by a priority-queue flood fill** whose cost sums culture, population, biome, height, river and type terms, divided by each state's expansionism.
9. **Routes over an Urquhart graph** (a pruned Delaunay triangulation), then pathfinding with costs for habitability, height and connections.
10. Religions, provinces, **markets, production, taxes**, military, markers and zones.

Notably, FMG already derives each burg's seed for Watabou's city generator as `map seed + zero-padded burg id` [41]. That is direct precedent for the seed hierarchy proposed in §4.5.

**4.3 Settlements, cities and interiors.**

| Generator | License and status | Technique | Use for Rehoboam |
|---|---|---|---|
| Watabou MFCG, open source (TownGeneratorOS) | **GPL-3.0**, Haxe/OpenFL. Missing newer features such as water bodies; current versions are closed [42] | Voronoi wards, walls, streets from the gates, recursive lot subdivision | **Reimplement the ideas**: GPL code cannot go into a closed client. Terms for using its output are unverified. |
| Parish & Müller 2001 [80] (R) | Paper | Extended L-system roads under global goals and local constraints | Growth of road networks over centuries |
| CityEngine / CGA shape grammar [81] (R) | Commercial (Esri) | Split grammars for building massing and facades | Adopt the *grammar idea* as a small in-house DSL |
| ProbableTrain MapGenerator | **LGPL-3.0** [43] | Tensor-field street graphs ("American-style") | Modern grid and organic street layouts |
| Wave Function Collapse | **MIT** [44] | Constraint solver over tiles; used in *Bad North*, *Caves of Qud* and *Townscaper* | Facades, roofs, block infill, interiors placed onto a Godot GridMap with a MeshLibrary [3] |
| *Shadows of Doubt* (R) | Unity, commercial | Fully enterable procedural city, room-template interiors, citizen schedules | Proves lazy interiors are possible, but at a scale of thousands of citizens, not 10⁵ |
| *Cities: Skylines* (R) | Commercial | Zoning on road-aligned lots | Lot-from-road pattern for modern sprawl |

**4.4 Modern cities.** Combine tensor-field arterials [43], block subdivision, zoning derived from macro land value, kit skyscrapers (stacked floor modules from a grammar), AreaLight3D and emissive signage [3], and traffic as spline-following VAT or rigid instances. Interiors exist only behind doors the player opens, built from room templates with a stable seed per door.

**4.5 Chaining into one consistent street scene (*Inference*).**
- **Use counter-based hashing.** Derive every seed as `seed = H(world_seed, generator_id, generator_version, key...)` with xxHash64 or SplitMix, never with a stateful RNG stream.
  - Region: `H(ws, "region", cx, cy)`
  - Settlement: `H(ws, "burg", burg_id)`
  - Plot: `H(settlement_seed, plot_id)`
  - Building: `H(plot_seed, era)`
- **Generation is a function of (seed, sim snapshot).** The snapshot includes population, wealth, tech era and damage, so street layout tracks 1,000 years of change.
- **Growth must be incremental.** New plots infill along existing roads, and plots are never re-solved globally, so the view stays stable between two observations.
- **Sim events become persistent overrides.** Fires, demolitions and new keeps are stored as deltas over the generated baseline.
- **Generators are versioned.** A content update must not silently re-roll saved worlds.

### 5. Characters at scale

**5.1 Kits and animation sources.**

| Source | License | Notes |
|---|---|---|
| Quaternius Universal Animation Library (standard edition) | **CC0** [60]; Pro and Source editions are paid | Large set of humanoid clips; the free edition is a subset |
| Kenney 3D assets | **CC0** (code MIT) [61] | Low-poly; consistent with the stylized look |
| KayKit (R) | CC0 (R) | Low-poly adventurer and city kits |
| Synty (POLYGON, Sidekick) [83] (R) | Paid EULA, about $20–150 per pack (R) | Huge modular era coverage. **Check** redistribution and AI-training clauses. |
| Mixamo [78] (R) | Free, royalty-free in games; raw files may not be redistributed (R) | Service has been static for years, so its future is a risk |
| CMU MoCap [79] (R) | Free, including commercial; no resale of the raw data (R) | Raw BVH data needs cleanup |
| MDM / MoMask (text-to-motion) | MIT code [56] | Trained on HumanML3D, AMASS and SMPL, whose data and model licences are **non-commercial** [84] (R) |
| HY-Motion 1.0 (Dec 2025) | **Tencent licence; excludes EU, UK and South Korea** [55] | 1B-parameter flow-matching DiT; needs 24–26 GB of VRAM. Use offline, if at all. |

**5.2 Recommended rig and animation stack.**
- One SkeletonProfileHumanoid-compatible skeleton for everything [12].
- Verbs map to clips through a table; the blend tree is used only in tier 0.
- IK foot planting via the 4.6 IK framework [4].
- Text-to-motion is offline at most, to draft rare verbs such as TILL_SOIL, followed by manual cleanup.

**5.3 Crowd LOD tiers and Godot budget (*Inference*; validate in Spike 1).**

| Tier | Screen-space or distance trigger | Technique | Target count at 60 FPS on a mid-range GPU |
|---|---|---|---|
| T0 | Under about 25 m | Skinned mesh, AnimationTree, IK | 20–40 |
| T1 | 25–80 m | Skinned mesh, AnimationPlayer, staggered updates at 10–20 Hz | 50–150 |
| T2 | 80–400 m | **VAT on a MultiMesh**, with clip and phase in custom data [22] | 1k–5k |
| T3 | 400 m–2 km | Octahedral impostor sprites | 5k–30k |
| T4 | Beyond 2 km | Aggregate: density textures, lit windows, smoke, traffic streaks | Unbounded |

### 6. AI 3D asset generation and world models (2026)

| Model | Date | License | Quality for game assets |
|---|---|---|---|
| Hunyuan3D 2.0 / 2.1 / Part / Omni | Jan–Sep 2025 | **Tencent Community licence. Territory excludes the EU, UK and South Korea; using outputs outside the territory is "unlicensed"; >1M MAU needs a separate licence; outputs may not train other models** [45][46] | High-fidelity PBR meshes. 2.1 needs about 29 GB of VRAM for shape plus texture [45]. Hunyuan3D 3.x has no open repository in Tencent's org, so it is API-only (*Inference*). |
| TRELLIS.2 (4B) | Dec 2025 | MIT for code and weights, **but depends on nvdiffrast (research/evaluation-only licence)** [47][48] | Needs at least 24 GB of VRAM; takes 3–60 s on an H100; dense meshes with about 1M faces by default, so they need retopology [47] |
| Stable Fast 3D / SPAR3D | 2024–25 | Stability Community Licence (revenue cap, R) [49] | Fast and coarse |
| TripoSG / TripoSR | 2024–25 | MIT [50] | Good open baseline |
| Step1X-3D | 2025 | Apache-2.0 [51] | Open shape and texture model |
| MeshAnything V2 | 2024 | **S-Lab, non-commercial** [54] | Produces "artist-like" low-poly meshes, but cannot be used commercially |
| UniRig / Puppeteer (auto-rigging) | 2025 | MIT / Apache-2.0 [52][53] | Fine for single-mesh bodies, weak for layered garments (*Inference*) |
| Meshy, Tripo, Rodin (commercial) (R) | 2025–26 | Subscriptions, about $20–120 per month (R) | Low-poly modes and auto-rigging; best practical quality (R) |

**Verdict (*Inference*).** Use these tools offline for props (carts, stalls, statues, signage), then decimate and re-shade them with the palette atlas. They are not for modular architecture, which needs exact dimensions and sockets, and not for garment layers on the shared rig. Never generate at runtime: it breaks determinism and blows the VRAM budget. Keep a provenance log so licence-tainted assets can be purged.

**Generative world models.** Genie 3 runs at about 720p/24 fps with minutes of consistency and limited access [77] (R). **HY-World 1.5 (WorldPlay)**, released 2025-12-17, streams 480p at 24 fps, needs 28–72 GB of VRAM and carries the Tencent territory clause [57]. **Matrix-Game 4.0** (Apache-2.0, released 2026-10-15) lets an executable engine own gameplay and physics while a causal video model re-renders its low-poly RGB frames at about 38 FPS and 864×480 on an A100-80GB; an RTX 5090 path exists and 5080/5070 support is "expected next week" [58]. Tencent's **WorldClaw** (Aug 2026) is paper-only [59].

**Verdict:** unrealistic as the Diorama renderer in 2026 because of VRAM, resolution, no access to sim state, and identity drift. Watch the "engine-authoritative plus neural re-render" pattern as an optional premium mode after 2027.

### 7. Desktop wallpaper hosting

**Windows (*Inference* plus [63–65]).** The WorkerW technique:
1. Send message `0x052C` to `Progman` to spawn a WorkerW window behind the desktop icons.
2. Find that WorkerW.
3. Call `SetParent(ourHwnd, workerw)` and position the window at the target monitor's rectangle within the virtual-screen coordinates.

Godot exposes the HWND through `DisplayServer.window_get_native_handle(WINDOW_HANDLE)` and has `--wid <id>` ("request parented to window") [16][17]; whether `--wid` works directly with WorkerW needs testing. Windows 11 24H2 changed the desktop window hierarchy enough that Lively "fully rewrote" its core [65], which is precedent that this approach breaks with OS updates.

**Lively** (GPL-3.0, about 19.8k stars):
- Runs "Unity or Godot games as wallpaper" through `LivelyInfo.json` application packages, though not in the Microsoft Store edition.
- The app must not resize itself or go fullscreen.
- **"Wallpaper playback pause is currently disabled for application wallpapers"** [62][63].
- Its general pause rules cover fullscreen apps, battery, and a grid algorithm that pauses at 95% screen coverage [64].

**Wallpaper Engine** [76] (R): supports application wallpapers locally, but they reportedly **cannot be published to the Steam Workshop**; only web and scene types can. Verify before planning distribution there.

**macOS** (*Inference*): a borderless NSWindow at desktop window level, ignoring mouse events and joining all Spaces, set up through a small Objective-C GDExtension. Plash, the usual reference, is now **closed source** and web-only [66].

**Linux**: wlr-layer-shell's background layer works on wlroots compositors and KDE; X11 uses `_NET_WM_WINDOW_TYPE_DESKTOP`; **GNOME has no supported path**. linux-wallpaperengine (GPL-3.0) shows how fragile this is across compositors [67].

**Power and GPU budget.** Godot provides:
- `--max-fps` and Engine.max_fps [17];
- `RenderingServer.render_loop_enabled = false`, which stops rendering while logic keeps running [85];
- FSR 1/FSR 2 and MetalFX for 3D scaling;
- low-processor mode.

Godot 4 has **no battery API**: the OS class reference has none [86]. Detect battery state in the sim host process or a native helper. Proposed policy:
- 30 FPS cap at 0.5–0.67 render scale with FSR;
- 0 FPS when 95% of the desktop is covered or a fullscreen app is running;
- 5–10 FPS or a "photograph" mode on battery;
- the sim keeps running in its own process throughout.

**Market evidence for ambient and companion games.** *Tiny Pasture* (Godot, Feb 2025) lives at the bottom of the screen, and *KinitoPET* (Godot, 2024) is a desktop-companion horror game [28]. *Rusty's Retirement* (2024) and *Bongo Cat* (2025) were large Steam successes [82] (R). A hobby repo already runs Godot 4 + FastAPI as a WorkerW wallpaper [68]. All of these are 2D strips with tiny GPU budgets: **no one has shipped a heavy 3D simulation as a wallpaper.** That is both the opportunity and the risk.

### 8. Architecture checks

**WebSocket JSON-RPC vs in-process GDExtension (*Inference*).**
- Keeping two processes buys:
  - crash isolation;
  - the ability to pause or kill the renderer independently of the 1,000-year sim;
  - a Python sim with no binding project.
- The payload is the problem. Example: 2,000 visible entities at 5 Hz.
  - Packed binary (id, quantized position, yaw, verb, flags) is about 16 B per entity, or about 160 KB/s.
  - JSON is about 120 B per entity, or about 1.2 MB/s, plus GDScript parse time on the main thread.
- Fix:
  - binary frames (FlatBuffers or a fixed struct) decoded in GDExtension straight into `MultiMesh.buffer` [14];
  - **interest management**: the client subscribes to view tiles and LOD;
  - send **verbs plus waypoints** and let the client interpolate locally, rather than sending per-frame positions;
  - JSON-RPC only for control traffic.
- An in-process GDExtension only makes sense if the sim is rewritten in Rust or C++ [25]. LibGodot (4.6) would allow the inverse, a host process that embeds Godot [4]. No Python LibGodot binding exists (*Inference*, from a GitHub search).

**PixiJS** [70] belongs in a 2D Observer or map view. However, a **three.js/WebGPU** diorama [71] or Godot's web export [10] would be the only route to Wallpaper Engine Workshop web wallpapers and to Plash on macOS. That is a portability argument, but it costs performance and tooling.

**Hash-mod kitbash fix.** Use eligible-set rendezvous hashing:

`variant = argmax over v in Eligible(era, culture, sex, age_band, occupation, wealth, climate) of H(entity_id, slot, v)`

- Adding a variant then reassigns only about 1/(N+1) of entities, instead of nearly all of them under `mod N`.
- Families can share face and skin slots by hashing `lineage_id`.
- Garment and status slots re-roll only on sim events such as promotion or a change of season.
- Named characters store explicit overrides.
- CorruptionDecal becomes a shader mask parameter, packed alongside the VAT data within MultiMesh's custom-data limits [14][22].

**Hydration.** Replace the 100 m radius with the frustum tiers in §5.3. Entities in T2 and T3 must still be *real*, addressable sim entities so the camera can approach them without a pop. Plausible but unbound "filler" pedestrians at T3–T4 would contradict the "real errands" promise, so if they are used they must be labelled as such internally.

---

## Recommendations for Rehoboam

1. **Engine.** Use Godot 4.7.x now and 4.8 once stable (texture streaming), with Forward+ (D3D12 or Vulkan) on desktops and a Compatibility preset for weak iGPUs. Write hot paths in Rust gdext or C++: crowd system, chunk mesher, protocol decoder, WorkerW and macOS window hooks. Use GDScript for glue. Avoid C# unless the team is already .NET-native; it rules out web export [11]. **No** double-precision builds.
2. **Process and protocol.** Keep the Python sim as its own process. Use a binary, interest-managed snapshot protocol over a local WebSocket or named pipe, moving to a shared-memory ring only if profiling demands it. Keep JSON-RPC 2.0 for commands, queries and Oracle interactions.
3. **Art bible: "painted miniature".** Palette-atlas materials (one 256×256 atlas per era), vertex-colour AO, soft rim and outline, a LUT per world, fog, a stylized sky, tilt-shift depth of field in Diorama mode, and AreaLight3D plus emissives for the neon world. No realistic faces. **All bought, CC0 and AI-generated assets go through one re-shading step.**
4. **Content pipeline.**

| Content | How it is made | Source | Cost (estimate) |
|---|---|---|---|
| Macro world (states, cultures, routes, rivers) | Procedural; port Azgaar's algorithms in Python | MIT [39] | 1–2 PM |
| Terrain and rivers at street level | Procedural upsample plus river carving | Terrain3D or custom [23] | 2–3 PM |
| Road and plot layout | Procedural: ward/organic plans for medieval towns, tensor-field streets for modern cities | Own code (ideas from [42][43][80]) | 3–4 PM |
| Buildings | Procedural assembly of **hand-made or bought kits** using a grammar plus WFC on GridMap | Synty or CC0 kits plus the art pass | 4–6 PM plus $300–1,500 |
| Interiors | Lazy room templates, Oracle and Street modes only | Hand-made templates | 2–3 PM |
| Props (long tail) | **AI-generated offline** (TripoSG, Step1X-3D, or a commercial tool), then decimated and re-shaded | MIT/Apache tools; avoid Hunyuan3D and MeshAnything | $0–1,500/yr |
| Characters | Bought or CC0 base bodies and garments, one rig | Quaternius, Kenney, Synty | $200–800 plus 2–3 PM |
| Animation | CC0 library plus Mixamo, with hand fixes for the verbs | [60], Mixamo (R) | 1–2 PM |
| Dragons and creatures | Commissioned or hand-made, with VAT for flocks | Freelancer | $1–5k |

5. **Wallpaper strategy.** Windows first: our own WorkerW host mode inside the client, plus a Lively package. Wallpaper Engine is optional, and only once its rules are confirmed. On macOS, run natively at desktop level; on GNOME, offer a windowed "picture-frame" mode. Self-throttle per the §7 power policy, with the sim running headless and independent of the renderer.
6. **Three de-risking spikes, about 6 weeks in total.**
   - **Spike 1 (crowd):** 50 T0/T1 characters plus 3,000 T2 VAT agents plus T3 impostors at 1440p. Go if it holds 60 FPS on an RTX 3060-class card and 30 FPS on a Radeon 780M-class iGPU.
   - **Spike 2 (street from seed):** one Azgaar-style burg must produce the same 3D town on two machines, and growing its population from 2k to 5k must keep more than 90% of buildings unchanged.
   - **Spike 3 (wallpaper):** the Windows 11 24H2 host on a secondary monitor must stay under 10% GPU and under 5 W extra package power at the 30 FPS cap, and must drop to 0 FPS when covered.

---

## Risks and unknowns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Godot cannot hit the crowd targets | Medium | High | Spike 1; VAT and impostors; reduce T0 |
| The street generator disagrees with the macro sim (rivers, roads, wealth) | High | High | Carve rather than re-derive; sim snapshot as input; incremental growth |
| Licence contamination: Hunyuan3D territory, nvdiffrast, SMPL/AMASS, GPL Watabou code [45][48][42] | Medium | High | Allow-list of tools; provenance log; reimplement rather than copy |
| A Windows update breaks WorkerW (24H2 precedent [65]) | Medium | Medium | Fall back to a windowed mode; also ship via Lively |
| Battery or GPU drain causes uninstalls | Medium | High | Hard caps, pause on occlusion, "photograph" mode |
| Antivirus flags an EXE parented to the desktop | Low–Medium | Medium | Code signing; no injection |
| Godot 4.8 slips | Medium | Low | 4.7 is sufficient |
| Bevy or Unreal temptation causes churn | Low | High | Decide now; revisit only after Spike 1 fails |
| Unverified (R) items: Wallpaper Engine Workshop rules, Synty EULA, UE 5.8 status, Mixamo continuity | n/a | Medium | Verify before relying on any of them |

---

## Sources

Sources marked [R] were not fetched in this session.

1. https://github.com/godotengine/godot-website/blob/master/_data/versions.yml: official version and date table (4.7.2, 4.8-dev7)
2. https://github.com/godotengine/godot-website/blob/master/collections/_article/godot-4-7-lights-camera-action.md: 4.7 announcement (2026-06-18)
3. https://github.com/godotengine/godot-website/blob/master/_data/release_4_7/features.yml: 4.7 features (HDR, AreaLight3D, MeshLibrary editor)
4. https://github.com/godotengine/godot-website/blob/master/_data/release_4_6/features.yml: 4.6 features (Jolt, IK, SSR, D3D12, LibGodot)
5. https://github.com/godotengine/godot-website/blob/master/collections/_article/dev-snapshot-godot-4-8-dev-5.md: texture streaming
6. https://github.com/godotengine/godot/pull/113429: texture-streaming PR, merged about 2026-08-27
7. https://github.com/godotengine/godot-website/blob/master/collections/_article/dev-snapshot-godot-4-8-dev-4.md: 4.8 rendering additions (also dev6 and dev7)
8. https://github.com/godotengine/godot/releases: release tags
9. https://github.com/godotengine/godot-docs/blob/master/tutorials/physics/large_world_coordinates.rst: double-precision costs and the 8 km guidance
10. https://github.com/godotengine/godot-docs/blob/master/tutorials/export/exporting_for_web.rst: WebGL2 only, no WebGPU, threading
11. https://github.com/godotengine/godot-docs/blob/master/tutorials/scripting/c_sharp/c_sharp_basics.rst: C# has no web export
12. https://github.com/godotengine/godot-docs/blob/master/tutorials/assets_pipeline/retargeting_3d_skeletons.rst: BoneMap and SkeletonProfileHumanoid
13. https://github.com/godotengine/godot-docs/blob/master/tutorials/3d/visibility_ranges.rst: HLOD, mesh LOD, occlusion culling
14. https://github.com/godotengine/godot/blob/master/doc/classes/MultiMesh.xml: instancing limits and the buffer API
15. https://github.com/godotengine/godot/blob/master/doc/classes/RetargetModifier3D.xml: runtime retargeting
16. https://github.com/godotengine/godot/blob/master/doc/classes/DisplayServer.xml: native handle, transparency and passthrough flags
17. https://github.com/godotengine/godot-docs/blob/master/tutorials/editor/command_line_tutorial.rst: `--wid` and `--max-fps`
18. https://docs.godotengine.org/en/stable/tutorials/performance/vertex_animation/animating_thousands_of_fish.html: vertex-shader crowds (search snippet)
19. https://forum.godotengine.org/t/performance-of-crowds-of-animated-rigged-characters-in-3d/130409: community crowd limits (search snippet)
20. https://talks.godotengine.org/godotcon-ams-2026/talk/RXLHDR/: "1000 rigged AnimationPlayers" talk abstract (search snippet)
21. https://github.com/godotengine/godot/issues/88954: GLES3 skinning slowdown (search snippet)
22. https://github.com/antzGames/OpenVAT-for-Godot: VAT MultiMesh (MIT) and its limits
23. https://github.com/TokisanGames/Terrain3D: terrain GDExtension (MIT) and its releases
24. https://github.com/Zylann/godot_heightmap_plugin: maintenance-mode notice
25. https://github.com/godot-rust/gdext: Rust bindings (MPL-2.0), releases up to 0.5.5
26. https://github.com/touilleMan/godot-python: Godot 4 port incomplete
27. https://github.com/niklas2902/py4godot: early-stage Python bindings
28. https://github.com/godotengine/godot-website/tree/master/collections/_showcase: Tiny Pasture, KinitoPET, Of Life and Land, PVKK, DOGWALK
29. https://github.com/GuilhermeGSousa/godot-motion-matching: motion-matching plugin
30. https://github.com/bevyengine/bevy-website/blob/main/content/news/2026-10-08-bevy-0.20/index.md: Bevy 0.20
31. https://github.com/bevyengine/bevy-website/blob/main/content/news/2026-06-19-bevy-0.19/index.md: big-scene benchmarks
32. https://github.com/bevyengine/bevy-website/blob/main/content/news/2026-08-10-bevys-sixth-birthday/index.md: editor status and Jackdaw
33. https://github.com/bevyengine/bevy-website/blob/main/content/news/2025-08-10-bevys-fifth-birthday/index.md: Tiny Glade's custom renderer
34. https://github.com/bevyengine/bevy/releases: Bevy release dates
35. https://github.com/FyroxEngine/Fyrox/releases: Fyrox 1.0
36. https://github.com/stride3d/stride/releases: Stride 4.4.0
37. https://github.com/o3de/o3de/releases: O3DE 2605.0
38. https://github.com/Unity-Technologies/EntityComponentSystemSamples: Unity 6.2 with Entities 1.4
39. https://github.com/Azgaar/Fantasy-Map-Generator/blob/master/LICENSE: MIT licence (repo and CONTEXT.md)
40. https://github.com/Azgaar/Fantasy-Map-Generator/blob/master/src/generators/generation-pipeline.ts: pipeline order
41. https://github.com/Azgaar/Fantasy-Map-Generator/tree/master/src/generators: state, route, burg and river generators (Watabou seed links)
42. https://github.com/watabou/TownGeneratorOS: open-source MFCG (GPL-3.0)
43. https://github.com/ProbableTrain/MapGenerator: tensor-field city generator (LGPL-3.0)
44. https://github.com/mxgmn/WaveFunctionCollapse: WFC (MIT) and its users
45. https://github.com/Tencent-Hunyuan/Hunyuan3D-2.1/blob/main/LICENSE: territory and output restrictions (README gives VRAM figures)
46. https://github.com/Tencent-Hunyuan/Hunyuan3D-2: 2.0 licence with the same exclusions
47. https://github.com/microsoft/TRELLIS.2: 4B model, MIT, VRAM and timings
48. https://github.com/NVlabs/nvdiffrast/blob/main/LICENSE.txt: research and evaluation use only
49. https://github.com/Stability-AI/stable-fast-3d: Stability Community Licence
50. https://github.com/VAST-AI-Research/TripoSG: MIT
51. https://github.com/stepfun-ai/Step1X-3D: Apache-2.0
52. https://github.com/VAST-AI-Research/UniRig: auto-rigging, MIT
53. https://github.com/ByteDance-Seed/Puppeteer: auto-rigging, Apache-2.0
54. https://github.com/buaacyw/MeshAnythingV2: S-Lab non-commercial licence
55. https://github.com/Tencent-Hunyuan/HY-Motion-1.0: text-to-motion, Tencent licence
56. https://github.com/GuyTevet/motion-diffusion-model: MDM (MIT); also https://github.com/EricGuo5513/momask-codes
57. https://github.com/Tencent-Hunyuan/HY-WorldPlay: HY-World 1.5 specifications and licence
58. https://github.com/SkyworkAI/Matrix-Game/tree/main/Matrix-Game-4: engine plus neural re-render, 2026-10-15
59. https://github.com/Tencent-Hunyuan/Hunyuan3D-WorldClaw: agentic open-world generation (paper only)
60. https://github.com/J-Ponzo/gltf-universal-animation-library: Quaternius UAL standard edition, CC0 mirror
61. https://github.com/KenneyNL/Starter-Kit-3D-Platformer: Kenney assets are CC0
62. https://github.com/rocksdanister/lively: Lively, GPL-3.0, supports Godot and Unity executables
63. https://github.com/rocksdanister/lively/wiki/Application-Wallpaper: application wallpapers and the pause caveat
64. https://github.com/rocksdanister/lively/wiki/Performance: pause algorithms
65. https://github.com/rocksdanister/lively/releases/tag/v2.2.0.0: core rewrite for Windows 11 24H2
66. https://github.com/sindresorhus/Plash: now closed source
67. https://github.com/Almamu/linux-wallpaperengine: Linux constraints
68. https://github.com/b2cm92ts5k-alt/et-office-desktop: hobby Godot plus FastAPI WorkerW wallpaper
69. https://github.com/migeran/libgodot: LibGodot lineage
70. https://github.com/pixijs/pixijs/releases: PixiJS v8.22 (2D)
71. https://github.com/mrdoob/three.js/releases: three.js r186 (WebGPU)
72. https://godotengine.itch.io/godot/devlog/1034954/godot-45-making-dreams-accessible: 4.5 highlights (search snippet)
73. [R] https://www.unrealengine.com/en-US/license: UE royalty terms
74. [R] https://unity.com/blog/unity-is-canceling-the-runtime-fee: Unity licensing
75. [R] https://dev.epicgames.com/documentation/en-us/unreal-engine/city-sample-project-unreal-engine-demonstration: City Sample figures
76. [R] https://docs.wallpaperengine.io/en/: application-wallpaper and Workshop rules
77. [R] https://deepmind.google/discover/blog/genie-3-a-new-frontier-for-world-models/: Genie 3
78. [R] https://helpx.adobe.com/creative-cloud/faq/mixamo-faq.html: Mixamo terms
79. [R] http://mocap.cs.cmu.edu/: CMU MoCap terms
80. [R] https://dl.acm.org/doi/10.1145/383259.383292: Parish & Müller 2001
81. [R] https://dl.acm.org/doi/10.1145/1141911.1141931: Müller et al. 2006, CGA shape grammar
82. [R] https://store.steampowered.com/app/2666510/Rustys_Retirement/: desktop idle game
83. [R] https://syntystore.com/pages/end-user-licence-agreement: Synty EULA
84. [R] https://smpl.is.tue.mpg.de/modellicense.html and https://amass.is.tue.mpg.de/license.html: non-commercial body and motion data
85. https://github.com/godotengine/godot/blob/master/doc/classes/RenderingServer.xml: `render_loop_enabled`; FSR, FSR2 and MetalFX scaling modes are listed in Viewport.xml in the same folder
86. https://github.com/godotengine/godot/blob/master/doc/classes/OS.xml: low-processor mode; no power or battery query
