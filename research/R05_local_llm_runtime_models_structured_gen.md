# R05 — Local LLM Runtimes, Model Selection, Structured Generation, Grounded Role-Play & API Economics (as of 2026-10-15)

**Evidence tags.** **[V]** = verified this session from a primary source (repo README/docs/license file, official pricing page, PyPI metadata). **[S]** = secondary (search-result summary, aggregator, press); plausible but not checked against a primary. **[U]** = analyst recollection that could not be re-verified (Hugging Face, arXiv, NVIDIA/Steam/news sites were unreachable from this sandbox and the shared web-search quota ran out mid-session). **Inference** = my own reasoning. Treat [S]/[U] items as leads, not facts.

---

## TL;DR

1. **The draft's model choices are out of date.** Qwen2.5-3B and Llama-3.2-3B (Sep 2024) and Llama-3.3-70B (Dec 2024) have been replaced as the obvious picks by Qwen3.5 0.8B/2B/4B/9B (2026-03-02) [1], Gemma 4 E2B/E4B/26B-A4B/31B (2026-04-02, Apache-2.0) [4][5], Qwen3.6-27B/35B-A3B (Apr 2026) and Qwen3.8-27B (2026-08-14) [1][3]. Llama 3.x also brings a custom licence with "Built with Llama" attribution [14]. A 70B model is not a consumer-PC model.
2. **Embed llama.cpp. Do not ship Ollama, vLLM or LM Studio.** llama.cpp is MIT-licensed, has daily builds for Windows/macOS/Linux (Vulkan, CUDA, Metal, ROCm), a grammar/JSON-schema engine, slots, prompt caching and speculative decoding [20][21][22]. For decode speed, Vulkan roughly matches CUDA (RTX 4090: 188 vs 186 tok/s; RTX 3060: 76 vs 76) [42][43]. Ship it as a private sidecar `llama-server` first, and move in-process later if needed.
3. **The "vLLM: 50+ concurrent conversations at >100 tok/s" claim is wrong for this product.** vLLM "does not support Windows natively" [33] and by default takes 92% of VRAM [34], which would starve Godot. The player talks to one NPC at a time. What we need is one low-latency foreground stream plus 1–4 background slots.
4. **"Outlines guarantees 100% valid manifests on first pass" is false as written.** Constrained decoding only guarantees conformance to the schema subset that the engine supports. Truncation at `max_tokens`, refusals, enum casing (Anthropic documents all three [54]) and silently skipped schema keywords (llama.cpp [23]) still break outputs. Semantic validity is never guaranteed. Also, Outlines is a Python library: we cannot ship it inside a Godot game.
5. **World-compiler architecture:** the LLM emits a small *intent* object (discriminated unions, registry-generated enums). A deterministic compiler expands that into the 100k–1M-person manifest. A validator and a bounded repair loop follow. No LLM should ever write 100k citizens.
6. **Interactive latency target:** Mantella's docs aim for an LLM response in under 0.5 s [40]. On consumer GPUs, prompt prefill dominates time-to-first-token (TTFT), so persona prefixes have to stay cached. Qwen3.5 (Gated-DeltaNet hybrid [1]) and Gemma (sliding-window attention, SWA) need checkpoint-aware caching in llama.cpp [22][38]. Verify that this works before committing.
7. **Knowledge-bounded role-play cannot be done by prompting alone.** On TimeChara, GPT-4o scores 64.5% overall and 46% on "future" knowledge leaks; Mistral-7B averages 46.8% [60]. Let the simulation own the truth: a constrained *speech-act plan* (reveal/conceal/lie/deflect over belief IDs) followed by free-text realisation and a post-hoc claim check.
8. **A procedural world is easier on this axis than famous-IP role-play.** The model has no parametric knowledge of Rehoboam's world, so the main risk is *fabrication* and *context contamination*, not leaking canon (inference).
9. **Per-archetype LoRA is low priority.** Mantella's own Skyrim fine-tune (Llama-3-8B) is "now very outdated and performs worse than newer, non fine-tuned models" [41]. Fine-tunes lose value quickly.
10. **Offline frontier work is cheap and needs no free tiers.** Prices per Mtok in/out: Claude Opus 5.5 $4/$20, Sonnet 5.5 $2/$10, Haiku 5.5 $0.10/$0.50. Batch is 50% off, with 128K synchronous / 300K batch-beta output [55][56]. A full novel comes out around $3–15 in batch; a world compile costs under $0.10 (inference). Free tiers are fine for modders (Mantella defaults to OpenRouter's free Gemma 4 26B-A4B [39]) but are not a product dependency.
11. **Simulation budget reality check:** 1M inhabitants × 1,000 years leaves room for LLM calls only for named characters at rare moments. A 0.1%-per-year call rate is already about 1M calls, roughly 17 GPU-days at 1.5 s each (inference).

---

## Draft claims checked

| Draft claim | Verdict | Evidence |
|---|---|---|
| Outlines gives "100% valid JSON on first pass" | **False as stated.** Syntactic validity is achievable; full validity is not guaranteed | Outlines advertises "Guaranteed valid structure" [47], but with Ollama it just passes a JSON schema to Ollama's own engine [48], and with API models it relies on provider features [47]. llama.cpp's schema converter skips unsupported keywords *silently* [23]. Anthropic lists refusal, `max_tokens` truncation and enum-casing drift as remaining failures, plus a limit of 24 optional / 16 union parameters [54]. Semantic constraints (population totals, valid succession) are outside any grammar. |
| World compiler on Qwen 2.5 7B / Llama 3.3 70B / free endpoints | **Outdated / unsuitable** | Qwen3.5-9B (Mar 2026) and Qwen3.6/3.8-27B supersede Qwen2.5 [1]. Llama 3.3 is Dec 2024, 70B, and community-licensed [14]. A 70B model at Q4 is about 40 GB (inference), which is not consumer-local. Free endpoints are rate-limited and change without notice ([U]). |
| "Inexpensive/free-tier LLMs" for compile | **Use cheap paid tiers instead** | Haiku 5.5 is $0.10/$0.50 per Mtok [55]; GPT-6-luna is $0.10/$0.50 [58]; Gemini 3.5 Flash-Lite is $0.30/$2.50 [57]. A compile costs fractions of a cent to a few cents. |
| vLLM serves "50+ concurrent pedestrian conversations at >100 tok/s" | **Irrelevant and wrong platform** | Linux only [33], defaults to 92% of VRAM [34], server-oriented. Per-stream speed at 50-way concurrency on one consumer GPU would be far below 100 tok/s (aggregate ≠ per-stream; inference). The player converses with one NPC at a time. |
| NPC model: Qwen 2.5 3B / Llama 3.2 3B via Ollama/vLLM | **Outdated; wrong runtime** | Successors: Qwen3.5-4B, Gemma 4 E4B, Ministral 3 3B, SmolLM3-3B [1][4][9][10]. Ollama is a system-wide service that users must install (Mantella has users install it [39]). vLLM has no Windows support [33]. |
| Citizen "knows only what they have witnessed", can lie and conceal | **Not achievable with prompting alone** | Point-in-time hallucination is high even for GPT-4o (46% on future-knowledge probes) [60]. Small models are worse [60]. Needs an architectural fix (§6). |
| Frontier reasoning model for novels/screenplays | **Reasonable** | Batch at 50% off [55]. Output caps: 128K synchronous, 300K on batch with a beta header (Anthropic) [56]; GPT-6 has 128K [58]; Gemini 65,536 [58]. |
| LLM at rare high-stakes decisions, constrained to a legal action set | **Good idea** | Generate enums from the live legal-action registry. Let the model reason first, then constrain the decision (CRANE finding [52]). |
| LLM judges player persuasion | **Risky as specified** | A player's text can manipulate the judge (prompt injection; [U] reports of players talking AI NPCs into rewards in NetEase's *Where Winds Meet*). Restructure as feature extraction plus a deterministic decision (§Recommendations). |

