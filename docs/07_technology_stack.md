# 07 — Technology Stack and Radar

*Status: Draft v1.0 (2026-10-15). Versions are as verified by research R01–R11 in Oct 2026. **Pin exact versions in lockfiles, and re-verify at each phase start.** Licence notes are summaries; ADR-0021 governs.*

**Radar rings**
- **Adopt**: the default choice.
- **Trial**: use it, behind a facade or spike, before committing.
- **Assess**: watch it, and prototype only if a need appears.
- **Hold**: do not use.

---

## 1. Selected stack

| Layer | Choice | Version (Oct 2026) | Licence | Ring | Rationale | Fallback |
|---|---|---|---|---|---|---|
| Core language | Rust | 1.9x stable | MIT/Apache | Adopt | Determinism, performance, embeddable everywhere, compiler as an agent guardrail | C++ (Flecs) |
| Parallelism | rayon (fixed chunking) | 1.12 | MIT/Apache | Adopt | Data-parallel kernels | std threads |
| RNG | rand_chacha (streams) / Philox (Random123 port) + SplitMix64/xxh3 | 0.10 | MIT/Apache, BSD | Adopt | Counter-based, keyed, seekable | — |
| Math | libm (pure Rust) | 0.2.16 | MIT/Apache | Adopt | Bit-stable transcendentals | — |
| ECS (named actors) | bevy_ecs, standalone, pinned | 0.20.0 | MIT/Apache | Trial | Observers, change detection, relationships | hecs 0.11 |
| Snapshot serialisation | Arrow IPC (arrow-rs) + zstd | arrow-rs current; zstd | Apache-2.0 / BSD | Adopt | Columnar, zero-copy, readable by DuckDB, Polars and Rerun | rkyv 0.8 (in-process ring) |
| Messages | postcard / bitcode | 1.1 / 0.6 | MIT/Apache | Adopt | Compact Rust messages | — |
| ~~bincode~~ | — | 3.0 is a tombstone | — | **Hold** | RUSTSEC-2025-0141 | postcard |
| Event store | Parquet (arrow-rs/parquet) | current | Apache-2.0 | Adopt | ~11 B/event typed | — |
| Analytics | DuckDB (read-only views) | 1.5.6 (2.0 coming) | MIT | Adopt | 18–105 ms pattern joins over 10M events | Polars 2.0 |
| Journal / metadata | SQLite (WAL) | 3.45+ | Public domain | Adopt | Small, durable | — |
| Incremental IVM (escalation) | differential-dataflow / dbsp | 0.25.1 / 0.366 | MIT / MIT-Apache | Assess | Recursive or aggregate sifting rules | — |
| Datalog (offline research) | Ascent / Soufflé | 0.8.1 / 2.5 | MIT / UPL | Assess | Batch hindsight analysis | DuckDB SQL |
| Server | tokio + axum + tokio-tungstenite | current | MIT | Adopt | WebSocket control and state planes | — |
| Python bindings | PyO3 + maturin | 0.30 / 1.15 | MIT/Apache | Adopt | Wheel for compiler, notebooks and tests | — |
| Python data | pyarrow, polars, duckdb | 26 / 2.0 / 1.5.6 | Apache / MIT | Adopt | Analytics | pandas |
| Notebooks | marimo | 0.25 | Apache-2.0 | Adopt | Reactive, .py files, Rerun embedding | Jupyter |
| Sim debugging | Rerun | 0.39 | MIT/Apache | Adopt | Time-scrubbable spatial and series logs | — |
| Ops TUI | Textual | 8.2 | MIT | Adopt | Headless dashboard | — |
| Console | TypeScript + Vite | current | MIT | Adopt | — | — |
| Console map | deck.gl | 9.4 | MIT | Adopt | Orthographic view, binary attributes | regl |
| Causal graph | Sigma.js + graphology; dagre / Graphviz-WASM for small DAGs | 4.0.3; 3.1 / 1.29 | MIT; MIT / Apache | Adopt | WebGL graphs of thousands of nodes | Cytoscape.js 3.34 |
| Desktop shell | Tauri (Win/macOS); Electron on Linux | 2.12 / 44 | MIT/Apache; MIT | Trial | Small binaries; Linux WebKitGTK risk | Browser |
| 3D engine | Godot | 4.7.2 (4.8 dev) | MIT | Adopt (from P5) | Small runtime, wallpaper-friendly | Unreal 5.x (Hold unless Spike 1 fails) |
| Godot bindings | godot-rust gdext | 0.5.5 (api-4-7) | MPL-2.0 | Trial | Hot paths in Rust; shares `rh-worldgen` | godot-cpp |
| Terrain | Terrain3D or custom clipmap | 1.0.2 | MIT | Trial | Heightmaps 64 m–65 km | Custom mesher |
| Crowds | OpenVAT-for-Godot pattern (VAT MultiMesh) | Godot 4.5+ | MIT | Trial | 1k–5k animated at 60 FPS | Impostors |
| World-gen reference | Azgaar FMG algorithms (reimplemented) | FMG 2.0 refactor | MIT | Adopt (ideas) | Region graph, routes, states | — |
| Town / street generation | Own: ward plans + tensor fields + shape grammar + WFC | — | own; WFC is MIT | Adopt | GPL MFCG code excluded | — |
| Local LLM runtime | llama.cpp `llama-server` sidecar | b11669+ | MIT | Adopt | Cross-platform, grammars, slots, caching | NVIGI (Windows/NVIDIA) |
| Cluster LLM serving | vLLM / SGLang | 0.31 / 0.5.21 | Apache-2.0 | Adopt (cluster) | Batch throughput on Linux GPUs | llama.cpp server |
| Structured generation | Provider structured outputs; llguidance; XGrammar | 1.9.1 / 0.2.8 | MIT / Apache | Adopt | Grammar-constrained decoding | GBNF |
| ~~Outlines in runtime~~ | — | 1.3.3 | Apache | **Hold** (runtime); Assess (offline) | Python-only front end | — |
| LLM gateway | In-house adapter (~300 LoC); LiteLLM SDK optional, pinned with hashes | 1.105 | MIT | Adopt | Avoid proxy CVEs | — |
| Local NPC models | Qwen3.5-4B/9B, Gemma 4 E4B / 26B-A4B, Qwen3.6-35B-A3B | 2026 | Apache-2.0 (verify cards) | Trial (RP-200 eval) | Chosen by eval | Cloud small models |
| Frontier synthesis | Claude Opus 5.5 / Fable 5.1 (batch + cache); cross-family critic (GPT-6.x / Gemini) | Oct 2026 | Provider ToS | Adopt | Best verified creative-writing results | Open GLM-5.x / DeepSeek V4 on cluster |
| Fact-check NLI | MiniCheck-class (Bespoke-MiniCheck-7B) | — | check | Trial | Cheap claim verification | LLM checker |
| TTS (real-time) | Chatterbox-Nano/Turbo; Pocket TTS | 2026 | MIT; MIT + CC-BY-4.0 | Trial | CPU, ~200 ms to first audio | Kokoro (without espeak) |
| TTS (drama) | Qwen3-TTS 1.7B; IndexTTS-2.5; MOSS-TTSD v1.0 | 2026 | Apache; bilibili licence; Apache | Trial | Voice design, emotion, dialogue | ElevenLabs v3 (BYOK) |
| ASR | Moonshine streaming; Whisper-turbo (QA) | 2026 | MIT | Adopt | Push-to-talk, WER gate | Parakeet |
| SFX / music generation (offline) | MOSS-SoundEffect v2; ACE-Step 1.5; Stable Audio Open small | 2026 | Apache; MIT; community < $1M | Trial | Asset variations | Libraries |
| Watermarking | AudioSeal; Perth | — | MIT | Adopt | Synthetic-media marking | — |
| Fountain tooling | screenplain / afterwriting | — | MIT | Adopt | Validation and export | — |
| CI | GitHub Actions + cargo-nextest + criterion + cargo-deny | — | — | Adopt | Gates (`09`) | — |

