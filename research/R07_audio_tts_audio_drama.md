# R07: Procedural Audio, TTS, ASR and Audio-Drama Production for Rehoboam

*Research date: 2026-10-15. Status: dossier for architecture review.*

**Method and evidence limits (read first).** From this sandbox, only GitHub (github.com and raw.githubusercontent.com) could be fetched. Hugging Face, vendor sites, arXiv, Wikipedia, GDC Vault, standards bodies and news sites all failed DNS or were blocked by the proxy. The web-search quota also ran out partway through. So every source is tagged:
- **[V]**: I read it this session, usually the primary repo, README, LICENSE or API spec.
- **[S]**: secondary, taken only from search-result snippets.
- **[K]**: background knowledge with a canonical URL I could not open. Re-verify before relying on it.

One source carries a lot of the license work. The audio.cpp maintainers keep a per-model license table that they re-checked against the original model cards between 21 Sep and 14 Oct 2026 [20]. I use it heavily, so it is a single point of failure. Spot-check the model cards for any model we actually adopt.

---

## TL;DR

- **Diorama ambience: use Godot's native audio. Do not run neural audio at runtime.** Build parameter-driven layered beds, density beds and rate-limited spatialized one-shots. Godot already has buses, sidechain compression, AudioStreamInteractive and AudioStreamRandomizer [2][3][4]. Do procedural DSP (wind, rain, water) in C++ through GDExtension, not GDScript [1]. Verdict: **GO, low risk.**
- **The real cost of always-on wallpaper audio is power, not CPU.** Godot keeps the audio device open, and that can stop the machine sleeping. The macOS fix was merged for 4.8 on 2026-08-19. The Windows (WASAPI) fix is still an open PR [6][7][8]. The diorama has to *stop* its streams, not just mute them.
- **FMOD and Wwise are optional. Don't adopt them by default.** Both Godot integrations are maintained, but FMOD's lags: its newest release targets Godot 4.5 while Godot is at 4.7 [9][10][18]. Both are proprietary, with revenue or budget caps on their free tiers [13][14][K].
- **Real-time NPC voices: use a small model on the CPU, with a bank of designed voices.** The best fits are Chatterbox‑Nano/Turbo (MIT, 110M/350M) [34] and Kyutai Pocket TTS (100M, about 200 ms to first audio, 2 CPU cores; weights CC‑BY‑4.0) [46][20]. Kokoro‑82M (Apache‑2.0) is good but its repo has been dormant since Aug 2025 [31].
- **License trap: espeak‑ng is GPL‑3.0.** sherpa‑onnx's TTS build pulls in espeak‑ng, and so do Kokoro's OOV fallback and Piper [62][33][51]. Do not link any of these statically into a closed-source game binary without legal review.
- **Offline audio drama: render line by line, then verify every take with ASR.** Candidates: Qwen3‑TTS (Apache‑2.0, includes text-prompted VoiceDesign) [49], IndexTTS‑2/2.5 (free commercial use under 100M MAU or RMB 1B revenue) [42], MOSS‑TTSD v1.0 (Apache‑2.0, 1–5 speakers, up to 60 min) [44]. Per the tool's maintainers, open models "are unreliable at book scale" without automatic re-takes [61].
- **Many "open" leaders are non-commercial.** That includes Fish S2‑Pro, Higgs Audio v3, Voxtral TTS, Breeze TTS 2, F5‑TTS, Echo‑TTS and OmniVoice, which switched to CC‑BY‑NC on 2026‑07‑03 [20][43][39][47]. XTTS‑v2 is non-commercial (CPML) and the original Coqui repo is unmaintained [48]. Fish Audio Drama 3 allows commercial use only below US$500k revenue and requires "Built with Fish Audio" [43].
- **VibeVoice is not a production option.** Microsoft removed the long-form TTS code on 2025‑09‑05. Only Realtime‑0.5B remains, with fixed voices, English only, and labelled "research and development purposes only" [41].
- **Generative SFX and music belong in the offline asset pipeline only.** Commercial-safe options: Stable Audio Open / Stable Audio 3 small‑sfx (free under US$1M revenue, attribution required) [20], MOSS‑SoundEffect v2 (Apache‑2.0) [24], ACE‑Step 1.5 (MIT, under 4 GB VRAM) [25] and ElevenLabs SFX v2 on a paid plan [28]. Exclude AudioCraft, MMAudio, TangoFlux and YuE2, which are non-commercial [21][22][23][26].
- **Phone delivery works:** MP3 with ID3 CHAP chapters, a Podcasting 2.0 JSON chapters file and a transcript, served from a private RSS feed [68][69]. The hard part is making the feed reachable from the phone (LAN, a synced folder, or a host the user owns), not the format.
- **Ethics: designed voices only, watermarking and disclosure.** OpenAI and Google now require a recorded consent statement before cloning a voice [57][58]. EU AI Act Art. 50 marking duties apply from 2 Aug 2026 [K][79].

---

## Draft claims checked