---

## Landscape / state of the art

### 1. Open-weight models (Oct 2026)

**~1–4B (fits next to a game on 6–8 GB VRAM, or runs on CPU)**

| Model | Released | License | Notes / evidence of quality | Fit |
|---|---|---|---|---|
| Qwen3.5-0.8B/2B/4B | 2026-03-02 [1] | Apache-2.0 [S][2] | Hybrid Gated-DeltaNet architecture, native vision, thinking/non-thinking modes [1][2]. Artificial Analysis index (reasoning): 4B=27, 2B=16, 0.8B=9 [S][2] | **Primary candidate** (4B) |
| Gemma 4 E2B / E4B | 2026-04-02 | **Apache-2.0** (first time for Gemma; Gemma 3 used the Gemma Terms) [S][5] | τ2-bench retail tool use: E4B 57.5%, E2B 29.4% [S][4]. Mantella's suggested local model is Gemma 4 E4B Q4_K_M [39] | **Primary candidate** (E4B) |
| Ministral 3 3B (also 8B, 14B) | 2025-12 [S] | Apache-2.0 [S][9] | Base, instruct and reasoning variants; vision; 256K context [S] | Alternative |
| SmolLM3-3B | 2025-07 [U] | Apache-2.0 [U] | Trained on 11T tokens, 128K context; claims to beat Llama-3.2-3B and Qwen2.5-3B [10] | Fully open data; weaker |
| Phi-4-mini (3.8B) | 2025 [U] | MIT [S] | No Phi-5 found; cookbook is still Phi-4 [16] | Ageing |
| Granite 4.0 micro / h-micro (3B) | 2025-10; 4.1/4.2 in 2026 [S] | Apache-2.0 [11] | Dense and hybrid-Mamba variants [11] | Enterprise-flavoured |
| LFM2 / LFM2.5 (350M–2.6B; 24B-A2B) | 2025–26 | **LFM Open License**: free commercial use only under $10M annual revenue [S][12] | Strong edge tooling [S] | **Licence risk** |
| Llama 3.2 1B/3B | 2024-09-25 [14] | Llama 3.2 Community License: 700M-MAU clause, "Built with Llama" display requirement, EU restriction on the multimodal models [14] | Superseded | Avoid |

**~7–14B (12–16 GB tier)**

| Model | Released | License | Notes | Fit |
|---|---|---|---|---|
| Qwen3.5-9B | 2026-03-02 [1] | Apache-2.0 [S] | AA index 32 [S][2]; 262K context [S] | **Primary mid-tier** |
| Ministral 3 8B / 14B | 2025-12 [S] | Apache-2.0 [S] | Vendor VRAM guidance at FP8: 12 GB / 24 GB [S] | Alternative |
| Granite 4.0 h-tiny (hybrid MoE) | 2025-10 [U] | Apache-2.0 [11] | Low active parameters | Niche |
| "Gemma 4 12B" | — | — | Mentioned only by aggregators; **not** in DeepMind's lineup [S][4] | Treat as non-existent |

**~20–35B incl. MoE (24 GB+ / Apple ≥36 GB)**

| Model | Released | License | Notes | Fit |
|---|---|---|---|---|
| Qwen3.6-35B-A3B (MoE, ~3B active) | 2026-04-16 [1] | Apache-2.0 [S] | Fast because only ~3B parameters are active. Note: Qwen's README says the licence ships per model on HF [1] | **High tier** |
| Qwen3.6-27B / Qwen3.8-27B (dense) | 2026-04-22 / 2026-08-14 [1] | Apache-2.0 [S][3] | 3.8-27B: thinking on by default, 262K context [S][3] | High tier / offline |
| Gemma 4 26B-A4B / 31B | 2026-04-02 | Apache-2.0 [S] | τ2-bench 85.5% / 86.4% [S][4]. Mantella's default cloud model is the 26B-A4B [39] | **High tier** |
| gpt-oss-20b (21B, 3.6B active) | 2025-08 | Apache-2.0 [7] | MXFP4, "runs within 16GB"; requires the harmony format [7] | Good reasoner; worse at role-play [U] |
| Nemotron 3 Nano 30B-A3B | 2025-12 [S] | NVIDIA Nemotron Open Model License; Nemotron 3.5 reportedly under OpenMDW-1.1 [S][13] | — | Check licence |
| Meta "Muse Glimmer" 30B | 2026-08-10 [S] | Apache-2.0 [S][15] | Meta's own repo still lists Llama 4 (Apr 2025) as the latest Llama [14]. No "Llama 4.5" exists in that list | Unverified |
| Mistral Small 4 | 2026-03-16 [S][8] | Apache-2.0 [S] | Actually 119B-A6B; **not** a 24B model [S][8] | Out of class |

