---
id: REV-MINDS
title: "Independent review of the minds sprint plans (SPR-023, SPR-024a and SPR-024b, SPR-025) and D59"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [SPR-023, SPR-024, SPR-025, PRJ-002, SPR-021, SPR-022, SPR-038, SPR-015, SPR-019, SPR-030, BCK-002]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing commit (2026-10-18). The plans go to the director as D59.**
>
> | Plan | Revision | Size | Class |
> |---|---|---|---|
> | SPR-023: `rh-minds`, traits, opinions, promote and demote | 0.3.0 (`0e3337a`) | 4.75–5.25 agent-days | C2 |
> | SPR-024a: IAUS decisions on the probe type | 0.3.0 | part of 6.25–7 agent-days | C2 |
> | SPR-024b: the first real M7 + T1 integration and the one `BUILT` move | 0.3.0 | the rest | C3 (header only; X6) |
> | SPR-025: stress and breaks, V-SAGA-16b | 0.3.0 | 5.25–6 agent-days | C2 |
>
> - **Round 1** (`3beca61`): 0 BLK, 9 MAJ, 14 MIN, 5 NIT (§1). Of the drafter's 23 findings, 20 are real and 3 partly real.
>   - The majors: T1's missing `person.id` read (M1); the `exits()` design (M2); the spec PR's shape (M3); no owner of the real integration (N1); SPR-024's size (N2); V-SAGA-16b vacuous at `twin_vales`' values (P1); `break_timer` as a TimerId (P2); "whichever lands first" (Q1); D59 hiding an ADR change (Q2).
>   - Rulings: §0, with the cross-plan rulings X1–X13 that bind wave 7, the minds plans, SPR-038 and SPR-015 (§0a). Revision 1 is `f791a38`.
> - **Round 2** (`f791a38`, a joint re-check with wave 7): AFTER SMALL FIXES, 0 BLK, 1 MAJ (R4, the X5 probe "at least 50 minds" is false for a correct build), 5 MIN, 1 NIT (§2). Rulings: §0b. Revision 2 is `0e3337a`, merged with `main` `7031f6a` as `653c5e9`.
> - **Architect's verification of revision 2** (§0c): every minds row of §0b is present.
>   - R4: the probe reads `named_at_start full` from `fixture_worlds_v0.txt` (33, 35 and 26 on `twin_vales` seeds 1–3 on `main`; 24, 29 and 25 after A-62, PR #94), at least 20 there; `manor` has no floor.
>   - R5: 024b waits for SPR-022a and SPR-019b in D59, the graph and the lane.
>   - R7: T1-I/S is a new `next/T1` 0.5.0 after SPR-023 T-01, on 024a's critical path.
>   - R8: T1's defaults go through `registry_delta.toml`; `manor.yaml` is not edited.
>   - R9, R14: SPR-023 changes no `rh-core`; it consumes SPR-038b's ascending `exited()`, `Deferred`, the generic `Ended`, `cancel_because` and the cancel counter.
>   - R10: `events_of(MARRIAGE)`. R11: the M7-owned house → title-holders index in `rh-contracts::m7`.
>   - R13: V-SAGA-16b nightly-only, 30 breaks per variant or NOT EXERCISED, the planted FAIL asserted; the PR tier runs invariants only.
>   - R15: PRJ-002's graph has no dangling node; R17: `Demote` cancels four kinds and never `DEATH_DUE` or `MARRY_DUE`.
>   - One gap, ruled at close (§0c): the `manor` figure has no golden yet.

## 0. Architect rulings, round 1 (2026-10-18), on `3beca61`

The independent review found **0 BLK, 9 MAJ, 14 MIN and 5 NIT**. Of the drafter's 23 findings, 20 are real and 3 partly real. The cross-plan rulings X1–X12 (`scratchpad/p1a_shared_rulings.md`) bind this revision. **Every finding is accepted unless a row says otherwise.**

| # | Ruling | Conf. |
|---|---|---|
| M1 | Accepted. `[[reads]] person.id` (M2, same instant, identity) goes in the T1-A spec PR, plus a registration test | 95% |
| M2, Q1 | **Per X1 and X2.** SPR-023 consumes SPR-038b's `exited()`; `exits()`/`entries()` are dropped; SPR-023 T-02 becomes a consumer test | 70% |
| M3 | **Per X7:** two spec PRs, T1-A (0.4.0, before SPR-023) and T1-I/S (a `next/` draft after T-00a, promoted by 024a). Both go in PRJ-002, sized, with their own `check_all` lines. The Architect commissions T1-A now | 75% |
| M4 | **Per X4** | 85% |
| M5 | Accepted. `DropReason::Deferred` and a cancel counter, in SPR-023 T-02, with a `05.1` sentence | 85% |
| M6, M7, M9, M10, M11 | Accepted as the review fixes them | 85% |
| M8, N2, P5 | **Per X8.** SPR-023 one sprint, 4.75–5.25 days; 024a and 024b, 6.25–7; SPR-025 one sprint, 5.25–6 | 65% |
| N1 | **Per X5.** 024b owns the first real integration, after 022a, with the stated probes | 80% |
| N3–N7 | Accepted. N5: revalidate at fire, and write `MARRIED_TO` after M2 applies | 85% |
| P1 | **Accepted in full.**<ul><li>V-SAGA-16b's gate runs on a stress-capable fixture (parameters inside the Registry box where a cascade is possible), stated in T1-I/S.</li><li>`twin_vales` is reported only.</li><li>A massacre variant (4 deaths at closeness 1 in one instant) is added.</li><li>The gate fails below 30 breaks over the ensemble ("NOT EXERCISED").</li><li>The band leaves the gate.</li><li>A T-04 probe asserts that breaks occurred.</li></ul> | 80% |
| P2 | Accepted. `mind.break_timer` becomes a flag; the timers are found through `own_timers`; INV-T1-02 is checked by an observer with agenda access; tests run through a re-sample and a keyframe | 90% |
| P3 | **Per X1(c).** T1 declares `[[observes]]` (in T1-I/S) | 75% |
| P4, P6, P7 | Accepted | 85% |
| Q2 | **Per X11:** D59 item 4 is an ADR-0002 §2 addendum, a director item that names the doc lines. Items 5 and 6 are restated against X1, X2 and D58 | 80% |
| Q3 | **Per X10** | 75% |
| Finding 3 | Pin `rh-minds` as (A, system) in `lint_arch.py` with a pinned-names clause | 70% |
| Finding 13 (Q8) | An Architect ruling: no rename of the seven `rh-minds` verbs in P1a. The Architect files the BCK-002 item at close | 75% |

**D59, restated:**
1. Sizes per X8.
2. The spec PRs, T1-A and T1-I/S (X7).
3. Classes: 023, 024a and 025 are C2, and 024b is C3 for `BUILT` (M7 and T1) with N3.
4. The ADR-0002 addendum (X11).
5. The shared owners X1, X2 and X4.
6. The grief edge through the instant log (X1(c)).
7. The break band, with V-SAGA-16b non-vacuous (P1).
8. The merge graph X9.

## 0a. Cross-plan rulings X1–X13 (2026-10-18)

These rulings come from REV-WAVE7 (K27, K28, K1, K10, K13, K24, K32) and REV-MINDS (Q1, Q2, M2, M3, M4, N1), the two independent reviews of 2026-10-18. They bind all four plans.

**Rule:** every "whichever lands first" clause is deleted. Each mechanism has exactly one builder, and every other plan consumes it.

| # | Ruling | Confidence |
|---|---|---|
| X1 | **One engine-owned per-instant record, the "instant log".**<ul><li>It lives in `rh-core`, is read-only to steps through `StepCtx`, is cleared at the instant's end, and is outside every hash, keyframe and journal.</li><li>It is filled only at the serial merges of the instant, so it is thread-invariant.</li><li>It holds:<ul><li>(a) the person rows inserted this instant (`inserted()`), in ascending row order;</li><li>(b) the persons who exited this instant (`exited()`): rows whose `person.alive` went from 1 to 0 at a merge, found by scanning only when `person.alive` was written at that merge;</li><li>(c) the events merged so far this instant, by (kind, target), with their EventIds (A-59).</li></ul></li><li>**Builders:** SPR-038 stage B builds (a) and (b), and the log itself, to EVO-002's text if EVO-002 has merged, or to its own text otherwise, which EVO-002 then extends. SPR-021 T-02 builds (c) under EVO-002.</li><li>**Dropped:**<ul><li>SPR-023's `exits()`/`entries()`; `entries()` has no consumer;</li><li>SPR-021's EVO-002 part (ii), lagged foreign inputs to a derived index, until a consumer exists;</li><li>SPR-025's M2 `event_ref` column.</li></ul></li><li>**Merge-time resolution of a cause** (an `EventRef` given as "the same-instant event of kind K about target X", resolved at the merge, refused by name if absent) is allowed as API sugar over (c). EVO-002 says which form each consumer uses.</li></ul> | 65% |
| X2 | **One core-owned genealogy index over the life-course log** (children-by-parent; parents are already columns).<ul><li>It is maintained incrementally at the core's life-course append, O(births), and is outside every hash.</li><li>It is read by M7 (through SPR-017's host `Indexes`), T1 and M2 if wanted. No system keeps a private copy.</li><li>**Builder:** SPR-021 T-02 (its first consumer), specified in EVO-002. SPR-024's row 12 becomes a consumer test.</li></ul> | 70% |
| X3 | **M2's elite lookup against `house_member` (K10):** no foreign-input index.<ul><li>M7 0.6.0 states that `house_member` rows are kept in ascending `person` order: a sorted insert, which compaction preserves.</li><li>M2's F4/F6 read is a binary search on the lagged column. One sentence in SPR-015's plan; SPR-021 tests it with M7 registered.</li></ul> | 65% |
| X4 | **World-start naming has one route, M7's.** M7's dormancy pass at tick 0's `Year/Polity` sends `Promote` (SPR-022). Genesis names only the manifest's `cast`.<ul><li>SPR-023 row 6 becomes "T1 creates a mind for each person `Promote`d at t = 0, and for the cast".</li><li>The `[initialization]` texts of T1 (T1-A spec PR) and M7 (T-00) are reconciled to this.</li></ul> | 85% |
| X5 | **The first real M7 + T1 integration belongs to SPR-024b**, after SPR-022a (dormancy). SPR-023 and SPR-024a integrate against M7's stub with a scripted `Promote` set. Probes in SPR-024b: at least 50 minds at t = 0 on `manor` and `twin_vales`, at least one marriage, and one pressed claim. | 80% |
| X6 | **One `BUILT` move for M7 and T1 together, in SPR-024b's `[C3]` commit** (K13, K32). SPR-021, SPR-022, SPR-023 and SPR-024a are then **C2**, and test with their own built list (`compile` takes the list). This saves three header rebaselines. | 60% |
| X7 | **Spec PRs before their sprints**, each its own reviewed PR, like `control_v1`, scheduled and counted in PRJ-002:<ul><li>**T-00a** (C2): `next/M7` 0.6.0, the shared amendments, with EVO-002 (X1, X2) and an **executable Python model** of the instant log and hook timing. About 3–4 days plus two review rounds.</li><li>**T-00b**: the amendments only SPR-022 needs (F-10, F-19, F-20, F-25), reviewed in parallel.</li><li>**T1-A** (0.4.0): the A-steps, `person.id`, `mind.person`, world-start per X4, the crate pin, and an exact reference `specs/minds/selftest_minds_v0.py` with its A-cases.</li><li>**T1-I/S**: the I- and S-steps, `[[observes]]`, `break_timer` as a flag, and V-SAGA-16b's gate fixture. It comes after T-00a merges, as a `next/` draft promoted by SPR-024a.</li></ul> The Architect commissions the drafts now, starting with T-00a and T1-A in parallel. | 75% |
| X8 | **Sizes and splits:**<ul><li>SPR-021 → **021a** (T-01–T-03) and **021b** (T-04–T-06), about 8 days.</li><li>SPR-022 → **022a** (T-01–T-04) and **022b** (T-05–T-06, after SPR-019b), about 5.5–6 days.</li><li>SPR-023 is one sprint of 4.75–5.25 days.</li><li>SPR-024 → **024a** (T-01–T-02, C2) and **024b** (T-03–T-05, C3 for `BUILT`), 6.25–7 days.</li><li>SPR-025 is one sprint of 5.25–6 days.</li><li>The spec PRs are counted separately.</li></ul> | 65% |
| X9 | **Merge order (a graph, not a line):**<ul><li>SPR-014 → {SPR-020, SPR-038a} → SPR-015 → SPR-016 → wave 6;</li><li>SPR-038b in any gap after 038a;</li><li>T-00a and T1-A in parallel now; T1-I/S after T-00a;</li><li>021a after SPR-015, SPR-017, SPR-020 and T-00a; then 021b; then 022a; 022b after SPR-019b;</li><li>SPR-023 after SPR-015, T1-A and SPR-038b;</li><li>024a after 021b, SPR-023 and T1-I/S; 024b after 022a;</li><li>SPR-025 after 024b;</li><li>all of them before SPR-028 and SPR-030.</li></ul> | 65% |
| X10 | **CI.** PR #89 (A-63 as built) moves about 295 s out of part core into golden and stream (estimated slow core about 9:55). SPR-038a takes more. Each plan:<ul><li>lists its gates by role, for each of the three parts;</li><li>budgets by gate sums on one runner, ×1.5;</li><li>assumes #89 merged.</li></ul>K24's base is replaced by #89's first CI run. | 75% |
| X11 | **D59 item 4 (no `bevy_ecs` in P1a) is an ADR-0002 §2 addendum**: "P1a: named actors are columns behind `rh-minds`; `bevy_ecs` is revisited at P3.2b". It names the five doc lines (`docs/07:28`, `docs/14:120, :149`, `05.0:243`, `05.1` §5). It is a director item and says so. | 80% |
| X12 | **`docs/13` §4.2 gains a class rule** (K16): "a model change to a system registered in no golden world is C2; the PR proves it by its `systems` lines and `BUILT`; its rates are reported to the director at merge". This is a director item in D58. | 70% |

