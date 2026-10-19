---
id: VER-SPR-038a
title: "Retroactive record of SPR-038a (per-instant cost, stage A)"
type: record
status: DRAFT
owner: architect
agents: [architect]
tags: [verification]
related: [SPR-038, REV-SPR-038, SPR-014, SPR-015, SPR-038b, BCK-002, PRJ-002, VER-SPR-015-INTERIM]
created: 2026-10-18
updated: 2026-10-18
version: 0.1.0
---

> **BLUF: RECORDED, NOT AUDITED.** SPR-038a (PR #104) merged on 2026-10-18 with no Architect audit and **no independent Tester pass**. This file records what the repository and the PR show, so the record exists (BCK-002 A-78). It is not a pass. The Director approved a retroactive independent Tester pass (2026-10-18); this record stays DRAFT until `VER-SPR-038a-TEST.md` exists and the Architect has re-run the gates.

## 1. What merged

| Item | Value | Source |
|:--|:--|:--|
| PR | #104, `sprint/SPR-038a-per-instant-cost` | `gh pr view 104` |
| PR head | `f7e2985e` | same |
| Merge commit | `4eaf574d`, merged 2026-10-18T16:04:18Z | same |
| Change class | C2 `[perf-baseline]`; no state hash or ladder changed | PR body |
| Author | Developer (Rust core), a Gemini model (the branch's owner, GOV-011 §2) | PR body |
| CI on the PR | core, golden, stream all SUCCESS | `statusCheckRollup` |
| Reviews / comments on the PR | none (0 and 0) | same |

## 2. Rulings given by message before the PR (recorded in `rulings_by_message.md`)

- **T-02's death target met in substance (75%),** restated: compaction at most 5% of `death/n10k_*`'s instructions, and the whole death instant at least 3.5× faster at L = 10^5 (reported 4.6× at L = 10^4, 3.7× at 10^5; compaction 3.4% of the instant). The remainder is SPR-038b's: O(N+L) checks about 48%, the guard's O(L) value checks about 44%.
- **Six deviations accepted (80%), with conditions:**
  1. Guard mode 3 fails `life-course-one-open-entry`, and a test shows the append is otherwise legal.
  2. The unmarked-put-back mutant is gated by `ledger-no-dead-row`'s differential, failing by message.
  3. `rh_core::REFERENCE`: the A-52 control text names both features, and `test_mutant_features.py` covers it.
  4. Per-slot write stamps: a DECISION comment states the equivalence; **the Tester checks "since it last ran" at a skipped binding and at a resume.**
  5. The 2,500-founder size: the test says why smaller sizes cannot see the mutant.
  6. The debug `m2_system` target goes in the PR body.
- **Order:** merge `main` after SPR-014, open the draft PR, **the Tester runs phase 2.**

## 3. Figures the PR body reports (arena allocator, one machine; not re-run by the Architect)

| Case | Base (instructions) | Head (instructions) |
|:--|--:|--:|
| `quiet/n1_l1` | 81,760 | 14,306 |
| `quiet/n10k_l100k` | 26,150,856 | 14,306 |
| `death/n10k_l10k` | 11,901,511 | 2,594,905 |
| `death/n10k_l100k` | 47,227,270 | 12,900,470 |
| `insert/n10k_l100k` | 55,341,787 | 34,998,468 |
| `year/n10k_l10k` | 3,137,900,364 | 23,192,514 |
| `year/n10k_l100k` | 9,650,929,394 | 79,901,376 |

- Quiet instant: 26,150,856 to 14,306 (target at most 20,000), zero allocations.
- M2 invariants per call at L = 10^5: 17,887,853 to 4,848,162.
- The developer's earlier interim report (one machine, head `2116418`, base `1a67ab9`) also gave: debug `m2_system` 74.9 to 24.6 s; M2 gates summed 153 to 36 s; `kingdom_10k` about 1,530 s per 1,000 sim-years after stage A (from about 2,450); projection after stage B with `yearly` 80 to 117 s, "necessary, not sufficient" (D60).
- The PR body says tasks T-01..T-04 are done and that local `bin/check_all.sh` core, golden and stream passed.

## 4. Gaps against the plan and governance

| # | Gap | Evidence | Consequence |
|:--|:--|:--|:--|
| 1 | No independent Tester pass; the ruled "phase 2" has no record | no `VER-SPR-038a-TEST.md` on `main` | GOV-011 §4 not met; Tester pass commissioned |
| 2 | No Architect audit before merge | no `VER-SPR-038a` until this file | this file; stays DRAFT |
| 3 | No independent code review recorded for the head | no `REV-` for the code | owed |
| 4 | Ruling item 4 (stamps at a skipped binding and at a resume) has no named verifier | message only | the Tester must check it |
| 5 | The debug `m2_system` figure is in the interim report, not in the PR body as ruled (item 6) | PR body has no debug figure | the Tester measures it |
| 6 | `SPR-038` plan status still reads APPROVED; stage A has merged | plan file | update when audited |

## 5. Known downstream effect

SPR-015's head `cda5d16d` moves `instant/quiet` from 14,306 to 23,230 instructions (+62%) and `instant/year/n10k_l10k` by about +14.5% (CI golden, V-PERF-04; see `VER-SPR-015-INTERIM`). The stage A figure of 14,306 is the baseline that regression is measured against. Exuberant must diagnose it before any baseline moves.

## 6. What the Tester pass must cover

1. Re-run the instant bench on the merge commit `4eaf574d` and compare with section 3, on one allocator, stating which.
2. Differential and oracle gates: per-instant differential, `per_instant_cache`, `invariants_oracle`, and each mutant control fails on the planted violation.
3. Ruling item 4 (write stamps at a skipped binding and at a resume).
4. Byte-identity of ladders, run directories, tail and replay against the base.
5. The debug `m2_system` gate time.
6. A written finding for every gap in section 4.

## 7. Verdict

None. Recorded, not audited. Do not describe SPR-038a as verified until section 6 is done.