**Role-play-specific models.** CoSER-8B/70B (Llama-3.1-based, trained on persona simulations from books) [19]. "HER" (2026) adds reasoning and RL for role-play [19][65]. Community RP fine-tunes exist, but (a) they inherit the base licence, (b) many are tuned for NSFW, and (c) role-play evaluation is immature. BenchLM says it has "no defensible roleplay-quality score" [S][18]. rp-benchmark's authors report that their LLM-judge disagrees with users about half the time [S][17]. **Inference:** pick the base model with your own eval, not leaderboards.

### 2. Runtimes for shipping inside a desktop game

| Runtime | Version/date | License | Platforms | Embeddable / redistributable | Notes |
|---|---|---|---|---|---|
| **llama.cpp** (libllama / `llama-server`) | b11669, 2026-10-15 (daily builds) [21] | MIT [20] | Win/macOS/Linux. CUDA 12.4/13.4, Vulkan, ROCm, SYCL, Metal, CPU [21] | **Yes.** C API or sidecar server. CUDA runtime DLLs ship as a separate zip [21] | GBNF + JSON schema + llguidance [23][46]; slots, continuous batching, `--cache-reuse`, slot save/restore, context checkpoints for SWA/hybrid models, draft-model speculation [22]. ggml.ai joined Hugging Face on 2026-02-20; MIT licence retained [S][24] |
| Ollama | v0.40.x (Oct 2026) [25] | MIT [V] | Win/macOS/Linux | Technically yes, practically awkward. It is a global service with its own model store and fixed port. Inference | MLX runner is the default on Apple Silicon since v0.40.0 [25]. JSON-schema output only (as used via Outlines) [48] |
| LM Studio SDK (`@lmstudio/sdk`, `lmstudio` py) | py 1.5.0 (Aug 2025) [PyPI] | SDK MIT [26] | — | **No.** The SDK is a *client* of the LM Studio desktop app [26], which the user must install ([U]: app is closed-source) | Good for modders only |
| MLX / mlx-lm | 0.32.4 / 0.32.0 (Oct 2026) [PyPI] | MIT | macOS only | Yes (C API via mlx-c) | Ollama's switch to MLX by default suggests a speed edge on Apple chips [25] (inference) |
| ONNX Runtime GenAI | 0.18.0, 2026-10-14 [PyPI] | MIT [27] | Win/macOS/Linux | Yes | EPs: CPU, CUDA, DirectML, TensorRT-RTX, OpenVINO, QNN, WebGPU. Constrained decoding (llguidance integration [46]) [27]. Needs ONNX conversion. The `-directml` package stalled at 0.14.1 (Jun 2026) while `-winml` is at 0.18 [PyPI] |
| DirectML | maintenance mode | MIT | Windows | — | "No new functionality or feature updates are planned"; points to Windows ML [28] |
| Foundry Local | SDK 2.1.0 (Sep 2026) [PyPI] | SDK MIT; CLI under a Microsoft licence [29] | Win/macOS(AS)/Linux | Yes: in-process, ~20 MB [29] | **Curated catalog only**; "may collect usage data and send it to Microsoft" [29]. Explicitly single-user, not a server [29] |
| NVIDIA In-Game Inferencing (NVIGI) | 1.7.0 [30] | MIT, except MS DLLs and NVIDIA models [31] | **Windows only**; min RTX 30-series 8 GB, 12 GB recommended [30] | Yes | GPT plugin is **GGML/llama.cpp-based** with CUDA, D3D12 and Vulkan backends (D3D12/Vulkan "experimental"), a cloud REST backend, a VRAM-budget API, and CUDA-in-Graphics (CiG) on a D3D12 queue [31] |
| TensorRT-LLM | 1.2.1 (Apr 2026, sdist only) [PyPI] | Apache-2.0 | Data-centre Linux | No | Windows appears only in 2023–24 news [32]. The RTX path is now the TensorRT-RTX EP in ORT [27] |
| ExecuTorch | 1.5.1 (Sep 2026) [PyPI] | BSD-3 | incl. win_amd64 wheels [PyPI] | Yes | Mobile/edge orientation; poor fit (inference) |
| vLLM | 0.31.0 (2026-10-05) [PyPI] | Apache-2.0 | **Linux only**; WSL or community fork on Windows [33] | Server only | `gpu_memory_utilization` default 0.92 [34] |
| SGLang | 0.5.21 (Oct 2026) [PyPI] | Apache-2.0 | Linux/data-centre | Server only | Same verdict as vLLM |
| NobodyWho (Godot plugin) | 5.1.0, 2026-10-09 [38] | **EUPL-1.2**: usable in proprietary games; changes to the plugin itself must be open [37] | Godot 4.5+ desktop (Vulkan/Metal) [37] | Yes | llama.cpp-based; grammar-constrained tool calling. 5.1 stopped re-reading the whole conversation for hybrid models like Qwen3.5 [38]. Good reference or starting point |

**Verdict:** llama.cpp is the only option that is cross-platform, permissively licensed, embeddable and current, and that has native grammar support. NVIGI is a thin, MIT-licensed wrapper over the same engine. It is worth it only for a Windows/NVIDIA-specific D3D12/CiG optimisation path later.

### 3. Shipped games and mods with LLM NPCs

