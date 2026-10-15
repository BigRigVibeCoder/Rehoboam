# 14 — Reuse Inventory (Bootstrap vs Build)

*Status: Draft v1.0 (2026-10-15). This answers the director's question: "How much can we bootstrap or cannibalize from GitHub projects that already solve a lot of the problems and tech in the stack?" It extends `07_technology_stack.md` and is governed by ADR-0021 (licence allow-list). **All effort numbers are estimates** made for planning and have roughly ±50% error. This is not legal advice.*

**How the facts were checked (2026-10-15)**

- **Licences.** For each GitHub or GitLab project I read the LICENSE/COPYING file at the default-branch HEAD (shallow, blobless `git clone`). Where a repo has no LICENSE file, that is stated.
- **Last activity.** This is the date of the last commit on the default branch at HEAD (`git log -1`). "rel" marks a registry release date (crates.io, PyPI or npm API).
- **Adoption.** Stars come from the GitHub search API. Crate downloads come from the crates.io API. PyPI and npm download counters were blocked from the sandbox.
- **Unreachable from the sandbox:** huggingface.co, kenney.nl, quaternius.com and gnu.org. Facts that depend on them come from secondary sources and are marked [U].

**Markers**

| Marker | Meaning |
|---|---|
| **[V]** | Verified this session against the primary source: the LICENSE file at HEAD, the git log, the registry API, the GitHub API, or the repo README. |
| **[V·reg]** | Verified only against the registry's declared licence field. The LICENSE file was not opened. |
| **[U]** | Unverified: a secondary source, prior knowledge, or a primary source that was unreachable. |

---

## TL;DR

1. **About 35–40% of total build effort can be bootstrapped** (estimate; range 30–45%). That is roughly 85–95 of about 225 person-weeks (pw). The coverage is lopsided:
   - **Periphery, 50–70% covered:** storage, LLM serving, audio, console visualisation and the Godot client.
   - **Story-generating core, ~10–15% covered:** the systems that decide the G2 thesis gate (M2–M8 couplings, minds/epistemics, the incremental sifter, significance, the Registry and compiler expander).
2. **The top 15 reuse items save an estimated 65–110 pw.** The current plan already assumes about half of that. **New finds and stronger reuse modes add roughly 30–55 pw** (the sum of the line estimates) beyond the plan, mostly after G2 (P3–P6). Expect the low end once integration and determinism hardening are paid for.
3. **Highest-value new finds** (all permissive, all active in 2026):
   - **OpenVic-Simulation** (MIT [V]): a headless fixed-point C++ library (LUT-based exp/sin) for a Victoria-2-style simulation, covering pops, markets and production.
   - **Starsim + Covasim + LASER** (MIT [V]): epidemic modules, testing with sensitivity and delays, a common-random-number (CRN) hashing design, and gravity/radiation migration kernels.
   - **tts-audiobook-tool** (MIT [V]): ASR-verified TTS retakes, EBU R128 loudness, M4B output.
   - **Console analytics:** Mosaic + duckdb-wasm, Perspective, family-chart/topola.
   - **Eval harnesses:** inspect_ai and promptfoo.
   - **Calibration:** SALib and pyABC.
   - **rh-minds planner:** Fluid HTN.
   - **Population synthesis:** humanleague.
   - **Godot add-ons:** Sky3D, ProtonScatter and godot-road-generator.
4. **Upgrade "ideas" to "port" where MIT allows it.** The plan reimplements Azgaar FMG from ideas only. FMG is MIT [V], so translating its code with attribution is allowed and saves 2–4 pw. The same applies to Starsim/Covasim/LASER, OpenVic-Simulation, Fluid HTN and mewo2's terrain and naming-language.
5. **Licence traps found:**
   - **EbookLib** (the obvious Python EPUB writer) is **AGPL-3.0** [V].
   - **epydemix** is **GPL-3.0** [V].
   - **Bespoke-MiniCheck-7B** weights are **non-commercial** [U]; its README says to contact the vendor for commercial use [V]. The stack lists it as "check".
   - **xxhash-rust** is **BSL-1.0** [V], which is not on the ADR-0021 allow-list. Use twox-hash (MIT [V]).
   - **No LICENSE file:** WriteHERE (only the README claims MIT [V]), StoryWriter, Felt, Winnow and world-history-simulator [V].
   - **Viv** is proprietary and non-commercial [V].
6. **Dead or archived:**
   - Archived: big-brain [V], Neighborly [V] and epirust [V].
   - Unmaintained by its own notice: synthpops [V].
   - Dormant for 18 months or more: voronoice, Rant, tracery-rs, Felt, Winnow, Talk of the Town code, Generative Agents and afterwriting [V].
