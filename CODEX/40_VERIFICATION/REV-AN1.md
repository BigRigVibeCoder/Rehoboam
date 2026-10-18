---
id: REV-AN1
title: "Independent review of AN-1: annals fixtures for M7 F1 (REV-WAVE7 K2, REV-WAVE8 G15/A1, SPR-029 F-21/F-24)"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [SPR-029, AN-0, AN-1, REV-WAVE7, REV-WAVE8, PRJ-002, M7]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing review (2026-10-18).** AN-1 aligns the annals fixture suite and self-test with M7 0.6.0's rules for F1:
> - **K2 resolution:** Non-sovereign titles may only emit `INTERREGNUM_CRISIS` while their liege's interregnum is open. Verified by `selftest_annals_v0.py` check 5 and tested against regression by the positive control `fixture-vassal-liege-held`.
> - **Distinguishing child (F-21):** Elsa, daughter of Wynn by the houseless Orm, proves the agnatic membership rule in House Ashcombe—her birth is recorded through her mother, but her death is excluded because maternal house membership never passes.
> - **High Fells fixture (F-24):** A fourth fixture world (`fells`) exercises vassal crisis joining and reversion under tanistry rules.
> - **Extract canonical format:** Extract title rows updated to `title <id> <tier> <liege>`.
> - **Verification:** All 11 checks pass in `selftest_annals_v0.py --check`; all 17 positive controls fail as pinned; inventory chains 588 gates both ways; ruff, format, mypy, and `check_all.sh --part core` pass 100% green.

## 0. Architect review and findings (2026-10-18)

The independent review evaluated AN-1 against the M7 0.6.0 specification and SPR-029 deliverables.

| Finding | Severity | Description | Resolution |
|---|---|---|---|
| F1 (K2 Liege Rule) | PASS | Check 5 verifies that any `INTERREGNUM_CRISIS` on a vassal title occurs only while its liege's interregnum window is open. | Verified in `_check_fixtures` and `History.run()`. |
| F2 (Positive Control) | PASS | Positive control `fixture-vassal-liege-held` plants an illegal vassal crisis after liege succession and verifies refusal by name. | Verified: exits 2 with pinned diagnostic. |
| F3 (Agnatic House Walk) | PASS | Elsa probe in House Ashcombe confirms that agnatic membership rules are distinguished from cognatic alternatives. | Verified in `ashcombe` goldens. |
| F4 (Fells Fixture) | PASS | Fourth fixture world `fells` added with complete chronicle, keyframe, names, and world TOML. | Verified by `gen_fixtures.py --check` and `selftest_annals_v0.py --check`. |
| F5 (AN-0 Fold Compatibility) | PASS | Integration with AN-0's `#segment` folding verified cleanly; `load_chronicle` accepts both single- and multi-segment chronicles. | Verified across all four fixture worlds. |

## 1. Verification results

| Gate / Check | Result |
|---|---|
| `specs/annals/fixtures/gen_fixtures.py --check` | PASS (14 fixture files verified) |
| `specs/annals/selftest_annals_v0.py --check` | PASS (11 checks pass across 4 worlds and 11 groups) |
| Positive control: `fixture-vassal-liege-held` | PASS (exits 2 with `vassal title's crisis while liege is held`) |
| Positive controls suite (17 controls) | PASS (all 17 controls fail with expected diagnostic exit codes) |
| `bin/check_all_selftest.py inventory` | PASS (588 gates chained and listed, each once, both ways) |
| `pytest python/tests/test_check_all_inventory.py` | PASS (20 passed in 1.90s) |
| `ruff check` & `ruff format --check` | PASS (clean across repo) |
| `mypy --strict` | PASS (clean on `specs/annals/selftest_annals_v0.py`) |
| `bin/check_all.sh --part core` | PASS (clean run of all core gates) |
