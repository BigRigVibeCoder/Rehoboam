---
id: REV-VIEWER
title: "Independent review of the viewer sprint plans SPR-033 (stages a and b), SPR-034 and SPR-035, PRJ-003 and D63"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [SPR-033, SPR-034, SPR-035, PRJ-003, SPR-027, SPR-026, SPR-018, SPR-020, BCK-002, ADR-0036, D62]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing commit (2026-10-18). The plans go to the director as D63.**
>
> | Plan | Revision | Size | Class |
> |---|---|---|---|
> | SPR-033a: the Bevy-free core, `hamlet`'s terrain from a written definition, the isolation lint, the CI job | v1.5.0 | about 2.5 agent-days | C1 |
> | SPR-033b: the Bevy app, cameras, the render-work level-of-detail controller, the HUD, the bench, the live panel | v1.5.0 | about 2.3–2.7 | C1 |
> | SPR-034: stream ingestion and plane-B hydration | v1.2.0 | about 4.2–4.5 | C1 |
> | SPR-035: the materializer hookup and the timeline scrubber | v1.2.0 | about 4.0 | C1 |
>
> - **Round 1** (`c51f07c`): 0 BLK, 10 MAJ, 24 MIN, 6 NIT (§1). SPR-033 NEEDS REVISION; the others AFTER SMALL FIXES. Rulings in §0: split SPR-033 (A1); a level-of-detail controller that measures render work (A2); the isolation rule names the render stack, never `bevy_*` (A3); T-06 on PR #89's framework (A4); written terrain and grammar specs with Python references (A5); the P51 report is not merge acceptance (A6); no `main` push trigger (B1); a view-state digest (B2); prefetch off beside a live run (C1); a cache key that tells builds and histories apart (C2). Revision 1 is `b2c75f6`; the Architect's close `2afe53a`.
> - **Round 2** (`2afe53a`): 0 BLK, 3 MAJ, 10 MIN, 4 NIT (§2), three of them found by running code on `main`: 30-year `hamlet` and `solo` runs are refused (V1); a pty corrupts `--dump -` (V2); the two-week rule is not repository practice (V3). Rulings in §0b. Revision 2 is `0e76b69`; the Architect's round-2 edits `d2b0178`.
> - **The Architect's verification of revision 2 and the close rulings:** §0c. The full scheduled run is twice a week, not daily (F17).

## 0. Architect rulings, round 1 (2026-10-18), on `c51f07c`

The independent review returned 0 BLK, 10 MAJ, 24 MIN, 6 NIT. SPR-033 NEEDS REVISION; SPR-034, SPR-035 and PRJ-003/D63 are AFTER SMALL FIXES. The report is `viewer_review/REPORT.md`. It confirmed the drafter's facts: the Bevy date and toolchain, the 62³ arithmetic, the crate, channel A's gaps, `engine_build`, and `adjutant`'s idle rule. **Every finding is accepted unless a row says otherwise.**

