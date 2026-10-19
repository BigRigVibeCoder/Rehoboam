---
id: REV-SPR-038
title: "Independent review of the SPR-038 plan (BCK-002 A-60, the per-instant engine cost) and D60"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [SPR-038, BCK-002, PRJ-002, SPR-014, SPR-015, SPR-018, SPR-020, SPR-021, SPR-023, SPR-030, SPR-007, REV-WAVE6]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing commit (2026-10-18). The plan goes to the director as D60.**
>
> | Stage | Revision | Size | Class |
> |---|---|---|---|
> | A: exact gating, O(N+L) checks, positional compaction | 0.3.1 (revision 3, `6f5e54a`) | 3–4.5 agent-days | C2 `[perf-baseline]`; no baseline moves |
> | B: the cadence parameter, compiled-out value checks, the incremental digest with append marks, the instant log (a) and (b), the one `DropReason` change | the same | 2.75–3.75 agent-days | C2 `[perf-baseline]` |
>
> - **Round 1** (`fb5d68b`): 0 BLK, 7 MAJ, 7 MIN, 2 NIT (§1).
>   - The review reproduced the diagnosis (about 92k instructions per instant on `solo`) and verified that the month-end clone-and-hash blocks G1a.
>   - The majors: the release `Yearly` default (S1–S3), compaction (S4), the digest's owner (S5), order and size (S6, S7).
>   - Rulings: §0. The digest became incremental and hash-identical in stage B, as a C2 change, because `TableDigest::of` and `MerkleCache` share `chunk_hash` and `table_root`.
> - **Round 2** (`88abc16`): AFTER SMALL FIXES, 0 BLK, 2 MAJ, 7 MIN, 2 NIT (§2).
>   - With births, the digest still re-hashes the log columns and re-encodes the agenda (T1). So the plan adds append marks, files A-68 unconditionally, and D60 says "necessary, not sufficient".
>   - The stream host's second hash is T2.
>   - Rulings: §0b. Revision 2 is `2d800c9`.
> - **Cross-plan ruling R14** (round 2 of the wave-7 and minds re-check): stage B owns the one `DropReason` change. This is revision 3, `6f5e54a`.
> - **Architect's verification of revision 3:** every row of §0, §0b and R14 is present.
>   - Append marks: `SystemView::append()` and `Appender`; the mutant `append-rehashes-all`.
>   - A-68 is filed.
>   - `StreamHost::collect` shares the stop's digest.
>   - The cache's lifetime, `state_ref` and the release cross-check are stated.
>   - The cache test is in `rh-snapshot`; `reference-gating-off` is a feature.
>   - The instant log yields PersonIds in ascending order.
>   - `ledger_release` is in part golden.
>   - "Ledger-writing" means owning `person.id`.
>   - The cadence goes in `run.toml` only when it is not the default.
>   - The 20k target multiplies the 17,833 baseline.
>   - `DropReason`: `Ended` takes a generic meaning; `Deferred` and `cancel_because` are added.
>   - The graph has S038b → S021a and S038b → S023.
>   - D60 is restated with a confidence per item, and says "necessary, not sufficient" with the arithmetic.
>
> **Reviewers:** two independent sessions, each on a model different from the drafter's.

# REV-SPR-038: independent review of the SPR-038 plan

## 0. Architect rulings, round 1 (2026-10-18), on `fb5d68b`

An independent review of `fb5d68b`, made against `main` `3e2e6ef` and SPR-014's `921d372`, found **0 BLK, 7 MAJ, 7 MIN and 2 NIT** (§1). It reproduced the diagnosis and confirmed the drafter's finding 3: the month-end clone and hash block G1a. **Every finding is accepted unless a row says otherwise.**

The drafter applies these rulings in one revision on `arch/spr038-plan`, merges `origin/main` if it has moved, and pushes once **without force**. The Architect then verifies the revision.

