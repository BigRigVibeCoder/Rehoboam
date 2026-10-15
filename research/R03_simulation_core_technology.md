# R03 — Simulation-Core Technology: Language, ECS, Determinism, Scheduling, Performance

*Research dossier for Rehoboam. As of 2026-10-15.*

**Method and evidence labels.** This session's egress policy blocked most project websites (bevy.org, flecs.dev, godotengine.org, docs.rs, arXiv, Wikipedia, factorio.com and others), and the shared web-search budget ran out early. Verified facts therefore come from GitHub repositories and release pages, raw source files, the crates.io and PyPI registries, and source code downloaded locally. Each verified fact has a numbered URL. **[PK]** marks prior knowledge that I could not re-fetch this session; treat it as plausible but unverified. **[B]** marks micro-benchmarks I ran for this report on a 4-vCPU Intel Xeon (2.1 GHz, AVX-512) sandbox with Rust 1.97, NumPy 2.5.4, Numba 0.68 and unified-planning 1.3.0. A modern 8–16-core desktop should be roughly 2–4× faster in total, though not per kernel. Paragraphs marked *Inference* are my own reasoning.

---

## TL;DR

- **Flecs has no Python bindings, so the draft's "Flecs C99/C++/Python bindings" claim is false.** Flecs (v4.1.6, 29 Jun 2026, MIT) is the strongest ECS for relationships and introspection [1][2]. Its README lists community bindings for C#, Rust, Zig, Lua and Clojure, but not Python [1]. The only Python package is a 0.0.2 pybind11 prototype with a single CPython 3.9/Linux wheel, and it explicitly prioritises "arbitrary Python types… rather than optimized performance" [6]. The Rust binding is self-declared alpha [8][9].
- **">5,000 ticks/s" fails for the workload the draft describes.** [B] With 8 diffusion layers on a 512² grid plus binomial/Poisson cohort demography for 1M people, NumPy manages about 50 ticks/s and multithreaded Rust about 290 ticks/s. One NumPy diffusion layer on its own costs 1.5 ms, which caps it at about 680 ticks/s. Reaching 5,000 ticks/s means at most 0.2 ms per tick.
- **"1,000 years in under 60 s" works only at monthly macro resolution, in compiled code, for the macro strata alone.** At daily resolution it would take 73 s even at the claimed 5,000 ticks/s. Named actors at 10–30 Hz cannot run in fast-forward: 1,000 years at 30 Hz is 9.5×10¹¹ ticks.
- **The draft's speed presets are off by about five orders of magnitude.** At 10,000×, 1,000 years takes **36.5 days** of wall time. Doing it in 60 s needs **≈5.3×10⁸×**, and doing it in 10 minutes needs ≈5.3×10⁷×. Time controls should be bands of sim-time per wall-second plus "run until" conditions, not multipliers (see §5).
- **The Micropolis and Ceptre parts of the plan carry licence problems.** Micropolis is GPLv3 with EA's Section 7 additional terms [55][56], and its "kernels" are integer 5-point smoothing on downsampled grids that take about 10 lines to rewrite. Ceptre ships under the **Anti-Capitalist Software License v1.4**, which bars most for-profit organisations, and it is a command-line Standard ML research prototype [58][59].
- **Validating every action with unified-planning is not feasible.** [B] One validation of a one-action plan costs **11.9 ms**. At 5k actors × 10 Hz that is about 600 CPU-seconds per simulated second. Keep PDDL for offline checks of LLM-compiled manifests.
- **Talk of the Town is unusable as a dependency.** It is Python 2 code (`print` statements, `xrange`), and its last code commit was in June 2021. It simulates one town from 1839 to 1979, and only about 10 of 730 timesteps per year actually run [62][63]. Borrow the ideas, not the code.
- **Recommended language is Rust for the authoritative core,** with Python bolted on through PyO3/maturin for tooling and LLM work, and Godot attached in-process through gdext. The same crate builds as a headless CLI, a server and a Python wheel. Rust does not contract FMA by default, has `rayon`, `rand_chacha` stream IDs and pure-Rust `libm`, and its PyO3 and gdext ecosystems are the strongest of the candidates [37][32][75].
- **Most state should live outside the ECS.** Stratum 1 belongs in dense SoA arrays and age-structured cohort tensors. Use an ECS only for the 1k–5k named actors. For that layer, take **bevy_ecs** (standalone, pinned version) or **hecs**, and keep many-to-many social relations in a dedicated deterministic graph store. Bevy still has no many-to-many relationships as of 0.20 [11][13].
- **Determinism must be designed in from the start.** [B] The local benchmarks showed that `rayon`'s `par_iter().sum()` gives **different bits at 1, 2 and 4 threads**, that std `HashMap` order changes on every run, and that NumPy's `Generator` carries an explicit "No Compatibility Guarantee" [46]. Fixed-chunk reductions and per-chunk ChaCha streams produced bit-identical results across thread counts.
- **Bincode is dead.** RUSTSEC-2025-0141 (16 Dec 2025) declares it permanently unmaintained, and v3.0.0 is a tombstone containing nothing but `compile_error!` [51][52]. Use rkyv for snapshots and postcard, bitcode or wincode for messages.

---

## Draft claims checked

