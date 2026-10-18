---
id: REV-WAVE8
title: "Independent review of the wave-8 sprint plans (SPR-028, SPR-029, SPR-030) and D61"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [SPR-028, SPR-029, SPR-030, PRJ-002, SPR-038, SPR-019, SPR-021, SPR-026, SPR-027, BCK-002]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing commit (2026-10-18). The plans go to the director as D61.**
>
> | Plan | Revision | Size | Class |
> |---|---|---|---|
> | SPR-028: the M9 validation harness (stages A and B) | 0.3.0 (`ad5e6fd`) | 8.0–9.0 agent-days | C1 (tool), with C2 parts |
> | SPR-029: the family annals implementation | 0.3.0 | 4.75–5.25 agent-days, with a checkpoint after T-01 | C1 |
> | SPR-030: scale, the `twin_vales` golden, G1a (stages A, B1, B2; SPR-030c conditional) | 0.3.0 | about 10–12 agent-days | A `[perf-baseline]`; B1 C3 (the last of P1a); B2 the G1a session |
>
> - **Round 1** (`dc39967`): 0 BLK, 8 MAJ, 14 MIN (§1). Rulings G1–G22 (§0). Stage M was deleted, because SPR-038b owns the digest. Revision 1 is `1dfc563`.
> - **Round 2** (`1dfc563`): 0 BLK, 5 MAJ, 16 MIN, 5 NIT (§2). The majors:
>   - C1: the 4-thread allocation gate;
>   - C2: V-PERF-09 on SPR-026's built memory gate;
>   - C3: A-68's trigger and an early probe;
>   - A1: the annals spec PR's owner;
>   - X1: the `twin_vales` world directory.
>
>   Rulings: §0b. Revision 2 is `ad5e6fd`, after merging main `99a5b3d`.
> - **The Architect's verification of revision 2** (§0c):
>   - C1's 1-thread rule with the per-spawn constant;
>   - T-14's 10-year `kingdom_10k` probe before B1;
>   - C4's five verdict terms and the cores-0–3/4–7 topology;
>   - A-67 on more than 4 unsealed years, with SPR-030c conditional in PRJ-002;
>   - S1's 99.9%;
>   - AN-0 and AN-1 in the graph;
>   - X1 cited.
>
>   The Architect's own rows are in the closing commit:
>   - PRJ-002 §6's AN-0, AN-1 and PY-PKG rows;
>   - SPR-019 0.3.3 (X1's `worlds/twin_vales/` and its soak; F-20's rename; INV-M7-09's genesis order from the T-00a review);
>   - SPR-021 0.3.1 (F-21 → AN-1; the T-00a review's +0.5 day for the merge-time form);
>   - SPR-038 0.3.3 (the no-re-entry refusal in T-07, T-00a review MIN-4).

## 0. Architect rulings, round 1 (2026-10-18), on `dc39967`

The independent review found **0 BLK, 8 MAJ and 14 MIN**. Of the drafter's 24 findings, 19 are real, 2 partly real, 1 already resolved by SPR-038b, and 2 not stated in the plans. The SPR-038 rulings (`spr038_review/RULINGS.md` §0 and §0b, revision 3 `6f5e54a`), the cross-plan rulings X1–X13 (`p1a_shared_rulings.md`) and the A-62 rulings (`a62_review/SUMMARY.md`) bind this revision. **Every finding is accepted unless a row says otherwise.**

**Priority (the director's directive of 2026-10-18).** All three plans are Priority 1, because they carry G1a's gates:
- the M9 harness;
- the family annals that G1a reads ("read the templated family history of 3 seeds", `docs/11:81`);
- the G1a run.

| # | Ruling | Conf. |
|---|---|---|
| G1 | **Delete stage M.**<ul><li>The run-loop digest is SPR-038b T-06, including `StreamHost::collect`'s second hash (SPR-038 T2), and its residual is BCK-002 A-68.</li><li>SPR-030 stage A keeps T-08 (h): a measurement-gated residual that applies if the digest exceeds 10% of the G1a run after 038b.</li><li>T-11 moves to stage A.</li><li>No SPR-030m.</li></ul> | 85% |
| G2 | **Split T-02.**<ul><li>(i) The machinery allocations at 1 thread and the multi-thread buffers, stated as the delta after 038a.</li><li>(ii) The persistent pool becomes a P1b spike: no P1a world has a parallel level (`--expect-serial`), and `unsafe` is forbidden. A-32's own text agrees.</li></ul> | 85% |
| G3 | **Accepted.**<ul><li>T-11 starts with a per-term callgrind breakdown.</li><li>`PurposeId` is cached in the spec purpose.</li><li>The guard design is stated: debug-only, or per job.</li><li>**Class: C2**, provided every draw is bit-identical (`gen_rng_golden.py --check`, the sampler goldens, the ladders); otherwise C3. D61 says so.</li></ul> | 75% |
| G4 | **Accepted.**<ul><li>No typed numbers: `population_band.py` reads the regenerated golden.</li><li>The CRN gate's seeds are what A-62's E4 fix chooses: nightly needs at least 2 VALID seeds; the PR tier takes the first VALID of seeds 1–4.</li><li>The band holds only with M1, M2, S3, M7 and T1 registered, and is re-derived after 022b (as K21 does).</li><li>The step calls the N4 helper.</li></ul> | 85% |
| G5 | **Accepted. Pre-register the G1a verdict:**<ul><li>engine-only `rh-cli run kingdom_10k --years 1000 --check-cadence yearly`, with no run directory and no viewers;</li><li>the stated `--threads`, with one vCPU per physical core pinned;</li><li>5 runs per seed (RUN-002 §5), seeds 1–3, and the verdict on the median;</li><li>V-PERF-09 measured separately, with viewers and 5 paired runs;</li><li>the named bound is a year-end sample;</li><li>(c)'s replay uses the recorded cadence;</li><li>(i) counts `person.named` transitions (F-1).</li></ul> | 80% |
| G6 | **Accepted.** A-67's byte clause is replaced by V-PERF-09's restated bound, which includes the seal queue (SPR-026 GAP-1). If A-67 fires, V-PERF-09 FAILs at G1a until the cap lands, unless the director waives it | 75% |
| G7, G8, G9 | **Accepted.**<ul><li>Two serial numbers: the engine's overhead outside system jobs, and the largest job's share.</li><li>`twin_vales` (300 years) joins the nightly debug soak with SPR-038's inventory gate in the sprint that first runs its genesis.</li><li>Thread invariance on `twin_vales` is declared vacuous; `smoke` is the evidence.</li><li>Compare ladders within one cadence.</li><li>Cite 038a and 038b.</li></ul> | 85% |
| G10 | **Accepted.**<ul><li>V-SF-01 gates on the seed-mean r, with a t-interval.</li><li>The population range is reported with its expected spread.</li><li>**M2's `[isolation]` expect (1) is amended the same way in the `next/M2` draft.** The Architect routes this into A-62's open PR (`[[amends]]` if the field is in the frozen slice), before SPR-015 freezes 0.3.0.</li></ul> | 85% |
| G11 | **Accepted.** Reigns come from channel A (`SUCCEED`, `DEPOSED` and the holder's `DEATH`; payload `[title]`, K7) through the DuckDB reader. There is no keyframe reader, and SPR-029 is not a dependency for it | 85% |
| G12 | **Accepted.**<ul><li>V-GOLD-03 as an Architect-run dispatch is a D61 item, with the `docs/13` text.</li><li>A `pull_request` path trigger runs the smoke case.</li><li>The mutant `rh-cli` build runs nightly.</li></ul> | 80% |
| G13 | **Accepted.**<ul><li>The `historical:<id>` files are hashed into the manifest.</li><li>The test-only forcing crate is **C2**, with an `ENGINE_CRATES` edit and a stated rule that no golden world registers it; this is a D61 item.</li><li>The residence forcer reads `inserted()`.</li></ul> | 75% |
| G14 | **Accepted.**<ul><li>The review round is counted.</li><li>Cox is stratified by polity.</li><li>The zero-variance TOST rule is stated.</li><li>Use R `survival` outputs as the reference.</li><li>Use the world names and read vitals by key.</li></ul> | 85% |
| G15 | **Accepted.**<ul><li>**SPR-021 owns the annals-fixture changes:** K2/K7, plus F-21's distinguishing child, through a reviewed spec PR.</li><li>Check 5: `INTERREGNUM_CRISIS` on a non-sovereign title only while its liege's interregnum is open at that tick.</li><li>SPR-029 reproduces the fixtures and regenerates none.</li><li>State the order against the `[C3]` regenerations.</li></ul> | 80% |
| G16 | **Cross-check, not a new file.** The exporter checks the rebuilt living founders against the chronicle's tick-0 `LIFE_ENTRY` lines (id, sex, birth tick, parents) and refuses by name on any difference. There is no `genesis.toml`, so the run-directory format is unchanged | 75% |
| G17, G18 | **Accepted.**<ul><li>Memoize `label_of`, and add a 300-year timing line.</li><li>The Python packaging is conditional on SPR-027 T-05 landing first.</li><li>The Tester's own renderer is the independent oracle.</li></ul> | 85% |
| G19 | **Accepted.** Restate the dependencies by task, against 021a/b, 022a/b, 024a/b and 038a/b:<ul><li>the dynasty and dynastic cases need 024b (`BUILT`, X6);</li><li>bench 1 needs SPR-019 stage A;</li><li>bench 3 needs 019b.</li></ul> | 90% |
| G20 | **Accepted. CI is restated per three parts** (PR #89):<ul><li>run-directory gates go to `stream`;</li><li>the hosted nightlies' minutes are checked against GOV-008's budget, or they run weekly or on `adjutant`;</li><li>the `adjutant` session is a separate job, without `check_all`, with `timeout-minutes: 300`.</li></ul> | 80% |
| G21, G22 | **Accepted.**<ul><li>State or drop F-15 and F-19; apply F-17 in PRJ-002 §5.</li><li>**Sizes:**<ul><li>SPR-028: A and B, 7–9 days.</li><li>SPR-029: one sprint, 4.5–5 days, with a checkpoint after T-01.</li><li>SPR-030: **A** (with T-11, without the pool), **B1** (golden, bands, CRN) and **B2** (session tooling and measurements), 10–12 days.</li></ul></li></ul> | 70% |
| F-20 | **Accepted.** SPR-019 T-07's mutant is renamed `mutant-m2-fertility-x2`; the Architect applies this at a close | 90% |

**D61, restated:**
1. Sizes and splits, as G22 says, with no stage M.
2. The corpus: `twin_vales` is the fifth golden world.
3. Meso benches; coverage as a nightly ratchet, after a branch-coverage probe.
4. The held-out bands stay sealed.
5. A-67 per G6.
6. The `adjutant` sessions per G20; 038b owns the digest.
7. The reading set after 024b.
8. CI per three parts.
9. Classes: T-11 (G3), the forcing crate (G13), and V-GOLD-03 as a dispatch (G12).

## 0b. Architect rulings, round 2 (2026-10-18), on `1dfc563`


The re-check returned 0 BLK, 5 MAJ, 16 MIN, 5 NIT. SPR-028 and SPR-029 are AFTER SMALL FIXES; SPR-030 NEEDS REVISION, a text-only revision 2. The report is `wave8_review/REPORT_R2.md`. **Every finding is accepted unless a row says otherwise.**

| # | Ruling | Conf. |
|---|---|---|
| C1 | **Accepted.** T-02 gates per-site zero allocations at 1 thread only. At 4 threads it gates "no growth, and allocations ≤ the 1-thread count + a recorded per-spawn constant × parallel levels". The recorded constant is the pool's P1b target. The specs row and the acceptance say so | 85% |
| C2 | **Accepted.**<ul><li>(f) is restated on SPR-026's built gate: VmHWM growth ≤ 2 × peak `bytes()` + an allowance per viewer, with the headless baseline the maximum of three runs (GAP-8). SPR-026's measured plateau of about 4–5 MB of uncounted host memory is a stated term.</li><li>A-67 fires on unsealed years > 4 (two nominal years plus slack).</li><li>The counting-allocator language is dropped.</li><li>A conditional byte-cap sprint (D42(b), "SPR-030c, if A-67 fires") is pre-planned in PRJ-002 with a size, because firing is likely.</li></ul> | 75% |
| C3 | **Accepted.**<ul><li>(i) A-68 fires only if (a) misses 60 s **and** the median minus the measured digest time is ≤ 60 s, so that the digest is the cause.</li><li>(ii) Stage A adds a probe: a 10-year `kingdom_10k` run after SPR-038b and SPR-019b, measuring the digest per stop at 10k. The Architect rules A-68's form on it **before B1**.</li><li>(iii) The risk's likelihood is "Likely".</li><li>§4(e): the reading "SPR-030 keeps T-08 (h)" is confirmed, with this probe as the early decision.</li></ul> | 80% |
| C4 | **Accepted.** The verdict's terms:<ul><li>**the clock:** the external wall time of the whole `rh-cli run` process (`/usr/bin/time -f %e`), genesis and the final ladder write included;</li><li>**the ladder:** the default `--ladder yearly`, stated;</li><li>**vitals:** `--vitals` off in timed runs, with one separate vitals run for the breakdown;</li><li>**the build:** release at the recorded commit, the repository's own Cargo config, no extra RUSTFLAGS;</li><li>**the setup check:** pre-registered and run before any time is read (affinity verified in `/proc`, no other process above 5% CPU). A run that fails it is discarded and re-run once, with the reason logged.</li></ul>(f)'s topology: V-PERF-09 is measured separately, with the engine at 4 threads on cores 0–3 and the viewers on 4–7 | 80% |
| C5 | **Accepted** as the fix says: the `g1a` job is its own workflow file in the same concurrency group; `actionlint` plus `g1a_scale.sh --dry-run` run in core; an itemised time table; ensembles at `--check-cadence yearly`; `if: always()` on the upload; `uv sync` | 85% |
| C6 | **Accepted.** `validation.yml` and `dynastic.yml` each get a `pull_request` path trigger with a reduced case, as `ensemble.yml` does. "First run green within 45 minutes" moves out of the merge gate into the first post-merge dispatch, which the Architect records at the audit. The 32-seed pre-check runs on `adjutant`. The weekly job gets a subtotal table, and is split if over 45 minutes | 80% |
| C7 | **Accepted:** swap T-06 and T-07 or state the dependency; the fallback ("if a level holds two jobs, the declaration is withdrawn and the N-thread run gated"); the pre-check at 300 years | 85% |
| C8 | **Accepted:** F-15 gains `M2…toml:788` and the `registry_delta.toml` comment. (i) gains PR #96's two reports: living members per house, and male-line depth at genesis, against `fixture_worlds.md` §3.3 | 90% |
| C9, C10 | **Accepted.**<ul><li>T-11: if the per-term breakdown shows the per-job sort-and-scan cannot meet 1.5×, the guard becomes debug-only, with a release test of the key path (DET-11's pattern).</li><li>The job-local vector is an engine-owned reused buffer.</li><li>Citations are fixed; "parallel efficiency" is defined (speed-up ÷ threads); typed derived figures cite their derivation line.</li><li>C10 is re-sized at the Tester's first pass.</li></ul> | 75% |
| S1 | 99.9% | 95% |
| S2 | **Accepted:** `check_pin` (a tool crate) covers the forcing files, with the replay check. D61 item 9 names the `manifest_v0.py`, parity (V-WC-04) and replay edits | 85% |
| S3 | **Accepted:** the replicates are recorded in the golden and recomputed only with `--resim` (nightly or weekly) | 85% |
| S4 | **Accepted:** the M7 arithmetic is stated for 500 reference years and the constant-0 runs that carry the rate clause; import SPR-021b's `check_m7_successions.py` reader; a reign ends at the triggering `DEATH` or `DEPOSED` tick | 80% |
| S5 | **Accepted:** drop the clustered-errors line. Print the HR without a CI, labelled descriptive | 85% |
| S6 | **Accepted:** the Python TUI packaging step (`package = true` and layout) is carved out of SPR-027 T-05 as a 0.25-day C1 PR, "PY-PKG", consumed by SPR-027 T-05, SPR-028a and SPR-029, with a PRJ-002 row. The Architect files it | 75% |
| S7 | Edits. The F-20 rename (`mutant-manor-fertility-x2` → `mutant-m2-fertility-x2`) is applied by the Architect in SPR-019 at this close | 90% |
| A1 | **Accepted. A new spec PR, "AN-1: annals fixtures for M7 F1".** It covers K2's vassal `INTERREGNUM_CRISIS` fixture, F-21's distinguishing child, check 5's rule and its control, with goldens by `--write`. About 1–1.5 days plus one review round. It is commissioned by the Architect after T-00a merges (K2 is M7 0.6.0 text) and merges before SPR-029 T-02. **F-3's fold is its own micro-PR, "AN-0",** commissioned now. The Architect adds both to PRJ-002 §6 and the graph, and corrects SPR-021's "F-21 is SPR-029's to resolve" (`SPR-021:347`) at this close | 80% |
| A2, A3, A4 | **Accepted** as the fixes say: the neighbour method for the port's `label_of`, with the reference as the differential; the provenance memoized separately; siblings indexed for `rank`; the rung-1 ladder comparison in G16's cross-check; the edits | 85% |
| X1 | **Ruled: SPR-019b creates `worlds/twin_vales/`** (the manifest pin; no baseline, not a golden world). It is the first sprint whose genesis has twin_vales' dynasties, and every consumer (SPR-022b, SPR-024b, SPR-028b, SPR-029) follows it. Because `twin_vales` registers M2, which owns `person.id`, SPR-038's soak-inventory rule then requires its nightly debug soak: SPR-019b adds the soak line, if SPR-038b has merged, or SPR-038b does when it lands. The Architect amends SPR-019 at this close | 70% |
| X2 | **Accepted:** PRJ-002 gains one nightly-minutes ledger across the plans, which the Architect keeps. A plan whose nightly addition would exceed the golden nightly's headroom moves its case to `dynastic.yml` | 75% |
| X3 | **Accepted:** the 1,300- and 390-minute figures are dropped. D61 item 8 becomes one question to the director: "what is the real Actions allowance?" (462 PR-tier runs in 7 days). The Architect also asks the director directly now | 90% |
| X4, X5 | PRJ-002's graph and version at the union; REV-WAVE8 written at close | 90% |

**The drafter's revision 2:**
1. Merge `origin/main` after PR #95 (wave 7, minds, D60) lands, and union PRJ-002 and `decisions_pending` as §5 of the report says.
2. Apply every row except the Architect's own: S6's PRJ-002 row; A1's PRJ-002 rows and the SPR-021 correction; X1's SPR-019 amendment; S7's rename. The drafter may cite them as "filed by the Architect at the close".
3. Push once. The Architect then verifies and closes REV-WAVE8.

## 0c. The Architect's verification of revision 2 (2026-10-18), on `ad5e6fd`

Every row of §0b is present in `ad5e6fd`, either applied or cited as the Architect's. The Architect's rows are applied in the closing commit (BLUF). `gen_manifest --check` builds cleanly. The PRJ-002 union (0.5.13) keeps main's split graph and adds the wave-8 nodes, `S022b --> S028b`, AN-0, AN-1, PY-PKG and SPR-030c. `decisions_pending` keeps D58, D59 and D61 waiting, and D60 is decided.

## 1. Round-1 report (`dc39967`)

Reviewer: a session on a different model from the drafter's. Applied: the Architect's rulings of 2026-10-18 (SPR-038 rev 1 `88abc16` S1-S16; X1-X12; A-62 `6db3aa9`; PR #89 `a4be717`). Nothing was built; no `target/` exists.

### 1. BLUF

| Plan | Verdict | BLK | MAJ | MIN |
|---|---|---|---|---|
| SPR-028 (M9 harness; V-SF-22; coverage) | Ready after fixes | 0 | 2 | 3 |
| SPR-029 (family annals over real runs) | Ready after fixes | 0 | 1 | 3 |
| SPR-030 (scale and goldens) | **Fix before approval** | 0 | 3 | 6 |
| Across the plans, and D61 | Fix before approval | 0 | 2 | 2 |
| **Total** | | 0 | 8 | 14 |

- **The 24 findings (§4):** 19 real (the fixes of F-18 and F-24 need changing; F-16 and F-17 are not applied), 2 partly real (F-4, F-14), 1 real but already resolved by SPR-038b (F-23), 2 not stated in the text (F-15, F-19).
- **Majors:**
  - G1: stage M duplicates SPR-038b (ruling S5); its acceptance is circular. Delete it.
  - G2: the persistent pool cannot be written in safe Rust, and no P1a world has a parallel level.
  - G3: T-11 caches the wrong term.
  - G10: the V-SF-01 population clause fails a **correct** build (simulated: 69% over 32 seeds).
  - G11: SPR-028's "shared keyframe reader" does not exist.
  - G15: F-24's fix contradicts ruling K2 and adds a fourth regeneration of the annals goldens.
  - G19: M7 and T1 are not `BUILT` until SPR-024b (X6); the stage dependencies ignore it.
  - G20: CI is stated for two parts, not three.
- Sizes are not credible (G22): about 7-9, 4.5-5 and 10-12 agent-days, against 4.5, 3.25 and 8.35.

### 2. Findings

#### SPR-030

| ID | Sev | Where | Problem and evidence | Fix |
|---|---|---|---|---|
| G1 | MAJ | BLUF, T-10, T-12, division table | **Stage M duplicates SPR-038b T-06 (S5):** one kept `MerkleCache`, no clone, `snapshot()` only for keyframes and dumps. The plan was written against `fb5d68b`, where the digest was out of scope. **T-10's acceptance is circular:** it needs `kingdom_10k`'s year-1,000 keyframe and a 100-year run, but the plan places stage M right after SPR-038, before 019b (REV-WAVE6 V4: a dynastic manifest is refused until then) and before M7/T1 are `BUILT` (024b), so the measured world is not G1a's. Its compare-with-a-copy design is O(L), like 038b's stated residual (a written log column is re-hashed in full, `hash.rs:480-483`). **F-23's count is right and 038b's is short:** with a run dir each month digests at the month-end stop (1) and at the month-start stop twice (`run.rs:680`, `stream.rs:539-551`): 36 a year, not 24. | **Delete T-10 and T-12's digest parts; no SPR-030m.** Stage M keeps only: (a) one line in 038b T-06, "the stream's second snapshot and hash shares the stop's digest"; (b) a measurement-gated residual in T-08 (h): if the digest is over 10% of the run after 038b, the follow-up is row-level dirty marks for log columns from `inserted()` and `exited()` accumulated between stops (BCK-002 A-68). T-11 moves to stage A. |
| G2 | MAJ | T-02, lint row | **The persistent pool cannot be written as specified.** `dispatch` runs `impl Fn + Sync` closures that borrow engine state (`engine.rs:1326-1350`). Long-lived workers need `'static` jobs or lifetime erasure; the workspace sets `unsafe_code = "forbid"` (`Cargo.toml:22`) and `rh-core` has no third-party dependency (`PLANE_A_EXTERNAL_DEPS`). "`std` only" fails, unless every level `Arc`-clones its data (O(rows)). **No P1a world has a parallel level:** `--expect-serial` holds on every rung (my `hamlet` run: "526 levels, 0 with two or more jobs"), so the pool gives G1a nothing, as the plan admits. T-02's first bullet is also SPR-038 T-02's list. | Split T-02: (i) machinery allocations to 0 at 1 thread plus the multi-thread buffers, stated as the delta after 038a (about 1 day); (ii) the pool as a spike (a reviewed dependency, or a redesign), then P1b. A-32 itself says "before any real system runs hourly parallel levels". |
| G3 | MAJ | T-11, F-18 | **T-11 caches the wrong term.** At epoch 0 the epoch lookup misses an empty `BTreeMap` (`system.rs:455-459`), yet the cost is 1,241 instructions there (`instructions_v1.txt`). The per-draw terms are `purpose_id`, a linear `contains` plus `h64_bytes` of the purpose name every draw (`system.rs:591-594`, `rh-ids/rng.rs:129`), and the guard's `BTreeMap` insert (`system.rs:574`, which its own comment calls "most of a draw's cost"). A-46's "epoch lookup and guard" omitted `purpose_id`. The target (at most 1.5 times raw at epoch 0) is unreachable as designed; "the guard becomes an O(1) comparison" is not a design. T-11 is RNG machinery, C3 under `docs/13` §4.2; D61 item 1 names only the digest. | Begin with a per-term callgrind breakdown; cache `PurposeId` in the spec purpose; state the guard design (debug-only or per job). Put the class call in D61. |
| G4 | MIN | Band row, T-06, T-07, T-08 | **A-62 stales typed numbers.** Regenerated projection: `twin_vales` 1,725, 1,991 and 2,311 at years 100, 200, 300 (plan: 1,726, 1,995, 2,319); capped `kingdom_10k` 10,486 and 10,626 at years 100 and 1,000 (plan: 10,466, 10,569); uncapped `kingdom_10k` 42,096 at year 1,000 (plan: 38,200), so the non-vacuous band top is about 14,560. **The CRN gate has one VALID seed:** seed 1, target founder k = 106, **aged 12 (a minor sovereign)**, `T_IN` 1,188,367, re-sample missed; seeds 2 and 3 SKIPPED. "Mutants nightly at seeds 1 and 3" fails; the reference's counts are 368, 28 and 6, not 376, 38, 7. T-06's input row rests on that one seed. T-06's "extend the projection to intermediate years" is moot (A-62's blocks print 50-year steps). The M2 helper replaying N4's lists is a second implementation unless the step calls it. | Never type the numbers: `population_band.py` reads the regenerated golden. Mutants at seed 1; pin the engine's skip digests. The step calls the helper. Mirror K21: the band holds only with M1, M2, S3, M7 and T1 registered, and is re-derived after 022b. |
| G5 | MIN | T-08 | **Not measurable as written.** (1) Which time is the 60 s verdict: with a run dir, or engine-only? All of seeds 1-3, or the median? (2) `adjutant` is 16 vCPU = 8 physical cores (RUN-002:23): "`--threads 8` and viewers on cores the engine does not use" has no topology; pin one vCPU per core. (3) V-PERF-09's 95% is inside VM noise with one pair; RUN-002 §5 asks 5 runs. (4) (c)'s replay must use the run's recorded `--check-cadence` (S2). (5) (i) counts "Promote and Demote", which are commands, not channel A (F-1). (6) The named bound is a year-end sample: say so. | Pre-register these before the session. |
| G6 | MIN | A-67 row, T-08 (f), (g) | The rule's clauses are 50 times apart: about 1 MB of records a year at 10k makes 64 MiB about 85 unsealed years; the byte clause never fires first. With F-5's restated bound the queue sits inside the bound by construction. "Recorded as A-67's outcome" does not say whether G1a's V-PERF-09 passes. | Replace the byte clause by V-PERF-09's restated bound; A-67 firing = V-PERF-09 FAIL at G1a until the cap lands, or a director waiver. |
| G7 | MIN | T-03, A-45 row | **The definition degenerates.** At 1 thread, "time outside levels with two or more jobs" is about 100% on every P1a world (all levels single-job), so G1a's "serial share reported" reads 100%. | Report two numbers: the engine's overhead outside system jobs, and the largest job's share. |
| G8 | MIN | T-06 | (1) Thread invariance on `twin_vales` is vacuous (no multi-job level): declare it, assert the dispatch line; `smoke` is the evidence. (2) SPR-038 S3 requires `twin_vales` (300 years) in the **nightly debug soak with the inventory gate** in the sprint that first runs its genesis; "weekly, or in the session" does not discharge it. (3) The G1a run uses `--check-cadence yearly`, so its ladder header differs; compare within one cadence. | Add the nightly debug run and gate; keep the weekly 1,000 years. |
| G9 | MIN | Specs "A-32", Compliance | Stage A's text says "SPR-038 before stage A" and cites the single-stage plan; it must say 038a (and 038b for G1's line), with the `--check-cadence` wording. | Update. |

#### SPR-028

| ID | Sev | Where | Problem and evidence | Fix |
|---|---|---|---|---|
| G10 | MAJ | T-03 acceptance, F-14, T-06 M2 (1) | **A correct build fails "every seed's population at year 1,000 within [0.3, 5] x its start".** Expected growth is 3.13 times (r = +0.114%/yr, `selftest_m2_f1.py`). I simulated a female-line Leslie process (Poisson births, M2's schedule, 500 females, 4,000 replicates, lambda = e^0.00114): sd(ln N) at year 1,000 is 0.31 (a lower bound: no second sex, no marriage market); P(N > 5 x start) = 3.6% per seed, **69% that at least one of 32 seeds exceeds it** (71% at any time). M2's own `[isolation]` expect (1) shares the flaw. The r clause is robust (seed-mean standard error about 0.006 points). | Gate V-SF-01 on seed-mean r (t-interval); report the population range with its expected spread. Or give `demography_1000` a `density_capacity` and re-derive V-SF-02 under the density term. Amend M2 expect (1). |
| G11 | MAJ | "How the harness reads a run", T-02; SPR-029 merge route | **The shared keyframe reader is fictional.** SPR-028 reads M7's tenure, title and life-course rows "at the horizon" through a reader that "whichever of SPR-029 and this sprint lands first builds". SPR-029 builds none: it rebuilds genesis because Python cannot read Arrow IPC with zstd (F-2). Run-directory keyframes come every 10 years (`stream.rs:14-21`), so a 305-year run has none at the horizon. | Derive reigns from channel A (`SUCCEED`, `DEPOSED`, the holder's `DEATH`; payload `[title]`, SPR-021 K7), which the DuckDB reader already reads; or size a keyframe export in T-02. |
| G12 | MIN | V-GOLD-03 row, T-03, T-04 | A manual dispatch the Architect runs deviates from `docs/13` §4.2 and GOV-002 tier 13 ("32-seed ensemble on every C3 PR") and is not in D61. A workflow added in a PR is likely not dispatchable before it is on the default branch, so "runs end to end in this sprint's PR" needs a `pull_request` path trigger for the smoke case. It first gates SPR-024b (every other P1a C3 precedes it). A mutant `rh-cli` build is minutes, not "about 15 s": nightly by the plan's own rule. | D61 and `docs/13` text; the trigger. |
| G13 | MIN | T-05 | (1) `historical:<id>` files are read by `rh-manifest` but are outside `manifest_hash`, so a run is not a function of its history tuple. (2) A new plane-A crate, an `ENGINE_CRATES` edit and stub owners of real columns (`polity.exec_constraint`, `house_member.*`): "C1" needs an explicit class ruling. (3) The residence forcer should read SPR-038b's `inserted()`. | Hash the files into the manifest; rule the class in D61. |
| G14 | MIN | T-01, T-04 | The review round before T-02 is uncounted. The Cox model must be **stratified by polity** (`docs/09:477`). A TOST with zero variance (the identical-ensemble control) needs a rule. NITs: "published to six digits" for t tables and the Freireich coefficient (tables print 3-4; use an R `survival` output); the world is `fixture-demography_1000` (`world.rs:126`); read vitals by key. | As stated. |

#### SPR-029

| ID | Sev | Where | Problem and evidence | Fix |
|---|---|---|---|---|
| G15 | MAJ | Specs F-21, F-24; Completion | **F-24's fix contradicts ruling K2.** Under K2 a vassal that joins its liege's interregnum **does** emit `INTERREGNUM_CRISIS` (payload its title, `triggered` to the `DEATH`); "refuse it on a non-sovereign title" would refuse a legal event. The `ashcombe` event is still impossible (Cedric holds the Lordship when Giles dies, so no liege interregnum is open; next M7 `:733` sends a vassal with no heir to a reversion `SUCCEED`). **The fixtures are regenerated by SPR-015's `[C3]`, by SPR-021 (K2/K7 fixtures and mutant), by 024b's bump (X6) and by this sprint (F-21, F-24)**; "the 17 goldens were reproduced, not regenerated" contradicts its own spec PR. | One owner for fixture changes: SPR-021's K2/K7 work plus F-21's child. Check 5: `INTERREGNUM_CRISIS` on a non-sovereign title only while its liege's interregnum is open at that tick (the extract records tier and liege). A reviewed spec PR (W30). State the order against the `[C3]` regenerations. |
| G16 | MIN | F-2 row | **The rebuild is deterministic but unverified.** `engine_build` is `rh-golden 0.0.0 built:M2.demography` (my `run.toml`): a version and the built set, not a code hash. A worldgen change between run and export goes undetected, and `verify` re-renders from the same wrong extract, so V-NAR-05 passes over false `kf:` rows. Replay compares; the export compares nothing. It is also archive-hostile. | Cross-check the rebuilt living founders against the chronicle's tick-0 `LIFE_ENTRY` lines (id, sex, birth tick, parents) and refuse by name; better, write `genesis.toml` at run time. |
| G17 | MIN | T-04, Risks | **Scale unmeasured, and "linear in events" is false for the reference.** `label_of` is O(pool) per designation: 4.3 ms a call at a 15,000-person pool (my timing of its loop). A 300-year `twin_vales` year annal makes about 10^5 calls, about 10 minutes a render; `annals_g1a.sh` renders, verifies and differentials, 3 seeds: hours. Tests run 20 years. | Memoize (pure in pid and pool); add a 300-year timing line. |
| G18 | MIN | T-02, T-05 | "uv lock unchanged" cannot hold for the first Python package (`package = false` is `source = { virtual = "." }` in `uv.lock`; SPR-027 T-05 changes it; `python -I -m` needs an install). A transliterated port is not independent; the Tester's own renderer is. | Condition on SPR-027 landing first; say so. |

#### Across the plans

| ID | Sev | Problem and evidence | Fix |
|---|---|---|---|
| G19 | MAJ | **Dependencies against X6 and X9.** M7 and T1 join `BUILT` in 024b, so before it `rh-cli run twin_vales` or `manor` runs M2/M1/S3 only. Needing 024b: SPR-028 stage A's dynasty report ("after SPR-022"), SPR-029 T-04's dynastic case (a `SUCCEED` citing a `DEATH` cannot occur), SPR-030 stage B. SPR-030 stage A's bench 1 needs 019 stage A, and bench 3 (`kingdom_10k`) needs 019b and runs M2-only until 024b. The plans cite unsplit 021/022/024 and single-stage 038. X9's "all before SPR-028 and SPR-030" is right for 028 and 030b; stage A, 029 on the rungs, and 028's demography half may go earlier. | Restate dependencies by task with 021a/b, 022a/b, 024a/b, 038a/b. |
| G20 | MAJ | **CI after PR #89 has three parts.** All deltas use two parts against 12:00-14:43 and 12:46. Run-directory readers (SPR-029's real-run gate, SPR-028's V-SCALE-02) belong in `stream`, which builds its own `stream_inputs`; golden is the tight part. **Free-plan minutes:** GOV-008 gives about 2,000 a month and says nightly heavy work runs on `adjutant`; the plans add two hosted nightlies (25-35 min each) to the 45-minute golden nightly. **The session:** `adjutant.yml` runs `bin/check_all.sh` (30+ min) and the bench session before any appended step, inside `timeout-minutes: 120`. The ensembles and (a)-(j) are 3-5 hours by my estimate (`kingdom_10k` 32 seeds x 1,000 years is 4 waves of 5-30 min). | Restate per part from #89's first run (X10); `stream` for run-directory gates; weekly cadence or the minutes arithmetic for the nightlies; a separate `adjutant` job without `check_all`, timeout 300. |
| G21 | MIN | F-15 and F-19 are cited (SPR-030:68; SPR-028:79) and never stated. **F-17's fix is not applied:** PRJ-002 §5 `:147` still gives V-SAGA-15 to "SPR-020; SPR-025". D61 omits: V-GOLD-03 as manual (G12), T-11's class (G3), the pool decision (G2), the new plane-A crate (G13), the minutes arithmetic (G20). | State or drop; edit §5; add to D61. |
| G22 | MIN | **Sizes.** SPR-028: T-01 is a stdlib statistics library (t, beta, gamma quantiles, stratified Cox, logistic, KM, keyed bootstrap, TOST) plus lints, golden and a review round: 1.5-2 days, not 0.75; T-02 1.5; T-05 and T-06 1.5 each: 7-9. SPR-029: T-01 1.5, T-02 2 (about 1,800 lines under `mypy --strict`), plus spec PRs: 4.5-5. SPR-030: stage B is 5.5-6 (T-08 has ten sub-measurements and two tools); 10-12 in all, so stage B exceeds GOV-005 §3's norm. | Re-size; split stage B (B1: golden, bands, CRN; B2: session tooling and measurements). |

### 3. Also judged

- **The feed is sound.** Canonical files from `rh-cli annals-inputs` are the contract's own input (§1); DuckDB would put a native library with its own I/O into V-NAR-07's audit; `wire_v1` is the unsealed edge. `#segment` comments are legal (`canonical_v0.md` §1). Plane C reads plane A at rest through files and writes nothing back; the exporter calls `rh-worldgen` as `replay` does. Caveats: G16, G17.
- **`twin_vales` golden.** The corpus is right (fifth world; `kingdom_10k` not golden; `manor` a test world). The 20 s rule fixes Y once; record the measurement in the PR. See G4, G8.
- **F-20.** Real, a NIT: mutants are features of the system (`mutant-resample-hit-k1` precedent), not of a world. Rename `mutant-manor-fertility-x2` in SPR-019 T-07 to `mutant-m2-fertility-x2`; SPR-030 mirrors K21.
- **G1a protocol:** G5. **A-67 rule:** G6. **Stage M:** G1.

### 4. The 24 findings

| F | Verdict | Evidence and the fix |
|---|---|---|
| F-1 | Real | `crn_pairing_v0.md:52`, `:124` say "Promote and Demote records"; M7 `[[emits]]` lists ten kinds (`next/M7:349-385`); `Promote` is a `[[commands]]` to T1 (`:398`). Reading `person.named` between instants is sound. |
| F-2 | Real; sound with G16 | `stream.rs:17-21`: no genesis keyframe in a run directory. |
| F-3 | Real | `annals_v0.md` §9 gives the fold no oracle. `solitary` has records in years 0 and 3 (ticks 0 and 1,634,400), so a two-segment split works. |
| F-4 | Partly real (NIT) | A wording gap: REV-SPR-029's independent renderer reproduced the goldens by printing the token. One sentence. |
| F-5 | Real | VER-SPR-026-TEST GAP-1 (peak 16.0 to 25.0 MB; the seal queue uncounted); PERF-5 already allows "lateness of any seal". Fix right; G6. |
| F-6, F-7 | Real | `docs/11:189` against `docs/08:236-238` and PERF-2 "Later". Fix sound; G7 for F-7. |
| F-8 | Real | Six plans say "(SPR-030)" (SPR-014:282, 015:240, 016:186, 018:220, 019:230, 020:347). Fix sound; G12. |
| F-9 | Real | `lint_odd.py:152`, `:1108-1110`; M7's step is in `note`. A typed table is sound; G13. |
| F-10 | Real | M7 expect "computed but sealed" against GOV-002 law 11 and A-04. "Sealed means not computed" is sound. |
| F-11 | Real | GOV-002 §3 "CI blocks merge"; nightly ratchet is a director item. Probe stable branch coverage first. |
| F-12 | Real | Next M2 `:789`. Report-only is sound. |
| F-13 | Real | Next M2 `:972` (uniform ages). Synthesised ages are sound; G10 for the 5 x clause. |
| F-14 | **Partly real** | K = start population is wrong for an uncapped world (G10). |
| F-15, F-19 | Not stated | G21. |
| F-16 | Real; partly applied | §4 rows fixed, §5 not. |
| F-17 | Real; not applied | `envelope_v0.md:186`, SPR-020:325 say SPR-030; PRJ-002:147 says 020/025. |
| F-18 | Real; fix partly sound | 1,241 and 552 per draw confirmed; G3. |
| F-20 | Real (NIT) | §3. |
| F-21 | Real | **Reproduced:** a cognatic fallback in `Kinship.house_of` leaves `selftest_annals_v0.py --check` at PASS, all 17 goldens. An un-named house daughter with a houseless father is the distinguishing child. |
| F-22 | Real | **Reproduced:** stationary CDR at e0 29.95 is 3.28% (`compile_schedule`, stable weights at r = +0.114%), against `fixture_worlds.md:61`, `:99` at 2.1%; 311/323 deaths are a closed cohort with no infants. |
| F-23 | Real; resolved by SPR-038b | G1. |
| F-24 | Real in substance; fix wrong under K2 | G15. |

### 5. D61

| # | Verdict |
|---|---|
| 1 Sizes and splits | **Change.** SPR-028 split: approve, re-sized 7-9 (65%). SPR-029 one sprint at 4.5-5, checkpoint after T-01 (70%). SPR-030: two stages, not three: A with T-11 and without the pool; B split in two; no stage M (65%). Add T-11's class and the pool decision. |
| 2 Corpus | Approve (75%) if G4, G8 are applied. |
| 3 Benches, coverage | Approve (70%, 65%); coverage after the branch-coverage probe. |
| 4 Sealed bands | Approve (80%). |
| 5 A-67 | Approve with G6's clauses (60%). |
| 6 `adjutant`, SPR-038 | Approve sessions with G20's budget (70%); the division changes: 038b owns the digest. |
| 7 Reading set | Approve (75%), after 024b and G17. |
| 8 CI | **Restate** per three parts (G20). |

### 6. Checked and sound

- Mortality numbers: q(15) 0.4443 and l(65)/l(15) 0.3146 at e0 30; 0.2463 and 0.4530 at e0 45; the e0 + 15 mutant fails V-SF-02 and V-SF-02b.
- 13,000 draws a month (S8/S9: one `k = 0` draw per living person per month).
- The M7 murder mutant: about 64,000 ruler-years, about 416 expected murders, a tripled rate far outside the 99.9% interval.
- Sends to absent owners are dropped (`05.1` §2 rule 2), so the M7 isolation world registers without T1.
- D61's CI sums (36 s and 193 s; 56 s and 292 s at x1.5).
- The CRN mutants' reasons; the `ALLOC_GATED`, `BENCH_TARGETS` and `regen_baseline` citations.

### 7. What I ran

Scratch: `.../scratchpad/wave8_review/work/`.

- `rh-cli` (the ci_review build): `run hamlet --years 40 --run-dir`, `tail --canonical`; `run.toml` and the dispatch line.
- `compile_schedule` at e0 30, 45, 37, 23.4 (`mort.py`).
- A Leslie Monte Carlo, 4,000 replicates (`mc.py`).
- A probe: the cognatic `house_of` patch, then `selftest_annals_v0.py --check`.
- A timing loop of `label_of`.
- `git diff 3e2e6ef origin/arch/M2-elite-couple` on the projection and CRN goldens.
- Code read at the cited lines.

## 2. Round-2 report (`1dfc563`)

The re-checker ran on a model different from the drafter's. The base is revision 1, `1dfc563`, which merged `1950df8`; it was also checked against `main` `78fef08` (PR #93, PR #96). Nothing was built. The Architect recorded this report from the re-checker's returned text. Its scratch work is in `wave8_review/work2/`.

### 1. BLUF

| Plan | Verdict | BLK | MAJ | MIN | NIT |
|---|---|---|---|---|---|
| SPR-028 (M9 harness) | AFTER SMALL FIXES | 0 | 0 | 6 | 1 |
| SPR-029 (family annals) | AFTER SMALL FIXES, plus one cross-plan edit (A1) | 0 | 1 | 2 | 1 |
| SPR-030 (scale, `twin_vales`, G1a) | NEEDS REVISION: a text-only revision 2 and three Architect decisions | 0 | 3 | 5 | 2 |
| Across the plans, PRJ-002 and D61 | AFTER SMALL FIXES | 0 | 1 | 3 | 1 |
| **Total** | | **0** | **5** | **16** | **5** |

- **Rulings G1–G22:** 17 APPLIED, 3 PARTIAL (G1, G15, G19), 0 NOT APPLIED. The defects lie mostly in what the rulings left unspecified, or in facts that moved underneath them.
- **The five MAJ:**
  - C1: T-02's zero allocations at 4 threads is impossible while `thread::scope` stays.
  - C2: V-PERF-09's memory term and A-67 clause 2 are stale against SPR-026 as built.
  - C3: A-68's trigger is vacuous and comes late.
  - A1: the annals spec PR has no owner.
  - X1: `twin_vales` is not runnable before SPR-030b1.
- **The pre-registered G1a verdict** is falsifiable, but its terms are incomplete (C4).
- **The drafter's choices are sound:** F-25, the 1.25 factor (reproduced with a Leslie Monte Carlo), F-22, and the F-3 fold (though it is gated on the unowned PR). The "SPR-030 keeps T-08 (h)" reading is defensible but forfeits an early A-68 decision.
- **Merge tests:** two conflicting files, PRJ-002 and `decisions_pending`, against both `main` and wave 7.

### 2. Rulings G1–G22

| # | Verdict | Note |
|---|---|---|
| G1 | PARTIAL | Stage M is deleted, T-11 moved to stage A, and A-68 is a residual. But T-08 (h) sits in stage B2, and the plan does not say it departs from "stage A keeps (h)" |
| G2–G14 | APPLIED | The defects found are C1, C2, C4, C9, S1, S2 and C6 |
| G15 | PARTIAL | The wave-8 side is applied; SPR-021's plan and PRJ-002 §6 do not carry the task (A1) |
| G16–G18 | APPLIED | Defects A2, A3 and S6 |
| G19 | PARTIAL | The runnable `twin_vales` dependency (X1) and the `S022b --> S028b` edge (X4) are missing |
| G20–G22 | APPLIED | Defects C5, X3, C8, C10 and S7 |

### 3. New findings

#### SPR-030

| ID | Grade | Problem | Fix |
|---|---|---|---|
| C1 | MAJ | T-02 wants every machinery site of `smoke` at 4 threads at 0 allocations, while keeping `thread::scope`. Dispatch and the scoped spawns allocate: `engine.rs:1337–1360` shows 819,401 allocations at 1 thread against 828,161 at 4. Sites are attributed only at 1 thread (`alloc_budget.rs:14`, `:23`, `:53`; A-32) | Gate per-site zero at 1 thread only. At 4 threads, gate "no growth, and allocations ≤ the 1-thread count + a recorded per-spawn constant", and make the constant the pool's P1b target |
| C2 | MAJ | V-PERF-09's memory term and A-67 clause 2 are stale. On SPR-026's head, `bytes()` counts the seal queue, and `tests/memory.rs` gates VmHWM growth ≤ 2 × peak `bytes()` + 2 MiB. The plan's counting allocator in `rh-cli` is forbidden by `lint_arch` (global-allocator). Clause 2 is tautological, so only clause 1 (unsealed years > 4) can fire, and it will (sealing was 3–10× slower than the simulation in SPR-026's runs) | Restate (f) on SPR-026's gate; A-67 is unsealed years > 4; pre-plan the byte-cap sprint conditionally; drop the counting-allocator language |
| C3 | MAJ | A-68 fires if the digest exceeds 10% of a run. By SPR-038's arithmetic (35–142 s per 1,000 years) that happens on every run, even a passing one. And it is decided at B2, after B1, the last C3 of P1a | Trigger only on a miss: A-68 if (a) misses 60 s and the median minus the digest time is ≤ 60 s. Add a stage-A probe on a 10-year `kingdom_10k` run so A-68's form is ruled before B1. Raise the risk to Likely |
| C4 | MIN | The verdict's terms are incomplete: the clock, `--ladder`, `--vitals`, the build, and a setup-failure rule. Also (f) pins viewers to free cores while (a) uses all 8 | Add the five terms, and state (f)'s engine topology |
| C5 | MIN | The `g1a` job text contradicts itself: both jobs run on a dispatch; a hosted dry run is impossible on a self-hosted runner; the hours are not derived; no `if: always()` and no `uv sync` | Give the job its own workflow; `actionlint` plus a `--dry-run` in core; an itemised time table; ensembles at `yearly`; `if: always()`; `uv sync` |
| C6 | MIN | "First run green within 45 minutes" cannot be met before merge for `dynastic.yml` and `validation.yml`, which have only schedule and dispatch triggers. The weekly job has no subtotal | A `pull_request` path trigger with a reduced case, or move "first run" out of the merge gate. Run the pre-check on `adjutant`. Add a subtotal table or split the job |
| C7 | MIN | T-06 and T-07 are in the wrong order; "declared vacuous" has no fallback; the band pre-check runs at 100 years, where the ratio is 0.96–0.98 | Swap them or note the dependency; add a fallback that gates the N-thread run; run the pre-check at 300 years |
| C8 | MIN | F-15 misses `M2…toml:788` and the `registry_delta.toml` comment. PR #96's disposition asks the G1a report for living members per house and male-line depth at genesis | Add both |
| C9 | NIT | T-11's 1.5× target at epoch 0 looks unreachable with a per-job sort-and-scan; the job-local vector must be an engine-owned buffer; citations have drifted; "parallel efficiency" is undefined; typed derived figures need their source line | Edits |
| C10 | NIT | T-08 at 3.0–3.5 days is the likeliest overrun | Re-size at the Tester's first pass |

#### SPR-028

| ID | Grade | Problem | Fix |
|---|---|---|---|
| S1 | MIN | T-03's acceptance says "90%"; the rest of the plan says 99.9% | Make it 99.9% |
| S2 | MIN | The history-tuple fold is mislocated: `manifest_hash` is computed in `rh-golden`, and `rh-manifest` has no SHA-256. The change also touches the Python reference, the parity gate and replay's check | Extend `check_pin` (a tool crate) to cover the forcing files and the replay check; name the parity and spec edits in D61 item 9 |
| S3 | MIN | 400 seeded Leslie replicates in about 5 s is impossible in Python (at least 12 s) | Record them in the golden; recompute only with `--resim` |
| S4 | MIN | The M7 murder arithmetic assumes 1,000 reference years, but M7's expect steps the state at year 500; the reign reader duplicates SPR-021b's `check_m7_successions.py`; the reign-end tick is unstated | Edits; import the reader |
| S5 | MIN | "Polity-clustered robust errors" with 2–3 clusters cannot be estimated | Drop it, or print the HR without a CI, labelled descriptive |
| S6 | MIN | Priority inversion: stage A and SPR-029 both need SPR-027 T-05's Python packaging, the last task of a P2-adjacent plan | Carve it out as a 0.25-day C1 PR |
| S7 | NIT | The mutant-feature list; the F-20 rename is pending (`SPR-019:194`); `--vitals` is a stdout line; T-05 plus T-06 sizing is thin | Edits |

#### SPR-029

| ID | Grade | Problem | Fix |
|---|---|---|---|
| A1 | MAJ | The "annals spec PR that SPR-021 owns" has no owner. SPR-021 says F-21 is "SPR-029's to resolve" (`SPR-021:347`) and that the annals goldens are SPR-024b's (`:278`); PRJ-002 §6 has no row | The Architect adds the task and a PRJ-002 row, with a size and a reviewer; consider splitting the fold out as a micro-PR |
| A2 | MIN | G17's memo is still O(distinct pids × pool): about 132 s for 18,000 labels. A sorted-neighbour method gives identical labels on a sample and labels all 18,000 in 0.05 s | Specify the neighbour method for the port; keep the reference as the differential; memoize the provenance side effect; index siblings for `rank` |
| A3 | MIN | G16's cross-check misses drift in `house_member`, tenure and title rows | Advance the rebuilt genesis one year and compare rung 1 of `run.ladder` |
| A4 | NIT | Golden filenames with hashes; the A-52 treatment of `mutant-worldgen-sex-flip`; K7 overstated | Edits |

#### Across the plans

| ID | Grade | Problem | Fix |
|---|---|---|---|
| X1 | MAJ | `twin_vales` has no world directory before SPR-030b1 (`world.rs:127–137`), yet SPR-029 T-04, SPR-028b T-06 and SPR-024b's checkpoint run it | Create `worlds/twin_vales/` (manifest pin only) early, or add edges; pick one |
| X2 | MIN | The golden nightly's 12.8 minutes of headroom is claimed by several plans with no ledger | One nightly ledger across all plans |
| X3 | MIN | The GOV-008 arithmetic omits the PR tier: 462 runs in 7 days | One question to the director: what is the real Actions allowance? |
| X4 | MIN | PRJ-002's graph lacks `S022b --> S028b` and has unsplit nodes; its version collides | Fix at the union |
| X5 | NIT | REV-WAVE8 does not exist yet | Written at close |

### 4. The drafter's choices

- **(a) F-25: sound.** In every P1a world a polity has one law, so the stratified HR cannot be identified.
- **(b) The 1.25 factor: sound.** The Leslie Monte Carlo gives simulated sd ÷ formula of 0.85–1.13 across 50–1,000 years and three population sizes. "Within 5%" is optimistic: it reaches 9.7% at 500 years. The pre-check should run at 300 years.
- **(c) F-22: sound.** The stable crude death rate is 3.28%.
- **(d) The F-3 fold:** technically sound, but it should be a micro-PR.
- **(e) "SPR-030 keeps T-08 (h)":** defensible, not the letter. The Architect confirms it and adds the stage-A probe.

### 5. Merge tests

- **Against `main` (`78fef08`) and against wave 7 (`8e81b8f`):** two files conflict, PRJ-002 and `decisions_pending`.
  - **PRJ-002:**
    - take main's split graph;
    - add the wave-8 nodes with their edges re-pointed, including `S022b --> S028b` and an annals-spec-PR node;
    - `classDef crit` is the union;
    - the G1a table and the wave rows are the union;
    - bump the version to the next free number.
  - **`decisions_pending`:** keep D58, D59 and D61 waiting, and do not resurrect D60's waiting row.
- `gen_manifest --check` passes.
