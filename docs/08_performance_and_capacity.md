# 08 — Performance Budgets and Capacity Model

*Status: Draft v1.4 (2026-10-16).*
*v1.1 changes: re-baselined after red-team finding RT-M8. 1M persons now costs ≈ 25–60 ms per normal month and ≈ 60–130 ms per crisis month, with the arithmetic shown; a serial-fraction budget is added; PERF-1 (P3) and PERF-2 are provisional; Foresight at 1M is a batch job; memory, keyframe and storage estimates include the life-course log, bounded T1 belief stores, knowledge fronts and the keyframe LRU; burn-in is split into the liveness gate and certification; speed-ups no longer re-baseline the performance gate automatically.*
*v1.2 (2026-10-16): the deliberation-consult cost line is removed (ADR-0034, D27).*
*v1.4 (2026-10-16): §1.4's MOP-1 line follows G1a and D33: G1a measures the 10k dynastic world, and M1 F1 joins at G1b.*
*v1.3 (2026-10-16), ADR-0035: the default saga world is meso (≤ 50k persons, ≤ 2,000 named actors), and MOP-1 is gated there at G1b. The 100k and 1M budgets below stand for the optional scale-up (P3.10, Later). Media costs (images, TTS) are outside the engine and estimated in `05.11` §11.*

**Where the numbers come from**
- **[M]** Measured in research sandboxes: 4-vCPU Xeon at 2.1 GHz (R01, R03, R10).
- **[E]** Engineering estimates derived from [M]. These must be re-measured in P0/P1. Regression gates in CI enforce them once measured.
- **[P]** Provisional targets. Each is replaced by a measured value at the gate named, and the change is logged in `decisions-log.md`.
- **Reference desktop:** 8 performance cores at ~4.5 GHz (the 1M estimates are quoted for 8–16 cores), 64 GB dual-channel RAM, RTX 4070/4090-class GPU.
- **Reference cluster node:** 64+ cores, plus GPU nodes for LLM and TTS.
- **Development laptop (D6, 2026-10-16):** ThinkPad P51, 4 cores / 8 threads at 2.9–3.9 GHz (2017), 32 GB. Expect roughly **3× the reference desktop's wall times** (an estimate from core count and clock; SPR-009 measures it). Budgets here stay quoted on the reference desktop; the laptop checks correctness and instruction-count regressions, not these wall-clock targets (GOV-008 §1).
- **Benchmark VM `adjutant` (2026-10-16):** Google Cloud `c3d-standard-16`, 8 AMD Genoa cores, 64 GB. The closest match to the reference desktop; it carries the wall-clock benchmarks, on demand only (RUN-002). Its ratio to the reference is measured in SPR-009.

---

## 1. Macro step budget at 1M persons (Rust, reference desktop, per sim-month)

The v1.0 figure (≈ 12 ms per month) counted one hazard draw per person and little else. It left out the scheduled-outcome timers (ADR-0025), the life-course log (ADR-0026), knowledge fronts, T1 belief updates and the serial path. This re-baseline counts them.

### 1.1 A normal month

**Assumptions.** 1M living persons, 300–2,000 regions, ~5k named actors (T1), ~21k named events per month. Seven worker threads run the simulation at ≈ 80% parallel efficiency (≈ 5.6 effective cores). The eighth core runs the live sifter and the Chronicle Writer.