| Draft claim | Verdict | Evidence |
|---|---|---|
| "Flecs C99/C++/Python bindings" | **False for Python** | The README's binding list has no Python entry [1]. The flecs-hub org has no Python repo, and its C# binding `flecs-cs` is archived [7]. The PyPI package `flecs` 0.0.2 (23 Jul 2025) is a personal pybind11 prototype with one wheel and 2 GitHub stars [6]. Flecs-Rust calls itself an alpha with an "experimental" API [8]. |
| Flecs REST explorer | **True, but development-only** | REST server on port 27750 with entity, query and world endpoints. Its docs say the built-in HTTP server "should not be used in production" [5]. Explorer pages were expanded in 4.1.2 and 4.1.6 [2]. |
| ">5,000 ticks/sec headless" (512² float32 multi-layer diffusion plus 1M-person demography) | **Implausible by 10–100×** | [B] NumPy: 1.48 ms per layer per tick, 8 layers ≈ 12 ms, cohort demography 6.8–8.9 ms, total ≈ 50 ticks/s. Rust with rayon on 4 threads: 0.65 ms for 8 layers plus 2.8 ms demography ≈ 290 ticks/s. |
| "1,000 years in < 60 s" | **Conditional** | It needs at most 5 ms per monthly tick (12,000 ticks). [B] Macro strata in Rust take about 3.5 ms per tick, giving ≈42 s on 4 vCPUs. Daily ticks (365,242) need 164 µs per tick, which is out of reach. It is incompatible with 10–30 Hz actors. |
| Speed presets 1×/100×/10,000× give "1,000 years in minutes" | **Wrong arithmetic** | 1,000 y = 3.156×10¹⁰ s. At 10,000× that is 3.16×10⁶ s ≈ 36.5 days. A 60 s run needs 5.26×10⁸×. |
| Micropolis diffusion kernels via ctypes | **Pointless and legally hazardous** | The code is GPLv3 [55] with header text "ADDITIONAL TERMS per GNU GPL Section 7… No trademark or publicity rights are granted" [56]. The kernels are integer 5-point averages (`z = (z + src) >> 2`) on half-, quarter- and eighth-resolution maps [56]. Loading GPL code into the process via ctypes makes a combined work under the FSF's reading [64, PK]. The name "Micropolis" is itself a trademark, and the maintained C++/WASM engine is also GPL-3.0 [54][57]. |
| Ceptre for invariants | **Impractical; licence blocks commercial use** | Standard ML/MLton command-line interpreter with no embedding API [58]. The last commits (Aug 2024) are vim-syntax pull requests [58]. Licence: Anti-Capitalist Software License v1.4, limited to individuals, non-profits, educational bodies and worker co-ops, and excluding military and police [59]. |
| unified-planning PDDL validation before each commit | **Infeasible at runtime** | [B] `SequentialPlanValidator`: 11.9 ms per one-action validation, plus 4.9 ms to build a 100-object problem. UP is Apache-2.0, v1.3.0, 17 Dec 2025 [60], but the Fast Downward engine wrapper is GPL-3.0-only [61]. |
| Talk of the Town for genealogy seeding | **Not as code** | Python 2 syntax; last code commit 2 Jun 2021; MIT licence added Aug 2024 [62]. One town, 1839–1979, with `number_of_timesteps_to_simulate_a_year = 10.0` and a global `random.seed` [63]. |
| bevy_ecs as a headless Rust alternative | **True** | "can easily be used as a standalone crate" [14]. Relationships are one-to-many and non-fragmenting only [13]. 0.20.0 was released 8 Oct 2026 [10]. |
| "Flecs / Python ECS" for 1k–5k actors at 10–30 Hz | **Muddled** | Two runtimes cannot both own authoritative state. [B] A toy utility-AI pass over 5k actors takes 44.7 ms in pure Python and 0.81 ms in Rust. |

---

## Landscape / state of the art

### 1. ECS options (2026)

| Name | Version / date | Licence | Maturity | Fit for Rehoboam | Notes |
|---|---|---|---|---|---|
| **Flecs** (C/C++) | v4.1.6, 29 Jun 2026 [2] | MIT [1] | Mature and very active | Excellent for relations and introspection; poor for Python | Relationships are pairs with wildcards. Traits include `Exclusive` ("only a single instance of a relationship"), `Symmetric`, `Traversable` and `DontFragment` (sparse, "does not fragment tables") [4]. The docs warn that heavy relationship use causes "high levels of fragmentation" and that table creation is expensive; `(*, Target)` queries may need linear scans [3]. In a third-party benchmark Flecs 4.0.1 updated 1M entities with 7 systems in ≈19 ms, against ≈40 ms for EnTT [18]. |
| flecs_ecs (Rust) | 0.2.2, 17 Nov 2025 [9] | MIT | **Alpha** [8] | Risky | Tracks Flecs v4; "API is experimental… breaking changes". |
| **bevy_ecs** | 0.20.0, 8 Oct 2026 [10] | MIT/Apache-2.0 | Active; breaking releases every 4–5 months (0.17 Sep 2025, 0.18 Jan 2026, 0.19 Jun 2026, 0.20 Oct 2026) [10] | Good once pinned | One-to-many non-fragmenting relationships since PR #17398 (merged 18 Jan 2025); many-to-many and fragmenting relations deferred [13], with none listed in the 0.20 notes [11]. 0.19 stores resources as components and adds contiguous query slices [12]. 0.20 adds schedule randomisation via `shuffle_seed` to expose ordering bugs, plus `chain_weak` ordering [11]. `bevy_remote` provides the Bevy Remote Protocol, roughly equivalent to Flecs REST [15]. |
| **hecs** | 0.11.2, 4 Oct 2026 [16] | MIT/Apache-2.0 | Stable and minimal | Good if you write your own scheduler | "A library, not a framework"; no systems, scheduler or relations [16]. |
| EnTT (C++) | v4.0.0 (23 Jul, presumably 2026; now C++20) [17] | MIT | Mature | OK if the core is C++ | Sparse-set design. 3.16 added a work-in-progress viewer [17]. No built-in relations. |
| Unity Entities | 1.4 with Unity 6.2 [19] | Unity Companion License [PK] | Production inside Unity | Engine lock-in; no | Samples target Unity 6.2 and Entities 1.4 [19]. |
| Unreal Mass | UE5 [PK] | Unreal EULA [PK] | Used for crowds and traffic (City Sample) [PK] | Engine lock-in; no | Docs domain unreachable [20]. |
| specs | 0.20.0 (Sep 2023); last commit 7 Jun 2024 [23] | MIT/Apache-2.0 | **Dormant** | No | Amethyst engine archived 18 Apr 2022 [21]. |
| legion | 0.4.0 (Feb 2021); last commit 3 Oct 2021 [22] | MIT | **Abandoned** | No | — |
| esper (Python) | 3.9, Sep 2026 [76] | — | Maintained, pure Python | Prototyping only | — |

