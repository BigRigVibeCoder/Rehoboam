# Work Order: SPR-015 Stage B Completion & CI Remediation

- **Assignee:** Developer Exuberant (`AGT-002-CORE`)
- **Assigner:** Architect (`AGT-001`)
- **Date:** 2026-10-18
- **Priority:** Priority 1 (Crucible Critical Path)
- **Branch:** `sprint/SPR-015-births-f1` (PR #107)
- **Current Head:** `5ee789ff6f275573d29d5ec88749cc25855d84f2`
- **Change Class:** **C3 `[perf-baseline]`**

---

## 1. Executive Summary

Your Stage A work (T-01..T-03) and Genesis sentinel column updates (`5ee789ff`) have been audited by the Architect and verified by the Red Team. Red Team probe `B09-01` (`probe_fertility_adversarial_traps.py`) ran against your implementation and passed all 6 adversarial traps.

CI run 39176612229 on PR #107 encountered 3 specific, isolated failures. This work order specifies the exact fixes required to turn PR #107 green and complete Stage B.

---

## 2. Immediate CI Fixes Required

### Fix 1: Synchronize Registry Fixture (Part Core)
- **File:** `tests/registry_effect/registry_absolute_primogeniture.toml`
- **Issue:** `python/tests/test_registry_effect_extension.py:37` fails because `specs/manifest/registry_v0.toml` was updated with `BIRTH`, `MARRIAGE`, and F1 parameters (`tfr`, `elite_fertility_mult`, `density_capacity`, `density_mortality_slope`), but the fixture copy was not updated.
- **Action:** Copy the additions made to `specs/manifest/registry_v0.toml` in commit `a61e9195` into `tests/registry_effect/registry_absolute_primogeniture.toml` so that stripping `[succession_laws.absolute_primogeniture]` leaves `registry_v0.toml` byte-identical.
- **Verification:** Run `pytest -c python/pyproject.toml python/tests/test_registry_effect_extension.py`. Must PASS.

### Fix 2: Update Solo Story Snapshot (Part Stream)
- **File:** `crates/rh-golden/tests/snapshots/solo_story.txt`
- **Issue:** Gate 8 (`rh-golden::tail the_display_form_matches_its_snapshots`) panicked because the solo year-10 keyframe state plane-A hash moved:
  ```diff
  -    121 0009-12-30 23:59 KEYFRAME keyframes/year-0010.rhkf state dc4436a63873b9b9
  +    121 0009-12-30 23:59 KEYFRAME keyframes/year-0010.rhkf state 14a9bacfe1c3bbc0
  ```
- **Action:** Rerun the test with snapshot updating enabled:
  ```bash
  RH_UPDATE_SNAPSHOTS=1 cargo nextest run -p rh-golden --release --test tail
  ```
  Commit the updated `crates/rh-golden/tests/snapshots/solo_story.txt`.

### Fix 3: Re-record Instruction Count Baselines (Part Golden)
- **File:** `crates/rh-bench/baselines/instructions_v1.txt`
- **Issue:** Gate 9 (`V-PERF-04`) reported expected instruction regressions in `instant/quiet` and `instant/year` due to M2 demography passes.
- **Action:** In your final `[C3] [perf-baseline]` commit, re-record the instruction baseline:
  ```bash
  python -I bin/perf_gate.py record
  ```
  Include `Reason: SPR-015 M2 F1 births and household passes` in the commit body.

---

## 3. Stage B Task Completion (Task T-04)

Complete the remaining Stage B deliverables:
1. **Partnering (N1–N7):**
   - Annual marriage eligibility evaluation for unmarried adults $\ge 16$.
   - Marriage rate and spouse pairing from `rh-sample`.
   - `apply_marry` command application.
2. **Household Accounting (H3/H4):**
   - Creation of new `HouseholdId` upon marriage when leaving natal household.
   - Dissolution of sole-occupant households on death (`INV-M2-09`).
   - Clean handling of immediate ID reuse without aliasing.
3. **Run Full Gates:**
   - Execute `bin/check_all.sh` across all parts.
   - Verify `python -I bin/check_all_selftest.py inventory`.

---

## 4. Final Commit & Merge Protocol (GOV-009)

When all tests pass locally:
1. Stage all changes.
2. Commit with title:
   ```
   feat(SPR-015): complete Stage B births & households [C3] [perf-baseline]
   ```
3. Push to `sprint/SPR-015-births-f1`.
4. Ensure PR #107 turns green across all 3 CI jobs (`core`, `golden`, `stream`).
5. Hand off to Architect for final merge.