| Work item | Basis | Parallel (wall ms) | Serial (ms) |
|---|---|---|---|
| **Per-person monthly work**: hazards for death, fertility, partnering, migration, illness and mobility; 4–6 keyed draws; entitlement and density modifiers; sampling of intra-pulse outcome times | [M] 1M Bernoulli deaths in 8.3 ms, vectorised NumPy (R03); [E] 60–130 ns per person-month → 60–130 CPU-ms | 11–23 | — |
| Pop and regional aggregation (region × class × faith; morale, belief shares) | [E] 10–20 ns per person | 2–3.5 | — |
| **Scheduled-outcome timers** (ADR-0025): ≈ 10⁴ per month (≈ 3k deaths, ≈ 3.5k births and conceptions, plus marriages, migrations and illness onsets). Heap insert and fire; lifecycle commands applied in `(phase, PersonId)` order | [E] 0.15–0.3 µs per command | — | 1.5–4 |
| Raster fields: 8 layers of 512², 5-point stencil | [M] 0.65 ms on 4 threads (R03) | 0.7 | — |
| Region systems: economy, polity, agriculture; S1/S2 diffusion on pops | [E] 300–2,000 regions | 1–2 | — |
| **Agenda scoring**: ~5k due decisions against the snapshot taken at the start of the `Agenda` phase (`05.1` §4.3) | [M] 0.81 ms for a 5k toy (R03); [E] 1–3 µs per decision with belief reads, opinion sums and HTN | 1–3 | — |
| **Agenda application**: keyed permutation order, invalidation check, at most one re-score per actor | [E] 0.2–0.6 µs per action | — | 1–3 |
| T1 belief, knowledge and suspicion updates (bounded stores, `05.3` §4) | [E] | 1–3 | — |
| Knowledge fronts: one shortest-path pass over the region graph per notable public event (`05.2` §4.1) | [E] 100–300 events × 20–60 µs | 0.5–2.5 | — |
| Event emission: roles, anchored snapshots and causal edges built in per-thread buffers, then merged in `(tick, phase, seq)` order and assigned EventIds | [E] 0.3–0.6 µs per event to build | 1–2.5 | 0.5–1.5 |
| Life-course log and journal appends | [E] | — | 0.3–0.8 |
| Plane hashes: leaf hashing (yearly in release, amortised; monthly in CI), then the Merkle merge | [E] | 0.1–1.5 | ≈ 0.1 |
| Scheduler: 11 phase barriers, agenda heap, lifecycle-queue hand-off | [E] 20–50 µs per barrier | — | 0.5–1 |
| Invariants: debug/CI builds only | — | (0 in release) | — |
| **Subtotal** | | **≈ 18–42** | **≈ 4–10** |

**Total: ≈ 22–52 ms, or ≈ 25–60 ms per normal month with ≈ 15% headroom.**

- **Serial share** of the subtotal: 4 of 22 ms to 10 of 52 ms, i.e. **≈ 18–20%** (§1.3).
- **Live sifter.** 21k events at ≥ 10⁷ events/s per core is ≈ 2 ms [E] on its own core. It is off the critical path unless it lags (`05.4` §2.3 item 10). Its budget is PERF-3. Indexed lookup is mandatory: Winnow's scan of every partial costs about 0.9 ms per partial per event (R12 §19).
- **8 vs 16 cores.** More cores shrink only the parallel column. On a dual-channel desktop the ledger passes become memory-bandwidth-bound, and the serial column does not shrink at all. A 16-core desktop is therefore ≈ 1.3–1.5× faster, not 2×. That is why the band is quoted for 8–16 cores: the uncertainty in the [E] constants is larger than the core-count effect.

### 1.2 A crisis month

A crisis month has an active epidemic, a campaign or siege, or a succession war in play.

| Extra work in a crisis month | Parallel (wall ms) | Serial (ms) |
|---|---|---|
| 30 daily sub-steps of M5 and M8 in active regions: SEIRS, testing, fear, per-person status assignment; armies and sieges | 15–30 | 1.5–4 (30 extra instants of phase barriers) |
| 3–5× scheduled outcomes (≈ 30–50k: infections, deaths, flight) | 1–2 | 3–8 |
| 2–3× agenda decisions (interrupts) | 2–5 | 1–3 |
| 2–3× belief, fear and testimony updates | 2–4 | — |
| 2–3× events and knowledge fronts | 2–4 | 0.5–1.5 |
| **Extra** | **≈ 22–45** | **≈ 6–16** |

Crisis subtotal: ≈ 40–87 ms parallel plus ≈ 10–26 ms serial, i.e. ≈ 50–113 ms. **With ≈ 15% headroom: ≈ 60–130 ms per crisis month.** The serial share is ≈ 20–23%.

### 1.3 Serial-fraction budget (Amdahl)

**Budget: serial work ≤ 20% of tick wall time on the reference desktop, in normal and crisis months.** It is measured by `bench/serial_fraction` from P1a (§9).