*Inference.* For 1k–5k named actors, ECS throughput doesn't matter; even 1M entities take only tens of milliseconds [18]. What matters is deterministic scheduling, relation modelling, change detection that drives the story sifter and facade, and inspector tooling. Flecs wins on relation expressiveness. In Rust, bevy_ecs wins on batteries (observers, change ticks, relationships, reflection, remote protocol), at the price of churn. The relations Rehoboam needs are mostly many-to-many and weighted (Suspects, Trust, Knows), and those should not be archetype pairs in any ECS. They belong in a dedicated graph store with sorted adjacency.

### 2. Agent-based modelling frameworks

| Name | Version / date | Licence | Scale and speed | Embeddable as a game core? | Notes |
|---|---|---|---|---|---|
| FLAME GPU 2 | v2.0.0-**rc.5**, 14 Oct 2026; still a release candidate since Dec 2022 [25] | **AGPL-3.0 or commercial** since rc.3 (Nov 2025); MIT before [25] | Millions of agents on GPU [PK]; README gives only qualitative claims [24] | Poor fit | NVIDIA CUDA only (compute capability 5.0+), SWIG Python, C++ API with "no stability guarantees" [24]. |
| Mesa | 3.5.1 (15 Mar 2026); 4.0.0a0 (14 Mar 2026) [26] | Apache-2.0 | 3.3–159× slower than Agents.jl [27] | No | 3.5 added `schedule_event`, `run_until` and recurring events; 4.0a0 removes `batch_run`, `mesa.space` and `PropertyLayer` [26]. |
| Agents.jl / Ark.jl | Agents 6.2.10; Ark 0.5.1 [27] | MIT [PK] | Fastest in the comparison; the Ark.jl archetype ECS beats Agents.jl by 0.17–0.99× time [27] | No (Julia runtime) | Good for offline research and calibration. |
| MASON | 22.0 [27] | [PK] | 0.6–7.8× Agents.jl time [27] | Java; no | Active repo [30]. |
| NetLogo | 7.0.4 (1 May 2026); 7.1.0-alpha1 [31] | GPL [PK] | 4.8–19× slower [27] | No | Teaching tool. |
| Repast4Py | 1.3.0 (2 Oct 2026) [29] | BSD-3 | MPI-distributed HPC | No | Needs MPI/mpi4py and PyTorch [29]. |
| krABMaga (Rust) | 0.6.2 (19 Jun 2026) [28] | MIT | No published numbers [28] | Library; small user base | Bevy visualisation, MPI parameter sweeps [28]. |

**Lessons that transfer.** (a) Keep the model separate from the scheduler and the visualisation. MASON and Mesa both do this, and Mesa 3.5 moved to an explicit discrete-event API [26]. (b) GPU ABM is fast because agents are binned spatially and talk through messages. Rehoboam's 16×16 chunks already play that role on the CPU. (c) The fastest Julia framework is an archetype ECS, Ark.jl [27]: SoA layout beats object-per-agent by 1–2 orders of magnitude, which argues against Mesa-style Python objects for anything hot. (d) Benchmarks should run in CI, as ABMFrameworksComparison does [27].

### 3. Language and bindings

| Option | State (2026) | Determinism | Godot embedding | Python bridge | Verdict |
|---|---|---|---|---|---|
| **Rust** | Mature; rayon 1.12, glam 0.34, wgpu 30 (crates.io) | No FMA contraction by default [PK]. Std transcendentals have "Unspecified precision… varies by platform, Rust version" [41]; pure-Rust `libm` 0.2.16 avoids that [75]. | **gdext** (`godot` crate 0.5.5, 9 Aug 2026, MPL-2.0) [77][33]: "usable for games"; occasional breaking changes; wasm experimental; api-4-7 support [32][33] | PyO3 0.30.1 / maturin 1.15 [37]. Free-threaded builds supported since 0.23, thread-safe by default since 0.28 [38]. | **Recommended** |
| C++ | Mature; Flecs, EnTT | GCC/Clang contract FMA unless `-ffp-contract=off`. Box2D sets this explicitly under "# Deterministic math" [47]. | godot-cpp (MIT): extensions built for older 4.x run on newer, not the reverse [34] | nanobind/pybind11 | Viable second choice, especially with Flecs |
| C# | .NET; Godot .NET | Same-platform determinism plausible; CRT math varies [PK] | Godot C# **still has no web export**; issue locked 19 Jun 2026 with "foundation is evaluating solutions" [36] | pythonnet (clunky) [PK] | Not recommended for the core |
| Python (NumPy/Numba/JAX) | NumPy 2.5.4, Numba 0.68, JAX 0.11.2 (PyPI) | NumPy `Generator`: "**No Compatibility Guarantee**… the bit stream may change" [46] | Not realistic in-process | n/a | Prototyping, tooling and analytics. [B] Numba serial matches Rust per kernel. |
| Julia | 1.13.2 [40] | Same-machine reproducible | Runtime and JIT embedding impractical [PK] | PythonCall [PK] | Offline research only |
| Zig | No 1.0; GitHub's latest is 0.15.1 [39] | Good control | Possible via C ABI | via C | Too unstable |

