# R10: Data Infrastructure, Sifting Engines, Snapshots/Replay, Client Protocol and Console UI

*Research dossier for Rehoboam. Date: 2026-10-15. Status: research input, not a decision record.*

**Conventions.** Bracketed numbers like [3] point to Sources. **Measured** marks numbers from a synthetic benchmark run for this dossier (method in the Appendix). *Inference* marks my own reasoning rather than a sourced fact. Where a fact comes only from a search snippet, because the primary page could not be fetched (duckdb.org, Wikipedia, gafferongames.com and factorio.com had DNS failures in this session), the text says so. The session's web-search budget ran out partway through, so a few items rest on training knowledge and are flagged as such.

---

## TL;DR

- **The chronicle is small; the draft's storage worries are misplaced.** Measured: a typed event row costs 10.8 B in Parquet+zstd and 13.1 B in DuckDB native. That puts 250 M named-actor events (5k actors × 50/yr × 1,000 yr) at about **2.7–3.3 GB**. The draft's SQLite schema with JSON columns costs **83 B/event**, or **114 B with two indexes**, which is about **28 GB**. Two things grow without bound: commoner-level events (~5 B over 1,000 years) and keyframe snapshots (~0.2 GB each at 1 M agents). Size those, not the chronicle.
- **Do not hard-delete events below weight 7.** Deleting them breaks causal chains and destroys the low-weight building blocks that patterns like covert sabotage are made of. The disk saving is about 2 GB. Replace deletion with tiering plus causal-aware salience (see Recommendations).
- **Sifting engine: build a custom incremental matcher in the sim process,** using Winnow-style partial matches [37] keyed by bound variables and indexed by verb. Measured: a pure-Python version ran at **2.2 M events/s** for a betrayal pattern and returned exactly the same 6,073 matches as the DuckDB SQL version. A Rust version should be one to two orders of magnitude faster (*inference*). Use DuckDB SQL for retrospective queries and to validate the matcher.
- **Matching at event-emission time gives "as-of" state for free.** The matcher sees the world exactly as it was when the event happened. Retrospective queries still need interval (SCD2) tables for slowly changing facts such as titles and factions; DuckDB answered such an as-of join over 10 M events in **18 ms** (measured).
- **DuckDB 1.5.6 (2026-09-28) is right for warm/cold analytics but wrong as the per-tick engine.** Small incremental queries hit a floor of about 20 ms (measured), and only one process may write [6]. DuckDB 2.0, with a new default storage format, is due in fall 2026 [3]. Keep Parquet or Arrow as the durable format and treat DuckDB files as caches.
- **Several draft dependencies are dead or risky:** **Kùzu was archived 2025-10-10** (fork: LadybugDB 0.21.2, MIT, active) [12][13]; **CozoDB has had no commits since 2024-12-04** [15]; **DDlog is archived** [23]; **bincode is permanently unmaintained** (RUSTSEC-2025-0141, Dec 2025) [39]; and **Materialize is BSL with a license key** [27].
- **Replay design:** deterministic re-simulation from keyframes plus an input journal. Store permanent keyframes every 10 sim-years (~100 × 0.2 GB ≈ 20 GB at 1 M agents; ~2 GB at 100k), add a lazily built cache of yearly keyframes around the cursor, and record state hashes for divergence checks. **Scrubbing the timeline should read the chronicle and aggregates, never re-simulate.** Full rehydration happens only when the user "drops in".
- **Keep JSON-RPC 2.0 for the control plane only.** At 200 entities and localhost speeds, bandwidth is not the problem (measured: 1.7–1.8 Mbit/s as JSON). The problems are semantics (RPC, not a stream; no sequence or baseline handling), schema drift and GDScript dictionary churn. Send state as **binary columnar delta frames** on the same WebSocket; measured at about 9× smaller than JSON.
- **A "SQLite spatial index for hydration" is the wrong tool.** Measured: R*Tree queries are fast (49 µs), but updates run at about 50k/s. Moving 1 M agents through it would take about 20 s per tick. Use the sim's in-memory spatial grid.
- **The Flecs explorer only makes sense if the ECS is Flecs.** The Rust binding `flecs_ecs` is at 0.2.2 with about 3k recent downloads [54]. Use **Rerun 0.39** (MIT/Apache, Rust/Python/C++ SDKs) [40] for time-scrubbable debug views and **Textual 8.2.8** for a headless ops TUI [67].
- **God-console:** a web console (deck.gl 9.4 + Sigma.js 4 + a custom timeline) served on localhost by the sim. Package it with Tauri 2.12 on Windows and macOS; Linux webview GPU performance is unverified. Use **marimo 0.25** plus DuckDB and Polars 2.0 for notebook analytics.

---

## Draft claims checked

| Draft claim | Verdict | Evidence |
|---|---|---|
| `chronicle_events` with `context_json`, `causes_json` in SQLite/DuckDB | **Revise.** Use typed columns plus an overflow blob, and store causes as a `LIST<UBIGINT>` or an edge table. | Measured: the JSON columns cost 1.6× in Parquet (16.9 vs 10.8 B/event) and 1.9× in DuckDB (25.5 vs 13.1 B/event). SQLite holds the JSON schema at 83 B/event. |
| "Compact every 100 years by deleting weight < 7" | **Reject as written.** | It breaks `causes` references and the sim's determinism (if the sim reads the chronicle), and removes low-weight parts of composite tropes. Measured: the saving is about 2.3 GB per 250 M events. Weight is assigned at emission, before significance is known (*inference*). |
| "DuckDB as primary analytical event engine" | **Accept for analytics; reject for the per-tick or write path.** | Measured: DuckDB is 60× faster than SQLite on a one-sim-year window (27 ms vs 1,668 ms) but 13× slower on a single tick (23 ms vs 1.7 ms). DuckDB allows one read-write process [6]. 2.0 changes the storage format [3]. |
| "Felt Datalog → Python/SQLite CTEs" | **Partly wrong.** | Felt is a JS library over DataScript and is retrospective [36]. Incremental sifting is what Winnow, from the same group, adds [37]. Sequence patterns compile to self-joins plus anti-joins, not recursion; recursive CTEs are needed only for causal-chain walks. Measured: SQLite took 1.7 s per sim-year window, against about 0.11 s per sim-year for the pure-Python incremental matcher (4.5 s for all ~40 sim-years). |
| "SQLite spatial index for hydration" | **Reject.** | The R*Tree uses 32-bit floats and is a virtual table [49]. Measured: 50k updates/s and 9 s to bulk-load 1 M points. |
| "FastAPI WebSocket JSON-RPC 2.0 with entity JSON diffs" | **Split it.** JSON-RPC for control, binary frames for state. | Measured: 22.6 KB per full 200-entity JSON frame against 2.6 KB columnar binary. Godot's WebSocketPeer handles binary frames, but its default inbound buffer is **65,535 B** [51]. Larger JSON snapshots need configuration. |
| "Flecs explorer as debug console" | **Conditional.** | A REST-backed web UI on port 27750, documented with C and C++ examples only [54]. Not a timeline or causal tool. |
| "Kùzu (or similar) embedded graph DB" | **Kùzu is dead upstream.** | Archived 2025-10-10 at v0.11.3 [12]. Apple acquisition is *reported*, not officially confirmed [14][68]. The LadybugDB fork is active (0.21.2, 2026-10-01, MIT) [13]. |
| "Scrubbable 1,000-year timeline" | **Feasible, but only as two mechanisms.** | Chronicle and aggregate scrubbing are instant. Exact state needs keyframes plus deterministic replay (see Recommendations). |

