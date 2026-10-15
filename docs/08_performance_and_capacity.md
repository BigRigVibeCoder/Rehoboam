# 08 — Performance Budgets and Capacity Model

*Status: Draft v1.0 (2026-10-15).*

**Where the numbers come from**
- **[M]** Measured in research sandboxes: 4-vCPU Xeon at 2.1 GHz (R01, R03, R10).
- **[E]** Engineering estimates derived from [M]. These must be re-measured in P0/P1. Regression gates in CI enforce them once measured.
- **Reference desktop:** 8 performance cores at ~4.5 GHz, 64 GB RAM, RTX 4070/4090-class GPU.
- **Reference cluster node:** 64+ cores, plus GPU nodes for LLM and TTS.

---

## 1. Macro step budget at 1M persons (Rust, 8 cores, per sim-month)

| Work item | Basis | Budget |
|---|---|---|
| Raster fields: 8 layers of 512², 5-point stencil | [M] 0.65 ms on 4 threads (R03) | 0.7 ms |
| Ledger vital events: keyed hazard draw per person | [E] ~2–5 ns per row hash + compare, 1M rows, parallel | 1.0 ms |
| Pop aggregation: group-by region × class × faith | [E] single pass, 1M rows, parallel | 1.0 ms |
| Region systems: economy (seasonal), polity (yearly), agriculture | [E] 2,000 regions, trivial | 0.5 ms (amortised) |
| Named-actor decisions: ~5k due decisions/month × 1–5 µs, **evaluated in parallel against the start-of-instant snapshot and applied sequentially in agenda order** | [M] 0.81 ms for a 5k toy; [E] realistic ×3–5 | 3.0 ms |
| Event emission: ~21k named events/month (250k/yr) into Arrow builders | [E] | 0.5 ms |
| Live sifter: 21k events × trigger-indexed patterns; target ≥ 10⁷ events/s/core in Rust | [M] 2.2M events/s, optimised pure Python (R10); the repo reference matcher measures ≈ 0.2M events/s; [E] Rust | 2.0 ms |
| State hash: chunked Merkle xxh3 with dirty tracking | [E] | 1.0 ms |
| Invariants: debug/CI builds only | — | (0 in release) |
| Headroom and misc | — | 2.3 ms |
| **Total** | | **≈ 12 ms/month** |

**Implications**
- 1,000 years = 12,000 months ≈ **2.4 min**, plus active daily sub-steps for epidemics, campaigns and sieges (≈ +30–80%). That gives **≈ 3–5 min per 1,000 years at 1M**, inside PERF-1's P3 target of ≤ 10 min.
- At 100k persons, **assuming ~1k named actors**, ledger-proportional terms shrink ~10× and actor-proportional terms (decisions, emission, sifter) ~5×: 0.7 (raster) + 0.1 + 0.1 + 0.5 + 0.6 + 0.1 + 0.4 + 0.1 + ~0.8 headroom ≈ **3.5 ms/month**. That is ≈ 42 s per 1,000 years, or **≈ 55–75 s** with active daily sub-steps, inside the P1 target of ≤ 2 min.
- **Risk to G1:** if 100k-person worlds carry the full 5k named actors, the estimate rises to ≈ 9 ms/month (≈ 2.4–3.4 min per 1,000 years), which misses MOP-1. Named-actor count must scale with population, or the actor tier must be optimised first. Measure at G1.
- Release builds hash yearly by default. CI and debug builds hash monthly; the chunked Merkle scheme keeps that cheap.

**Two-phase decision evaluation.** Named-actor decisions due in the same instant are *scored* in parallel against an immutable snapshot, then *applied* sequentially in `(time, phase, entity_id, seq)` order. If an earlier application invalidates a later actor's choice, that actor re-scores. This keeps the result deterministic and the work parallel. Semantics: "simultaneous deliberation, ordered action".

## 2. Timeline operations

| Operation | Calculation | Target |
|---|---|---|
| Scrub (chronicle and aggregates only) | DuckDB read-only views over Parquet; overview strip precomputed per year | < 100 ms (MOP-3) |
| Drop in with a yearly keyframe cache | mmap Arrow IPC ([M] 90 MB in 0.6 ms) + ≤ 12 months replay × 12 ms | **≈ 0.2 s at 1M** |
| Drop in with only the 10-year permanent keyframes | ≤ 120 months × 12 ms | **≈ 1.5 s at 1M** (MOP-4 is ≤ 10 s) |
| Keyframe capture | Copy-on-write snapshot of tables (~350 MB raw at 1M) + background zstd-3 ([M] ~0.65 GB/s on 4 threads) | Sim stall < 100 ms; ~0.5 s background |
| Fork | New timeline ID + keyframe reference + journal tail | < 1 s |
| Burn-in: 200 years × 8 seeds at 100k | 2,400 months × ~4 ms per seed, seeds in parallel | **≈ 10–20 s wall** |

## 3. Foresight (Monte-Carlo) capacity

| Scenario | Per-rollout cost | N | Wall time (desk, 8 cores) | Wall time (cluster, 64 cores) |
|---|---|---|---|---|
| 25-year horizon, 100k persons, single-threaded rollout | 300 months × ~25 ms (1 core) ≈ 7.5 s | 256 | ≈ 4 min | ≈ 30 s |
| Same, with reduced fidelity outside the predicate's regions | ≈ 3 s | 256 | ≈ 1.6 min | ≈ 12 s |
| 25-year horizon, 1M persons | ≈ 300 × 90 ms ≈ 27 s | 256 | ≈ 15 min | ≈ 2 min |
| Intervention A/B with common random numbers (paired) | 2× the above | 128 pairs give a CI similar to ~1,000 independent runs (variance reduction; [E], to measure) | — | — |