| # | Ruling | Conf. |
|---|---|---|
| Bevy | **Wait for `=0.20.0`,** which enters the viewer lock at or after 2026-10-22 22:31Z, with `viewer/rust-toolchain.toml` at 1.97.1. Do not pin 0.19.1. The split (A1) makes the wait cost nothing | 75% |
| A1 | **Split SPR-033.**<ul><li>**SPR-033a (Bevy-free; about 2.5 days; starts when SPR-026 merges):** the `rh-view-core` workspace, T-02 (mesher, digests), T-03 (terrain export and voxeliser) and T-06 (the isolation lint, its fixtures, and the CI job without Bevy).</li><li>**SPR-033b (about 2.2–2.7 days; Bevy enters the lock on or after 10-22):** `rh-view`, the pin and toolchain, T-04 (the app, cameras, the level-of-detail controller, the HUD, the bench) and T-05 (the live panel).</li></ul>Each is its own PR with its own Tester pass | 85% |
| A2 | **The level-of-detail controller measures render work, not the presented interval:** CPU frame time without the present wait, or GPU timestamps, or immediate present in `auto` mode. Thresholds are relative to the refresh interval: step down when more than 5% of frames miss it; step up when measured work stays under 60% of it. A synthetic vsync-capped series must be able to step up | 85% |
| A3 | **The isolation rule names the render and window stack explicitly** (`bevy`, `bevy_internal`, `bevy_render`, `bevy_winit`, `bevy_window`, `bevy_pbr`, `bevy_ui*`, `bevy_core_pipeline`, `wgpu*`, `winit`, `naga`), never `bevy_*`. A passing fixture puts `bevy_ecs` in the root lock. A viewer rule must never block an engine PR (D62) | 90% |
| A4 | **T-06 is rewritten on PR #89, now merged** (`f00cf17`): its scope framework, three parts, the two-way inventory, `check_all.sh:14–17`'s merge rule. New workflow pins go in a new test file. Add a test that a rename from `crates/` into `viewer/` still runs golden | 90% |
| A5 | **Specs before code:** `specs/viewer/terrain_v0.md` is T-03's first commit, with a small Python reference run as a differential on a few hundred random columns (about +0.3 day). SPR-034 gets the same for `grammar_v0_e1` | 85% |
| A6 | **The P51 report is not merge acceptance.** SPR-033b merges on its headless gates, the Tester's pass and the Architect's audit. The FPS report is posted afterwards. At 033b's merge the director is asked for one 10-minute "it opens and renders" run, before SPR-034 builds on it | 90% |
| A7–A20 | **Accepted** as the report fixes them:<ul><li>A7: a surface-preserving reduction (top-most solid material); seats and routes as render-only overlay lines on the heightfield; seams stated.</li><li>A8: the cursor machine is driven by restarting `Catchup` from `status.live.next_seq` after `behind`; ask SPR-026 for a skip input only if needed.</li><li>A9: `opt-level = 2` for `rh-view-core` in dev.</li><li>A10: the report header records session type, refresh rate and PRIME mode; the verdict has a tolerance; `wayland` is an opt-in feature.</li><li>A11: a forward allow-list for the viewer's path dependencies (PRJ-003 §2 gains `rh-ids`, and `rh-core` as a dev-dependency).</li><li>A12: **the two-week rule applies to direct pins only** (a crate the viewer names in its `Cargo.toml`, at least 14 days old when it enters the lock). Transitive versions are what cargo resolves, recorded in a lock-age table. It is stated in T-01, PRJ-003 and GOV-003 (the Architect's edit), not in CLAUDE.md.</li><li>A13: run cargo with cwd `viewer/`.</li><li>A14: see B1.</li><li>A15: effects first in the cut order; drop the glTF and ktx2 features; select `bevy_pbr` and `bevy_core_pipeline` directly.</li><li>A16: add the template's Blockers and Completion sections.</li><li>A17: reword the `bevy_ecs` rationale.</li><li>A18: a hand-rolled argument parser; fix F10's wording; pin `sha2`.</li><li>A20: `terrain_golden --check` in the P51 and `adjutant` protocols.</li></ul> | 85% |
| B1 | **No push trigger on `main`.** A daily scheduled run plus `workflow_dispatch`, with the cache's `save-if` on that schedule (it doubles as the keep-alive) and viewer caches keyed on `hashFiles('viewer/Cargo.lock')`. The report prints cache usage and `df -h`. Decision rule: if two of five consecutive runs are cold, the job moves to the nightly on `adjutant` | 85% |
| B2 | **V-R3D-13 compares a view-state digest** (XXH3 over the sorted person rows) **as well as the content digest,** in every clause. Assert that the content digest changes at least N times across the sampled ticks. Use `rh-cli tail --digest` as a twin of the fold's counts | 85% |
| B3–B8 | **Accepted:**<ul><li>B3: an unknown `place_id` is "unplaced" with a warning, never a guess, and the namespace is pinned with SPR-018 and SPR-020's owners.</li><li>B4: validate remote paths and check the 107-byte socket limit.</li><li>B5: measure, and consider a release `rh-cli`.</li><li>B6: the household backlog item lists both forms (household roles in the existing casts; a kind-8 record sharing A-61's format bump).</li><li>B7: SPR-034's size is stated honestly, 4–4.5 days, with a checkpoint after T-02.</li><li>B8: a task graph.</li></ul> | 80% |
| C1 | **Accepted.** A probe materializes and prefetches beside a live host run; the ladder must stay byte-identical and the run directory unchanged. `--prefetch` defaults to none while the run is live, and on for a finished run | 85% |
| C2 | **Accepted.** The persisted cache key adds the run's sealed-year `canonical_digest` chain head, or the first and last `state_hash`. A reuse verifies each checkpoint's origin digests. **BCK-002 item** (the Architect files it): `run.toml` records the builder's commit (the drafter's F8) | 85% |
| C3–C9 | **Accepted:**<ul><li>C3: the view-state digest and non-vacuity, as B2.</li><li>C4: the unsealed-tail tick is served from the live window or refused; the genesis cross-check is against the export and the first digest's replay.</li><li>C5: `-tt` or a remote `timeout`, verified once on `adjutant`.</li><li>C6: a cost-table row; fix the cut note.</li><li>C7: the residence divergence, with BCK gating S3 F1 on kind 7, or `from_dump` ignoring residence for non-founders until then.</li><li>C8: the Architect edits SPR-027.</li><li>C9: SPR-035's size is stated honestly, about 4 days, with a checkpoint after T-02.</li></ul> | 80% |
| P1–P4 | **The Architect's, at the close.**<ul><li>PRJ-003 becomes v1.4.1: the 62³ chunks, the bench scene, the CI rule, the path list, the `bevy_ecs` rationale.</li><li>`docs/11` V.1: "households and houses" wait for the engine household record (the backlog item); recorded with D63.</li><li>D63 restated: director time about 45–60 minutes in all; CI figures on the 2-vCPU column; "approve as the Architect revised them"; the sizes and splits put to the director.</li><li>P4: `decisions_pending`'s header collapsed to one short line, so branches only add rows.</li></ul> | 85% |

**Optional for the director, now:** run Bevy's own 3D example from a throwaway checkout on the P51, to retire risk 1 (Maxwell, driver 580) with no repository change.

