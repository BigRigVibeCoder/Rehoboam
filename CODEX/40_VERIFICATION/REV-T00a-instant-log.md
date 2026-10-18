---
id: REV-T00a
title: "Independent review of the T-00a spec PR: EVO-002 (the instant log and the core genealogy index), next/M7 0.6.0, the [[observes]] lint and the executable model of the instant log"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [EVO-002, SPR-021, SPR-022, SPR-023, SPR-024, SPR-025, SPR-038, SPR-015, SPR-019, REV-WAVE7, REV-MINDS, REV-T1A, PRJ-002, BCK-002]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing commit (2026-10-18).** EVO-002 is 0.2.0, and M7 0.6.0 stays a `next/` draft until SPR-021a T-01 promotes it (after T-00b).
>
> - **Round 1** (`a3ffe64`, on `main` `78fef08`): **AFTER SMALL FIXES**, 0 BLK, 2 MAJ, 14 MIN, 10 NIT (§1). Every touched gate passed. The reviewer's independent oracle found 0 mismatches over 1,000 seeds (131,330 views). EVO-002 matches X1, X2, R9, R10 and R14.
>   - MAJ-1: the model could not kill an unsorted `exited()`, because every later death had a higher id.
>   - MAJ-2: `05.1` §4.2's visibility sentence contradicted the instant log.
> - **Rulings** (§0): every finding accepted. The model gains `--emit-vectors` (MIN-5). Criterion (e) reads a compiled `cast_ids` constant (MIN-11, which also answers REV-T1A F8). F-3's exit timer becomes a 0/1 flag (MIN-1).
> - **Revision 1** (`dcb2c06`, then merges of `main` `17ca24f` and `511b7cf`, head `085c400`): every row applied. The Architect's verification is §0c.
> - **The Architect's rows are already on `main`:** MIN-4 (SPR-038 0.3.3 T-07, the no-re-entry refusal, +0.1 day), MIN-12 (SPR-019 0.3.3, `house_member` ascending at genesis), MIN-14 (SPR-021 0.3.1, T-02 +0.5 day), all in PR #97. MIN-13's BCK-002 A-69 is on `arch/viewer-plans` (BCK-002 1.22.0), which merges next.

## 0. Architect rulings, round 1 (2026-10-18), on `a3ffe64`

The independent review returned AFTER SMALL FIXES: 0 BLK, 2 MAJ, 14 MIN, 10 NIT. The reviewer's oracle found 0 mismatches over 1,000 seeds (131,330 views), and EVO-002 matches X1, X2, R9, R10 and R14. The report's text is in this conversation's record and in `t00a_review/REPORT.md`. **Every finding is accepted unless a row says otherwise.**

