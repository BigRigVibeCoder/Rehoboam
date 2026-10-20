# Work Order: Adjutant (AGT-002-TOOLS) — SPR-028a Stage A (Tasks T-01 Close & T-02 Dispatch)

**Role:** Tools & Validation Developer (`AGT-002-TOOLS`)  
**Assigned Agent:** `Adjutant`  
**Current Branch:** `spec/SPR-028-validation-contract` ([PR #124](https://github.com/BigRigVibeCoder/Rehoboam/pull/124))  
**Date:** 2026-10-19  
**Author:** Lead Architect (`AGT-001`)  

---

> [!IMPORTANT]
> ### MULTI-AGENT ENVIRONMENT CONCURRENCY WARNING
> **You are NOT the only agent active on this machine.** There are currently **TWO (2) active developer agents** operating concurrently:
> 1. **Adjutant (YOU - `AGT-002-TOOLS`):** Owning the tooling, validation harness, and calibration plane on branch `spec/SPR-028-validation-contract` (SPR-028).
> 2. **Exuberant (`AGT-002-CORE`):** Owning the Rust engine core demography systems on branch `sprint/SPR-016-mortality-f1` (PR #88 / SPR-016).
> 
> **Strict Concurrency Rules (GOV-009, GOV-011 §2, HANDOFF.md §1):**
> - **Branch Isolation:** Only push to and operate within your own branch (`spec/SPR-028-validation-contract`). Never touch, checkout, modify, or commit to `sprint/SPR-016-mortality-f1` or `main`.
> - **Shared Resource Discipline:** Cargo `target/` directories require 8–11 GB each. Never run heavy cargo builds or full `bin/check_all.sh` concurrently with Exuberant. Run focused package checks (`cargo nextest run -p ...`, python selftests, and targeted gates).
> - **Unstaged Changes:** Do not run `git reset --hard` or `git clean -fd` indiscriminately across the repository; check `git status` first to ensure you do not destroy sibling agent work in flight.

---

## 1. Task T-01 Formal Review Status: APPROVED (1.0.0)

The Lead Architect has completed the independent review of your Task T-01 deliverables under commit `730bac2`:
- **Review Record:** Committed to [`CODEX/40_VERIFICATION/REV-SPR-028-harness.md`](file:///home/bdavidriggins/Documents/Rehoboam/CODEX/40_VERIFICATION/REV-SPR-028-harness.md) (Status: **APPROVED 1.0.0**).
- **Sprint Document:** [`CODEX/05_PROJECT/SPR-028_ValidationHarness.md`](file:///home/bdavidriggins/Documents/Rehoboam/CODEX/05_PROJECT/SPR-028_ValidationHarness.md) updated with Task T-01 marked **`[x] done`**.
- **Architect Commits Merged:** Commit `77685889` (Review doc & SPR-028 update) and your commit `48cc724b` (lint_forcing refactor & ruff/mypy configs) have been merged and pushed to `origin/spec/SPR-028-validation-contract` at commit `315ecea8`.

---

## 2. Immediate Action Required: PR #124 Stream CI Forward-Fix

CI on PR #124 is green on `core` and `golden`, but failed on `stream` at gate 8:
```text
FAIL rh-golden by name: tail prints every record once in seq order from any cursor... (SPR-013 T-10)
SNAPSHOT hamlet_min_tellability_100 differs at line 1:
-     76 0001-04-04 00:00 DEATH target Job's servant 5 (natural) [event #0]
+     61 0000-01-20 00:00 MARRIAGE actor #d8521cbfd69fb647 target Job's servant 6 [event #0]
rerun with RH_UPDATE_SNAPSHOTS=1 and review the diff
```

### Forensic Root Cause
This is an **upstream regression on `origin/main`** caused by the merge of PR #107 (SPR-015: Births & Households). PR #107 introduced the M2 partnering and marriage system, which legitimately emits a new `MARRIAGE` event at tick 61 in `hamlet` with tellability $\ge 100$. PR #123 merged subsequently without updating the committed snapshot. Because `main` moves only by green PR (GOV-009 §6, GOV-010), this snapshot must be updated on your branch.

### Resolution Steps
1. Make sure you are on `spec/SPR-028-validation-contract` and pulled to head:
   ```bash
   git checkout spec/SPR-028-validation-contract
   git pull origin spec/SPR-028-validation-contract
   ```
2. Regenerate the snapshot using the committed test tool:
   ```bash
   RH_UPDATE_SNAPSHOTS=1 cargo nextest run -p rh-golden --release --locked --test tail
   ```
3. Inspect the diff to confirm that only line 1 of `crates/rh-golden/tests/snapshots/hamlet_min_tellability_100.txt` changes:
   ```bash
   git diff crates/rh-golden/tests/snapshots/hamlet_min_tellability_100.txt
   ```
4. Commit and push:
   ```bash
   git add crates/rh-golden/tests/snapshots/hamlet_min_tellability_100.txt
   git commit -m "fix(stream): update hamlet tail snapshot for SPR-015 marriage events [C1]"
   git push origin spec/SPR-028-validation-contract
   ```
5. Confirm CI on PR #124 turns green, then convert PR #124 from Draft to Ready for Review.

---

## 3. Dispatch: Task T-02 Implementation (`rehoboam_calib`)

You are authorized to begin **Task T-02** (`rehoboam_calib`: the statistics, the readers, the band evaluator and the report) per [`CODEX/05_PROJECT/SPR-028_ValidationHarness.md`](file:///home/bdavidriggins/Documents/Rehoboam/CODEX/05_PROJECT/SPR-028_ValidationHarness.md#t-02-rehoboam_calib-the-statistics-the-readers-the-band-evaluator-and-the-report-g11-g14-g18-15-days).

### 3.1 Scope & Package Architecture (`python/rehoboam_calib/`)
Port and package the ratified standard-library reference implementation into the production calibration library:
- **`rehoboam_calib.stats`**: Production port of statistical primitives verified in T-01 (`ibeta`, Student-t quantiles, TOST zero-variance, Clopper–Pearson, Wilson score, exact Poisson, Kaplan–Meier Greenwood SE, Breslow-tied stratified Cox with F-25 non-identification, logistic regression, SHA-256 keyed bootstrap, expected rate $r_{\text{exp}}$, expected spread $s(y)$).
- **`rehoboam_calib.read`**: Run directory reader (Channel A via DuckDB over sealed Parquet segments; `LIFE_ENTRY`, `LIFE_EXIT`, `EVENT`, `STEP_DIGEST`; parse `--vitals` lines **by key**; Ruling G11: strictly channel A and vitals, **no keyframe reader**).
- **`rehoboam_calib.bands`**: Machine-readable evaluator reading [`specs/validation/harness_v0.toml`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/harness_v0.toml) / [`bands_v0.toml`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/bands_v0.toml), computing per-band statistics, and enforcing the sealed rule (emitting `SEALED` for V-SF-06c and V-SF-07 with zero numerical leak).
- **`rehoboam_calib.report`**: Formatter emitting the canonical `rehoboam/m9/report/v0` format with trailing SHA-256 digest, plus Markdown summaries.
- **CLI Entry Point**:
  ```bash
  python -I -m rehoboam_calib report --runs <dir>... --bands <ids> --out <dir>
  ```

### 3.2 Acceptance Criteria for T-02
1. **Parity with Reference:** Running `rehoboam_calib report` on T-01's synthetic ensemble must match [`specs/validation/golden/harness_v0.txt`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/golden/harness_v0.txt) byte-for-byte.
2. **V-SCALE-02 Rungs:** Run `report` on the 200-year stream run directories of `solo` ($N=1$), `household` ($N=6$), and `hamlet` ($N=60$). Every band must report either `INSUFFICIENT` or a valid evaluated result; no band may pass or fail below its minimum sample threshold $N_{\min}$.
3. **Independent Real-Run Parity:** On `hamlet`'s 200-year run directory, verify that computed births, deaths, and age-at-death distributions match independent raw DuckDB queries.
4. **Determinism:** Reports generated under different `PYTHONHASHSEED` values must be bit-identical. Execution must use `python/.venv/bin/python -P -s -m rehoboam_calib ...` (PY-PKG review P3).
5. **Code Quality:** `ruff check`, `ruff format --check`, and `mypy --strict` completely clean across `python/rehoboam_calib/`.
