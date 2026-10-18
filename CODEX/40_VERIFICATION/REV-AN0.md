---
id: REV-AN0
title: "Independent review of AN-0: the multi-segment chronicle fold (annals_v0.md §9.1; SPR-029 F-3; REV-WAVE8 A1)"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [SPR-029, AN-0, AN-1, REV-WAVE8, PRJ-002]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing commit (2026-10-18).** AN-0 establishes the multi-segment chronicle fold contract in `specs/annals/annals_v0.md` §9.1, the reference implementation (`fold`, `segment`, `check_segment_years`, check 11) in `specs/annals/selftest_annals_v0.py`, the positive control `annals_fold_control` in `bin/check_all.sh`, and updates SPR-029's plan.
>
> - **Round 1** (`0eb4243`, on `main` `99a5b3d`): **AFTER SMALL FIXES**, 0 BLK, 0 MAJ, 7 MIN, 6 NIT (§1). Every touched gate passed. Independent oracle (from §9.1 and `canonical_v0.md` §4 alone; stdlib XXH3 and C xxHash 4.0.1) agreed on 607 comparisons (341 with 2+ blocks) with 0 mismatches.
> - **Rulings** (§0): every finding accepted (R1–R12; R13 declined). Owner F rulings applied to spec, reference, and tests. Owner A rulings applied to SPR-029 plan.
> - **Verification** (§0c): all 11 checks pass in `selftest_annals_v0.py --check`; `annals_fold_control` fails as pinned; inventory chains 587 gates both ways; ruff, format, mypy, and `check_all.sh --part core` pass 100% green.

## 0. Architect rulings (2026-10-18), on `0eb4243`

The independent review returned AFTER SMALL FIXES: 0 BLK, 0 MAJ, 7 MIN, 6 NIT. The oracle agreed on 607 comparisons with 0 mismatches. R1–R12 accepted; R13 declined.

| # | Ruling | Owner | Implementation |
|---|---|---|---|
| R1 | `load_chronicle` splits on LF only (from `checked.records`), so a record hidden behind form-feed in a comment is never rendered; plant in check 11 | F | `load_chronicle` reads `checked.records`; `_fold_plants_and_differentials` verifies hidden event is not loaded |
| R2 | Plants for surviving mutants: free comments anywhere; 7-digit year; `y000002` after `y000003`; one-record world cut at 360- and 365-day year boundaries | F | Added to `FOLD_REFUSALS`, `_fold_comments_and_seq`, and `_fold_calendar_check` |
| R3 | Differential: each of `canonical_v0.REFUSED_FILES` must be refused by `fold` | F | `_fold_plants_and_differentials` asserts `fold` raises on every planted file |
| R4 | §9.1: refusal names the line where rule has one, and segment where rule has one | F | Updated `specs/annals/annals_v0.md` §9.1 line 152 |
| R5 | Rule 2 reads "no line begins `#segment`"; file with no records refused | F | Updated `specs/annals/annals_v0.md` §9.1 rule 2 & rule 1; added refusal plant to `FOLD_REFUSALS` |
| R6 | Merge `origin/main` and keep both helpers in `bin/check_all.sh` | F | Merged `origin/main` (2fc58c3) cleanly |
| R7 | `fold` stays calendar-free; `check_segment_years` checks sim-year on world calendar when known; SPR-029 T-03 ports it | F, A | `check_segment_years` implemented and called by `load_chronicle`; documented in §9.1; ported to SPR-029 T-03 |
| R8 | §9.1: only exact `#segment` prefix is reserved; other spellings (`#Segment`, `# segment`) are free comments | F | Documented in §9.1; verified in `_fold_comments_and_seq` |
| R9 | Bounded integer grammar (1–20 digits plus u64 range check) | F | Enforced in `SEGMENT_LINE` regex and `_segment_line`; 21-digit and past-u64 plants added to `FOLD_REFUSALS` |
| R10 | `fold` and `segment` handle missing files and errors cleanly without tracebacks; document `ONE_SEGMENT_LINE` | F | Clean error handling in `_fold_command` and `_segment_command`; `ONE_SEGMENT_LINE` documented in §9.1 |
| R11 | First block may start at any seq; exporter starts at 0 | F | Documented in §9.1; seq 5 block test added in `_fold_comments_and_seq` |
| R12 | SPR-029 T-03 reworded: changed record with unchanged segment line is refused | A | Updated in `CODEX/05_PROJECT/SPR-029_FamilyAnnals.md` T-03 |
| R13 | Declined: fold stays in `selftest_annals_v0.py` | — | Kept in `selftest_annals_v0.py` |

## 0c. Architect verification

| Check | Result |
|---|---|
| `specs/annals/selftest_annals_v0.py --check` | PASS (all 11 checks pass; check 11 verifies KAT answers, comments, R1-R3 plants, differentials, and fixture cuts) |
| `annals_fold_control` | PASS (exits 1 and prints `FOLD REFUSED: target/rendered-fixtures/annals_fold/altered.txt:4: segment y000003 (records on lines 5-7): its #segment line says canonical_digest=00000000000000000000000000000000; its records recompute to 497ac1184aadd39d525247eb1f164cc7`) |
| `bin/check_all_selftest.py inventory` | PASS (587 gates chained and listed, each once, both ways) |
| `pytest python/tests/test_check_all_inventory.py` | PASS (20 passed in 1.87s) |
| `ruff check` & `ruff format --check` | PASS (clean across repository with `python/pyproject.toml`) |
| `mypy --strict` | PASS (clean across repository) |
| `bin/check_all.sh --part core` | PASS (all core gates green) |

## 1. Review summary (round 1, `0eb4243`)

The review ran on a model different from the drafter's, with no cargo build.
Independent oracle agreed on 607 comparisons with 0 mismatches.
Mutants: check 11 originally killed 28 of 35 mutants; with R1, R2, R3, R9, R11 plants added, all surviving mutants are killed.
Refusals: all 16 drafter refusals reproduced, plus CRLF, lone CR, trailing/leading space, 7-digit year, seq past u64, empty file, and single record without blocks.
