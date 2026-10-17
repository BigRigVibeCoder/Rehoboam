# Worldgen-lite v0: companion to `M1.regions` F0 and `S3.migration` F0 (SPR-018)

*Status: draft companion, round 2 after `REV-SPR-018` (ADR-0033 §4: prose for reasoning only; where it differs from the TOML, the TOML is the spec). Normative text: `specs/systems/M1.regions.odd.toml` `[algorithm.worldgen_lite]`, `specs/systems/S3.migration.odd.toml` `[algorithm.residence_f0]`, the data of `specs/worldgen/templates_v0.toml`. Executable contract: `specs/worldgen/selftest_worldgen_lite.py` with `specs/worldgen/golden/worldgen_lite_v0.txt`.*

> **BLUF:** Worldgen-lite turns a manifest's `geography` section (a heightmap template, a raster resolution, a region count) and the world seed into a u16 elevation raster, K regions, one settlement per region and one route per region adjacency, in integers only, with every draw keyed and every tie broken by the lowest cell index. S3 F0 then places each manifest household on a settlement by one keyed, land-weighted draw and gives every newborn its mother's settlement; nobody moves. The exact Python reference reproduces all of it bit for bit and pins it with golden outputs. Bench 4's fixture is the 512² raster of `continental_plains` at seed 1.

## 1. Why the algorithm is stated in the TOML and not here

ADR-0033 §4 makes the `.odd.toml` the spec and this file its narrative companion. M2's rung-0 slice set the precedent: `[algorithm.mortality_rung0]` in the TOML, `selftest_m2_rung0.py` as the exact reference, check values pinned in both. Worldgen-lite follows it. The `[algorithm.*]` tables are also what `lint_odd.py --next` will protect byte for byte once M1 and S3 freeze (SPR-015's `lint_next`, `specs/systems/next/README.md`), so putting the algorithm anywhere else would leave it unprotected. **Recommendation: TOML, with this companion** (confidence 85%).

## 2. The pipeline, and what each step owes to the sources

| Step | What | Precedent (docs/14; research) |
|---|---|---|
| W1–W3 | A template is a list of integer blob operations (domes and rings) with keyed centres, heights and radii, summed on a base level and clamped to u16 | Azgaar FMG's heightmap templates are lists of `Hill`, `Range`, `Pit`, `Trough`, `Strait` steps with ranged counts and positions (MIT; PORT of the idea, docs/14 §B; R06 §4 item 2). Our blobs are the integer, resolution-independent form of FMG's hills and pits; ranges and straits are F1 if a template needs them |
| W4 | A 5-point integer smoothing pass, Jacobi, edges clamped | The stencil fields of `05.1` §5 and docs/08 §1 (R03 §6: 8 layers of 512² in 0.65 ms on 4 threads). Integer because A-22's `f32_field` rule is not due while elevation stays u16 (PRJ-002 §8) |
| W5 | Land iff elevation ≥ the template's sea level | FMG's sea level; integer threshold |
| W6 | The first region seed by a keyed draw, then one seed per 4-connected land component without one (in order of their lowest cell), then farthest-point picks over all land | FMG seeds states on scored cells; mewo2/terrain scores city sites (docs/14 §B). Farthest-point keeps regions spread without a second random draw per seed; the single keyed draw makes a seed change move everything (R06 §4.5: generation as a function of the seed). The per-component pass (REV-SPR-018 R5) keeps islands from going unregioned while K allows |
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

Founders are placed once, at genesis, after M1's tables and M2's founder rows exist: one draw per manifest household, keyed by its head's PersonId, picks a settlement with probability proportional to its region's land cells (`[algorithm.residence_f0]` R1–R3); every member of the household lives there; a founder in no household draws for themself. Each person gets one `residence_interval` row opening at their entry tick (0 for founders). A newborn takes its mother's settlement in the same Vital slot as the birth (R4: S3's Day/Vital binding is scheduled after M2's by the edge its `same_instant` read of `person.mother` creates; the newborns are the rows whose `person.residence` is still the type's zero SPR-014's row insert wrote, never a founder, even one born at tick 0: REV-SPR-018 R4). An interval's end is never stored: it is the next interval's start or the person's `life_course.exit_tick` (R5), so S3 never reacts to a death and the table is append-only (ADR-0007). V-DATA-04's P1a variant audits that the one interval per person tiles `[entry_tick, exit_tick)` and that per-settlement counts reconcile with entries and exits.

**Marriage in P1a stays inside a settlement** (REV-SPR-018 R8): M2 F1's husband draw takes its candidates from the bride's settlement, reading `person.residence` as a mode read whose neutral 0 means no restriction, so households never span two settlements before moves exist and M2 still registers without S3.

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