| # | Ruling | Confidence |
|---|---|---|
| S6 + S7 (MAJ) | **Split into two stages in one plan document, as SPR-019 did.** Each stage is its own PR, with its own Tester pass and audit. Total size about 6–7 agent-days.<ul><li>**Stage A (about 3 days, C2, exact):** T-01 (the before-figures, the bench and the pricing of release `EveryChange`); T-02 (`ColumnId` slots; every put-back still takes a new version); the gating of compaction, `Ledger::check`, the invariants and the lagged-read snapshot, with **`EveryChange` semantics in every build**; the O(N+L) `Ledger::check` and M2 invariants; positional compaction (S4).</li><li>**Stage B (about 3 days, C2):** the release-build rule (S1–S3); the incremental, hash-identical run-loop digest (S5); the no-work rule (S12); the `inserted()` API (S13); the targets (S8–S10).</li><li>**Merge order:** SPR-014, then SPR-020 and stage A in either order, then SPR-015. SPR-015 may be developed in parallel; it merges `main` after stage A and hooks its derived index onto stage A's write set. Stage B lands in any gap after stage A, before SPR-018 if possible, and before SPR-030 in any case.</li></ul> | 70% |
| S1 (MAJ) | **The check cadence is an explicit run parameter.**<ul><li>It is `--check-cadence {every-change,yearly}` in `rh-cli` and `RunArgs`.</li><li>**The default is `every-change` in every build and host.** `yearly` is opt-in, for production and the G1a run (`05.1` §6 item 6: release is "configurable").</li><li>It is recorded in `run.toml` and in the crash artifact. In the ladder header it appears **only when it is not the default**, so the default ladders, and therefore every baseline, stay byte-identical.</li><li>T-01 prices release `every-change` on the CI gates. `rh-golden` is edited, so the "not edited" row goes.</li></ul> | 80% |
| S2 (MAJ) | **Accepted.**<ul><li>`Engine::check_now()` runs all checks once, O(N+L). Hosts call it at run end and before each keyframe, whatever the cadence.</li><li>A replay or resume whose cadence differs from the run's recorded one is refused by name.</li><li>"Fails at the same instant" is reworded to cover only violations that persist.</li></ul> | 80% |
| S3 (MAJ) | **Accepted: a standing obligation, with an inventory gate.**<ul><li>Every world that registers a ledger-writing system has a debug run in `golden_nightly.sh`.</li><li>A small check, in the inventory test or a nightly lint, refuses a world that registers such a system without one.</li><li>`twin_vales` (300 years) joins the debug soak in the sprint that first makes its genesis runnable (SPR-019b, or SPR-030 at the latest). The gate enforces this.</li></ul> | 80% |
| S4 (MAJ) | **Accepted.**<ul><li>Compaction is one scan of `person.alive`, then a positional removal per column.</li><li>The `death/n10k_*` bench is gated on it.</li><li>The person-column count after SPR-015 and SPR-021 is stated in the cost table.</li></ul> | 80% |
| S5 (MAJ) | **Changed: the month-end digest becomes incremental and hash-identical, in stage B (C2), not a C3 sprint.**<ul><li>`state_digest` hashes each world column with `TableDigest::of`, which is `chunk_hash` plus `table_root` (`rh-snapshot/src/hash.rs:162`, `:181`, `:208`). `MerkleCache::update_world` (`:515`) uses the same two functions over versioned dirty chunks. The run loop can therefore keep one plane-A cache across stops and produce the same `PlaneDigest`:<ul><li>world columns go through the cache;</li><li>the engine tables are encoded at each stop as now;</li><li>`engine.snapshot()` is cloned only for a keyframe or a dump.</li></ul></li><li>Every ladder and every baseline stays byte-identical.</li><li>The guard against a missing version bump is S11's test: cached equals full at every stop in debug, and at every rung and keyframe in release.</li><li>If any part cannot be made identical, that part becomes BCK-002 **A-68** (a C3 digest-cadence item before SPR-030), and the plan says so.</li><li>**D60's text:** SPR-038 is **necessary** for G1a. With S4 and S5 it is expected to remove the core-controlled terms, but SPR-030 measures; T-01 states the G1a arithmetic after both stages.</li></ul> | 75% |
| S8, S9, S10 (MIN) | **Accepted.**<ul><li>The debug `m2_system` target comes from T-01's measurement (probably ≤ 30 s).</li><li>`solo` ≤ 23k, so that every rung is at least 4× faster.</li><li>Both baselines on one allocator, with the 20k target stated as about 4× the stub floor.</li><li>One-sim-year `solo` and `hamlet` instruction benches join `BENCH_TARGETS`, plus a greedy-writer case. `05.1` says `write()` is for writing.</li></ul> | 85% |
| S11 (MIN) | **Accepted.** Cached `update_world` equals the full hash after every instant, in the per-instant test, plus the mutant `compact-keeps-version`. Stage A owns it (T-02), because positional compaction lands there. | 90% |
| S12 (MIN) | **Accepted.**<ul><li>`CheckCadence` (read-only checks) is separate from a test-only `Gating::Off`.</li><li>The no-work refusal is kept only as the "do not dispatch" fallback, in stage B.</li></ul> | 80% |
| S13 (MIN) | **Accepted.** Ship the API and a consumer test, with sorted insert stated. S3's conversion is SPR-018 T-03's, and SPR-038's completion does not depend on it. | 85% |
| S14, S15, S16 | **Accepted.**<ul><li>SIM-8 and V-INV-01's wording go into the docs list and D60.</li><li>Use PERF-4 rather than SIM-15. CI improvement is the ratio of the median per-part step times over 3 runs each.</li><li>Zero allocations is asserted in `instant/quiet/*` through the counting arena.</li></ul> | 90% |
| The drafter's finding 5 | Overstated. Replace it with S2's exposures. | 85% |

