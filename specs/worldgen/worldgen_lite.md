# Worldgen-lite v0: companion to `M1.regions` F0 and `S3.migration` F0 (SPR-018)

*Status: draft companion (ADR-0033 §4: prose for reasoning only; where it differs from the TOML, the TOML is the spec). Normative text: `specs/systems/M1.regions.odd.toml` `[algorithm.worldgen_lite]`, `specs/systems/S3.migration.odd.toml` `[algorithm.residence_f0]`, the data of `specs/worldgen/templates_v0.toml`. Executable contract: `specs/worldgen/selftest_worldgen_lite.py` with `specs/worldgen/golden/worldgen_lite_v0.txt`.*

> **BLUF:** Worldgen-lite turns a manifest's `geography` section (a heightmap template, a raster resolution, a region count) and the world seed into a u16 elevation raster, K regions, one settlement per region and one route per region adjacency, in integers only, with every draw keyed and every tie broken by the lowest cell index. S3 F0 then places each manifest household on a settlement by one keyed, land-weighted draw and gives every newborn its mother's settlement; nobody moves. The exact Python reference reproduces all of it bit for bit and pins it with golden outputs. Bench 4's fixture is the 512² raster of `continental_plains` at seed 1.

## 1. Why the algorithm is stated in the TOML and not here

ADR-0033 §4 makes the `.odd.toml` the spec and this file its narrative companion. M2's rung-0 slice set the precedent: `[algorithm.mortality_rung0]` in the TOML, `selftest_m2_rung0.py` as the exact reference, check values pinned in both. Worldgen-lite follows it. The `[algorithm.*]` tables are also what `lint_odd.py --next` will protect byte for byte once M1 and S3 freeze (SPR-015's `lint_next`, `specs/systems/next/README.md`), so putting the algorithm anywhere else would leave it unprotected. **Recommendation: TOML, with this companion** (confidence 85%).

## 2. The pipeline, and what each step owes to the sources

| Step | What | Precedent (docs/14; research) |
|---|---|---|
| W1–W3 | A template is a list of integer blob operations (domes and rings) with keyed centres, heights and radii, summed on a base level and clamped to u16 | Azgaar FMG's heightmap templates are lists of `Hill`, `Range`, `Pit`, `Trough`, `Strait` steps with ranged counts and positions (MIT; PORT of the idea, docs/14 §B; R06 §4 item 2). Our blobs are the integer, resolution-independent form of FMG's hills and pits; ranges and straits are F1 if a template needs them |
| W4 | A 5-point integer smoothing pass, Jacobi, edges clamped | The stencil fields of `05.1` §5 and docs/08 §1 (R03 §6: 8 layers of 512² in 0.65 ms on 4 threads). Integer because A-22's `f32_field` rule is not due while elevation stays u16 (PRJ-002 §8) |
| W5 | Land iff elevation ≥ the template's sea level | FMG's sea level; integer threshold |
| W6 | The first region seed by a keyed draw, the rest by farthest-point sampling over land cells | FMG seeds states on scored cells; mewo2/terrain scores city sites (docs/14 §B). Farthest-point keeps regions spread without a second random draw per seed; the single keyed draw makes a seed change move everything (R06 §4.5: generation as a function of the seed) |
| W7 | Regions by a multi-source integer Dijkstra over land with a slope-weighted step cost | FMG grows states by a cost-weighted flood over Voronoi cells (R09 §10; docs/14 §B). Over the raster instead of a Voronoi diagram: Delaunay and Voronoi on integer inputs are DEPEND candidates (`spade`, docs/14 §B) but F0 needs no polygonal cells, and a raster flood gives connected regions by construction |
| W8 | One seat per region: the cell nearest the region's floor centroid | FMG places burgs by cell score; F0 takes the simplest deterministic choice, F1 scores sites (coast, rivers) |
| W9 | Region graph = 4-adjacency of regions; one route per edge with a Euclidean length | `05.1` §3 M1 F0 "static region graph"; F1 adds Urquhart-style routes with modes and costs (FMG routes; `05.2` §2 item 8). `petgraph` and `pathfinding` are DEPEND candidates for F1's graph algorithms, with sorted iteration (docs/14 §B); F0's edge list and Dijkstra are small enough to write in the crate, which keeps the tie-break rule explicit |

**Not reused, and why.** `noise-rs` and FastNoiseLite (docs/14 §B) are float noise: a worldgen-lite raster must be integer and bit-identical across machines (V-INV-10), so templates are integer blobs instead of noise. The full FMG port (Voronoi, rivers, climate, biomes, cultures) is P1.1 in P4.

## 3. Determinism rules the algorithm obeys