| Draft claim | Verdict | Evidence / correction |
|---|---|---|
| "Generative procedural audio synthesizing real-time soundscapes tied to telemetry" | **Partly true; reword** | Feasible as *procedural mixing and DSP* driven by aggregated telemetry. Neural audio generation takes seconds per clip and competes for the GPU with the game and the LLM (inference from the speeds in [25][22]). Reserve "generative" for offline asset creation. |
| "Base environmental loop + procedural one-shots triggered by sim events" | **Incomplete** | One loop sounds static. The system needs several layered beds with parameter-driven gain and filters, density beds for crowds and traffic, and voice limiting and virtualization. With 100k–1M agents, one-shots must come from *aggregated* events with caps and cooldowns, or sirens and shrieks will spam. |
| Wind velocity, water flow, cart wheels, thunder and transit rumble can be driven by telemetry | **True** | Standard RTPC-style mapping. Godot has no RTPC object, but bus/effect parameters can be set from script every frame, and the compressor supports a sidechain [3]. Thunder delay can be computed from strike distance (343 m/s). |
| Always-on wallpaper audio is cheap | **Misleading** | Mixing costs little CPU (inference: well under 2% of one core for about 40 voices). Power and sleep are the problem: Godot's audio driver can keep a PC awake. macOS fix merged for 4.8; WASAPI PR still open; a reviewer reports the NVIDIA audio driver holds a power request while any stream is open [6][7][8]. |
| Player can talk to any NPC in real time with low-latency TTS and many distinct voices | **Feasible with constraints** | Sub-300 ms first audio is claimed for CPU and small-GPU models (Pocket ~200 ms [46]; Qwen3‑TTS "as low as 97 ms" [49]; VibeVoice‑RT ~300 ms [41]). "Many distinct voices" requires a designed-voice bank plus per-NPC variation. No small model ships hundreds of voices. |
| Optional player voice input via ASR | **True** | MIT/Apache/CC‑BY streaming ASR exists: Moonshine streaming (MIT), Parakeet‑TDT‑0.6B‑v3 (CC‑BY‑4.0), Nemotron‑3.5‑ASR‑Streaming (OpenMDW, latency adjustable from 80 ms to 1 s), Voxtral Mini 4B Realtime (Apache‑2.0) [20][65][66]. Prefer push-to-talk. |
| Audio dramas "automatically routed through multi-voice TTS pipelines" | **Feasible, not turnkey** | Needs ASR-verified retries, speaker-consistency checks and loudness mastering [61]. Most top-quality open weights are non-commercial or capped [20][43]. |
| "Ready to download to your phone" | **True, with friction** | Formats are solved (ID3 CHAP [69], `podcast:chapters` [68]). Apple Podcasts needs a reachable URL [K]. AntennaPod can read local folders [K]. |
| Godot 4 as the engine | **Fine for audio** | Godot 4.7 shipped 2026‑06‑18 [18]. Native audio covers this design. Gap: no HRTF. The Godot Steam Audio plugin is alpha and "not really being maintained" [11]. |
| (Implicit) "open-source TTS = free for commercial use" | **False** | See the license tables below. F5‑TTS, Fish S2‑Pro, Higgs v3, Voxtral TTS, YuE2, AudioCraft, MMAudio and OmniVoice are all non-commercial [20][47][21][22]. |

---

## Landscape / state of the art

### 1. Systemic ambience techniques

| Technique | What it is | Runtime cost | Fit for Rehoboam |
|---|---|---|---|
| Layered beds plus parameter mapping (RTPC-style) | 6–12 looping layers, each with gain, LPF and pitch curves mapped from telemetry (wind, rain, river flow, time of day) | Tiny | **Core.** The cheapest way to reach high quality. |
| Density beds | 3–4 pre-mixed loops per density level (sparse / busy / packed crowd or traffic), crossfaded by local agent density | Tiny | **Core** for crowds, carts and transit. Never iterate over agents. |
| Emitter clustering | Bin agents into grid cells; spatialize the loudest K cells as virtual emitters | Low | **Core** for street-level 3D. |
| Event one-shots | Sim events become pooled, prioritized, distance-culled one-shots with cooldowns and per-category caps; randomized with AudioStreamRandomizer [4] | Low | **Core.** Add aggregation, e.g. "3 sirens max". |
| Granular synthesis | Re-synthesize grains from a recording to vary texture and density without audible loops | Low–medium (C++) | Good for rain, gravel, crowds. Implement in GDExtension. |
| Physically-informed procedural DSP (Farnell) | Noise sources, filters and modal models for wind, rain, fire, engines; see *Designing Sound* (MIT Press, 2010), built in Pure Data [15][K] | Low–medium | Use for **wind and rain**, where continuous parameter control pays off. libpd is BSD-licensed and embeddable [16]. Recorded material still sounds better for most other things. |
| Interactive or stem music | Clip transitions driven by sim mood | Tiny | AudioStreamInteractive [2]. |
| Runtime neural generation | Text-to-audio model generating beds on the fly | High (GPU, seconds of latency) | **No for v1.** At best, regenerate a background bed asynchronously. |

**Godot native audio vs middleware**

| Option | Version / date | License / cost | Maturity | Fit | Notes |
|---|---|---|---|---|---|
| Godot native | 4.7 (2026‑06‑18) [18] | MIT | Mature | **Recommended** | Unlimited buses and effects; compressor with sidechain [3]; AudioStreamPlayer3D with attenuation, Doppler and Area3D reverb routing [4]; AudioStreamInteractive [2]. AudioStreamGenerator is "best used from C# or … GDExtension" [1]. No HRTF and no designer tooling. |
| FMOD + fmod‑gdextension | Latest plugin release targets Godot 4.5 / FMOD 2.03.06 [9] | Plugin MIT; FMOD proprietary. Free indie tier has revenue and budget caps (historically under US$200k revenue and under US$600k budget) [13][K] | Mature engine, plugin lags Godot | Optional | Gives designers parameters, events and live tuning. Official Steam Audio plugin for HRTF and occlusion [12]. C# bindings not provided [9]. |
| Wwise + wwise‑godot‑integration | Wwise 2024.1 / 2025.1 / 2026.1 beta; Godot 4.3+ [10] | Proprietary; free tier for small budgets [14][K] | Active (community-run, not Audiokinetic) | Optional | Richest feature set: rooms, portals, RTPC, states. Integration is maintained by a third party [10]. |
| Steam Audio (direct) | 4.8.1 [12] | Apache‑2.0 (includes some proprietary libraries) [11] | Mature (Valve) | Later | The Godot plugin is alpha and under-maintained [11]. |
| OS TTS via `DisplayServer.tts_speak` | Godot core [5] | Free | Mature | Accessibility fallback only | Uses OS voices. SSML depends on the engine on Windows/Linux [5]. |

**Reference games (all [K]; I could not open the primary talks this session).**
- *No Man's Sky*: Paul Weir's procedural music system ("Pulse") rearranges stems recorded by 65daysofstatic, and creature vocals are procedural (GDC 2017, "The Sound of No Man's Sky") [17].
- *SimCity* (2013, GlassBox): Maxis presented simulation-driven music and ambience at GDC.
- *Cities: Skylines*: ambience changes with zoom level and zone density (observed behaviour; no primary source found).
- *RDR2*: widely praised dense systemic ambience (weather, wildlife), but there is little primary technical documentation.
- *Townscaper* and *Tiny Glade*: interaction-driven procedural one-shots. I found no primary audio write-ups for either.
- Lesson for us: every shipped example is **sample-based plus systemic mixing**. Pure synthesis is the minority.