| Title | Inference | What is known | Lesson |
|---|---|---|---|
| **Mantella** (Skyrim/FO4 mod) [V] | Cloud by default (OpenRouter free Gemma 4 26B-A4B); local optional via koboldcpp/Ollama/LM Studio [39] | Targets <0.5 s LLM latency, then STT and TTS tricks such as "fast response mode" and lazy lip-sync (≈0.5 s each) [40]. Per-NPC conversation summaries act as memory [39]. Its own 8B fine-tune is deprecated [41] | Even the most mature mod defaults to cloud; local needs "a high-end GPU" [40] |
| inZOI "Smart Zoi" (Krafton + NVIDIA ACE) | Local, small ACE model | Opt-in, behaviour/thought flavour rather than open dialogue | [U] |
| PUBG Ally; Mecha BREAK ACE demo | Local SLM demos (NVIDIA ACE) | Announced in 2024–25; shipping status unclear | [U] |
| CHIM/Herika (Skyrim) | Cloud or local via a heavy server stack | Enthusiast-grade setup | [U] |
| Suck Up!, Vaudeville, 1001 Nights, Whispers from the Star | Cloud | Persuasion/detective/storytelling loops. Mixed reviews often cite latency and off-script answers | [U] |
| Dead Meat (Meaning Machine) | Reportedly on-device | Interrogation game | [U] |
| Where Winds Meet (NetEase) | Cloud | Reports of players manipulating AI NPCs | [U] (prompt-injection lesson) |

**Inference (low confidence, given verification gaps):** almost every commercial title with open-ended NPC dialogue uses cloud inference. On-device use has been limited to vendor-sponsored, narrow-role integrations with small models. Rehoboam's "local by default" plan is ambitious, not proven.

### 4. Performance

**llama-bench, Llama-2-7B Q4_0 (3.56 GiB), decode tg128 / prefill pp512, tok/s** [42][43][44]:

| GPU | CUDA tg / pp | Vulkan tg / pp |
|---|---|---|
| RTX 3060 12 GB | 76 / 2,138 | 76 / 1,816 |
| RTX 4060 Ti | 64 / 3,395 | — |
| RTX 4070 | — | 92 / 3,179 |
| RTX 4070 Ti SUPER | 132 / 6,925 | — |
| RTX 4090 | 186 / 11,993 | 188 / 9,452 |
| RTX 5060 Ti / 5070 / 5090 | 91 / 128 / 290 | — |
| RX 7900 XTX / RX 9070 XT | — | 183 / 137 |
| Arc B580 | — | 70 / 621 |
| Ryzen AI 300 iGPU / Ryzen AI Max+ | — | 22 / 52 |

| Apple (Metal, same model) | tg / pp |
|---|---|
| M1 / M2 / M3 / M4 (base) | 14 / 22 / 21 / 24 tg; 108–221 pp |
| M4 Pro / M4 Max | 50 / 70–83 tg |
| M5 Max (newer build, not directly comparable) | 120 tg / 3,220 pp |

**Scaling to a 4B Q4 model (inference).** Decode is memory-bandwidth-bound. A ~2.4 GB 4B model should decode roughly 1.4–1.6× faster than this 7B: about 100 tok/s on an RTX 3060, 250+ on a 4090, ~30 on base M-series, and 10–25 on a dual-channel DDR5 CPU (theoretical ceiling ≈ bandwidth / model size). That is with the GPU *not* rendering. These numbers have not been measured.

**TTFT is the real problem.** A 1,500-token persona/world prefix on a 7B model prefills in about 0.7 s on an RTX 3060, but in about **10 s on an M2 base** (146 pp t/s) and longer on CPU. **Prefix caching is mandatory.** Keep a stable system prefix (world rules, then NPC persona, then belief snapshot) and append only the new turn. llama.cpp provides `--cache-reuse`, per-slot caches and `--slot-save-path` to persist an NPC's KV state to disk [22]. Caveat: hybrid/recurrent (Qwen3.5) and SWA (Gemma) models cannot reuse arbitrary prefixes. They rely on **context checkpoints**, and by default llama.cpp creates up to 32 per slot with a minimum spacing of 8,192 tokens [22]. That spacing is larger than a typical persona prompt, so tune `-cms` and measure. NobodyWho only fixed full re-reads for Qwen3.5 on 2026-10-09 [38], which shows how new this is.

**KV memory.** Per token: 2 × layers × KV-heads × head-dim × bytes. A 36-layer, 8-KV-head, 128-dim layout costs ~144 KiB/token at f16, so 4K context ≈ 0.56 GiB. q8_0 KV (`-ctk/-ctv`) halves that [22]. Hybrid models store far less (inference).

**VRAM budgeting with Godot (inference).** Plan for one resident model only. Swapping models costs seconds and thrashes VRAM.

| GPU VRAM | Game budget | LLM budget | Fits |
|---|---|---|---|
| 8 GB | ≤4.5 GB | ~3 GB | 4B Q4_K_M (~2.5 GB) + 4–8K ctx |
| 12 GB | ≤6 GB | ~5.5 GB | 4B with headroom, or 9B Q4 (~5.5 GB) with a tight ctx |
| 16 GB | ≤7 GB | ~8 GB | 9B Q4 + 2–4 slots |
| 24–32 GB | ≤8 GB | 16–22 GB | 26B-A4B / 35B-A3B Q4 (~16–20 GB) |

**GPU contention.** Decode and rendering compete for memory bandwidth and SMs, so expect frame-time spikes. No public measurements were found. Mitigations: give the LLM context a lower CUDA/Vulkan queue priority where available; NVIGI's CiG routes CUDA work through the game's D3D12 queue [31]. Cap generation rate. Schedule background generation for pauses, menus and time-skips. Run background jobs on CPU or iGPU when the GPU is saturated. **This needs an in-house benchmark early.**

**Concurrency.** "50+ concurrent" is not a requirement. Size for **1 foreground slot plus 2–4 background slots** (`-np`, continuous batching [22]). Batching a handful of streams raises aggregate throughput at little per-step cost, because decode is bandwidth-bound (inference). Ambient barks should be templated or pre-generated in batches during idle time, not generated live per pedestrian.