---

## Landscape / state of the art

### 1. Event store and analytics engines

| Name | Version / date | License | Maturity | Fit for Rehoboam | Notes |
|---|---|---|---|---|---|
| **DuckDB** | 1.5.6, 2026-09-28 [1][2]; 1.4 LTS line (1.4.5 on 2026-06-17) [2]; 2.0 previewed 2026-08-17 [3] | MIT | High | Warm and cold analytics, retrospective sifting, console queries | `USING KEY` recursive CTEs since 1.3 [4][69]; v2.0 changes the semantics [5]. One read-write process; other processes read-only only when no one writes [6]. 2.0 adds triggers, VARIANT, a new parser, a new default storage format and stable Quack client-server [3]. EOL dates per a search snippet: 1.5 on 2026-11-01, 1.4 LTS on 2026-11-17 [9]. Rust crate `duckdb` 1.10506.0 (2026-09-30) encodes the engine version in its minor number; supports `appender-arrow` and `vtab-arrow` [7]. |
| **SQLite (WAL)** | 3.45.1 tested locally | Public domain | Very high | Small config/metadata stores, input journal | Measured: 320k inserts/s from Python, 83 B/event, quick indexed point queries (1.7 ms). Large scans are 60× slower than DuckDB. |
| **Parquet + Arrow IPC** | PyArrow 26.0.0 (installed) | Apache-2.0 | Very high | Durable, immutable segment format; snapshot format | Measured: Arrow IPC opened zero-copy 90 MB in 0.6 ms. Every tool here (DuckDB, Polars, deck.gl, Rerun) speaks Arrow. |
| **Polars** | **2.0.0, 2026-10-06** [10] | MIT | High; 2.0 is a major release | In-process transforms in Python tooling | Just released; expect breaking changes. No storage or indexes. |
| **LanceDB** | 0.40.0 [11] | Apache-2.0 | Medium | **Not an event store.** Useful only for vector search, e.g. over LLM memories. | Columnar plus vector index. |

**DuckDB as a sifting engine (measured on 10 M events, 4 vCPU):**
- Betrayal pattern over the full history (self-join plus `NOT EXISTS`): **105 ms**.
- Regicide as an interval join against `titles(valid_from, valid_to)`: **18 ms**. Usurpation (two interval joins): **24 ms**.
- Causal walk over 8.4 M edges with `USING KEY`: **27 ms** for ancestors of 694 seeds to depth 50, **86 ms** for descendants of 6,940 seeds to depth 50.
- Building the edge table with `unnest(causes)`: 332 ms.

*Inference:* scaling linearly, a full-history retrospective pattern over 250 M events takes a few seconds. That is fine for authoring and validation, not for every tick.

### 2. Datalog, graph and incremental engines (2026 status)