Godot itself is at 4.7.2 (stable since 18 Jun 2026) [35]. Rehoboam doesn't need web export, so C#'s web gap is not decisive. The real reasons against C# are the weaker headless and Python story and a smaller ecosystem for deterministic numerics.

### 4. Determinism

**Floating point.** IEEE-754 basic operations are correctly rounded, so bit-identical results need the same operation order, no contraction, no fast-math, no platform `libm`, and SIMD-width-independent reductions [68, PK]. Box2D's build documents that FMA contraction "is the one value changing transform enabled by default, and whether it fires depends on the target", and disables it [47][48]. Rust's std `sin`, `exp` and `powf` are explicitly non-deterministic across platforms and versions [41]. That matters directly here, because the draft's mortality and fertility code will call `powf` and `exp`.

Even `rand_distr` warns that enabling `std_math`, which any dependency enabling `num-traits/std` does silently, "may produce different random values" [44]. The safest pattern is integer or fixed-point arithmetic for every authoritative quantity that crosses a threshold (population counts, money, loyalty, timers), with floats confined to fields. Those fields should be computed with fixed operation order and `libm`, then quantised to integers at macro boundaries. *Inference:* fixed-point everywhere (Photon Quantum style) costs too much for 512² stencils. Quantising at boundaries plus golden-hash CI is the pragmatic middle path.

**RNG.** Use counter-based, keyed streams, never one global generator. ChaCha in `rand_chacha` 0.10 offers "2⁶⁴ unique streams… per seed/key" via `set_stream`, plus O(1) seeking with `set_word_pos` [45]. Philox from Random123 is the GPU-friendly equivalent [66, PK], and NumPy exposes `Philox` with `advance`, `jumped` and `SeedSequence.spawn` [46]. `rand::StdRng` is "Non-portable: any future library version may replace the algorithm" [43]. PCG is fine for single-threaded use but harder to split safely [67, PK].

Derive each stream key as `(world_seed, system_id, chunk_or_entity_id)` and set the position from the tick. Adding a new system then never perturbs the others. It also enables **lazy deterministic hydration**: a street-level NPC's traits can be regenerated on demand from `hash(seed, chunk, cohort, index)` instead of being stored. Pin or vendor the binomial and Poisson samplers, since algorithm changes in the distribution crates are value-breaking.

**Iteration order.** [B] Std `HashMap` iteration order differed on every run: `[6,2,5,…]`, then `[2,7,0,…]`, then `[7,8,9,…]`. Use `BTreeMap`, `IndexMap` with fixed insertion order, or sorted `Vec`s in authoritative paths. Rayon documents that for non-associative operators "the results are not fully deterministic" [42]. [B] Summing 10M floats gave serial bits `4a989293`, and rayon `par_iter().sum()` gave `4a9891bf`, `4a9891ec` and `4a9891e2` at 1, 2 and 4 threads. A fixed 65,536-element chunk reduction gave `4a9891dd` at every thread count. Demography with per-chunk ChaCha streams hashed identically (`fa482b21ce9137cc`) at 1, 2 and 4 threads.

**Lessons from lockstep games.**
- AoE-style lockstep sends only inputs, and every peer runs the same simulation [69, PK].
- Factorio runs a fixed 60 UPS lockstep. Peers compare state checksums, a mismatch triggers a desync report and a map re-download, and `game.speed` scales the tick rate [70][71, PK].
- OpenRA marks fields with `[Sync]`, XOR-hashes them per type, and wraps UI and rendering code in `RunUnsynced`. That guard recomputes the world hash and throws "sync-changing code may not run here" if the state changed [49]. This is the guard pattern Rehoboam needs around LLM, facade and renderer code.
- openage stores state as keyframed "curves", so any past or future time can be queried. The costs are memory and interpolation work [50]. That idea fits the scrubbable timeline.

**Hashing, snapshots and serialisation.** [B] BLAKE2b in Python hashed 9.2 MB of state in 12.9 ms (0.72 GB/s); xxh3 in Rust should be about 10× faster [PK]. A monthly hash costs very little. Float32 fields compress badly: zlib took 8.4 MB down only to 6.6 MB. Quantising to u16 and delta-coding brought them to 2.1 MB [B].

*Inference on sizes.* A keyframe should run about 15–40 MB compressed: 16 layers is 16 MiB raw, about 4 MB quantised. Cohorts are about 0.8 MB at single-year ages, or more with compartments. Each of 5k actors carries perhaps 1–2 KB of state plus a relation graph. Taking a keyframe every 10–25 sim-years plus a compressed event log gives about 1–4 GB per 1,000-year run. Scrubbing then means loading the nearest keyframe and re-simulating forward. At ≈3.5 ms per monthly tick, 25 years takes about 1 s.