7. **Grand-strategy codebases are mostly GPL** and so REFERENCE only: Freeciv, OpenTTD, openage and Veloren. The exceptions are **OpenVic (MIT)** and **Unciv (MPL-2.0, file-level)**. No open project does the integrated job (deterministic multi-century emergent history with a causal chronicle and sifting at 10⁵–10⁶ people). The closest are closed (Dwarf Fortress, CK3), GPL with other goals (Veloren's rtsim), or unlicensed and tiny (world-history-simulator, ★0).
8. **"Personal-use mode" would unlock an extra 10–20 pw**, mostly from Veloren (GPL Rust world and town generation), watabou's town generator, Viv and non-commercial models. The cheap-to-remove part is only about 3–6 pw: out-of-process oracles, swappable model weights and EbookLib. Anything ported into `rh-core`, `rh-sys-*`, `rh-sifter` or `rh-worldgen` costs more to remove than it saved, because swapping authoritative code changes history hashes and invalidates golden runs and saved worlds.
9. **The stack versions in `07` check out** against the registries: bevy_ecs 0.20.0, rand_chacha 0.10.0, libm 0.2.16, pyo3 0.30.1, gdext 0.5.5, deck.gl 9.4.0, sigma 4.0.3, vLLM 0.31.0, SGLang 0.5.21, llguidance 1.9.1, XGrammar 0.2.8, marimo 0.25.1 and DuckDB 1.5.6, all [V].
10. **Recommended actions:**
    - Adopt the PORT items in §Top-15.
    - Swap xxhash-rust for twox-hash, or add BSL-1.0 to the allow-list.
    - Use epub-builder (MPL-2.0) or the pandoc CLI instead of EbookLib.
    - Drop the Bespoke-MiniCheck-7B assumption, or tag it personal-only.
    - Add a "tool-only / out-of-process" clause and a personal-only `lab/` tier to ADR-0021 (§Personal-use mode).

---

## How to read this

### Reuse modes

| Mode | Meaning | Typical licence |
|---|---|---|
| **DEPEND** | Use as an unmodified library, binary, sidecar or asset. Pin exact versions. | Allow-list (MIT, Apache-2.0, BSD, ISC, MPL-2.0, Zlib, CC0, CC-BY) |
| **PORT** | Copy or translate the code (for example JS/C++/C#/Python → Rust) with attribution kept in `third_party/ATTRIBUTION` and in file headers. | Permissive. MPL-2.0 is allowed file-by-file, but the ported files stay MPL. |
| **REFERENCE** | Read for ideas, then reimplement cleanly from the paper or description, not from the source. Use this when the licence blocks copying (GPL, AGPL, LGPL-into-core, NC, proprietary, no licence) **or** when the language or design is the wrong fit. | Any |
| **AVOID** | Dead, unsafe, licence-toxic for this use, or a poor fit. | — |

### Licensing nuance: personal versus distributable

Rehoboam is currently **personal and not distributed**. That matters because most copyleft obligations trigger on *distribution*, not on use.

| Licence family | Personal, local, not distributed | Distributed binary or source | Network service to others | ADR-0021 today |
|---|---|---|---|---|
| MIT / BSD / ISC / Apache-2.0 / Zlib / CC0 | Free | Free, with notices (Apache NOTICE files too) | Free | **Allowed** |
| BSL-1.0 (Boost), Clear BSD, Artistic | Free | Permissive (Artistic 1.0 is vague) | Free | **Not listed.** Add BSL-1.0 and Clear-BSD, or avoid them. |
| MPL-2.0, EUPL-1.2 | Free | Modified *files* stay under the same licence; the rest of the work is unaffected. NobodyWho's README states that linking under EUPL does not extend it to your code [V]. | Same | MPL allowed; EUPL "conditional" |
| LGPL-3.0 | Free | Fine as a dynamically linked library. **Ported code becomes LGPL.** | Same | Not listed. Treat like GPL for porting. |
| GPL-2.0/3.0 | **Free.** Private modification and use create no obligation to release source [U: secondary, see Sources 175–176]. | Must ship source under the GPL for the *combined work* | No trigger (GPL) | **Excluded from shipped artifacts** |
| AGPL-3.0 | Free | As GPL | §13 triggers when you **modify** it and others interact over a network. A localhost console used only by you has no third-party users [U]. | Excluded |
| Non-commercial (Viv, CC BY-NC weights, Moonshine Community Licence) | **Allowed** for personal, educational and research use. Read each definition: Viv treats government-funded research as commercial [V]. | Blocked unless you buy a licence | Blocked | Excluded |
| **No licence** (Felt, Winnow, StoryWriter, world-history-simulator) | Reading is fine. **Copying is not granted, even for personal use.** | Blocked | Blocked | Reference-only |
| Territorial (e.g. Hunyuan3D, per `07`) | May be **unusable outright** depending on where you live | Blocked | Blocked | Excluded |

**Two rules that matter more than the table.**

1. **Out-of-process tools are not linking.** Running a GPL or AGPL program *as a separate tool* never puts our code under the GPL, even in a distributable product, as long as we don't ship it bundled. Examples: as a CI validation oracle, as the Blender add-on that bakes VAT textures, or as the Lively wallpaper host. That is the "mere aggregation" reading [U]. So many items marked REFERENCE can still be **run** today without any personal-use exception. What is blocked is **copying their code into our crates**.
2. **In Rehoboam, removal cost is dominated by determinism.** Code that runs inside the authoritative simulation (`rh-core`, `rh-sys-*`, `rh-minds`, `rh-sifter`, the shared `rh-worldgen`) defines history. Replacing it changes state hashes. That invalidates golden runs, saved worlds, branches, chronicles, keyframe caches and calibrations. A clean-room rewrite of contaminated core code therefore costs the rewrite **plus** re-baselining (about 1–3 pw per system). Out-of-process tools, model weights and exporters are cheap to swap.

ADR-0021 keeps strict hygiene so that publishing stays possible (`12` Q13). This document keeps that default and prices the alternative in §Personal-use mode.

---

## Inventory by subsystem

### Coverage at a glance (estimates)

The baseline is the effort to deliver the plan's scope for that subsystem when every reuse item is replaced by the *simplest viable home-grown substitute* (not by re-creating the external project). Coverage is the share removed by reuse, **net of integration and determinism hardening**.

| Subsystem (plan component) | Phase | Baseline (pw, est.) | Reuse covers (est.) | What reuse covers | What we must build |
|---|---|---|---|---|---|
| `rh-core` + determinism harness (`rh-ids`, `rh-snapshot`) | P0 | 14 | **~30%** | RNG streams, samplers (vendored), libm, fixed-point, xxh3, Arrow IPC, rayon, test and bench tooling, the Starsim CRN pattern | Scheduler, phase order, AccessSpec checker, agenda heap, Merkle hash ladder, unsynced guard, golden-run CI |
| M1 regions / `rh-worldgen` (also feeds Godot) | P1, P5 | 16 | **~45%** | FMG/mapgen4/O'Leary ports, Delaunay/Voronoi, petgraph, pathfinding, noise, naming languages, WFC | Region graph semantics, route modes, settlement growth, *growth-stable* street-from-seed (SPK-2) |
| M2 demography + population synthesis | P1 | 8 | **~30%** | IPF and integerisation (humanleague, ipfn), BTPE binomial, vital-event module patterns (Starsim, LASER) | Brass-logit hazards, density/entitlement modifiers, households, partnering, genealogical IDs and the life-course log (ADR-0026) at 1M |
| M3 agriculture and climate | P1 | 4 | **~15%** | Noise fields, WorldEngine climate ideas | Seasons, AR(1) anomalies, volcanic shocks, seed reserve, famine couplings |
| M4 economy | P1 | 8 | **~25%** | OpenVic-Simulation markets/pops/producers (port), bazaarBot price beliefs | Pre-modern goods, entitlement, rents and taxes, couplings to M6 and M2, calibration |
| M5 contagion + hive | P3 | 9 | **~45%** | Starsim/Covasim (SEIR, testing sensitivity/delay/loss, quarantine, Dx), LASER migration kernels, ndlib fear diffusion | Metapopulation on the region graph, covert compartment, hive coupling, fixed-point determinism |
| M6 polity (SDT) | P1 | 6 | **~5%** | Nothing usable; papers only | Everything |
| M7 succession and dynasts | P1 | 5 | **~5%** | Nothing usable | Everything |
| M8 campaign war | P3 | 6 | **~10%** | Pathfinding; OpenVic military ideas | Supply, attrition, Helmbold battles, sieges, camp disease |
| S* culture, religion, tech, ecology | P3 | 6 | **~15%** | ndlib opinion-dynamics kernels, FMG culture/religion seeding | Coupled diffusion on the region graph; dragons as T1 with energetics |
| `rh-minds` (IAUS, HTN, beliefs, ToM, persuasion) | P1, P3 | 16 | **~20%** | Fluid HTN port, bonsai-bt, big-brain curve design, bevy_ecs, Talk of the Town evidence types (paper) | Psych core, overlays, belief store, channels/hive, ToM, Whisper parse→ground→score |
| `rh-sifter` + significance | P2 | 10 | **~15%** | DuckDB SQL twins (spec already prototyped), ascent/dbsp for escalation, Felt/Winnow/Viv as designs | DSL compiler, partial-match automata at 10⁹ events, anchoring lint, suspense, retroactive significance |
| Chronicle, storage, keyframes | P1, P2 | 14 | **~70%** | arrow-rs, parquet, DuckDB, rusqlite, zstd, polars | Event schema, causal edges, tiering and compaction policy |
| `rh-server` / `rh-protocol` | P2 | 10 | **~45%** | tokio, axum, tungstenite, jsonrpsee, ts-rs/specta, schemars, Arrow frames | Timeline manager, fork and seek, interest management, material-input scoping and timer re-sampling (ADR-0025) |
| Foresight | P3 | 5 | **~15%** | CRN pattern (Starsim), rayon | Prophecy DSL, rollout orchestration, belief-resampled rollouts, validity tests (PIT, Brier/log score, Spiegelhalter Z) |
| World compiler + calibration harness (M9) | P1, P4 | 12 | **~35%** | pydantic, instructor, llguidance/XGrammar, SALib, pyABC, marimo, Rerun | Intent schema, mapping table, expander, semantic validators, liveness gate, deterministic calibration, background certification, gap reports |
| LLM gateway + evals | P3 | 10 | **~65%** | llama.cpp, vLLM/SGLang, llguidance/XGrammar, LiteLLM (optional), inspect_ai/promptfoo | Typed job API, journaling, budgets, routing policy, red-team corpus content |
| Narrative compiler + Annals | P2, P4 | 10 | **~35%** | Templating (bladeink, Tracery/improv patterns), WriteHERE/DOC planning patterns, MiniCheck code, Fountain/EPUB/RSS exporters | Digest, canon gate against the causal chronicle, style cards, NOVEL-20 |
| Console (Time-Machine, inspector, Oracle-2D) | P2 | 14 | **~55%** | deck.gl, sigma/graphology, dagre, cosmos.gl, Mosaic + duckdb-wasm, Perspective, family-chart/topola, Tauri | Time-Machine UX, speed bands, auto-pause, causal inspector semantics, rating tool |
| Godot client (Diorama, Street, crowds, content) — **art phase 2, optional (ADR-0029)** | P5–P7 | 36 | **~50%** | gdext, Terrain3D, Sky3D, Scatter, road-gen, OpenVAT, impostors, LimboAI, Dialogue Manager, motion matching, CC0 kits + animation library | Frame decoder, crowd LoD policy, rendezvous kitbash, camera director, hydration bubble, wallpaper host |
| Audio | P4, P6 | 8 | **~65%** | Chatterbox/Pocket/Qwen3-TTS/MOSS-TTSD, Moonshine/whisper, AudioSeal, pyloudnorm, tts-audiobook-tool patterns | Voice bible, systemic audio telemetry, latency masking, RSS packaging |
| **Total** | | **~227** | **~38% (≈ 87 pw)** | | **≈ 140 pw** to build |

**Thesis-critical slice (P0–P2 only).** This is the subset that decides G2: core, worldgen, M2–M4, M6, M7, sifter, chronicle, server, console and calibration. Reuse covers about **35–37%**. But the simulation and sifting *logic* inside it (M2–M8, minds, sifter, about 70 pw) is only **~10–15%** covered.

### A. `rh-core`: determinism foundations

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| rand_chacha (rust-random/rand) | ChaCha streams with `set_stream`/`set_word_pos`, keyed and seekable | MIT OR Apache-2.0 [V] | rel 0.10.0, 2026-02-02; repo 2026-09-20 [V] · 1.5B dl | DEPEND | Pin exactly. `StdRng` is not portable across versions (R03). | [crates.io](https://crates.io/crates/rand_chacha) |
| rand_distr | BTPE binomial, ZIGNOR normal, gamma/beta, alias tables; libm option "to ensure reproducibility" [V README] | MIT OR Apache-2.0 [V] | rel 0.6.0, 2026-02-10; repo 2026-07-26 [V] · 168M dl | **PORT** (vendor + golden vectors, P0.3) | Poisson is Knuth + rejection, **not PTRS** [V src]. PTRS still has to be written. | [gh](https://github.com/rust-random/rand_distr) |
| Random123 (D. E. Shaw) | Reference Philox/Threefry counter-based RNG and known-answer tests | BSD-3-Clause [V] | 2022-01-17 [V] · ★129 | PORT (Philox4x32 → Rust) | Algorithm is stable; a dormant repo is fine | [gh](https://github.com/DEShawResearch/random123) |
| **Starsim CRN** (`distributions.py`) | Per-agent *slot*-keyed splitmix64 hashing: "the same slot always yields the same value regardless of how many other agents are drawn" [V src]. This is the `H(seed,id)` and Foresight CRN design, already proven. | MIT [V] | 2026-10-06 [V] · ★44 | **PORT** (pattern → `rh-ids`) | Trivial port; adds validated rationale | [gh](https://github.com/starsimhub/starsim) |
| libm | Pure-Rust, bit-stable transcendentals | MIT [V·reg] | rel 0.2.16, 2026-01-24 [V] · 520M dl | DEPEND | Repo moved to rust-lang/compiler-builtins [V·reg] | [crates.io](https://crates.io/crates/libm) |
| fixed | Fixed-point types for threshold quantities | MIT OR Apache-2.0 [V] | rel 1.31.0, 2026-03-20; repo 2026-09-17 [V] · 15.5M dl | DEPEND | No transcendentals; see the OpenVic LUTs in §D | [gitlab](https://gitlab.com/tspiteri/fixed) |
| twox-hash | XXH64, XXH3-64 and XXH3-128 for the Merkle state hash | MIT [V] | rel 2.1.5, 2026-10-04 [V] · 267M dl | **DEPEND (preferred)** | Allow-list compliant | [gh](https://github.com/shepmaster/twox-hash) |
| xxhash-rust | The same, with const fns | **BSL-1.0** [V] | rel 0.8.19, 2026-09-28 [V] · 118M dl | AVOID (unless BSL-1.0 is allow-listed) | BSL-1.0 is permissive but missing from ADR-0021 | [gh](https://github.com/DoumanAsh/xxhash-rust) |
| indexmap, rustc-hash | Insertion-ordered maps (deterministic iteration); fast fixed hasher | Apache-2.0 OR MIT [V·reg] | rel 2.14.2 (2026-09-05); 2.1.3 (2026-07-02) | DEPEND | Enforces the "no HashMap iteration" rule | [crates.io](https://crates.io/crates/indexmap) |
| rayon | Data-parallel kernels | MIT OR Apache-2.0 [V·reg] | rel 1.12.0, 2026-04-14 · 600M dl | DEPEND | Float `sum`/`reduce` are not deterministic (R03). Use integer or fixed reductions. | [crates.io](https://crates.io/crates/rayon) |
| bevy_ecs (standalone) | ECS for T1 named actors | MIT OR Apache-2.0 [V·reg] | rel 0.20.0, 2026-10-08 [V] · 9.5M dl; Bevy ★49k | DEPEND (Trial, pinned) | 0.20 adds schedule randomisation (R03). Pin the order. | [crates.io](https://crates.io/crates/bevy_ecs) |
| hecs | Minimal ECS fallback | MIT OR Apache-2.0 [V] | rel 0.11.2, 2026-10-04 [V] · ★1.4k | DEPEND (fallback) | — | [gh](https://github.com/Ralith/hecs) |
| rkyv / postcard / bitcode / zstd | In-process ring, messages, compression | MIT; MIT/Apache; MIT/Apache; BSD-3 [V·reg] | rel 0.8.18 (2026-08); 1.1.3 (2025-07); 0.6.9 (2025-12); 0.14.2 (2026-10) | DEPEND | bincode stays on Hold (RUSTSEC-2025-0141, per `07`) | [crates.io](https://crates.io/crates/postcard) |
| proptest / insta / criterion / cargo-deny | Property tests, golden snapshots, benchmarks, licence gate | MIT/Apache; Apache-2.0; Apache/MIT; MIT/Apache [V·reg] | rel 1.12.0 (2026-10-10); 1.49.0 (2026-10-03); 0.8.2 (2026-02); 0.20.2 (2026-07) | DEPEND | cargo-deny enforces ADR-0021 in CI | [crates.io](https://crates.io/crates/cargo-deny) |
| krABMaga | Rust ABM engine with Bevy visualisation | MIT [V] | 2026-10-07 [V]; rel 0.6.2 (2026-06-19) · ★224, 21k dl | REFERENCE | Own scheduler and float models; not compatible with the determinism contract | [gh](https://github.com/krABMaga/krABMaga) |
| Syren | Rust ECS ABM with a "deterministic stage scheduler" over rayon [U: docs.rs via search] | MIT [V] | rel 0.6.0, 2026-08-15 [V] · **17 dl** | REFERENCE (Assess) | New, single author | [gh](https://github.com/AshvinPerera/Syren-ABM-Framework) |
| Mesa | Python ABM framework | Apache-2.0 [V] | 2026-10-09 [V]; PyPI 3.5.1 (2026-03-15) · ★3.9k (now `mesa/mesa`) | REFERENCE | Wrong language and scale | [gh](https://github.com/mesa/mesa) |

### B. M1 regions and world generation (`rh-worldgen`, shared with Godot)

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| **Azgaar Fantasy Map Generator** | Voronoi cells, heightmap templates, temperature and precipitation, rivers, biomes, culture and religion expansion, burgs, states, provinces, routes, name bases | MIT [V] | 2026-10-14 [V] · ★6.1k | **PORT** (upgrade from "ideas") | JS with global state. Port module by module against golden outputs; keep attribution. | [gh](https://github.com/Azgaar/Fantasy-Map-Generator) |
| mapgen4 (Red Blob Games) | Painted elevation, wind and rainfall, river flow, dual-mesh structure | Apache-2.0 [V] | 2026-03-29 [V] · ★933 | PORT | Keep NOTICE and attribution | [gh](https://github.com/redblobgames/mapgen4) |
| mewo2/terrain (Martin O'Leary) | Erosion, city-placement scoring, territories, label placement | MIT [V] | 2016-08-15 (dormant, stable) [V] · ★3.0k | PORT | Small, well-understood algorithms | [gh](https://github.com/mewo2/terrain) |
| mewo2/naming-language | Phonotactic "naming languages" per culture | MIT [V] | 2016-07-29 [V] · ★321 | PORT | Consistent names per culture for free | [gh](https://github.com/mewo2/naming-language) |
| WorldEngine | Plate tectonics, rain shadow, erosion, biomes | MIT [V]; depends on **PyPlatec, LGPL-3.0** [V] | 2026-08-27 [V]; PyPI 0.20.0 (2025-11-18) · ★1.1k | REFERENCE / PORT (MIT parts only) | Keep platec out of the core | [gh](https://github.com/Mindwerks/worldengine) |
| terrain-erosion-3-ways | Hydraulic erosion and river networks (NumPy) | MIT [V] | 2021-06-15 [V] · ★964 | PORT | Direct NumPy → Rust | [gh](https://github.com/dandrino/terrain-erosion-3-ways) |
| SimpleHydrology (weigert) | Particle hydrology: rivers, lakes, vegetation | README says MIT but there is **no LICENSE file** and it links an external notice [V] → [U] | 2023-03-20 [V] · ★751 | REFERENCE | Ask the author before copying | [gh](https://github.com/weigert/SimpleHydrology) |
| Undiscovered Worlds | Global climate, rivers, lakes, regional zoom | **GPL-3.0** [V] | 2024-09-09 [V] · ★106 (Classic ★30) | REFERENCE | README: "The same seed does not always produce the same planet" [V] | [gh](https://github.com/JonathanCRH/Undiscovered_Worlds) |
| mapgen2 | Classic polygon island generator | MIT [V] | 2025-05-07 [V] · ★1.4k | REFERENCE | ActionScript | [gh](https://github.com/amitp/mapgen2) |
| spade | Delaunay, constrained Delaunay, Voronoi | MIT OR Apache-2.0 [V] | rel 2.15.1, 2026-03-24 [V] · 21M dl | DEPEND | Integer or fixed inputs for determinism | [gh](https://github.com/Stoeoef/spade) |
| delaunator-rs | Fast Delaunay | ISC [V] | rel 1.1.0, 2026-05-12 [V] · 621k dl | DEPEND | — | [gh](https://github.com/mourner/delaunator-rs) |
| voronoice | Voronoi built on delaunator | MIT [V] | last commit 2022-12-09 [V] · 31k dl | AVOID (dormant) | Use spade, or vendor about 300 LoC | [gh](https://github.com/andreesteve/voronoice) |
| petgraph | Region graph, Dijkstra, MST, components | MIT OR Apache-2.0 [V] | 2026-10-12 [V]; rel 0.8.3 · 556M dl | DEPEND | Use StableGraph with sorted iteration | [gh](https://github.com/petgraph/petgraph) |
| pathfinding | A*, Dijkstra, flows, Kuhn–Munkres | Apache-2.0/MIT [V Cargo.toml] | 2026-10-15 [V]; rel 4.16.0 · ★1.1k | DEPEND | — | [gh](https://github.com/evenfurther/pathfinding) |
| rstar / kiddo | R*-tree / k-d tree | MIT OR Apache-2.0 [V] | rel 0.13.0 (2026-05-24) / 6.3.0 (2026-09-06) [V] | DEPEND | Bubble grid hash remains custom (R01) | [gh](https://github.com/georust/rstar) |
| FastNoiseLite / noise-rs | Simplex, Perlin and cellular noise | MIT [V] / MIT OR Apache-2.0 [V] | FNL repo 2026-06-21 [V] · ★3.5k; noise-rs 2025-02-13 [V] | DEPEND (FNL preferred) | FNL has the same algorithm in GDShader and C++, so terrain stays identical in Godot | [gh](https://github.com/Auburn/FastNoiseLite) |
| geo / i_overlay / hexx | Polygon booleans for wards and territories; hex overlays | MIT OR Apache-2.0; same; Apache-2.0 [V/V·reg] | rel 0.33.1; 9.1.0 (2026-10-14); 0.25.0 | DEPEND | — | [crates.io](https://crates.io/crates/i_overlay) |

### C. M2 demography and population synthesis

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| **humanleague** | IPF plus quasirandom *integer* microsynthesis (C++ core; Python and R) | MIT [V] | 2026-10-11 [V]; PyPI 2.4.5 (2026-08-29) · ★19 | DEPEND (offline) or PORT the core | One maintainer | [gh](https://github.com/virgesmith/humanleague) |
| PopulationSim | IPF, list balancing, integerisation across geographies | BSD-3-Clause [V] | 2026-10-02 [V]; PyPI 0.10.0 (2025-11) · ★70 | REFERENCE / PORT | Heavy pipeline, overkill for P1.2 | [gh](https://github.com/ActivitySim/populationsim) |
| ipfn | n-dimensional IPF (~200 LoC) | MIT [V] | 2021-12-30 (dormant) [V] | PORT | Trivial | [gh](https://github.com/Dirguis/ipfn) |
| synthpops | Households, age mixing, school and work contact layers | MIT [V] | "no longer being actively maintained" [V] | REFERENCE / PORT small pieces | — | [gh](https://github.com/synthpops/synthpops) |
| Starsim demographics | Birth, death and pregnancy modules with age-specific rates | MIT [V] | 2026-10-06 [V] | PORT (patterns) | Brass-logit and Coale–Demeny tables remain ours | [gh](https://github.com/starsimhub/starsim) |
| LASER demographics | Kaplan–Meier estimator, age pyramids, spatial population init | MIT [V] | laser-core 2026-07-15 [V] | PORT | — | [gh](https://github.com/laser-base/laser-core) |
| ActivitySim | Day-pattern → tours → trips hierarchy (for P6.2 schedules) | BSD-3-Clause [V] | 2026-10-08 [V]; PyPI 1.6.0 · ★252 | REFERENCE | Already cited in R01 | [gh](https://github.com/ActivitySim/activitysim) |

### D. M3 agriculture/climate and M4 economy

No open project models pre-modern harvest, seed-reserve and famine dynamics in a form worth porting (GitHub and web search, Oct 2026). M3 is essentially a build.

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| **OpenVic-Simulation** | Headless C++ library with unit tests. Covers pops; per-good markets with buy and sell orders; artisanal, factory and RGO producers; and a **fixed-point type with LUT-based exp and sin**. Also weighted sampling, military, diplomacy and politics [V tree]. | MIT [V] (COPYRIGHT: logo CC-BY-SA-4.0) | 2026-10-15 [V] · ★14 (OpenVic ★473) | **PORT** (economy + fixed-point LUTs) | Victoria-2 rules (19th century) need re-theming. Game data is not in the repo; it loads the user's Paradox files [U]. | [gh](https://github.com/OpenVicProject/OpenVic-Simulation) |
| bazaarBot | Doran & Parberry "Emergent Economies": agents with price-belief ranges, market clearing | MIT [V] | 2017-07-10 (dormant) [V] · ★393 | PORT (~500 LoC Haxe) | For T1 merchants and market price discovery | [gh](https://github.com/larsiusprime/bazaarBot) |
| Unciv | Ruleset-driven ("uniques") moddable mechanics in JSON | MPL-2.0 [V] | 2026-10-15 [V] · ★11.4k | REFERENCE (Registry design) | PORT is legal file-by-file, but the code is Kotlin and game-specific | [gh](https://github.com/yairm210/Unciv) |
| OpenTTD (cargodist) | Demand and flow assignment on transport networks | GPL-2.0 [V] | 2026-10-15 [V] · ★8.4k | REFERENCE | — | [gh](https://github.com/OpenTTD/OpenTTD) |
| Simutrans | Passenger and freight economy | Artistic licence (1.0-style text) [V] | Mirror 2026-08-01 [V] · ★451 | REFERENCE | Not on the allow-list | [gh](https://github.com/aburch/simutrans) |

### E. M5 contagion (metapopulation SEIRS, covert infection, testing)

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| **Starsim** | Modular disease, network, demographics and intervention framework; `Dx` diagnostic products; calibration; CRN | MIT [V] | 2026-10-06 [V]; PyPI 3.7.2 · ★44 (IDM / Gates Foundation) | **PORT** | Python/NumPy → Rust. Move thresholds to fixed-point. | [gh](https://github.com/starsimhub/starsim) |
| **Covasim** | `test_prob`/`test_num` with **sensitivity, loss-to-follow-up, test and swab delays**; quarantine policies; contact tracing [V src] | MIT [V] | 2026-10-06 [V]; PyPI 4.0.0 (2026-10-06), re-based on Starsim [V] · ★291 | **PORT** | Strip COVID-specific parameters. Moved to `starsimhub/covasim`. | [gh](https://github.com/starsimhub/covasim) |
| **LASER** (laser-core, laser-generic) | Numba spatial ABM for large populations; **gravity, competing-destinations, Stouffer and radiation** migration functions [V src]; SEIR model family | MIT [V] | 2026-07-15 / 2026-07-10 [V]; PyPI laser-core 1.0.5 · ★4 | **PORT** (migration kernels → M5 and M1 flows) | Low stars, but institutionally backed | [gh](https://github.com/laser-base/laser-core) |
| ndlib | SIR/SEIR on networks plus opinion dynamics (fear coupling) | BSD-2-Clause [V] | 2026-07-31 [V]; PyPI 6.0.1 · ★303 | REFERENCE / PORT kernels | — | [gh](https://github.com/GiulioRossetti/ndlib) |
| epydemix | Age-structured models, contact matrices, ABC calibration | **GPL-3.0** [V] | 2026-10-12 [V]; PyPI 1.4.0 · ★71 | REFERENCE (can run out-of-process as an oracle) | Not in the plan; licence trap | [gh](https://github.com/epistorm/epydemix) |
| OpenABM-Covid19 | Network ABM with testing, tracing and app modules | **GPL-3.0** [V] | 2025-10-09 [V] · ★130 | REFERENCE | — | [gh](https://github.com/BDI-pathogens/OpenABM-Covid19) |
| CovidSim / JUNE | Spatial individual-based models | **GPL-3.0** [V] | 2023-02-21 / 2026-02-13 [V] · ★1.2k / ★56 | REFERENCE | — | [gh](https://github.com/mrc-ide/covid-sim) |
| epirust | Rust epidemic ABM | **AGPL-3.0**, **archived** [V] | 2024-02-01 [V] · ★107 | AVOID | — | [gh](https://github.com/thoughtworks/epirust) |
| scikit-mobility | Gravity and radiation models | BSD-3-Clause [V] | 2023-01-20 (dormant) [V] · ★808 | REFERENCE | LASER is fresher | [gh](https://github.com/scikit-mobility/scikit-mobility) |

### F. M6 polity (SDT), M7 succession, M8 war, S* culture and technology

This is the thinnest area. No usable open code exists for structural-demographic (SDT) state dynamics, dynastic succession hazards or campaign logistics (search, Oct 2026). Build from the R09 literature.

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| Cliodynamics notebooks (e.g. dmely/cliodynamics) | Toy Turchin equations | **No licence** [V] | 2021-05-02 [V] · ★6 | REFERENCE | Use the papers instead | [gh](https://github.com/dmely/cliodynamics) |
| OpenVic-Simulation (military, diplomacy, politics) | Wars, war score, rebels, ideologies, elections (Victoria-2 abstractions) | MIT [V] | 2026-10-15 [V] | PORT (selectively) | No supply or attrition logistics | [gh](https://github.com/OpenVicProject/OpenVic-Simulation) |
| ndlib opinion dynamics | Voter, Sznajd, majority-rule, threshold and bias models for S1/S2 religion, ideology and rumour | BSD-2-Clause [V] | 2026-07-31 [V] | PORT (kernels) | Recast on the region graph | [gh](https://github.com/GiulioRossetti/ndlib) |
| Azgaar FMG cultures and religions | Seed culture and religion fields at t = 0 | MIT [V] | — | PORT | — | [gh](https://github.com/Azgaar/Fantasy-Map-Generator) |
| Freeciv | AI diplomacy, war and tech | **GPL-2.0** [V] | 2026-10-15 [V] · ★1.6k | REFERENCE | — | [gh](https://github.com/freeciv/freeciv) |
| ftomassetti/civs | Tribes → chiefdoms → nations, language drift (Clojure) | Apache-2.0 [V] | 2016-09-01 (dormant) [V] · ★150 | REFERENCE | — | [gh](https://github.com/ftomassetti/civs) |

### G. Whole-game and world-history projects (open-source grand strategy, procedural history)

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| **OpenVic** (Godot 4 front-end) | A province-map grand-strategy client: a Godot 4 front-end over a C++ GDExtension sim library [V COPYRIGHT and README] | MIT (code); logo CC-BY-SA-4.0 [V] | 2026-10-14 [V] · ★473 | PORT / REFERENCE (map view, GDExtension bridge) | This is the closest analogue to `rh-godot` + `rh-core` | [gh](https://github.com/OpenVicProject/OpenVic) |
| Unciv | Moddable Civ V remake | MPL-2.0 [V] | 2026-10-15 [V] · ★11.4k | REFERENCE | — | [gh](https://github.com/yairm210/Unciv) |
| Freeciv / OpenTTD / Simutrans | Mature strategy and transport sims | GPL-2.0 / GPL-2.0 / Artistic [V] | Active 2026 [V] | REFERENCE | OpenTTD's lockstep desync detection is worth reading | (§D, §F) |
| openage | **Curves:** keyframed, time-queryable state (cited in R03) | **GPL-3.0** [V] | 2026-10-15 [V] · ★14.5k | REFERENCE | Clean-room from its design doc | [gh](https://github.com/SFTtech/openage) |
| **Veloren** (GitLab) | Rust voxel RPG with `world/src/{civ,site,sim}`, `site/economy` and `rtsim/{ai,rule,generate}` [V tree]: civilisation and site generation, town plots and an NPC real-time simulation | **GPL-3.0** [V] | 2026-10-14 [V] · stars n/a | REFERENCE | **Same language.** The highest personal-mode value (see below). | [gitlab](https://gitlab.com/veloren/veloren) |
| JacksonAHannan/world-history-simulator | "Deterministic procedural-history simulator" with cohorts, food, trade, migration, control, languages and tech; conservation checks; checkpoint branching [U: search summary + repo description V] | **No licence**: "No software license grant has been supplied" [V README] | 2026-10-03 [V] · ★0 | REFERENCE | The closest conceptual twin. Read-only. | [gh](https://github.com/JacksonAHannan/world-history-simulator) |
| javolio/World-Generator | Civilisation growth and conflict on generated terrain | **No licence** [V] | 2013-12-29 [V] | AVOID | — | [gh](https://github.com/javolio/World-Generator) |
| **LegendsViewer-Next** | Dwarf Fortress Legends browser: entities, event collections, histories (.NET 10 / Vue) | MIT [V] | 2026-09-06 [V] · ★202 | REFERENCE / PORT (UX for Annals and person/place pages) | Proven "read a generated history" UX | [gh](https://github.com/Kromtec/LegendsViewer-Next) |
| LegendsBrowser | Older Java legends viewer | MIT [V] | 2020-10-10 [V] · ★63 | REFERENCE | — | [gh](https://github.com/robertjanetzko/LegendsBrowser) |

### H. `rh-minds`: utility AI, HTN, behaviour trees, beliefs, LLM-agent references

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| **Fluid HTN** | Compact game HTN planner (domain builder, methods, partial planning, world-state effects) | MIT [V] | 2026-03-18 [V] · ★457 | **PORT** (C# → Rust) | No mature Rust HTN crate exists (`htn` 0.2.1 has 24 downloads [V]) | [gh](https://github.com/ptrefall/fluid-hierarchical-task-network) |
| GTPyhop | Goal- and task-based HTN (Dana Nau) | Clear BSD (no patent grant) [V] | 2021-07-29 [V] · ★99 | REFERENCE / PORT | Add Clear-BSD to the allow-list if ported | [gh](https://github.com/dananau/GTPyhop) |
| big-brain | Utility AI with scorers, curves and thinkers (Bevy) | Apache-2.0 [V] | **Archived** [V]; last commit 2025-10-07; rel 0.22.0 (2024-11) · ★1.3k | REFERENCE / PORT design | Do not depend on it | [gh](https://github.com/zkat/big-brain) |
| bonsai-bt | Behaviour trees "for deterministic AI" | MIT [V] | 2026-09-22 [V]; rel 0.14.0 · ★1.1k, 113k dl | DEPEND (street-level BT execution) | — | [gh](https://github.com/Sollimann/bonsai) |
| bevy_behave | Bevy behaviour trees | MIT OR Apache-2.0 [V] | 2026-07-03 [V] · ★132 | REFERENCE | Bevy-coupled | [gh](https://github.com/RJ/bevy_behave) |
| Talk of the Town | Evidence-typed beliefs (paper; master lacks the module, per R04) | MIT [V] | 2024-09-23 (licence added; code 2021) [V] · ★106 | REFERENCE | "Port the paper, not the repo" (R04) | [gh](https://github.com/james-owen-ryan/talktown) |
| Concordia | Entity-component agent patterns, game-master (GM) pattern | Apache-2.0 [V] | Repo 2026-10-15 [V]; PyPI 2.4.0 (2026-03-06) [V] · ★1.8k | REFERENCE | "Not an officially supported Google product" [V]. Its LLM GM decides outcomes, which runs against the thesis. | [gh](https://github.com/google-deepmind/concordia) |
| Generative Agents | Memory stream with importance, recency and relevance; reflection | Apache-2.0 [V] | 2023-08-11 (dormant) [V] · ★22k | REFERENCE | — | [gh](https://github.com/joonspk-research/generative_agents) |
| AI Town | LLM town starter kit (Convex) | MIT [V] | 2026-08-25 [V] · ★10.6k | REFERENCE | — | [gh](https://github.com/a16z-infra/ai-town) |
| Neighborly | ECS settlement life-sim, life events | MIT [V] | **Archived**; "no longer maintained" notice dated 2026-04-07 [V] | REFERENCE | — | [gh](https://github.com/ShiJbey/neighborly) |
| Anansi | Simulation-driven storylet visual novels (Unity) | MIT [V] | 2025-09-19 [V] · ★78 | REFERENCE | Relevant to the storylet-hybrid pivot (`11`) | [gh](https://github.com/ShiJbey/Anansi) |
| FAtiMA Toolkit | OCC-style emotional appraisal and decay (C#) | Apache-2.0 [V] | 2024-05-31 [V] | REFERENCE / PORT | — | [gh](https://github.com/GAIPS-INESC-ID/FAtiMA-Toolkit) |
| pyactr | ACT-R declarative memory | **GPL-3.0** [V] | 2026-08-19 [V] · ★187 | REFERENCE | The activation equations are published, so implement from the papers | [gh](https://github.com/jakdot/pyactr) |

### I. `rh-sifter`, significance and legends

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| Felt | Datalog sifting patterns as action preconditions | **No licence** [V] | 2020-08-14 [V] · ★23 | REFERENCE | 291 lines; clean-room is cheap | [gh](https://github.com/mkremins/felt) |
| Winnow | Incremental, partial-match sifting DSL; `unless-event`; pruning | **No licence** [V] | 2021-10-05 [V] · ★33 | REFERENCE | The design ancestor of `rh-sifter` | [gh](https://github.com/mkremins/winnow) |
| statistical-sifting | "Select the Unexpected" rarity heuristic | **No licence** [V] | 2022-07-29 [V] | REFERENCE | Feeds the significance "rarity" component | [gh](https://github.com/mkremins/statistical-sifting) |
| **Viv** (Sifty) | Causal bookkeeping, knowledge relay, sifting patterns, tropes | **Proprietary, non-commercial** ("For non-commercial purposes only…") [V] | 2026-09-05 [V] · ★38 | REFERENCE | A commercial licence is available from Sifty | [gh](https://github.com/siftystudio/viv) |
| Ascent | Datalog in Rust (batch hindsight analysis) | MIT [V] | rel 0.8.1, 2026-08-29 [V] · ★590, 610k dl | DEPEND (offline) | — | [gh](https://github.com/s-arash/ascent) |
| crepe | Datalog procedural macro | MIT OR Apache-2.0 [V] | 2025-12-13 [V] · ★534 | Assess | — | [gh](https://github.com/ekzhang/crepe) |
| differential-dataflow | Incremental view maintenance (IVM) for recursive or aggregate rules | MIT [V] | 2026-07-29 [V]; rel 0.25.1 · ★3.0k | Assess | — | [gh](https://github.com/TimelyDataflow/differential-dataflow) |
| DBSP (Feldera) | IVM engine | MIT for the open-source edition; enterprise code excluded [V] | 2026-10-15 [V]; dbsp 0.366.0 · ★2.1k | Assess | Check that no enterprise paths are pulled in | [gh](https://github.com/feldera/feldera) |
| DuckDB | SQL twins of every pattern (already in `specs/`) | MIT [V] | 2026-10-15 [V]; PyPI 1.5.6 · ★42k | DEPEND | — | [gh](https://github.com/duckdb/duckdb) |

### J. Chronicle, storage and analytics

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| arrow-rs + parquet | Event store (~11 B/event typed), Arrow IPC keyframes | Apache-2.0 [V] | rel 60.0.0, 2026-10-15 [V] · ★3.6k, 88M dl | DEPEND | Fast major cadence; pin | [gh](https://github.com/apache/arrow-rs) |
| duckdb-rs | In-process analytics and views from Rust | MIT [V] | 2026-10-12 [V]; rel 1.10506.0 (2026-09-30) · ★981 | DEPEND | — | [gh](https://github.com/duckdb/duckdb-rs) |
| rusqlite | SQLite WAL journal | MIT [V·reg] | rel 0.40.2, 2026-08-08 · 126M dl | DEPEND | — | [crates.io](https://crates.io/crates/rusqlite) |
| polars / pyarrow | Python analytics | MIT / Apache-2.0 [V·reg] | PyPI 2.0.0 (2026-10-06) / 26.0.0 (2026-10-09) | DEPEND | — | [pypi](https://pypi.org/project/polars/) |

### K. `rh-server` and `rh-protocol`

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| tokio / axum / tokio-tungstenite | Async host, HTTP, WebSocket | MIT [V·reg] | rel 1.53.3 (2026-10-13) / 0.8.9 (2026-04) / 0.30.0 (2026-07) | DEPEND | — | [crates.io](https://crates.io/crates/axum) |
| **jsonrpsee** | JSON-RPC 2.0 server and client over WebSocket (ADR-0023 control plane) | MIT [V] | 2026-10-01 [V]; rel 0.26.1 · ★858, 26M dl | DEPEND | Saves hand-rolled RPC framing | [gh](https://github.com/paritytech/jsonrpsee) |
| **ts-rs** / specta | Generate TypeScript types from Rust structs (console ↔ server contract) | MIT [V] / MIT [V·reg] | 2026-10-02 [V]; rel 12.0.1 · ★1.9k, 18.6M dl | DEPEND | Removes a whole class of contract drift | [gh](https://github.com/Aleph-Alpha/ts-rs) |
| schemars | JSON Schema from Rust types (Registry and manifests) | MIT [V·reg] | rel 1.2.3, 2026-10-12 · 547M dl | DEPEND | — | [crates.io](https://crates.io/crates/schemars) |
| PyO3 + maturin | `rh-py` wheel | MIT OR Apache-2.0 [V] | 2026-10-14 [V]; rel 0.30.1 · ★16k | DEPEND | — | [gh](https://github.com/PyO3/pyo3) |

### L. World compiler, calibration and validation (M9, `rehoboam_calib`)

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| pydantic | Intent and manifest schemas (already used in `specs/`) | MIT [V·reg] | PyPI 2.14.1 (2026-10-11) | DEPEND | — | [pypi](https://pypi.org/project/pydantic/) |
| instructor | Typed LLM outputs with validation and retry | MIT [V] | 2026-10-10 [V]; PyPI 1.17.0 · ★14k | DEPEND | — | [gh](https://github.com/567-labs/instructor) |
| **SALib** | Morris screening, Sobol and FAST sensitivity (P1.10 names Morris) | MIT [V] | 2026-10-10 [V]; PyPI 1.6.0 · ★1.0k | DEPEND | — | [gh](https://github.com/SALib/SALib) |
| **pyABC** | ABC-SMC likelihood-free calibration against stylized facts | BSD-3-Clause [V] | 2026-08-31 [V]; PyPI 0.13.0 · ★221 | DEPEND | — | [gh](https://github.com/ICB-DCM/pyABC) |
| marimo / Rerun / Textual | Notebooks, time-scrubbable debug logs, ops TUI | Apache-2.0 / MIT OR Apache-2.0 / MIT [V] | 2026-10-15 / 2026-10-15 / 2026-10-09 [V] · ★23k / ★11.6k / ★37k | DEPEND | Already in `07` | [gh](https://github.com/rerun-io/rerun) |

### M. LLM gateway, structured generation and evals

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| llama.cpp | Local `llama-server` sidecar: grammars, slots, caching | MIT [V] | 2026-10-15 [V] · ★131k | DEPEND | — | [gh](https://github.com/ggml-org/llama.cpp) |
| vLLM / SGLang | Cluster serving | Apache-2.0 [V] | 2026-10-15 [V]; PyPI 0.31.0 / 0.5.21 · ★94k / ★37k | DEPEND | — | [gh](https://github.com/vllm-project/vllm) |
| llguidance / XGrammar | Grammar-constrained decoding | MIT / Apache-2.0 [V] | 2026-10-14 [V]; PyPI 1.9.1 / 0.2.8 · ★888 / ★2.0k | DEPEND | — | [gh](https://github.com/guidance-ai/llguidance) |
| LiteLLM (SDK only) | Provider adapters | MIT except `enterprise/` [V] | 2026-10-15 [V]; PyPI 1.105.1 · ★61k | DEPEND (optional, pinned with hashes, per `07`) | Avoid the `enterprise/` tree and the proxy | [gh](https://github.com/BerriAI/litellm) |
| **inspect_ai** | Eval framework: datasets, solvers, scorers, logs (for RP-200, ARG-300, INTENT-100, NOVEL-20) | MIT [V] | 2026-10-15 [V]; PyPI 0.3.281 · ★3.0k | DEPEND | Not in the plan | [gh](https://github.com/UKGovernmentBEIS/inspect_ai) |
| **promptfoo** | Declarative prompt tests and **red-teaming** in CI | MIT [V] | 2026-10-15 [V]; npm 0.124.1 · ★26k | DEPEND | Use for the Whisper-Counsel injection corpus | [gh](https://github.com/promptfoo/promptfoo) |
| NobodyWho | llama.cpp in Godot (GDExtension) | **EUPL-1.2** [V] | 2026-10-15 [V] · ★1.6k | DEPEND (conditional) | README claims linking does not extend the EUPL [V] | [gh](https://github.com/nobodywho-ooo/nobodywho) |

### N. Narrative compiler, templated Annals and exporters

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| Tracery / pytracery | Grammar expansion for templated Annals (Caves of Qud style) | Apache-2.0 [V] | 2024-11-03 / 2024-03-08 [V] · ★2.2k / ★262; PyPI `tracery` 0.1.1 (2017) | PORT (tiny) | Rust ports are dormant: tracery-rs MIT, 2021 [V] | [gh](https://github.com/galaxykate/tracery) |
| **improv** | *Model-backed* grammar: snippets filtered by tags from a world model, which matches "templated prose conditioned on sim facts" | MIT [V] | 2019-11-05 (dormant) [V] · ★146 | PORT (design ~1k LoC JS) | — | [gh](https://github.com/sequitur/improv) |
| Rant | Rust procedural templating language | MIT OR Apache-2.0 [V] | 2022-05-20 (4.0 alpha, dormant) [V] · ★215 | AVOID as dependency | — | [gh](https://github.com/rant-lang/rant) |
| ink / inkjs / **bladeink** | Authored storylets and in-world documents; Rust runtime and compiler | MIT / MIT / Apache-2.0 [V] | 2026-05-05 / 2026-09-01 / 2026-10-15 [V]; bladeink 2.0.0 (2026-07-18) · ★5.0k / ★658 / ★82 | DEPEND (if storylets are needed, e.g. after a G2 pivot) | — | [gh](https://github.com/bladecoder/blade-ink-rs) |
| WriteHERE | Recursive heterogeneous planning for long-form writing (EMNLP 2025) | README badge says MIT; **no LICENSE file** [V] | 2026-09-03 [V] · ★976 | REFERENCE (ask for a LICENSE file before porting) | R08 recorded it as MIT | [gh](https://github.com/principia-ai/WriteHERE) |
| StoryWriter (THU-KEG) | Event-based outline agent → chapters | **No licence** [V] | 2025-06-18 [V] · ★46 | REFERENCE | — | [gh](https://github.com/THU-KEG/StoryWriter) |
| DOC v2 | Detailed outline control for long stories | Apache-2.0 [V] | 2023-12-03 (dormant) [V] | REFERENCE / PORT prompts | — | [gh](https://github.com/yangkevin2/doc-storygen-v2) |
| MiniCheck | Grounded fact-checking (NLI) for scene QA | Code Apache-2.0 [V]; PyPI `minicheck` 0.4.0 MIT [V·reg]; **Bespoke-MiniCheck-7B weights CC BY-NC 4.0** [U] (README: "contact … for commercial use" [V]) | 2025-08-27 [V] · ★229 | DEPEND (code + a permissive checkpoint) | Flan-T5 checkpoint licence [U] | [gh](https://github.com/Liyan06/MiniCheck) |
| screenplain / fountain-js / rustwell | Fountain parsing and export | MIT [V] / MIT [V·reg] / [V·reg] | 2026-04-28 [V] / npm 1.2.4 (2023) / crate 0.3.1 (2026-04) | DEPEND | afterwriting is MIT but has **no LICENSE file** and has been dormant since 2024-09 [V] | [gh](https://github.com/vilcans/screenplain) |
| epub-builder | EPUB writer (Rust) | MPL-2.0 [V·reg] | rel 0.8.3, 2026-04-10 | DEPEND | Use this instead of EbookLib | [crates.io](https://crates.io/crates/epub-builder) |
| EbookLib | EPUB writer (Python) | **AGPL-3.0** [V] | 2026-10-07 [V]; PyPI 0.20 · ★1.8k | AVOID | Licence trap | [gh](https://github.com/aerkalov/ebooklib) |
| python-feedgen | RSS for the podcast feed | Dual BSD-2-Clause / LGPL-3.0 [V] | 2023-12-25 [V] | DEPEND (under BSD) | — | [gh](https://github.com/lkiesow/python-feedgen) |

### O. Console and debugging

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| deck.gl | Map layers with binary attributes | MIT [V] | 2026-10-15 [V]; npm 9.4.0 · ★14.7k | DEPEND | — | [gh](https://github.com/visgl/deck.gl) |
| sigma.js + graphology | WebGL causal graph | MIT [V] | 2026-10-15 / 2026-09-02 [V]; sigma 4.0.3 · ★12k / ★1.8k | DEPEND | — | [gh](https://github.com/jacomyal/sigma.js) |
| dagre / cosmos.gl | Small-DAG layout / GPU force layout for large graphs | MIT [V] | 2026-08-08 / 2026-10-14 [V] · ★5.8k / ★1.3k | DEPEND | — | [gh](https://github.com/cosmosgl/graph) |
| **Mosaic** + **duckdb-wasm** | Linked, crossfiltered views backed by DuckDB, in the browser, over Parquet | BSD-3-Clause / MIT [V] | 2026-10-15 / 2026-09-29 [V] · ★1.4k / ★2.1k | DEPEND | Turns the chronicle into an explorable dataset without server endpoints | [gh](https://github.com/uwdata/mosaic) |
| Perspective | Arrow-native pivot grid and charts (WASM) | Apache-2.0 [V] | 2026-10-13 [V] · ★11.3k (moved to `perspective-dev`) | DEPEND | — | [gh](https://github.com/perspective-dev/perspective) |
| **family-chart** / **topola** | Genealogy and dynasty views | MIT (LICENSE) / npm says ISC [V]; Apache-2.0 [V] | 2026-04-08 / 2026-10-04 [V] · ★787 / ★123 | DEPEND | Dynasties are central to M7 | [gh](https://github.com/donatso/family-chart) |
| Tauri | Optional desktop shell | MIT OR Apache-2.0 [V] | 2026-10-15 [V] · ★112k | DEPEND (Trial) | — | [gh](https://github.com/tauri-apps/tauri) |

### P. Godot client: terrain, towns, crowds, AI, assets

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| godot-rust gdext | Rust GDExtension (`rh-godot`) | MPL-2.0 [V] | 2026-10-04 [V]; rel 0.5.5 · ★5.3k | DEPEND | — | [gh](https://github.com/godot-rust/gdext) |
| Terrain3D | Clipmap terrain, 64 m–65 km | MIT [V] | 2026-10-07 [V] · ★4.4k | DEPEND | — | [gh](https://github.com/TokisanGames/Terrain3D) |
| **Sky3D** | Day and night cycle, atmosphere | MIT [V] | 2026-05-19 [V] · ★913 | DEPEND | — | [gh](https://github.com/TokisanGames/Sky3D) |
| **ProtonScatter** | Rule-based prop and vegetation scattering | MIT [V] | 2026-09-27 [V] · ★3.0k (mirror of Codeberg) | DEPEND | Needs a seeded RNG for growth stability | [gh](https://github.com/HungryProton/scatter) |
| **godot-road-generator** | Road meshes and lane-following traffic | MIT [V] | 2026-10-12 [V] · ★1.2k | DEPEND / PORT | — | [gh](https://github.com/TheDuckCow/godot-road-generator) |
| OpenVAT-for-Godot | VAT + animated MultiMesh crowds | MIT [V] | 2026-10-11 [V] · ★122 (created 2026-03) | DEPEND | Young project | [gh](https://github.com/antzGames/OpenVAT-for-Godot) |
| OpenVAT (Blender add-on) | Bakes vertex-animation textures | **GPL-3.0** [V] | 2026-10-02 [V] · ★288 | DEPEND as an **offline tool** | Baked textures are outputs, not GPL code [U]. Fine even if distributed. | [gh](https://github.com/sharpen3d/openvat) |
| Godot-Octahedral-Impostors | Octahedral impostor baker and shader | MIT [V] | Default branch last commit 2021-06-03 [V] · ★313 | PORT (Godot 4 update needed [U]) | — | [gh](https://github.com/wojtekpil/Godot-Octahedral-Impostors) |
| godot-motion-matching | Avatar locomotion | MIT [V] | 2026-06-04 [V] · ★247 | DEPEND | Avatar only (R06) | [gh](https://github.com/GuilhermeGSousa/godot-motion-matching) |
| godot-constraint-solving | WFC and CSP in Godot 4 | MIT [V] | 2025-05-13 [V] · ★465 | DEPEND / PORT | Prefer porting WFC into Rust `rh-worldgen` so the sim and the client share it | [gh](https://github.com/AlexeyBond/godot-constraint-solving) |
| WaveFunctionCollapse / DeBroglie | Reference WFC; WFC with non-local constraints | MIT [V] | 2026-03-21 / 2025-12-21 [V] · ★25k / ★527 | PORT | — | [gh](https://github.com/BorisTheBrave/DeBroglie) |
| **LimboAI** / Beehave | Behaviour trees and state machines in Godot | MIT [V] | 2026-10-04 / 2026-09-20 [V] · ★3.1k / ★3.3k | DEPEND (client-side presentation AI only) | Authoritative decisions stay in `rh-minds` | [gh](https://github.com/limbonaut/limboai) |
| Dialogue Manager | Dialogue UI and runtime | MIT [V] | 2026-10-14 [V] · ★3.9k | DEPEND | LLM lines enter as data | [gh](https://github.com/nathanhoad/godot_dialogue_manager) |
| watabou TownGeneratorOS (MFCG) | Ward-based medieval town plans | **GPL-3.0** [V] | 2019-04-07 [V] · ★1.9k | REFERENCE | Already excluded in `07` | [gh](https://github.com/watabou/TownGeneratorOS) |
| ProbableTrain MapGenerator | Tensor-field street networks | **GPL-3.0 + LGPL-3.0** (COPYING + COPYING.LESSER) [V] | 2022-01-14 [V] · ★1.5k | REFERENCE | — | [gh](https://github.com/ProbableTrain/MapGenerator) |
| Lively | Windows wallpaper host | **GPL-3.0** [V] | 2026-09-29 [V] | DEPEND as a separate app | We ship a package for it, not its code | [gh](https://github.com/rocksdanister/lively) |
| **KayKit** (Medieval Hexagon, Medieval Builder) | 200+ stylised medieval tiles, buildings and props | CC0 (itch.io listing; Godot Asset Library) [U] | 2026 listings [U] | DEPEND (assets) | Paid tiers add extras (also CC0 [U]) | [itch](https://kaylousberg.itch.io/kaykit-medieval-hexagon) |
| Kenney | Large CC0 asset library | CC0: "public domain licensed (CC0)" per kenney.nl/support via search [U] | — | DEPEND (assets) | Don't use the Kenney logo | [site](https://kenney.nl/support) |
| **Quaternius Universal Animation Library 1–2** | Humanoid animations on one rig: UAL1 has 120+ (45 in the free tier); UAL2 adds 130+ | CC0 (itch.io page via search) [U] | — | DEPEND (assets) | Avoid Mixamo for redistributable builds (Adobe terms) [U] | [itch](https://quaternius.itch.io/universal-animation-library) |

### Q. Audio (TTS, ASR, mastering, watermarking)

| Project | What it gives us | Licence (verified) | Last activity · adoption | Mode | Notes / risks | Link |
|---|---|---|---|---|---|---|
| Chatterbox (Turbo, Nano, Multilingual V3) | CPU-friendly TTS with paralinguistic tags [V README] | Code MIT [V]; weights MIT [U] | 2026-07-21 [V]; PyPI 0.1.7 · ★27k | DEPEND | — | [gh](https://github.com/resemble-ai/chatterbox) |
| Pocket TTS | ~CPU real-time TTS | Code MIT [V]; weights CC-BY-4.0 [U]; per-voice licences on HF [V README] | 2026-10-14 [V]; PyPI 3.3.0 · ★9.9k | DEPEND | Track attribution per voice | [gh](https://github.com/kyutai-labs/pocket-tts) |
| Qwen3-TTS | Voice design and cloning (drama) | Apache-2.0 [V] (weights [U]) | 2026-03-17 [V] · ★13.8k | DEPEND | — | [gh](https://github.com/QwenLM/Qwen3-TTS) |
| MOSS-TTSD | Multi-speaker dialogue synthesis | Apache-2.0 [V README + LICENSE] | 2026-09-06 [V] · ★1.4k | DEPEND | — | [gh](https://github.com/OpenMOSS/MOSS-TTSD) |
| Kokoro | Small TTS | Apache-2.0 code and weights [V README] | 2025-08-06 [V] · ★9.3k | DEPEND (fallback) | Uses espeak-ng (GPL) for some languages [V README]. Keep it out of process or disabled. | [gh](https://github.com/hexgrad/kokoro) |
| Moonshine | Streaming ASR | Code and most models MIT; **legacy non-English non-streaming models under a non-commercial Community Licence**; TTS/G2P under third-party terms [V] | 2026-08-24 [V] · ★11.2k | DEPEND (English or streaming models) | — | [gh](https://github.com/moonshine-ai/moonshine) |
| whisper.cpp / faster-whisper | QA transcription for the ASR gate | MIT [V] | 2026-10-12 / 2026-10-15 [V] · ★54k / ★26k | DEPEND | — | [gh](https://github.com/SYSTRAN/faster-whisper) |
| sherpa-onnx | Offline TTS, ASR and VAD with many language bindings | Apache-2.0 [V] | 2026-10-15 [V] · ★15k | DEPEND (alternative runtime) | — | [gh](https://github.com/k2-fsa/sherpa-onnx) |
| AudioSeal | Localised watermarking | MIT, "including the model weights" (README) [V] | 2026-05-19 [V]; PyPI 0.2.0 · ★796 | DEPEND | — | [gh](https://github.com/facebookresearch/audioseal) |
| pyloudnorm | ITU-R BS.1770 / EBU R128 loudness | MIT [V] | 2026-01-04 [V] · ★783 | DEPEND | — | [gh](https://github.com/csteinmetz1/pyloudnorm) |
| **tts-audiobook-tool** | Long-form pipeline with STT-verified retakes ("keeps the most accurate take"), EBU R128, M4B/FLAC with word timings, chapter cut points [V README] | MIT [V] | 2026-10-15 [V] · ★224 | **PORT** (pipeline patterns) / REFERENCE | Supports many NC backends (Fish, VibeVoice, etc.). Use permissive ones only. | [gh](https://github.com/zeropointnine/tts-audiobook-tool) |

---

### R. Lo-fi glyph client (art phase 1; added 2026-10-15, ADR-0029)

Verified with shallow clones (LICENSE + `git log -1`) and the npm and crates.io APIs. api.github.com, opengameart, kenney.nl, itch.io and int10h were blocked, so those facts are [U].

| Name | What | Licence | Last activity / version | Mode |
|---|---|---|---|---|
| deck.gl | `IconLayer`/`TileLayer`, binary attributes; host for the `GlyphGridLayer` | MIT [V] | 2026-10-15; 9.4.0 [V] | **DEPEND** (already Adopt) |
| wglt | WebGL roguelike terminal, single batched draw | MIT [V]; bundles an IBM ROM-dump font and a CC BY-SA font [V] | 2024-05-07; 0.6.4 [V] | **PORT** (shader pattern only; never its fonts) |
| beamterm | WebGL2 terminal, one instanced draw | MIT [V] | 2026-04-01; 1.0.0 [V] | Fallback |
| PixiJS + @pixi/tilemap | 2D WebGL engine | MIT [V] | 2026-10-10; 8.22.0 [V] | Fallback |
| ratatui + crossterm | Rust TUI | MIT [V] | 2026-10-09; 0.30.2 / 0.29 [V] | DEPEND (optional `rh-tui`) |
| ratzilla | ratatui in the browser | MIT/Apache [V] | 2026-07-04; 0.3.1 [V] | Assess |
| rot.js | Roguelike toolkit | BSD-3 [V] | 2024-11-13; 2.2.1 [V] | REFERENCE (one draw call per cell) |
| bracket-lib + Rust roguelike tutorial | CP437 consoles | MIT [V] | crates last released 2022-10 [V] | REFERENCE (owns the game loop) |
| Hauberk + malison | Dart web roguelike | MIT [V] | 2026-09 / 2025-11 [V] | REFERENCE (message log and panel UX) |
| malwoden; doryen-rs | TypeScript / Rust roguelike libraries | MIT [V] | 2022 / 2023 [V] | AVOID (dormant) |
| Brogue CE; Cataclysm-DDA; Goblin Camp | Look and UX references | AGPL-3.0; CC BY-SA 3.0; GPL-3.0 [V] | active / active / 2018 [V] | REFERENCE only |
| **Bitlands** (DragonDePlatino) | Full CP437 12×18 sheet + ~500 tintable creature sprites; map, building, item, plant sheets | **CC-BY 4.0** [V] | 2019-01-18 [V] | **DEPEND** (default glyph pack; attribution tracked) |
| **Unscii-8/16** | Bitmap fonts (ttf, otf, woff) | PD/CC0 [V]; **`unscii-16-full` and `unifont.hex` are GPL** [V] | 2021-02-24 [V] | **DEPEND** (dialogue and log panels) |
| Kenney 1-Bit / Micro Roguelike | 16×16 / 8×8 tiles | CC0 [U] | — | DEPEND (archive the licence on download) |
| DawnLike | 16×16 tileset | **Conflict:** CC-BY-SA 3.0 vs CC-BY 4.0 [V both] | — | HOLD |
| int10h Oldschool PC fonts | CP437 TTFs | CC BY-SA 4.0 [U] | — | HOLD (share-alike) |
| DF community tilesets | Tilesets | No LICENSE [V] | — | AVOID |
| Lively; Plash | Wallpaper hosts | GPL-3.0 (separate app) [V]; closed source [V] | 2026-09 / 2025-10 | DEPEND as user-installed apps |

**Effort.** 7–11 pw for the lo-fi Diorama + Street client, against about 18 pw net (36 baseline) for the Godot client plus art content and SPK-1 to SPK-3. About 40% of the lo-fi work carries over if the 3D art phase is chosen later.

## Projects we had missed (new finds)

These are not in `07` or the research dossiers (R01–R11), or they appear there only as "ideas" when a stronger reuse mode is available.

| # | Find | Why it matters | Mode |
|---|---|---|---|
| 1 | **OpenVic-Simulation** (MIT [V], active) | A fixed-point, headless C++ grand-strategy sim library with tests, structurally similar to `rh-core` behind a Godot client. Gives M4 market and producer code and fixed-point exp/sin LUTs. Its own determinism guarantees are unverified [U]. | PORT |
| 2 | **Starsim CRN slot hashing** (MIT [V]) | A production-proven version of our `H(seed,id)` and Foresight CRN idea | PORT |
| 3 | **Covasim testing interventions** (MIT [V]) | Sensitivity, loss-to-follow-up and delays: exactly the "testing base rates" M5 needs | PORT |
| 4 | **LASER migration kernels** (MIT [V]) | Gravity, competing destinations, Stouffer and radiation, ready to port | PORT |
| 5 | **humanleague** (MIT [V]) | Integer IPF microsynthesis for P1.2 ledger initialisation | DEPEND/PORT |
| 6 | **SALib + pyABC** (MIT / BSD-3 [V]) | Morris/Sobol sensitivity and ABC calibration for the M9 harness and Gate G1 | DEPEND |
| 7 | **inspect_ai + promptfoo** (MIT [V]) | Ready eval and red-team harnesses for RP-200, ARG-300, INTENT-100, NOVEL-20 and the Whisper-injection corpus | DEPEND |
| 8 | **jsonrpsee + ts-rs** (MIT [V]) | JSON-RPC control plane and Rust→TS types for the console | DEPEND |
| 9 | **Mosaic + duckdb-wasm, Perspective** (BSD-3 / MIT / Apache [V]) | Chronicle exploration in the browser over Parquet | DEPEND |
| 10 | **family-chart / topola** (MIT / Apache [V]) | Dynasty and genealogy views for M7 | DEPEND |
| 11 | **Fluid HTN** (MIT [V]) | The missing HTN planner for `rh-minds` (no usable Rust crate exists) | PORT |
| 12 | **bonsai-bt** (MIT [V]) | Deterministic Rust behaviour trees for street-level execution | DEPEND |
| 13 | **Sky3D, ProtonScatter, godot-road-generator, Octahedral Impostors, LimboAI, Dialogue Manager** (MIT [V]) | Remove most of the generic Godot plumbing in P5 and P6 | DEPEND/PORT |
| 14 | **tts-audiobook-tool** (MIT [V]) | Cited in R07 but not adopted; it already implements the ASR-gated audio-drama render loop | PORT |
| 15 | **improv** (MIT [V]) | World-model-filtered grammar, the right pattern for templated Annals before any LLM | PORT |
| 16 | **mapgen4, mewo2/terrain, naming-language, terrain-erosion-3-ways** (Apache/MIT [V]) | Worldgen pieces that complement FMG | PORT |
| 17 | **ndlib** (BSD-2 [V]) | Opinion-dynamics kernels for S1/S2 and rumour spread | PORT |
| 18 | **bazaarBot** (MIT [V]) | Emergent price discovery for merchant actors | PORT |
| 19 | **Random123** (BSD-3 [V]) | Philox reference and known-answer tests (the plan names Philox but not the source) | PORT |
| 20 | **twox-hash** (MIT [V]) | An allow-list-clean XXH3 | DEPEND |
| 21 | **Veloren** (GPL-3 [V]) | Rust town, site and economy generation plus rtsim. Reference only, unless personal-use mode is adopted. | REFERENCE |
| 22 | **world-history-simulator** (no licence [V]) | Same concept (deterministic history, branching, conservation checks), tiny and new. Watch it and ask the author about a licence. | REFERENCE |
| 23 | **LegendsViewer-Next** (MIT [V]) | Cited in R02, but its UX is directly portable to Annals and person/place pages | REFERENCE/PORT |
| 24 | **epub-builder** (MPL-2.0) in place of **EbookLib (AGPL-3.0)** | Avoids a licence trap in the exporters | DEPEND / AVOID |

**Corrections to the current plan (licence facts):**

- R08 records WriteHERE as MIT. There is **no LICENSE file** at HEAD; only a README badge [V].
- `07` lists Bespoke-MiniCheck-7B as "check". Its weights are **non-commercial** [U]; its README says commercial use requires contacting the vendor [V].
- `07` names "xxh3" without a crate. The popular `xxhash-rust` is **BSL-1.0** [V].
- epydemix (not in the plan) is **GPL-3.0** [V].
- big-brain and Neighborly are now **archived** [V].

---

## What cannot be bootstrapped (and why): the genuinely novel core

No public project combines deterministic, replayable multi-century emergent history at 10⁵–10⁶ people with a causal chronicle and live story sifting. The pieces below are where Rehoboam's value lives, and reuse covers about 0–20% of each.

1. **The determinism contract at scale.** Primitives exist: ChaCha, Philox, libm, fixed, CRN hashing. The *contract* does not: phase order, `AccessSpec` conflict checking, keyed streams per system, fixed-point thresholds, a parallel-safe event bus, and a Merkle hash ladder that is identical across thread counts and platforms. Every ported algorithm must be hardened to it. That hardening is the hidden cost of every PORT above.
2. **The 1M-row ledger with hydration as a plane-B view, scheduled outcomes for everyone, material-input scoping, pins and observer invariance** (`05.2`, ADR-0024/0025). ActivitySim and Covasim are batch tools. Nothing open hydrates a live bubble from a macro ledger without changing history.
3. **The coupled macro-history systems:**
   - SDT polity (Ψ, elite overproduction, fiscal state);
   - succession and dynast hazards;
   - campaign war with supply and attrition;
   - climate → harvest → entitlement → revolt couplings;
   - calibration to stylized facts.

   No permissive code exists for SDT, succession or logistics. OpenVic and Freeciv model different eras and abstractions. This is where "history happens" (G1) and where the thesis (G2) lives.
4. **Minds and epistemics.** These cover evidence-typed beliefs, channels (the hive among them), theory of mind for flagged propositions, mimic cover utility, and Whisper Counsel that is parsed, grounded and deterministically scored. References (Talk of the Town paper, Concordia, Generative Agents) give vocabulary, not code that respects "the simulation decides".
5. **An incremental sifter at 10⁹–10¹¹ events**, with anchored `attr@eN` reads, SQL-twin equivalence, suspense from open partial matches, and retroactive significance with a visibility sidecar. The published state of the art (Winnow) was benchmarked on five characters (R02), and the main designs (Felt, Winnow, Viv) are unlicensed or non-commercial.
6. **The Mechanics Registry and World Compiler**: intent → mapping table → expander → semantic validators → liveness gate and certification → gap reports. Typed LLM output libraries help, but the semantics (what an intent *means* in the engine) are ours.
7. **Foresight**: forked rollout workers, a prophecy DSL, CRN A/B, belief-resampled rollouts and validity tests. Only the CRN idea is reusable.
8. **A canon-gated narrative compiler** with zero contradictions against the *causal chronicle graph*. WriteHERE, DOC and MiniCheck supply planning and NLI patterns; grounding against Rehoboam's typed chronicle is new.
9. **Street-from-seed towns that stay stable under growth** (SPK-2). FMG and MFCG give 2D plans; deterministic, growth-stable 3D towns shared by the sim and the client are ours.
10. **Time-Machine UX semantics**: speed bands, run-until predicates, auto-pause on suspense, a causal inspector and rating capture. The visualisation libraries are reusable; the interaction design is the product.

---

## Top-15 leverage list with estimated person-weeks saved

**Unit.** 1 pw is one week of one directed workstream (director plus agents).

**Method.** "Saved" is net of integration, against the simplest viable home-grown substitute. "Incremental" is the saving *beyond what the current plan already assumes*.

**Caveat.** These are estimates, with roughly ±50% error per line.

| Rank | Reuse item(s) | Subsystem | Mode | Saved (pw, est.) | In current plan? | Incremental vs plan (pw) |
|---|---|---|---|---|---|---|
| 1 | Godot add-on stack: Terrain3D, Sky3D, ProtonScatter, road-generator, OpenVAT-for-Godot, Octahedral Impostors, WFC add-on, LimboAI or Beehave, Dialogue Manager, motion matching | Godot client (P5–P6) | DEPEND/PORT | **8–14** | Partly (Terrain3D, OpenVAT, WFC, motion matching) | 4–7 |
| 2 | Arrow/Parquet + DuckDB + duckdb-rs + rusqlite + zstd | Chronicle, keyframes, SQL twins | DEPEND | **8–12** | Yes | 0 |
| 3 | Azgaar FMG port (+ mapgen4, mewo2 terrain and naming-language, erosion) | `rh-worldgen` (M1) | **PORT** | **6–10** | As "ideas" | 2–4 |
| 4 | CC0 content: KayKit medieval kits, Kenney, Quaternius incl. Universal Animation Library 1–2 | Godot content (P5) | DEPEND (assets) | **6–10** | Mentioned in R06 | 2–4 |
| 5 | Console visualisation: deck.gl, sigma/graphology, dagre, cosmos.gl, **Mosaic + duckdb-wasm**, Perspective, **family-chart / topola** | Console (P2) | DEPEND | **6–10** | Partly | 2–4 |
| 6 | LLM runtime and structured decoding: llama.cpp, vLLM/SGLang, llguidance, XGrammar, instructor | LLM gateway | DEPEND | **5–8** | Yes | 0–1 |
| 7 | **Starsim + Covasim + LASER** (SEIR modules, testing sensitivity/delay/loss, Dx, quarantine, CRN, migration kernels) | M5 contagion (P3) | **PORT** | **4–7** | No | 4–7 |
| 8 | Audio: Chatterbox/Pocket/Qwen3-TTS/MOSS-TTSD + faster-whisper/Moonshine + AudioSeal + pyloudnorm + **tts-audiobook-tool** loop | Audio (P4, P6) | DEPEND/PORT | **4–7** | Mostly | 1–2 |
| 9 | **OpenVic-Simulation** (markets, pops, producers, fixed-point LUTs; selective military and politics) | M4 economy, `rh-core` math (P1) | **PORT** | **3–6** | No | 3–6 |
| 10 | tokio/axum/tungstenite + **jsonrpsee** + **ts-rs**/specta + schemars | `rh-server` / `rh-protocol` (P2) | DEPEND | **3–6** | Partly | 1–2 |
| 11 | **inspect_ai / promptfoo** | Evals and red-team (P3–P6) | DEPEND | **3–5** | No | 3–5 |
| 12 | **SALib + pyABC** (+ marimo, Rerun) | M9 calibration and validation (P1) | DEPEND | **3–5** | Partly | 2–3 |
| 13 | **Fluid HTN** port + bonsai-bt + big-brain curve design + GTPyhop | `rh-minds` (P1, P3) | PORT/DEPEND | **2–4** | No | 2–4 |
| 14 | Templated Annals and exporters: improv/Tracery patterns or bladeink, screenplain/fountain-js, epub-builder, feedgen | Annals (P2), narrative compiler (P4) | PORT/DEPEND | **2–4** | Partly (Fountain) | 1–3 |
| 15 | Determinism primitives: rand_chacha, vendored rand_distr (BTPE, ZIGNOR), Random123 Philox, libm, fixed, twox-hash, Starsim CRN, proptest/insta | `rh-core` (P0) | DEPEND/PORT | **2–4** | Mostly | 1–2 |
| | **Total** | | | **≈ 65–110** | | **≈ 30–55** |

**Just outside the top 15:**

- Population synthesis (humanleague, PopulationSim, synthpops): 2–3 pw.
- Datalog/IVM escalation (Ascent, DBSP), only if recursive rules are needed: 2–4 pw.
- WriteHERE, DOC and MiniCheck planning and QA patterns: 1–3 pw.
- ndlib opinion kernels: 1–2 pw.
- LegendsViewer-Next UX: 1–2 pw.
- bazaarBot: about 1 pw.

**Interpretation.**

- The plan's existing DEPEND choices already capture most of the value of ranks 2, 6 and 15.
- The **new** leverage is in the PORT items (ranks 3, 7, 9, 13, 14) and in tooling (ranks 11 and 12). These are where the agents should start from existing code rather than a blank file.
- Realistically, expect the **low end (~30 pw incremental)**: every PORT needs fixed-point conversion, removal of unordered iteration, golden vectors and attribution.

---

## Personal-use mode: what extra it unlocks and at what cost

**What changes.** Rehoboam would be treated as never distributed. GPL code could then be copied freely, AGPL would be harmless on localhost, and non-commercial terms would be satisfied.

**What does *not* change:**

- **No-licence repos** (Felt, Winnow, StoryWriter, world-history-simulator) are still not licensed for copying.
- **Territorial** licences may still exclude you.

**Recommendation.** Keep ADR-0021 strict for **Tier A** crates:

- `rh-core`
- `rh-sys-*`
- `rh-minds`
- `rh-sifter`
- `rh-events`
- `rh-snapshot`
- `rh-protocol`
- `rh-worldgen`
- `rh-godot`

Add two explicit tiers:

- **Tier B, tools.** GPL or AGPL programs may run **out of process** as oracles, bakers or hosts and are never shipped. This is already compatible with publishing.
- **Tier C, `lab/`.** Personal-only code, weights and outputs, enforced by cargo-deny, pip-licenses and license-checker so they cannot leak into Tier A. Generated artifacts are tagged `licence_class=nc_personal` in `provenance.jsonl`.

### What each blocked item would unlock

**Removal cost** includes Rehoboam's determinism penalty. Code in Tier A changes history hashes, which forces golden-run, calibration and saved-world re-baselining.

| Item (licence) | What personal-use mode unlocks | Extra saving (pw, est.) | Where it would live | Removal cost later | Recommendation |
|---|---|---|---|---|---|
| **Veloren** (GPL-3, Rust) | Copy town, plot and site generation, `site/economy`, civ placement and `rtsim` NPC rules directly. Same language. | 3–6 | `rh-worldgen`, P6 schedules | **High:** 4–8 pw clean-room + re-baseline. Town layouts and seeds change, and the Godot client is affected too. | **No** in Tier A. Tier C visual prototypes only. |
| **watabou MFCG** (GPL-3, Haxe) | Port ward-based town plans | 2–4 | `rh-worldgen` | **Med-high:** 3–5 pw; saved towns change shape | Reimplement from the published description |
| **ProbableTrain MapGenerator** (LGPL-3/GPL-3, TS) | Port tensor-field streets | 1–2 | `rh-worldgen` | **Medium:** 2–3 pw | Reimplement (the algorithm is well documented) |
| **Undiscovered Worlds** (GPL-3, C++) | Climate, rivers and lakes code | 1–2 | `rh-worldgen` | **Medium:** 2–3 pw + re-baseline | Reference; FMG/mapgen4 are permissive alternatives |
| **openage curves** (GPL-3) | Keyframed, time-queryable containers | ~1 | `rh-snapshot` | **Medium:** 1–2 pw + hash re-baseline | Clean-room from the design doc |
| **OpenABM-Covid19 / CovidSim / JUNE / epydemix** (GPL-3) | Port testing, tracing and app logic, ABC calibration | 1–2 beyond the MIT Starsim/Covasim | `rh-sys-contagion` | **High** if in core: 2–4 pw + recalibration | **Run as out-of-process validation oracles (Tier B, allowed today).** Do not port. |
| **Viv** (proprietary NC, JS) | Run Viv's runtime as a sifting and knowledge-relay oracle; port causal bookkeeping, knowledge relay and tropes | 2–4 | Oracle: Tier C. Port: `rh-sifter`/`rh-events`. | Oracle **Low** (delete the harness). Port **High** (3–6 pw + chronicle schema migration). | Oracle only, or buy Sifty's commercial licence if publishing |
| **EbookLib** (AGPL-3) | Mature Python EPUB writer | ~0.5 | Narrative exporters | **Low:** 2–3 days to swap to epub-builder or pandoc | Not worth it; use epub-builder now |
| **pyactr** (GPL-3) | ACT-R memory implementation | ~0.5 | `rh-minds` | Medium | Not worth it; the equations are published |
| **Bespoke-MiniCheck-7B** (CC BY-NC [U]) | The strongest small grounded fact-checker for NOVEL-20 QA | 1–2 (quality, not code) | Narrative QA (out of process) | **Low:** swap the model and re-tune thresholds, about 1 pw | Allow in Tier C with provenance tags |
| **Non-commercial TTS** (F5-TTS, XTTS-v2, Fish S2-Pro, VibeVoice, Higgs; Hold list in `07` [U]) | More expressive audio-drama voices | Quality gain | Audio renderer (out of process) | **Low technically** (re-render), but voice identity changes across episodes | Allow in Tier C; re-render with permissive voices before any publication |
| **Moonshine legacy non-English models** (Community Licence, NC [V]) | Non-English push-to-talk | — | Audio (out of process) | Low | Allow in Tier C if needed |
| **Felt / Winnow / statistical-sifting / StoryWriter / world-history-simulator** (no licence) | **Nothing.** Personal use does not grant copying rights. | 0 | — | — | Ask the authors for a licence; Felt is 291 lines, so clean-room is cheap |
| **Hunyuan3D-class territorial licences** | Possibly nothing, depending on your residence | ? | Asset pipeline | — | Check the territory clause first |

**Net.** Personal-use mode adds about **10–20 pw** of theoretical saving. Only **3–6 pw** of that is cheap to undo: oracles, weights and EbookLib. The rest (Veloren, MFCG, Viv ported into core) would cost **as much or more to remove** as it saves, because Rehoboam's history *is* its core code. Unless the director decides never to publish (`12` Q13), the expected value favours keeping Tier A strict.

---

## Sources

All accessed 2026-10-15. Repo facts come from LICENSE/COPYING at HEAD and `git log -1` via shallow clone. Stars come from the GitHub search API. Registry facts come from the crates.io, PyPI and npm JSON APIs.

**Simulation and ABM**

1. https://github.com/starsimhub/starsim (LICENSE MIT; `starsim/distributions.py` CRN; HEAD 2026-10-06)
2. https://pypi.org/project/starsim/ (3.7.2)
3. https://github.com/starsimhub/covasim (redirect from InstituteforDiseaseModeling/covasim; LICENSE MIT; `covasim/interventions.py`)
4. https://pypi.org/project/covasim/ (4.0.0)
5. https://github.com/laser-base/laser-core (LICENSE MIT; `src/laser/core/migration.py`)
6. https://github.com/laser-base/laser-generic (LICENSE MIT)
7. https://pypi.org/project/laser-core/ (1.0.5)
8. https://github.com/epistorm/epydemix (LICENSE GPL-3.0)
9. https://github.com/BDI-pathogens/OpenABM-Covid19 (LICENSE GPL-3.0)
10. https://github.com/mrc-ide/covid-sim (LICENSE.md GPL-3.0)
11. https://github.com/IDAS-Durham/JUNE (LICENSE GPL-3.0)
12. https://github.com/thoughtworks/epirust (LICENSE AGPL-3.0; archived)
13. https://github.com/scikit-mobility/scikit-mobility (BSD-3-Clause)
14. https://github.com/GiulioRossetti/ndlib (BSD-2-Clause)
15. https://github.com/krABMaga/krABMaga (MIT) and https://crates.io/crates/krabmaga
16. https://github.com/AshvinPerera/Syren-ABM-Framework (MIT) and https://crates.io/crates/syren
17. https://github.com/mesa/mesa (Apache-2.0) and https://pypi.org/project/mesa/
18. https://github.com/ActivitySim/activitysim (BSD-3-Clause)
19. https://github.com/ActivitySim/populationsim (BSD-3-Clause)
20. https://github.com/matsim-org/matsim-libs (`matsim/LICENSE`: GPL-2.0-or-later)
21. https://github.com/synthpops/synthpops (MIT; "no longer being actively maintained")
22. https://github.com/virgesmith/humanleague (MIT) and https://pypi.org/project/humanleague/
23. https://github.com/Dirguis/ipfn (MIT)

**Grand strategy and world history**

24. https://github.com/OpenVicProject/OpenVic (LICENSE.md MIT; COPYRIGHT)
25. https://github.com/OpenVicProject/OpenVic-Simulation (LICENSE.md MIT; source tree incl. `core/object/FixedPoint*`, `economy/`)
26. https://github.com/yairm210/Unciv (MPL-2.0)
27. https://github.com/freeciv/freeciv (COPYING GPL-2.0)
28. https://github.com/OpenTTD/OpenTTD (COPYING.md GPL-2.0)
29. https://github.com/aburch/simutrans (LICENSE.txt Artistic licence; SVN mirror)
30. https://github.com/SFTtech/openage (copying.md GPL-3.0)
31. https://gitlab.com/veloren/veloren (LICENSE GPL-3.0; `world/src/{civ,site,sim}`, `rtsim/src`)
32. https://github.com/JacksonAHannan/world-history-simulator (README: no licence grant)
33. https://github.com/ftomassetti/civs (Apache-2.0)
34. https://github.com/javolio/World-Generator (no licence)
35. https://github.com/Kromtec/LegendsViewer-Next (MIT)
36. https://github.com/robertjanetzko/LegendsBrowser (MIT)
37. https://github.com/larsiusprime/bazaarBot (MIT)
38. https://github.com/dmely/cliodynamics (no licence)

**World generation**

39. https://github.com/Azgaar/Fantasy-Map-Generator (MIT)
40. https://github.com/redblobgames/mapgen4 (Apache-2.0)
41. https://github.com/mewo2/terrain (MIT)
42. https://github.com/mewo2/naming-language (MIT)
43. https://github.com/Mindwerks/worldengine (MIT; `setup.py` requires PyPlatec) and https://pypi.org/project/worldengine/
44. https://github.com/Mindwerks/plate-tectonics (LGPL-3.0)
45. https://github.com/dandrino/terrain-erosion-3-ways (MIT)
46. https://github.com/weigert/SimpleHydrology (README "MIT"; no LICENSE file)
47. https://github.com/JonathanCRH/Undiscovered_Worlds (GPL-3.0; README determinism note) and https://github.com/JonathanCRH/Undiscovered_Worlds_Classic
48. https://github.com/amitp/mapgen2 (MIT)
49. https://github.com/watabou/TownGeneratorOS (GPL-3.0)
50. https://github.com/ProbableTrain/MapGenerator (COPYING GPL-3.0 + COPYING.LESSER LGPL-3.0)
51. https://github.com/mxgmn/WaveFunctionCollapse (MIT)
52. https://github.com/BorisTheBrave/DeBroglie (MIT)
53. https://github.com/Stoeoef/spade and https://crates.io/crates/spade
54. https://github.com/mourner/delaunator-rs (ISC)
55. https://github.com/andreesteve/voronoice (MIT)
56. https://github.com/petgraph/petgraph and https://crates.io/crates/petgraph
57. https://github.com/evenfurther/pathfinding (Cargo.toml `Apache-2.0/MIT`)
58. https://github.com/georust/rstar and https://github.com/sdd/kiddo
59. https://github.com/Auburn/FastNoiseLite (MIT) and https://github.com/Razaekel/noise-rs
60. https://crates.io/crates/hexx, https://crates.io/crates/geo and https://crates.io/crates/i_overlay

**Determinism and core**

61. https://github.com/rust-random/rand and https://crates.io/crates/rand_chacha
62. https://github.com/rust-random/rand_distr (`src/binomial.rs` BTPE; `src/poisson.rs`; README "Portability and libm")
63. https://github.com/DEShawResearch/random123 (BSD-3-Clause)
64. https://crates.io/crates/libm
65. https://gitlab.com/tspiteri/fixed and https://crates.io/crates/fixed
66. https://github.com/shepmaster/twox-hash (MIT; XXH3-64/128)
67. https://github.com/DoumanAsh/xxhash-rust (BSL-1.0)
68. https://crates.io/crates/indexmap, https://crates.io/crates/rustc-hash, https://crates.io/crates/rayon, https://crates.io/crates/rkyv, https://crates.io/crates/postcard, https://crates.io/crates/bitcode, https://crates.io/crates/bincode, https://crates.io/crates/zstd
69. https://crates.io/crates/proptest, https://crates.io/crates/insta, https://crates.io/crates/criterion, https://crates.io/crates/cargo-deny
70. https://github.com/bevyengine/bevy and https://crates.io/crates/bevy_ecs
71. https://github.com/Ralith/hecs

**Minds**

72. https://github.com/zkat/big-brain (Apache-2.0; archived)
73. https://github.com/Sollimann/bonsai (MIT)
74. https://github.com/RJ/bevy_behave
75. https://github.com/ptrefall/fluid-hierarchical-task-network (MIT)
76. https://github.com/dananau/GTPyhop (Clear BSD)
77. https://crates.io/crates/htn and https://crates.io/crates/htn_planner (immature HTN crates)
78. https://github.com/james-owen-ryan/talktown (MIT)
79. https://github.com/google-deepmind/concordia (Apache-2.0) and https://pypi.org/project/gdm-concordia/
80. https://github.com/joonspk-research/generative_agents (Apache-2.0)
81. https://github.com/a16z-infra/ai-town (MIT)
82. https://github.com/ShiJbey/neighborly (MIT; archived; notice 2026-04-07)
83. https://github.com/ShiJbey/Anansi (MIT)
84. https://github.com/GAIPS-INESC-ID/FAtiMA-Toolkit (Apache-2.0)
85. https://github.com/jakdot/pyactr (GPL-3.0)

**Sifting and data**

86. https://github.com/mkremins/felt (no LICENSE)
87. https://github.com/mkremins/winnow (no LICENSE)
88. https://github.com/mkremins/statistical-sifting (no LICENSE)
89. https://github.com/siftystudio/viv (LICENSE.txt: proprietary, non-commercial)
90. https://github.com/s-arash/ascent (MIT)
91. https://github.com/ekzhang/crepe
92. https://github.com/TimelyDataflow/differential-dataflow (MIT)
93. https://github.com/feldera/feldera (MIT open-source edition)
94. https://github.com/apache/arrow-rs and https://crates.io/crates/parquet
95. https://github.com/duckdb/duckdb (MIT) and https://pypi.org/project/duckdb/
96. https://github.com/duckdb/duckdb-rs (MIT)
97. https://crates.io/crates/rusqlite and https://pypi.org/project/polars/

**Server, protocol and calibration**

98. https://crates.io/crates/tokio, https://crates.io/crates/axum, https://crates.io/crates/tokio-tungstenite
99. https://github.com/paritytech/jsonrpsee (MIT)
100. https://github.com/Aleph-Alpha/ts-rs (MIT), https://crates.io/crates/specta, https://crates.io/crates/schemars
101. https://github.com/PyO3/pyo3
102. https://github.com/SALib/SALib (MIT)
103. https://github.com/ICB-DCM/pyABC (BSD-3-Clause)
104. https://github.com/marimo-team/marimo
105. https://github.com/rerun-io/rerun and https://pypi.org/project/rerun-sdk/
106. https://github.com/Textualize/textual

**LLM**

107. https://github.com/ggml-org/llama.cpp (MIT)
108. https://github.com/vllm-project/vllm (Apache-2.0)
109. https://github.com/sgl-project/sglang (Apache-2.0)
110. https://github.com/guidance-ai/llguidance (MIT)
111. https://github.com/mlc-ai/xgrammar (Apache-2.0)
112. https://github.com/BerriAI/litellm (MIT except `enterprise/`)
113. https://github.com/567-labs/instructor (MIT)
114. https://github.com/UKGovernmentBEIS/inspect_ai (MIT)
115. https://github.com/promptfoo/promptfoo (MIT)
116. https://github.com/nobodywho-ooo/nobodywho (EUPL-1.2; README licence note)

**Narrative**

117. https://github.com/galaxykate/tracery (Apache-2.0)
118. https://github.com/aparrish/pytracery (Apache-2.0)
119. https://github.com/caranatar/tracery-rs (MIT)
120. https://github.com/rant-lang/rant (MIT/Apache-2.0)
121. https://github.com/sequitur/improv (MIT)
122. https://github.com/maetl/calyx (MIT)
123. https://github.com/inkle/ink (MIT) and https://github.com/y-lohse/inkjs (MIT)
124. https://github.com/bladecoder/blade-ink-rs (Apache-2.0)
125. https://github.com/principia-ai/WriteHERE (README MIT badge; no LICENSE file)
126. https://github.com/THU-KEG/StoryWriter (no LICENSE)
127. https://github.com/yangkevin2/doc-storygen-v2 (Apache-2.0)
128. https://github.com/Liyan06/MiniCheck (Apache-2.0; README commercial-use note) and https://pypi.org/project/minicheck/
129. https://www.promptlayer.com/models/bespoke-minicheck-7b (secondary: CC BY-NC 4.0) [U]
130. https://github.com/vilcans/screenplain (MIT)
131. https://github.com/ifrost/afterwriting-labs (README and package.json MIT; no LICENSE file)
132. https://www.npmjs.com/package/fountain-js and https://crates.io/crates/rustwell
133. https://crates.io/crates/epub-builder (MPL-2.0)
134. https://github.com/aerkalov/ebooklib (AGPL-3.0)
135. https://github.com/lkiesow/python-feedgen (BSD-2-Clause / LGPL-3.0)

**Console**

136. https://github.com/visgl/deck.gl (MIT)
137. https://github.com/jacomyal/sigma.js (MIT) and https://github.com/graphology/graphology (MIT)
138. https://github.com/dagrejs/dagre (MIT)
139. https://github.com/cosmosgl/graph (MIT)
140. https://github.com/uwdata/mosaic (BSD-3-Clause)
141. https://github.com/duckdb/duckdb-wasm (MIT)
142. https://github.com/perspective-dev/perspective (Apache-2.0)
143. https://github.com/donatso/family-chart (LICENSE MIT; npm ISC)
144. https://github.com/PeWu/topola (Apache-2.0)
145. https://github.com/tauri-apps/tauri (MIT/Apache-2.0)

**Godot**

146. https://github.com/godot-rust/gdext (MPL-2.0)
147. https://github.com/TokisanGames/Terrain3D (MIT)
148. https://github.com/TokisanGames/Sky3D (MIT)
149. https://github.com/HungryProton/scatter (MIT)
150. https://github.com/TheDuckCow/godot-road-generator (MIT)
151. https://github.com/antzGames/OpenVAT-for-Godot (MIT)
152. https://github.com/sharpen3d/openvat (GPL-3.0)
153. https://github.com/wojtekpil/Godot-Octahedral-Impostors (MIT)
154. https://github.com/GuilhermeGSousa/godot-motion-matching (MIT)
155. https://github.com/AlexeyBond/godot-constraint-solving (MIT)
156. https://github.com/limbonaut/limboai (MIT)
157. https://github.com/bitbrain/beehave (MIT)
158. https://github.com/nathanhoad/godot_dialogue_manager (MIT)
159. https://github.com/rocksdanister/lively (GPL-3.0)
160. https://kenney.nl/support (CC0 statement; read via search, the site was unreachable) [U]
161. https://kaylousberg.itch.io/kaykit-medieval-hexagon (CC0; via search) [U]
162. https://quaternius.itch.io/universal-animation-library (CC0; via search) [U]

**Audio**

163. https://github.com/resemble-ai/chatterbox (MIT)
164. https://github.com/kyutai-labs/pocket-tts (MIT code; README voice licences)
165. https://github.com/QwenLM/Qwen3-TTS (Apache-2.0)
166. https://github.com/OpenMOSS/MOSS-TTSD (Apache-2.0)
167. https://github.com/hexgrad/kokoro (Apache-2.0; README espeak-ng note)
168. https://github.com/moonshine-ai/moonshine (LICENSE: MIT plus Community Licence exceptions)
169. https://github.com/ggml-org/whisper.cpp (MIT)
170. https://github.com/SYSTRAN/faster-whisper (MIT)
171. https://github.com/k2-fsa/sherpa-onnx (Apache-2.0)
172. https://github.com/facebookresearch/audioseal (MIT; README weights note)
173. https://github.com/csteinmetz1/pyloudnorm (MIT)
174. https://github.com/zeropointnine/tts-audiobook-tool (MIT; README features)

**Licence interpretation (secondary; not legal advice)**

175. https://www.blackduck.com/blog/agpl-affero-gpl-3.html (GPL private use; AGPL §13) [U]
176. https://writing.kemitchell.com/2021/01/24/Reading-AGPL (AGPL §13 triggers on modification and remote interaction) [U]
177. https://interoperable-europe.ec.europa.eu/collection/eupl/licence-compatibility-permissivity-reciprocity-and-interoperability (EUPL linking position, as quoted in the NobodyWho README) [U]
178. https://github.com/RyanBabij/WorldSim and https://github.com/ReedOei/History-Generator (surfaced by the procedural-history search; not assessed further) [U]