**Speculative decoding** is supported via draft models [22]. For 3–4B targets the gains are small and the draft costs VRAM. Consider it only on the 24 GB tier for 20–35B models (inference).

### 5. Structured generation

| Engine | Version | License | Status / facts |
|---|---|---|---|
| **llguidance** | 1.9.1, 2026-09-30 [PyPI] | MIT | ~50 µs CPU per token mask with a 128K vocabulary, no significant startup cost. In llama.cpp (since b4613), vLLM, SGLang, ORT-GenAI, Chromium, and powers OpenAI's JSON-schema Structured Outputs [46] |
| **XGrammar** | 0.2.8, 2026-09-24 [PyPI]; XGrammar-2 announced 2026-05 [45] | Apache-2.0 | Default backend in vLLM/SGLang/TensorRT-LLM/MLC [45]. vLLM's backend choices are now xgrammar/guidance (plus outlines and lm-format-enforcer for regex) [49] |
| **Outlines** | 1.3.3, 2026-08-06; outlines-core 0.2.14 (Jan 2026) [PyPI] | Apache-2.0 | Python front-end that delegates to transformers/llama.cpp locally and to vLLM/Ollama/OpenAI/Gemini remotely [47]. With Ollama, JSON schema only [48]. dottxt now also sells "enterprise-grade libraries" [47]. **Not shippable in a C++/Godot runtime** |
| **llama.cpp GBNF / json_schema** | built-in | MIT | `additionalProperties` defaults to false. `true` "may produce keys that contain unescaped newlines". **Unsupported features are skipped silently** (`uniqueItems`, `patternProperties`, remote `$ref`) [23]. `x? x? x?…` repetition is pathologically slow; use `x{0,N}` [23] |

**Quality effects.** *Let Me Speak Freely?* (EMNLP 2024 Industry) found that format restrictions hurt reasoning and help classification [50]. dottxt's *Say What You Mean* rebutted it, and the authors' update argues the finding stands for reasoning-plus-format tasks [50][51]. **CRANE** (ICML 2025) shows that strict grammars can reduce reasoning ability, and recovers it by alternating unconstrained reasoning with constrained output [52]. **JSONSchemaBench** found engines differ widely in schema coverage and compile behaviour [53]. **Practical rule:** reason first (a thinking block or a short free-text field), then emit a tightly constrained object. Constrain *selection* (enums), not *creativity*.

**Schema design for Rehoboam**
- Generate `enum`s from the live registries (culture IDs, legal actions, belief IDs) at request time. Never let the model invent IDs.
- Use discriminated unions (`kind` const + branch). Avoid `Dict[str, Any]` / `additionalProperties: true` [23]. Keep optional fields few, because Anthropic caps total optional parameters at 24 and union-typed parameters at 16 per request [54].
- Put numeric ranges in the validator, not the schema. Anthropic does not support `minimum`/`maxLength` in strict schemas [54], and llama.cpp support is partial [23].
- Keep the LLM output *small* (intent, parameters, names, prose seeds). A deterministic generator produces the bulk.
- **Syntactic vs semantic validity.** A grammar ensures the JSON parses and matches the schema subset. Pydantic plus domain validators then check cross-field invariants (population sums, succession rules referencing existing houses, dragon ecology vs biome). On failure, send a **repair** prompt with machine-readable errors, at most 2–3 rounds, then fall back to defaults or ask the user.
- Always set `max_tokens` with headroom and treat `stop_reason ∈ {max_tokens, refusal}` as failure [54].

**Frontier structured outputs.** Anthropic: `output_config.format` / `strict: true` tools, grammar-compiled and cached for 24 h, with the limits above. Enum *casing* can drift, so compare case-insensitively [54]. OpenAI: JSON-schema Structured Outputs powered by llguidance [46]. Google/DeepSeek/Qwen: the LiteLLM table flags `supports_response_schema` for current models [58]; behaviour not verified.

### 6. Knowledge-bounded role-play

| Work | Year | Finding relevant to Rehoboam |
|---|---|---|
| TimeChara [60] | ACL Findings 2024 | Point-in-time hallucination: o1 81.8%, GPT-4o 64.5% (46% on "future"), Mistral-7B 46.8%. Reasoning models help; small models leak |
| RoleLLM / RoleBench [61] | 2023 | Instruction-tuned role-play dataset and benchmark |
| CharacterEval [62] | 2024 | 13 metrics; a trained reward model correlates with humans better than GPT-4 does |
| CharacterBench [63] | 2024–25 | Character customisation benchmark with a dedicated judge model |
| Character-LLM [64] | EMNLP 2023 | "Protective scenes" training to refuse out-of-character knowledge, i.e. refusal-to-know data |
| CoSER / HER [19][65] | 2025 / 2026 | Book-derived persona data; reasoning + RL improves role-play |
| Version-specific portrayal [66] | 2025 | Benchmarks mostly ignore persona evolution over time [S] |
| Persona drift (Li et al., COLM 2024, "Measuring and Controlling Instruction (In)Stability…") | 2024 | Persona adherence degrades within ~8 turns [U] |

**How well do 3–8B models stay in character? (inference from the above).** They are fluent in voice and style. They are unreliable at: (a) withholding facts that are in context, (b) not inventing events, (c) sustaining a lie consistently across turns, and (d) staying out of "assistant mode" under player pressure. Mantella's experience points the same way [41].