### X13. Addendum (2026-10-18), on the revision-1 questions of wave 7 (`b2c6fb1`) and the minds plans (`f791a38`)

| # | Ruling | Confidence |
|---|---|---|
| X13a | **`TitleId` and `HouseId` are built by SPR-019 T-01**, as its plan already says (`SPR-019:19`, `:111`: the `house/v1` and `title/v1` layouts). SPR-019 stage A precedes wave 7 in X9. SPR-021a consumes them. The wave-7 proposal to move them to 021a is declined | 80% |
| X13b | **The back-fill fold stays under REV-SPR-019 R13's rule, defined both ways.** SPR-019b folds `[algorithm.dynastic_backfill]` into whichever `next/M7` draft is current: 0.6.0 if T-00a has merged, or a new draft over the frozen file if 021a has promoted M7. T-00a does not fold it, because T-00a is in flight and A-62 has just edited B6. The proposal that "SPR-019b follows 021a" is declined | 60% |
| X13c | **`rh-contracts::m7` is built only by 021a.** SPR-023 reads M7's columns by name through `SystemView`, as SPR-015 reads `person.named`. 024a switches to the contract | 75% |
| X13d | **Dormancy is in 022a**, as revision 1 places it, so 024b can follow 022a | 85% |
| X13e | **024b also waits for SPR-019b**: its X5 probes run on `manor` and `twin_vales`, whose dynasties come from the back-fill. X9 is amended accordingly | 85% |
| X13f | **`mind.break_timer` stays `u64`.** The next-version lint refuses a type change on a frozen column (`lint_odd.py:576–578`). T1-I/S amends its meaning to a 0/1 flag | 90% |
| X13g | **SPR-015's plan gains X3's sentence:** "M2's F4/F6 lookup in `house_member.person` is a binary search over the lagged column, which M7 keeps in ascending person order (M7 0.6.0)". The Architect adds it in the wave-7 close commit | 85% |
| X13h | **K19:** a daughter already married at her majority date draws no vocation | 60% |

