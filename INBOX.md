# Work Order: Exuberant (AGT-002-CORE) — SPR-016 Stage 3 (Merge Conclusion & Final Verification)

**Role:** Core Developer (Rust Core Engine)  
**Assigned VM:** `exuberant`  
**Current Branch:** `sprint/SPR-016-mortality-f1` (PR #88)  
**Date:** 2026-10-19  
**Author:** Lead Architect (`AGT-001`)  

---

## 1. Incident Root Cause & Architect Remediation

During your previous session, your execution was halted after 6 consecutive tool errors:
- **Root Cause:** A syntax error occurred in tool calling where shell commands were prefixed with `>` (e.g. `>cargo check ...`) and passed with a second array element `"description"`. The shell interpreted `>` as an output redirection, creating accidental empty files named `cargo`, `cd`, `grep`, `python3`, `rm` and reporting `description: command not found` (exit code 127).
- **Remediation Applied by Architect:**
  1. Removed all accidental files (`cargo`, `cd`, `grep`, `python3`, `rm`).
  2. Patched all 5 downstream `Demography::new` call sites across the workspace to match the 8-argument signature (`density_capacity: 10_000_000`, `density_mortality_slope: Q16_16::ZERO`).
  3. Resolved all Clippy warnings and disallowed `unwrap()` calls in `demography.rs`.
  4. Ran `cargo fmt` cleanly across all modified crates.
  5. Staged all resolved conflicts and fixes into git (`git add -u`).

---

## 2. Current Verified State

The workspace is in an advanced, near-complete state:
- ✅ **Upstream Main Merge:** PR #107 (SPR-015: Births & Households) and PR #123 (SPR-027: Stream Materializer) are merged into working tree. All conflicts resolved and staged.
- ✅ **Python Verification:** `python/tests/test_mutant_features.py` indentation fixed. `ruff check`, `ruff format --check`, `mypy --strict`, and `pytest` are **100% green**.
- ✅ **Rust Compilation & Linting:**
  - `cargo check --tests --workspace` passes with **0 errors and 0 warnings**.
  - `cargo clippy --workspace --all-targets -- -D warnings` passes cleanly.
  - `cargo fmt --check` passes cleanly.
- ✅ **Test Coverage:**
  - `partnering_tests` (7 passed).
  - `schedule_f1_golden` (5 passed).
  - `rh-core` (63 passed).
  - `rh-contracts` (all passed).
  - `demography.rs` implements `inv_m2_08` and `inv_m2_09`.
  - `household_on_death` is verified in `apply_death`.

---

## 3. Action Steps to Execute

### Step 1: Conclude the Merge Commit
All merge conflicts and fixes are already staged in git. Conclude the merge:
```bash
cd /home/bdavidriggins/Documents/Rehoboam
git commit -m "chore(SPR-016): merge origin/main and resolve M2 F1 constructor parity [C1]"
```

### Step 2: Handle F1 Draws Golden Fold (Task T-03 / W19)
Note that `crates/rh-sys-demography/tests/draws_golden.rs` is the old Rung-0 test where founders > 90 face linear mortality to 100. Under F1 (MR8), founders > 90 face logistic mortality to 120.
- Per SPR-016 Task T-03 and ruling W19: the draws golden is folded into the F1 model (`f1_draws_golden.rs` / `specs/demography/golden/m2_f1_mortality.txt`).
- Confirm `python3 -I specs/demography/selftest_m2_f1_mortality.py` passes cleanly (it generates 21,109 lines of golden vectors bit-for-bit).

### Step 3: Run Verification
Ensure your environment includes Rust and Python tools:
```bash
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
cd /home/bdavidriggins/Documents/Rehoboam
cargo check --tests --workspace
cargo test -p rh-sys-demography --test partnering_tests
cargo test -p rh-sys-demography --test schedule_f1_golden
python/.venv/bin/python -I -m ruff check --config python/pyproject.toml --no-cache .
```

### Step 4: Push to Origin
Once verification succeeds, push your branch:
```bash
git push origin sprint/SPR-016-mortality-f1
```

---

## 4. Critical Command Execution Rules for Cline
1. **Never prefix commands with `>`**. Do not type `>cargo check`.
2. **Proper JSON command format:** Pass commands as a single object with `"command": "..."` and `"description": "..."`, or run commands directly without extra strings in the array.
3. If cargo or rust tools report command not found in your subshell, prefix with `export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH" && ...`.
