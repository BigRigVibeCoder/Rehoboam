# Work Order: Agent 2 (AGT-002-CORE) — SPR-016 Final Merge & PR #88 Green CI

**Role:** Core Engine Developer (`AGT-002-CORE`)  
**Assigned Agent:** `Agent 2` / `Exuberant`  
**Target Branch:** `sprint/SPR-016-mortality-f1` ([PR #88](https://github.com/BigRigVibeCoder/Rehoboam/pull/88))  
**Assigned Worktree:** `.claude/worktrees/agent-2-spr016` (Mandatory; see §1)  
**Date:** 2026-10-19 / 2026-10-20  
**Author:** Lead Architect (`AGT-001`)  

---

> [!IMPORTANT]
> ### CRITICAL MULTI-AGENT ENVIRONMENT CONCURRENCY RULES
> **You are sharing this physical machine with sibling developer agent Agent 1 (`AGT-002-TOOLS`):**
> 1. **MANDATORY WORKTREE ISOLATION:**
>    - You **MUST** work strictly inside your dedicated worktree:
>      `/home/bdavidriggins/Documents/Rehoboam/.claude/worktrees/agent-2-spr016`
>    - **NEVER** run `git checkout` in the main repo root (`/home/bdavidriggins/Documents/Rehoboam`).
> 2. **NO HEAVY BUILDS IN PARALLEL:**
>    - Coordinate with Agent 1 before running full workspace release builds to avoid running out of RAM/disk.

---

## 1. Executive Briefing: Stage 3 Implementation Complete & Upstream Healed

### 1.1 What Has Been Accomplished
- **Stage 3 F1 Mortality & Obituaries:** Complete! (T-03, T-04, T-05; MR10–MR12 monthly Bernoulli trials, density multiplier, age-in-minutes DEATH payload).
- **H4 Partnering & Marriages:** Complete! (Household dissolution and marriage closure upon spouse death).
- **Sorted Invariant Bugfix:** Discovered and fixed by Exuberant: replaced `Vec::push()` with `binary_search_by_key()` sorted insertion for new households, strictly enforcing `INV-M2-09`.
- **Golden Baselines Regenerated:** `solo`, `household`, and `hamlet` yearly ladders regenerated via `bin/regen_baseline.sh` and committed with canonical `[perf-baseline]` / `Reason:` tags.
- **CI Status:** Gate 2 (`Stream part input`) and Gate 7 (`V-STR-01`) passed in CI!

### 1.2 The Gate 8 Blocker Has Been Healed on `main`
- The *only* failing gate in PR #88 CI was Gate 8 (`FAIL rh-golden by name: tail prints every record once in seq order... SNAPSHOT hamlet_min_tellability_100 differs at line 1`).
- As diagnosed by the Architect, this was **not** a flaw in SPR-016. It was an upstream regression on `origin/main` at `c452fd88` caused by an inter-PR merge race between PR #107 and PR #123.
- **PR #124 has just landed on `main` (`775f939105aa1867a0f5338aefd74f75574acc16`)!**
- It updated the committed tail snapshots. `origin/main` is now healthy, 100% green, and ready for you to absorb.

---

## 2. Action Plan: Final Steps to Green PR #88

Follow these exact steps inside your worktree:

### Step 1: Navigate to Worktree & Fetch Remote
```bash
cd /home/bdavidriggins/Documents/Rehoboam/.claude/worktrees/agent-2-spr016
git fetch origin
```

### Step 2: Merge Healed `origin/main`
Merge `origin/main` into your branch:
```bash
git merge origin/main
```
> [!NOTE]
> All code files, golden snapshots, and manifests will merge automatically and cleanly.
> If Git pauses on a merge conflict in `INBOX.md`, resolve it simply with:
> ```bash
> git checkout --ours INBOX.md
> git add INBOX.md
> git commit --no-edit
> ```

### Step 3: Run Local Fail-Fast Verification
Verify that the merged snapshots and all gates pass locally:
```bash
# 1. Target tests
cargo test -p rh-sys-demography --lib
cargo test -p rh-sys-demography --test draws_golden
python3 -I specs/demography/selftest_m2_f1_mortality.py

# 2. Gate 8 tail snapshots check (should now PASS cleanly)
cargo nextest run -p rh-golden --release --locked --test tail

# 3. Fast PR-tier checks
bin/check_all.sh --part core
bin/check_all.sh --part stream
```

### Step 4: Push to PR #88
Push the merge commit to GitHub:
```bash
git push origin sprint/SPR-016-mortality-f1
```

### Step 5: Mark PR #88 Ready for Review
```bash
gh pr ready 88
```

---

## 3. What Comes Next
Once PR #88 CI turns 100% green:
1. Update the PR #88 summary body documenting Stage 3 completion.
2. The Lead Architect will commission the independent Determinism Tester (`AGT-003-DET`) for verification report `VER-SPR-016-TEST.md`.
3. The Lead Architect will write the final audit `VER-SPR-016.md` and merge PR #88 with an immutable merge commit.
4. SPR-016 will be closed and work will transition to Wave 6 / SPR-021a.