**D60, restated after these rulings:**
1. SPR-038 in two stages: A (exact gating, O(N+L) checks, positional compaction; C2; about 3 days) and B (the cadence parameter, compiled-out value checks, the incremental digest, the no-work rule, `inserted()`; C2; about 3 days).
2. The release rule:
   - the guards' O(rows) value checks are compiled out of release, with a test that proves it, after SPR-007;
   - the row-count and log-length checks stay in every build;
   - the invariant cadence is a run parameter whose default is `every-change`, with `yearly` an explicit opt-in for production and G1a;
   - a debug soak is required of every world that registers a ledger-writing system.
3. The narrowing removed: 85%.
4. `inserted()` as an API only: 85%.
5. The merge order above.
6. **G1a:** SPR-038 is necessary; SPR-030 measures whether it is sufficient.

## 0b. Architect rulings, round 2 (2026-10-18), on `88abc16`

The independent re-check found **0 BLK, 2 MAJ, 7 MIN and 2 NIT** and returned AFTER SMALL FIXES. The rulings are applied faithfully. Two corrections concern what the plan promises for G1a. **Every finding is accepted unless a row says otherwise.**

| # | Ruling | Conf. |
|---|---|---|
| T1 (MAJ) | **Accepted.** In a world with births, the incremental digest still re-hashes the ten `life_course.*` columns in full each month, and it re-encodes the agenda. The re-checker's microbenchmark gives about 6.2 ms per stop against 8.5 ms today.<ul><li>**Stage B adds append marks:** a column that was only appended to since its last version re-hashes from its old last chunk. `MerkleCache` already handles a resize that way (`hash.rs:480–481`), so the change is hash-identical and C2. Name the API (`append()` beside `write()`, or an append-only flag per column), give its test, and add the mutant `append-rehashes-all` (an observable cost, through the bench).</li><li>**BCK-002 A-68 is filed unconditionally,** covering an incremental agenda digest and `05.1` §6 item 6's release digest cadence (C3, because the ladder embeds the monthly chain). It is decided at SPR-030 on T-01's and T-08's measurements.</li><li>**D60 item 6:** "necessary, not sufficient", with the arithmetic: the per-stop terms after stage B, with and without append marks, and the agenda term at P = 10^4 and 3×10^4.</li></ul> | 75% |
| T2 (MAJ) | **Accepted.** T-06 covers `StreamHost::collect` (`rh-golden/src/stream.rs:540–551`). Compute one cached digest per stop that needs one, pass its hash into `collect`, and skip stops that need none. T-06's stop count for a `--run-dir` run is stated (up to 36,000 per 1,000 years today) | 90% |
| T3 | **Accepted.** State:<ul><li>the cache's lifetime: created in `run_reporting` after start or resume, primed in full at the first stop, re-primed after `perturb` by the identity rule, with a test through V-DET-11 `--inject`;</li><li>a borrowed accessor for the engine tables;</li><li>the release cross-check at keyframes and the last rung only, priced.</li></ul> | 85% |
| T4 | **Accepted.** The `MerkleCache` test moves to `rh-snapshot/tests/`; `rh-core` compares whole `WorldTables`. State how test-only `Gating::Off` is exposed (a feature, not `cfg(test)`) | 90% |
| T5 | **Accepted.**<ul><li>`inserted()` yields PersonIds, not rows.</li><li>`exited()` is ascending by PersonId across the instant, matching SPR-023.</li><li>A reader declares no extra read; it must run in a later phase or level than the merge it reads.</li><li>A newborn who dies in the same instant is in both `inserted()` and `exited()`.</li></ul> | 85% |
| T6 | **Accepted.**<ul><li>This plan's PR adds SPR-015's sentence: "`MergedWrites` is built from the engine's write set (SPR-038a); M2 0.3.0's new invariants are O(N+L) in stage A's pattern, with its oracle."</li><li>PRJ-002: SPR-015's row gains 038a; SPR-023's row gains 038b; a 038b → SPR-028 edge is added.</li><li>SPR-021's stale "yearly in release" (line 83) is fixed at the wave-7 close.</li></ul> | 85% |
| T7 | **Accepted.** `ledger_release.rs` goes to golden (the release part). The X10 per-part sums are computed. The moot row is dropped | 90% |
| T8 | **Accepted.** "Ledger-writing" means owning `person.id`; `smoke` is excluded by that definition; the claim is corrected to three `world.toml`. The gate's enforcement for `twin_vales` is conditional on a `worlds/` directory, and the plan says so | 90% |
| T9 | **Accepted.** D60 shows ranges: A 3–4.5 days, B 2.5–3.5 days | 70% |
| T10, T11 | **Accepted.**<ul><li>The S14 wording reads "after every change unless the run sets `yearly`".</li><li>`check_cadence` is written to `run.toml` only when it is not the default.</li><li>The 20k target multiplies the SPR-014 head's 17,833 baseline, as re-recorded by stage A's `[perf-baseline]` commit.</li></ul> | 90% |