**Techniques, ranked by value for Rehoboam:**
1. **Context = belief DB only.** Retrieve the NPC's own beliefs (with provenance, confidence and timestamp) and never world truth. In a procedural world this removes most leakage at the source (inference).
2. **Plan-then-realise.** First a constrained JSON *speech act*, e.g. `{act: reveal|conceal|lie|deflect|gossip, belief_ids:[enum], stance, emotion}`. The simulation can veto it or decide it: the covert-infection concealment is a sim decision. Then a short free-text line conditioned on that plan. Gossip propagation uses the plan, not the prose.
3. **Post-hoc claim check.** Have the model emit or extract asserted claims mapped to belief IDs. Reject or regenerate if a line asserts an unknown fact or mentions a concealed topic, using keyword plus embedding similarity against secret beliefs (inference).
4. **System-prompt patterns.** An explicit "you do not know X; say so in character" rule, plus in-character refusal exemplars (Character-LLM's protective scenes [64]). Keep it short and cached.
5. **LoRA per archetype.** Defer until the base model is frozen, and only for voice/style.

### 7. Frontier APIs for offline batch jobs (prices per Mtok, input/output)

| Provider / model | Price | Context / max out | Batch | Cache | Source |
|---|---|---|---|---|---|
| Anthropic Fable 5.1 | $10 / $50 | 1M / 128K (300K batch beta) | 50% → $5/$25 | read $0.25; writes 1.25× (5 min) / 2× (1 h) | [55][56] |
| Anthropic Opus 5.5 | $4 / $20 | 1M / 128K | $2 / $10 | read $0.20 | [55] |
| Anthropic Sonnet 5.5 | $2 / $10 | 1M / 128K | $1 / $5 | read $0.10 | [55] |
| Anthropic Haiku 5.5 | $0.10 / $0.50 (≤100K prompt; $0.50/$2.50 above) | 1M / 128K | $0.05 / $0.25 | read $0.01 | [55] |
| OpenAI GPT-6 astra / sol (and 6.1-sol) / luna | $10/$50; $2/$10; $0.10/$0.50 | 922K / 128K | 50% | yes | [58] (LiteLLM table; OpenAI site unreachable) |
| Google Gemini 4 Argon | $4/$20 from 2026-12-01 (promo $2/$10 via 50% credit-back to 11-30) | 1M / 65,536 | $2/$10 (promo $1/$5) | $0.20 | [57][58] |
| Google Gemini 3.8 Flash; 3.5 Flash-Lite | $0.75/$3.75 intro to 2026-12-31, then $1.50/$7.50; $0.30/$2.50 | 1M / 65,536 | yes | yes | [57] |
| DeepSeek V4 Pro / V4 Flash | $1.32/$3.96; $0.30/$1.20 | 1M / 393K | — | yes | [58] |
| Qwen3.8-Max / 3.8-Flash (DashScope) | $2/$6; $0.15/$0.47 | ~1M / 131K | — | yes | [58] |
| Others | Kimi K3 $3/$15; GLM-5.3 $1.40/$4.40; Grok 4.7 $2/$6; Mistral Medium 3.5 $1.50/$7.50; Mistral Small 4 $0.15/$0.60 | | | | [58] |

**Gateways.** The LiteLLM SDK is MIT, very active (1.105.1, 2026-10-14) [PyPI], and its price table matched Anthropic's official page exactly [55][58]. But the *proxy* had multiple Critical/High advisories between May and Oct 2026 [59], and versions 1.82.7/1.82.8 are absent from PyPI, consistent with a reported March 2026 supply-chain compromise ([V] absence, [U] cause). Use the SDK pinned with hashes, or write a ~300-line adapter. OpenRouter: convenient, with `:free` variants used by mods [39]. Free-tier rate limits, data-use terms and longevity are [U]. Do not depend on them.

---

## Recommendations for Rehoboam

**1. Runtime.** Ship **llama.cpp `llama-server` as a managed sidecar process** on a random localhost port. Use pinned build tags, Vulkan as the default backend on Windows/Linux, Metal on macOS, and an optional CUDA pack. Reasons: crash and OOM isolation from Godot; slots, cache and grammar support for free; MIT licence; the same binary on all OSes [21][22]. Start the Godot side with NobodyWho [37] or a thin GDExtension HTTP client, and move to in-process libllama only if IPC overhead is measurable. Keep NVIGI (Windows/NVIDIA, CiG) as a later optimisation path [31]. Do not use Ollama, LM Studio, vLLM, SGLang or TensorRT-LLM in the shipped product.

**2. One resident model, many roles.** Load one model per session, matched to the hardware tier. Use slot 0 for dialogue (non-thinking) and slots 1–3 for background work (thinking enabled per request). Run hardware detection plus a 10-second self-benchmark on first launch to choose the tier.

**3. Hardware tiers**

| Tier | Resident model (Q4_K_M) | Notes |
|---|---|---|
| No GPU (CPU, 16 GB RAM) | Qwen3.5-2B or Gemma 4 E2B | Short prompts, templated barks, optional opt-in cloud dialogue (Haiku 5.5 / GPT-6-luna / Gemini Flash-Lite) |
| 8 GB | **Qwen3.5-4B or Gemma 4 E4B** (choose by in-house eval) | 4–8K ctx, q8_0 KV |
| 12–16 GB | Qwen3.5-9B (or Ministral 3 8B) | 2–4 background slots |
| 24 GB+ | Gemma 4 26B-A4B or Qwen3.6-35B-A3B | Few active parameters, so fast. 27B dense only for offline work |
| Apple Silicon | 16 GB: 4B; 24–32 GB: 9B; ≥48 GB: 26B-A4B / 35B-A3B | Metal via llama.cpp; MLX later [25] |

**4. Per-job model map**

| Job | Local default | Cloud (opt-in/offline) | Constraint method | Budget |
|---|---|---|---|---|
| World compile | Resident model (thinking) → *intent* JSON → deterministic expander | Sonnet 5.5 or Gemini 3.8 Flash with native structured outputs | llguidance/GBNF schema + Pydantic + semantic validators + ≤3 repairs | 30–120 s; < $0.10 cloud |
| NPC dialogue | Resident model, non-thinking, cached persona | Haiku 5.5 / GPT-6-luna (adds network latency) | Constrained speech-act plan (~30 tokens) → free-text line → claim check | TTFT < 0.5 s; first sentence < 1 s, streamed to TTS |
| Crisis deliberation (named characters) | Background slot, thinking on | Sonnet 5.5 batch in "chronicle mode" | Reason freely, then enum of legal actions from the registry + ≤40-word rationale [52] | async; cap ~10–20 calls per simulated year on long runs |
| Persuasion judge | Separate call; the player's text is quoted as data | Haiku/Sonnet 5.5 | Rubric features (appeals to value X, cites belief Y, threat/bribe) as enums and scores. **The simulation computes the outcome** | < 2 s |
| Novel / screenplay / audio drama | — (27B on 24 GB+ as an offline fallback) | Opus 5.5 or Fable 5.1 via Batch; Gemini 4 Argon or GPT-6-astra as alternates | Outline JSON → per-chapter prose (unconstrained), with cached event-graph context | ~180K output tokens per 100K words (≈1.8 tokens/word on the current Claude tokenizer [56]); about $3 (Sonnet 5.5) to $15 (Fable 5.1) per novel in batch |
| Summarisation / chronicles | Background slot | Haiku 5.5 batch ($0.05/$0.25) | JSON summary schema | async |

**5. API strategy.** Use a provider-agnostic adapter with a bring-your-own-key option. Defaults: Anthropic Batch for the synthesiser, and prompt caching for repeated event-graph context (note that batch cache hits are best-effort). Every cloud job needs a local fallback, and the game must stay fully playable offline. Do not use free tiers in any critical path.

**6. Evals before choices.** Build a 200-scenario Rehoboam role-play eval covering concealment, lies, gossip, unknown-fact refusal, multi-turn drift and player jailbreaks. Score it automatically against the belief DB. Pick between Qwen3.5-4B and Gemma 4 E4B with that eval, not with leaderboards.

---

## Risks & unknowns

- **Verification gaps.** Model licences for Qwen3.5/3.6/3.8 and Gemma 4 rest on secondary reports [S]. Read each model's `LICENSE` on HF before shipping. Every claim about shipped games is [U].
- **Licence drift.** Qwen ships licences per model [1]; Gemma moved to Apache only with Gemma 4 [5]; LFM has a $10M revenue cap [12]; NVIDIA licences vary by model [13]; Llama requires attribution [14].
- **Hybrid/SWA caching maturity.** Context checkpoints are new [22][38]. Persona-prefix reuse may underperform on Qwen3.5/Gemma 4. Benchmark TTFT per model.
- **GPU contention.** Unmeasured. It could force dialogue onto smaller models or CPU while rendering. Vulkan performance differs across vendors (e.g., Arc B580 prefill at 621 tok/s [43]).
- **Download size.** 1.5–6 GB of weights per tier affects storefront depots and patching.
- **Safety and ratings.** Local models will say anything. Player injection against the judge and NPCs is a real attack surface. Platform AI-content disclosure rules apply ([U]).
- **Cloud volatility.** Gemini promo pricing ends on 2026-11-30 / 12-31 [57]. Models get deprecated. Refusal stop-reasons occur with violent fiction (Anthropic documents refusals even under structured outputs [54]).
- **Gateway supply chain.** LiteLLM advisories [59].
- **Benchmark unreliability.** Role-play leaderboards are judge-based and noisy [17][18].
- **Long-run budget.** LLM calls must scale with named characters, not with population.

---

## Sources

1. https://github.com/QwenLM/Qwen3.6 — Qwen README: release dates for Qwen3.5/3.6/3.8, Gated-DeltaNet hybrid architecture, per-model licence note [V]
2. https://artificialanalysis.ai/articles/qwen3-5-small-models — Qwen3.5 small-model index scores, Apache-2.0, 262K context [S]
3. https://www.eweek.com/news/alibaba-qwen3-8-27b-license-apac-china/ — Qwen3.8-27B under Apache-2.0, 262K context [S]
4. https://www.deepmind.google/models/gemma/gemma-4 — Gemma 4 lineup and τ2-bench scores [S]
5. https://the-decoder.com/googles-gemma-4-is-now-available-with-apache-2-0-licensing-for-the-first-time/ — Gemma 4 Apache-2.0 vs Gemma 3 Terms [S]
6. https://github.com/google-deepmind/gemma — Gemma library exposes Gemma4_E4B; links the model card [V]
7. https://github.com/openai/gpt-oss — gpt-oss sizes, Apache-2.0, MXFP4 "within 16GB", harmony format [V]
8. https://mistral.ai/news/mistral-small-4/ — Mistral Small 4 = 119B-A6B, Apache-2.0 [S]
9. https://recipes.vllm.ai/mistralai/Ministral-3-14B-Instruct-2512 — Ministral 3 family details [S]
10. https://github.com/huggingface/smollm — SmolLM3-3B claims and context length [V]
11. https://github.com/ibm-granite/granite-4.0-language-models — Granite 4.0 sizes, hybrid variants, Apache-2.0 [V]
12. https://www.liquid.ai/lfm-license — LFM Open License with $10M revenue threshold [S]
13. https://www.nvidia.com/en-us/ai-data-science/foundation-models/nemotron/llm-info/ — Nemotron licence changes (OpenMDW-1.1 for 3.5) [S]
14. https://github.com/meta-llama/llama-models — Llama release table (latest Llama 4, Apr 2025); Llama 3.2 licence clauses [V]
15. https://codersera.com/blog/meta-open-weights-return-2026/ — reported Muse Glimmer 30B Apache-2.0 release [S]
16. https://github.com/microsoft/PhiCookBook — Phi line still at Phi-4 family [V]
17. https://github.com/LeviTheWeasel/rp-benchmark — community RP benchmark; judge/user disagreement caveat [S]
18. https://www.benchlm.ai/best/roleplay — admits no defensible role-play score [S]
19. https://github.com/Neph0s/CoSER — CoSER datasets/models; HER (2026) news [V]
20. https://github.com/ggml-org/llama.cpp — MIT licence, backends, server, GBNF [V]
21. https://github.com/ggml-org/llama.cpp/releases — build b11669 (2026-10-15) and per-OS/backend binaries [V]
22. https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md — slots, continuous batching, cache-reuse, slot save, context checkpoints, KV types, draft models [V]
23. https://github.com/ggml-org/llama.cpp/blob/master/grammars/README.md — GBNF; JSON-schema limitations; silent skipping [V]
24. https://gihyo.jp/article/2026/02/ggml-join-hugging-face — ggml.ai joins Hugging Face (2026-02-20) [S]
25. https://github.com/ollama/ollama/releases/tag/v0.40.0 — Ollama MLX-by-default on Apple Silicon; gemma4/qwen3.5/3.6 [V]
26. https://github.com/lmstudio-ai/lmstudio-js — LM Studio SDK is a client of the LM Studio app; MIT [V]
27. https://github.com/microsoft/onnxruntime-genai — ORT GenAI EPs, platforms, constrained decoding [V]
28. https://github.com/microsoft/DirectML — DirectML maintenance mode notice [V]
29. https://github.com/microsoft/Foundry-Local — in-process SDK, curated catalog, telemetry, licences [V]
30. https://github.com/NVIDIA-RTX/NVIGI-Core — NVIGI 1.7.0, Windows requirements, hardware guidance [V]
31. https://github.com/NVIDIA-RTX/NVIGI-Plugins/blob/main/docs/ProgrammingGuideGPT.md — GGML-based GPT plugin, CUDA/D3D12/Vulkan/cloud, VRAM budget, CiG [V]
32. https://github.com/NVIDIA/TensorRT-LLM — Windows mentioned only in 2023–24 news [V]
33. https://github.com/vllm-project/vllm/blob/main/docs/getting_started/installation/gpu.md — "does not support Windows natively" [V]
34. https://github.com/vllm-project/vllm/blob/main/vllm/config/cache.py — gpu_memory_utilization default 0.92 [V]
35. https://pypi.org/project/vllm/ — PyPI metadata used for versions/dates of vllm, sglang, onnxruntime-genai(-directml/-winml), executorch, tensorrt-llm, mlx, lmstudio, foundry-local-sdk, litellm [V]
36. https://pypi.org/project/outlines/ — Outlines 1.3.3 (2026-08-06); outlines-core, xgrammar, llguidance versions via the same API [V]
37. https://github.com/nobodywho-ooo/nobodywho — Godot llama.cpp plugin, EUPL-1.2 terms, platforms [V]
38. https://github.com/nobodywho-ooo/nobodywho/releases — v5.1.0 (2026-10-09) fixes for hybrid-model re-reads [V]
39. https://github.com/art-from-the-machine/Mantella/blob/gh-pages/docs/pages/installation.md — Mantella defaults (OpenRouter free Gemma 4 26B-A4B), local options [V]
40. https://github.com/art-from-the-machine/Mantella/blob/gh-pages/docs/pages/real_time_npcs.md — <0.5 s LLM target and latency tricks [V]
41. https://github.com/art-from-the-machine/Mantella/blob/gh-pages/docs/pages/fine_tuned_models.md — own fine-tune deprecated as outdated [V]
42. https://github.com/ggml-org/llama.cpp/discussions/15013 — CUDA llama-bench scoreboard [V]
43. https://github.com/ggml-org/llama.cpp/discussions/10879 — Vulkan llama-bench scoreboard [V]
44. https://github.com/ggml-org/llama.cpp/discussions/4167 — Apple Silicon llama-bench scoreboard [V]
45. https://github.com/mlc-ai/xgrammar — XGrammar integrations, XGrammar-2 news [V]
46. https://github.com/guidance-ai/llguidance — ~50 µs/token masks; integrations incl. OpenAI, llama.cpp [V]
47. https://github.com/dottxt-ai/outlines — Outlines claims, backends, dottxt enterprise libraries [V]
48. https://github.com/dottxt-ai/outlines/blob/main/docs/features/models/ollama.md — Ollama supports JSON schema only [V]
49. https://github.com/vllm-project/vllm/blob/main/docs/features/structured_outputs.md — vLLM structured-output backends [V]
50. https://github.com/appier-research/structure-gen — *Let Me Speak Freely?* (arXiv 2408.02442) and the authors' update/rebuttal [V]
51. https://blog.dottxt.co/say-what-you-mean.html — dottxt rebuttal (linked from [50]) [V link]
52. https://github.com/uiuc-focal-lab/CRANE — CRANE (ICML 2025, arXiv 2502.09061) [V]
53. https://github.com/guidance-ai/jsonschemabench — JSONSchemaBench (arXiv 2501.10868) and MaskBench [V]
54. https://platform.claude.com/docs/en/build-with-claude/structured-outputs — constrained decoding, limits, failure modes [V]
55. https://platform.claude.com/docs/en/about-claude/pricing — Claude prices, batch and cache multipliers [V]
56. https://platform.claude.com/docs/en/about-claude/models/overview — context, max output, 300K batch output, tokenizer words/token [V]
57. https://cloud.google.com/vertex-ai/generative-ai/pricing — Gemini 4 Argon and 3.x Flash pricing and promos [V]
58. https://github.com/BerriAI/litellm/blob/main/model_prices_and_context_window.json — cross-provider prices, contexts, output caps (secondary for non-Anthropic) [V data / S pricing]
59. https://github.com/BerriAI/litellm/security — 2026 LiteLLM security advisories [V]
60. https://github.com/ahnjaewoo/timechara — TimeChara (arXiv 2405.18027) leaderboard [V]
61. https://github.com/InteractiveNLP-Team/RoleLLM-public — RoleLLM/RoleBench (arXiv 2310.00746) [V]
62. https://github.com/morecry/CharacterEval — CharacterEval (arXiv 2401.01275) [V]
63. https://github.com/thu-coai/CharacterBench — CharacterBench (arXiv 2412.11912) [V]
64. https://github.com/choosewhatulike/trainable-agents — Character-LLM, protective scenes (arXiv 2310.10158) [V]
65. https://arxiv.org/abs/2601.21459 — HER: reasoning + RL for role-play (linked from [19]) [V link]
66. https://www.arxiv.org/pdf/2510.14351 — version-specific character portrayal benchmark gap [S]
67. https://github.com/NVIDIA/ACE — ACE component list (Nemotron-3 4.5B SLM early access) [V]
68. https://github.com/SystemPanic/vllm-windows — community Windows fork referenced by vLLM docs [V link]
