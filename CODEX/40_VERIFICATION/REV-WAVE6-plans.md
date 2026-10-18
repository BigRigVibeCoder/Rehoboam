---
id: REV-WAVE6
title: "Independent review of the wave-6 sprint plans: SPR-018, SPR-019 and D57"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, BCK-002, SPR-018, SPR-019, SPR-014, SPR-015, SPR-016, SPR-020, SPR-021, SPR-026, SPR-027, SPR-030, REV-SPR-018, REV-SPR-019, REV-WAVE5, REV-M7-membership]
created: 2026-10-17
updated: 2026-10-17
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing commit (2026-10-17). The two wave-6 plans go to the director as D57.**
>
> | Plan | Revision | Size | Class |
> |---|---|---|---|
> | SPR-018 (worldgen-lite, M1 F0, S3 F0, the validator's two refusals, bench 4) | 0.3.0 | about 4.25–4.5 agent-days, one sprint, hard checkpoint after T-02 | C3: `solo`, `household` and `hamlet` move |
> | SPR-019 (population synthesis; the dynastic back-fill) | 0.3.0 | stage A about 3.25 agent-days; stage B about 3.25 | A is C3 (`hamlet` moves); B is C2 and waits for "(e) settled" under D56 |
>
> - **Round 1** (`a1d25d9`): 0 BLK, 7 MAJ, 13 MIN, 8 NIT (§1). The majors:
>   - per-instant invariants under SPR-014 (X1);
>   - the format bump's ripple (X2), so kind 7 and the regional digest are **deferred to S3 F1 in P1b** (D57 item 2; BCK-002 A-61);
>   - M7 is not promoted in SPR-019 (Y1);
>   - `from_manifest` (Y2);
>   - the M2 fold in stage A (Y3);
>   - the D56 table (Y4);
>   - CI (Z1).
>
>   Rulings: §0.
> - **Round 2** (`f6bf6d2`): 0 BLK, 0 MAJ, 8 MIN, 12 NIT (§2). Dispositions: §0b. Revision 2 is `7b010a8`.
>
> **Reviewers:** two independent sessions, each on a model different from the reviser's.

# REV-WAVE6: independent review of the wave-6 sprint plans

## 0. Architect rulings, round 1 (2026-10-17), on `a1d25d9`

The independent review of `a1d25d9`, checked against `main` at `39b8d76`, found **0 BLK, 7 MAJ, 13 MIN and 8 NIT** (§1). Of the drafter's twelve contract findings, nine are real and three partly real. **Every finding is accepted unless a row says otherwise.** The drafter applies these rulings in one revision on `arch/wave6-plans`, merges `origin/main` (keeping both D56 and D57 rows, D57 after D56), and pushes once **without force**. The Architect then verifies.

| # | Ruling | Confidence |
|---|---|---|
| X1 (MAJ) | **Accepted. Nothing in M1 or S3 runs O(cells) or O(rows) per instant.**<ul><li>INV-M1-01 to 05 are checked at genesis and at restore, in `rh-worldgen` and the tests.</li><li>INV-M1-06 becomes the ladder's per-column root test (V-DET-01).</li><li>S3's `invariants()` is O(1) or incremental. The month-end and tiling invariants are checked by test observers.</li><li>T-07 measures the cost with SPR-014 merged, and cross-references BCK-002 A-60 (per-instant engine cost).</li></ul> | 85% |
| X2 (MAJ) + F-1 | **Ruled: defer kind 7 (`RESIDENCE_INTERVAL`) and the regional digest to S3 F1 (P1b). This is a director item in D57.**<ul><li>`channel_a_v0.sql` calls the v0 shape "final for P1a". SPR-020's envelope tables were folded into v0 before SPR-013 wrote a segment, so kind 7 would be the first real format bump.</li><li>The ripple is wide: `gen_fixtures.py`, the annals fixtures, `wire_v1.txt`, SPR-026's `CANONICAL_FORMAT`, `rh-stream`'s canonical test and `wire_v1.md`. It would land while wave 4 and wave 5 are in flight.</li><li>Residence at birth stays in state (`person.residence`). The materializer (SPR-027) and the keyframe extract show it.</li><li>SPR-018b therefore leaves wave 6. SPR-018 is stage A plus T-04 (the validator refusals, C2 inside the C3).</li><li>The alternative, if the director wants the regional digest in P1a: the v1 bump with X2's full ripple list, landed first as a reviewed spec change (W30), then the code.</li></ul> | 65% |
| X3 (MIN) | **Accepted.** S3's `[[reads]]` of `region.land_cells` gets a typed `neutral` (or `required`) at the freeze, listed with NIT-A to C. | 90% |
| X4 (MIN) | **Accepted.** Key the mutant by a member other than the head, or add a fixture whose head is not first. | 90% |
| X5 (MIN) | **Accepted**, in both plans: after the named month only W3 (b) binds; or enumerate what may differ (`STEP_DIGEST` `living`, `births` and `deaths`; `EXTINCTION`'s tick and seq). | 90% |
| X6 (MIN) | **Accepted. No M2 text edit:** cite `envelope_v0.md`:55. T-03 states SPR-020 (`Event.place`) as a precondition, with the fallback stated. | 85% |
| X7 (MIN) | **Accepted.** Expect the rungs' allocation budgets (V-PERF-11) to move, because of the lagged-column clones per instant. Note it under A-60, and plan the `alloc-record` in the `[C3] [perf-baseline]` commit. | 85% |
| X8 (MIN) | **Moot under X2.** It moves with the regional digest to P1b, and the backlog item records it. | — |
| X10 (MIN) | **Accepted, re-sized after X1 and X2.** SPR-018 is about 4.25–4.5 agent-days as **one C3 sprint** with a hard checkpoint after T-02. | 70% |
| X9 and the SPR-018 NITs | X9 is moot under X2 (it is noted in the P1b backlog item: entry 0, residence 1, exit 2). The other NITs are accepted. | 90% |
| Y1 (MAJ) | **Accepted. Fold `[algorithm.dynastic_backfill]` and INV-BF into the `next/M7` 0.5.0 draft without promoting it.** SPR-021 promotes M7, after D56. `next/` keeps the M7 draft, so `check_all`'s deny-by-default controls keep their target. | 85% |
| Y2 (MAJ) | **Accepted.** `"dynasties"` joins S3's `initialization.from_manifest` at its SPR-018 freeze, and M2's through an `[[amends]]` on `initialization.from_manifest` in the 0.3.2 fold (stage A). Both are listed in the PR body. | 85% |
| Y3 (MAJ) | **Accepted.** The M2 0.3.2 fold (`[algorithm.population_synthesis]`, with INV-PS as `[[invariants]]`, Y6) lands in **stage A**, with the code that builds it. Stage B keeps the M7 fold, unpromoted, and the back-fill. | 85% |
| Y4 (MAJ) | **Accepted. Restate the D56 table against what merged:**<ul><li>membership (all three rules agree on every back-filled row) and lever (d) have no effect at genesis;</li><li>under (e), B6's text, `population_synthesis_v0.txt`, `fixture_worlds_v0.txt` and `crn_pairing_v0.txt` regenerate in (e)'s own M2 PR, before SPR-015 freezes 0.3.0;</li><li>stage B's gate is "(e) settled": approved and landed, or declined.</li></ul>Drop the `member_of` hook. | 85% |
| Y5–Y9 (MIN) | **Accepted.**<ul><li>Y5: the IPF mutant uses 2 sweeps.</li><li>Y6: INV-PS folded as M2 `[[invariants]]`.</li><li>Y7: the orphan count goes to `run.toml`, or is printed only when nonzero.</li><li>Y8: drop `--stage a`; T-01 takes 0.5 day; SPR-019 is about 6.25–6.5 days, A about 3.25 and B about 3.25.</li><li>Y9: the kind-7 criteria are dropped, because kind 7 is deferred (X2).</li></ul> | 85% |
| SPR-019 NITs | Accepted. | 90% |
| Z1 (MAJ) | **Accepted.**<ul><li>D57 item 4 is withdrawn. The third PR-tier job is D54 item 6, which the Architect has put to the director with current numbers (recommended, 70%).</li><li>Each plan restates its added time per part against the measured state after wave 4 (budgeting for slow runners), and names its nightly candidates.</li><li>Path-scoping the pure-Python spec references (run only when their inputs change) becomes a BCK-002 item for the Architect, not part of these plans.</li></ul> | 80% |
| Z2 (MIN) | **Accepted.** Restate PRJ-002's rows and D57 after Y1, Y4, X2 and Z1. | 90% |

**D57, restated after these rulings:**
1. Sizes: SPR-018 as one sprint, about 4.25–4.5 days; SPR-019 split, A about 3.25 days and B about 3.25 days.
2. **Kind 7 and the regional digest deferred to S3 F1, in P1b** (65%; the alternative is a v1 bump as a reviewed spec change).
3. No `manor` golden.
4. CI: see D54 item 6.
5. D56: stage B waits for "(e) settled". M7 is promoted by SPR-021. SPR-016 is a precondition of SPR-019.

## 0b. Architect dispositions, round 2 (2026-10-17), on `f6bf6d2`

The re-check of revision 1 (§2) found **0 BLK, 0 MAJ, 8 MIN and 12 NIT**. All 22 round-1 rulings were applied, or applied in substance. **Every finding is accepted.** The reviser applied all of them except V6 in revision 2 (`7b010a8`), after merging `main` `9bf8454`:
- **V1:** `rh-core/events.meta` differs from the first event that carries a `place`, once SPR-020 is on main.
- **V2:** the mutant is keyed by the household's second dependant (k = 7 on hamlet, which moves Job's household to settlement 2), and the synthetic cases are a second kill. The reviser reproduced both results on the reference. The text says `cast[2]`.
- **V3:** V-ids stay in `test`, and the cadence moves to `statement`.
- **V4:** B7 moves to stage B. Stage A refuses a dynastic manifest by name.
- **V5:** D57 has a confidence for every item.
- **V7, V8.**
- **N1–N12.**

**V6 is the Architect's,** in the closing commit. D54 item 6 now carries wave 4's CI measurements and recommends adding the third PR-tier job (70%), which supersedes "keep two jobs". D54's regional-digest line is marked superseded by D57 item 2.

**The drafting history.** The drafter wrote revision 0 and began revision 1, then stopped when its model hit the account's monthly spend limit. A second agent, on a different model, finished revisions 1 and 2 from the drafter's worktree. The round-1 reviewer and the round-2 re-checker each ran on a model different from the reviser's.

## 1. The review of revision 0 (`a1d25d9`, checked against `main` at `39b8d76`)

#### 1. BLUF

| Plan | Verdict | BLK | MAJ | MIN | NIT |
|---|---|---|---|---|---|
| SPR-018 (worldgen-lite, M1/S3 F0, kind 7, regional digest) | Ready after fixes | 0 | 2 | 7 | 5 |
| SPR-019 (population synthesis, back-fill) | Ready after fixes | 0 | 4 | 5 | 3 |
| Across the plans, and D57 | Fix before approval | 0 | 1 (CI) | 1 | 0 |

**Totals:** 0 BLK, 7 MAJ, 13 MIN, 8 NIT.

**The drafter's 12 findings:** 9 are real and 3 partly real. Of the real ones, F-1 and F-3 are real but the plan acts on them incompletely, and the new PRJ-002 rows (F-12) are already stale.

**The majors that change the plans:**
- **Per-instant invariants (X1).** Under SPR-014, about 360 instants a year, M1's raster invariants would run at every Bookkeeping and cost minutes per CI part.
- **The format bump's ripple (X2).** It breaks gates the plan does not list.
- **M7 promotion (Y1).** The `next/M7` draft is now 0.5.0, carrying D56's content.
- **Ruling 10's `from_manifest` (Y2).** It refuses the back-fill's M2 and S3 writes.
- **Stage A's spec (Y3).** Under the split, stage A builds against an unfolded spec.
- **D56 (Y4).** The plan's D56 table is stale.
- **CI (Z1).** D57 item 4 no longer holds.

#### 2. Findings

#### SPR-018

| ID | Sev | Where | Problem | Evidence | Fix |
|---|---|---|---|---|---|
| X1 | MAJ | :132, :149, :156 | INV-M1-01 to 06 and INV-S3 run in `invariants()` "at every Bookkeeping". | `engine.rs:914` (main) and SPR-014 `:930` call `check_invariants` at every instant's Bookkeeping, in release too. SPR-014 makes that about 360 instants a year. On hamlet (R = 256), INV-M1-02's connectivity check over 53,859 land cells plus the 65k-cell passes cost about 0.5–1 ms per instant: +36–72 s per 200-year hamlet run in release. Part golden has about 10 hamlet runs, and core runs hamlet in debug (`tail_live`). Three cadences have no home (W4/W31): <ul><li>INV-M1-06 needs the "genesis roots", but M1 is built from `&WorldSpec` before genesis;</li><li>`rh-snapshot` is not an allowed system dependency (`lint_arch` `SYSTEM_ALLOWED_DEPS`);</li><li>`invariants(&self, &WorldTables)` gets no time (SPR-014 `system.rs:1057`), so "every Vital end" and "every month end" cannot be expressed.</li></ul> | <ul><li>Check INV-M1-01 to 05 at genesis and at restore (in `rh-worldgen` and the tests).</li><li>Make INV-M1-06 the ladder's per-column root test (V-DET-01). Nothing per instant.</li><li>Make S3's `invariants()` O(1) or incremental; check the month-end and tiling invariants with test observers.</li><li>T-07 measures the result with SPR-014 merged; cross-reference A-60.</li></ul> |
| X2 | MAJ | :70, :173–177 | The v1 bump's ripple list is incomplete. | I simulated the bump on a copy of main:<ul><li>`gen_fixtures.py --check` reports STALE: `gen_fixtures.py:169` hardcodes `channel-a/v0`;</li><li>`selftest_annals_v0.py --check` crashes with "not a canonical header line", because the three committed annals fixtures are v0;</li><li>`selftest_wire_v1.py --check` reports STALE: `wire_v1.txt:18,20` carry `canonical_format` from `_CANON.FORMAT`. This file is on main whether or not SPR-026 is;</li><li>SPR-026's `rh-protocol/src/control.rs:32` hardcodes `CANONICAL_FORMAT` v0, so the host would announce v0;</li><li>the literal at `rh-stream/tests/canonical.rs:766`, and `wire_v1.md:132–133`.</li></ul>`channel_a_v0.sql:11` also calls the shape "final for P1a". | <ul><li>T-05 lists all of these, with the annals fixtures and goldens regenerated by their tools and the wire golden regenerated unconditionally.</li><li>Per W30, land the DDL and canonical v1 as a reviewed spec change, with the references and goldens, before SPR-018b's code.</li><li>Alternatively, defer kind 7 and the regional digest to S3 F1 (P1b).</li></ul> |
| X3 | MIN | :63, :163 | The S3 freeze fails its own acceptance. | Status flipped on a copy: `lint_odd` gives "ERROR S3.migration: read region.land_cells from M1.regions needs a neutral value" (a rate read). | Add a typed `neutral` (or `required`) on that `[[reads]]` at the freeze, listed with NIT-A to C. |
| X4 | MIN | :153, :161 | `mutant-founder-residence-loose-head` cannot be observed on hamlet. | `job` is `cast[0]` and heads `job_house`, so "first member" is the head; `oswin` is the same in household. | Key the mutant by another member, or add a fixture whose head is not first. |
| X5 | MIN | :204 (also SPR-019 :143) | The W2 clause after divergence is false for a correct build. | After the month it names, `STEP_DIGEST` `living`, `births` and `deaths` differ, and so do `EXTINCTION`'s tick and seq. | After the named month, only W3 (b) binds, or enumerate what may differ. |
| X6 | MIN | :68, :151, :38 | `place`: an "Architect text edit" to frozen M2 0.3.1's `[[emits]]` bypasses `next/` (REV-SPR-019 R4; the lint is deny-by-default). | `envelope_v0.md:55` already states the rule. T-03 also needs SPR-020's `Event.place`; SPR-020 is still in flight (run 39073625341), and the plan's fallback covers stage B only. | Change no M2 text: cite the envelope. A `next/M2` route would collide with SPR-019's 0.3.2. State the SPR-020 precondition for T-03. |
| X7 | MIN | :75, :207 | "S3's daily scan allocates nothing" is wrong. | `run_instant` clones every lagged column every instant. M2's `person.residence` read becomes live, plus S3's three M1 reads: 4 clones per instant. | Expect the rung allocation budgets to move; note it under A-60. |
| X8 | MIN | :188 | The regional fold has no stated start state for `--from-seq N` or a live join. | Attributing a death to a region needs each resident's settlement from before N. | Seed the fold from the latest KEYFRAME at or before N (`person.residence`, `settlement.region`), or read the sealed prefix; state the cost. |
| X10 | MIN | :27 | The sizes are tight. | T-03 (the differential-model extension) and T-05 (with X2) at 1.0 each; with X1 and X2 the total is about 5.75–6 days and stage A about 4. | Move T-04 (the validator: C2, no baseline; no example manifest violates either new rule, checked) into SPR-018b. Then A is about 3.5 days and B about 2.25. |

**NITs:**
- :183: two header tokens change (`#…canonical/v1` and `format=…/v1`), not one.
- :74 and :203: the ladder's `systems` line changes too (built and not-built lists), not only `tuple`.
- :119 and :124: "every later blob's draws shift" is wrong; draws are keyed by (blob, k). The mutant still fails through the elevation.
- :188: the v0 refusal duplicates T-05's.
- :69 (X9): k = (person, 2) puts a residence record after a same-instant exit. Since the format moves anyway, use entry 0, residence 1, exit 2.

#### SPR-019

| ID | Sev | Where | Problem | Evidence | Fix |
|---|---|---|---|---|---|
| Y1 | MAJ | :48, :169–175, :240, :271 | T-06 re-derives and promotes "M7 0.4.0 with exactly the three amendments". | `next/M7` on main is 0.5.0 with ten `[[amends]]` (PR #79). Promoting it freezes D56's runtime rules (agnatic `on_birth`, vocations for later children, dormancy precedence) before D56 is ruled. Companion §8 item 3 says M7 is "promoted with M7's next sprint" (SPR-021). Emptying `next/` breaks `check_all.sh:330–389`, whose 17 deny-by-default controls are planted into `next/M7`. | Fold `[algorithm.dynastic_backfill]` and INV-BF into the 0.5.0 draft without promoting it, and let SPR-021 promote. Otherwise, gate the promotion on D56 and re-point the controls at a synthetic next version. |
| Y2 | MAJ | :48, :81, :156, :64 | Ruling 10's `from_manifest` half. The back-fill writes M2's person, life-course, household and marriage columns and S3's residence columns for people from the `dynasties` section. | `check_owner` (`rh-worldgen/src/lib.rs:246–268`) refuses any column whose owner's `from_manifest` lacks the section. M2's lists (frozen `:331`, next `:742`) and S3's (`:171`) are all `["peoples","cast","households"]`. The plan says "nothing else" changes in M2 0.3.2 and "S3 is read, not changed". | Add "dynasties" to S3 at its SPR-018 freeze, and to M2 through an `[[amends]]` on `initialization.from_manifest` (before SPR-015 freezes 0.3.0, or in the 0.3.2 fold). |
| Y3 | MAJ | :105, :171 | Under the split, stage A's C3 code for P0–P12 merges while the M2 fold sits in stage B, which waits on D56 and SPR-017. | `population_synthesis.toml:3–5`: "binds nothing until the Architect folds" the tables. D-1's reasoning applies. | Put the M2 0.3.2 fold, with INV-PS (Y6), in stage A; stage B keeps the M7 fold only. |
| Y4 | MAJ | :191–203 | The D56 table no longer matches what merged. | <ul><li>Outcome (c), "bounded by degree", is A-56's old wording. D56 and `M7_membership.md:31` define bounded as the mother's house when she is named-eligible, and all three rules agree on every back-filled row (`:29–31`), so the `member_of` hook buys nothing.</li><li>D56's recommendation includes (e), the elite fertility read on the couple: an M2 amendment that makes B6's spine wives draw `c_i^e` and moves `population_synthesis_v0.txt`, `fixture_worlds_v0.txt` and `crn_pairing_v0.txt` (D56; `M7_membership.md:68`). The plan says (b) moves no golden and omits (e).</li></ul> | Restate the table: membership and (d) have no effect at genesis; under (e), B6's text and the goldens regenerate in its own M2 PR before SPR-015 freezes 0.3.0. Stage B's gate is (e) settled. |
| Y5 | MIN | :120, :125 | `mutant-ipf-sweeps-31` cannot be observed. | The reference with `IPF_SWEEPS` set to 31, 24, 16, 8 or 4 renders the four cases and the 20-seed sweep byte-identically (2 sweeps differs). The PR-tier fixture worlds are identical at 31. | Use 2 sweeps, or pin the fitted table in 2^-32 units. |
| Y6 | MIN | :171, :173 | INV-PS-01 to 07 vanish: "the table and nothing else", and the fold file becomes a pointer. | — | Fold them as M2 `[[invariants]]`. My simulated fold passes `lint --next`. |
| Y7 | MIN | :118, :142 | The orphan count "in the run header" has no path out of `genesis()`, and an unconditional line changes solo's and household's headers, against T-04's "headers included". | — | Print it only when nonzero, or put it in `run.toml`. |
| Y8 | MIN | :25, :108, :122 | The sizes are optimistic, and `--stage a` is a weak gate. | T-01 at 0.25 days covers two id layouts, `GenesisInputs` checked against two references, and the reorder: it needs at least 0.5. `--stage a` changes the spec's reference inside the sprint to make new expected lines. | Drop `--stage a`; stage A gates hamlet, which has no dynasty. Total about 6.25–6.5 days (A about 3.25, B about 3.25). |
| Y9 | MIN | :143, :164 | Its channel-A criteria assume kind-7 lines exist. | SPR-018b may not be on main when these run. | State the criteria conditionally. |

**NITs:**
- :157: `LifeTracker::resumed` already skips closed rows (`seen = len`, and `open` holds OPEN rows only); only `new` and `observe` need the rule.
- :61: the machinery-to-system alternative needs no `lint_arch` change (`check_edge` constrains system sources only); it would only set a precedent.
- :172: the 0.4.0 numbering is stale.

#### Across the plans

| ID | Sev | Problem | Evidence | Fix |
|---|---|---|---|---|
| Z1 | MAJ | CI (W42): the base the plans use (core 11:46, golden 9:01) predates SPR-014 and SPR-026. | **Gate-time sums, compared on equal runners against PR #79's head** (golden step 10:23, core 12:00):<ul><li>SPR-026 golden +143 s, all new gates; step 12:46;</li><li>SPR-014 golden +145 s;</li><li>SPR-014 core +249 s before its fix (q(15) +122, `m2_system` +85, V-DATA-04 +57, kill +29).</li></ul>**Projection on a slow runner:**<ul><li>after wave 4 (without SPR-020): golden about 15:10, core about 14–16;</li><li>after wave 5 (D54: +1.3 and +2.5): golden about 16.5, core about 16.5–18.5;</li><li>after wave 6 (the plans' own +2.4 and +0.35): golden about 19, core about 17–19.</li></ul>Runner variance: within one hour, main's golden ran 6:28 and SPR-026's 12:46. | <ul><li>Put D54 item 6 (a third PR-tier job) to the director now.</li><li>Budget against slow runners.</li><li>Path-scope the pure-Python spec references (population synthesis 25–31 s, fixture worlds 10–12 s): run them only when their inputs change.</li><li>Move q(15)'s 32 seeds and the N-thread smoke run (150–160 s) to nightly.</li></ul> |
| Z2 | MIN | The PRJ-002 rows and D57's text are already stale. | "M7 (0.4.0)", "the 0.4.0 draft", D57 item 4's base figures and item 5's analysis. | Restate them with Y1, Y4 and Z1. |

#### 3. The drafter's twelve findings

| # | Verdict | Evidence |
|---|---|---|
| F-1 format bump | Real; the fix is incomplete | `channel_a_v0.sql:6–8` and the `canonical_v0.md` header rule; SPR-013 writes segments. The ripple is X2. |
| F-2 SPR-016 | Real | P0 (`population_synthesis.toml:82`) reads MR9's knots; the reference loads `selftest_m2_f1_mortality.py` (`:87`). |
| F-3 manor golden | Real | Companion §8 item 5. Item 3 also defers M7's promotion to SPR-021, which the plan contradicts (Y1). |
| F-4 back-filled dead | Real | `convert.rs:151–215`: `new()` emits every row as an entry, so a close would be a pre-epoch `LIFE_EXIT`, which R3 refuses. `resumed` is already safe. |
| F-5 `WorldSpec` | Real | `world.rs:20–36` |
| F-6 `GenesisInputs` | Real | `rh-worldgen`'s dependencies; `world.rs:249–251`. `from_world(&spec)` makes the reorder trivial, and there is one production caller. |
| F-7 R9 refusals | Real | `manifest_v0.py:468–491` has neither. |
| F-8 `place` | Real | `envelope_v0.md:39, 55`; SPR-020 `lib.rs:71`. It depends on SPR-020 (X6). |
| F-9 "register M7" | Partly real | "Name" is right (`check_owner` reads every spec on disk), but the `from_manifest` half of the same rule bites (Y2). |
| F-10 reader-side fold | Partly real | It equals INV-S3-05's reconciliation, but it has no start state (X8). |
| F-11 generated templates | Real | `rh-worldgen` has no TOML dependency. It is a choice, since `rh-registry` has `toml`. Edit `worldgen_lite.md` §6 item 1. |
| F-12 PRJ-002 row | Real | The fixed row is already stale (Z2). |

#### 4. D57

1. **Sizes and splits: approve with changes.**
   - **SPR-018:** about 5.75–6 days in total. Move T-04 into SPR-018b; A is then about 3.5 days (after X1) and B about 2.25.
   - **SPR-019:** about 6.25–6.5 days in total. The M2 fold goes in stage A (Y3); stage B keeps only the M7 fold, unpromoted (Y1); T-01 is 0.5. A is about 3.25 and B about 3.25.
2. **Format bump v0 → v1 in place: approve only with X2's full list, landed as a reviewed spec change.** The bump is forced by the rule. Deferring kind 7 and the regional digest to S3 F1 is a valid alternative.
3. **No `manor` golden: sound.** Approve.
4. **CI: does not hold.** Golden is already the tighter part (12:46 with SPR-026). Moving 40 s of Python into it makes it worse, and both parts are projected past the 15-minute hard limit by wave 6. A third job is now unavoidable unless about 5 minutes per part go to nightly (Z1).
5. **D56 interplay: the plan stays buildable, but its analysis is wrong** (Y4, Y1).
   - SPR-016 as a precondition is real.
   - The M7 promotion belongs to SPR-021 or waits for D56.
   - Under (e), the spec goldens move before SPR-019 starts.
   - The merge order holds with these changes.

#### 5. Checked and sound

- Folding both tables, with INV-PS and INV-BF, into main's `next/M2` and `next/M7` passes the deny-by-default `lint --next` (10 M7 and 45 M2 amendments, lock matched).
- A promoted M7 0.5.0's skeleton differs only in `SPEC_VERSION`.
- Freezing M1 and S3 leaves both skeletons unchanged.
- No example manifest or parity fixture violates the two new household refusals.
- Hamlet's 50/4/2/4 residence arithmetic adds up.
- The ladder roots are per column, so SPR-018's W3 (a) is checkable; `engine.meta` is unaffected.
- `smoke` is unchanged.
- `--expect-serial` holds: S3 sits at a later level through the `same_instant` edge.
- `mutant-sibling-gap-560` changes `persons_sha256`.
- The genesis-only digests of 10k worlds are affordable.
- The compile-level fixtures follow W20.
- AGT-003-IF is assigned in both plans.
- The W3 two-part forms are non-vacuous.

#### 6. What I ran

Everything is in `the Architect's scratchpad: wave6_review/work/`. I built no Rust, so there is no `target/`, and I deleted the copies of main after use. I committed and pushed nothing.

- **The bump:** simulated on a copy of main; the gate runs are in `bump.log` and `wire_bump.log`.
- **The fold:** both tables folded into the next drafts and linted (`fold_lint.log`).
- **The M7 promotion:** simulated, with a skeleton diff (`m7_frozen.rs`, `m7_promoted.rs`).
- **The M1 and S3 freeze:** status flipped and linted (the S3 error, X3).
- **The mutants:** probed against the references (`mutants_ps*.py`, `mutants_fw.py`, `ipf_probe.log`, `ipf_fixture_worlds.log`).
- **The household refusals:** every example manifest scanned for both bad cases.
- **CI:** step and per-gate times from the logs of runs 39071719033 (SPR-014), 39072940168 and 39071812231 (SPR-026), 39071576483 (PR #79), 39073315181 (main) and others (`gate_times.py`, `gate_diff.py`, `ci_steps.txt`, `logs/`).
- **Code read:** SPR-014's `engine.rs`, `ledger.rs` and `system.rs` from `origin/sprint/SPR-014-lifecycle-queues`; SPR-026's `rh-protocol` from its branch.

## 2. The re-check of revision 1 (`f6bf6d2`)

#### 1. BLUF
**Ready after fixes.** 0 BLK, 0 MAJ, 8 MIN, 12 NIT.

- All 22 rulings are applied or applied in substance.
- Two X-ruling fixes are not right yet: X5's divergence clause (V1) and X4's mutant (V2).
- Several MIN items are stale or loose text from revision 0.

**Gates**
- `git merge-tree --write-tree origin/main f6bf6d2` is clean (tree `d03aefb`). It is also clean against live main `9bf8454` (PR #80, control_v1).
- `gen_manifest.py --check` passes (204 documents).
- `test_check_all_inventory.py`: 13 passed.
- No collision for A-61 or D57 on live main, nor on `sprint/SPR-014`, `SPR-020` or `SPR-026`.

#### 2. Findings

| ID | Sev | Where | Problem and evidence | Fix |
|---|---|---|---|---|
| V1 | MIN | SPR-018:79, :184 | T-05 (a) lists `events.meta` among roots that must be byte-identical ("anything else differing is a defect"). SPR-020's digest v2 hashes `has_place, place` (`envelope_v0.md:132–133`; "places landing in SPR-018 need no v3"). With T-03's `place` set, the root `rh-core/events.meta` differs from the first event on every rung, even `solo`'s one DEATH. The same plan's channel-A clause (:185) already expects `place_id` to differ. A correct build fails the criterion. | Name `events.meta` as differing (when SPR-020 is on main) in (a), or drop it from the identical list. |
| V2 | MIN | SPR-018:165 (also :78, :158, :215) | **X4 is not fixed.** The text says `mutant-founder-residence-last-member` "fails the hamlet residence line (Job's household keyed by its last servant … moves the 42)". I ran the reference: keyed by k = 45, Job's household still lands in settlement 1, so hamlet's line is identical at R 256 and 128. Settlement 1 holds 51% of the land. The mutant does differ on 5 of the 6 SYNTHETIC cases (all but `river_delta_city`). "Head is `cast[1]`" is also wrong: SYNTHETIC's non-first head is `cast[2]` (`selftest_worldgen_lite.py:712`). | State that the mutant is killed by the synthetic lines only. Or key it by a member that moves hamlet (k = 7 or 8 gives settlement 2). Write `cast[2]`. |
| V3 | MIN | SPR-018:136, :156; SPR-019:110, :174 | The plans say invariant `test` fields "name where each runs" or "say genesis and restore". `lint_odd.py:1203–1204` requires a docs/09 V-id there; the existing INV-PS/BF fields are V-ids. A literal edit fails the lint. | Keep V-ids in `test`; put the cadence in the `statement`. |
| V4 | MIN | SPR-019:134, :68 | Stage A's T-03 includes genesis refusing "a dynasty beyond `max_rows` (B7)". B7 counts rows the back-fill writes (`population_synthesis.toml` B7), so it is stage B. Stage A also has no rule for a manifest with dynasties: it would silently generate none. Not caught in revision 0. | Move the B7 refusal to T-05. Have stage A refuse a dynastic manifest by name until stage B. |
| V5 | MIN | `decisions_pending.md:16` | D57 items 4 and 5 give no confidence, and item 3 uses "8/10" among percentages. CLAUDE.md requires a recommendation with a confidence for each. | Add them (for example 70% to item 4, per D54 item 6; item 5 as a stated default). |
| V6 | MIN | `decisions_pending.md:13` vs :16, SPR-018:198 | D57 item 4 and both plans say D54 item 6 carries current numbers and recommends the third job (70%). D54 on this head still has core 7:58 / golden 10:21 and recommends "keep two jobs … decide the third only if a part passes 12 minutes". | The Architect updates D54 item 6 before the director reads D57. |
| V7 | MIN | PRJ-002:161 | Z2 is incomplete. The population-synthesis row still says "a new M7 0.4.0 draft". The M7 row (:163) was fixed. | Say "0.5.0 draft, unpromoted". |
| V8 | MIN | SPR-019:64 | Stale: "INV-BF-05 at every Bookkeeping". It contradicts X1's rule and T-07 (:183, "at every year end … never per-instant"). | Change to "at every year end (observer)". |

**NITs**
- **N1.** SPR-018:122: `check_all.sh:399–404` is now `:425–431` (`worldgen_lite_golden_control`, used at :1322). :48: the `instructions_v1.txt` slot is at :11, not :12. :49 and :64: `engine.rs:1021` is the doc line; the fn is at :1022. SPR-019:176: `next/README.md:70–74` should be `:74–77`.
- **N2.** SPR-018:63 and :156: NIT-A and NIT-B are already fixed on main. S3:221 no longer mentions `birth_tick`. M1:329 already says K = 1 and 4 only. Only NIT-C is real.
- **N3.** SPR-018:39: the fallback says "SPR-020's T-03 … moves M2's emit sites". It is T-02 (`SPR-020:137`), and SPR-020's plan has no scope to fill `place`.
- **N4.** SPR-018:69: BIRTH, MARRIAGE and EXTINCTION `place` choices are marked "Ruled (X6)". `envelope_v0.md:55` names only DEATH.
- **N5.** SPR-018:140 and :184: "the six M1 roots" are 18 per-column ladder roots (the baselines list `person.id`-style per-column roots). `events.meta` is named `rh-core/events.meta`.
- **N6.** SPR-018:180, :160: R4's selector pass is in the specs table (:64) and the Tester row, but not in T-05's per-instant record. T-03's "rows visited = births" counter conflicts with the selector, which visits all rows.
- **N7.** SPR-018:63: X3's `neutral` has no value. Say `neutral = 1`.
- **N8.** SPR-018:95: the Architect re-point list omits `docs/11_roadmap.md:190`, SPR-027 (:18, :44, :56, :177, :184), `05.10:106` and D54 item 4.
- **N9.** SPR-019:121 says the orphan count is "only when nonzero". `population_synthesis.toml:92` (P10) says "reported by the run header". T-01's fold edits do not list the wording change.
- **N10.** BCK-002 goes A-59 → A-61. SPR-018 (:36, :64, :74, :95, :234) and PRJ-002:39 cite A-60 as filed. Note it until the Architect files it. :80 uses "A-60" for a CI move, a different sense.
- **N11.** The X/Y/Z IDs are defined nowhere in the repo. `REV-WAVE6` is not committed and is not in `related:`. PRJ-002's graph lacks an `S016 -.merge order.-> S018` edge and an SPR-019b node.
- **N12.** Plans cite golden 12:44 (the review measured 12:46). `check_all.sh:1264` becomes `:1274` after PR #80.

#### 3. Per-ruling verdicts
- **X1: applied.**
  - The scheme matches the code. `engine.rs:914–916` calls `check_invariants` at every instant's Bookkeeping, unconditionally. `:1051` is the loop over `registered.entries` calling `invariants(&WorldTables)`, which has no time. `:847–852` clones the lagged columns.
  - The four clones check out (S3's three M1 reads plus M2's `person.residence`).
  - `check_all.sh:1264` is the V-DATA-04 1,000-year release gate, in part core (`part core` at :1022, `part golden` at :1390).
  - M1 and S3 have no per-instant O(cells) or O(rows) work.
  - The one O(persons) pass, R4's selector (`S3.migration.odd.toml:154`, `scale_behaviour`), is stated plainly (SPR-018:64). Its measurement under A-60 is in the specs table, :95 and the Tester row (N6).
- **X2: applied.**
  - Kind 7, the regional digest and `canonical/v1` are out of scope in both plans, in PRJ-002 and in D57. No stale in-scope reference remains.
  - A-61 is well formed (4 columns, 1.19.0 → 1.20.0). It records X8's start state and X9's k order.
  - A-61 does not collide with anything.
- **X3: applied.** Only the neutral's value is missing (N7).
- **X4: partly applied.** See V2.
- **X5: partly applied.** Post-month clauses are fixed in both plans (SPR-019:69, :146 are right). V1 remains on the pre-month clause.
- **X6: applied.** `envelope_v0.md:39` and `:55` are cited, with no M2 edit and a precondition plus fallback (N3, N4).
- **X7: applied.** Allocation budgets are expected to move; `alloc-record` is in the `[C3] [perf-baseline]` commit; the cost is noted under A-60.
- **X8, X9: moot, and recorded in A-61.**
- **X10: applied.** The task inventory sums to 4.5 days (T-01 1.5, T-02 0.75, T-03 1.0, T-04 0.5, T-05 0.75). The hard checkpoint after T-02 is present.
- **Y1: applied.** SPR-019 never promotes M7. `check_all.sh:330–389` plants its seventeen controls into `next/M7`, which stays in `next/`.
- **Y2: applied.**
  - S3 gets `"dynasties"` at its freeze (SPR-018 T-03).
  - M2 gets it through an `[[amends]]` on `initialization.from_manifest`.
  - I simulated this on a copy: the amendment lints, and the only error is the lock until it is regenerated. `from_manifest` is not in the skeleton.
- **Y3: applied and consistent.**
  - SPR-015 promotes M2 0.3.0 and SPR-016 bumps it to 0.3.1 directly in the frozen file.
  - SPR-019 stage A folds 0.3.2 through `next/` and promotes it in the same PR. `next/` then holds only M7.
  - The README's checklist items apply, and SPR-015's controls no longer depend on a real `next/M2`.
  - Remaining gap: V3, N9.
- **Y4: applied.**
  - The D56 table matches `M7_membership.md:29–31` and `:68`, and D56's text in `decisions_pending.md`.
  - Only (e) reaches genesis (B6; no vocation is written at genesis).
  - Stage B's gate is "(e) settled". The `member_of` hook is gone.
- **Y5: applied and verified.**
  - I ran the reference with 2, 4, 8 and 32 IPF sweeps. 32 sweeps equals the committed golden.
  - Only 2 sweeps differs, and only in `case manor 1`: lines 77, 79 and 81–84 (`ipf` rows 2, 4, 6–9) plus lines 103–104 (the two shas).
  - 4 and 8 sweeps change nothing, and `case manor 2` is unchanged.
- **Y6–Y9: applied.**
  - SPR-019's size sums to 6.5 days (A 3.25, B 3.25).
  - `--stage a` is dropped.
  - The kind-7 criteria are gone.
  - Y7 is applied except for N9.
- **Z1: applied.**
  - Both plans restate added time per part with ×1.5 budgets and nightly candidates, and no gate moves.
  - The arithmetic adds up: SPR-018 +50 s / +40 s → +75 s / +60 s; SPR-019 +15 s / +55 s, as D57 states them.
  - V6 stands.
- **Z2: partly applied.** PRJ-002 and D57 are restated, except V7.
- **D57:**
  - It sits after D56 in the Question | Recommendation | Default form.
  - It holds the five restated items.
  - Items 1 to 3 have recommendations with confidence. Items 4 and 5 have no confidence (V5).

#### 4. What I ran
All scratch is in `the Architect's scratchpad: wave6_review/work2/`.
- Gates and merge: `gen_manifest.py --check`; the inventory pytest; `git merge-tree` against `origin/main` and live main. I fetched live main and the three sprint branches to temporary refs (deleted afterwards) to check numbering.
- Y2 amendment: a copy of `specs/` with the `lint_odd.py --next` amendment, run to check that it lints.
- Y5: `ipf_driver.py`, the reference rendered at 32, 8, 4 and 2 sweeps, diffed against the golden.
- X4: `mut_last.py` and `mut_keys.py`, placements keyed by the last member and by each member of Job's household. This produced V2.
- T-01's `mutant-blob-k-shift`: `mut_k.py`. Elevation changes on all 8 non-flat cases and not on `solo` or `household`, as the plan says.
- Citations: engine.rs, check_all.sh, lint_odd.py, envelope_v0.md, M7_membership.md, S3 and M1 specs, the next/ drafts, and `perf_gate.py`.
- No Rust built. No commit or push.