| Serial step | Why it is serial |
|---|---|
| Scheduler: phase barriers, agenda heap pops | Phase order is part of the determinism contract (`05.1` §4.2) |
| Lifecycle-command application | Births, deaths, migrations and transfers apply in `(phase, PersonId)` order through one path (`05.1` §2 rule 2) |
| Agenda application | Actions apply in keyed permutation order, and each may invalidate the next (`05.1` §4.3) |
| Event emission ordering | EventIds follow emission order `(tick, phase, seq)`, so per-thread buffers merge in that order (ADR-0025) |
| Journal and life-course appends | Append-only and ordered |
| Hash merge | The Merkle root over chunk hashes is combined in a fixed order (`05.1` §6.6) |

**Why 20%.** If 20% of the 8-core tick is serial, a 16-core machine is at most 1.67× faster per timeline, a 64-core machine at most 3.3×, and no machine more than 5×. Past that point, extra cores go to more timelines (Foresight, ensembles; ADR-0020), not to a faster tick.

**Where it binds.** Normal months sit at ≈ 18–20%; crisis months at ≈ 20–23%. If `bench/serial_fraction` confirms the crisis overshoot in P1a, the levers are, in order:
1. Cheaper lifecycle commands: batch the life-course appends and move aggregate updates into the parallel pass.
2. Apply region-local lifecycle commands per region in parallel, each region in `PersonId` order. This needs a determinism review (ADR-0006) before it lands.

Adding cores is not a lever.

### 1.4 Implications

- **1,000 years at 1M** is 12,000 months. With 85% normal and 15% crisis months, that is 12,000 × (0.85 × 25 + 0.15 × 60) ms ≈ **6 min** at the low end and 12,000 × (0.85 × 60 + 0.15 × 130) ms ≈ **14 min** at the high end.
- **PERF-1 at P3 [P]: ≤ 15 min per 1,000 years at 1M** on the reference desktop. It replaces v1.0's ≤ 10 min. It is confirmed or revised at G3b.
- **PERF-2 [P]: ≤ 60 ms per normal month and ≤ 130 ms per crisis month at 1M, with serial share ≤ 20%.** v1.0's ≤ 15 ms is withdrawn.
  - The binding value is set at **G1b** from P1 measurements on a synthetic 1M world (`bench/ledger_month_1M`, `bench/timers_lifecycle_1M` and `bench/serial_fraction`; WP P1.14).
  - It is logged in the decisions log.
- **At 100k persons** with ~1k named actors:
  - Ledger-proportional terms shrink ~10× and actor-proportional terms ~5×. Raster and region terms and the phase barriers do not shrink.
  - Single-core cost is ≈ 25 CPU-ms per month [E] (range 16–43), i.e. **≈ 5 CPU-min per 1,000 simulated years**. Liveness gates, certification and ensembles use this figure (§2, §4).
  - On 8 cores it is ≈ 4–11 ms per month, or **≈ 1–2.5 min per 1,000 years** including crisis months (central estimate ≈ 1.5 min).
  - MOP-1 (≤ 2 min, gated at G1b on the meso kingdom world) is reachable at 100k but not certain. G1a measures the 10k dynastic world with M1 F0, M2, M7 and `T1.minds` (≤ 60 s, `11` G1a); G1b measures every P1 system, M1 F1 included (D33).
- **Risk to G1b.** If a 100k-person world carries the full 5k named actors, actor-proportional terms add ≈ 3–7 ms per month (≈ 2–4 min per 1,000 years), which misses MOP-1. The T1 count must scale with population (~1k at 100k).
- Release builds hash yearly by default. CI and debug builds hash monthly; dirty tracking keeps that cheap.

**Same-instant decision evaluation** (`05.1` §4.3).
1. Named-actor decisions due in the same instant are *scored* in parallel against the snapshot taken at the start of the `Agenda` phase.
2. They are then *applied* serially in a keyed permutation order `H(seed, salt, t, entity_id)`, which avoids the founder and low-ID bias of `entity_id` order.
3. An action is invalidated only when an earlier write set intersects its read set *and* its precondition now fails. That actor re-scores once; if the new choice is also invalid, it defers to its next poll.
4. Worst-case serial work is therefore one re-score per actor per instant. The typical invalidation rate is expected below 5% [E].