**Round 3:** one push. The Architect verifies it, then closes REV-SPR-038.


## 1. Round-1 review (`fb5d68b`)

### Independent review of the SPR-038 plan (BCK-002 A-60), `arch/spr038-plan` at `fb5d68b`

The review is by an independent reviewer on a different model from the drafter's. It was checked against `main` `3e2e6ef` (docs and tools) and against `sprint/SPR-014-lifecycle-queues` `921d372` (code). The reviewer's scripts and logs are in `spr038_review/work/`. The subagent could not write files, so the Architect recorded this report from its returned text.

#### BLUF

**Verdict: approve with changes. D60 is not to go as written: items 1, 2 and 5 change.** The counts are 0 BLK, 7 MAJ, 7 MIN and 2 NIT.

- **The diagnosis is right, and the reviewer reproduced it.**
  - `solo` costs 92k instructions per instant on a build of `921d372`; the plan says 93k.
  - The cost shares match: column moves 40%, guards about 16%, check plus compact 13%, and `memcmp` 21% self.
  - At N = 10^4 the cost grows with L: a quiet day takes 1.1 ms at L = 10^4 and 4.0 ms at L = 1.1×10^5.
- **C2 is the right class.** For runs that violate no invariant, the gating is exactly neutral. The reviewer found no hole, only things the plan must also state or test (S11, S12).
- **The release-build rule is where the plan goes wrong.**
  - Compiling the guards' value checks out of release is sound, with SPR-007 as precedent.
  - `Yearly` as the default for every release binary is not. The docs say "debug and CI", and CI's golden jobs are release builds.
  - `Yearly` leaves the last partial year and every keyframe cut unchecked, and it makes crash artifacts non-portable between profiles.
  - It also rests on a coverage premise that is false for the dynastic systems (S1–S3).
  - The alternatives omit the one that matches the docs: O(N+L) checks, `EveryChange` for CI and test hosts, and `Yearly` only for production and G1a runs.