**The drafter's revision 1:**
1. Merge `origin/main` (99a5b3d or later; PRs #89 and #95 are on it).
2. In `decisions_pending`, take main's file and add D63 last.
3. Apply every row except the Architect's own: P1–P4, the GOV-003 line of A12, C2's backlog item, and C8's SPR-027 edits.
4. Push once.

## 0b. Architect rulings, round 2 (2026-10-18), on `2afe53a` (revision 1 and the Architect's close)

The independent re-check (report `viewer_review/REPORT_R2.md`) found every round-1 MAJ applied; A12, B3 and P2 PARTIAL as stated in the text; C2 and C5 applied with a defect. New: 0 BLK, 3 MAJ, 10 MIN, 4 NIT, three of them found by running code on `main`. **Every finding is accepted unless a row says otherwise.** Owner: D = the drafter's revision 2 (SPR-033, SPR-034, SPR-035); A = the Architect (PRJ-003, D63, BCK-002, SPR-027, REV-VIEWER).

| # | Ruling | Owner | Conf. |
|---|---|---|---|
| V1 | `rh-cli run hamlet --years 30` and `solo --years 30` are refused (an input at year 34 / 53 sits past the horizon). Use `hamlet --years 40` and `solo --years 55`, sample ticks kept in years 1–30, the `INPUT` record in the stream stated. The "different history" control gets its own `--inputs` script with an input inside the covered years, so the chain-head control can fire. Every 30-year run in SPR-034 and SPR-035 is re-checked against `worlds/*/inputs.toml` | D | 90% |
| V2 | **No pty on the data path.** The command that carries `--dump -` runs without `-t`. Cancellation: the remote side is `sh -c 'echo $$ > <pidfile>; exec timeout 900 rh-cli …'`, and a cancel sends `ssh <host> kill $(cat <pidfile>)` over the same multiplexed connection (ControlMaster). The `ssh` stub refuses `-t` on a data command, and a stub case carries a multi-line dump with a stderr report and asserts byte equality. The director's one-time `adjutant` check covers cancellation by the pidfile route | D | 85% |
| V3 | **The two-week rule is the viewer workspace's own** (the red team's text in the old SPR-033 T-01, A12): the viewer's direct pins, checked offline by `viewer/tools/lock_age.py`; the engine's lock is not covered. "Repository practice" is withdrawn (the engine's `twox-hash` entered 12 days old). **No GOV-003 edit.** D63's effect no longer records one | A (PRJ-003, D63), D (SPR-033:89) | 90% |
| V4 | The CI tables count `ci.yml` too: every viewer PR push also runs part core (about 10 min); stage a, which edits `bin/`, runs all three parts (about 25 min a push). PR runs are cold unless a schedule or dispatch on `main` saved the key; the "warm" figure is measured at the first daily run after the merge (T-06's acceptance moves there). The restore-only daily shortcut is rare while `main` moves daily. The sums are recomputed. D63 item 3 restates the total | D, A (D63) | 85% |
| V4b | **The fallback when caches churn** (B1's two-in-five rule) is restated honestly: `adjutant.yml` is dispatch-only and runs only when the director starts the VM (about $0.83 an hour, RUN-002), so it is not a nightly. **The fallback becomes:** the Bevy half of the job drops to a weekly hosted schedule plus dispatch, and the Bevy-free half (`rh-view-core`, the terrain, the lint) keeps running on PRs. No director action needed | D, A (PRJ-003, D63) | 70% |
| V5 | The `rh-cli` build and the run helpers move from SPR-034 T-05 to T-01 (+0.1 day), so the Tester's checkpoint can run T-01's probes in CI. The same check in SPR-035 | D | 85% |
| V6 | `rh-view-core` takes exactly one named allow, `std::process::Command` in `source.rs`; SPR-035's `materialize` spawns through `source.rs`. Every other ban in the root `clippy.toml` holds in `rh-view-core` (`Instant`, `env::args` live in `rh-view`) | D | 80% |
| V7 | `rh-protocol` is a path dependency of `rh-view-core` from T-01b (it is on the allow-list), with the fake host as a dev-dependency; T-05's client lives there | D | 85% |
| V8 | Cite SPR-018 X6: the place is a `SettlementId(u32)` (a `BIRTH`'s is the mother's). SPR-034's table is "`PlaceId` = `SettlementId`", tested against the golden's seat numbering; an unknown id stays unplaced with a warning. F8's request to SPR-018's and SPR-020's owners is withdrawn | D | 85% |
| V9 | D63 lists the engine-lane asks the plans add (BCK-002 A-70, SPR-027's `--dump -`, A-61's recommendation), the director's one-time `adjutant` cancellation check (about 5 minutes with the VM start) inside the time total, the allowance denominator (GOV-008's about 2,000 minutes a month on Free; the real figure is D61 item 8's question), that the CI rule narrows ADR-0036:78 (the director's approval covers it), and "applied, with three items recorded on approval" instead of "every finding applied". REV-VIEWER is filed at the close | A | 85% |
| V10 | The cache key always holds the sealed-digest chain head and the first and last `state_hash`; it adds the builder's commit when `run.toml` has it (A-70). SPR-035 F1 and BCK-002 A-70 say so | D, A | 90% |
| V11 | The live-host probe runs at `--pace 1y` (30 wall-seconds over 30 years; `05.1` §4.4's bands are sim-time per wall-second), with V1's horizon | D | 85% |
| V12 | Add a back-off: after a step-down that follows a step-up on the same notch, the step-up wait for that notch doubles (cap 8×); a synthetic series with a notch costing more than 1.67× the next must not oscillate | D | 80% |
| V13 | Risk 7's mitigation and C7 become: `from_dump` ignores `person.residence` altogether; founders are placed from the export, everyone else from their `BIRTH` place. A-61's bullet is worded as the viewer lane's recommendation, not an engine constraint (D62) | D, A | 85% |
| V14–V17 | Accepted as the report fixes them: the 033b sum (2.2–2.6) and T-01b's heading; A-70 cites SPR-035 F1; REV-T1A's "filed at the SPR-014 audit" is superseded by this filing (A-69 is not filed twice); SPR-027's `updated` date; water counts as a surface in the reduction (a coarse voxel takes the top-most non-air layer, transparent if that layer is water); the resident figures are render-only, outside both digests and the content key; rust-cache's own lock hash instead of an explicit `hashFiles` key; the daily cron off `17 5 * * *` (it collides with `advisories.yml`); one job, two toolchains (1.97.0 for the root `rh-cli`, 1.97.1 for the viewer), stated; PRJ-003's attribution of the two-in-five rule to ADR-0036 corrected | D, A | 85% |

## 0c. Architect's verification of revision 2 and the close rulings (2026-10-18)

Revision 2 (`0e76b69`) merged with the Architect's round-2 edits (`d2b0178`) on the close branch; `gen_manifest --check` builds cleanly (223 documents).

| Check | Result |
|---|---|
| V1 | SPR-034:74 states the refusal; its runs are `hamlet --years 40`, `solo --years 55`, `household --years 30` (household's input sits at year 4), sample ticks in years 1–30; SPR-035's runs are 40 years, and the different-history control has its own `--inputs` fixture with an input in sim-year 12 |
| V2 | SPR-035:67, :177: `-T` and `sh -c '… exec timeout 900 <rh-cli> materialize …'` with a pidfile; the stub refuses `-t` and `-tt` on a data command (:212); the director's one-time check uses `[r]h-cli materialize` |
| V3 | SPR-033:89 and PRJ-003 §3 and §8 item 13: the viewer workspace's own rule, with the `twox-hash` evidence; no GOV-003 edit; D63 records none |
| V4, V4b | The cost sections count `ci.yml`; PR runs cold; warm measured on `main` after the merge (F18); the churn fallback is a weekly hosted schedule for the Bevy half; `adjutant` is dispatch-only |
| V8, V10–V13 | `PlaceId = SettlementId` (SPR-034:68); the key always holds the chain head and state hashes (SPR-035:90, A-70); `--pace 1y` (SPR-035:205); `mutant-lod-no-backoff` (SPR-033:396, :452); `from_dump` never reads `person.residence` |
| Versions | SPR-033 v1.5.0, SPR-034 v1.2.0, SPR-035 v1.2.0, PRJ-003 v1.4.2, SPR-027 0.2.2, BCK-002 1.22.1 |

**The close rulings on revision 2's own findings:**

| # | Ruling | Conf. |
|---|---|---|
| F1 (SPR-033 F18) | Accepted: the warm figure comes from the next full run on `main` after the one that saves | 85% |
| F2, F3 | Accepted: rust-cache keeps crates.io dependencies only, and its key includes every installed `rustc`; both toolchains from stage b | 75% |
| F4 (SPR-033 F17) | **The full scheduled run is twice a week (Monday and Thursday, `43 4 * * 1,4`) plus dispatch, not daily,** until the director answers D61 item 8's allowance question: about 95–167 minutes a month after SPR-035 instead of 315–555. Daily is a one-field change. Applied in SPR-033, SPR-034, SPR-035, PRJ-003 and D63 | 75% |
| F5 (SPR-035 F9) | Accepted: a founder's genesis settlement is read once from the `residence_interval` row with `from_tick` 0 | 70% |
| F6, F7 | Accepted: timing and the environment in `rh-view`, never in `rh-view-core`; the one-time check's `[r]h-cli materialize` pattern | 85% |
| F8 | D63 restated with the final figures (the Architect) | — |

## 1. The independent review, round 1 (`c51f07c`)

The review was independent, on a different model from the drafter's. The Architect recorded this report from the reviewer's returned text. Its scratch work (crate dates, a dependency probe, a relief script) is in `viewer_review/work/`.

### BLUF

| Plan | Verdict | BLK | MAJ | MIN | NIT |
|---|---|---|---|---|---|
| SPR-033 v1.3.0 | NEEDS REVISION | 0 | 6 | 10 | 3 |
| SPR-034 v1.0.0 | AFTER SMALL FIXES | 0 | 2 | 5 | 1 |
| SPR-035 v1.0.0 | AFTER SMALL FIXES | 0 | 2 | 6 | 1 |
| PRJ-003 v1.4.0, the docs, D63 | AFTER SMALL FIXES | 0 | 0 | 3 | 1 |
| **Total** | | **0** | **10** | **24** | **6** |

- **What decides SPR-033:** split it; rewrite T-06 on PR #89; fix the level-of-detail controller, the `bevy_*` lint rule, the missing written terrain definition, and the P51-in-the-PR acceptance.
- **The Bevy pin:** wait for `=0.20.0` with a viewer-only toolchain at 1.97.1 (75%). With SPR-033 split, the wait costs no schedule.
- **CI cost:** the "200–400 minutes a month" claim is optimistic by 2–5×. A `main` push trigger fires about 10 times a day, and cache churn is likely.
- **Merge:** the branch conflicts with `main` and with PR #95 only in `docs/decisions_pending.md`.

### 1. Fact check

**Confirmed:**
- bevy 0.20.0, published 2026-10-08T22:31Z with `rust-version = 1.97.1`; 0.19.1 declares 1.95.0;
- `wesl` and `wgsl-parse` also need 1.97.1;
- Rust 1.97.1 was released 2026-07-16;
- the T-01 feature names, with transitive extras: `3d_bevy_render` pulls `bevy_gltf` and `gltf_animation`, and `3d_api` pulls the LUTs, `ktx2` and `zstd`;
- 353 crates;
- `binary-greedy-meshing` 0.5.2: MIT, no dependencies, `no_std`, no `unsafe`, 9 months old. Its `fast_mesh` needs both opaque and transparent masks, and air is id 0;
- 62³ chunks: `CS ≤ 62` because `CS_P = CS + 2` must fit a u64;
- the 64×64 diorama is 22.4 m;
- hamlet is 7.68 km across with 130.8 m of relief, and twin_vales 3.84 km with 611.9 m;
- channel A has no households, houses or titles; the `MARRIAGE` payload is empty; kind 7 is deferred (`EntityKind::Household` exists, unused);
- `engine_build` names no commit (`world.rs:256`);
- `adjutant`'s idle shutdown counts only a runner worker, `who` and the marker, so `ssh -t` keeps it up and `ssh -N` does not;
- the P51 spec.

**Unverifiable** (Bevy not built): the 178 s build, the 2.5 GB target, and the Wayland build failure.

**Wrong as cited:**
- PRJ-003 §3 has no two-week rule.
- "The repository already pins `bevy_ecs`" is wrong: no bevy package is in the root lock.

### 2. The Bevy pin

The Bevy-free work (T-01's core half, T-02, T-03, T-06's lint) is about 1.9 days. It starts at the SPR-026 merge and ends around then plus 2 days. 0.20.0 becomes two weeks old on 10-22. So waiting costs 0–1 day of idle time, which T-06 fills. Pinning 0.19.1 buys at most an earlier window, at the cost of about 0.5 day of migration later. Risk 1 (Maxwell, driver 580) can be retired now: the director runs Bevy's own 3D example on the P51 from a throwaway checkout.

### 3. The drafter's findings

- **SPR-033 F1–F14:** mostly agreed.
  - F1 (75%).
  - F2 (95% on the arithmetic, 80% on DEPEND).
  - F3: the reduction rule loses thin features (A7).
  - F4: put the constants in the export file.
  - F6: right scope, but the size is wrong (A1).
  - F7: agreed for PRs, not for the `main` trigger (B1).
  - F8: rewrite on PR #89 (A4).
  - F9: use `tmux` inside `ssh -t`.
  - F10: expect many use-site allows.
  - F11: drop the glTF features.
  - F12: make `wayland` an opt-in feature.
  - F13: run cargo with cwd `viewer/`.
  - F14: agreed.
- **SPR-034:** F1–F5 agreed (F1 with B6). F6 agreed in direction but not in the trigger. F7: `wire_v1.md` §6.3 already names the path.
- **SPR-035:** F1–F6 agreed (F3's key is wrong: C2). F7 disagreed: prefetch should not be on by default beside a live run (C1).

### 4. Findings

### SPR-033

| ID | Grade | Problem | Fix |
|---|---|---|---|
| A1 | MAJ | Size: 3.2 days stated, but T-04 alone is 1.5–2 days. Realistic total 4.5–5.5 | Split: 033a, Bevy-free (about 2.5 days), and 033b, Bevy (about 2.2–2.7) |
| A2 | MAJ | The level-of-detail fallback cannot work under vsync: p95 sits on 16.7 ms, and step-up can never fire | Measure render work; thresholds relative to the refresh interval; a vsync-capped synthetic test |
| A3 | MAJ | Isolation rule (b) bans `bevy_*` in the root lock, including `bevy_ecs`, the engine's planned ECS. A viewer rule would block an engine PR | List the render and window stack explicitly; a passing `bevy_ecs` fixture |
| A4 | MAJ | T-06 collides with PR #89 (`ci_scope.py`, `check_all.sh`, `ci.yml` and the tests) | Name #89; rewrite on its scope framework and three parts; a rename test |
| A5 | MAJ | `terrain_v0` has no written definition and no independent reference: 30 m ÷ 0.35 m = 600/7, a non-integer ratio, so rounding, sampling and route draping need pinning | `specs/viewer/terrain_v0.md` plus a Python reference differential; the same for `grammar_v0_e1` |
| A6 | MAJ | A P51 report as PR acceptance makes the director's availability gate the merge, against D62 | Merge on headless gates; post the report afterwards; one 10-minute render check at 033b's merge |
| A7 | MIN | The majority reduction loses routes and plazas at L1; the heightfield beyond 360 m shows none; seams unstated | A surface-preserving reduction; overlay lines; state the seams |
| A8 | MIN | `Catchup` cannot skip history after `stream.behind` | Restart from `next_seq`, or ask SPR-026 for a skip input |
| A9 | MIN | Debug test time | `opt-level = 2` |
| A10 | MIN | The FPS verdict is razor-edged on a 60 Hz panel, under XWayland, with Optimus | A report header, a tolerance, an opt-in `wayland` |
| A11 | MIN | No forward allow-list | Rule (e) |
| A12 | MIN | The two-week rule's scope: 88 packages in a fresh resolve are under 14 days old, and the engine's own pins break it too | Rule that it covers direct pins only; a lock-age table |
| A13 | MIN | cwd matters for the toolchain, the target and the env | `cd viewer` |
| A14 | MIN | The CI cost headline | See B1 and §5 |
| A15 | MIN | The cut order and the feature set | Effects first; drop glTF and ktx2 |
| A16–A18, A20 | NIT/MIN | Template sections; the stale `bevy_ecs` rationale; the parser, F10's wording, `sha2`; the terrain golden check on the P51 | Edits |

### SPR-034

| ID | Grade | Problem | Fix |
|---|---|---|---|
| B1 | MAJ | A `main` push trigger on `crates/**` fires about 10 times a day: 1,200–1,800 minutes a month, not 100–150 | A daily scheduled run plus dispatch; or `cancel-in-progress` |
| B2 | MAJ | The V-R3D-13 content digest is a lossy projection: residents enter only as `ceil(residents/5)` | Compare a view-state digest too; assert the digest changes; `tail --digest` as a twin |
| B3 | MIN | `BIRTH.place_id` is an opaque `PlaceId` | Pin the namespace; an unknown value is unplaced |
| B4 | MIN | Remote path quoting; the 107-byte socket limit | Validate; test |
| B5 | MIN | The debug-engine slowdown is a guess, and the hosted runner's disk is small | Measure; consider a release `rh-cli`; `df -h` |
| B6 | MIN | The household item has two forms | List both; prefer sharing A-61's bump |
| B7 | MIN | T-03 at 0.9 day is a stretch; no grammar spec; realistic 4–4.5 days | A spec file; an honest size; a checkpoint |
| B8 | NIT | No task graph | Add one |

### SPR-035

| ID | Grade | Problem | Fix |
|---|---|---|---|
| C1 | MAJ | No test of materialize or prefetch beside a live run, and prefetch is on by default against an authoritative run on `adjutant` | A live-host probe; prefetch off while the run is live |
| C2 | MAJ | The persisted cache key cannot tell engine builds apart (`engine_build` has no commit) | Add the sealed digest chain or state hashes to the key; verify on reuse |
| C3 | MIN | The five-path equality compares the content digest only | View-state digests and non-vacuity |
| C4 | MIN | A tick in the unsealed tail; tick 0 has no genesis keyframe or published hash | Serve from the window or refuse; change the genesis cross-check |
| C5 | MIN | Cancelling a remote command over `ssh` without a pty leaves `rh-cli` running | `-tt` or `timeout` |
| C6 | MIN | `rh-golden` is always linked; the cost table | Fix |
| C7 | MIN | S3 F1 residence divergence before kind 7 | Gate S3 F1 on kind 7, or ignore residence |
| C8 | NIT | `Dump::digest`'s compared field; `--dump -` is not in SPR-027 T-02; SPR-027's text is stale | The Architect edits SPR-027 |
| C9 | MIN | Size: stated 3.0, realistic about 4 | A checkpoint |

### PRJ-003, `docs/11`, D63

| ID | Grade | Problem | Fix |
|---|---|---|---|
| P1 | MIN | PRJ-003 is stale: chunks, scene, CI rule, path list | The Architect edits it, v1.4.1 |
| P2 | MIN | `docs/11` V.1 promises "households and houses" | Update the row, or file a follow-up |
| P3 | MIN | D63's director time (really about 45–60 minutes), its CI figures, and "as written" | Fix |
| P4 | NIT | `decisions_pending`'s header is a repeated conflict point | Collapse it to one line |

### 5. CI cost

- Hosted runners for a private repo on the Free plan are 2 vCPU, 7 GB and 14 GB of disk. The honest cold times are 11–16 minutes for SPR-033 and 15–20 after SPR-034.
- `ci.yml`'s rust-cache has no `save-if`. With 459 runs in about three days, the 10 GB cache limit likely churns, so cold runs are the norm.
- **Fixes:**
  - a daily scheduled run on `main`;
  - `save-if` on that schedule;
  - viewer caches keyed on the viewer lock;
  - print cache usage and `df -h`;
  - a rule to move the job to `adjutant` if two of five runs are cold.

### 6. Other checks

- **The V-R3D-13 paths:** the naive-mesher differential and the export-against-golden check are genuine independent oracles; the voxelisation has none (A5). In SPR-034, clause 5 is a real differential, and SPR-035's dump comparison is the strongest oracle.
- **The isolation gate:** sound apart from A3, A11 and A13.
- **D62 conformance:** consistent apart from A6.
- **Tester scopes:** the right shape. Add an independent terrain voxeliser (A5), the live-host probe (C1) and tampered-cache cases (C2).

### 7. Merge test

`decisions_pending.md` is the only conflict against `main` and against PR #95. Land #95 first, merge main, take its file and add D63 last, with a one-line header (P4).

## 2. The independent re-check, round 2 (`2afe53a`)

The re-check ran on a model different from the drafter's. Nothing in the repository was changed; the re-checker only fetched. The Architect recorded this report from the re-checker's returned text. Its scratch work is in `viewer_review/work2/`.

### BLUF

Round 1's ten MAJ findings are all applied. Three rulings are PARTIAL (A12, B3, P2), and in each the open part is stated in the text. Two others, C2 and C5, are applied but introduced a defect.

The revision introduced 3 MAJ, 10 MIN and 4 NIT of its own, no BLK. Three were found by running code on `main`:
- `rh-cli run hamlet --years 30` is refused, so every 30-year hamlet run in SPR-034 and SPR-035 cannot be run as written.
- `ssh -tt` corrupts the `--dump -` output that SPR-035 reads.
- The two-week rule is not "repository practice", and D63's default would write it as a repository-wide rule.

| Plan | Verdict | BLK | MAJ | MIN | NIT |
|---|---|---|---|---|---|
| SPR-033 v1.4.0 (stages a and b) | AFTER SMALL FIXES | 0 | 0 | 4 | 2 |
| SPR-034 v1.1.0 | AFTER SMALL FIXES | 0 | 1 (V1, shared with SPR-035) | 2 | 1 |
| SPR-035 v1.1.0 | AFTER SMALL FIXES | 0 | 1 (V2; V1 applies too) | 3 | 0 |
| PRJ-003 v1.4.1, D63, BCK-002 1.22.0, SPR-027 0.2.2 | AFTER SMALL FIXES | 0 | 1 (V3) | 1 | 1 |
| **Total, each finding counted once** | | **0** | **3** | **10** | **4** |

- **Merge test:** the branch merges cleanly into `origin/main` (`3d26090`, `511b7cf` and `17ca24f` all tried).
- **Manifest:** `gen_manifest.py --check` passes on the branch (223 documents) and on the merged trees (224, 226).

### 1. The ruling table

| Row | Verdict | Evidence |
|---|---|---|
| Bevy | APPLIED | SPR-033:24, 89, 373, 385–386; PRJ-003:82; D63 item 2. crates.io: bevy 0.20.0 published 2026-10-08T22:31:14Z, rust-version 1.97.1 |
| A1 | APPLIED | SPR-033:15–28, stage headings 193 and 369, task graph 170–189; PRJ-003:101; D63 item 1 |
| A2 | APPLIED | SPR-033:156–168, 435–440; residual V12 |
| A3 | APPLIED | SPR-033:303, 310 (`lock_has_bevy_ecs` must PASS), 312; PRJ-003:72. The root lock has no bevy, wgpu, winit or naga |
| A4 | APPLIED | SPR-033:313–327, 340–354, 61, 125; `f00cf17` is PR #89's merge |
| A5 | APPLIED | SPR-033:91, 256–272; SPR-034:72, 193–223. cell = floor(7x/600), h = floor(e·100/2240) checked |
| A6 | APPLIED | SPR-033:34, 540, 620, 661; SPR-034:34, 357; SPR-035:291, 379; D63 item 6 |
| A7 | APPLIED | SPR-033:132–154, 94, 250, 408; PRJ-003:85–86 |
| A8 | APPLIED | SPR-033:100, 453, 470; matches `wire_v1.md` §6.3 |
| A9 | APPLIED | SPR-033:200 |
| A10 | APPLIED | SPR-033:98, 385, 562–579 |
| A11 | APPLIED | SPR-033:99, 306; PRJ-003:72; SPR-034:71, 137, 234 |
| A12 | PARTIAL | T-01 and PRJ-003:82 done; the GOV-003 line deferred to D63's approval; see V3 |
| A13 | APPLIED | SPR-033:124, 202–204, 220, 328, 496 |
| A14 | APPLIED | through B1; see V4 |
| A15 | APPLIED | SPR-033:97, 381, 390, 480–482. The T-01b feature set with `=0.20.0` locks 457 packages; none of `bevy_gltf`, `gltf_animation`, `ktx2`, `zstd_rust`, `tonemapping_luts`, `smaa_luts` in `cargo tree -e features`; all 15 feature names exist |
| A16 | APPLIED | Blockers and Completion in all three plans |
| A17 | APPLIED | PRJ-003:82; SPR-033:99, 303 match `docs/07:28` |
| A18 | APPLIED | SPR-033:376, 211 (`sha2 =0.11.0`, 2026-03-25; 0.11.1 is 2026-10-12), 114, 676 |
| A20 | APPLIED | SPR-033:498, 534, 546 |
| B1 | APPLIED | SPR-033:101, 340–354, 596; SPR-034:273–277; SPR-035:298; PRJ-003:123; D63 item 3; figures: V4, V9 |
| B2 | APPLIED | SPR-034:66, 177–178, 247–256; SPR-035:279, 286; PRJ-003:118. On hamlet 40 years from `rh-cli tail`, over 500 draws of the 10 extra ticks the minimum was 8 view-state and 5 content digests: the floors of 8 and 3 hold, with no margin on the first |
| B3 | PARTIAL | the viewer side applied (SPR-034:68, 172, 350); the pin with SPR-018's and SPR-020's owners is a request with no owner; SPR-018:69 already says `SettlementId` (V8) |
| B4 | APPLIED | SPR-034:126–129, 148; the 107-byte limit is `wire_v1.md:24` |
| B5 | APPLIED | Measured: debug `rh-cli` builds cold in 89 s on 4 cores (about 2.7 min on 2 vCPU); release 358 s; a debug 200-year hamlet run 6.8 s against 2.2 s; the debug ladder equals the baseline |
| B6 | APPLIED | SPR-034:67, 372; A-71; kind 8 is free in `channel_a_v0.sql` |
| B7 | APPLIED | 0.9 + 0.7 + 1.6–1.9 + 0.6 + 0.3 = 4.1–4.4 |
| B8 | APPLIED | task graphs in all three plans and PRJ-003 §6 |
| C1 | APPLIED | SPR-035:65, 128–131, 197–202; pace: V11 |
| C2 | APPLIED, with a contradiction | SPR-035:85–97, 224–226; A-70 filed; the F1 sentence contradicts the body (V10) |
| C3 | APPLIED | SPR-035:279, 286 |
| C4 | APPLIED | SPR-035:66, 178–196 |
| C5 | APPLIED, design defective | SPR-035:67, 175, 188, 205 (V2) |
| C6 | APPLIED | SPR-035:63, 306, 326 |
| C7 | APPLIED | SPR-035:68, 185, 290, 393; A-61's last bullet; weaker than stated (V13) |
| C8 | APPLIED | SPR-027 T-02 `--dump [file or -]` with a byte-equality test; SPR-035:184 compares `Dump::digest().hash` |
| C9 | APPLIED | SPR-035:30, 235, 303; the sum is 3.8 |
| P1 | APPLIED | PRJ-003 §2–§6, §8 item 12, except V3 and V17 |
| P2 | APPLIED as ruled | D63 item 5 and the default; A-71; `docs/11:431` unchanged until the director answers |
| P3 | APPLIED, incomplete | the four sub-items add to 45–55 minutes; V3, V9 |
| P4 | APPLIED | `decisions_pending.md:3` one line; D63 last |

### 2. New findings

### MAJ
- **V1 (SPR-034, SPR-035): 30-year hamlet and solo runs are refused by `rh-cli` on `main`.** `rh-cli run hamlet --years 30` stops: "the input script has 1 input(s) at or past the 30-year horizon, the first at tick 17772847 (sim-year 34) … nothing was written"; solo the same (year 53); household is fine (year 4). The cause is `worlds/hamlet/inputs.toml`. Used at SPR-034:74, 180, 247, 268 and SPR-035:191, 198, 225, 269. Second effect: "run hamlet twice, one with a journaled input" (SPR-035:225) cannot work: every hamlet run of 35 years or more already carries the year-34 input, so for a cache entry covering years 0–30 both runs share the chain head and the control cannot fire. Fix: `--years 40` for hamlet and 55 for solo with sample ticks in years 1–30, or `--inputs`; the different-history control gets its own inputs script with an input inside the covered years.
- **V2 (SPR-035): `-tt` and `--dump -` cannot both hold.** A pty has one output stream, applies ONLCR and merges stderr: stdout `line1\nline2\n` plus stderr `report\n` arrived as `b'line1\r\nline2\r\nreport\r\n'`. The `ssh` stub (:205) runs without a pty and the director's one-time check (:348–352) tests only cancellation, so nothing catches it. Fix: no `-tt` on the data command; cancel another way (a pidfile and a second multiplexed `ssh host kill`, or `stty -opost -echo` with stderr to a file); a stub case with a pty and a multi-line dump that must arrive byte-equal.
- **V3 (PRJ-003:82, SPR-033:89, D63's default): the two-week rule is not repository practice, and D63 would write it repository-wide.** SPR-026 states no age rule; the engine's `twox-hash` 2.1.5 (published 2026-10-04T14:59Z) entered the root lock on 2026-10-16 in `9c051fe`, 12 days later. D63's default would write a repository-wide rule into GOV-003 §5 ("Forbidden everywhere"), binding engine PRs, which D62 puts first; A12, T-01 and PRJ-003 scope it to the viewer's direct pins. Fix: say it is the viewer workspace's own rule; scope any GOV-003 line to the viewer's direct pins checked by `lock_age.py`, or drop the GOV-003 edit.

### MIN
- **V4 (CI cost model):** `ci.yml`'s part core runs on every viewer push (PR #89's first run: core 10:00), and stage a edits `bin/`, so it runs all three parts (about 25 min a push); `save-if` on the schedule only means no PR run warms the cache, so every 033b PR push is cold; T-06's "warm time in the PR" cannot be met; the restore-only daily shortcut rarely fires while `main` moves daily (462 PR-tier runs in 7 days); "about 45 PR runs" vs "about 15 pushes per stage"; SPR-034's sums are 16–25 cold, 6–12 warm, SPR-035's 18–25 and 8–15. Fix: add `ci.yml`'s rows, state PR runs are cold, measure warm at the first daily run, correct the sums.
- **V5 (task order):** SPR-034 T-01's acceptance runs on the job's `rh-cli`, but its build and the run helpers are T-05's; move them to T-01 (about 0.1 day). The same in SPR-035.
- **V6 (allow policy):** SPR-033 says `rh-view-core` takes no `allow`; SPR-034 puts `std::process::Command` in `rh-view-core/src/source.rs` with one; SPR-035's `materialize` spawns `rh-cli` from `rh-view-core`; the root `clippy.toml` also bans `Instant` and `env::args`. Fix: exactly the named allow in `source.rs`, `materialize` through it, or spawning in `rh-view`.
- **V7 (`rh-protocol` in the wrong crate):** T-01b gives it to `rh-view` only; T-05's client is in `rh-view-core::live` and stage a has no path dependency, so it cannot compile. Add it (and a dev-dependency for the fake host) to `rh-view-core` in T-01b.
- **V8 (the PlaceId pin is stale and has no owner):** SPR-018:69 (X6) already says the place is a `SettlementId` (a `BIRTH`'s is the mother's place), SPR-018:76 `SettlementId(u32)`; cite X6, make the table `PlaceId = SettlementId`, test against the golden's seat numbering, drop or file F8.
- **V9 (D63 incomplete):** it omits the engine-lane asks (A-70, A-61's new sentence, SPR-027's `--dump -`); SPR-035's Completion requires the director's one-time `adjutant` cancellation check (not in the minutes); `adjutant.yml` is `workflow_dispatch` only and waits for the director to start the VM (about $0.83 an hour, RUN-002:27), so the fallback is not a nightly; the plans narrow ADR-0036:78 ("when `viewer/` or a crate it reads changes") and D63 should say so; no allowance denominator (GOV-008: about 2,000 minutes a month on Free; D61 item 8: unknown); "every finding applied" is overstated (A12, P2, B3 pending); REV-VIEWER is cited but not yet filed.
- **V10 (the Architect's F1 sentence contradicts the key):** F1 and A-70 say "the commit when present, the chain head otherwise"; the body always keys on the chain head and the first and last `state_hash`, and `mutant-key-tuple-only` tests that a commit-only key cannot tell two histories of one build apart. Fix: "adds the commit when present; the chain head and state hashes are always in the key".
- **V11 (the live-host probe's pace):** `--pace 10y` over 30 years is 3 wall-seconds; ten materializations cannot overlap it. Use `--pace 1y`, with V1's horizon.
- **V12 (no anti-oscillation):** a step-up at p95(W) < 0.6R overshoots when a notch costs more than 1.67× the next, then steps down after two windows and up again after ten: a limit cycle of about 12 windows. Add a back-off and a synthetic test.
- **V13 (C7 weaker than claimed):** "founders only" agrees only while no founder moves; Risk 7's mitigation misses it; A-61's new bullet turns a viewer preference into an engine constraint against D62. Fix: `from_dump` ignores `person.residence` (founders from the export, others from the `BIRTH` place); word A-61's bullet as a recommendation.

### NIT
- **V14:** 033b sums to 2.2–2.6, not 2.7; T-01b's heading 0.3 day with its own job build another 0.3; T-04's 1.2–1.6 below the review's 1.5–2.
- **V15:** A-70 cites "SPR-035 F8" for the builder-commit finding, which is F1; A-68 and A-69 are other lanes' rows, and REV-T1A:67 says A-69 is filed at the SPR-014 audit, so a second filing would duplicate; SPR-027's `updated` not bumped for 0.2.2; the PY-PKG edit is not in the commit subject.
- **V16:** the reduction does not say whether water counts as solid; SPR-034's resident figures are not placed outside both digests and the content key.
- **V17:** an explicit `hashFiles` in the rust-cache `key:` duplicates its own lock hash and defeats its prefix fallback (about 80% sure); the daily 05:17 UTC collides with `advisories.yml:10`; after stage b one job needs two toolchains (1.97.0 for the root `rh-cli`, 1.97.1 for the viewer); PRJ-003:123 attributes the two-in-five rule to ADR-0036, which (:90) says only "move its job to the nightly".

### 3. Other facts checked and correct
hamlet has 4 regions, 4 seats and 5 routes; founders at genesis 50, 4, 2 and 4 by settlement; 46 of 60 exit within 38 years. `binary-greedy-meshing` 0.5.2 MIT, 2026-01-07. `rh-cli build-info` prints `mutants: none`. `RESERVED_SYSTEM_IDS`, `t{timeline:010}`, `--to-seq`, the `Dump` API and the `pace.state` tokens exist or match. `STEP_DIGEST` births and deaths are per interval. The 62³/64³ arithmetic, the 6+6+6+3+11 = 32 vertex bits and the 14 content forms are right.
