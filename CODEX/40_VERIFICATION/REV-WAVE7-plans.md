---
id: REV-WAVE7
title: "Independent review of the wave-7 sprint plans (SPR-021a and SPR-021b, SPR-022a and SPR-022b) and D58"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [SPR-021, SPR-022, PRJ-002, SPR-015, SPR-019, SPR-023, SPR-024, SPR-038, REV-MINDS, REV-SPR-019, BCK-002]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing commit (2026-10-18). The plans go to the director as D58.**
>
> | Plan | Revision | Size | Class |
> |---|---|---|---|
> | SPR-021a: the M7 0.6.0 promotion, the instant log's events (X1 (c)), the core genealogy index (X2), the id keys, `rh-contracts::m7` | 0.3.0 (`0983ea9`) | about 4 agent-days | C2 (X6) |
> | SPR-021b: succession and the interregnum | the same | about 4 agent-days | C2 |
> | SPR-022a: ruler-exit hazards, claims, regencies, vocations, dormancy with world-start `Promote` (X4) | 0.3.0 | about 4.25 agent-days, with a checkpoint after T-02 | C2 |
> | SPR-022b: `twin_vales` named set and V-INV-12, after SPR-019b | the same | about 1.5 agent-days | C2 |
>
> The spec PRs T-00a (`next/M7` 0.6.0 with EVO-002 and its executable model; about 3–4 days) and T-00b (SPR-022's M7 amendments; about 1.5–2 days) are counted in PRJ-002 §6, outside these sizes.
>
> - **Round 1** (`d37fc5b`): 0 BLK, 8 MAJ, 21 MIN, 4 NIT (§1). Of the drafter's 26 findings, 22 are real and 4 partly real. The majors: three plans each building a "this instant" surface (K27); two designs for world-start naming (K28); the CI base (K24); two holes in the drafter's own fixes (K2, K3); optimistic sizes and the T-00 route (K4, K5). Rulings: §0, with the cross-plan rulings X1–X13 recorded in REV-MINDS §0a. Revision 1 is `b2c6fb1`.
> - **Round 2** (`b2c6fb1`, joint with the minds plans): AFTER SMALL FIXES, 0 BLK, 3 MAJ (R1 and R2 applied X13 late; R3, the K3 defaults would have turned T-00a red), 4 MIN, 1 NIT (§2). Rulings: §0b. Revision 2 is `0983ea9`, merged with `main` `7031f6a` (`41c4ceb`) and with the minds branch (`6bb5df5`, the PRJ-002 and `decisions_pending` union).
> - **Architect's verification of revision 2** (§0c): every wave-7 row of §0b is present.
>   - R1: `TitleId`/`HouseId` are SPR-019 T-01's; 021a adds only `OfficeId`, `ClaimId`, the `EntityKey` impls and the Title tag, and depends on SPR-019 stage A.
>   - R2: the T-00a fold and the "019b after 021a" edge are gone; REV-SPR-019 R13 is stated both ways.
>   - R3: M7's 16 defaults go through `next/registry_delta.toml`, with `lint_odd.py`'s self-check fix sized inside T-00a.
>   - R5, R6: 024b waits for SPR-022a and SPR-019b; PRJ-002 §6 has T-00a and T-00b rows and their order.
>   - R9, R11, R12, R14: `exited()` ascending, cited; the house → title-holders index is M7's; 021a T-03 replaces stage A's `DynastyParams`; SPR-038b owns the one `DropReason` change.
>   - R15, R16: one PRJ-002 graph with no dangling node; X13h ruled and the vocation mutant's kill condition stated.
>   - Typed golden numbers are replaced by citations.
>   - Two closing rulings (§0c): F-28 and the PRJ-002 lane change.

## 0. Architect rulings, round 1 (2026-10-18), on `d37fc5b`

The independent review found **0 BLK, 8 MAJ, 21 MIN and 4 NIT**. Of the drafter's 26 findings, 22 are real and 4 partly real. The cross-plan rulings X1–X12 (`scratchpad/p1a_shared_rulings.md`) bind this revision. **Every finding is accepted unless a row says otherwise.**

| # | Ruling | Conf. |
|---|---|---|
| K1 | **Per X1.** EVO-002 specifies the instant log; SPR-021 T-02 builds part (c), the merged events by (kind, target) with their ids. Part (ii) is dropped. T-00a carries an executable Python model of the hook timing | 65% |
| K2 | **Emit INTERREGNUM_CRISIS for a vassal that joins its liege's interregnum** (payload: its title; `triggered` → the DEATH), lifting "sovereign only" for that case. Add a fixture and a mutant. V-SAGA-19 is unchanged | 70% |
| K3 | Accepted. T-00a adds `default` to the 16 `[[params]]`. The Registry defaults go in T-01's promotion commit, with no `registry_version` bump before a `[C3]` commit (there is none until 024b, X6). The count parameters get integer defaults | 85% |
| K4, K17, K18 | **Per X8:** 021a (T-01–T-03) and 021b (T-04–T-06), about 8 days; 022a (T-01–T-04) and 022b (T-05–T-06, after SPR-019b), 5.5–6 days | 65% |
| K5 | **Per X7:** T-00a (C2: M7 0.6.0 shared amendments + EVO-002 + the model) and T-00b (the amendments only SPR-022 needs), counted, and each reviewed independently. The Architect commissions T-00a now | 75% |
| K6 | Accepted. `on_death` runs in two phases: void the dead claims, vacate offices, close regencies and count houses; then arbitrate by tier. Same-instant fixture, plus `mutant-dead-claimant-candidate` | 85% |
| K7 | Keep the annals fixtures. CLAIM_* payloads are `[title id]`. Add `CLAIM_CREATED`'s `triggered` edge and `set_cause` for a passed-over heir to T-04 and to the checker | 70% |
| K8–K12, K14, K15 | Accepted as the review fixes them. For K12, the dead holder still holds for the Year pass (one rule, with a fixture) | 85% |
| K13, K32 | **Per X6.** M7 and T1 join `BUILT` together in SPR-024b, so SPR-021 and SPR-022 are C2. Restate the classes, the baselines (none move in wave 7) and D58 item 2 | 60% |
| K16 | **Per X12.** A `docs/13` §4.2 rule, a director item in D58; SPR-022's rates are reported at merge | 70% |
| K19 | **The vocation is drawn at the majority date**, from a timer `on_birth` schedules, before M2's marriage age can matter. The timer is re-sampled under F-9's rule | 65% |
| K20 | Accepted. An in-place 0.6.x erratum, with the Architect's approval | 85% |
| K21 | Accepted. SPR-019 T-07's manor band assertion applies only when the registered systems equal those the band was projected for (M1, M2, S3). The band is re-derived once, after SPR-022, in 022b. The drafter adds one line to SPR-019's T-07 bullet | 75% |
| K22 | Accepted. +90–150 s on golden or stream, +20–30 s on core, by role (X10) | 80% |
| K24–K26 | **Per X10.** D58 adds items for the shared owners (X1, X2), T-00's class, SPR-038's order, CI and the `BUILT` move. Each plan gets a one-page brief at its top | 80% |
| K29–K31, K33 | Per X1 and X2. K31: the house-to-title-holder lookup is the genealogy owner's (X2), exposed by M7's contract | 70% |

**D58, restated:**
1. Sizes and splits per X8.
2. Classes: wave 7 is C2 throughout; the `docs/13` class rule (X12); `BUILT` moves in SPR-024b (X6).
3. Spec prep first: T-00a and T-00b (X7); A-59 through the instant log (X1).
4. Registry defaults (K3).
5. SPR-021 does not depend on SPR-019; 022b waits for SPR-019b.
6. The requirement texts: SIM-9, D35's "claims", `docs/11` P1.7, V-INV-03.
7. CI per X10.
8. The shared owners X1 to X4, and the merge graph X9.

## 0a. Cross-plan rulings X1–X13

Recorded in full in REV-MINDS §0a (`CODEX/40_VERIFICATION/REV-MINDS-plans.md`). They bind wave 7, the minds plans, SPR-038 and SPR-015.

## 0b. Architect rulings, round 2 (2026-10-18), on `b2c6fb1` (joint with the minds plans)

The rows addressed to wave 7 are R1–R3, R5, R6, R9, R11, R12 and R14–R16; R4, R7, R8, R10, R13 and R17 are the minds plans' (REV-MINDS).

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

## 0c. Architect's verification of revision 2 and the closing rulings (2026-10-18)

Every wave-7 row of §0b is present in `0983ea9` (BLUF). `gen_manifest --check` builds cleanly on the closing commit.

| # | Ruling | Conf. |
|---|---|---|
| F-28 | **Accepted, and applied by the Architect in SPR-015 (0.2.3), as with X13g.** SPR-015 T-01 merges only M2's section of `next/registry_delta.toml` and the verbs into the Registry, and deletes the file only when nothing else remains in it. If T-00a merges first, M7's defaults stay in the delta until SPR-021a T-01 moves them: the frozen M7 0.3.4 has no `default` lines, so the frozen lint's two-way check (`lint_odd.py:1171–1179`) would refuse them in the Registry. T1's defaults (the T1-I/S spec PR) stay until SPR-024a T-01 | 85% |
| X13g | **Already met by A-62** (PR #94): SPR-015's T-03 text (`SPR-015_BirthsF1.md:154` on that branch) states the binary search on the lagged `house_member.person`, which M7 0.6.0 keeps in ascending order. No second sentence is added here | 90% |
| L1 | **The PRJ-002 §4 lane change is accepted:** SPR-019 stage A sits in wave 6, lane 3, after SPR-018, and SPR-021a waits for it (X13a), so wave 6 no longer shows SPR-021 merging beside SPR-018 | 85% |
| U1 | **The union with the minds branch** (`6bb5df5`): PRJ-002 0.5.12 keeps both plan sets' rows and one graph (spec PRs as stadium nodes; wave 7's edge labels; the minds plans' S024a/S024b split); `decisions_pending` keeps D58, D59 and D60 once each under one header | 90% |

## 1. Round-1 report (`d37fc5b`)

Reviewed on a different model from the drafter's, against `main` `3e2e6ef`. Cross-checked against:
- `arch/minds-plans` `3beca61`;
- `arch/spr038-plan` `fb5d68b`;
- SPR-014 `921d372`;
- `SPR-015_BirthsF1.md`.

The reviewer could not write files, so the Architect recorded this report from the text it returned. Its scratch work is in `wave7_review/work/`.

### 1. BLUF

| Plan | Verdict | BLK | MAJ | MIN | NIT |
|---|---|---|---|---|---|
| SPR-021 | Ready after fixes | 0 | 5 | 9 | 1 |
| SPR-022 | Ready after fixes | 0 | 0 | 7 | 1 |
| Across wave 7 (PRJ-002, D58, CI) | Fix before approval | 0 | 1 | 1 | 1 |
| Across wave 7 and the minds plans, SPR-038 and SPR-015 | Fix before D58 and D59 | 0 | 2 | 4 | 1 |

**Totals: 0 BLK, 8 MAJ, 21 MIN, 4 NIT.** Of the drafter's 26 findings, 22 are real and 4 partly real; none is wrong. The majors are things the drafter did not look at:
- K27: three plans each build a "this instant's rows and events" surface, and none owns it.
- K28: world-start naming has two designs, because two specs disagree.
- K24: the CI base mixes different PRs, and A-63 as filed does not touch part core.
- K2 and K3: two holes in the drafter's own fixes.
- K4 and K5: the sizes and the T-00 route are optimistic.

The promotion was simulated on a copy and works as the plan says (§8).

### 2. SPR-021

| ID | Sev | Problem | Evidence | Fix |
|---|---|---|---|---|
| K1 | MAJ | **EVO-002 is prose only, and its mechanism is the weakest part of the plan.**<br>(a) It adds per-system hook state.<br>(b) The drafter rates it 6.5/10, and a flip re-plans T-02 and T-04.<br>(c) Part (ii), lagged foreign inputs, has no wave-7 consumer.<br>(d) Alternative (c) was dismissed as "a second read surface", yet SPR-038's `inserted()` and SPR-023's `exits()`/`entries()` add exactly that | `control_v1` shipped with a 14-scenario reference and still took 4 rounds. ADR-0024:25: events are plane A | Specify **one engine-owned per-instant record**: read-only to steps, cleared at the instant's end, and outside every hash. It holds the rows inserted (SPR-038), the rows exited (SPR-023), and events by kind with their ids. Alternatively, resolve the ids at the merge with an `EventRef` keyed by (kind, target). Drop (ii) until a consumer exists. Add a small executable model of the hook timing to T-00 |
| K2 | MAJ | **A correct build fails V-SAGA-19 under F-11(b).** A vassal with no candidate, whose liege is in interregnum, "joins that interregnum" with neither SUCCEED nor INTERREGNUM_CRISIS | `next/M7:733`; INV-M7-02 `:771`; `docs/09:425` | Emit INTERREGNUM_CRISIS for the joined vassal (`triggered` → DEATH), or amend INV-M7-02 and V-SAGA-19 through D58. Add the fixture and a mutant |
| K3 | MAJ | **The Registry defaults are mis-sequenced and half-specified.** The frozen lint requires each Registry default mirrored as `default` in `[[params]]`, both ways. T-00 lists no `default` amendments, and T-03 and T-06 add the Registry defaults after the promotion | `lint_odd.py:1171–1179`; `registry_v0.toml:20–30`; `manor.yaml:42–44` | Add `default` to the 16 `[[params]]` in T-00. Add the Registry defaults in T-01's promotion commit, with no `registry_version` bump before the `[C3]` commit. The count parameters get integer defaults |
| K4 | MAJ | **About 4.75–5 agent-days is not credible** (W14) | T-02, T-03, T-04 and T-05 are each overpacked; SPR-019 stage B alone is 3.25 days | About 7.5–9 days. **Split into 021a (T-01–T-03) and 021b (T-04–T-06)**, with D58 item 1 at 65% |
| K5 | MAJ | **T-00 is sprint-sized critical-path work, counted nowhere, misclassed as C1, and reviewed with no executable model.** It holds about 30–35 `[[amends]]` with byte-exact `frozen` texts and a lock, EVO-002 in `05.1` §2, and a lint schema with fixtures | PR #80 (a contract) was C2. EVO-002 changes the engine contract | Count T-00 in the size (M7 F1 is then about 12–13 days). Class it C2. Add the executable model. Split the SPR-022-only amendments into a second prep PR, reviewed in parallel |
| K6 | MIN | The order inside `on_death` is unspecified. A claimant or marshal who died in the same Vital slot can be seated or can score | `next/M7:729`, `:733` | Two phases: first void the dead claims, vacate offices and close regencies; then arbitrate by tier. Add a same-instant fixture and `mutant-dead-claimant-candidate` |
| K7 | MIN | The F-8 payload justification is false: the annals fixtures carry `(LORDSHIP,)`, not `[title, claim]`. `CLAIM_CREATED`'s `triggered` edge and `set_cause` are missing for a passed-over heir | `gen_fixtures.py:420–432`; `annals_v0.md:172` | Use `[title id]`, or regenerate the fixtures through their generator. Add the edge and `set_cause` |
| K8 | MIN | The checker "reads the genesis keyframe", but no Python keyframe reader exists | | Read the scripted fixture TOML and an `rh-cli` extract, and budget the work |
| K9 | MIN | `mutant-scan-ledger-daily` is observable only at 10,000 persons. The PR tier's three-feature mutant build names neither the features nor their kill checks | | Name them; give the probe a tight constant at 1,500 persons |
| K10 | MIN | EVO-002 (ii) is built for M2, yet nobody edits M2. M2's elite lookup is O(women × members) against unsorted `house_member.person` | `next/M2:586`; SPR-014 `world.rs:234` | State M2's lookup (a sorted insert, or an M2 index) and put it in a task |
| K11 | MIN | Missing acceptance lines:<br>(a) no desync check for `exit_timer`;<br>(b) no journal or keyframe decode test for `EntityKind::Title`;<br>(c) `sealed` also lists `WorldKey`;<br>(d) `DropReason` cannot express "ruler alive but lost the title" | `system.rs:437–449` | Add the four lines |
| K12 | MIN | F-20's fix contradicts M7's `on_birth` rule that "a holder who dies this instant still holds here" | `next/M7:725` | One rule: the holder still holds for the Year pass. Add a fixture |
| K13 | MIN | Header-only C3 is optional. M7 and T1 could join `BUILT` together in SPR-024 (or in SPR-022 T-06), which makes SPR-021 C2 | SPR-023 is C2 with its own built list | Offer the alternative in D58 item 2 (55%) |
| K14 | MIN | The plan is silent on SPR-038 | `SPR-038:116–118`, `:338` | State the order. Add M7's five bindings to the A-60 record. Gate the hook by the write set if SPR-038 lands first |
| K15 | NIT | `next/README.md:154` is stale. The M2 controls also `cp "$m7"` (`check_all.sh:332–355`) | | Fix in T-01 |

### 3. SPR-022

| ID | Sev | Problem | Fix |
|---|---|---|---|
| K16 | MIN | "C2 with C3's evidence" holds only as a written rule. CI cannot detect the class, and the director's veto on rates is dropped | Amend `docs/13` §4.2: "a model change to a system registered in no golden world is C2; the PR proves it by its `systems` lines and `BUILT`". Announce the reported rates at merge |
| K17 | MIN | The PR cannot merge until T-05 and T-06, which wait for SPR-019b and A-62 | Split into **022a** (T-01–T-04, C2) after SPR-021, and **022b** (T-05–T-06) after SPR-019b |
| K18 | MIN | About 4.25 days is low | About 5.5–6 days |
| K19 | MIN | The vocation draw can come after marriage. M2 draws wives from age 15, and `majority_age` is 16 | Draw at the majority date from a timer scheduled by `on_birth`, or accept the "married nun" case explicitly |
| K20 | MIN | The errata route differs from SPR-016's and SPR-024's | Allow an in-place 0.6.x erratum bump with the Architect's approval |
| K21 | MIN | SPR-019's manor band assertion turns red on `main` once M7 is built: the elite read goes live, and holy orders draws from the pool | Key the assertion on the registered systems, or re-derive the band once, after SPR-022, in that PR |
| K22 | MIN | +45 s on part golden is low | +90–150 s golden, +20–30 s core |
| K23 | NIT | `murder_due` drops as `Dead` | Fold into K11 |

### 4. Wave 7 against the minds plans, SPR-038 and SPR-015

| Mechanism | Who builds it | Same design? | Order stated? |
|---|---|---|---|
| The instant's ledger changes | SPR-023 `exits()`/`entries()`; SPR-038 `inserted()`; SPR-021 (index inputs and observed events) | **No: three APIs over overlapping data** | No: it goes in a circle |
| Foreign-input derived index | SPR-021 only | n/a | The M2 consumer is unscheduled (K10) |
| Genealogy index | SPR-024 T-01 "unless SPR-021 landed it", while SPR-021 builds M7-private structures | **No** | Hollow |
| A-59's EventId | SPR-021, consumed by SPR-025 | Partly (K29) | Yes |
| World-start naming | SPR-022 T-05 (M7 Promotes at tick 0) against SPR-023 row 6 (genesis writes `person.named`) | **No** | No |
| `Promote`/`Demote` | M7 sends, T1 applies | Yes | Yes |
| `rh-contracts::m7` | SPR-021, or SPR-023 if it lands first | Yes | Yes |
| `TitleId`/`HouseId` | SPR-019 T-01 or SPR-021, whichever is first | Yes | Yes |

| ID | Sev | Problem | Fix |
|---|---|---|---|
| K27 | MAJ | The per-instant buffers overlap, and the genealogy index has no named owner. Each plan defers to another, so the second sprint cannot "reuse" anything | **Name the owners in D58:** one engine record (K1), built by SPR-038 or SPR-021 T-02 and read by SPR-023/024; and one core genealogy index (SPR-024 T-01, fed by the core's life-course append) that M7 reads instead of a private copy. Delete the "whichever first" clauses |
| K28 | MAJ | World-start naming has two designs, because the specs disagree: M7 0.5.0 names by `Promote` at the first dormancy pass, while T1 0.3.3 says world creation names | One route, M7's. SPR-023 row 6 becomes "T1 creates a mind for each person Promoted at t = 0". SPR-024 depends on SPR-022 T-05, or on a stub. Reconcile both specs' `[initialization]` |
| K29 | MIN | SPR-025's "preferred form" is SPR-021's rejected alternative (a). T1 also needs `[[observes]] DEATH` | Drop the preferred form, and add `[[observes]]` to the T1 draft |
| K30 | MIN | `DropReason::Deferred` is missing | Add it in SPR-023 T-02 |
| K31 | MIN | The court of an actor with no seat needs a lookup from house to title holder over M7's columns, which nothing exposes | State it in both plans, or give the lookup to the genealogy owner |
| K32 | MIN | Four header and rebaseline C3s (SPR-021, SPR-024, SPR-018, SPR-019 A) | Fold into K13: one `BUILT` move for M7 and T1 together |
| K33 | NIT | SPR-038:383 says "M7 owns `person.house`", which is stale | Fix in SPR-038 |

### 5. CI

| ID | Sev | Problem | Fix |
|---|---|---|---|
| K24 | MAJ | **The arithmetic is right, but the base is wrong.** Single runs on different PRs; the same SHA measured golden at 8:13 and 11:11. **A-63 as filed splits golden only, so core stays at about 19–20 minutes** | D58 item 7 must say what happens to core, and SPR-038 is a stated predecessor of wave 7. Budget by gate sums on one runner |
| K25 | MIN | D58 has no item for the shared owners, T-00's class, SPR-038, A-63's scope or the `BUILT` alternative | Add them |
| K26 | NIT | Plans of 80 KB and 45 KB | A one-page brief per plan |

The projection, by state:

| State | Core | Golden |
|---|---|---|
| `main` (docs-only runs) | 11:37–11:38 | 6:28 |
| SPR-014's head | 14:48 / 14:43 | 11:11 / 8:13 |
| SPR-026's head | 11:45 | 12:46 |
| Wave 4 combined | 14:48 | 13:34 |
| + wave 5 | 17:18 | 14:52 |
| + wave 6 | **18:23** | **16:27** |
| + wave 7, as planned | 18:53 | 18:07 |
| + wave 7, at the reviewer's estimate | 19:30–20:00 | 19:00–20:30 |
| + the minds plans | about 22 | about 20–21 |

### 6. The drafter's 26 findings

| # | Verdict | Note |
|---|---|---|
| F-1 | Real | Sound and hash-neutral under ADR-0024 and ADR-0025, but not the best fit (K1) |
| F-2 | Partly real | (ii) has no consumer |
| F-3 | Real | Sound; add the desync probe (K11) |
| F-4 | Real | Sound; add the journal decode (K11) |
| F-5 | Real | Sound |
| F-6 | Real | Simulated; sound |
| F-7 | Real | Sound |
| F-8 | Partly real | The fixture claim is false (K7) |
| F-9 | Real | Sound |
| F-10 | Real | Sound |
| F-11 | Real | (b) breaks V-SAGA-19 (K2); (a) must carry the rule in M7's text |
| F-12 | Real | Sound |
| F-13 | Real | Sound; plus the `$m7` controls |
| F-14 | Real | Incomplete (K3) |
| F-15 | Real | Also amend `docs/11` P1.7 |
| F-16 | Real | Sound |
| F-17 | Real | Fixed in the PR |
| F-18 | Real gap | Sound if written as a rule (K16) |
| F-19 | Real | Leaves a window (K19) |
| F-20 | Real | Inconsistent with `on_birth` (K12) |
| F-21 | Partly real | Belongs to SPR-029 |
| F-22 | Real | Deferring is sound; add the §3 note |
| F-23 | Partly real | K10 |
| F-24 | Real | Sound |
| F-25 | Real | Sound |
| F-26 | Real premise | Incomplete (K21) |

### 7. D58

| Item | Verdict |
|---|---|
| 1. Sizes | Object. SPR-021 about 8 days, plus 3–4 for T-00; SPR-022 5.5–6. Split 021a/021b and 022a/022b (65%) |
| 2. Classes | SPR-022 C2: approve, written as a rule (K16). SPR-021: offer the `BUILT` deferral (55%) |
| 3. Spec prep first; A-59 | Prep first: agree (75%), with T-00 as C2, counted, and with a model. A-59: the plan's mechanism 45%, the engine record or merge-time resolution 55% |
| 4. Registry defaults | Approve with K3's fix (65%) |
| 5. Independence from SPR-019 | Approve for SPR-021 (70%); split SPR-022 |
| 6. Requirement texts | Approve (70%), and add `docs/11` P1.7 and V-INV-03 |
| 7. CI | Object: insufficient (K24) |
| Missing | Shared owners (K27), T-00's class, SPR-038, world-start naming (K28) |

### 8. Checked and sound

- **The promotion, simulated on a copy:**
  - the frozen lint passes;
  - the skeleton diff is only the version line and the parameter types;
  - the lock tool removes the M7 lock section;
  - `gen_manifest` passes;
  - today's lint silently accepts an unknown `[[observes]]` table, so T-00 needs its schema.
- **No baseline root can move:** the rung templates exclude M7.
- **The numbers:**
  - the named sets: 31/33, 33/35 and 22/26;
  - the murder and deposition counts;
  - the contest-margin fixtures;
  - the phase order.
- **Cross-plan:** the `rh-contracts::m7` coverage, the id-layout order, `Promote`/`Demote`, and kind 6.
- **Lessons:** W1, Y1, X1, V3 and W42 are applied, except where noted above.

### 9. What the reviewer ran

- The promotion simulation in `work/sim1/`.
- Step times and per-gate logs from GitHub Actions for nine runs.
- Full reads of SPR-021/022, SPR-023/024, the `next/M7` draft, `lint_odd.py`, and the SPR-014 sources it cites.

## 2. Round-2 report (`b2c6fb1`, joint with the minds plans' `f791a38`)

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
