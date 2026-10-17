---
id: REV-SPR-018
title: "Independent review of the worldgen-lite, M1.regions F0 and S3.migration F0 specs (PR #52)"
type: record
status: OPEN
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-018, SPR-015, SPR-013, SPR-019, REV-SPR-015, EVO-001]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF:** Round 1 on PR #52 at `45e74d7`: **no blocker; three majors to fix before merge**, plus seven minors and the nits. The algorithm is sound. An independent re-implementation of W1–W9, written from the spec text alone, reproduced every number in the golden file, including both bench 4 digests. S3's same-instant read is scheduled legally, and the implicit interval end agrees with SPR-011 ruling 5. The majors are these: the `hamlet` golden case uses the wrong raster size; the founder order is not ruling 10's; and no world registers S3, so it would run on no rung. The Architect's rulings are in §0. The review ran in a session on a different model from the drafter's.

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