Semantics: "simultaneous deliberation, ordered action".

## 2. Timeline operations

| Operation | Calculation | Target |
|---|---|---|
| Scrub (chronicle and aggregates only) | DuckDB read-only views over Parquet; overview strip precomputed per year | < 100 ms (MOP-3) |
| **Drop in, typical** (passive, read-only) | Nearest keyframe in the RAM tier of the LRU + replay of < 1 month (≤ 60 ms at 1M, ≤ 130 ms in a crisis month) + frame publish. The Time-Machine captures a copy-on-write keyframe at every auto-pause, and each drop-in leaves one, so the nearest keyframe is usually under a month away. | **≈ 0.3 s at 1M**; < 0.1 s at 100k |
| Drop in, warm (yearly keyframe in the on-disk LRU) | Decompress (≈ 0.2–0.5 s at 1M) + ≤ 12 months of replay | ≈ 0.1–0.2 s at 100k; ≈ 0.5–2 s at 1M |
| Drop in, cold (10-year permanent keyframes only) | ≤ 120 months of replay | ≈ 0.5–2.5 s at 100k; ≈ 3–7 s at 1M, up to ≈ 16 s in a crisis-heavy decade |
| **MOP-4 worst case** | | **≤ 10 s.** Always met at 100k. At 1M it is met once the yearly LRU around the cursor has filled (a background job that starts when the cursor stops), or with a 5-year permanent cadence (DET-8 is configurable; it doubles keyframe storage) |
| Keyframe capture | Copy-on-write snapshot of plane-A tables (≈ 0.35–1.1 GB raw at 1M; §5) + background zstd-3 ([M] ~0.65 GB/s on 4 threads) | Sim stall < 100 ms; 0.5–2 s in the background |
| Fork | New timeline ID + keyframe reference + journal tail. Acting in the past = drop in + fork (ADR-0025) | < 1 s after the drop-in |
| **Liveness gate**: 200 years × 8 seeds at 100k (`05.5` §4.2) | Each seed single-threaded on its own core: 2,400 months × ≈ 25 ms ≈ 1 CPU-min | **≈ 60 s wall on 8 cores** |
| Liveness gate at 1M | 2,400 months × ≈ 105–245 CPU-ms per seed; 8 concurrent 1M worlds contend for memory bandwidth | ≈ 6–13 min wall: Genesis at 1M is a batch step |
| Spin-up (optional): 100–200 years at 100k | Multi-threaded, 1,200–2,400 months × ≈ 5–8 ms | ≈ 10–20 s |
| **Certification**: 1,000 years × 32 seeds at 100k (background) | 32 × ≈ 5 CPU-min ≈ 160 CPU-min | **≈ 20 min wall on 8 cores**; ≈ 3 min on a 64-core node |
| Calibration repair (only when the gate fails) | At most 30 gate runs (`05.5` §4.2) | ≤ 30 min wall |

## 3. Foresight (Monte-Carlo) capacity

Rollouts are independent timelines, so they parallelise without limit (ADR-0020). **Interactive Foresight targets 100k-class worlds. At 1M, Foresight is a batch job** (`05.3` §7.3).

| Scenario | Per-rollout cost | N | Wall time (desktop, 8 cores) | Wall time (cluster node, 64 cores) |
|---|---|---|---|---|
| 25-year horizon, 100k, interactive default; reduced cost outside the predicate's regions | 300 months × ≈ 10 ms (1 core) ≈ 3 s | 64 | ≈ 25 s | ≈ 3 s |
| 25-year horizon, 100k, full fidelity, single-threaded rollouts | 300 months × ≈ 25–35 ms ≈ 7.5–10.5 s | 256 | ≈ 4–6 min | ≈ 30–45 s |
| Same, reduced cost outside the predicate's regions | ≈ 3–4 s | 256 | ≈ 1.6–2.1 min | ≈ 12–16 s |
| **50-year horizon, 1M (batch)** | 600 months × ≈ 45–85 ms on 8 threads ≈ 27–51 s, plus ≈ 1–2 s for fork, keyframe load and re-sampling | **64** | **≈ 30–55 min** (one rollout at a time) | **≈ 4–7 min** (8 rollouts at a time × 8 cores each) |
| Intervention A/B with common random numbers (paired) | 2× the above | 128 pairs give a CI similar to ~1,000 independent runs (variance reduction; [E], to measure) | — | — |