| Name | Latest / date | License | Maturity / health | Embeddable in Rust / Python? | Incremental? | Notes |
|---|---|---|---|---|---|---|
| **Soufflé** | 2.5, 24 Mar (2025 by packaging evidence); commits through 2026-10-02 [17] | UPL-1.0 (*training knowledge*) | Mature, slow release cadence | C++ codegen or interpreter. No native Python. | No (batch) | The reference for performance. Good for offline batch runs over Parquet exports. |
| **Ascent** | 0.8.1, 2026-08-29 [18] | MIT | Active, research origin | Rust macro; parallel via rayon | No. Each run starts from its inputs. | Lattices, aggregates, stratified negation, BYODS [18]. Paper claims performance comparable to Soufflé and datafrog [21]. |
| **crepe** | 0.2.0, 2025-12-14 [19] | MIT/Apache-2.0 | Small, alive | Rust proc-macro | No | Stratified negation; README does not mention aggregates [19]. |
| **datafrog** | 2.0.1 (2019) [20] | MIT/Apache-2.0 | Frozen, but stable | Rust library | Semi-naive only | A low-level building block (leapjoin). |
| **DataScript** | 1.8.1 [34] | EPL-1.0 | Stable | JS/Clojure only | No | Felt's substrate [36]. In-memory and transient by default. |
| **Datahike** | Active (copyright 2014–2026) [35] | EPL-1.0 | Medium | JVM; Python and C bindings in beta | No | Durable, time-travel Datalog. JVM-centric. |
| **CozoDB** | v0.7.x; **last commit 2024-12-04** [15] | MPL-2.0 | **Dormant; DBDB lists it as abandoned** [16] | Rust and Python | No | Would have been an ideal fit (embedded Datalog with time travel). Avoid. |
| **XTDB v2** | 2.0 GA 2025-06-12 [28]; 2.2.0-beta3 Sep 2026 [28] | MPL-2.0 | Production | JVM node, Postgres wire, ADBC | No (bitemporal store) | All tables are bitemporal (system plus valid time) [28]. Too heavy for a local headless app. |
| **TerminusDB** | v12.0.8, 12 Oct (2026) [29] | Apache-2.0 | Maintained by DFRNT since 2025 [29] | Server | No | Git-like versioned graph. Poor fit. |
| **Kùzu** | **Archived 2025-10-10**, final release v0.11.3 [12] | MIT | **Dead upstream** | (was Rust/Python) | No | README says the team is "working on something new" [12]. |
| **LadybugDB** (Kùzu fork) | 0.21.2, 2026-10-01 [13] | MIT | Active fork, about one year old | Rust `lbug`, Python `real_ladybug`/`ladybug` [13] | No | Cypher property graph. Sustainability of the fork is unknown. |
| **DuckPGQ** | Community extension; described as "research project… work in progress" (Jul 2026) [33] | (DuckDB ext.) | Experimental | Via DuckDB | No | SQL/PGQ (SQL:2023) graph matching inside DuckDB. |
| **Differential Dataflow** | 0.25.1, 2026-07-15 [22] | MIT | Mature research engine | Rust library | **Yes** | README numbers: 10 M nodes / 50 M edges load in ~15 s; one change propagates in ~230 µs; batched rounds reach ~1 M updates/s [22]. Steep learning curve. |
| **DDlog** | **Archived** (banner says 2026-07-13) [23] | MIT | Dead | — | Was incremental | Its authors moved on to DBSP/Feldera (*training knowledge*). |
| **DBSP / Feldera** | `dbsp` 0.366.0, **2026-10-15**; releases every few days [24] | MIT OR Apache-2.0 [24] | Very active, VC-funded | `dbsp` Rust crate, embeddable in-process (tutorial) [24]. SQL front end needs Java 19–21, Maven and Calcite at build time [25]. | **Yes**: full IVM for SQL [26] | The strongest industrial incremental view maintenance option. Constant minor-version churn means API risk. |
| **Materialize** | Self-managed Community Edition since 2025-03 [27] | **BSL 1.1** → Apache-2.0 after 4 years; license key required; ≤24 GiB memory [27] | Production server | No | Yes | Licensing and server-only form rule it out for an embedded desktop sim. |
| **Nemo** | v0.10.1, 1 Aug (year inferred 2026) [30] | Apache-2.0/MIT | Research; README says "unstable" [30] | Rust, Python bindings | Not documented | VLog's successor. Reasons over 10^5–10^8 facts on a laptop [30]. |
| **RDFox** | Commercial | Proprietary | Production | C++ server/library | Yes (incremental reasoning) | Oxford Semantic was acquired by Samsung in July 2024 [31]. License rules it out. |
| **FlowLog** | VLDB 2026 paper [32] | ? | Research | Built on DD (Rust) | Yes | Batch and incremental Datalog. Mixed results against Soufflé (e.g. CSPA-httpd: Soufflé 67.8 s vs FlowLog 112 s single-config) [32]. Worth watching. |

**What emerges.**
- No maintained, embeddable engine combines **Datalog syntax, incremental evaluation and Python access**.
- The incremental engines that are healthy (DD, DBSP) are Rust libraries with dataflow APIs.
- The healthy Datalog front ends (Ascent, crepe, Soufflé) are batch-only.
- Story sifting at Rehoboam's scale is mostly **bounded-window sequence matching**: joins on actor or target plus time windows plus "unless" anti-joins. A recursive fixpoint is rarely needed. *Inference:* that makes a purpose-built automaton both simpler and faster than general incremental view maintenance.

**Prior art.** Felt describes sifting patterns as Datalog over DataScript [36]. Winnow compiles to Felt for retrospective queries and adds **incremental** and **partial** matching. For example, its "violation of hospitality" pattern uses `unless-event` [37], and the Winnow paper explicitly targets the "fundamentally retrospective nature of existing sifters" [37].

### 3. Temporal (as-of) state for sifting rules

| Design | How it works | Cost | Best for |
|---|---|---|---|
| **A. Capture at emission** | Copy the few attributes patterns need (actor title, faction, affinity toward target, secret or public) into typed event columns. | ~5–15 B per event raw; compresses well (*inference*) | Hot attributes used by many patterns |
| **B. Interval tables (SCD2)** | `state_intervals(entity, attr, value, valid_from, valid_to)`, queried with an interval join or DuckDB `ASOF JOIN` | Small: titles and factions change rarely. Measured: 18 ms join over 10 M events. | Slowly changing facts: titles, marriages, alliances, residence |
| **C. Versioned components / snapshots** | Keyframe plus replay, then read the state | Seconds to minutes per query | Rare deep forensics ("what did X believe on day D?") |
| **D. Bitemporal (XTDB-style)** | Valid time plus system time on every row [28] | Doubles the temporal machinery | Only if history is *revised*. Rehoboam's history is append-only per timeline. |
| **E. Incremental matcher at emission** | The matcher's state *is* the as-of state | Zero extra storage | Live sifting and auto-pause |

*Inference:* Use E for live sifting, A plus B for retrospective SQL and C for forensics. Avoid full bitemporality. A "what-if" edit in the time machine creates a **branch** (`timeline_id` plus fork tick), not a correction to valid time. In-world misinformation (what characters believe happened) is simulation content and belongs in belief components, not database time axes.

### 4. Snapshots and deterministic replay

**Serialization options.**

| Format | Version / status | License | Fit |
|---|---|---|---|
| **Arrow IPC (+ zstd buffers)** | PyArrow 26 | Apache-2.0 | **Best for keyframes.** Columnar per component, zero-copy reads (0.6 ms for 90 MB, measured), and DuckDB, Polars and Rerun can read snapshots directly. Columns can be added as the schema evolves. |
| **rkyv** | 0.8.18, 2026-08-05 [38] | MIT | Zero-copy Rust structs. Good for an in-process ring buffer. Brittle across struct-layout changes, so version every archived type (*inference*). |
| **bincode** | **3.0.0 is a `compile_error!` stub: development has ceased** [39] | MIT | **Do not use.** RUSTSEC-2025-0141 suggests wincode, postcard, bitcode or rkyv [39]. |
| **FlatBuffers / Cap'n Proto** | (not re-verified) | Apache-2.0 / MIT | Zero-copy with schemas. A good wire format, but GDScript support comes from community add-ons (*unverified*). |
| **MessagePack** | — | — | Measured: only 10% smaller than JSON when keys are strings (20.3 KB vs 22.6 KB). |
| **zstd** | — | BSD | Measured on 4 threads: level 1 runs at ~1.1 GB/s and level 3 at ~0.65 GB/s with near-identical ratios. Level 19 manages only 6–11 MB/s for a 1.65× vs 1.37× ratio. Use level 3 for live keyframes and recompress cold keyframes at high levels in the background. |