| Format | Version | Status | Use |
|---|---|---|---|
| rkyv | 0.8.18 (Aug 2026), MIT | Active; zero-copy with `bytecheck` validation [53] | Local keyframes (mmap) |
| postcard / bitcode / wincode | 1.1.3 / 0.6.9 / 0.6.3 | Active | Compact messages; the RUSTSEC advisory names these as alternatives [51] |
| FlatBuffers | 25.12.19 | Active, Apache-2.0 | Cross-language streaming to non-Rust clients |
| Arrow/Parquet via polars 2.0 | Oct 2026 | Active | Event logs and analytics in Python |
| **bincode** | 3.0.0 = `compile_error!` [52] | **Dead** [51] | Do not use |

### 5. Multi-rate scheduling and time controls

**Precedents.**
- **Paradox (Clausewitz/Jomini):** fixed daily ticks with heavier monthly and yearly pulses, and five discrete speeds whose ceiling is set by the cost of a daily tick [PK]. Victoria 3 "pops" are aggregates of people sharing location, culture and profession [74, PK]. That is the same move as Rehoboam's cohorts.
- **Dwarf Fortress:** a fortress-mode day is 1,200 ticks (403,200 per year), with many subsystems updated only every N ticks. World generation produces up to about 1,050 years of history with a far coarser model than fortress play [72][73, PK].
- **Talk of the Town:** runs only about 10 of 730 half-day steps per year during worldgen [63]. Stochastic temporal sparsity is an accepted technique.
- **Factorio:** a fixed tick, with speed implemented as tick-rate scaling [71, PK].

*Inference.* Every shipped system that covers centuries does it by changing resolution, never by executing fine ticks quickly.

**The arithmetic.** 1,000 years = 365,242 days = 3.156×10¹⁰ s.

| Target | Multiplier needed | Budget per monthly tick (12,000) | Budget per daily tick (365,242) |
|---|---|---|---|
| 60 s | 5.26×10⁸× | 5.0 ms | 164 µs |
| 10 min | 5.26×10⁷× | 50 ms | 1.6 ms |
| Draft "10,000×" | — | 1,000 y takes **36.5 days**; 1 y takes 52.6 min | — |

**Proposed scheduler.** *Inference, opinionated.*
1. **One canonical schedule in integer sim-time**, using `u64` minutes since epoch. History must be a pure function of `(seed, manifest, input log)`, independent of wall-clock speed and of which modality is watching ("observer invariance").
2. **Macro tier (Stratum 1):** monthly ticks for demography (age-structured binomial cohorts), economy and contagion fields. Fields substep internally only when the CFL condition requires it; explicit Euler needs `4α+β ≤ 1`. Fields that relax quickly should be solved quasi-statically. Solve `(β−α∇²)H = S` only when sources change, by multigrid or CG; with constant α, an FFT exponential integrator steps any Δt exactly in about 9 ms per layer in NumPy [B]. Politics runs quarterly or yearly.
3. **Meso tier (named actors):** a discrete-event queue keyed `(time, phase, entity_id, seq)` so ties break deterministically, instead of polling at 10–30 Hz. Actors get a daily or weekly agenda plus interrupts (events they perceive). A binary heap is enough at 10⁴–10⁶ pending events. Calendar queues [65, PK] or timing wheels only matter beyond that.
4. **Micro tier (30 Hz bubble within ~100 m):** non-authoritative elaboration of meso agendas: locomotion, animation, barks. Anything consequential the player does becomes a timestamped **input event** written to the log, exactly like lockstep inputs.
5. **Time controls:** bands of *sim-time per wall-second*: Live (1 s/s), 1 min/s, 1 h/s, 1 day/s, 1 week/s, 1 month/s, 1 year/s, 1 decade/s, and **MAX**. Add **run-until** conditions (date, predicate, or the next sifted pattern above a salience threshold) with auto-pause, and timeline scrubbing via keyframes and re-simulation. Bands only change presentation and how often the client is published to, never the simulation itself.

### 6. Performance measurements [B]

| Workload (per tick) | NumPy | Numba | Rust serial | Rust rayon (4 thr) |
|---|---|---|---|---|
| 512² 5-point diffusion, 1 layer | 1.48 ms (slicing), 2.10 ms (`np.roll`) | 0.16 ms | 0.13–0.16 ms | — |
| Same, 8 layers | ≈12 ms | 1.5–1.8 ms (0.94 ms on 2 thr) | 1.28–1.54 ms | **0.65 ms** |
| FFT exponential integrator, 1 layer, any Δt | 9.0 ms | — | — | — |
| Cohort demography: 1,024 chunks × 2 sexes × 101 ages = 206,848 binomial cells + Poisson births (≈1M people) | 6.8–8.9 ms | — | 9.5 ms | **2.8 ms** |
| 1M individual Bernoulli deaths | 8.3 ms | — | — | — |
| Utility AI: 5k actors × 16 actions × 4 logistic considerations | 44.7 ms (pure Py), 2.5 ms (vectorised) | 2.2 ms | 1.35–1.64 ms | **0.81 ms** |
| unified-planning one-action plan validation | 11.9 ms each | — | — | — |

Numba's parallel mode was erratic in this sandbox: 8–10 ms at 4 threads, which looks like oversubscription. Treat thread scaling as a hardware-dependent unknown.

