# INBOX: Work Order for Exuberant (AGT-002-CORE) — SPR-016 Stage 3 Completion

**Role:** Core Engine Developer (`AGT-002-CORE`)  
**Assigned Agent:** `Exuberant`  
**Target Branch:** `sprint/SPR-016-mortality-f1` ([PR #88](https://github.com/BigRigVibeCoder/Rehoboam/pull/88))  
**Author:** Lead Architect (`AGT-001`)  
**Date:** 2026-10-19  

---

## 1. Status & Acknowledgment

Outstanding work on the previous pass:
- ✅ Clean resolution of the 10-file conflict across 5 crates (unifying `Demography` with density + fertility parameters).
- ✅ Clean fix for Python lint indentation in `python/tests/test_mutant_features.py:30`.
- ✅ Verification passing across workspace compiles, core unit tests, and `selftest_m2_f1_mortality.py`.
- ✅ Implementation of H4 household dissolution, `INV-M2-08` (marriage exclusivity/symmetry), and `INV-M2-09` (household consistency).

Your analysis regarding the `draws_golden` failure is **100% accurate**: expanding `OWNED` from 19 to 33 columns and integrating the F1 mortality schedule shifted random draws, necessitating a standard `[C3]` golden baseline regeneration per GOV-009.

---

## 2. CI Context: Stream Part & `main`

The recent CI failure on `part stream` (`Stream part input`) was caused by two factors:
1. The un-regenerated golden baseline for SPR-016.
2. An upstream stream snapshot mismatch inherited from `main` at `c452fd8` (an inter-PR race between PR #107 and PR #123). PR #124 (`spec/SPR-028`) has fixed this snapshot in commit `711f371183` and is currently finalizing CI to restore `main`.

---

## 3. Action Plan: Step-by-Step

### Step 1: Sync with `origin/main`
Ensure you pick up the healed stream snapshots from `main`:
```bash
git fetch origin
git merge origin/main
```
*(If PR #124 has just merged, this will cleanly bring in the updated `hamlet` stream snapshots).*

### Step 2: Regenerate Golden Baseline (`[C3]`)
Regenerate `m2_draws_v1.txt` and the golden ladders using the canonical tool:
```bash
bin/regen_baseline.sh
```
Verify the changes with `git diff crates/rh-sys-demography/`.

Commit the regenerated baselines using strict GOV-009 conventions (including the `[perf-baseline]` / `Reason:` tag required by `bin/perf_gate.py`):
```bash
git commit -m "feat(SPR-016): regenerate m2_draws_v1 golden baseline for F1 mortality [C3] [perf-baseline]" \
  -m "Why: OWNED column list expanded from 19 to 33 columns and F1 mortality schedule shifts random draws." \
  -m "Reason: OWNED column expansion to 33 columns and F1 mortality schedule shifts random draws." \
  -m "What: Regenerated m2_draws_v1.txt and golden ladders via bin/regen_baseline.sh." \
  -m "Refs: SPR-016, GOV-009, PR #88"
```

### Step 3: Run Local Fail-Fast Checks
Run the PR tier gates locally to verify clean passes before pushing:
```bash
# 1. Verify targeted tests first
cargo test -p rh-sys-demography --lib
cargo test -p rh-sys-demography --test draws_golden
python3 -I specs/demography/selftest_m2_f1_mortality.py

# 2. Run PR-tier checks
bin/check_all.sh --part core
bin/check_all.sh --part golden
bin/check_all.sh --part stream
```

### Step 4: Push & Mark Ready
Push to `origin/sprint/SPR-016-mortality-f1`:
```bash
git push origin sprint/SPR-016-mortality-f1
```
Once pushed:
1. Update the PR #88 description to indicate Stage 3 completion.
2. Mark PR #88 as ready for review.
3. The Architect will commission independent Tester verification (`AGT-003-DET`) and perform the final audit (`VER-SPR-016.md`).