**Measured snapshot sizes** (synthetic, so compression is a pessimistic lower bound). 1 M agents with 17 component fields, including needs, traits, skills and beliefs arrays, take **90 B/agent** in Arrow. 20 M relationship edges at 13 B each take **260 MB**. Total raw is **≈350 MB** and zstd-3 output is **≈195 MB** per keyframe. A one-tick delta in which 10% of agents change four columns is 2.2 MB compressed. Memories and inventories are not modeled and could dominate (see Risks & unknowns).

**Prior art.**
- **Deterministic lockstep** sends only inputs, so bandwidth is independent of object count. It needs bit-identical floating point and the same binary [44].
- **StarCraft II replays** store game events and are decoded with a build-specific protocol library [46]. *Training knowledge:* they need the matching game build to play back.
- **Factorio** (training knowledge, site unreachable this session) runs deterministic lockstep with CRC checks to detect desyncs. Its replays are input logs tied to the game version.
- **Rocket League** (training knowledge, unverified) records network-stream frames with keyframes. It is snapshot-based rather than input-based.
- **rr** records nondeterministic inputs and replays deterministically with reverse execution on Linux; it needs hardware performance counters [47]. Pernosco, rr's sponsor, adds "omniscient" queries over a precomputed execution database.
- **Replay.io** applies record and replay to the browser; its devtools repo is still active [48], but its product direction was not verified.

*Inference:* the transferable lessons are to log only nondeterministic inputs, checkpoint periodically, hash state to detect divergence, and keep a precomputed query database (Pernosco ≈ chronicle plus sifter) so most "time travel" questions never need a replay.

**Rerun** fits as a sim debug viewer. GitHub tag 0.39.0 is dated 12 Oct, with asset timestamps indicating 2026; crates.io's latest is 0.38.1 (2026-09-16). It is dual MIT/Apache-2.0 with Python, Rust and C++ SDKs, and it is open-core: "everything in this repository will stay open source", while the commercial product is Rerun Hub [40]. Data is logged against named timelines and the viewer scrubs them, with a configurable memory buffer [40]. Release 0.39 adds embedding in marimo notebooks [40]. Use it to log sampled agents, the hydration bubble, area-of-interest cells and per-system timings on a `sim_tick` timeline. Do not stream 1 M agents per tick for 1,000 years into it.

### 5. Sim → client protocol

**Transport options.** *Inference, except where cited.*
- **In-process GDExtension**, via godot-rust gdext (crate `godot` 0.5.5, MPL-2.0, "usable", with occasional breaking changes [53]). Zero-copy, but it couples the sim's lifecycle and crashes to the renderer and complicates headless runs.
- **Shared memory or pipes.** Fastest, but platform-specific and harder to debug.
- **Localhost WebSocket.** Built into Godot through WebSocketPeer, which supports binary frames via `put_packet` or `send(..., WRITE_MODE_BINARY)` and defaults to 64 KB buffers and 4,096 queued packets [51]. Easy to debug, works from any language, and allows remote viewing later.
- **WebTransport.** No built-in Godot class as far as I know (*unverified*), and it has no advantage on localhost.

**Measured payloads for a 200-entity bubble.** Each entity carries id, position, yaw, animation, action text, mood and HP. The delta assumes 60% of entities moved and 5% changed state.

| Encoding | Full frame | Delta | Full @10 Hz | Delta @30 Hz |
|---|---|---|---|---|
| JSON-RPC 2.0 notification | 22,592 B | 6,910 B | 1.8 Mbit/s | 1.66 Mbit/s |
| MessagePack (same maps) | 20,305 B | 6,667 B | 1.6 Mbit/s | 1.6 Mbit/s |
| JSON + deflate (≈ permessage-deflate) | 5,651 B | 2,481 B | 0.45 Mbit/s | 0.6 Mbit/s |
| Columnar quantized binary (u16 position in 100 m bubble, u16 yaw) | 2,606 B | 1,446 B | 0.21 Mbit/s | 0.35 Mbit/s |

Python `json.dumps` on a full frame takes 0.35 ms. **Bandwidth and CPU are not the issue on localhost.** The real issues are:
- JSON-RPC is a request/response protocol. Its notifications have no sequence numbers, baselines or keyframe and resync semantics.
- Floats travel as text.
- Each frame becomes per-entity GDScript Dictionaries.

Godot can decode columnar frames in bulk with `PackedByteArray.to_float32_array()`, `to_int32_array()` and `decode_u16()` [52], and it supports `COMPRESSION_ZSTD` [52].

**Game-networking practice to borrow** [41][42][43]:
- **Snapshot interpolation.** Render 2–3 packet intervals behind; Fiedler uses ~150 ms at 30 pps [41]. On localhost TCP, 100 ms at 20 Hz suffices (*inference*).
- **Snapshot compression.** Smallest-three quaternions (29 bits), bounded positions (~2 mm) and deltas against a baseline take 901 cubes at 60 Hz down from 17.4 Mbit/s [42].
- **Priority accumulator.** Fill a fixed per-packet byte budget by accumulated priority [43]. Use it for "hydrated" detail fields, not transforms.
- WebSocket runs over reliable, ordered TCP, so **previous-frame deltas plus a full frame every 1 s** are safe without UDP-style acked baselines (*inference*).

**SpatialOS lessons.** Verified: SpatialOS's Unreal GDK was archived 2024-08-29, and the Unity GDK repos are archived too [45]. *Training knowledge, not re-verified:* flagship SpatialOS games (Worlds Adrift, Mavericks) shut down or were cancelled around 2019; Unity's January 2019 terms-of-service dispute hit the platform; Improbable later left the games-platform business. *Inference:* spreading one world simulation across many server processes, with entity hand-off at boundaries, brought consistency, latency and cloud-cost problems that most designs did not need. Rehoboam should stay in **one process and one address space**, and scale with data-oriented layout and level-of-detail. Keep only the good idea, interest management.

### 6. God-console and time-machine UI

