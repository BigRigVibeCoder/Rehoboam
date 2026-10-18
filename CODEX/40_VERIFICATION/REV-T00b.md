---
id: REV-T00b
title: "Independent review of T-00b: M7 0.6.0 amendments for SPR-022 and instant log model scenarios (F-10, F-19/K19, F-20/K12, F-25, MIN-10, Q6)"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [EVO-002, SPR-021, SPR-022, SPR-023, REV-T00a, REV-WAVE7, PRJ-002, BCK-002]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing review (2026-10-18).** T-00b completes the preparatory M7 0.6.0 amendments and instant-log model scenarios required for SPR-022:
> - **K12 / F-20 (Year Pass Ordering):** At a year's first instant, `Year`/`Polity` runs before `Day`/`Polity`. The `Year` pass sees a holder dead this instant and does not re-grant or send on their behalf; `on_death` arbitrates in the subsequent `Day` pass. Verified by scenario `year_start_dead_holder` and positive control `day-before-year`.
> - **K19 / F-19 & X13h (Vocation Timing):** `VOCATION_DUE` timers fire in `Polity`'s timer pass at majority date, before bindings. Married persons (`person.spouse` at `same_instant`) and dead persons are excluded from drawing. Verified by scenario `vocation_majority_day` and positive controls `vocation-yearly` and `married-draws`.
> - **MIN-10 & F-10 (Regency End & World Start):** World-start pass names a genesis minor's regent without an edge; `regency_end` declared as a no-draw purpose whose re-sample keeps its own tick. Verified by scenario `regency_genesis_minor` and positive control `regency-redrawn`.
> - **Q6 (Exited Persons Skipped):** Persons in `exited()` are skipped for Promote, Demote, regrant, and vocation. Verified by positive control `exited-not-skipped`.
> - **Amendments Lock:** `specs/systems/next/amendments.lock.toml` locks 40 amendments for M7 0.6.0 (total 107 across M2, M7, and T1).
> - **Model Verification:** `instant_log_model.py` passes all 14 scenarios; 350-line transcript and 5,130-line vectors verified exact against goldens; all 5 T-00b controls and all 8 T-00a controls fail as pinned; inventory chains 593 gates both ways; `check_all.sh --part core` passes 100% green.

## 0. Architect review and findings (2026-10-18)

The independent review evaluated T-00b against EVO-002, M7 0.6.0, and SPR-022 requirements.

| Finding | Severity | Description | Resolution |
|---|---|---|---|
| F1 (K12 Year Pass) | PASS | `Year`/`Polity` runs before `Day`/`Polity` at year start; dead holder seen by `Year` pass before arbitration. | Verified by scenario `year_start_dead_holder` and `day-before-year` control. |
| F2 (K19 Vocation Due) | PASS | Four `VOCATION_DUE` timers fire in timer pass at majority date; marriage and death exclusions enforced. | Verified by scenario `vocation_majority_day` and `vocation-yearly` / `married-draws` controls. |
| F3 (MIN-10 Regency End) | PASS | `regency_end` defined as no-draw RNG purpose; re-sample preserves timer tick. | Verified by scenario `regency_genesis_minor` and `regency-redrawn` control. |
| F4 (Q6 Exited Skipping) | PASS | Persons in `exited()` skipped for Demote/Promote/vocation. | Verified by `exited-not-skipped` control. |
| F5 (Amendments Lock) | PASS | Next spec amendments lock updated to 107 entries (46 M2, 40 M7, 21 T1). | Verified by `lint_odd.py --next` self-check. |
| F6 (Model Goldens) | PASS | Transcript (350 lines) and vectors (5,130 lines) match exact references. | Verified by `instant_log_model.py --check`. |

## 1. Verification results

| Gate / Check | Result |
|---|---|
| `specs/engine/instant_log_model.py --check` | PASS (all 14 scenarios complete/refused as expected; goldens match) |
| Positive control: `day-before-year` | PASS (fails with `YEAR PASS (K12): the Year pass at day 360`) |
| Positive control: `exited-not-skipped` | PASS (fails with `SKIP (Q6): M7 sent Demote for P6`) |
| Positive control: `vocation-yearly` | PASS (fails with `VOCATION: P21's vocation was written at day 360`) |
| Positive control: `married-draws` | PASS (fails with `VOCATION: P22, married at the fire`) |
| Positive control: `regency-redrawn` | PASS (fails with `RESAMPLE: REGENCY_END about P40 moved from day 9`) |
| `specs/systems/lint_odd.py --next` | PASS (M7 40 amendments locked, all self-checks pass) |
| `bin/check_all_selftest.py inventory` | PASS (593 gates chained and listed both ways) |
| `pytest python/tests/test_check_all_inventory.py` | PASS (20 passed in 1.91s) |
| `ruff check` & `ruff format --check` | PASS (clean across repo) |
| `mypy --strict` | PASS (clean across repo) |
| `bin/check_all.sh --part core` | PASS (all core gates green) |
