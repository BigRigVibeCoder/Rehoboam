# Work Order: Exuberant (AGT-002-CORE) — PR #88 Final Check 89 Fix & SPR-018 Handoff

**Role:** Core Engine Developer (`AGT-002-CORE`)  
**Assigned Agent:** `Exuberant`  
**Working Directory:** `/home/bdavidriggins/Documents/Rehoboam`  
**Current Branch:** `sprint/SPR-016-mortality-f1` (Stay here until PR #88 is merged!)  
**PR:** [#88 (SPR-016: M2 F1 Mortality)](https://github.com/BigRigVibeCoder/Rehoboam/pull/88)  
**Date:** 2026-10-20  
**Author:** Lead Architect (`AGT-001`)  

---

## 1. Urgent Action: PR #88 CI Gate 89 Fix (2-Line Fix)

### 1.1 Why PR #88 Has Not Merged Yet
In CI Run 39355723911:
- `part stream`: **SUCCESS** (all golden ladders passed!)
- `part core`: **SUCCESS**
- `part golden`: **FAILED on only Check 89** (`q15_of_10000_newborns_is_inside_the_binomial_band_for_32_seeds`).

### 1.2 Root Cause
In `crates/rh-worldgen/tests/m2_system.rs` (lines 442 and 452), the test computes the expected null binomial band using `m2.schedule().survivors_at(...)`.
- `m2.schedule()` is preserved as the **legacy Rung-0 linear schedule** (expecting ~3,473 deaths per 10k founders by age 15).
- The simulation engine actually runs against **`m2.f1().survivors_at(...)`** (which yields ~3,014 deaths under the F1 schedule at e0=30).
- Under SPR-016 contract ruling **W18**, the null expectation must evaluate against the F1 schedule!

### 1.3 Exact Steps to Fix

#### Step 1: Update `crates/rh-worldgen/tests/m2_system.rs`
In test `q15_of_10000_newborns_is_inside_the_binomial_band_for_32_seeds`:

1. At **line 442**:
   ```rust
   // Replace:
   let l15 = m2.schedule().survivors_at(15 * year).unwrap() as f64;
   // With:
   let l15 = m2.f1().survivors_at(15 * year).unwrap() as f64;
   ```

2. At **line 452**:
   ```rust
   // Replace:
   let lx = m2.schedule().survivors_at(-b).unwrap() as f64;
   // With:
   let lx = m2.f1().survivors_at(-b).unwrap() as f64;
   ```

#### Step 2: Verify Gate 89 Locally
Run:
```bash
cargo test -p rh-worldgen --test m2_system q15_of_10000_newborns --release
```
Confirm the test passes (worst |z| <= 3.29 and pooled z passes).

#### Step 3: Commit and Push
```bash
git add crates/rh-worldgen/tests/m2_system.rs
git commit -m "test(SPR-016): evaluate q(15) binomial null band against F1 schedule [C1]

Why: Check 89 in part golden compares observed deaths under the F1 mortality
model against the schedule survivors. In SPR-016, m2.schedule() was kept as
legacy rung-0 while m2.f1() holds the active F1 schedule, causing the null
band calculation to expect rung-0 mortality (~3473 deaths) instead of F1 (~3014).

What: Use m2.f1().survivors_at in q15_of_10000_newborns test per W18.

Refs: SPR-016, W18, GOV-002"
git push origin sprint/SPR-016-mortality-f1
```

---

## 2. Next: What Happens After You Push

1. As soon as you push commit [C1], CI on PR #88 will run and turn 100% green across all three parts (`core`, `golden`, `stream`).
2. The Architect will immediately merge PR #88 into `main` with a merge commit.
3. Once the Architect merges PR #88, you will receive confirmation to switch to `main`, pull, and branch `sprint/SPR-018-worldgen-lite`:
   ```bash
   git fetch origin
   git checkout -b sprint/SPR-018-worldgen-lite origin/main
   ```