| # | Ruling | Conf. |
|---|---|---|
| MAJ-1 | **Accepted.** Reorder `succession_day` so that a later merge's death has a lower PersonId (for example, a murder of P3 while P5 and P8 die in the timer pass). Add an `inserted()` case whose later-merge id is below an earlier one. Add an `unsorted-exited` control with its `check_all` gate and its CLAUDE.md line | 85% |
| MAJ-2 | **Accepted.** `05.1` §4.2: "…visible to later phases in the same instant only through owned tables, the instant log and the merge-time citation form (§2 rule 2)" | 95% |
| MIN-1 | **Accepted.** Amend `owns.title.exit_timer.meaning` to a 0/1 flag in 0.6.0, with its record; F-3 is T-00a's. Its type stays u64 (X13f's pattern) | 85% |
| MIN-2 | **Accepted, F-27.** EVO-002 §6 says: an owner's derived index (M7's house → title-holders) is read by other systems through the owner's contract crate (`rh-contracts::m7`, built by 021a; R11), as an accessor over lagged columns. The core hosts only the genealogy index | 85% |
| MIN-3 | **Accepted.** The `registry_delta.toml` header says M7's section stays until SPR-021a T-01 moves it, and T1's until SPR-024a T-01. SPR-015 0.2.3 already carries F-28 (merged in PR #95) | 90% |
| MIN-4 | **Accepted.** SPR-038b builds the no-re-entry refusal; delete "or SPR-021a". The Architect adds the deliverable and its acceptance line to SPR-038 T-07 (+0.1 day: a stub inserting at Bookkeeping after compaction), with "conform to `05.1` as amended" | 85% |
| MIN-5 | **Accepted, in T-00a.** The model gains `--emit-vectors`: JSON of systems, bindings, edges, observes, world, scripted actions and the expected merge records, with a staleness check like the repository's other `gen_*` vectors. Without it, V-DET-22's "engine equals the model" cannot be run | 75% |
| MIN-6, MIN-7 | **Accepted.** Add reads of a founder-parent's children on day 1 and after the keyframe resume on days 2–3, so RESUME compares them and `m10` dies. Add an EXPECT that the Year pass sees the pre-death holder (K12) | 85% |
| MIN-8 | **Accepted.** Render planted controls for the six unpinned observes rules (missing `why`, unknown field, duplicate, own kind, phase rule, non-table entry), as `odd_next_control` does, each failing with its own diagnostic | 80% |
| MIN-9 | **Accepted.** Add an exception clause to `05.1`'s re-sample sentence: a timer whose drawn time decided an outcome already emitted (the interregnum crisis's resolution timer) keeps its tick at a re-sample. State that no P1a input scope can name a title | 70% |
| MIN-10 | **Accepted.** T-00b declares the regency-end `[[rng]]` purpose (no-draw, as T1's `outcome_delay`). T-00a rewords `scales.restart` so it states no rule for a purposeless timer | 80% |
| MIN-11 | **Accepted, in T-00a.** Criterion (e)'s data source is declared. M7's `[initialization]` compiles the manifest `cast` into a sorted run constant `cast_ids`, resolved by V-DATA-05's founder index, and states it in `[input_data]`. It enters the history through `manifest_hash`, and dormancy's criterion (e) tests membership in it. This also answers REV-T1A F8's marker question | 75% |
| MIN-12 | **Accepted.** INV-M7-09's genesis half, `house_member` written in ascending PersonId, is SPR-019b's, with a test. The Architect amends SPR-019 at the wave-8 close, with X1 | 80% |
| MIN-13 | **Accepted.** The `next/README.md` row names the two `type = "count"` additions. **BCK-002 A-69** (filed by the Architect): the deny-by-default lint does not track new fields on frozen entries | 70% |
| MIN-14 | **Accepted.** SPR-021a T-02 is +0.5 day for the merge-time form, the Rust analogues of the model's controls as mutants, and `UndeclaredObserve` and `InstantCause`. The Architect amends SPR-021 at the T-00a close | 75% |
| Q1–Q9 | The reviewer's views are adopted:<ul><li>Q1: the merge-time form, +0.5 day.</li><li>Q2: SPR-038b builds the refusal.</li><li>Q3: `05.1` is amended here, with MAJ-2.</li><li>Q4: MIN-9.</li><li>Q5: MIN-10.</li><li>Q6: T-00b also states that persons in `exited()` are skipped for Promote, Demote, regrant and vocation.</li><li>Q7: the requirement texts go to the director in D58 item 6, as a narrowing, honestly put.</li><li>Q8: observes is an error on drafts.</li><li>Q9: the `OBSERVES` constant's shape goes into EVO-002 §5.</li></ul> | 75% |
| NIT-1–NIT-10 | **Accepted:**<ul><li>NIT-1: six re-stated records.</li><li>NIT-2: a separator in the `reason` strings.</li><li>NIT-3: the `envelope_v0.md` §2 sentence.</li><li>NIT-4: V-DET-22 in the traceability table.</li><li>NIT-5: none needed.</li><li>NIT-6: state that `own_draft` wins when both match.</li><li>NIT-7: the `Option` target form.</li><li>NIT-8: "by a step's `write()` put-back".</li><li>NIT-9: keep, noted.</li><li>NIT-10: pre-existing; recorded, no change.</li></ul> | 85% |

**The drafter's revision 1:** all rows except the Architect's own (MIN-4's SPR-038 line, MIN-12's SPR-019 amendment, MIN-13's A-69, MIN-14's SPR-021 line). Merge `origin/main` (99a5b3d or later). Regenerate the lock only by the tool and the goldens only by `--write`. Run the touched gates. Push once.

## 0c. Architect's verification of revision 1 (2026-10-18), on `085c400`

Run in a detached worktree of `arch/M7-0.6-evo-002` at `085c400` (revision 1 `dcb2c06`, with `main` `511b7cf` merged: T1-A is in).

| Check | Result |
|---|---|
| `specs/engine/instant_log_model.py --check` | `INSTANT LOG MODEL v0: PASS`; the transcript (237 lines) and the vectors (`instant_log_vectors_v0.json`, 3,485 lines) match the reference |
| The eight `--control` runs | Each exits non-zero and prints its pinned `check_all` text: `peer-visible` (THREAD INVARIANCE), `unsorted-inserted` and `unsorted-exited` (ORDER), `log-not-cleared` (CLEARED), `hash-includes-log` (HASH), `genealogy-late` (GENEALOGY), `late-watermark` (VISIBILITY), `cast-demoted` (NAMED) |
| MAJ-1 in the golden | `succession_day`: the timer pass exits P5 and P8, then M2's drain exits P3, so `exited()` reads `[5, 8]` and then `[3, 5, 8]`. `newborn_dies`: P45, inserted at a later merge, reads before P50. An appended `exited()` now fails ORDER |
| MAJ-2 | `05.1` §4.2:176: "visible to later phases in the same instant only through owned tables, the instant log and the merge-time citation form (§2 rule 2; EVO-002)" |
| `lint_odd.py --next` with M2, M7 and T1 drafts | M2: 46 planted changes rejected, 45 amendments locked. M7: 42 rejected (2 by the lock), **34 amendments recorded and locked**. T1: 43 rejected, 21 locked. The clean next version kept for each. The lock holds 100 paths, regenerated by `--write-amendments-lock` after the T1-A merge |
| `lint_odd.py` (frozen) | PASS: 8 system specs linted |
| `gen_manifest --check` | builds cleanly (223 documents) |
| MIN-11 | M7 0.6.0's `[initialization]` declares `cast_ids` (the cast's founder ids by V-DATA-05's index, sorted, entering the history through `manifest_hash`); criterion (e) tests membership. The `cast-demoted` control fails without it |

**Not run here:** the cargo gates of part core (CI runs them). The drafter's local part core without cargo had one FAIL, V-DATA-10 on the envelope fixture, which needs a segment a skipped cargo test builds (the same in round 1); CI decides it.

**Process note:** the merge commit `a200517` (of `99a5b3d`, made before the drafter's context was compacted) carries no `Co-Authored-By` trailer. It is a merge commit with no content of its own beyond the resolution, and it is left as it is rather than rewritten across later merges.

## 1. The independent review, round 1 (`a3ffe64`)

The review was independent, on a different model from the drafter's. The Architect recorded this report from the reviewer's returned text. The evidence (oracles, harness, mutants) is in `t00a_review/work/`.

### Verdict

**Verdict: AFTER SMALL FIXES.** 0 BLK, 2 MAJ, 14 MIN, 10 NIT.

| Area | Result |
|---|---|
| Touched gates | All green: 43 lint and model gates, `gen_manifest`, ruff, mypy, and 690 pytest. Warning counts equal main's |
| Controls | All seven model controls fail on their own message; all 23 `--next` controls and the 4 observes fixtures behave as pinned |
| Independent oracle | 0 mismatches over 1,000 random seeds and 131,330 views, with random job orders and all six citation outcomes. It kills 6 of the model's mutants; the model's self-test misses one (MAJ-1) |
| EVO-002 against X1, X2, R9, R10, R14 | Matches every ruling. Loose ends: the no-re-entry refusal's builder; an "or SPR-021a" conditional; F-27; the model cannot be compared with Rust as written |
| M7 0.6.0 | All 32 `[[amends]]` texts match. The 16 defaults equal `twin_vales.yaml`, the delta and the bounds. Gaps: F-3 omitted; INV-M7-09's genesis half and criterion (e)'s data source have no owner |
| `lint_odd.py` | The observes schema is right; the R3 fix breaks no control; observes as an error on drafts is fine |
| The T1-A branch | One textual conflict (`next/README.md`'s table). A trial merge lints, and 697 pytest pass |

### Key results

- **Mutant kill matrix:**
  - `exited()` appended instead of sorted-inserted: **survives** the model, killed by the oracle.
  - The index never built from construction rows: **survives** both.
  - Swapping the Year and Day pulse order (K12): only the golden compare catches it.
- **The thread-invariance check** varies job order per level. That is a reasonable abstraction, but only two levels have several jobs.
- **The merge-time citation form is well defined:** the watermark is the length of (c) at level start, and the four named refusals plus `UndeclaredObserve` are each reachable.

### Findings

| ID | Sev | Problem | Fix |
|---|---|---|---|
| MAJ-1 | MAJ | The model cannot kill an unsorted `exited()`: every later death has a higher id. R9 is unchecked | Reorder the deaths; add an `inserted()` case; add an `unsorted-exited` control |
| MAJ-2 | MAJ | `05.1` §4.2:176, "visible … only through owned tables", contradicts part (c) | Reword |
| MIN-1 | MIN | F-3 (`title.exit_timer` as a 0/1 flag) is neither done nor listed | Amend, or give it to T-00b |
| MIN-2 | MIN | F-27 (how another system reads an owner's index) is unanswered | One sentence |
| MIN-3 | MIN | F-28 is half applied in the delta header | A header sentence |
| MIN-4 | MIN | The no-re-entry refusal has no builder; an "or SPR-021a" clause survives | SPR-038b builds it |
| MIN-5 | MIN | "Engine equals model" cannot be run: the golden is a transcript | `--emit-vectors` JSON with a staleness check |
| MIN-6 | MIN | Genealogy coverage is thin; `m10` survives; V-DET-22's restore clause is unexercised | Add reads on day 1 and after the resume |
| MIN-7 | MIN | K12 is asserted nowhere | Add an EXPECT |
| MIN-8 | MIN | Only 3 of 9 observes rules have a planted FAIL gate | Render controls |
| MIN-9 | MIN | The interregnum timer is kept at a re-sample, against `05.1`'s sentence | An exception clause |
| MIN-10 | MIN | The regency-end timer has no `[[rng]]` purpose | T-00b declares it; reword `scales.restart` |
| MIN-11 | MIN | Criterion (e) has no declared data source | Declare the channel (cast ids compiled from the manifest) |
| MIN-12 | MIN | INV-M7-09's genesis half is unowned, and no golden pins the order | SPR-019b, with a test |
| MIN-13 | MIN | `type = "count"` on frozen params bypasses the record trail | Name it in the README; note the lint hole |
| MIN-14 | MIN | SPR-021a's +0.25 day estimate for the merge-time form is light | +0.5 |
| NIT-1–10 | NIT | Six re-stated records, not three; reason separators; `envelope_v0.md` §2; the traceability table; checks that hold by construction; `own_draft` vs `same_level`; the `Option` target form; the exit-scan wording; sprint names in normative text; a pre-existing `exit_deposition` fork difference | Edits |

### The drafter's nine questions

1. Agree, +0.5 day (7/10).
2. SPR-038b builds the refusal, +0.1 day (7/10).
3. Amend `05.1` here and fix §4.2 (8/10).
4. Keep the timer at a re-sample, with an exception clause (7/10).
5. T-00b declares the purpose (8/10).
6. Agree; T-00b also states that exited persons are skipped (7/10).
7. Leave to the director, honestly as a narrowing (8/10).
8. Right (9/10).
9. Keep it in 021a T-01, and state the constant's shape in EVO-002 §5 (7/10).