*Derived budgets.*
- **NumPy draft stack:** about 20 ms per tick, so about 50 ticks/s. 1,000 years takes about 4 minutes at monthly resolution and about 2 hours at daily resolution.
- **Rust macro stack:** about 3.5 ms per tick, about 290 ticks/s. 1,000 years takes about 42 s monthly and about 21 minutes daily.
- **With 5-year age bands** (≈43k cells, about 5× cheaper; inference): about 1.2 ms per tick, so about 15 s per 1,000 years.
- **Named actors:** realistic utility decisions with graph and spatial queries cost about 1–5 µs each in Rust (inference from the 0.27 µs toy). A monthly cadence (6×10⁷ decisions) costs 60–300 CPU-s. Daily cadence (1.8×10⁹) costs 0.5–2.5 CPU-hours, and roughly 30× more in Python.

So a full-fidelity 1,000-year run is a **minutes-to-tens-of-minutes** job. Only the macro layer fits in a minute. GOAP and A* planning per actor per tick is out of the question at this scale [PK: F.E.A.R. used it for a handful of agents]; reserve it for rare strategic replans and cache the plans.

**GPU (wgpu or CUDA).** 8 layers take 0.65 ms on 4 CPU threads, so moving the stencil to the GPU buys almost nothing. It also adds readback latency, and cross-vendor float results are not bit-identical [PK]. Use the GPU to render fields, not to own them.

---

## Recommendations for Rehoboam

1. **Core language: Rust.** Build one workspace with these crates:
   - `rehoboam-core`: pure library, no I/O, owns all authoritative state.
   - `rehoboam-cli`: headless `run`, `replay`, `verify-hash` and `bench`.
   - `rehoboam-py`: PyO3/maturin wheel. Releases the GIL with `Python::detach` during runs and exposes snapshots as NumPy/Arrow views [37][38].
   - `rehoboam-godot`: gdext GDExtension, pinned to `api-4-7` [33].
   - `rehoboam-server`: Rust WebSocket server with JSON-RPC 2.0 for control and binary frames (FlatBuffers, or rkyv for Rust clients) for state deltas.

   Keep FastAPI only as the Python-side LLM gateway, as a *client* of the core. Never stream 1M-entity state as JSON.
2. **State layout.**
   - **Stratum 1** is dense SoA, not ECS: `Vec<f32>`/`ndarray` fields quantised at macro boundaries, plus `u32` age-structured cohort tensors `[chunk][sex][age-band][compartment]`. Draw deaths as **binomial, not Poisson**, because Poisson draws are unbounded and can drive small chunks negative.
   - **Stratum 2** uses **bevy_ecs pinned to one release** behind a thin facade. Run the single-threaded executor for authoritative systems, enable ambiguity checks, and use the 0.20 `shuffle_seed` in CI [11]. Use rayon *inside* heavy systems with fixed chunking. Model ChildOf/FealtyTo with Bevy one-to-many relationships [13], whose single target makes them exclusive.
   - Keep Suspects, Trust, Knows and the hive-mind link in a `SocialGraph` resource: sorted CSR or BTreeMap adjacency with weights and provenance.
   - Keep per-character knowledge as sparse sets of event IDs, so the LLM facade can only quote what that character knows.
   - If the team prefers C++, use Flecs v4 + godot-cpp + nanobind instead. It is coherent, but you will write and maintain the Python bindings yourself [1][6].
3. **Scheduler:** the canonical integer-time calendar with three tiers and an event queue described in §5. Replace multiplier presets with sim-time bands, MAX, run-until conditions and auto-pause.
4. **Determinism contract:**
   - Integers or fixed-point for counts, money and thresholds. `libm`, not std math, in authoritative code [41][75].
   - Never use `mul_add` or fast-math; set `-ffp-contract=off` for any C/C++ dependency [47].
   - Keyed ChaCha8 streams per system and chunk/entity [45], with vendored samplers.
   - No `HashMap` iteration in authoritative paths. Fixed-chunk reductions only.
   - xxh3 state hash every sim-month, written into the log.
   - **Golden-run CI:** 1,000-year runs on Linux x86_64, Windows and macOS ARM, compared by hash every sim-year.
   - An OpenRA-style `RunUnsynced` guard so that the renderer, LLM and facade paths get read-only snapshots [49].
5. **Threading:** one sim thread owns the World, a rayon pool runs data-parallel kernels, and immutable snapshot frames are published through `arc-swap` or a triple buffer. Godot's main thread and Python only read frames. Wallpaper mode runs at low priority with a CPU budget.
6. **Python's role:** NL→DSL manifest compiler (schema-validated, then hashed into the input log); LLM orchestration for facade and synthesiser; calibration notebooks fitting cohort rates to life tables; story-sifter authoring; analytics over Parquet event logs; Hypothesis-based property tests. Hot sifting patterns used for auto-pause should be compiled into Rust.
7. **Drop or replace:**
   - Micropolis: clean-room 5-point smoothing.
   - Ceptre: Rust `debug_assert!` invariants per tick (population conservation, non-negativity, acyclic ChildOf/FealtyTo), proptest, and optionally TLA+ or Alloy at design time [PK].
   - unified-planning: offline only, to check that LLM-compiled scenarios are achievable.
   - Talk of the Town: write a cohort-to-lineage generator.
   - FLAME GPU: AGPL and CUDA-only.
   - bincode: dead.

---

## Risks & unknowns