| Component | Option (version, license) | Fit / notes |
|---|---|---|
| Desktop shell | **Tauri** 2.12.4 stable, 3.0.0-alpha.5 (2026-10-14), Apache-2.0/MIT [55] | Uses the system webview: WebView2, WKWebView, or **WebKitGTK on Linux** [55]. *Unverified:* WebGL/WebGPU performance in WebKitGTK may lag Chromium. Test deck.gl on Linux before committing. |
| | **Electron** 44.7.2, MIT [56] | Bundled Chromium gives consistent WebGL2/WebGPU at the cost of size and RAM. Fine for a developer tool. |
| | **In-engine Godot UI** (Godot 4.7.2, Aug 2026 [50]) | One codebase with the 3D client, but no mature large-graph or data-viz libraries (*inference*). Use it for in-world overlays, not for analysis. |
| Map | **deck.gl** 9.4.0, MIT [57] | Use `OrthographicView` for a fictional world, binary or Arrow attributes for 1 M agents, and tile or bitmap layers for terrain (*inference* from deck.gl design). |
| | MapLibre GL JS 6.14.0, BSD-3 [58] | Built for Web Mercator geography. Only useful if you want vector-tile styling. |
| Causal graph | **Sigma.js** 4.0.3, MIT; "graphs of thousands of nodes", WebGL, graphology [59] | Primary renderer for neighborhoods of 1k–10k nodes. |
| | Cytoscape.js 3.34.4, MIT [60] | Richer analysis and layouts, but canvas-rendered. Fine up to a few thousand elements (*inference*). |
| | ELK.js 0.12.0, **EPL-2.0 OR GPL-3.0** [61]; @dagrejs/dagre 3.1.1, MIT [62]; Graphviz-WASM (@hpcc-js/wasm-graphviz 1.29.2, Apache-2.0) [63] | Layered (Sugiyama) layout for small explanation subgraphs of ≤500 nodes. Check that EPL-2.0 is acceptable. |
| Notebooks | **marimo** 0.25.1 (2026-10-01), Apache-2.0 [64] | Reactive notebooks stored as plain .py files. Pairs with DuckDB, Polars and Rerun [40]. |
| | Observable Framework (ISC, active) [65]; Evidence (MIT, active) [66] | Static report generators, not interactive debuggers. |
| TUI | **Textual** 8.2.8 (2026-06-30), MIT; `textual serve` serves to a browser [67] | Good for a headless ops dashboard (ticks/s, memory, pattern hits, logs, pause/seek). Not suitable for maps or large graphs. |
| ECS inspector | **Flecs** v4.1.6 (29 Jun) + Explorer (MIT, REST on :27750) [54] | Only if the sim uses Flecs. The Rust binding `flecs_ecs` 0.2.2 is low-adoption [54]. |

**Timeline UI.** *Inference:*
- **Overview plus detail.** A 1,000-year overview strip shows event density and drama per year, precomputed from aggregates, with a brush. Below it, a detail lane chart gives one lane per actor or faction, with story beats as spans.
- **Keyframes as markers.** The scrub cursor reads only the chronicle and aggregates, so it responds instantly. "Drop in here" triggers keyframe-plus-replay with a progress bar.
- **Causal DAG layout.** Events already carry ticks, so **x = time** comes for free. Layout reduces to assigning lanes (y) per actor; a full Sugiyama layout is unnecessary for big graphs.

---

## Recommendations for Rehoboam

### R1. Event schema (Arrow/Parquet-native, typed)

```
chronicle_events  -- partitioned by timeline_id / century / year
  event_id    UBIGINT   -- (tick << 20) | seq_in_tick : deterministic, ordered
  tick        UINTEGER  -- 1 tick = 1 day → 360k ticks / 1000 yr
  verb        USMALLINT -- FK verbs(verb_id, name, schema_version, default_weight)
  actor_id    UINTEGER
  target_id   UINTEGER NULL
  object_id   UINTEGER NULL   -- item / place / faction
  place_id    UINTEGER
  weight      UTINYINT  -- emission-time dramatic weight (0-255, not 0-10)
  salience    UTINYINT  -- retrospective, recomputed (see R2)
  secrecy     UTINYINT  -- public / witnessed / secret (needed for covert-mimic tropes)
  -- emission-time captured context (typed, ~8–16 cols, chosen from pattern needs):
  actor_title USMALLINT, actor_faction USMALLINT, target_title USMALLINT,
  affinity_at INTEGER8 (actor→target), actor_mood INTEGER8, ...
  causes      LIST<UBIGINT>   -- or causal_edges(child, parent, kind, strength)
  extra       BLOB NULL       -- msgpack/VARIANT overflow, rarely queried
state_intervals(timeline_id, entity_id, attr, value, valid_from, valid_to)
story_beats(beat_id, pattern_id, status, tick_start, tick_end, bindings, event_ids, score)
keyframes(timeline_id, tick, path, state_hash, bytes, schema_version)
input_journal(timeline_id, tick, seq, kind, payload)   -- god actions, LLM outputs, settings
state_hashes(timeline_id, tick, xxh3_64)                -- every N ticks
historical_actors(actor_id, born, died, promoted_at, name, lineage...)
```

**Commoners.** Do not log per-person events forever. Keep regional aggregates (region × metric × sim-month) and log individual events only for actors **promoted** to named status. Promotion is deterministic, and its seed is the entity id, so a hydrated commoner is the same person on every visit (*inference*).

### R2. Storage tiers and compaction

| Tier | Contents | Technology | Budget (1 M agents, 5k named) |
|---|---|---|---|
| **Hot** (sim RAM) | Live ECS state, matcher partial-match store, last ~30 sim-days of events as Arrow batches, spatial grid | In-process Rust or Python structures | ~0.5–2 GB (*inference*) |
| **Warm** (disk, append-only) | One immutable Parquet segment per sim-year per timeline (~250k events ≈ 2.7 MB), aggregates, intervals, story beats | Sim's writer thread writes Parquet. The console opens **DuckDB read-only over a glob**, so there is never a second writer [6]. | Chronicle ≈ 2.7–3.3 GB per 1,000 yr (measured basis); aggregates ≈ 0.2–2.5 GB |
| **Cold** | Per-century segments recompressed at zstd-19, plus low-salience archive partitions | Parquet | ~same or smaller |

