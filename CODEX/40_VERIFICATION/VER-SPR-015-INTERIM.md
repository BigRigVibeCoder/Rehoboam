---
id: VER-SPR-015-INTERIM
title: "Interim Audit of SPR-015 (Stage A/B checkpoint)"
type: record
status: DRAFT
owner: architect
agents: [architect]
tags: [verification]
related: [SPR-015, REV-SPR-015-births, SPR-016, PRJ-002, BCK-002]
created: 2026-10-18
updated: 2026-10-18
version: 0.3.0
---

> **BLUF: NOT READY. No pass is given.** `sprint/SPR-015-births-f1` (PR #107, head `cda5d16d`) has T-01 to T-04 committed, but **all three CI parts are red on this head**, and CI is fail-fast, so later gates have not run. The red parts have four causes. Three are known and mechanical. The fourth, an incomplete T-01, is a defect in the work, not in a fixture. The Architect has read no code line by line and has run no Rust gate. This record lists what was verified, what was not, and what must change before the audit (`VER-SPR-015`).
>
> Revision 0.3.0 replaces 0.2.0, which was written from the project-manager inbox and the first CI run. It stated checks as done that no one had run. They are removed below.

# VER-SPR-015-INTERIM

## 1. What this record rests on

| Evidence | Source | Verified by the Architect |
|:---|:---|:---:|
| Head `cda5d16d`; commits `f332ead9` (T-04), `5ee789ff`, `3ead3ab2`, `2519e912`, `a61e9195` | `git log` on the remote | Yes |
| CI on `cda5d16d`: core, golden and stream all `failure` (run 39176905920) | `gh run`, the three `check_report.md` artifacts | Yes |
| The two pytest failures | Reproduced in a worktree of `cda5d16d` with system Python (two further failures there come from the machine: no `duckdb`; CI does not show them) | Yes |
| Red-team probe `B09-01` passed 6 of 6 traps | Red-team report in `pm_inbox.md` | **No.** Reported, not re-run |
| Tester interim pass (`VER-SPR-015-TEST`) | None exists on disk | **No such record** |
| The Rust code, the M2 0.3.0 spec text, any Rust gate | — | **No** |

## 2. Task status (claimed by commits, not audited)

| Task | Commit | State |
|:---|:---|:---|
| T-01 spec promotion and Registry delta | `a61e9195` | **Incomplete** (§3, cause 2) |
| T-02 derived index in `rh-core` | `34b5e4f9`, `a61e9195` | Committed, unaudited |
| T-03 fertility F1, `HouseholdId` | `2519e912` | Committed, unaudited |
| T-04 partnering, `apply_marry`, genesis H2 | `f332ead9` | Committed, unaudited |
| Genesis sentinel columns | `5ee789ff`, `cda5d16d` | Committed, unaudited |

The work order's hard checkpoint after Stage A (T-01 to T-03) was not observed: T-04 followed within the hour, with no Tester pass between. GOV-005 §4.2 requires the Tester's interim pass first. **Ruling: the Tester's pass on Stage A is still owed and now covers T-04 as well.**

## 3. Why CI is red (four causes)

1. **Python, `test_registry_effect_extension.py::test_the_fixture_is_the_registry_with_two_edits`.** `a61e9195` changed `specs/manifest/registry_v0.toml`. `tests/registry_effect/registry_absolute_primogeniture.toml` is meant to equal it plus one law, and was not touched. Fix: regenerate the fixture from the Registry (it is a derived copy; check how it was produced before typing it).
2. **Python, `test_lint_odd_frozen_default.py::test_the_probe_moves_a_frozen_default_and_the_next_lint_rejects_it`.** T-01 *copied* M2 0.3.0 to `specs/systems/M2.demography.odd.toml` and left `specs/systems/next/M2.demography.odd.toml` (still `status = "draft"`). The next-version lint then reports `spec_version 0.3.0 must be a semver above the frozen 0.3.0`, and the self-check fails. The work order said *move*. Also not done, and each will fail a gate once this one clears:
   - `next/registry_delta.toml` still holds M2's section and the `BIRTH` and `MARRIAGE` verbs. `lint_odd --next --overlay` already reports `[verbs] entries already in the Registry`.
   - `next/README.md` still describes M2 as a draft (lines 119, 153, 155, 158).
   - `next/amendments.lock.toml` carries M2's amendments. It is regenerated only by `lint_odd.py --next … --write-amendments-lock`, never typed.
3. **Stream, `rh-golden::tail::the_display_form_matches_its_snapshots`.** The solo year-10 keyframe state hash moved: `dc4436a63873b9b9` to `14a9bacfe1c3bbc0`. The five new person columns plausibly explain it, but I have not shown that they alone do. Fix: regenerate with `RH_UPDATE_SNAPSHOTS=1`, then read the diff. Any other changed line is a finding.
4. **Golden, V-PERF-04.** Three regressions against `instructions_v1.txt`: `instant/quiet` (both sizes) 14,306 to 23,230 (+62%), and `instant/year/n10k_l10k` +14.5%. Re-recording is allowed only in the final `[C3] [perf-baseline]` commit, with a `Reason:` line. **I do not accept "expected" for the quiet instant.** A 62% rise on an instant with no work in it means new per-instant cost that is not proportional to births. SPR-038a spent a stage removing exactly that cost, and G1a (`kingdom_10k`, 1,000 years in at most 60 s) is already projected to miss (HANDOFF §8). Required: the developer explains where the 8,900 instructions go before the baseline is re-recorded. If the cause is the O(N) work that 038a removed, it is a defect, not a baseline move.

## 4. Order of work for the developer

1. Finish T-01 (cause 2). Run `python -I specs/systems/lint_odd.py --next specs/systems/next --overlay specs/systems/next/registry_delta.toml`; it must pass.
2. Fix cause 1. Run pytest.
3. Answer cause 4 in the PR body; fix any avoidable cost.
4. Only then regenerate the snapshot and the baselines in the final `[C3] [perf-baseline]` commit.
5. Run all of `bin/check_all.sh` locally (CI stops at the first red gate, so these are the failures seen so far, not all).
6. Request the Tester's pass (different model from the developer's; GOV-011 §4), then the Architect's audit.

## 5. Not yet decided

- Probe `B09-01` and the Batch 09 dossier exist only on `arch/red-team-batch-04` (43 commits ahead of `main`, no PR). `main` has neither file, and no gate in `bin/check_all.sh` or the workflows runs a probe from `tests/probes/`. A pass reported by message protects nothing once the code changes. Filed as BCK-002 A-80.
- Sign-off of the keyframe hash change: pending cause 3.