## 0b. Architect rulings, round 2 (2026-10-18), on `f791a38` (joint with wave 7)

The rows addressed to the minds plans are R4, R5, R7–R11, R13–R15 and R17; R1–R3, R6, R12 and R16 are wave 7's (REV-WAVE7).

The joint independent re-check found the following:

| Set | Verdict | BLK | MAJ | MIN | NIT |
|---|---|---|---|---|---|
| Wave 7 | AFTER SMALL FIXES | 0 | 3 | 4 | 1 |
| Minds | AFTER SMALL FIXES | 0 | 1 | 5 | 1 |
| The union | — | — | — | 2 | — |

The report is in `p1a_recheck/REPORT_R2.md`. **Every finding is accepted unless a row says otherwise.** X13 was issued after the drafters started, so R1 and R2 are applications of X13, not defects of judgement.

| # | Ruling | Conf. |
|---|---|---|
| R1 | **Apply X13a.** SPR-019 T-01 builds `TitleId`/`HouseId`; 021a T-02 only adds the `EntityKey` impls and the `EntityKind::Title` tag. 021a depends on SPR-019 stage A, which goes in its depends list, blockers, graph and D58 | 85% |
| R2 | **Apply X13b.** Delete the T-00a back-fill-fold proposal and the dotted edge "SPR-019b after 021a". State REV-SPR-019 R13's rule, both ways. D58 item 5 no longer offers either move | 85% |
| R3 | **Accepted. The K3 mechanism is corrected.** T-00a puts the 16 defaults in `specs/systems/next/registry_delta.toml` under `[systems."M7.dynasty"]` (M2's mechanism), and they move into `registry_v0.toml` at 021a T-01. T-00a fixes `lint_odd.py`'s self-check (`:1574–1578`) to pick a param whose frozen spec has a `default`, and counts this in its size. The T-00a drafter is told directly | 85% |
| R4 | **Accepted.** The X5 probe is: minds at t = 0 equal the golden's `named_at_start full` count for that world and seed (read from `fixture_worlds_v0.txt`, never typed), and at least 20 on `twin_vales`, plus one marriage and one pressed claim. On `manor`: the golden's figure, with no floor | 85% |
| R5 | **Accepted.** Add 019b, and SPR-019 stage A for 021a, everywhere: 21:309, 22:271, D58, D59 item 8, PRJ-002 §3 graph and §4 lane | 90% |
| R6 | **Accepted.**<ul><li>PRJ-002 §6 gains T-00a (about 3–4 days, C2) and T-00b (about 1.5–2 days, C2) rows.</li><li>T-00b merges after T-00a and before 021a T-01, so both land in 0.6.0.</li><li>Rows 154 and 21:64 are reconciled.</li><li>The M7 row's "SPR-019b folds" follows R13.</li></ul> | 80% |
| R7 | **Ruled: T1-I/S merges after SPR-023 T-01 promotes T1 0.4.0**, as a new `next/T1` 0.5.0 draft promoted by 024a, because `next/` holds one draft per system. It sits on 024a's critical path; state that | 70% |
| R8 | **Accepted, with a change.**<ul><li>T1-I/S gives T1's parameters `default`s through `registry_delta.toml` (as R3 does), and 024a T-01 moves them into the Registry.</li><li>`manor.yaml` is not edited, so SPR-019's manor goldens do not move.</li><li>024b T-05 compiles `manor` with T1 built against the defaults.</li></ul> | 70% |
| R9 | **Resolved by SPR-038 revision 2** (T5): `exited()` is ascending by PersonId across the whole instant. Cite that; do not sort in M7 | 90% |
| R10 | **Accepted.** EVO-002 adds `events_of(kind)`, the instant's merged events of one kind in merge order, beside the (kind, target) lookup. T1 lists `MARRIAGE` through it. The T-00a drafter is told directly | 75% |
| R11 | **Accepted.** The house → title-holders lookup is an **M7-owned** derived index, exposed by `rh-contracts::m7` (021a), and cited in SPR-024 row 5. It is not in `rh-core` | 80% |
| R12 | **Accepted.** 021a T-03 replaces SPR-019's `DynastyParams` (after stage A) with the full M7 parameter set, and says so | 85% |
| R13 | **Accepted.** V-SAGA-16b's floor applies per variant; the PR tier's count is stated (or the gate is nightly-only, and the plan says so); one planted FAIL is asserted | 85% |
| R14 | **Ruled: SPR-038b changes `DropReason` once,** because it precedes both 021a and SPR-023:<ul><li>`Ended`'s generic meaning: "the timer's or command's premise no longer holds";</li><li>`Deferred`;</li><li>the `cancelled{system, timer kind, reason}` counter;</li><li>the `05.1` sentence.</li></ul>021a and SPR-023 consume them. The SPR-038 drafter is told | 70% |
| R15 | **Accepted.** Fix the graph nodes: S024 → S024a/S024b, S038 → S038a/S038b; add 021a/b, 022a/b, T-00a, T-00b, T1-A and T1-I/S nodes. Correct the stale BLUF count. The Architect resolves the PRJ-002 and `decisions_pending` union at merge, with one version bump | 85% |
| R16, R17 | **Accepted.** X13h is ruled, not proposed; state the vocation mutant's kill condition; fix the row citation | 90% |

## 0c. Architect's verification of revision 2 and the closing ruling (2026-10-18), on `653c5e9`

Every minds row of §0b is present in `0e3337a` (BLUF). `gen_manifest --check` builds cleanly on `653c5e9`.

| # | Ruling | Conf. |
|---|---|---|
| C1 | **The `manor` figure for R4's probe.** No golden holds a `named_at_start` figure for `manor`: `fixture_worlds_v0.txt` has `twin_vales` and `kingdom_10k` only, and its reference knows only those two worlds (`selftest_fixture_worlds.py:170` `WORLDS`; `:95–98` `SEEDS`). **Ruled as the drafter proposed:** the T1-I/S spec PR adds a `manor` case at seeds 1 and 2 (the seeds SPR-019's golden pins, `population_synthesis_v0.txt:70`, `:107`) to that reference and renders its line by `--write`, never by hand. It is a blocker of 024b T-05 (SPR-024 blockers row 2). One owner: T1-I/S, the probe's first consumer and a reviewed spec PR | 70% |
| C2 | **The PRJ-002 and `decisions_pending` union with wave 7 and wave 8** is the Architect's at their merges (R15): every D-row once, one version bump per merge | 90% |