**Compaction (replaces "delete weight < 7").** At each century boundary:
1. **Recompute salience.** An event's salience is the maximum of its own weight and a decayed share of the salience of any story beat or high-weight event it causally feeds (walk ancestors to depth k using `USING KEY`; measured at tens of ms).
2. **Move, don't delete.** Low-salience events go to an `archive` partition. Hot indexes and console views exclude it by default.
3. **Contract causal edges.** Edges that point into archived events are also mirrored as an edge to the nearest retained ancestor, so the visible graph stays connected.

Hard deletion should be an explicit, opt-in disk-pressure policy that runs only on cold archives and never touches events the sim reads back. If sim logic reads the chronicle (memories, grudges), any deletion must itself be a deterministic sim operation, or replays will diverge.

### R3. Sifting engine: a custom incremental matcher, validated by SQL

- **Pattern DSL.** Winnow-like and declarative: an ordered list of event clauses with variable bindings, time windows, `unless` clauses and state guards on captured columns. Store it as data (RON, TOML or S-expressions) so designers can add tropes without recompiling.
- **Runtime, in the sim process and in the sim's language:**
  - Index triggers by verb. Each pattern stage keeps a hash map from join keys to partial matches.
  - Expire partial matches with a timer wheel at window end. An `unless` event kills the partial matches that share its key.
  - On completion, write a `story_beat`, emit a JSON-RPC notification, and optionally **auto-pause**.
  - Track partial matches as "in progress" beats. This gives the prospective sifting Winnow describes [37].
- **Determinism makes lag harmless.** If the matcher runs on a worker thread and finishes L ticks late, the console pauses and seeks back to the match tick using keyframe plus at most L ticks of replay (*inference*).
- **Throughput target.** Named-actor events arrive at about 250k per sim-year, and commoner aggregates are not sifted per event. The measured Python matcher handles about 9 sim-years/s per pattern. A Rust version should reach tens of millions of events/s (*inference*).
- **Validation.** Every pattern ships with a DuckDB SQL twin. CI replays a recorded chronicle and asserts identical match sets, as this dossier did: 6,073 = 6,073.
- **Escalation path.** If designers need recursive or aggregate rules (transitive kin feuds, faction-level sentiment), embed **differential-dataflow** (MIT, 0.25.1) or the **dbsp** crate (MIT/Apache, very active, API churn) in-process. Do not adopt a DB server. **Avoid** CozoDB, DDlog, Kùzu upstream and Materialize. For offline research runs, Ascent (or Soufflé over Parquet exports) is fine.

### R4. Snapshots, replay and scrubbing (with numbers)

- **Determinism contract** (*inference*):
  - Use counter-based RNG keyed by (world seed, tick, system, entity).
  - Use fixed iteration order; never iterate hash maps in sim logic.
  - Make parallel reductions order-fixed.
  - Run the same binary on the same CPU architecture for bit-exact replays.
  - Journal every nondeterministic input. Lockstep games do the same [44].
  - Hash state every 360 ticks and compare on replay, as Factorio-style CRCs do (*training knowledge*).
- **Keyframe cadence and budgets** (synthetic sizing of 0.195 GB per keyframe at 1 M agents, 0.02 GB at 100k):
  - **Permanent keyframe every 10 sim-years:** 100 keyframes ≈ **20 GB** at 1 M agents, ≈ **2 GB** at 100k. A capture costs one memcpy of ~350 MB (~50–100 ms stall, *inference*) plus ~0.5 s of background zstd-3 (measured throughput).
  - **Session cache:** yearly keyframes within ±10 years of the cursor, built lazily by background re-simulation and kept in an LRU (~4 GB at 1 M agents).
  - **RAM ring:** monthly deltas for the last ~5 sim-years (~2 MB/tick-delta measured, so keep monthly not per-tick: ~60 × ~20 MB ≈ 1.2 GB, *inference*).
  - **Seek cost** = (distance to nearest keyframe) × (sim speed). At an assumed 10 s per sim-year for 1 M agents, a decade cadence gives a worst-case ~100 s seek and the yearly cache gives ≤10 s. Measure the real sim speed and tune.
- **Branching ("what if").** Fork = new `timeline_id` + parent timeline + fork tick. The chronicle partitions share the prefix copy-on-write.

### R5. Client protocol

- **One localhost WebSocket with two channels:**
  - **Text frames carry JSON-RPC 2.0** for commands, queries, subscriptions (`bubble.subscribe {center, radius}`), inspector fetches, and notifications for story beats and auto-pause.
  - **Binary frames carry state:** `[magic, schema_ver, seq, tick, flags(full|delta)]` followed by **columnar arrays**: ids u32, quantized positions as 3×u16 within the bubble, yaw u16, animation u8, and changed-field bitmasks. Send a full frame every 1 s and previous-frame deltas otherwise. Measured: ~2.6 KB full and ~1.4 KB delta for 200 entities.
  - Raise Godot's `inbound_buffer_size` above 64 KB for large bubbles [51].
- **Rates and interest management:**
  - Transforms at 20 Hz, rendered ~100 ms behind with interpolation [41].
  - Area of interest on a 25 m grid. Enter at 100 m, leave at 120 m (hysteresis).
  - Hydrated detail (names, mood, action text) on change, budgeted by a priority accumulator [43].
- **Server and future transport.** If the sim is Rust, serve from the sim (axum or tokio-tungstenite) and skip FastAPI. If the sim is Python, keep socket encoding out of the sim thread or process. Keep the frame format transport-agnostic so a later **GDExtension in-process mode** (gdext [53]) can pass the same bytes through memory.

### R6. Console and debug stack

- **God-console:** a TypeScript + Vite web app served by the sim on localhost, using deck.gl, Sigma.js, a custom canvas timeline and DuckDB on the server side.
  - Develop in a Chromium browser. Package with **Tauri 2.x** on Windows and macOS. On Linux, use Electron or the browser until WebKitGTK WebGL performance is proven.
  - Send map and timeline data to the browser as Arrow IPC.
- **Debugging:** Rerun 0.39 for spatial and time-series inspection of sampled agents and systems; a Textual TUI for headless runs; marimo + DuckDB + Polars for analysis. Use the Flecs explorer only if Flecs is chosen.
- **Godot in-engine UI** only for in-world overlays (selection, labels, beat callouts).

---

## Risks & unknowns