**CPU and power.** The verified facts:
- Godot's audio keeps the OS audio device active and has prevented sleep (issue #28039).
- PR #117162 lets CoreAudio sleep after 3 s with no active playbacks. It merged 2026‑08‑19 for 4.8.
- The WASAPI equivalent (#122738) is still open.
- A reviewer saw the NVIDIA audio driver hold a power request while a stream is open [6][7][8].

Design implications:
1. Pause audio when a fullscreen app is focused, when other apps are playing audio, or when on battery.
2. When paused, *stop* all players. Muting is not enough.
3. Offer an "audio off" mode that starts the wallpaper with the Dummy audio driver [8].
4. Stream long beds as Ogg Vorbis. Keep one-shots resident (inference: about 50–150 MB).

### 2. Generative models for offline SFX, ambience and music assets

| Model | Version / date | Weights license | Commercial? | Fit | Notes |
|---|---|---|---|---|---|
| Stable Audio Open 1.0 / Small | 2024 / 2025 [K] | Stability AI Community License | **Conditional:** free under US$1M revenue [20][19] | SFX and ambience prototyping | SAO 1.0 was trained on CC-licensed Freesound and FMA audio [K]. Small is aimed at on-device use [K]. |
| Stable Audio 3 (small‑sfx, small‑music, medium) | 2026 [20] | Stability AI Community License | **Conditional:** under US$1M, register, display "Powered by Stability AI"; Gemma text-encoder terms also apply [20] | Promising | Details not verified (HF unreachable). |
| MOSS‑SoundEffect v2.0 | 2026‑05‑26 [24] | Apache‑2.0 (family-wide) [24] | **Yes** | **Good** | DiT plus flow matching, 48 kHz, up to 30 s [24]. Training-data provenance undisclosed. |
| ElevenLabs SFX (`eleven_text_to_sound_v2`) | Current [28] | Commercial on paid plans [K] | Yes (paid) | Good for one-off variations | Loopable output, 0.5–30 s [28]. Cloud only. |
| AudioCraft (AudioGen, MusicGen) | Weights CC‑BY‑NC‑4.0 [21] | — | **No** | Exclude | Code MIT, weights non-commercial [21]. |
| MMAudio | Weights CC‑BY‑NC‑4.0 [22] | — | **No** | Exclude | Video-to-audio, about 6 GB VRAM, 8 s clips [22]. |
| TangoFlux / Tango | "Non-commercial, research-only" [23] | — | **No** | Exclude | Stability Community License plus WavCaps academic-only data [23]. |
| ACE‑Step 1.5 (incl. XL 4B, 2026‑04‑02) | 2026 [25] | MIT [25][20] | **Yes** | **Best open music** | Under 4 GB VRAM; under 10 s per song on an RTX 3090; 10 s–10 min; LoRA from about 8 songs [25]. Training data undisclosed. |
| HeartMuLa‑oss‑3B | 2026 [20] | Apache‑2.0 [20] | Yes | Evaluate | Unverified beyond the license row. |
| DiffRhythm v1.2 | 2025‑05‑09 [27] | Apache‑2.0 [27] | Yes | OK | Minimum 8 GB VRAM with chunked decoding [27]. |
| YuE (v1) / YuE2 | v1 legacy; YuE2 2026 [26] | v1 Apache (legacy branch); **YuE2 non-commercial** [26][20] | v1 yes / YuE2 no | Skip | YuE2 claims to be competitive with Suno v5/v6 but needs 24 GB VRAM [26]. |
| Suno / Udio | — | ToS-governed | Risky | **Avoid for shipped assets** | Labels sued both in June 2024. Udio settled with UMG and WMG, and Suno with WMG, in late 2025, pivoting to licensed models with download restrictions; other claims (Sony, UMG vs Suno, GEMA) were still pending at my last knowledge [K][29]. |

**Copyright note.** The US Copyright Office's 2025 report says purely AI-generated material is not copyrightable [K][30]. Our generated assets may therefore be unprotectable, and we can't easily stop competitors reusing them.

**Real-time feasibility.** ACE‑Step's sub-10 s per song on a 3090 and SAO Small's on-device target show that clips can be produced *faster than real time* on a desktop GPU. The latency is still seconds, and it competes with rendering and the LLM. So: **offline only for v1**, plus an optional "regenerate background bed while idle" experiment.

### 3. TTS (Oct 2026)

**Leaderboards (weak evidence).**
- Artificial Analysis' open-weights snapshot [S][59]: Fish Audio Drama 3 Preview (1243) > Breeze TTS 2 (1221, non-commercial) > Fish S2 Pro (about 1118–1123) > Step Audio EditX > Voxtral TTS > Magpie‑Multilingual‑357M > Kokoro‑82M > Maya1 > OpenAudio S1 Mini > Higgs v3 > Chatterbox.
- On HF TTS Arena, a 10 Oct 2026 guide puts Chatterbox as the highest clearly open-weight model, at 28th of 39 [S][60].
- Takeaway: the best open-weight models still trail the best closed ones, and the leaders in arena quality are mostly non-commercial.

**Table A: on-device and real-time candidates (NPC speech)**

| Model | Version / date | Code / weights license | Size / speed (as claimed) | Voices / control | Verdict |
|---|---|---|---|---|---|
| **Chatterbox‑Nano / Turbo** | Nano Jul 2026; Turbo 2025–26 [34] | MIT / MIT [34][20] | Nano 110M, about 3× real time on 8 CPU cores. Turbo 350M (GPU), 1-step decoder [34] | Zero-shot cloning from a reference; tags like `[laugh]`; **built-in Perth watermark** [34] | **Top pick** (English only) |
| **Pocket TTS** (Kyutai) | Jan 2026; training code Aug 2026 [46] | MIT code / **CC‑BY‑4.0** weights (attribution) [46][20] | 100M; ~200 ms to first chunk; ~6× real time on 2 cores of an M4 [46] | Voice cloning plus pre-exported voice embeddings; 7 EU languages [46]. Cloning component reportedly gated [S]. Policy forbids cloning without consent [46] | **Top pick** (C++/ONNX/WASM ports exist [46]) |
| Kokoro‑82M | v1.0 (2025); repo last commit 2025‑08‑06 [31] | Apache‑2.0 / Apache‑2.0 [31][20] | 82M; CPU-friendly [31] | About 54 fixed voices [K]; weighted voice blending, e.g. `af_bella(2)+af_heart(1)` [32]; no cloning | Good, but **dormant**, with a GPL espeak fallback [33] |
| MOSS‑TTS‑Nano | 2026‑04‑13 [24] | Apache‑2.0 [20] | ~100M; streams on 4 CPU cores [24] | Multilingual cloning | Evaluate |
| Qwen3‑TTS 0.6B / 1.7B | 2026‑01‑22 [49] | Apache‑2.0 / Apache‑2.0 [49][20] | "As low as 97 ms" (claim); GPU [49] | 9 preset voices, instruction-driven emotion, 3 s cloning, **VoiceDesign** [49] | Strong if the GPU budget allows |
| NeuTTS Air / Nano / 2E | 2025–26 [50] | Air Apache‑2.0; Nano/2E "NeuTTS Open License" (commercial only under US$5M revenue) [50][20] | 120–360M active; GGUF; watermarked [50] | Cloning; 2E has 4 speakers × 6 emotions | OK |
| VibeVoice‑Realtime‑0.5B | 2025‑12‑03 [41] | MIT, but "research and development purposes only" [41] | ~300 ms first chunk; real time on a T4 [41] | Embedded fixed voices only; English [41] | **No** |
| Kitten TTS 0.8 / KittenTTS 2 | 2026 [52] | 0.8 Apache‑2.0; v2 conditional (Stellon Community License) [20] | 15–80M ONNX; v2 is a 1.7B 1-bit model that runs in real time on CPU [52] | 8 / 47 voices | Low-end fallback |
| Soprano‑1.1‑80M | 2026‑01‑14 [53] | Apache‑2.0 [20] | Claims 20× real time on CPU [53] | **One voice, no cloning** [53] | No (variety) |
| Supertonic 3 | archived [54] | OpenRAIL‑M [20] | 99M, 31 languages [54] | — | **Archived. Avoid.** |
| Piper (piper1‑gpl) | Active, Oct 2026 commits [51] | GPL‑3.0 code; voice licenses vary, e.g. Lessac voice is non-commercial [20][51] | Very fast CPU | Many single-speaker voices | **Avoid** for a closed commercial game |
| Orpheus 3B | Last release 2025 [37] | Apache‑2.0 repo; Llama-based weights [37] | ~200 ms streaming (claim) [37] | 8 voices plus emotive tags | Too heavy next to an LLM |
| Dia2 1B/2B | 2025 [36] | Apache‑2.0 code; weights license not stated [36] | Streaming; English; max ~2 min [36] | [S1]/[S2] tags; voices drift without a prefix [36] | Interesting for 2-person barks |

**Table B: offline, high-quality multi-voice candidates (audio drama)**

| Model | Version / date | Weights license | Commercial? | Strengths | Hardware |
|---|---|---|---|---|---|
| **Qwen3‑TTS 1.7B (Base / CustomVoice / VoiceDesign)** | 2026‑01‑22 [49] | Apache‑2.0 [20] | **Yes** | Text-prompted voice design, instruction-driven emotion, 10 languages; long-form WER about 2.8% EN [49] | 1.7B; FlashAttention2 recommended [49] |
| **IndexTTS‑2 / 2.5** | 2025‑09‑08 / 2026 [42] | bilibili Model Use License [42] | **Yes** under 100M MAU or RMB 1B revenue; outputs count as "Derivative Works"; downstream terms must be passed on [42] | 8-dim emotion vector, emotion reference audio, text-derived emotion; 2.5 adds 0.5–2× duration control [42] | Benchmarked on a 4090 [42] |
| **MOSS‑TTSD v1.0** | 2026‑02‑10 [44] | Apache‑2.0 [20] | **Yes** | 1–5 speakers, up to 60 min per session, 20 languages; beats Eleven v3 on speaker attribution and similarity in the authors' own eval [44] | Size not stated; SGLang [44] |
| Chatterbox Multilingual V3 | 2026‑06‑10 [34] | MIT [20] | **Yes** | 23 languages, "exaggeration" control, watermark [34] | 500M |
| Higgs Audio v2 | 2025 [39] | Boson Higgs Audio 2 Community License; expanded license required above 100k annual active users [S][40][20] | Conditional | Multi-speaker dialogue, auto voice assignment [39] | **24 GB** recommended [39] |
| Fish Audio Drama 3 | Oct 2026 (arXiv 2610.18010) [43] | Fish Audio Community License [43] | **Only under US$500k annual revenue**; "Built with Fish Audio" required; hosted TTS forbidden but game characters are explicitly allowed [43] | Arena leader; scene-level multi-character, infilling, director revisions [43] | **35B‑A3B MoE**; heavy |
| Fish S2‑Pro | 2026 [43] | Fish Audio Research License [43][20] | **No** | RTF 0.195 on an H200 [43] | — |
| DramaBox (Resemble) | 2026 [55] | LTX‑2 Community License; paid above US$10M revenue; machine-generation disclosure required [20][55] | Conditional | Prompt-driven expressive TTS [55] | ~24 GB peak, plus a Gemma‑3‑12B encoder [55] |
| Higgs v3, Voxtral TTS, Breeze TTS 2, F5‑TTS, Echo‑TTS, OmniVoice, XTTS‑v2 | — | NC / research / CPML [20][39][47][48] | **No** | — | — |
| VibeVoice 1.5B (long-form) | TTS code removed 2025‑09‑05 [41] | MIT weights [20] | Legally yes, ethically no | 90-min multi-speaker | Community forks only. **Avoid** [41] |

**Table C: commercial APIs (opt-in, bring-your-own-key only)**

| API | Current models | Multi-speaker / long-form | Voice creation | Notes |
|---|---|---|---|---|
| ElevenLabs | `eleven_v3`, `multilingual_v2`, `flash_v2_5`, `turbo_v2_5` [28] | Text-to-Dialogue: up to 10 unique voices per request; keep it under about 2,000 characters; request-ID stitching for continuity [28] | Voice Design and cloning | Best expressiveness. Per-character cost. |
| Google Gemini | `gemini-3.8-flash-tts`, `-flash-lite-tts` [58] | Multi-speaker turns with per-speaker style [58] | **Voice Design from text**; replication requires a **spoken consent clip** [58] | 30 prebuilt voices; inline tags such as `<laugh>` [58] |
| OpenAI | `gpt-4o-mini-tts` (snapshot 2025‑12‑15), `tts-1`, `tts-1-hd` [57] | Single speaker; 4,096-character limit [57] | `/audio/voice_consents` endpoint [57] | Instruction-steerable [57] |

**Creating voices without cloning real people.** We have four routes:
1. *Voice design*: Qwen3‑TTS VoiceDesign (Apache) [49], MOSS‑VoiceGenerator (Apache) [20][24], Maya1 (Apache) [20], or Gemini / ElevenLabs Voice Design.
2. *Fixed-voice blending*: Kokoro weighted mixes [32].
3. *Seed / speaker sampling*: Dia, Higgs "smart voice" [36][39].
4. *Post-processing*: pitch, formant and EQ offsets per NPC.

The robust pipeline: design N voices offline, then render a 10–20 s reference clip for each. Small cloning models (Chatterbox, Pocket) then use those clips as reference voices, so nobody real is ever cloned.

### 4. ASR for optional player voice input

| Model | License | Latency / footprint (claims) | Fit |
|---|---|---|---|
| Moonshine streaming (tiny / small / medium) | MIT [66][20] | Built for live streaming; tiny models down to 1 MB; C API, Windows/Linux/Mac [66] | **First choice (CPU)** |
| Parakeet‑TDT‑0.6B‑v3 / Parakeet‑unified‑en‑0.6b | CC‑BY‑4.0 [20][65] | Unified: streaming with 160 ms minimum latency [65] | Strong |
| Nemotron‑3.5‑ASR‑Streaming‑0.6B | OpenMDW‑1.1 [20] | 40 languages; latency adjustable from 80 ms to 1 s [65] | Strong (multilingual) |
| Voxtral Mini 4B Realtime (2602) | Apache‑2.0 [20] | 4B is heavy next to an LLM | Optional |
| Whisper large‑v3‑turbo | MIT [63] | 809M; about 6 GB VRAM in PyTorch; not trained for translation [63]; whisper.cpp offers Vulkan/Metal/CPU [64] | Offline QA in the drama pipeline |
| Kyutai STT 1B (EN/FR) | CC‑BY‑4.0 [45] | 0.5 s delay plus semantic VAD [45] | OK |
| VibeVoice‑ASR‑Streaming / BitNet (CPU) | MIT [41][20] | New in 2026 [41] | Evaluate |
| Turn-taking: Silero VAD (MIT), Smart Turn v3.2 (BSD‑2) | [20] | — | Use with any ASR |

### 5. The audio-drama production pipeline

**Script format.** The Narrative Compiler should emit structured data, not prose. Example:

```yaml
episode: 12
scenes:
  - id: s3
    location: {tile: 4411, time: "1032-04-17T21:40", weather: rain}
    ambience: {preset: city_night, telemetry_ref: s3_tel.json}   # re-render diorama engine offline
    lines:
      - {speaker: npc_88123, voice: V-0451, emotion: {angry: 0.6, afraid: 0.2}, text: "They took the bridge.", tags: ["[breath]"]}
      - {speaker: narrator, voice: V-0001, style: "low, measured", text: "..."}
    sfx: [{cue: cart_wheels_far, at: "line:1+0.4s"}]
    music: {cue: tension_low, duck: voice}
```

**Casting and consistency.** Keep a persistent *voice bible*: voice_id → model, reference clip, seed, style defaults and the post-processing chain. NPC → voice is a deterministic function of the NPC's ID and demographic bucket. On every take:
1. Run ASR and reject the take if WER is above threshold.
2. Check speaker similarity against the bible clip.
3. Generate up to N retries and keep the best.

This mirrors tts‑audiobook‑tool, which wraps 20+ models with STT verification, silence trimming and EBU R128 normalization. It needs about 2–4 GB extra VRAM for Whisper [61].

**Mixing and mastering**

| Step | Tool | Target |
|---|---|---|
| Ambience bed | Headless diorama audio engine driven by the scene's telemetry | Continuity between wallpaper and drama (our differentiator) |
| Ducking | Sidechain compressor (Godot [3], or ffmpeg / pedalboard offline) | Music/ambience at roughly −12 to −18 dB under dialogue (inference) |
| Loudness | Two-pass loudnorm / pyloudnorm | **−16 LUFS integrated, ≤ −1 dBTP** (Apple Podcasts guidance) [K][70]. EBU R128 is −23 LUFS for broadcast [K][71]. AES TD1008 range is −20 to −16 [K][72] |
| Watermark / provenance | Perth (Chatterbox built-in) [34], AudioSeal (MIT, code and weights) [77] | Mark every synthetic stem |

**Packaging and delivery**
- **Packaging:** MP3 with ID3v2 CHAP/CTOC chapters [K][69], or M4B for audiobook players. Add `<podcast:chapters>` JSON, which can be edited after publishing [68], plus `<podcast:transcript>` (VTT/SRT) [68], and AI-disclosure text in the description and ID3 comment.
- **Feed:** generate an RSS feed locally.
- **Getting it to the phone:**
  - (a) LAN HTTP server, which only works at home.
  - (b) A synced folder (e.g. Syncthing) plus AntennaPod's local-folder feeds [K].
  - (c) A bucket the user owns, with an unguessable URL. That URL is a privacy risk.
  - Apple Podcasts can follow a show by URL, but the feed must be publicly reachable [K].

**Existing open-source pipelines**

| Project | License | Engine | Quality / fit |
|---|---|---|---|
| tts‑audiobook‑tool | — | 20+ local models; ASR-validated retries; M4B chapters [61] | **Best reference design** |
| Podcastfy | Apache‑2.0 [73] | OpenAI / Google / ElevenLabs / Edge (cloud) [73] | NotebookLM-style; cloud-dependent |
| Open Notebook | MIT [74] | Provider TTS; 1–4 speakers [74] | Research-assistant UX |
| ebook2audiobook | Apache‑2.0 [75] | Many engines, XTTS-heavy [75] | Watch the per-engine licenses |
| audiblez / abogen | MIT [76] | Kokoro [76] | Single narrator; simple |

### 6. Ethics and policy

- **Consent is now the industry baseline.** Gemini voice replication requires a spoken consent statement, and OpenAI exposes a voice-consents endpoint [58][57]. Model licenses forbid cloning without consent (Kyutai [46], Dia [36], MOSS‑TTSD [44]).
- **Disclosure and marking** [K]:
  - EU AI Act Art. 50 (from 2 Aug 2026): providers must mark synthetic audio in machine-readable form, and deployers must disclose deepfakes. Evidently creative or fictional works get a lighter disclosure duty. A proposed omnibus delay for legacy systems needs checking [79].
  - Steam requires disclosure of live-generated AI content and the guardrails around it [80].
  - DramaBox's license itself requires disclosure [20].
- **Performers** [K][78]:
  - SAG‑AFTRA's 2024–25 video-game strike ended with a 2025 Interactive Media Agreement requiring informed consent, disclosure and compensation for digital replicas.
  - SAG‑AFTRA also filed an unfair-labor-practice charge over the AI Darth Vader voice in *Fortnite*.
  - Its 2024 deal with Replica Studios was followed by Replica shutting down (2025, [K]).
  - Relevance to us: if we ever license a real actor's voice, use union-compliant consent terms. Otherwise, use only designed voices.
- **Likeness laws** [K]: Tennessee ELVIS Act (2024), California AB 2602 / AB 1836, and the proposed federal NO FAKES Act. Don't let users clone arbitrary voices inside the product.

---

## Recommendations for Rehoboam

**A. Diorama: systemic ambience. Godot native, no middleware.**
1. **Telemetry contract.** The sim publishes an `AudioTelemetry` struct at about 10 Hz, aggregated per tile around the camera: wind speed and direction, precipitation, river and coastal flow, crowd density, cart/traffic density, transit throughput, lightning strikes (position, time), alarms, biome, time of day, season. The audio layer never touches agents.
2. **Bed layers** (about 10 buses):
   - Wind: GDExtension noise-filter synth plus gust recordings.
   - Water: 3 flow levels.
   - Rain: 3 intensities plus granular drops.
   - Crowd: 3 densities.
   - Carts/traffic: density bed plus clustered emitters.
   - City/transit: heavily low-passed rumble, gain set from transit throughput.
   - Fauna: by biome, time of day and season.
   - Interiors / reverb zones.
3. **One-shots.** A priority pool with per-category caps, cooldowns, distance culling and AudioStreamRandomizer [4]. Thunder is delayed by distance/343 m/s and low-passed with distance.
4. **Mix bus:** Master ← {Ambience, SFX, Music, Voice}. Voice sidechains Ambience and Music [3]. A zoom-level "LOD" crossfades the street, district and city beds.
5. **Wallpaper power policy.** Stop all players when fullscreen apps, other audio sources or battery power are detected. Use the Dummy driver when audio is disabled [8]. Track the WASAPI sleep PR [7].
6. **Assets.** Base layers are recorded or licensed. Variations come from MOSS‑SoundEffect v2 (Apache), Stable Audio Open / SA3 small‑sfx (under US$1M revenue, attribution) or ElevenLabs SFX (paid). Keep a provenance manifest of model, prompt, seed and license for every generated file.
7. **Revisit FMOD** only if a dedicated sound designer joins and wants it. Budget for plugin lag behind Godot releases [9].

**B. Street-level NPC speech (target 8-core CPU, 8 GB GPU shared with renderer and LLM)**

| Stage | Choice | Budget (claims, plus inference) |
|---|---|---|
| ASR (push-to-talk) | Moonshine streaming small (MIT); Parakeet‑unified as an alternative | CPU, about 1 core; 100–300 ms after end of speech |
| Turn end | Silero VAD plus Smart Turn | ~10 ms |
| TTS | **Chatterbox‑Nano (MIT)** or **Pocket TTS (CC‑BY‑4.0)** on 2 CPU threads via ONNX Runtime in a GDExtension. Chatterbox‑Turbo on the GPU if VRAM allows | ~200 ms to first audio; 0 GB VRAM |
| Voices | 300–600 designed voices (Qwen3‑TTS VoiceDesign, offline) → reference clips and precomputed embeddings → per-NPC pitch/EQ offsets | Embeddings in KB; a few MB total |
| Playback | PCM into AudioStreamGenerator from C++, through AudioStreamPlayer3D on the Voice bus | — |
| Latency masking | Pre-rendered barks ("Hm?", "Aye...") per voice; stream sentence by sentence from the LLM | End-to-end target 0.8–1.9 s |

- **Do not link espeak‑ng** (sherpa‑onnx TTS, Kokoro OOV fallback, Piper) into the game binary [62][33]. If we use Kokoro, run it in a separate process after legal review, or use dictionary-only G2P.
- Spike: measure Nano vs Pocket for quality, CPU percentage, first-audio latency and frame-time impact on a 6-core laptop.

**C. Offline audio drama (Narrative Compiler back end)**
1. **Script:** YAML/JSON as above, carrying emotion vectors and cue anchors.
2. **Voices:** Qwen3‑TTS VoiceDesign → voice bible.
3. **Rendering, by default per line:**
   - Emotional lines: IndexTTS‑2.5 (bilibili license; OK at our scale; pass the terms downstream).
   - Neutral lines and non-English: Qwen3‑TTS‑1.7B.
   - Banter scenes: MOSS‑TTSD v1.0 (Apache).
   - Evaluate Fish Drama 3 as a quality ceiling. Adopt it only if we accept the US$500k revenue cap and attribution.
4. **QA:** Whisper‑turbo WER gate, speaker-similarity gate, best-of-3.
5. **Beds:** the diorama engine rendered headless; ACE‑Step 1.5 (MIT) or library music.
6. **Master:** sidechain ducking; −16 LUFS / −1 dBTP; AudioSeal watermark.
7. **Export:** MP3 with ID3 CHAP plus JSON chapters, VTT transcript and private RSS.
8. **Hardware:** a 12 GB GPU is the minimum (TTS ~4–6 GB plus Whisper ~2–6 GB); 24 GB is comfortable. Expect 1–3× episode length in render time with retries (inference). Schedule renders when the PC is idle.
9. **Cloud option:** offer ElevenLabs v3 Dialogue and Gemini 3.8 TTS behind the user's own API key, for users who want top quality.

**License verdicts:**
- **Ship-safe:** Chatterbox, Kokoro*, Qwen3‑TTS, MOSS family, ACE‑Step, Moonshine, Whisper, AudioSeal, libpd.
- **Ship with conditions:** Pocket TTS (attribution), Parakeet (attribution), IndexTTS2 (bilibili terms), Stable Audio (under US$1M, attribution), Fish Drama 3 (under US$500k, attribution), Higgs v2 (under 100k AAU), NeuTTS Nano/2E (under US$5M).
- **Do not ship:** F5, XTTS, Fish S2‑Pro, Higgs v3, Voxtral TTS, Breeze 2, OmniVoice, YuE2, AudioCraft, MMAudio, TangoFlux, VibeVoice TTS, Piper (GPL plus voice data terms).

(*Kokoro only without espeak‑ng in-process.)

---

## Risks & unknowns

- **Evidence gap.** HF model cards, vendor pricing (FMOD, Wwise, ElevenLabs), SAG‑AFTRA texts, EU AI Act status and the Suno/Udio dockets could not be fetched. All [K] items need a re-check.
- **Licenses drift.** OmniVoice moved from Apache to NC in July 2026 [20]. Pin model revisions, archive the LICENSE file with each one, and re-check at every release.
- **Training-data provenance.** Permissive weight licenses (MOSS, ACE‑Step) do not guarantee clean data, and the outputs may be uncopyrightable [K][30].
- **Voice quality at scale.** Hundreds of designed voices cloned through 100M models may converge on a "same-ish" timbre. Needs a listening test.
- **CPU contention.** TTS on 2 threads plus sim plus renderer on a 6-core laptop may hurt frame time. Unmeasured.
- **Pocket TTS gating.** The cloning component is reportedly gated under separate terms [S]. Verify.
- **Godot gaps.** No HRTF; Steam Audio plugin under-maintained [11]; WASAPI sleep unmerged [7].
- **Regulatory.** Art. 50 marking applies to *us* if Rehoboam counts as a "provider" of a generative audio system in the EU. Get legal advice before the EU launch [K].
- **Hype flags.**
  - Most latency and real-time-factor numbers are vendor claims measured on H100, H200 or M4 hardware.
  - "Competitive with Suno v5" (YuE2, ACE‑Step) and "beats ElevenLabs" (MOSS‑TTSD) are the authors' own evaluations [25][26][44].

---

## Sources

[1] https://github.com/godotengine/godot/blob/master/doc/classes/AudioStreamGenerator.xml: [V] Godot AudioStreamGenerator; GDScript performance note.
[2] https://github.com/godotengine/godot/blob/master/modules/interactive_music/doc_classes/AudioStreamInteractive.xml: [V] interactive music clips and transitions.
[3] https://github.com/godotengine/godot/blob/master/doc/classes/AudioEffectCompressor.xml: [V] compressor `sidechain` for ducking.
[4] https://github.com/godotengine/godot/blob/master/doc/classes/AudioStreamPlayer3D.xml: [V] attenuation, Doppler, Area3D reverb; AudioStreamRandomizer referenced in the audio-streams docs.
[5] https://github.com/godotengine/godot/blob/master/doc/classes/DisplayServer.xml: [V] `tts_speak` OS TTS; SSML on Windows/Linux.
[6] https://github.com/godotengine/godot/pull/117162: [V] CoreAudio sleeps when idle; merged 2026‑08‑19 for 4.8.
[7] https://github.com/godotengine/godot/pull/122738: [V] WASAPI sleep, open; NVIDIA driver power-request note.
[8] https://github.com/godotengine/godot/issues/28039: [V] Godot audio prevents system sleep; Dummy-driver workaround.
[9] https://github.com/utopia-rise/fmod-gdextension: [V] FMOD GDExtension; latest release targets Godot 4.5 / FMOD 2.03.06.
[10] https://github.com/alessandrofama/wwise-godot-integration: [V] community Wwise integration; Wwise 2026.1 beta; Godot 4.3+.
[11] https://github.com/stechyo/godot-steam-audio: [V] alpha; maintainer says it is "not really being maintained".
[12] https://github.com/ValveSoftware/steam-audio: [V] Steam Audio 4.8.1; official FMOD/Wwise/Unity/Unreal plugins.
[13] https://www.fmod.com/licensing: [K] FMOD tiers and indie thresholds (unreachable).
[14] https://www.audiokinetic.com/en/pricing/: [K] Wwise pricing and free tier (unreachable).
[15] https://mitpress.mit.edu/9780262014410/designing-sound/: [K] Andy Farnell, *Designing Sound* (2010).
[16] https://github.com/libpd/libpd: [V] embeddable Pure Data; BSD license.
[17] https://www.gdcvault.com/: [K] search "The Sound of No Man's Sky" (Paul Weir, GDC 2017).
[18] https://godotengine.org/releases/4.7/: [V via godot-website source] Godot 4.7 released 2026‑06‑18.
[19] https://stability.ai/license: [K] Stability AI Community License.
[20] https://github.com/0xShug0/audio.cpp/blob/main/docs/model_licenses.md: [V] per-model license and commercial-use table, checked 2026‑09/10. Key secondary source.
[21] https://github.com/facebookresearch/audiocraft: [V] code MIT, weights CC‑BY‑NC‑4.0.
[22] https://github.com/hkchengrex/MMAudio: [V] CC‑BY‑NC checkpoints; ~6 GB VRAM.
[23] https://github.com/declare-lab/TangoFlux: [V] non-commercial, research-only.
[24] https://github.com/OpenMOSS/MOSS-TTS: [V] MOSS family (Apache‑2.0): TTS v1.5, Nano, VoiceGenerator, SoundEffect v2.
[25] https://github.com/ace-step/ACE-Step-1.5: [V] MIT; under 4 GB VRAM; XL 4B; speed claims.
[26] https://github.com/multimodal-art-projection/YuE: [V] YuE2 non-commercial; 24 GB VRAM.
[27] https://github.com/ASLP-lab/DiffRhythm: [V] Apache‑2.0; v1.2.
[28] https://github.com/elevenlabs/elevenlabs-python: [V] model list; Text-to-Dialogue limits; SFX v2 loop/duration.
[29] https://www.riaa.com/: [K] June 2024 label lawsuits vs Suno/Udio; later settlements per press coverage.
[30] https://www.copyright.gov/ai/: [K] USCO report on copyrightability of AI outputs (2025).
[31] https://github.com/hexgrad/kokoro: [V] Apache‑2.0; last commit 2025‑08‑06.
[32] https://github.com/remsky/Kokoro-FastAPI: [V] weighted voice combinations.
[33] https://github.com/hexgrad/misaki: [V] G2P with espeak fallback.
[34] https://github.com/resemble-ai/chatterbox: [V] Turbo/Nano/Multilingual V3; MIT; Perth watermark; Nano CPU speed.
[35] https://github.com/nari-labs/dia: [V] Dia 1.6B; Apache; 4090 real-time factor and VRAM.
[36] https://github.com/nari-labs/dia2: [V] Dia2 1B/2B; ~2 min max; misuse policy.
[37] https://github.com/canopyai/Orpheus-TTS: [V] 3B Llama-based; smaller sizes not released.
[38] https://github.com/SesameAILabs/csm: [V] CSM‑1B; Apache LICENSE file.
[39] https://github.com/boson-ai/higgs-audio: [V] v3 weights research/non-commercial; v2 README recommends 24 GB.
[40] https://huggingface.co/bosonai/higgs-audio-v2-tokenizer/discussions/8: [S] 100k-AAU clause quoted.
[41] https://github.com/microsoft/VibeVoice: [V] TTS code removed 2025‑09‑05; Realtime‑0.5B research-only; ASR releases.
[42] https://github.com/index-tts/index-tts: [V] IndexTTS‑2/2.5; bilibili license (100M MAU / RMB 1B).
[43] https://github.com/fishaudio/fish-speech: [V] Drama 3 / S2‑Pro; Fish Audio Community License (US$500k).
[44] https://github.com/OpenMOSS/MOSS-TTSD: [V] v1.0; 1–5 speakers; 60 min; evals vs Eleven v3.
[45] https://github.com/kyutai-labs/delayed-streams-modeling: [V] Kyutai STT/TTS; STT weights CC‑BY‑4.0.
[46] https://github.com/kyutai-labs/pocket-tts: [V] 100M CPU TTS; ~200 ms; prohibited-use policy; ports.
[47] https://github.com/SWivid/F5-TTS: [V] weights CC‑BY‑NC (Emilia data).
[48] https://github.com/idiap/coqui-ai-TTS: [V] fork of the "unmaintained" Coqui repo; XTTS CPML per https://github.com/Vaibhavs10/open-tts-tracker.
[49] https://github.com/QwenLM/Qwen3-TTS: [V] 2026‑01‑22; Apache‑2.0; VoiceDesign; 97 ms claim.
[50] https://github.com/neuphonic/neutts: [V] Air (Apache) vs Nano/2E (NeuTTS Open License); watermarked.
[51] https://github.com/OHF-Voice/piper1-gpl: [V] active; voice licenses vary; "personal use and research".
[52] https://github.com/KittenML/KittenTTS: [V] 0.8 ONNX and KittenTTS 2; models licensed separately.
[53] https://github.com/ekwek1/soprano: [V] 80M; single voice; no cloning.
[54] https://github.com/supertone-inc/supertonic: [V] repository archived.
[55] https://github.com/resemble-ai/dramabox: [V] LTX‑2-based; ~24 GB peak.
[56] https://github.com/breezeblue-ai/breeze-tts: [V] weights research/non-commercial.
[57] https://github.com/openai/openai-openapi: [V] `gpt-4o-mini-tts-2025-12-15`; voice-consents endpoint.
[58] https://github.com/google-gemini/cookbook/blob/main/quickstarts/Get_started_TTS.ipynb: [V] Gemini 3.8 TTS; Voice Design; consent for replication.
[59] https://artificialanalysis.ai/text-to-speech/leaderboard/provider-voice/open-weights: [S] open-weights Elo snapshot.
[60] https://www.yaps.ai/blog/best-local-tts-models: [S] Oct 2026 guide; Chatterbox's TTS Arena position.
[61] https://github.com/zeropointnine/tts-audiobook-tool: [V] ASR-verified long-form generation; EBU R128; M4B chapters.
[62] https://github.com/k2-fsa/sherpa-onnx: [V] Apache‑2.0; TTS build pulls espeak‑ng (GPL‑3.0) via cmake.
[63] https://github.com/openai/whisper: [V] turbo 809M, ~6 GB; MIT.
[64] https://github.com/ggml-org/whisper.cpp: [V] C/C++ Whisper; Vulkan/Metal/CPU; MIT.
[65] https://github.com/NVIDIA/NeMo: [V] Parakeet/Canary/Nemotron ASR and Magpie TTS release log.
[66] https://github.com/moonshine-ai/moonshine: [V] MIT models (except legacy non-English); streaming.
[67] https://github.com/huggingface/open_asr_leaderboard: [V] leaderboard methodology (2026).
[68] https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/chapters.md: [V] `podcast:chapters` and `podcast:transcript`.
[69] https://id3.org/id3v2-chapters-1.0: [K] ID3v2 CHAP/CTOC addendum.
[70] https://podcasters.apple.com/support/893-audio-requirements: [K] −16 LKFS ±1, −1 dBFS true peak.
[71] https://tech.ebu.ch/publications/r128: [K] EBU R128 −23 LUFS.
[72] https://www.aes.org/: [K] AES TD1008 streaming/podcast loudness recommendation.
[73] https://github.com/souzatharsis/podcastfy: [V] Apache‑2.0; cloud TTS back ends.
[74] https://github.com/lfnovo/open-notebook: [V] MIT; 1–4 speaker podcasts.
[75] https://github.com/DrewThomasson/ebook2audiobook: [V] Apache‑2.0; multi-engine.
[76] https://github.com/santinic/audiblez: [V] MIT, Kokoro; also https://github.com/denizsafak/abogen [V] MIT.
[77] https://github.com/facebookresearch/audioseal: [V] MIT code and weights; localized watermarking.
[78] https://www.sagaftra.org/: [K] 2025 Interactive Media Agreement AI terms; Replica and Llama Productions matters.
[79] https://artificialintelligenceact.eu/article/50/: [K] Art. 50 transparency and marking, applicable 2 Aug 2026.
[80] https://store.steampowered.com/news/group/4145017/view/3862463747997849618: [K, URL from memory] Valve AI-content disclosure rules (Jan 2024).