- **SPR-038 does not make G1a feasible, and D60 must say so.**
  - The month-end digest (the drafter's finding 3) is confirmed. It costs about 8.5 ms per stop at N = 10^4, L = 1.1×10^5, which is 100–270+ s per 1,000 years against a 60 s budget.
  - Compaction as designed ("in place") is a second term of the same size (S4).
- **The order and size do not fit CI.** Core is at 14:48 against a 15:00 hard limit, so the relief cannot wait for SPR-020 and SPR-015. The plan's 3.25–3.5 days is about half the real work (S6, S7).

#### Findings

| ID | Sev | Where | Problem | Evidence | Fix |
|---|---|---|---|---|---|
| S1 | MAJ | Specs table "release-build rule"; D60 item 2 | Default `Yearly` in release is also the default for CI's release runs, and it is not priced against the alternative that matches the docs | `05.1` §6 item 6, SIM-8 and DET-11 say "CI and debug", with release "configurable". `check_all`'s golden runs are `--release`, and the plan says `rh-golden` is not edited. Release `EveryChange` on the q15 test with O(N+L) checks: about 3.4×10^3 busy instants × 1.3M instructions × 32 seeds ≈ 45 s CPU, about 12 s on 4 cores (an estimate) | Make the cadence an explicit run parameter (`rh-cli --check-cadence`, `RunArgs`, recorded in the run header). Default `EveryChange` for debug, test and CI hosts; `Yearly` is the production and G1a default. Price release `EveryChange` in T-01 first. Drop the "rh-golden not edited" row |
| S2 | MAJ | Specs table, cadence row; compliance | `Yearly` has three holes, and two of the plan's statements are false. (i) The last partial year and every keyframe go unchecked. (ii) "Fails at the next yearly check" is false for a transient violation that heals. (iii) "The crash artifact differs anyway because `build` differs" is false | (i) `year_end_in` = `year·mpy − 1` (`rh-golden/run.rs:368`). A 200-year run never checks year 200, and a keyframe cut at a year end precedes the check. (iii) `provenance.build` = `"rh-golden <ver> built:…"` (`world.rs:256`, `run.rs:299`) carries no profile. `Engine::replay` (`engine.rs:733–780`) returns `NoFailure` for a debug artifact replayed in release and `Mismatch` for the reverse. The ladder's `failed {tick}` line (`run.rs:670`) also differs by profile | Add `Engine::check_now()`, called by hosts at run end and before each keyframe (O(N+L), once). Put the cadence in the artifact and the ladder header, or refuse a cross-cadence replay by name. Reword "fails at the same instant" so it covers persistent violations only |
| S3 | MAJ | Risk 2; specs table | The compile-out premise ("every system runs under the full guards in the nightly debug soak") holds only for M2 | `golden_nightly.sh:108–116` soaks only `household` and `hamlet`. `twin_vales`, `kingdom_10k` and `manor` (M7, S3, T1) have no debug run anywhere | Make a debug run of every world that registers a ledger-writing system a standing obligation, with a small inventory gate. Add `twin_vales` (300 years) to the nightly debug soak before SPR-030 |
| S4 | MAJ | "Known cost terms"; T-03 "compaction in place" | Compaction and insert are O(N·C) per death or birth, and the plan's design buys almost nothing | Microbenchmark at N = 10^4, in µs per death at 9 / 15 / 25 columns: current collect 71 / 109 / 309; in-place retain 63 / 93 / 161; locate and `Vec::remove` 11 / 17 / 26; insert 7 / 12 / 22. At 333k deaths per 1,000 years: 24 / 36 / 103 s now, 21 / 31 / 54 s "in place", 3.5 / 5.6 / 8.5 s with positional removal | Specify compaction as one scan of `person.alive`, then positional removal per column. Gate the `death/n10k_*` bench on it |
| S5 | MAJ | Out of scope (digest); D60 text | "What G1a needs", yet the largest G1a term is out of scope with no owner before SPR-030 | The drafter's finding 3 is verified (below) | D60 states that this sprint is necessary but not sufficient. Name the digest's owner and slot |
| S6 | MAJ | Merge route; D60 item 5 | The order "after SPR-014, SPR-020, SPR-015" leaves core no headroom | CI: core 14:48, golden 11:11 (run 39078266492). PR #83 records golden at 8:13 (run 39077091382), a 36% swing. A-63 splits golden, not core | Split along the exactness line. **038a** (exact): T-01, T-02, the gating with `EveryChange` semantics in every build, and the O(N+L) checks, landing right after SPR-014 (or SPR-020) and before SPR-015. **038b**: the release rule, the no-work rule, `inserted()` and the digest follow-up (65%) |
| S7 | MAJ | Size, T-01 to T-05 | 3.25–3.5 agent-days is about half the work | T-03 alone carries the write set, four gates, the cadence enum, `check_stats`, lazy pre-images, the compile-out and its release test, the no-work rule with six modes, two O(N+L) rewrites with differential oracles, the SPR-015 hook, three `05.1` sentences, four mutants with rebuild controls and 10+ planted violations | T-01 1.0, T-02 1.5–2, T-03 2–2.5, T-04 0.5, T-05 1.0: about 6–7 days. Present 038a and 038b at about 3 days each |
| S8 | MIN | Target: debug `m2_system` ≤ 15 s | Probably unreachable | CI's debug gate takes 82 s on 39078266492 (the plan says 99 s), against about 4 s before SPR-014. A 4.6× cut gives about 21 s. Debug/release per quiet instant is 7× | Set the target from a T-01 measurement (probably ≤ 30 s) |
| S9 | MIN | Targets table | (a) `solo` ≤ 25k is 3.7×, while "≥ 4×" needs ≤ 23k. (b) The 93k figure is from glibc malloc, while `instant` runs on the bump arena. (c) `draws/empty` is not "16.6k per instant" | (b) `perf_gate.py:230` uses `arena-alloc`, and malloc, free and realloc are about 15% of Ir. (c) `advance_until(1440)` is inclusive (`engine.rs:693`), so the bench runs 2 instants plus loops: about 5k per instant. The baseline of 16,611 is stale: the SPR-014 head measures 17,833 | Align (a). State both baselines on one allocator. Note that 20k is about 4× the stub floor |
| S10 | MIN | `instant` gating; flags | The only per-instant regression gate is a stub, and `write()` sets the flag at the call, not on a change | Nothing gates real M2 or `solo` per-instant cost. A greedy `write()` defeats the gating | Add a one-sim-year `solo` and `hamlet` instruction bench to `BENCH_TARGETS`, and a "greedy writer" case. `05.1` says `write()` is for writing |
| S11 | MIN | Risk 9; T-02 | The version discipline has no test that can fail | `rh-golden` hashes every stop in full (`run.rs:683`). `MerkleCache::update_world` is used only by tests and benches, so a missing version bump is invisible. In-place compaction must bump 9 or more versions | Add a check after every instant that cached `update_world` equals the full hash. Add the mutant `compact-keeps-version` |
| S12 | MIN | `CheckCadence::EveryInstant`; the no-work rule | (a) `EveryInstant` also turns off the state-affecting gates. (b) The no-work refusal is redundant with the lazy pre-image | | (a) Split `CheckCadence` (read-only) from a test-only `Gating::Off`. (b) Keep the refusal only as the "do not dispatch" fallback |
| S13 | MIN | T-04; D60 item 4 | Completion depends on SPR-018's merge order, and the value is small (about 3 s per 1,000 years) | The buffer must stay sorted across merges | Ship the API and a consumer test; the conversion belongs to SPR-018 T-03. State the sorted insert |
| S14 | MIN | Docs list | The requirement texts are not updated | `docs/03` SIM-8 and `docs/09` V-INV-01 ("after every pulse") | Add both to the docs list and to D60 |
| S15 | NIT | Compliance; risks | SIM-15 is "no minimum-N assumptions". "Gate-time sums lower, ×1.5" is ambiguous given the 36% swing | | Use PERF-4, or a ratio of medians over N runs |
| S16 | NIT | T-02 acceptance | "0 allocations on a quiet instant" cannot be read from `alloc_budget`'s report | | Assert 0 in `instant/quiet/*` through the bench's counting arena |

#### The drafter's five findings

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| 1 | Cost grows with L | True | At N = 10^4, a quiet day takes 1.12 ms at L = 10^4 and 3.99 ms at L = 1.1×10^5; a death day 1.31 and 10.07 ms. At L = 1.1×10^5: 44M instructions per instant (M2's invariants 19.3M, `run_passes` 15.6M, `Ledger::check` 8.8M, compact 0.7M) |
| 2 | M2's invariants are O(D log N) | True | 0.074 ms at L = 10^4 and 2.19 ms at 1.1×10^5. There is also an O(N log N) sort (`demography.rs:535–592`) |
| 3 | The month-end clone-and-hash is about 10^12 instructions and blocks G1a | **Verified; it blocks G1a** | `run.rs:683` hashes in full at every stop: every month end, and every month start with `--run-dir`, so 12,000–24,000 stops per 1,000 years. At 1.19M cells, `snapshot()` takes 4.2 ms and `state_digest` 4.3 ms, 8.5 ms per stop. That is 102 s at 12k stops, about 200 s at 24k, and about 270 s with L = 3×10^5 |
| 4 | Coordination with SPR-015, SPR-020 and SPR-018 | Sound, with caveats | SPR-015's `apply(&mut Index, &MergedWrites)` is feasible. Only S6 declares a `cause_column`. S3's discipline over `write()` decides whether the gating works (S10) |
| 5 | Release tests that expect a precise failure instant | Overstated | No release gate asserts a failure tick. The real exposures are in S2: the ladder's `failed` lines, crash-artifact replay, and bisecting a CI release ladder against a debug one |

#### D60, item by item

| Item | Verdict |
|---|---|
| 1. One sprint, 3.25–3.5 days | **Change:** split into 038a and 038b, about 6–7 days in total |
| 2. Release-build rule | **Approve in part (55%):** compiling out the guards' value checks, yes; `Yearly` as the default for every release binary, no |
| 3. Narrowing removed | **Approve (85%):** every column is a `Vec<i64>`, and `SystemView::write` is the only mutable path. On `921d372` only M2's `Day/Vital` is `drain_only`, and its empty drain writes nothing |
| 4. `StepCtx::inserted()` | **Approve, low priority:** decouple it from SPR-018 |
| 5. Order | **Change:** 038a goes before SPR-015 |
| Targets | **Change:** S8, S9 and S10 |

#### Checked and sound

- The cost table, reproduced.
- C2 neutrality for non-violating runs:
  - every check is a pure function of `&WorldTables`;
  - `SystemView::write` is the only mutable path;
  - construction and restore mark everything;
  - the flags are ORed in job order at the serial put-back.
- The lazy pre-image is a faithful replacement.
- The mutants are observable.
- Registration ties `drain_only` to the system's own `applies`.
- `perf_gate`'s handling of a decrease.
- The bench worlds can be built.
- The CI times, the instants per year and the SPR-020 line counts.

#### What the reviewer ran

- A probe on a 10^4-founder M2 world, built in release and debug at `921d372`.
- Callgrind on that probe and on `solo` over 60 years.
- Wall-clock runs of the three rungs.
- A compaction microbenchmark.
- Step times of the CI runs.
- About 60 citation checks on the right trees, with none found wrong.
- **Not run:** `check_all`, the iai benches and `nextest`.

## 2. Round-2 re-check (`88abc16`)

### Re-check of the SPR-038 plan, round 2 (`arch/spr038-plan` at `88abc16`)

The independent re-checker worked on a different model from the drafter's. It checked the plan against the code at `921d372`, PR #89 at `a4be717`, and `main` at `3e2e6ef`. The Architect recorded this report from the re-checker's returned text.

#### BLUF

**AFTER SMALL FIXES.** 0 BLK, 2 MAJ, 7 MIN, 2 NIT.

- **Rulings applied:**
  - All of S1–S10, S12–S16, X1, X9 and K33 are applied.
  - S11 and X10 are partly applied: S11's test is in the wrong crate, and X10 has no per-part sums.
  - The instant log (X1) is under-specified in places.
- **The two MAJ findings** are about what the plan promises for G1a:
  - **T1:** the incremental digest does not remove the digest cost in a world with births. It measures about 6.2 ms per stop after stage B, against 8.5 ms today.
  - **T2:** T-06 misses the second clone-and-hash in `StreamHost::collect`.
- **Stages:** stage A is self-contained and exactly neutral. Stage B's digest section needs tightening (T1–T3).

#### The drafter's additions

- **(a) A version only when written: exactly neutral, so C2.**
  - Versions enter no hash, keyframe, journal or ladder (`world.rs:33–43`). Every `.columns()` caller ignores the version.
  - `MerkleCache::update_world` has no production caller at `921d372`.
  - The identity rule holds: `Clone` assigns a fresh identity, and `hash.rs:515–518` clears the cache on a new identity, guarded by `p1_stale_cache_across_clones`.
  - A write without a change still takes a new version. That is conservative and correct.
- **(b) The ladder-header line: neutral by construction.** It follows the `leak thread` precedent (`ladder.rs:199–200`).
- **(c) `solo` in the debug soak:** about 30 s before stage A and about 8 s after. Negligible.
- **(d) The column counts 9, 14, 15, 16, 17:** verified.

#### The instant log

- Adding part (c) is additive; T-07 pins parts (a) and (b).
- "Scan only where `person.alive` was written" is exact.
- A death and a birth at the same merge are both handled correctly.
- A row compacted at Bookkeeping stays in `exited()`, which holds PersonIds.

#### New findings

| ID | Sev | Problem | Fix |
|---|---|---|---|
| T1 | MAJ | After stage B, a world with births still re-hashes the ten `life_course.*` columns in full each month, because every birth versions them, and re-encodes the agenda at every stop. **About 6.2 ms per stop, against 8.5 ms now: 74 s at 12,000 stops and 149 s at 24,000, against G1a's 60 s.** Only the 4.2 ms clone goes away. The figures come from microbenchmarks: `update_world` takes 4.5 ms against 4.3 ms for the full digest at N = 10^4 and L = 1.1×10^5, and 13.2 ms at L = 3×10^5; the agenda takes 1.7 ms at P = 10^4 and 6.2 ms at 3×10^4 | "Necessary, not sufficient", with this arithmetic. File A-68 unconditionally: row-level or append marks, an incremental agenda, and a release cadence |
| T2 | MAJ | With `--run-dir`, `StreamHost::collect` (`rh-golden/src/stream.rs:540–551`) runs `engine.snapshot()` and `state_hash` at every month start, and the run loop (`run.rs:683`) also computes a full digest at those stops. That is up to 36,000 per 1,000 years | One cached digest per needed stop, passed into `collect` |
| T3 | MIN | The digest section is under-specified: the cache's lifetime and `perturb`; a borrowed accessor for the engine tables (`agenda.state()` clones and sorts); an unpriced release cross-check | State each, and price the cross-check |
| T4 | MIN | The `MerkleCache` test sits in `rh-core/tests`, which are hash-agnostic by SPR-007's rule | Move it to `rh-snapshot/tests`. Expose `Gating::Off` through a feature |
| T5 | MIN | The instant log lacks the element type, the order, the read requirement, and the newborn-dies-same-instant case | State all four |
| T6 | MIN | SPR-015's plan never mentions SPR-038. Missing PRJ-002 edges. SPR-021's line 83 is stale | Add the sentence and the edges |
| T7 | MIN | No per-part sums. `ledger_release.rs` sits in core | Move it to golden |
| T8 | MIN | The inventory gate would false-positive on `smoke`; there are three `world.toml`, not four | Define "ledger-writing" as owning `person.id` |
| T9 | MIN | Stage A at 3 days looks light | Ranges: A 3–4.5, B 2.5–3.5 |
| T10 | NIT | The S14 wording; `check_cadence` in `run.toml` | Reword; write it only when not the default |
| T11 | NIT | The bench baseline is ambiguous (16,611 or 17,833) | State which |