1. **The sim's language and ECS are undecided.** Python, Rust and C++/Flecs each change the matcher's throughput, determinism guarantees, server choice and debug tooling. This is the biggest open dependency.
2. **Determinism in practice.** Multithreaded systems, floating point across CPUs and any LLM-in-the-loop output can break replay. Journal inputs and hash state, or the time machine silently lies.
3. **Snapshot sizes are synthetic.** Random fields understate compressibility, while unmodeled memories, beliefs and inventories may dominate state size. Measure a real 1 M-agent keyframe early.
4. **DuckDB churn.** 2.0 brings a new default storage format and `USING KEY` semantic changes [3][5]. EOL dates of 1.5 (Nov 1) and 1.4 LTS (Nov 17, 2026) come from a search snippet [9]. Mitigation: Parquet and Arrow are the system of record.
5. **Fork and engine sustainability.** The LadybugDB fork is about one year old. DBSP moves fast (366 minor releases). Polars 2.0 is 9 days old.
6. **Platform webviews.** Tauri on Linux uses WebKitGTK [55]; GPU-heavy deck.gl performance there is untested.
7. **Benchmark limits.** Numbers come from a 4-vCPU container, Python drivers, SQLite 3.45.1, DuckDB 1.5.6, a single synthetic data distribution and warm caches. Treat them as order-of-magnitude.
8. **Verification gaps.** duckdb.org, Wikipedia and factorio.com could not be fetched, and the web-search budget ran out mid-session. These rest on training knowledge: SpatialOS history, Factorio and Rocket League replay internals, the Soufflé license, and WebTransport and FlatBuffers support in Godot. The Kùzu→Apple acquisition is reported by third parties only [14][68].

---

## Appendix: benchmark method (reproducible)

- **Setup.** DuckDB 1.5.6, PyArrow 26, Python 3.13, 4 vCPU and 15 GB RAM. Synthetic data: 10 M events at 694 events/tick (250k/yr at 360 ticks/yr, so ~40 sim-years); actors drawn from `floor(5000·u²)` (skewed); 60 verbs drawn from `floor(60·u³)`; weight `floor(11·u³)` (P(weight<7) ≈ 0.86); 0–2 causes drawn from the preceding 5k–50k events.
- **Betrayal** = ally(A,B) at t1, attack(A,B) at t2 with t1 < t2 ≤ t1+3600, and no reconcile(A,B) in (t1, t2). Written as a DuckDB self-join with `NOT EXISTS`, the same query in SQLite with indexes, and a pure-Python streaming matcher holding per-pair deques. All produce **6,073** matches.
- **Regicide** = kill events joined to `titles` intervals (263 reigns).
- **Snapshot.** 1 M agents and 20 M edges built in DuckDB, written as Arrow IPC, compressed with zstd at levels 1, 3, 9 and 19.
- **Wire.** 200 entities encoded as a JSON-RPC notification, MessagePack, deflate and quantized columnar binary.
- **R*Tree.** 1 M points in an in-memory SQLite R*Tree, with 100k updates and 1k box queries.

---

## Sources