## 2. Explicitly rejected (Hold)

| Item | Why |
|---|---|
| Flecs via Python | No maintained binding (PyPI 0.0.2 prototype). |
| Micropolis code | GPLv3 plus EA Section-7 terms, and the kernel is trivial to rewrite. |
| Ceptre | Anti-Capitalist Software License; Standard ML command-line prototype only. |
| unified-planning per action | 11.9 ms per validation; Fast Downward wrapper is GPL. |
| Felt / Winnow / df-storyteller code | Unlicensed / AGPL and dormant. Kept as reference designs. |
| Concordia runtime | Its LLM game master decides outcomes, which is the opposite of the thesis. Reference only. |
| AgentTorch, FLAME GPU 2 | AGPL. FLAME is also CUDA-only. |
| Kùzu, CozoDB, DDlog, Materialize | Archived / dormant / archived / BSL with a server. |
| SQLite R*Tree for hydration | 378 ms per tick of updates at 5k movers. |
| vLLM/Ollama inside a desktop client | Linux-only / global service. |
| Hunyuan3D (all), TRELLIS.2 (nvdiffrast), MeshAnything, HY-Motion, SMPL/AMASS-trained motion | Territorial or non-commercial terms. |
| F5-TTS, XTTS-v2, Fish S2-Pro, Higgs v3, Voxtral TTS, VibeVoice TTS, Piper, espeak-ng in-process | Non-commercial or GPL. |
| AudioCraft, MMAudio, TangoFlux, YuE2, Suno/Udio | Non-commercial or litigation. |
| Generative world models as renderer | VRAM, resolution and identity drift in 2026. Assess "engine + neural re-render" later. |

## 3. Assess list (watch quarterly)

- **Godot 4.8:** texture streaming; any GPU skinning or crowd work.
- **Bevy:** many-to-many relations; editor progress.
- **DuckDB 2.0:** storage format; Quack client-server.
- **Matrix-Game 4.0-style neural re-render** of engine frames for a premium Diorama mode.
- **NVIGI CUDA-in-Graphics** for LLM/renderer GPU sharing on Windows.
- New open models: role-play-tuned, hybrid-attention caching maturity in llama.cpp.
- **Fish Audio Drama 3 / next-gen open dialogue TTS** under commercial-friendly terms.
- **LadybugDB** (Kùzu fork) sustainability, in case graph queries become necessary.
- **Learned emulators / surrogate models** of the simulation for cheaper Foresight (ADR-0015 revisit).