- **Keyed draws only** (ADR-0006; `05.1` §6 item 2): every draw is `H(world_seed, canonical salt, system_id("rh-worldgen"), epoch 0, purpose, entity, t = 0, k)`. Purposes: `heightmap_blob` (entity = blob ordinal, k = 0..3 or 0..4), `region_seed` (entity 0, k = 0), `founder_residence` (entity = the household head's or loose founder's PersonId, k = 0). All three are rh-worldgen's, under the reserved system id of SPR-011 ruling 10, declared in its `rng_purposes.rs`; M1 and S3 own no purpose at F0.
- **Fixed draw counts:** a dome takes 4 draws and a ring 5 whatever their values; one draw per manifest household and per loose founder. No rejection sampling anywhere, so common random numbers hold across templates and seeds (`05.1` §6 item 9).
- **No HashMap iteration, no float reduction:** land cells are visited in index order; the Dijkstra queue is a binary heap on `(dist, cell, region)`; edges are a sorted set; sums are integer. The reference uses Python sets only to deduplicate edges, then sorts.
- **Every tie broken:** farthest-point ties go to the lowest cell index; Dijkstra ties to the lower cell then the lower region; seats to the lowest index at equal distance.
- **Widths:** the accumulator is i64; elevation u16; a Dijkstra distance is below 2^33 at R = 4096 (u64); `pick(u, n)` is a u128 product.
- **Resolution-independent templates:** positions and radii are per-mille of R, so `solo` at 64 and a kingdom at 512 use one template file; the rasters differ, the shapes do not.

## 4. Residence (S3 F0) in one paragraph

Founders are placed once, at genesis, after M1's tables and M2's founder rows exist: one draw per manifest household, keyed by its head's PersonId, picks a settlement with probability proportional to its region's land cells (`[algorithm.residence_f0]` R1–R3); every member of the household lives there; a founder in no household draws for themself. Each person gets one `residence_interval` row opening at their entry tick (0 for founders). A newborn takes its mother's settlement in the same Vital slot as the birth (R4: S3's Day/Vital binding is scheduled after M2's by the edge its `same_instant` reads of `person.birth_tick` and `person.mother` create). An interval's end is never stored: it is the next interval's start or the person's `life_course.exit_tick` (R5), so S3 never reacts to a death and the table is append-only (ADR-0007). V-DATA-04's P1a variant audits that the one interval per person tiles `[entry_tick, exit_tick)` and that per-settlement counts reconcile with entries and exits.

**Why the mother, not the household.** M2 F1's `person.household` exists only in the next-version spec (`specs/systems/next/`, SPR-015, not yet on `main`); `person.mother` exists in the frozen 0.2.0 slice, so S3 F0 reads it and lints green against `main` today. At F0 the two agree: a child joins its mother's household (SPR-015 H5) and its mother's settlement. When moves arrive (F1) the co-residence rule will keep them agreeing.

## 5. Bench 4: `bench/raster_8x512` (docs/08 §9)

The director's G0 ruling moved bench 4's baseline to the first P1a sprint that builds a spatial field; SPR-018 is it. The fixture, so that the baseline measures one fixed computation:

| Item | Definition |
|---|---|
| Input | `E`, the 512² u16 elevation raster of template `continental_plains` at world seed 1, `[algorithm.worldgen_lite]` W1–W4 (its template has one smoothing pass). SHA-256 pinned as `bench4_elevation_sha256` in the golden file |
| Layers | 8 layers `L_i`, i = 0..7: `L_i(x, y) = E((x + 61 i) mod 512, (y + 37 i) mod 512)`, a cyclic shift per layer so that the eight layers differ and no layer is a copy |
| Kernel | One W4 smoothing pass over each layer: `floor((4c + n + s + e + w) / 8)`, edges clamped, Jacobi (read the input layer, write an output layer) |
| Parallelism | Fixed chunking of 65,536 cells (128 rows) in row-major order (`05.1` §6 item 5); the bench measures 1 thread (instructions) |
| Output check | SHA-256 over the eight output layers in order, little-endian u16: `bench4_smoothed_layers_sha256` in the golden file. The bench asserts it before measuring, so a baseline never records skipped work (docs/08 §9 regression policy) |
| Measure | callgrind instructions through iai-callgrind, as benches 7 and 8 (`bin/perf_gate.py`); the bench target is `raster_8x512` in `crates/rh-bench/benches/`, case `smooth/layers_8x512` |
| Recording | SPR-018 replaces the `slot raster_8x512` line of `crates/rh-bench/baselines/instructions_v1.txt` with the measured `bench` lines through `python -I bin/perf_gate.py record`, in a `[perf-baseline]` PR with a `perf-baseline-reason:` line (SPR-009 ruling 3). `BENCH_TARGETS` in `perf_gate.py` gains `raster_8x512` and `SLOTS` loses it, in the same PR |