- **Cross-platform bit-identity is unproven** until golden-run CI exists. Expect to spend weeks hunting libm, FMA and iteration-order leaks. A fallback is "same build + same OS family" determinism, with keyframe resync.
- **Churn in pre-1.0 dependencies:** Bevy (~3 breaking releases a year) [10], gdext 0.5.x with editor-placeholder changes coming in 0.6 [33], flecs_ecs alpha [8], Mesa 4 removals [26]. Pin versions and wrap each one behind a facade.
- **Speed independence versus attention:** if the design later wants extra detail wherever the player is looking, that attention must be logged as input. Otherwise watched and unwatched runs diverge.
- **Fidelity cliff:** at MAX speed named actors run monthly, so drama finer than a month only exists at slow bands. Story sifting must accept coarse event granularity.
- **Event-log growth** (~GBs per run) and snapshot schema versioning across releases. rkyv archives are tied to type layout.
- **Benchmark caveats:** a 4-vCPU sandbox and toy kernels. Real utility AI with queries, contagion with heterogeneous α, and Python↔Rust crossings must be re-measured. Numba threading misbehaved here.
- **Unverified [PK] items** because of egress limits: Paradox, DF and Factorio internals; Unity and Unreal licence details; GPL linking interpretation [64]. Get legal review before shipping anything near GPL or AGPL code.

---

## Sources