**Precision.** At N = 256 and P ≈ 0.3, the Wilson 95% CI is about ±0.056. At N = 1,024 it is about ±0.028. More compute buys tighter forecasts, which makes this the main horizontal-scaling workload (ADR-0020).

## 4. Calibration and validation ensembles

| Job | Size | CPU cost [E] | Wall time on cluster |
|---|---|---|---|
| Stylized-fact ensemble per reference world | 32 seeds × 1,000 years at 100k | 32 × ~60 s ≈ 32 CPU-min | < 1 min |
| Morris screening | ~20 params × 10 trajectories × 200 years | ~210 runs × 12 s ≈ 42 CPU-min | ~1 min |
| Sobol indices (selected params) | ~2,000 runs × 200 years | ≈ 7 CPU-hours | ~7 min |
| ABC / simulated-moments calibration | 10⁴–10⁵ short runs | 1–20 CPU-days | hours |

## 5. Memory budget (1M persons)

| Component | Size |
|---|---|
| Ledger (alive) | ≤ 64 MB |
| Lineage table (tracked) | 0.5–2 GB over 1,000 years (can be paged) |
| Rasters (16 layers at 512², f32) | 16 MB (higher resolution: 1024² gives 64 MB) |
| Pops, regions, polities, markets | < 50 MB |
| Named actors (5k × 1–4 KB) + social graph (≤ 10⁶ edges × ~16 B) | < 40 MB |
| Belief stores (named actors only; notable propositions) | 50–500 MB [E]: **the main unknown; measure in P3** |
| Sifter partial matches | < 200 MB with pruning |
| Hot event window (30 sim-days) | < 50 MB |
| **Sim host total** | **≈ 1–3 GB** |

## 6. Storage per 1,000-year world

| Item | 100k | 1M |
|---|---|---|
| Chronicle (Parquet) | 1–3 GB | 3–5 GB |
| Keyframes (10-year cadence) | ~2 GB | ~20 GB |
| Regional aggregates | ~0.2 GB | 0.5–2.5 GB |
| Journal, hashes, indices | < 100 MB | < 100 MB |
| **Total** | **≈ 3–5 GB** | **≈ 25–30 GB** |

Keyframes are the dominant cost. Cold keyframes can be thinned to a 50-year cadence, because they can be re-derived from the journal and seed with pinned model versions.

## 7. Interactive client budgets

| Item | Budget | Source |
|---|---|---|
| Console frame publish (MAX speed) | ≤ 4 Hz overview updates; map deltas as Arrow | R10 |
| 3D state plane, 200 embodied | ~2.6 KB per full frame, ~1.4 KB per delta at 20 Hz (≈ 0.35 Mbit/s) | [M] R10 |
| Godot crowd tiers at 60 FPS (mid-range GPU) | 20–40 near-rig, 50–150 mid-rig, 1k–5k VAT, 5k–30k impostors | [E] R06; validate in Spike 1 |
| Hydration throughput | 5–10 persons/s with velocity prefetch | [E] R01 §R7 |
| Wallpaper mode | ≤ 30 FPS; < 10% GPU; < 5 W extra package power; 0 FPS when covered | R06 Spike 3 |
| Local LLM dialogue | TTFT < 0.5 s (cached prefix); first sentence < 1 s; end-to-end with TTS 0.8–1.9 s | R05, R07 |
| VRAM split (12 GB card) | Game ≤ 6 GB, LLM ~5.5 GB (9B Q4 or 4B with headroom) | R05 |
| TTS (CPU path) | 2 threads, ~200 ms to first audio | R07 |

## 8. Narrative and LLM cost envelope (Oct 2026 prices)

| Artifact | Tokens [E] | Cost (recommended configuration) | Time |
|---|---|---|---|
| World compile + repairs | ~10–30k | < $0.10 | < 2 min + burn-in |
| Annals (per century) | 0 (templated) | $0 | seconds |
| Chronicle polish (per century) | ~100–300k in / 20–50k out | ~$0.05 (Haiku-class batch) – $1 (Opus-class) | minutes (batch) |
| Saga (15k words) | ~3–5M in / 0.4M out | ~$5–10 batch | 1–3 h (batch) |
| Novel (100k words) | ~21M in / 2.2M out | ~$28 batch+cache; ~$67 premium | 6–24 h (batch) |
| Screenplay (120 pp) | — | ~$15 batch | hours |
| 10-episode audio-drama script | — | ~$19 batch, plus TTS render 1–3× audio length on a ≥ 12 GB GPU | overnight |
| Pivotal consults | ≤ 20/sim-year/world × ~3k tokens | Cluster model: negligible; frontier: ~$0.01–0.05 each | Pre-batched |

## 9. Benchmarks to establish in P0/P1 (become CI gates)

1. `bench/ledger_month_1M`: vital events plus aggregation.
2. `bench/raster_8x512`: stencil fields.
3. `bench/agenda_5k`: two-phase decision evaluation.
4. `bench/sifter_throughput`: 50 patterns, synthetic 10M-event chronicle.
5. `bench/hash_merkle`: full and dirty-chunk.
6. `bench/keyframe_write_read`: 100k and 1M.
7. `bench/run_1000y_100k`: end-to-end MOP-1.
8. `bench/foresight_256x25y_100k`.

**Regression policy.** A slowdown of more than 10% fails the PR. A speed-up of more than 10% re-baselines automatically after merge.