**The 1M batch row** assumes N = 64 rollouts × 50 years. The blended month cost of ≈ 45–85 ms [E] reflects that prophecies are asked in troubled periods, so roughly half the horizon is crisis months; reduced cost outside the predicate's scope offsets part of that. Each concurrent 1M rollout holds its own world (≈ 1–2 GB, §5).

**Precision.** At P ≈ 0.3 the Wilson 95% half-width is about ±0.11 at N = 64, ±0.056 at N = 256 (OS-5's ±0.06) and ±0.028 at N = 1,024. More compute buys tighter forecasts: at 1M, N = 256 takes ≈ 2–4 h on the desktop or ≈ 15–30 min on a 64-core node. For a decision between interventions, a CRN-paired A/B at N = 64 is a better use of the same compute than an unpaired N = 256. This is the main horizontal-scaling workload (ADR-0020).

## 4. Calibration and validation ensembles

Per-run costs use ≈ 5 CPU-min per 1,000 years at 100k (≈ 1 CPU-min per 200 years) from §1.4.

| Job | Size | CPU cost [E] | Wall time, desktop (8 cores) | Wall time, 64-core node |
|---|---|---|---|---|
| Liveness gate (`05.5` §4.2) | 8 seeds × 200 years at 100k | ≈ 8 CPU-min | ≈ 60 s | — |
| Certification, or the stylized-fact ensemble per reference world | 32 seeds × 1,000 years at 100k | ≈ 160 CPU-min | ≈ 20 min | ≈ 3 min |
| CRN-paired ensemble on a C3 PR (`13` §4.2) | 2 × 32 seeds × 1,000 years at 100k | ≈ 320 CPU-min | ≈ 40 min | ≈ 5 min |
| Certification at 1M (P3+) | 32 seeds × 1,000 years at 1M | ≈ 13–32 CPU-h | ≈ 2–4 h | ≈ 15–30 min (32–64 GB RAM) |
| Morris screening | ~20 params × 10 trajectories × 200 years ≈ 210 runs | ≈ 3.5 CPU-h | ≈ 26 min | ≈ 3.5 min |
| Sobol indices (selected params) | ~2,000 runs × 200 years | ≈ 33 CPU-h | ≈ 4 h | ≈ 31 min |
| ABC / simulated-moments calibration | 10⁴–10⁵ short runs (50–100 years) | ≈ 2–35 CPU-days | — | hours to a day across nodes |

## 5. Memory budget (1M persons, sim host)

| Component | Size | Notes |
|---|---|---|
| Ledger (living only), 48–72 B/person | ≤ 72 MB | `05.1` §5 |
| ID collision index (every `PersonId` ever issued, ≈ 4×10⁷ per 1,000 years) | ≈ 50–350 MB | A Bloom filter in RAM with on-disk sorted runs, or a full in-RAM set (ADR-0026) |
| Life-course log append buffers (current year) | < 50 MB | The full log is on disk (§6) and paged on demand |
| Rasters (16 layers at 512², f32) | 16 MB | 1024² gives 64 MB |
| Pops, regions, polities, markets, pop belief-share templates | 50–150 MB | Templates are garbage-collected (`05.2` §4.1) |
| Named actors (1k–5k × 1–4 KB) + social graph (≤ 10⁶ edges × ~16 B) | < 40 MB | |
| **T1 belief stores** (bounded; named actors only) | 50–600 MB [E] | 1k–5k actors × 0.5k–2k live beliefs × ~48 B, plus ToM and sparse suspicion (+10–20%). The bound is a design rule, not a hope. Measured in P3a (`bench/belief_store_5k`) |
| Per-actor knowledge index (recent decades) | 50–200 MB | Rows for witnessed, faction, secret and testimony hops only; older rows are on disk |
| **Knowledge fronts in flight** | 50–100 MB | Public events whose fronts are still spreading (≈ 3–6k events × regions reached × 8 B). Finished fronts are on disk |
| Agenda: decisions and timers | < 10 MB | 10⁴–10⁵ pending entries |
| Sifter partial matches (plane C) | < 200 MB | Bounded by declared windows and match semantics (`05.4` §2.3). Per pattern, `latest` + `clear_binding` needs one open partial per join key per stage; `all`, `earliest` and `keep` hold every stage-1 event in the window per key, and fork-all patterns of three or more clauses multiply across stages. Clustered fixture: peak 1,566 partials (`latest`) vs 4,492 (`all`); a windowless pool grew to 21,269 with no plateau (R12 §19; V-SIFT-13) |
| Hot event window (30 sim-days) and Arrow builders | < 50 MB | |
| **Sim host, without the keyframe cache** | **≈ 0.5–2 GB** | |
| **Keyframe LRU, RAM tier** (2–4 decompressed keyframes near the cursor) | ≈ 1–4 GB | Holds auto-pause and recent drop-in keyframes (§2) |
| **Sim host total** | **≈ 1.5–6 GB** | A 64 GB desktop also fits 8 concurrent 1M rollouts (≈ 1–2 GB each, no cache) |

**Keyframe size.** A 1M keyframe is ≈ 0.35–1.1 GB raw and ≈ 0.15–0.5 GB compressed (zstd-3, with rasters quantised to u16 and delta-coded, R03). A 100k keyframe is ≈ 20–60 MB compressed. Belief stores are the largest uncertain term (R-11).

## 6. Storage per 1,000-year world

Estimates follow `05.2` §8. They are measured on a synthetic 100-year 1M world in P1 (WP P1.14) and extrapolated.

| Item | 100k | 1M |
|---|---|---|
| Chronicle events and roles (named actors), Parquet | 2–4 GB | 5–15 GB |
| Anchored snapshots and causal edges, Parquet | 1–3 GB | 5–15 GB |
| Knowledge rows and diffusion fronts, Parquet | 1–5 GB | 5–30 GB |
| Life-course log (everyone ever born), Parquet | ≈ 0.2 GB | 1.5–2.5 GB |
| Keyframes, permanent (10-year cadence) | 2–5 GB | 20–50 GB |
| Regional aggregates | ≈ 0.2 GB | 0.5–2.5 GB |
| Journal, hashes, indices | < 100 MB | < 200 MB |
| **Total (permanent)** | **≈ 6–18 GB** | **≈ 35–115 GB** |
| Keyframe LRU, on-disk tier (≤ 20 yearly keyframes; an evictable cache) | ≈ 0.4–1.2 GB | ≈ 3–10 GB |

A 1M world is **tens of GB**, central estimate ≈ 60 GB. Keyframes and knowledge dominate.
- **Thinning.** Cold keyframes can be thinned to a 50-year cadence (−80% of keyframe bytes), but only where an archived engine build can regenerate them (`05.2` §7).
- **Recompression.** Cold segments are recompressed per century (zstd-19).
- **Forks.** Segments before a fork are shared by reference, so a fork costs only its own tail.
- **No automatic deletion.** Hard deletion is never automatic (ADR-0007).

## 7. Interactive client budgets

| Item | Budget | Source |
|---|---|---|
| Console frame publish (MAX speed) | ≤ 4 Hz overview updates; map deltas as Arrow | R10 |
| 3D state plane, 200 embodied | ~2.6 KB per full frame, ~1.4 KB per delta at 20 Hz (≈ 0.35 Mbit/s) | [M] R10 |
| Godot crowd tiers at 60 FPS (mid-range GPU) | 20–40 near-rig, 50–150 mid-rig, 1k–5k VAT, 5k–30k impostors | [E] R06; validate in SPK-1 |
| Hydration throughput | 5–10 persons/s with velocity prefetch | [E] R01 §R7 |
| Wallpaper mode | ≤ 30 FPS; < 10% GPU; < 5 W extra package power; 0 FPS when covered | SPK-3 (R06) |
| Local LLM dialogue | TTFT < 0.5 s (cached prefix); first sentence < 1 s; end-to-end with TTS 0.8–1.9 s | R05, R07 |
| VRAM split (12 GB card) | Game ≤ 6 GB, LLM ~5.5 GB (9B Q4 or 4B with headroom) | R05 |
| TTS (CPU path) | 2 threads, ~200 ms to first audio | R07 |

## 8. Narrative and LLM cost envelope (Oct 2026 prices)

| Artifact | Tokens [E] | Cost (recommended configuration) | Time |
|---|---|---|---|
| World compile + repairs | ~10–30k | < $0.10 | < 2 min, then the liveness gate (≈ 60 s at 100k); certification runs in the background (≈ 20 min) |
| Annals (per century) | 0 (templated) | $0 | seconds |
| Chronicle polish (per century) | ~100–300k in / 20–50k out | ~$0.05 (Haiku-class batch) – $1 (Opus-class) | minutes (batch) |
| Saga (15k words) | ~3–5M in / 0.4M out | ~$5–10 batch | 1–3 h (batch) |
| Novel (100k words) | ~21M in / 2.2M out | ~$28 batch+cache; ~$67 premium | 6–24 h (batch) |
| Screenplay (120 pp) | — | ~$15 batch | hours |
| 10-episode audio-drama script | — | ~$19 batch, plus TTS render 1–3× audio length on a ≥ 12 GB GPU | overnight |

A simulation run makes no model calls of its own: no LLM decides anything, so nothing here sits on the tick's critical path (ADR-0034). Deliberation consults and their cost are retired.

## 9. Benchmarks to establish in P0/P1 (become CI gates)

They run under the V-PERF suite in `09`.


| # | Bench | Measures | From |
|---|---|---|---|
| 1 | `bench/ledger_month_1M` | Per-person hazard pass, keyed draws and pop aggregation on a synthetic 1M world | P1a |
| 2 | `bench/timers_lifecycle_1M` | Scheduled outcomes at normal and crisis loads: sampling, heap, ordered lifecycle application | P1a |
| 3 | `bench/serial_fraction` | An instrumented tick: serial vs parallel wall time per phase. Fails above 20% | P1a |
| 4 | `bench/raster_8x512` | Stencil fields | P1a: the first sprint that builds a spatial field, M1 or M3 (director, G0 rulings) |
| 5 | `bench/agenda_5k` | Snapshot scoring, keyed-permutation application, invalidation and re-score | P1b |
| 6 | `bench/sifter_throughput` | 50 patterns with declared match semantics on a clustered synthetic 10M-event chronicle | P2 |
| 7 | `bench/plane_hashes` | `state_hash`, `observation_hash` and `narrative_hash`: full and dirty-chunk | P0 |
| 8 | `bench/keyframe_write_read` | 100k and 1M: copy-on-write stall, compression, decompression | P0 |
| 9 | `bench/dropin` | 100k and 1M: RAM-tier hit, yearly-LRU hit, 10-year worst case (MOP-4) | P2 |
| 10 | `bench/run_1000y_100k` | End-to-end MOP-1 | P1b |
| 11 | `bench/liveness_gate_100k` | 200 years × 8 seeds, wall time | P1b |
| 12 | `bench/belief_store_5k` | Memory and update cost of bounded T1 belief stores | P3a |
| 13 | `bench/foresight_256x25y_100k` | Interactive-class Foresight | P3b |
| 14 | `bench/foresight_64x50y_1M` | Batch Foresight; nightly on a cluster node | P3b |

**Regression policy.**
- **Slowdowns fail.** A slowdown of more than 10% against the recorded baseline fails the PR (V-PERF-04 in `09`; `13` §4.3).
- **Speed-ups never re-baseline automatically.** A faster build does not tighten the gate. Auto-tightening would turn one lucky run, or ordinary runner noise, into red builds for everyone after it. It would also make a speed-up that came from skipped work the new bar.
- **Re-baselining is a deliberate, reviewed change.**
  - It is a dedicated PR that edits the recorded baselines.
  - The PR attaches the bench report (≥ 5 repeated runs on the same runner hardware) and shows unchanged plane-A hashes, which proves no work was skipped.
  - The reviewer agent reviews it, it lands through the merge queue (`13` §4.4), and it appears in your weekly review digest (`11` §Review-hours budget).