## 1. Round-1 report (`3beca61`)

The review was made on a model different from the drafter's. The code was read from SPR-014 `921d372`, the wave-7 plans from `d37fc5b` and SPR-038 from `fb5d68b`. The reviewer could not write files, so the Architect recorded this report from its returned text. The evidence is in `minds_review/work/` (`cascade_mc.py`, `cogs04_se.py`).

### Summary

| Plan | Verdict | BLK | MAJ | MIN | NIT |
|---|---|---|---|---|---|
| SPR-023 | Ready after fixes | 0 | 3 | 6 | 2 |
| SPR-024 | Ready after fixes, and split | 0 | 2 | 4 | 1 |
| SPR-025 | Ready after fixes | 0 | 2 | 3 | 2 |
| Across the plans and D59 | Fix before approval | 0 | 2 | 1 | 0 |

**Totals: 0 BLK, 9 MAJ, 14 MIN, 5 NIT.** Of the drafter's 23 findings, 20 are real and 3 partly real. The fix for finding 4 (the exits list) is unsound as designed.

### Findings

#### SPR-023

| ID | Sev | Problem | Fix |
|---|---|---|---|
| M1 | MAJ | T1 declares no `[[reads]] person.id`, so `person_row` returns `UndeclaredRead` (SPR-014 `world.rs:249–255`). SPR-021's F-5 is the same gap | Add the read in the T1 spec PR, plus a registration test |
| M2 | MAJ | The design of `exits()`/`entries()` rests on a false premise: the ledger guard keeps no before-image of `person.alive` (`ledger.rs:254–300`), and SPR-038 compiles the value checks out of release. It also duplicates SPR-038's `inserted()` and SPR-021's EVO-002. T1 reads no births | One surface, owned by SPR-038: `exited()` beside `inserted()`, filled at the merge when `person.alive` was written. Drop `entries()`. SPR-023 T-02 becomes a consumer test |
| M3 | MAJ | The spec PR is right but has the wrong shape and no schedule. It is about 6–9 Architect-days plus two reviews (comparable references run 1,800–2,500 lines). The I and S parts depend on SPR-017, SPR-020 and SPR-021 outputs | **Two reviewed PRs:** T1-A now, blocking SPR-023 (0.4.0); T1-I/S after SPR-021 T-00, blocking SPR-024 and SPR-025 (0.5.0, or kept in `next/`). Both go in PRJ-002, each with its own `check_all` line |
| M4 | MIN | World-start naming: route (b) duplicates SPR-022, and the plan names the wrong sprint | Genesis names the manifest cast only; the dormancy set arrives by `Promote` at tick 0. The partner is SPR-022 |
| M5 | MIN | `DropReason` has no `deferred` variant, and there is no cancel counter | Add both to T-02 core, or use `Ended` |
| M6 | MIN | `Demote` "cancels the actor's timers", but `own_timers` also returns `DEATH_DUE` and `MARRY_DUE` (ADR-0025 item 1) | Name the timer kinds cancelled |
| M7 | MIN | Index freshness inside a binding (opinion rows appended, then summed, in one step) | State the rule (scan past the index's covered length, or a step-local overlay), with tests |
| M8 | MIN | Size | 4.75–5.25 days, plus the spec PR |
| M9 | MIN | CI: the debug `rh-minds` tests (about 15 s) are in part core | A gate × profile × horizon × cost table; the heavy runs go to golden or nightly |
| M10 | NIT | "Rows sorted by `mind.person`, appended at first promotion" contradicts itself | Append in promotion order and iterate through the index |
| M11 | NIT | PRJ-002 is stale at `:17`, `:182`, `:201` and `:213` | Edit with D59's outcome |

#### SPR-024

| ID | Sev | Problem | Fix |
|---|---|---|---|
| N1 | MAJ | The real M7 integration has no minds: after SPR-021 alone, M7 sends no `Promote` (dormancy is SPR-022's). Neither plan set owns the integration | 024b's T-05 follows SPR-022's dormancy task, or scripts the `Promote` set. Probes: at least 50 minds at t = 0, one marriage, one pressed claim. Name one owner in all four plans |
| N2 | MAJ | Size: about 6.25–7 days, not 4.5 | **Split:** 024a (T-01, T-02, V-COG-03/04 on the probe type; C2) and 024b (T-03 to T-05; C3, header only) |
| N3 | MIN | The channel-A clause is false: the segment header also carries `manifest_hash` (`segment.rs:108–110`) | Add it |
| N4 | MIN | `MURDER_ATTEMPT.payload` is a third payload convention | Use F-8's `Payload:` clause |
| N5 | MIN | `MARRY_DUE` is not revalidated when it fires, and `MARRIED_TO` is written at submit | Revalidate at fire against N7's predicate. Write `MARRIED_TO` after M2 applies, by observing `person.spouse` |
| N6 | MIN | Timer details: (a) a first poll can land at the promotion's own instant, which `schedule()` refuses; (b) interrupts and refractory ends have no RNG purpose; (c) the poll anchor is unstated | "Strictly after"; a purpose and kind per timer; anchor at first + k·period |
| N7 | NIT | The "SPR-022 re-records the headers" sentence is wrong | Delete it |

#### SPR-025

| ID | Sev | Problem | Fix |
|---|---|---|---|
| P1 | MAJ | **V-SAGA-16b is vacuous at `twin_vales`' values.** With threshold 70, gain 20 and cap 40, the Registry bounds give cap ≤ 45 < threshold ≥ 50. `cascade_mc.py` finds 0 breaks in 92,160 actor-years. The ratio is 0/0, and the "teeth" mutants cannot fire | Report the ratio as "NOT EXERCISED" below a minimum count, with the gate failing below N breaks. Run the gate fixture at a stress-capable parameter set and report `twin_vales` only. Add a massacre variant (4 deaths at one instant). Take the band out of the gate, and probe that breaks occurred |
| P2 | MAJ | `mind.break_timer` cannot hold a TimerId: `schedule` returns `()`, re-sampling changes the id, and `invariants()` cannot see the agenda | Make it a flag, find the timers through `own_timers`, and check INV-T1-02 with an observer |
| P3 | MIN | Row 3 misdescribes A-59: the M2 `event_ref` column is SPR-021's rejected alternative | Reword. T1 declares `[[observes]]` |
| P4 | MIN | Breaks in the same instant fire in key order on one view, so the second break sees the first's relief | State the snapshot or the order; add a case |
| P5 | MIN | Size | 5.25–6 days, one sprint |
| P6 | NIT | The `MURDER_ATTEMPT → MENTAL_BREAK` `triggered` edge is missing | Add it |
| P7 | NIT | The Registry content lands with no `registry_version` bump | Note it; bump in SPR-030's C3 |

#### Across the plans

| ID | Sev | Problem | Fix |
|---|---|---|---|
| Q1 | MAJ | "Whichever lands first builds it once" is false, because the mechanisms differ: this instant's deaths (SPR-023 `exits()` against SPR-021's EVO-002 against SPR-038's `inserted()`), births, the genealogy index (three copies), A-59, world-start naming, and `rh-contracts::m7` | **Owners:** SPR-038 for `exited()`/`inserted()`; one core genealogy index (Architect, 6/10); A-59 at SPR-021 T-00; world-start naming at SPR-022; `rh-contracts::m7` at SPR-021 |
| Q2 | MAJ | D59 hides decisions: item 4 amends ADR-0002 §2 (a director-owned ADR), item 5 contradicts D58, and item 6 is already settled by D58 item 3 | Item 4 names ADR-0002 and an addendum ("P1a: columns; revisit at P3.2b"), with the five doc lines. Items 5 and 6 are restated |
| Q3 | MIN | CI and A-63: no plan says which A-63 job each gate joins | List the gates by role, and assume A-63 merged |

### The drafter's 23 findings

20 are real and 3 partly real:
- **#4:** the need is real; the mechanism is wrong (M2).
- **#14:** the V-COG-10 scope is acceptable (6/10).
- **#22:** `bevy_ecs` is sound technically but incomplete, because ADR-0002 is not amended.

The following were checked as sound:
- **#3:** pin `rh-minds` as (A, system) at 7/10; the alternative is renaming to `rh-sys-minds` (5/10).
- **#6:** the `due` hand-off is feasible.
- **#7:** keeping the tick on re-sample is legal.
- **#8:** the (u, v) grief shapes are right.
- **#12:** the `DEATH_DUE` timer.
- **#13:** Q8. Record it as an Architect ruling and file the BCK-002 item.
- **#23:** V-COG-04's SE is 0.0057, and the by-id mutant gives ρ = −0.577.

### D59

| Item | Verdict | Conf. |
|---|---|---|
| 1. Sizes | SPR-023 one sprint, 4.75–5.25 days; SPR-024 split, 6.25–7; SPR-025 one sprint, 5.25–6 | 65% |
| 2. Spec PR | As two PRs (T1-A, then T1-I/S), scheduled in PRJ-002 | 75% |
| 3. Classes | C2 / C3 (header only) / C2, with N3; 024a is C2 | 80% |
| 4. No `bevy_ecs` | Approve, naming ADR-0002 and its addendum | 80% |
| 5. Shared mechanisms | Replace with Q1's ruling | 60% |
| 6. The grief edge | Approve the fallback, restated against D58 item 3 | 65% |
| 7. The break band | Approve, with V-SAGA-16b made non-vacuous | 75% |

**Merge order (6.5/10):**
1. SPR-015
2. SPR-038, with `exited()`
3. The T1-A spec PR
4. SPR-023
5. SPR-021 (T-00 and EVO-002 first)
6. SPR-022
7. SPR-024a
8. SPR-024b `[C3]`
9. SPR-025

### What the reviewer ran

- `gen_manifest --check` and `lint_odd.py` on the branch.
- `cascade_mc.py` (stylised) and `cogs04_se.py`.
- `git show` of the SPR-014 sources.

It ran no cargo and no `check_all`.

## 2. Round-2 report (`f791a38`, joint with wave 7's `b2c6fb1`)

An independent re-checker, on a model different from the drafters'. The Architect recorded this report from its returned text.

### BLUF
| Set | Verdict | Rulings | New findings |
|---|---|---|---|
| Wave 7 | AFTER SMALL FIXES | K1–K33: 30 applied, K3 applied but defective (R3), K5 and K31 partial. X13a and X13b contradicted, X13e not applied | 0 BLK, 3 MAJ (R1–R3), 4 MIN, 1 NIT |
| Minds | AFTER SMALL FIXES | All applied. X13c and X13f applied; X13e only in SPR-024 | 0 BLK, 1 MAJ (R4), 5 MIN, 1 NIT |
| Union | — | — | 2 MIN (R14, R15) |

### Findings
| ID | Sev | Problem |
|---|---|---|
| R1 | MAJ | X13a contradicted: 021a builds `TitleId` and `HouseId`, while SPR-019 still builds them; 021a lacks the stage-A dependency |
| R2 | MAJ | X13b contradicted: the T-00a back-fill fold and "SPR-019b after 021a" |
| R3 | MAJ | K3 makes T-00a red. A draft `default` fails `lint_odd.py:1171–1179` in `--next` mode. A default in the overlay passes that check, but the lint's own self-check then asserts at `:1574–1578`. Verified on a copy |
| R4 | MAJ | The X5 probe "at least 50 minds at t = 0" is false for a correct build: the golden's `named_at_start full` counts for twin_vales seeds 1–3 are 33, 35 and 26, with no cast |
| R5 | MIN | X13e (024b waits for 019b) is missing outside SPR-024 |
| R6 | MIN | PRJ-002 §6 has no rows for T-00a or T-00b, T-00b has no size, and their order is unstated |
| R7 | MIN | T1-I/S cannot merge while the 0.4.0 draft is in `next/`, which holds one draft per system |
| R8 | MIN | `manor.yaml` has no `T1.minds` section, and T1 has no defaults |
| R9 | MIN | `exited()` ordering across the instant's merges |
| R10 | MIN | There is no accessor that lists events by kind, so T1 cannot list this instant's `MARRIAGE` events |
| R11 | MIN | K31's house → title-holder lookup has no sound home in `rh-core` |
| R12 | MIN | Two builders of M7's parameters in `WorldSpec` |
| R13 | MIN | V-SAGA-16b is still soft: the floor is not per variant, the PR tier's count is not shown, and no planted FAIL is asserted |
| R14 | MIN | The widened meaning of `DropReason::Ended` is too narrow, and two sprints edit the same enum |
| R15 | MIN | PRJ-002's graph has dangling and missing nodes, and its BLUF is stale |
| R16 | NIT | K19 is labelled "Proposed" although X13h ruled it |
| R17 | NIT | A row citation is wrong |

### Merge tests
- Each branch merges cleanly into `main`.
- Against each other they conflict in two files:
  - `PRJ-002`: the version line, adjacent rows, and the §3 graph;
  - `decisions_pending`: the D58 and D59 rows.
- `arch/spr038-plan` conflicts in the same two files.
- The resolution keeps both sides and bumps the version once.