1. https://github.com/SanderMertens/flecs — README: MIT licence; community binding list (C#, Rust, Zig, Lua, Clojure; no Python).
2. https://github.com/SanderMertens/flecs/releases — v4.1.2–v4.1.6 notes; v4.1.6 released 29 Jun 2026; explorer and REST additions.
3. https://github.com/SanderMertens/flecs/blob/master/docs/Relationships.md — pair semantics; fragmentation and table-creation cost warnings.
4. https://github.com/SanderMertens/flecs/blob/master/docs/ComponentTraits.md — Exclusive, DontFragment, Symmetric, Traversable.
5. https://github.com/SanderMertens/flecs/blob/master/docs/FlecsRemoteApi.md — REST API, port 27750, "should not be used in production".
6. https://pypi.org/project/flecs/ — 0.0.2 pybind11 prototype, single CPython 3.9 Linux wheel, 23 Jul 2025.
7. https://github.com/orgs/flecs-hub/repositories — no Python binding; flecs-cs archived.
8. https://github.com/Indra-db/Flecs-Rust — self-described alpha; experimental API.
9. https://crates.io/crates/flecs_ecs — v0.2.2, 17 Nov 2025.
10. https://crates.io/crates/bevy_ecs — 0.20.0 on 8 Oct 2026; release history 0.17–0.20.
11. https://github.com/bevyengine/bevy-website/blob/main/content/news/2026-10-08-bevy-0.20/index.md — 0.20 notes: schedule randomisation, chain_weak, no many-to-many.
12. https://github.com/bevyengine/bevy-website/blob/main/content/news/2026-06-19-bevy-0.19/index.md — resources as components, contiguous query access.
13. https://github.com/bevyengine/bevy/pull/17398 — one-to-many non-fragmenting relationships (merged 18 Jan 2025); many-to-many deferred.
14. https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/README.md — standalone use; schedules, observers, change detection.
15. https://crates.io/crates/bevy_remote — Bevy Remote Protocol crate, 0.20.0.
16. https://github.com/Ralith/hecs — minimal "library, not a framework" ECS (crates.io 0.11.2, Oct 2026).
17. https://github.com/skypjack/entt/releases — v4.0.0 (C++20), v3.16 viewer, release history.
18. https://github.com/abeimler/ecs_benchmark — Flecs 4.0.1 vs EnTT 3.13 at 1M–2M entities.
19. https://github.com/Unity-Technologies/EntityComponentSystemSamples — Entities 1.4 with Unity 6.2.
20. https://dev.epicgames.com/documentation/en-us/unreal-engine/overview-of-mass-entity-in-unreal-engine — Mass Entity docs [PK; unreachable this session].
21. https://github.com/amethyst/amethyst — archived 18 Apr 2022, development halted.
22. https://github.com/amethyst/legion/commits/master — last commit 3 Oct 2021.
23. https://github.com/amethyst/specs/commits/master — last commit 7 Jun 2024.
24. https://github.com/FLAMEGPU/FLAMEGPU2 — CUDA-only requirements, AGPL-3.0, SWIG Python, unstable C++ API.
25. https://github.com/FLAMEGPU/FLAMEGPU2/releases — rc.5 on 14 Oct 2026; MIT→AGPL/commercial at rc.3.
26. https://github.com/projectmesa/mesa/releases — 3.5.1, 4.0.0a0, event-scheduling API and removals.
27. https://github.com/JuliaDynamics/ABMFrameworksComparison — CI benchmark of Agents.jl, Ark.jl, MASON, NetLogo, Mesa.
28. https://github.com/krABMaga/krABMaga — Rust ABM with Bevy visualisation, MIT (crates.io 0.6.2).
29. https://github.com/Repast/repast4py — MPI-distributed Python ABM, BSD-3 (PyPI 1.3.0).
30. https://github.com/eclab/mason — MASON Java ABM repository.
31. https://github.com/NetLogo/NetLogo/releases — 7.0.4 (May 2026), 7.1.0-alpha1.
32. https://github.com/godot-rust/gdext — status statement, platforms, MPL-2.0.
33. https://github.com/godot-rust/gdext/releases — v0.5.0–0.5.5 (2026), api-4-7, wasm changes.
34. https://github.com/godotengine/godot-cpp — GDExtension forward-compatibility policy, MIT.
35. https://github.com/godotengine/godot/releases — 4.6 (26 Jan), 4.7 (18 Jun), 4.7.2 (18 Aug) 2026.
36. https://github.com/godotengine/godot/issues/70796 — C# web export still unsupported; locked 19 Jun 2026.
37. https://github.com/PyO3/pyo3 — PyO3 overview, maturin workflow, notable users.
38. https://github.com/PyO3/pyo3/blob/main/guide/src/free-threading.md — free-threaded Python support since 0.23, thread-safe default since 0.28.
39. https://github.com/ziglang/zig/releases — latest GitHub release 0.15.1; no 1.0.
40. https://github.com/JuliaLang/julia/releases — Julia 1.13.2.
41. https://github.com/rust-lang/rust/blob/master/library/std/src/num/f64.rs — "Unspecified precision… non-deterministic" for sin, exp and powf.
42. https://github.com/rayon-rs/rayon/blob/main/src/iter/mod.rs — sum and reduce "not fully deterministic" for floats.
43. https://github.com/rust-random/rand/blob/master/src/rngs/std.rs — StdRng is non-portable across versions.
44. https://github.com/rust-random/rand_distr — portability note on libm vs `std_math` (v0.6.0 README).
45. https://github.com/rust-random/rand — rand_chacha 0.10 `set_stream` (2⁶⁴ streams) and `set_word_pos` docs (crate source).
46. https://numpy.org/doc/stable/reference/random/generator.html — Generator "No Compatibility Guarantee" (checked in the installed 2.5.4 docstring).
47. https://github.com/erincatto/box2d/blob/main/CMakeLists.txt — "# Deterministic math": `-ffp-contract=off`.
48. https://box2d.org/posts/2024/08/determinism/ — Box2D cross-platform determinism write-up [PK; linked from 47].
49. https://github.com/OpenRA/OpenRA/blob/bleed/OpenRA.Game/Sync.cs — `[Sync]` field hashing and the `RunUnsynced` guard.
50. https://github.com/SFTtech/openage/blob/master/doc/code/curves.md — keyframe "curves" for time-queryable state.
51. https://rustsec.org/advisories/RUSTSEC-2025-0141.html — bincode permanently unmaintained; alternatives listed (read via the advisory-db GitHub mirror).
52. https://crates.io/crates/bincode/3.0.0 — tombstone release (`compile_error!`), checked by downloading the crate.
53. https://github.com/rkyv/rkyv — zero-copy deserialisation with bytecheck validation.
54. https://github.com/SimHacker/micropolis — GPL code, Micropolis trademark notice, "historical" repo.
55. https://github.com/SimHacker/micropolis/blob/master/micropolis-activity/COPYING — GPL version 3.
56. https://github.com/SimHacker/micropolis/blob/master/MicropolisCore/src/MicropolisEngine/src/scan.cpp — EA Section 7 additional terms; integer smoothing kernels.
57. https://github.com/SimHacker/MicropolisCore — current C++/WASM engine, GPL-3.0.
58. https://github.com/chrisamaphone/interactive-lp — Ceptre: Standard ML/MLton command-line tool; last commits Aug 2024.
59. https://github.com/chrisamaphone/interactive-lp/blob/master/LICENSE — Anti-Capitalist Software License v1.4.
60. https://github.com/aiplan4eu/unified-planning — Apache-2.0 planning API (PyPI 1.3.0, 17 Dec 2025).
61. https://pypi.org/project/up-fast-downward/ — Fast Downward engine wrapper, GPL-3.0-only.
62. https://github.com/james-owen-ryan/talktown — Talk of the Town source (Python 2; last code commit 2021).
63. https://github.com/james-owen-ryan/talktown/blob/master/config/basic_config.py — 1839–1979 span, 10 simulated timesteps per year, global seed.
64. https://www.gnu.org/licenses/gpl-faq.html#GPLStaticVsDynamic — FSF view on linking and combined works [PK].
65. https://doi.org/10.1145/63039.63045 — Brown 1988, calendar queues [PK].
66. https://doi.org/10.1145/2063384.2063405 — Salmon et al. 2011, Random123/Philox counter-based RNGs [PK].
67. https://www.pcg-random.org/paper.html — O'Neill, PCG family [PK].
68. https://gafferongames.com/post/floating_point_determinism/ — Fiedler on float determinism [PK].
69. https://www.gamedeveloper.com/programming/1500-archers-on-a-28-8-network-programming-in-age-of-empires-and-beyond — AoE lockstep [PK].
70. https://wiki.factorio.com/Desynchronization — Factorio desync detection and reports [PK].
71. https://lua-api.factorio.com/latest/classes/LuaGameScript.html — `game.speed` tick-rate scaling [PK].
72. https://dwarffortresswiki.org/index.php/Time — DF ticks per day and year [PK].
73. https://dwarffortresswiki.org/index.php/World_generation — DF worldgen history lengths [PK].
74. https://vic3.paradoxwikis.com/Pops — Victoria 3 aggregate pops [PK].
75. https://crates.io/crates/libm — pure-Rust libm 0.2.16 (rust-lang/compiler-builtins).
76. https://pypi.org/project/esper/ — esper 3.9 pure-Python ECS.
77. https://crates.io/crates/godot — gdext crate 0.5.5, MPL-2.0.