1. https://duckdb.org/2026/09/28/announcing-duckdb-156 — DuckDB 1.5.6 announcement (from search snippet; page DNS-blocked).
2. https://github.com/duckdb/duckdb/releases — Release list: v1.5.6 on 28 Sep, v1.4.5 LTS on 17 Jun.
3. https://duckdb.org/2026/08/17/duckdb-20-highlights — DuckDB 2.0 preview: Quack, triggers, VARIANT, new storage format, fall 2026 (search snippet).
4. https://duckdb.org/2025/05/23/using-key.html — USING KEY recursive CTEs, May 2025 (search snippet).
5. https://duckdb.org/2026/08/25/how-duckdb-runs-recursive-ctes-faster — USING KEY semantic change planned for v2.0 (search snippet).
6. https://duckdb.org/docs/current/connect/concurrency — Single read-write process with MVCC; multi-process read-only; Quack/DuckLake for multi-writer (search snippet).
7. https://crates.io/crates/duckdb — Rust crate 1.10506.0 (2026-09-30), MIT; Arrow appender and vtab features.
8. https://www.duckdb.org/2026/07/31/asynchronous-io — Async I/O becomes the default in 2.0 (search snippet).
9. https://endoflife.date/duckdb — EOL dates for 1.5 and 1.4 LTS (search snippet).
10. https://pypi.org/project/polars/ — Polars 2.0.0 uploaded 2026-10-06, MIT.
11. https://pypi.org/project/lancedb/ — LanceDB 0.40.0, Apache-2.0.
12. https://github.com/kuzudb/kuzu — Archived 2025-10-10; last release v0.11.3; "working on something new".
13. https://github.com/LadybugDB/ladybug — Kùzu fork, MIT; releases v0.21.2 (1 Oct); crates.io `lbug` 0.21.2 (2026-10-01).
14. https://dbdb.io/db/ladybugdb — Describes the fork after a reported Apple acquisition (third party).
15. https://github.com/cozodb/cozo/commits/main — Last commits dated Dec 4, 2024.
16. https://dbdb.io/db/cozodb — Lists CozoDB as abandoned since 2024.
17. https://github.com/souffle-lang/souffle/releases/tag/2.5 — Soufflé 2.5 (24 Mar; Fedora 41 / Ubuntu 24 packages); commits through 2026-10-02.
18. https://github.com/s-arash/ascent — Ascent features (lattices, aggregates, rayon, BYODS); crates.io 0.8.1 (2026-08-29), MIT.
19. https://github.com/ekzhang/crepe — crepe proc-macro Datalog; crates.io 0.2.0 (2025-12-14).
20. https://crates.io/crates/datafrog — datafrog 2.0.1 (2019), Apache-2.0/MIT.
21. https://conf.researchr.org/details/CC-2022/CC-2022-research-papers/8/Seamless-Deductive-Inference-via-Macros — Ascent paper; comparable to Soufflé and datafrog.
22. https://github.com/TimelyDataflow/differential-dataflow — MIT; README latency and throughput numbers; crates.io 0.25.1 (2026-07-15).
23. https://github.com/vmware-archive/differential-datalog — DDlog repository archived (read-only banner).
24. https://github.com/feldera/feldera/releases — Feldera v0.366.0 (15 Oct); crates.io `dbsp` 0.366.0, MIT OR Apache-2.0.
25. https://github.com/feldera/feldera/tree/main/sql-to-dbsp-compiler — SQL compiler in Java/Calcite; needs Java 19–21 + Maven; emits Rust.
26. https://arxiv.org/abs/2203.16684 — DBSP: automatic IVM for rich query languages (VLDB 2023).
27. https://github.com/MaterializeInc/materialize — BSL 1.1 → Apache-2.0 after 4 years; Community Edition limits; also https://materialize.com/changelog/2025-03-11-materialize-self-managed-and-community-editions/
28. https://www.xtdb.com/blog/launching-xtdb-v2 — XTDB v2 stable (2025-06-12), MPL-2.0, bitemporal SQL; https://github.com/xtdb/xtdb/releases — 2.2 betas.
29. https://github.com/terminusdb/terminusdb/releases — v12.0.8 published by a DFRNT account; https://en.wikipedia.org/wiki/TerminusDB — DFRNT maintenance since 2025 (search snippet).
30. https://github.com/knowsys/nemo — Nemo, Apache-2.0/MIT, "unstable"; https://arxiv.org/pdf/2308.15897 — Nemo paper.
31. https://www.uktech.news/tech-hubs/south-east/oxford/samsung-acquire-oxford-knowledge-graph-spinout-20240718 — Samsung acquires RDFox maker (Jul 2024).
32. https://arxiv.org/abs/2511.00865 — FlowLog: Datalog on differential dataflow; benchmarks vs Soufflé (search snippet).
33. https://duckdb.org/science/duckpgq — DuckPGQ SQL/PGQ extension; described as WIP research (search snippet).
34. https://github.com/tonsky/datascript — DataScript 1.8.1, EPL-1.0, "Stable".
35. https://github.com/replikativ/datahike — Datahike, EPL-1.0; Python/C bindings in beta.
36. https://github.com/mkremins/felt — Felt story sifter; requires DataScript; Datalog-style patterns.
37. https://ojs.aaai.org/index.php/AIIDE/article/view/18903 — Winnow: incremental story sifting DSL (AIIDE 2021); https://github.com/mkremins/winnow — `unless-event` example.
38. https://crates.io/crates/rkyv — rkyv 0.8.18 (2026-08-05), MIT.
39. https://static.crates.io/readmes/bincode/bincode-3.0.0.html — bincode development ceased; https://github.com/rustsec/advisory-db/blob/main/crates/bincode/RUSTSEC-2025-0141.md — unmaintained advisory (2025-12-16), with alternatives.
40. https://github.com/rerun-io/rerun — MIT/Apache-2.0, Python/Rust/C++ SDKs, open-core; https://github.com/rerun-io/rerun/releases/tag/0.39.0 — marimo embedding and state timeline; crates.io `rerun` 0.38.1 (2026-09-16).
41. https://gafferongames.com/post/snapshot_interpolation/ — Interpolation buffer sizing (read via the GitHub source repo).
42. https://gafferongames.com/post/snapshot_compression/ — Smallest-three, bounded positions, delta vs baseline (17.4 Mbit/s starting point).
43. https://gafferongames.com/post/state_synchronization/ — Priority accumulator with a per-packet byte budget.
44. https://gafferongames.com/post/deterministic_lockstep/ — Lockstep needs bit-exact determinism; bandwidth scales with input.
45. https://github.com/spatialos/UnrealGDK — Archived 2024-08-29; https://github.com/spatialos — Unity GDK repos archived.
46. https://github.com/Blizzard/s2protocol — SC2 replays: game/message/tracker events; build-specific decoding.
47. https://github.com/rr-debugger/rr — Record/replay with reverse execution; Linux; needs perf counters; sponsored by Pernosco.
48. https://github.com/replayio/devtools — Replay.io devtools repo, still active.
49. https://github.com/sqlite/sqlite/blob/master/ext/rtree/README — SQLite R-tree virtual table; 32-bit float coordinates.
50. https://github.com/godotengine/godot/releases — Godot 4.7.2-stable (18 Aug 2026).
51. https://github.com/godotengine/godot/blob/master/modules/websocket/doc_classes/WebSocketPeer.xml — Binary frames; 65,535-byte default buffers; 4,096 queued packets.
52. https://github.com/godotengine/godot/blob/master/doc/classes/PackedByteArray.xml — `to_float32_array`/`decode_u16`; FileAccess.xml lists COMPRESSION_ZSTD.
53. https://github.com/godot-rust/gdext — godot-rust: "usable", MPL-2.0; crates.io `godot` 0.5.5 (2026-08-09).
54. https://github.com/SanderMertens/flecs/releases — Flecs v4.1.6; https://github.com/flecs-hub/explorer — REST :27750, MIT; crates.io `flecs_ecs` 0.2.2.
55. https://github.com/tauri-apps/tauri — System webviews (WebKitGTK on Linux); crates.io `tauri` 2.12.4 stable, 3.0.0-alpha.5.
56. https://www.npmjs.com/package/electron — Electron 44.7.2, MIT.
57. https://www.npmjs.com/package/deck.gl — deck.gl 9.4.0, MIT.
58. https://www.npmjs.com/package/maplibre-gl — MapLibre GL JS 6.14.0, BSD-3-Clause.
59. https://www.npmjs.com/package/sigma — Sigma.js 4.0.3, MIT; WebGL, thousands of nodes.
60. https://www.npmjs.com/package/cytoscape — Cytoscape.js 3.34.4, MIT.
61. https://www.npmjs.com/package/elkjs — elkjs 0.12.0, EPL-2.0 OR GPL-3.0.
62. https://www.npmjs.com/package/@dagrejs/dagre — dagre 3.1.1, MIT.
63. https://www.npmjs.com/package/@hpcc-js/wasm-graphviz — Graphviz in WASM 1.29.2, Apache-2.0.
64. https://pypi.org/project/marimo/ — marimo 0.25.1 (2026-10-01), Apache-2.0.
65. https://github.com/observablehq/framework — Observable Framework, ISC, active.
66. https://github.com/evidence-dev/evidence — Evidence, MIT, active.
67. https://github.com/textualize/textual — Textual, MIT, `textual serve`; PyPI 8.2.8 (2026-06-30).
68. https://oneuptime.com/blog/post/2026-08-12-kuzu-archived-pin-0-11-3-fork-or-migrate/view — Third-party guide: pin Kùzu 0.11.3, fork or migrate.
69. https://duckdb.org/science/bamberg-using-key-sigmod — SIGMOD 2025 paper; USING KEY shipped in DuckDB 1.3.0 (search snippet).