The fixture is a plane-A computation in a tool crate (`rh-bench`), so it may use the generated raster without an engine; the generator itself lives in `rh-worldgen` and the bench depends on it as benches 7 and 8 depend on `rh-snapshot`.

## 6. What SPR-018 builds, in the order the spec implies

1. `rh-worldgen` (A, machinery; exists as the SPR-012 stub): templates loaded from `specs/worldgen/templates_v0.toml` at build time (`include_str!`, so the data is in the engine build and the history tuple), W1–W10, R1–R4; the three purposes in `rng_purposes.rs`.
2. `rh-sys-regions` (M1 F0): registration of the owned tables, `invariants()` for INV-M1-01..06, no binding, no step.
3. `rh-sys-migration` (S3 F0): `person.residence`, `residence_interval.*`, the Day/Vital `residence_at_birth` step (a no-op until SPR-015's births merge), `invariants()` for INV-S3-01..05.
4. `rh-contracts`: accessors for `settlement.*`, `region.*`, `route.*`, `region_edge.*`, `person.residence`.
5. The golden regeneration of `solo`, `household` and `hamlet` (C3: genesis changes every rung's hash), the V-INV-10 two-machine check, and the Rust port's agreement with the golden file (every line of `worldgen_lite_v0.txt` reproduced).
6. Bench 4 as §5.

## 7. Open points for the Architect (recommendations, not decisions)

| # | Point | Recommendation | Confidence |
|---|---|---|---|
| 1 | **A channel-A record for residence.** The chronicle's `life_interval` (kind 0 = residence) has no channel-A record kind in v0 (`channel_a_v0.sql` has LIFE_ENTRY and LIFE_EXIT only), so residence intervals would be in plane A but not in the stream or the Parquet chronicle | Add record kind 7 `LIFE_INTERVAL` {person_id, kind, value, from_tick} to channel A in SPR-018 (a C2 stream-format change; `05.9` IF-0 lists it as reserved), written at genesis for founders and at each birth; alternatively a `residence` column on LIFE_ENTRY, which covers F0 but not F1's moves | 70% for the record kind |
| 2 | **`schema.sql` says residence is a region.** `life_interval` kind 0's comment reads "residence (region)"; S3 records a settlement (the region is derivable through `settlement.region`, and S5 needs the settlement) | Change the comment to "residence (settlement id; region through M1)" when SPR-018 merges; no DDL change | 85% |
| 3 | **What a lifetime is for V-DATA-04's tiling.** Founders are born before the epoch; their residence before tick 0 is not history (`05.2` §1) | Define the audited span as `[life_course.entry_tick, exit_tick)`, as R5 does, and say so in V-DATA-04's P1a variant row | 80% |
| 4 | **`registry_version`.** The S3 entry is added to `registry_v0.toml` now (lint and `rh-cli ownership` need the system to exist) without bumping the version, as the M2 F1 delta also defers its bump | Bump to 0.3.0 in SPR-018's merge, together with the M2 F1 delta if it lands first; the example manifests' pins follow (they warn, not fail, on a mismatch) | 75% |
| 5 | **The farthest-point cost at the largest manifests.** W6 is O(K × land cells): fine to K = 2,000 at 512², about 10^10 at K = 5,000 and R = 4,096 | Accept for P1a (no manifest above 512² and K = 600 exists); note it as the F1 item to revisit with a grid-bucketed farthest-point when a kingdom manifest asks for more | 80% |
| 6 | **Seats on the coast.** The centroid-nearest seat is inland by construction; real settlements favour coasts and rivers | Keep for F0 (no rivers exist); F1 scores sites, as FMG does | 85% |
| 7 | **`world.focus[].settlement` names a settlement by slug** (`manifest_v0.py` `Focus`), while worldgen-lite settlements have numeric ids and no names | Settlement names are presentation (plane C); map a focus slug to `settlement.id` by the manifest's `households[].id` of a household placed there, or by an explicit `settlements:` section, when S5 F1 (P1b) needs focus regions; nothing in P1a reads `focus` | 60% |
| 8 | **Disconnected region graphs (archipelago).** F0 has no sea routes, so islands have no route to each other | Accept at F0: nothing travels in P1a; F1's sea mode connects them. The spec's `not_yet` says so | 85% |

No director item arises: no requirement, ADR or roadmap text changes. A-22 stays not due (elevation is u16), as PRJ-002 §8 expects.