1. `rh-worldgen` (A, machinery; exists as the SPR-012 stub): templates loaded from `specs/worldgen/templates_v0.toml` at build time (`include_str!`, so the data is in the engine build and the history tuple), W1–W10, R1–R4; the three purposes in `rng_purposes.rs`. The manifest validator (Python and the Rust loader, V-WC-04 parity) gains the two household refusals of R2 (REV-SPR-018 R9), an explicit SPR-018 deliverable in PRJ-002's row. Channel A gains the kind 7 `RESIDENCE_INTERVAL` record's fields.
2. `rh-sys-regions` (M1 F0): registration of the owned tables, `invariants()` for INV-M1-01..06, no binding, no step.
3. `rh-sys-migration` (S3 F0): `person.residence`, `residence_interval.*`, the Day/Vital `residence_at_birth` step (a no-op until SPR-015's births merge), `invariants()` for INV-S3-01..05.
4. `rh-contracts`: accessors for `settlement.*`, `region.*`, `route.*`, `region_edge.*`, `person.residence`. The Registry: `S3.migration` in every template's `default_systems` and the `registry_version` bump (REV-SPR-018 R3); the example manifests' pins follow.
5. The golden regeneration of `solo`, `household` and `hamlet` (C3: genesis changes every rung's hash), the V-INV-10 two-machine check, and the Rust port's agreement with the golden file (every line of `worldgen_lite_v0.txt` reproduced).
6. Bench 4 as §5.

## 7. Open points, as ruled in `REV-SPR-018` round 1

| # | Point | Ruling (Architect, REV-SPR-018 §0) |
|---|---|---|
| 1 | **A channel-A record for residence.** The chronicle's `life_interval` (kind 0 = residence) has no channel-A record kind in v0 (`channel_a_v0.sql` has LIFE_ENTRY and LIFE_EXIT; only DECISION_TRACE and ENCOUNTER are reserved), so residence intervals would be in plane A but not in the stream | R11 Q1: residence intervals become a non-event channel-A record: kind 7 `RESIDENCE_INTERVAL` is reserved in `channel_a_v0.sql` (on `main`, 7fb71c7); SPR-018's sprint defines the fields (C2) |
| 2 | **`schema.sql` said residence is a region.** S3 records a settlement (the region is derivable through `settlement.region`, and S5 needs the settlement) | R11 Q2: the comment now says settlement id; no DDL change |
| 3 | **What a lifetime is for V-DATA-04's tiling.** Founders are born before the epoch | R11 Q3: `[life_course.entry_tick, exit_tick)`, excluding the pre-epoch ancestors SPR-019 back-fills (R7); recorded in the `docs/09` P1a row as an Architect clarification |
| 4 | **`registry_version` and registration.** The S3 entry is in `registry_v0.toml` now, without a bump and in no template | R3: SPR-018's sprint PR adds S3 to every template's `default_systems` and bumps the version; until then a manifest naming S3 falls under SPR-011 ruling 3 |
| 5 | **The farthest-point cost at the largest manifests.** W6 is O(K × land cells): about 8 × 10^10 updates at K = 5,000 and R = 4,096 | Accepted for P1a; a grid-bucketed farthest point is the F1 remedy (M1 `scale_behaviour`) |
| 6 | **Seats on the coast.** The centroid-nearest seat is inland by construction | Kept for F0; F1 scores sites, as FMG does |
| 7 | **`world.focus[].settlement` names a settlement by slug**, while settlements have numeric ids and no names | R11 Q4: deferred, and never mapped through households (two households can share a settlement); a `settlements:` naming section when S5 F1 needs focus regions |
| 8 | **Disconnected region graphs (archipelago).** F0 has no sea routes | R5 and R11 Q5: accepted; every land component gets a seed while K allows, land left over when K is below the component count belongs to no region, and F1's sea mode connects islands. Each golden case pins its region-graph component count (`graph` line); desert_fringe seed 3 at K = 4 has an island region with no edge |
| 9 | **Bench 4 measures a genesis kernel**, not `docs/08` §1's per-step f32 stencil | R6: bench 4 stays as defined; `docs/08` §9 says the per-step budget waits for M3 (P1b) |

No director item arose: no requirement, ADR or roadmap text changes. A-22 stays not due (elevation is u16), as PRJ-002 §8 expects. The two Architect clarifications (the V-DATA-04 span, the bench 4 reading of the G0 ruling) are the Architect's to record in `docs/decisions-log.md`.
