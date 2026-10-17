---
id: REV-SPR-018
title: "Independent review of the worldgen-lite, M1.regions F0 and S3.migration F0 specs (PR #52)"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-018, SPR-015, SPR-013, SPR-019, REV-SPR-015, EVO-001]
created: 2026-10-17
updated: 2026-10-17
version: 1.0.0
---

> **BLUF: CLOSED, approved.** PR #52 took three rounds:
> - **Round 1** (`45e74d7`): no blocker and three majors: the `hamlet` golden case's raster size, the founder order, and S3 registered by no world.
> - **Round 2** (`d18973e`): no blocker and no major. The new W6, re-implemented from the text alone, reproduces all ten golden geography cases, and the residence lines match in ruling 10's order.
> - **Round 3** (`f27b33b`): verified by the Architect. Every golden case states its graph's components; the validator change, S3's Registry change and the kind 7 record are explicit SPR-018 deliverables in PRJ-002.
>
> Every review ran in a session on a different model from the drafter's.

# REV-SPR-018: independent review of worldgen-lite, M1 F0 and S3 F0

## 0. Architect rulings, round 1 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R1 | MAJ-1 | Accepted. Each golden case uses the parameters of the manifest it names. Add `hamlet` at R = 256, and keep R = 128 only as a separately named case | 9/10 |
| R2 | MAJ-2 | Accepted. The reference numbers founders in SPR-011 ruling 10's order: the cast first, then children and servants, then the rest. The golden residence lines are regenerated | 9/10 |
| R3 | MAJ-3, MIN-7 | Accepted, in two steps. This spec PR adds the S3 Registry entry with no version bump, as it does now. SPR-018's sprint PR, which builds S3, adds S3 to `default_systems` and bumps `registry_version`; S3's spec and SPR-018's plan row say so. Until then, a world naming S3 falls under ruling 3 (systems a manifest names that the build lacks) | 7/10 |
| R4 | MIN-1 | Accepted. Newborns are selected by `person.residence = 0`, the type's zero that SPR-014's row insert writes for other owners' columns, not by `birth_tick = now`. A founder born at tick 0 is then never re-placed | 8/10 |
| R5 | MIN-2 | Each connected land component gets at least one region seed while K allows, in a stated deterministic order (component by lowest cell index). Land left over when K is below the number of components belongs to no region, and the spec says so. A disconnected archipelago graph at F0 is accepted (the drafter's Q5) | 7/10 |
| R6 | MIN-3 | Accepted as documentation. Bench 4 measures the genesis smoothing pass, as defined. `docs/08` §1's per-step stencil budget stays unvalidated until the first per-step raster system (M3, P1b), and `docs/08` §9 says so | 8/10 |
| R7 | MIN-4 | Accepted. INV-S3-01 covers persons who entered at or after the epoch (founders and the born). Ancestors back-filled by SPR-019 have no residence row and are exempt | 8/10 |
| R8 | MIN-5 | P1a has no residence moves (PRJ-002 ruling M-6, approved at G0), so marriage must not create a household that spans two settlements. The husband draw's candidates are restricted to the bride's settlement. M2 reads `person.residence` as a `mode` read whose neutral (0) means "no restriction", so M2 still registers without S3 (REV-EVO-001 R4). The births drafter applies this in M2 F1; S3 states the read contract | 6/10 |
| R9 | MIN-6 | Accepted. R2 and R3 state the head's household in each case. The manifest validator refuses a person who heads two households, and a head whose `household` field names another household, by name | 8/10 |
| R10 | Nits | Accepted. The control expects `STALE:`. The companion claims no `LIFE_INTERVAL` reservation in `05.9` IF-0 that does not exist. Centroid sums are u64. Fix the reference's gaps and the cost figures | 9/10 |
| R11 | Drafter's Q1–Q5 | Q1: residence intervals become a non-event channel-A record. SPR-013's channel-A v0 reserves the record kind number now (one line, no format change); SPR-018's sprint defines its fields (C2). Q2: the `schema.sql` comment says settlement. Q3: V-DATA-04's P1a span is [entry_tick, exit_tick), excluding back-filled rows; an Architect clarification recorded in the `docs/09` P1a row. Q4: focus slugs are deferred, and not mapped through households. Q5: accepted (R5) | 7/10 |

## 1. The review (round 1, independent reviewer)

Reviewer: independent (Opus 5.5), 2026-10-17. Copy reviewed: `git archive 45e74d7`. The repository itself was not changed.

## BLUF

The algorithm is good. Every step of `[algorithm.worldgen_lite]` uses integers only, and every tie-break is total. An independent re-implementation written from the spec text reproduced every golden digest and count, including both bench 4 digests. S3's same-instant read is scheduled legally, and the implicit interval end fits ruling 5. **No BLK.**

There are three MAJ findings, all about fit:
1. The golden "hamlet" case is not hamlet. It runs at R = 128, but `hamlet.yaml` uses 256.
2. The reference numbers founders in a different order from ruling 10, so the hamlet residence line is wrong.
3. No template and no manifest names `S3.migration`. Under per-world registration, S3 would run on no rung, and V-DATA-04 would have no residence intervals at G1a.

Fix the three MAJ findings before merging. The MIN items can be fixed in the same pass.

## Findings

### MAJ-1: the golden "hamlet" case uses the wrong resolution (confidence 90%)

- **Where:** `specs/systems/M1.regions.odd.toml:246-249` (the facts block), `:326` (`[isolation].expect`); `specs/worldgen/selftest_worldgen_lite.py:548` (CASES).
- **Problem:** the case is labelled hamlet but runs `desert_fringe 42 128 4`. `hamlet.yaml:19` (and `uz_job.yaml:19`) set `raster_resolution: 256`. M1's own `scale_behaviour` (`:223`) says "65,536 for hamlet", which is 256².
- **Evidence:** I recomputed `desert_fringe 42 256 4` with my own implementation:

  | | Spec facts (R = 128) | Hamlet at its real R = 256 |
  |---|---|---|
  | Land cells | 13,453 | 53,859 |
  | Edges | 1-2 1-3 1-4 2-3 2-4 | 1-3 1-4 2-3 2-4 3-4 |
  | Route lengths | 43 to 81 | 78 to 148 |

  Region sizes at R = 256 are 27,691, 6,711, 5,996 and 13,461; seats are 18078, 55338, 54904 and 26409. The plan's acceptance criteria "may use" these facts, and they do not describe hamlet.
- **Fix:** add the case `desert_fringe 42 256 4`, call the 128 case what it is, and state the isolation claim at R = 256 as well. I checked that seeds 1–8 are connected at both 128 and 256.

### MAJ-2: the reference numbers founders differently from R0 and ruling 10 (confidence 90%)

- **Where:** `selftest_worldgen_lite.py:408-433` (`placement`).
- **Problem:** the reference numbers founders as household heads, then household members, then the loose founders. R0 and SPR-011 ruling 10 say: the cast in manifest order, then household children and servants, then the rest of the population.
- **Evidence:** in hamlet, the four friends (Eliphaz, Bildad, Zophar, Elihu) are founders k = 2..5. The reference never draws for them. Instead it draws for k = 42..45, who under ruling 10 are Job's servants and should take Job's settlement without a draw.
  - At the golden weights, per-settlement counts are 47/4/5/4 under R0 and 49/4/4/3 from the reference.
  - At R = 256 they are 50/4/2/4 under R0 and 52/3/2/3 from the reference.
  - Under R0 the friends draw settlements [4, 2, 1, 3].
- **Risk:** SPR-018 accepts the Rust port when it reproduces "every line" of the golden file, so the developer may copy the wrong numbering into `rh-worldgen`.
- **Fix:** derive the founder order from the YAML (cast, household membership, children and servants, the rest of the population), and generate the residence lines for solo, household and hamlet from the real manifests.

### MAJ-3: no world would register S3 (confidence 70%)

- **Where:** `specs/manifest/registry_v0.toml:453-460`; the manifest examples.
- **Problem:** no template's `default_systems` lists `S3.migration`, and no example manifest names it.
- **Why it matters:** worlds register only the systems they name. Ruling 3 builds the run header from the named set, and the M2 F1 draft says "solo, household and hamlet register neither T1.minds nor M7.dynasty", which only makes sense under per-world registration.
- **Consequence:** S3 never runs on a rung. Nothing writes `person.residence`, and V-DATA-04's P1a variant fails at G1a.
- **Fix:** add `S3.migration` to the templates' `default_systems` and bump `registry_version` in the same change. Alternatively, rule that S3 is registered whenever M1 and M2 are.

### MIN-1: newborns are selected by `birth_tick = now` (confidence 70%)

- **Where:** `S3.migration.odd.toml:136` and `:167` (R4).
- **Problem:** ruling 10 sets a founder's birth tick to −(age × minutes per year) − a keyed offset inside the year. Nothing pins the offset away from 0. An age-0 founder with offset 0 has `birth_tick = 0`, so at t = 0 S3 would treat that founder as a newborn:
  - with mother = 0, R4 reads the residence of PersonId 0, which is an error;
  - otherwise it appends a second interval, which breaks INV-S3-03.
- **Fix:** select rows with `person.residence = 0` (the sentinel the row insert writes), or `origin = born and entry_tick = now`.

### MIN-2: farthest-point seeding ignores land connectivity, so islands go unassigned (confidence 65%)

- **Where:** `M1.regions.odd.toml:240-241` (W6 and W7).
- **Evidence:**
  - `archipelago` at R = 512, K = 1, seed 8: 74,155 of 78,864 land cells (94%) have region 0.
  - At K = 4: up to 38,124 cells (about 50%) have region 0.
  - `desert_fringe` seed 3 also leaves a few unassigned cells (141 at R = 128).
- **Fix:** state this in `[isolation]` and `not_yet`, or place at least one seed per land component.

### MIN-3: bench 4 now measures a genesis-only kernel (confidence 70%)

- **Where:** `docs/08:239` against `docs/08:33`; `worldgen_lite.md` §5.
- **Problem:** the fixture measures a u16 integer smoothing pass that runs only at genesis. The §1 budget it stands for is the per-step f32 5-point stencil (0.65 ms on 4 threads), which this bench cannot validate.
- **Fix:** call bench 4 a proxy, say that a `[perf-baseline]` PR replaces it when the first f32 field arrives, and record the Architect's reading of the G0 ruling in the decisions log.

### MIN-4: INV-S3-01 breaks once SPR-019 back-fills dynastic ancestors (confidence 60%)

- **Where:** `S3.migration.odd.toml:189`.
- **Problem:** the invariant applies to "every life_course row". SPR-019's back-filled pre-epoch ancestors (PRJ-002, SPR-019 row) will have life-course rows and no residence intervals.
- **Fix:** scope the invariant to persons in the ledger at or after t = 0.

### MIN-5: households split across settlements in P1a (confidence 75%)

- **Where:** `S3.migration.odd.toml:153`; the M2 F1 draft (`specs/systems/next/M2.demography.odd.toml:51` on `origin/arch/M2-F1-births`).
- **Problem:** M2 F1 marriages (P1a) move a wife into her husband's household (N7), but S3 F0 never moves her residence. Households therefore span settlements throughout P1a.
- **Second problem:** the M2 F1 draft says "S3.migration F0 (SPR-018) bounds" the marriage pool by settlement, while S3 says nothing in P1a reads residence. The two drafts disagree.
- **Fix:** reconcile the two drafts and state the split explicitly.

### MIN-6: household membership is undefined in edge cases (confidence 65%)

- **Where:** `S3.migration.odd.toml:165-166` (R2, R3).
- **Problem:** R2 and R3 do not say what happens when:
  - a household head has no `household` field, or one that names another household;
  - one cast member heads two households.

  The validator accepts both. Such a founder is either placed twice or drawn as a loose founder.
- **Fix:** define membership as the head, the cast members whose `household` names h, and h's children and servants. Refuse a cast member who heads two households.

### MIN-7: Registry version (confidence 75%)

- **Where:** `registry_v0.toml:14-19`.
- **Assessment:** not bumping is safe today:
  - no world registers S3;
  - manifests only warn on a version mismatch;
  - provenance compares only the version string.

  It stops being safe with MAJ-3's fix, so bump the version together with that fix.

### NITs

- **NIT-1** (`check_all.sh:864-865`): the controls expect `WORLDGEN-LITE: FAIL`, which also matches the reference's own-failure path. Expect `STALE:` instead. Both controls do fail with STALE today.
- **NIT-2** (`worldgen_lite.md:69`): the claim "`05.9` IF-0 lists it as reserved" is false. Only DECISION_TRACE and ENCOUNTER are reserved (`channel_a_v0.sql:58`).
- **NIT-3** (W8): the centroid sums need u64 at R ≥ 2048 (n(R − 1) reaches about 2^36). State the width.
- **NIT-4** (the reference):
  - the accumulator check in `check_widths` cannot fail;
  - the founder rehash is assumed, never checked;
  - there is no check that the template ids equal the Registry's `[heightmap_templates]`;
  - it refuses a radius or width of 0 per mille, which W0 allows.
- **NIT-5**: S3's couplings omit S6.artifacts, which reads `person.residence`.
- **NIT-6**: two cost figures are off:
  - W6 at K = 5000, R = 4096 is about 8 × 10^10 distance updates, not 10^10;
  - S3's daily scan at 1M persons is about 3.6 × 10^8 compares a year, not 10^8.
- **NIT-7**: W5 says "the reference checks every template", but only flat and continental_plains appear at R = 64. My sweep confirms the claim itself for seeds 1–8.
- **NIT-8**: the lint fixtures moved to M3.agriculture will have to move again when M3 gets its spec (P1b). An overlay-only X-id would stay put.

## The drafter's open questions

1. **A channel-A `LIFE_INTERVAL` record.**
   - I agree with a non-event record:
     - `05.10` §4 promises that every move is in channel A;
     - DATA-5 keeps commoner moves out of `event` rows at scale, so moves need a record that is not an event.
   - It changes stream format v0, which SPR-013 is building, so rule on it with SPR-013, not in SPR-018. Fix the "reserved" claim (NIT-2).
   - Confidence 65%.
2. **`schema.sql`'s comment.** Agree: "residence (settlement id)", with no DDL change. Confidence 85%.
3. **V-DATA-04's span.**
   - Agree with `[entry_tick, exit_tick)`.
   - Also exclude SPR-019's back-filled pre-epoch rows (MIN-4).
   - This is the text of a G1a gate (D34), so record it as an Architect clarification in the decisions log and tell the director.
   - Confidence 75%.
4. **Focus slugs.**
   - Deferring is right, but do not map a slug through a household: a household's settlement is a keyed draw, and two households can share one.
   - Instead, warn on or refuse `world.focus` until a `settlements:` naming section exists. No example manifest uses `focus`.
   - Confidence 60%.
5. **A disconnected archipelago.** Accept it at F0, but document the unassigned islands (MIN-2). Confidence 80%.

## Governance

- **Not needed:** no ADR and no roadmap change.
- **Decisions log:** two items go there as Architect rulings:
  - V-DATA-04's span (open question 3);
  - bench 4's redefinition (MIN-3).
- **Registry:** adding S3 is C2. The MAJ-3 fix changes the templates, so it bumps the version, and it lands with SPR-018's C3 golden change, which happens anyway.
- **Change log:** the `CLAUDE.md` check line and the PRJ-002 status row follow earlier sprints' practice.

## Sound

- **Exactness and determinism:**
  - I re-implemented W1–W9 from the spec text alone:
    - numpy int64 for W1–W4, with W4 as a padded-edge Jacobi pass;
    - argmax-first for W6;
    - a bucket queue in (dist, cell, region) order for W7, with neighbours pushed in reverse order.
  - It reproduces all 9 golden cases: elevation and region SHA-256s, land counts, seeds, region sizes, seats, edges and routes. It also reproduces both bench 4 digests.
  - Every step is integer-only, and every tie-break is total:
    - W6: the lowest cell index wins;
    - W7: lexicographic order on (dist, cell, region), so the push order does not matter;
    - W8: order on (squared distance, index);
    - W9: a sorted set of edges.
  - Every draw is keyed, with a fixed count per blob and per household, and the key layout matches `05.1` §6.
- **Reference and gates:**
  - The reference passes.
  - Both controls fail with STALE.
  - `lint_odd` passes with 8 specs.
  - The emits and applies fixtures fail and pass as intended. Without the overlay, the overlay fixture fails.
  - The skeletons are up to date, and the stale control fails on M2 only.
  - The smoke-world lint and skeleton checks pass.
  - The inventory pytest passes (12 tests), and ruff and mypy are clean.
- **S3 scheduling and ownership:**
  - The `same_instant` reads create an M2 → S3 edge in Vital.
  - M2 inserts the newborn's row at the merge of its level, so S3 sees it at a later level. This is legal under 05.1 §2 rule 4.
  - A mother who dies in the same slot stays readable until Bookkeeping (ruling 5), and F9 drops a birth to a mother already dead.
  - The implicit interval end fits ruling 5.
  - Each column has one writer.
- **Manifest fit:**
  - The sweep covered 7 templates × R = 64/128/512 × K = 1/4/64 × seeds 1–8. No case was refused (W5).
  - val_khor (1024², K = 600): 1,023,137 land cells; the region graph is connected.
  - aethelgard (2048², K = 400): 4,044,364 land cells; the region graph is connected.
  - The smoke world is unaffected: it has no manifest and does not use `rh-worldgen`.
  - Every rung's golden changes, because of the new plane-A tables.
- **Bench 4:** the fixture is specified fully enough for `perf_gate.py record`.

## Not run

- No Rust or cargo checks, and not the full `check_all.sh`: `rh-worldgen` does not exist yet.
- `perf_gate.py` was not run.
- At R = 512, K = 64, only seeds 1–2 were run, not 3–8.
- No timing at K = 5000, R = 4096.
- `lint_odd` was not run against the M2 F1 branch.

## Reproduction

The scripts are in `/tmp/claude-0/-home-user-Rehoboam/78f410bc-aa41-55ce-883d-f1b2ebd2cbcd/scratchpad/indep/`:

| Script | What it does |
|---|---|
| `wg_indep.py` | Recomputes the golden cases |
| `bench4.py` | Recomputes the bench 4 digests |
| `sweep.py hamlet` | Checks the hamlet geography at R = 128 and 256 |
| `sweep.py iso` | Runs the isolation sweep |
| `big.py` | Generates the val_khor and aethelgard geographies |
| `hamlet_res.py` | Compares hamlet's residence under R0 order with the reference's numbering |

Each script takes the copy's root as its first argument.


## 2. Round 2 (2026-10-17, `d18973e`), with round 3 verified by the Architect

### 2.0 Architect rulings, round 2

| # | Finding | Ruling |
|---|---|---|
| R12 | R9 partly applied | The two validator refusals (Python and Rust, V-WC-04 parity) belong to the sprint and are written into PRJ-002's SPR-018 row as deliverables. S3's `default_systems` and the `registry_version` bump, and the kind 7 record's fields, are listed there too |
| R13 | NEW-1 | The `[isolation]` claim is corrected. Every golden case prints `graph components N connected yes|no`, and the reference asserts the desert_fringe R = 128, K = 4 claim for seeds 1–8 (connected except seed 3) |
| R14 | NEW-2, nits | The comment list is corrected; S3 no longer says `birth_tick = now`; the `[isolation]` horizon names exactly what the reference sweeps; kind 7 is named as reserved on `main` |

### 2.1 Round 3 (`f27b33b`), verified by the Architect
The reference passes, 10 cases. All 10 golden cases carry a components line: archipelago has 6 components and is disconnected, the others have 1. The corrected comment list and the PRJ-002 row are in place, and `lint_odd.py` passes (8 specs). The drafter's `--part core` passes 233 checks.

### 2.2 The review (round 2, independent reviewer)

Reviewer: independent (Opus 5.5), 2026-10-17. Copy reviewed: `git archive d18973e` in `worldgen_review_r2/src`. The repository was not changed.

> **BLUF:** **No blocker and no major.** Ten of the eleven rulings are APPLIED. R9 is PARTLY APPLIED: the rules are in the spec, but the validator still accepts both cases (the change is deferred to the sprint).
>
> I re-implemented the new W6 from the spec text alone, and it reproduces all 10 golden geography cases. That includes the archipelago case, now split into 6 land components with 0 unregioned cells. From the YAML in ruling-10 order, I independently recomputed all 9 residence lines, and they match. Hamlet at R = 256 matches the expected numbers exactly.
>
> **Two new minor findings, both text only:**
> - NEW-1: M1's `[isolation].expect` claims a connected graph for desert_fringe seeds 1–8. Under the new W6 (R5), seed 3 is now disconnected.
> - NEW-2: the M1 facts comment lists the wrong settlements for hamlet's 14 anonymous founders.
>
> **Three nits.**
>
> **The merge of main is clean.** It brings exactly `80343bf` and nothing else.

### Ruling status

| # | Status | Evidence |
|---|---|---|
| R1 | APPLIED | **Golden lines 27–36, case `desert_fringe 42 256 4`:** land 53859; edges `1-3 1-4 2-3 2-4 3-4`; routes `148 121 78 113 136`; region sizes 27691/6711/5996/13461; seats 18078 55338 54904 26409. My independent `w6new.py` reproduces every field, including both SHA-256s.<br>**R = 128:** kept as a separately named case (`selftest_worldgen_lite.py` CASES: "hamlet's template and seed at a smaller raster").<br>**`[isolation]`:** includes R = 256. But see NEW-2 for the facts comment. |
| R2 | APPLIED | **Order:** `Population.founder_order` (`selftest_worldgen_lite.py:473`) follows ruling 10: cast, then children and servants, then the rest. S3 R0 and R3 (`S3.migration.odd.toml:162,164`) state the order.<br>**Independent check:** `residence.py` derives the order from the YAML. It matches solo, household, hamlet@256, hamlet@128 and all 6 synthetic lines.<br>**Hamlet@256:** friends k = 2..5 draw `[4, 2, 1, 3]`; k = 6..45 all take Job's settlement 1; per-settlement counts 50/4/2/4. This is round 1's R0 prediction. |
| R3 | APPLIED | **Text:** the S3 header (`:9-11`), `[initialization]`, the PRJ-002 SPR-018 row and companion §6 item 4 all say it.<br>**Registry:** `registry_v0.toml` is unchanged (no bump; S3 is in no template). |
| R4 | APPLIED | **Selector `person.residence = 0`:** in the step (`:134`), R4 (`:165`), the submodel (`:182`), the column meaning (`:51`), sensing (`:148`) and `scale_behaviour`.<br>**Read removed:** the `person.birth_tick` read is gone; the `same_instant` read of `person.mother` keeps the M2→S3 edge.<br>**Residue:** NIT-A. |
| R5 | APPLIED | **Spec text:** W6 (`M1.regions.odd.toml:240`) says one seed per seedless 4-component, in order of lowest cell, then farthest-point over L. W7, INV-M1-02 (`:292`) and `not_yet` (`:35`) match.<br>**Independent check:** my re-implementation, written from the text, gives for `archipelago 7 96 6`: 6 components; seeds `3635 963 5697 6337 7990 8974`; regions `21c5c727…`; land_cells `953 1029 177 121 177 138`; 0 edges; 0 unregioned. All equal to the golden file, and all 10 cases match.<br>**Consequence:** NEW-1. |
| R6 | APPLIED | `docs/08:239`, row 4: the bench measures the genesis pass, and the per-step budget waits for M3 (P1b). |
| R7 | APPLIED | The scope (founders and the born; SPR-019's back-filled ancestors exempt) is stated in INV-S3-01 (`:186-187`), R5, `[initialization]` and the `docs/09:264` V-DATA-04 row. |
| R8 | APPLIED | **S3's side:** the `interaction` field (`:149`) and the M2 "out" coupling state a lagged mode read, the husband candidates restricted to the bride's settlement, and neutral 0 meaning no restriction; companion §4 says the same.<br>**M2's side:** the births branch (`13fbb70`) matches (`use = "mode"`, `neutral = 0`). |
| R9 | PARTLY APPLIED | **Applied:** R2 and R3 (`:163-164`) define the head's household (`household` absent, or h) and the membership. The reference's `Population.check` (`:462`) raises on both bad cases.<br>**Not applied:** `manifest_v0.py` still accepts both. My probes `manifests/two_heads.yaml` (Job heads two households) and `head_names_other.yaml` (eliphaz heads eliphaz_house but names job_house) both print `VALID … 0 errors`.<br>**Where it went:** the refusals are deferred to the SPR-018 sprint (S3 R2; companion §6 item 1). No SPR-018 sprint document exists yet to carry them.<br>**Recommendation:** accept the deferral, because V-WC-04 parity argues for changing Python and Rust together. Write it into SPR-018's task acceptance with two must-FAIL manifests. Confidence 7/10. |
| R10 | APPLIED | **Controls:** they expect `STALE:` (`check_all.sh:864-865`, `tests/lint_fixtures/README.md:112-113`), and both do fail with STALE.<br>**Companion:** the reservation claim is fixed.<br>**W8:** u64 sums are stated.<br>**Reference:** it checks the Registry template ids, founder-id distinctness (the only rehash trigger in `rh-ids`), widths of 0 per mille and the R = 64 sweep.<br>**Cost figures:** fixed (8 × 10^10; 3.6 × 10^8).<br>**S6:** the coupling is added.<br>**Residue:** the accumulator check (`:559`) is still vacuous (about 2^22 against 2^63); NIT-B. |
| R11 | APPLIED | **Q1:** S3 `not_yet` (`:39`).<br>**Q2:** `schema.sql:128`.<br>**Q3:** the `docs/09:264` row.<br>**Q4:** companion §7 row 7.<br>**Q5:** see R5.<br>**Residue:** NIT-C. |

### New findings

#### NEW-1 (MIN, 95%): M1's isolation claim is false for desert_fringe seed 3 under the new W6

- **Where:** `specs/systems/M1.regions.odd.toml:330`, which reads: "desert_fringe at R = 256 (hamlet) and at R = 128 gives 4 regions and a connected graph for seeds 1 to 8 … REV-SPR-018 §1 confirmed both".
- **Why it changed:**
  - Round 1 confirmed this under the old W6, which left seed 3's island unregioned.
  - Under R5, seed 3 has 2 land components at R = 64, 128 and 256. The island (22, 141 and 575 cells) now takes region 2, which has no edge.
  - The edges are `(1,3) (1,4) (3,4)` at both R = 128 and R = 256, so the graph is disconnected (`indep2/sweep2.py desert`).
- **Why nothing caught it:** the reference never checks connectivity, so it passes. A V-SF-22 isolation test written from this text would fail on seed 3.
- **Fix:** name seed 3 as the R5 exception. Its island is a settlement with no route until F1's sea mode.
- **For the Architect:** this is R5's intended trade-off, made visible. In a 4-region world, a 141-cell island takes one of the four settlements, and with R8 it has its own marriage pool.
- **The rest of the sweep is clean:** 7 templates × R = 64, 128 and 256 × K = 1, 4 and 64 × seeds 1–8, with no refusal. Only archipelago and desert_fringe seed 3 are disconnected or have leftover land, and leftover land appears only when K is below the component count.

#### NEW-2 (MIN, 99%): the hamlet facts comment lists the wrong settlements for the 14 anonymous founders

- **Where:** `M1.regions.odd.toml:251`. It says "the 14 others 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 2".
- **What is right:**
  - The golden line (`worldgen_lite_v0.txt:36`, positions 46–59) and my independent computation give `1 2 1 2 4 3 2 1 1 4 1 1 4 1`. Per settlement, that is 50/4/2/4.
  - The comment's list is a window offset into the run of 1s.
  - The comment also credits round 1's re-implementation with these numbers, but round 1 never computed this list.
- **Why it matters:** the comment heads "Facts … the plan's acceptance criteria may use".

#### NITs

- **NIT-A:** `S3.migration.odd.toml:221`, the M2 "in" coupling, still reads "(person.birth_tick = now, person.mother); same_instant reads". R4 removed the `birth_tick` read.
- **NIT-B:** `M1.regions.odd.toml:329`, `[isolation].horizon`, says "the reference runs the R = 64 part on every check". `check_sweep` runs K = 1 and 4 only (`SWEEP_REGIONS`, `selftest:90`), not K = 64.
- **NIT-C:** main has since reserved channel-A kind 7, RESIDENCE_INTERVAL (`7fb71c7`, not yet merged into the PR). On the next merge of main, two texts should name kind 7:
  - `worldgen_lite.md:71`, which says "only DECISION_TRACE and ENCOUNTER are reserved";
  - S3 `not_yet` (`:39`), which says "the Architect reserves its record-kind number".

#### Observations (low, no fix needed in this PR)

- **R4's premise needs an owner.** R4 assumes that SPR-014's row insert writes the type's zero into other owners' columns. That is asserted in this PR and in ruling R4, but neither EVO-001 nor PRJ-002's SPR-014 row states it yet. Carry it into SPR-014's contract.
- **Hard-coded rung populations.** The reference hard-codes them (`selftest:660-671`). I checked them against `solo.yaml`, `household.yaml` and `hamlet.yaml`, and they are equal today. But a manifest edit would not regenerate the golden file.
- **Untested paths:**
  - No golden case covers a head with no `household` field (the `order[head] = …` branch).
  - `birth_sequence` never exercises R4's selector, for example a founder with `birth_tick = 0`.

### Merge of main (80343bf)

- **Clean merge:** `f93490e` has parents `45e74d7` and `80343bf`. `git show --cc` shows no conflict hunk.
- **Nothing unexpected:**
  - `git diff 45e74d7 f93490e` is byte-identical to `git diff 132e17d 80343bf`.
  - That diff touches two files: the SPR-012 T-05 acceptance text (version 1.0.1) and the handoff session table.
- **Still merges onto current main:** `git merge-tree` of `d18973e` onto `7fb71c7` is clean.

### Checks run on the copy

| Check | Result |
|---|---|
| `selftest_worldgen_lite.py` | `WORLDGEN-LITE: PASS (10 cases, 109 lines)`, 4.5 s |
| Golden control | Fails (rc 1) with `STALE: … line 9` |
| Template control | Fails (rc 1) with `STALE: … line 8` |
| `lint_odd.py` | `PASS: 8 system specs linted (6 warnings)` |
| `gen_skeletons --check` | 8 up to date |
| The 13 fixture commands from the README / `check_all` | All behave as expected; the stale control flags `M2_demography.rs` only |
| Smoke-world lint and skeletons | Pass |
| The six example manifests | Validate |
| `gen_manifest --check` | Clean |
| Chronicle self-test | Passes |
| venv: inventory pytest | 12 passed |
| venv: `ruff check`, `ruff format --check`, mypy | Clean (mypy: 39 files) |
| Bench 4 digests | Reproduced |
| `templates_v0.toml`, `registry_v0.toml` | Unchanged |

**Not run:**
- No cargo, no full `check_all.sh` and no `perf_gate.py`.
- The sweep did not go to R = 512.

### Reproduction

Scripts are in `/tmp/claude-0/-home-user-Rehoboam/78f410bc-aa41-55ce-883d-f1b2ebd2cbcd/scratchpad/worldgen_review_r2/indep2/`. Each takes the copy's root as its first argument.

| Script | What it does |
|---|---|
| `w6new.py` | The round-2 W6, written from the text, on top of round 1's `wg_indep.py` |
| `cmp_golden.py` | Compares all 10 cases with the golden file |
| `sweep2.py desert` | The desert_fringe connectivity check behind NEW-1 |
| `sweep2.py sweep` | The full sweep; output in `sweep.out` |
| `residence.py` | Ruling-10 order from the YAML, compared with every golden residence line |

The R9 probe manifests are in `manifests/`.
