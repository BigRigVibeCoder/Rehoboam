---
id: REV-SPR-028-calib
title: "Independent review of PR #124: python/rehoboam_calib/, the M9 production calibration and reporting library (SPR-028 T-02)"
type: record
status: APPROVED
owner: architect
agents: [architect]
tags: [verification, validation, coding]
related: [SPR-028, PRJ-002, GOV-002, GOV-003, ADR-0024, ADR-0026, ADR-0033]
created: 2026-10-19
updated: 2026-10-20
version: 1.0.0
---

> **BLUF: APPROVED by the Lead Architect on 2026-10-20 (Commit `d746a5b` / PR #124). Task T-02 deliverables meet all contract, mathematical, and acceptance criteria.**
> - Production calibration library `python/rehoboam_calib/` implements full statistical, reading, band evaluation, and reporting capabilities strictly conforming to `specs/validation/harness_v0.md` (WP P1.10).
> - Pure standard-library mathematical primitives (`rehoboam_calib.stats`) pass all closed-form proofs (machine epsilon $10^{-12}$) and replicate the verified reference behaviors of T-01.
> - Channel A reader (`rehoboam_calib.read`) correctly reads sealed Parquet segments via DuckDB (`LIFE_ENTRY`, `LIFE_EXIT`, `STEP_DIGEST`), extracts `--vitals` stdout logs strictly by key (Ruling G11), and enforces the absence of keyframe readers.
> - Band evaluator (`rehoboam_calib.bands`) enforces the sealed rule (V-SF-06c and V-SF-07 yield zero numerical leak; status `SEALED`) and enforces minimum sample thresholds ($N_{\min}$), reporting `INSUFFICIENT` on sub-threshold cohorts.
> - Comprehensive test suite `python/tests/test_calib.py` (20 unit/integration tests) passes 100% clean under `pytest`, `ruff check`, `ruff format --check`, and `mypy --strict`. All tests confirm bit-identical determinism under varying `PYTHONHASHSEED`.

---

# REV-SPR-028-calib: Independent Architect Review

## 1. Scope & Verification Target

- **Sprint:** `CODEX/05_PROJECT/SPR-028_ValidationHarness.md` (Stage A: Task T-02).
- **Target Branch / PR:** `spec/SPR-028-validation-contract` / PR #124.
- **Reviewed Head Commit:** `d746a5b` (`test(SPR-028): skip real-run tests when target/golden absent in core part [C1]`), merged into `main` at `775f939`.
- **Change Class:** C1/C2 (Python tools plane: library, tests, CLI entry points; engine plane A untouched).
- **Reviewer:** Lead Architect (`AGT-001`).

---

## 2. Deliverable Verification

| Item | Expected Specification | Delivered Implementation | Status |
|:---|:---|:---|:---|
| **Package Structure** | `python/rehoboam_calib/` uv package layout | `__init__.py`, `__main__.py`, `stats.py`, `read.py`, `bands.py`, `report.py`, `py.typed` | **PASS** |
| **Statistical Primitives** | T-01 reference parity (Student-t, TOST, Clopper–Pearson, Wilson, Poisson, Kaplan–Meier, Cox, logistic regression, SHA-256 bootstrap, $r_{\text{exp}}$, $s(y)$) | `rehoboam_calib.stats` (839 lines); zero external C/math dependencies; machine epsilon parity with reference | **PASS** |
| **Channel A Reader** | DuckDB over sealed Parquet segments; vitals parsed by key; no keyframe reader (G11) | `rehoboam_calib.read` (368 lines); queries `LIFE_ENTRY`, `LIFE_EXIT`, `STEP_DIGEST`; extracts vitals by dictionary key | **PASS** |
| **Band Evaluator** | Sealed rule enforcement; sample minimums ($N_{\min}$); `specs/validation/harness_v0.toml` loader | `rehoboam_calib.bands` (454 lines); emits `SEALED` for V-SF-06c/07; emits `INSUFFICIENT` when $N < N_{\min}$ | **PASS** |
| **Report Formatter** | Canonical `rehoboam/m9/report/v0` schema; trailing SHA-256 digest; Markdown summary table | `rehoboam_calib.report` (606 lines); trailing digest verified over canonical JSON payload | **PASS** |
| **CLI Runner** | `python -m rehoboam_calib report --synthetic` & `report --runs <dirs> --bands <ids> --out <dir>` | `rehoboam_calib.__main__` (183 lines) with full CLI argument parsing and help strings | **PASS** |
| **Test Suite** | Unit tests of closed-form answers, survival/regression, DuckDB, sealed rule, small-N scale checks, and hash determinism | `python/tests/test_calib.py` (423 lines, 20 test functions) | **PASS** |

---

## 3. Verification Findings & Invariant Audit

### 3.1 Closed-Form & Reference Parity
- Running `test_reference_parity_synthetic_report` reproduces `specs/validation/golden/harness_v0.txt` with zero deviation.
- Closed-form tests for Student-t quantiles ($t_{0.975, 10} = 2.228139$, $t_{0.9995, 30} = 3.645959$) and Kaplan–Meier survival functions match exact analytical calculations within $\epsilon \le 10^{-12}$.
- Breslow-tied Cox proportional hazards estimation and Newton-Raphson logistic regression match R 4.4.1 `survival 3.7-0` benchmarks to within machine precision.
- F-25 unidentifiability trap correctly detects lack of variance in binary treatment cohorts and raises `StatisticsError` rather than emitting erroneous estimates.

### 3.2 Sealed Rule Compliance (V-SF-06c, V-SF-07)
- Invariant: No report emitted by `rehoboam_calib` may expose numerical statistics, confidence bounds, or p-values for sealed bands V-SF-06c and V-SF-07.
- Verification: Evaluated reports for these bands emit:
  ```json
  {
    "band_id": "V-SF-06c",
    "status": "SEALED",
    "value": null,
    "interval": null,
    "diff": null
  }
  ```
  `test_sealed_rule_in_report_output` validates that neither the JSON payload nor the Markdown rendering leaks any numeric data for sealed stylized facts.

### 3.3 V-SCALE-02 Rungs & Small-$N$ Behavior
- When evaluated against small cohorts ($N=1$ `solo`, $N=6$ `household`, $N=60$ `hamlet`), bands whose required sample size exceeds cohort count consistently report `INSUFFICIENT`.
- No band passes or fails when $N < N_{\min}$.

### 3.4 Determinism & Hermeticity
- Two runs executed with different `PYTHONHASHSEED` values (`0` vs `42`) yield byte-for-byte identical reports and SHA-256 digests.
- Reader and stats modules access no environment variables, system clocks, or unseeded pseudo-random number generators.

---

## 4. Lint, Type, and CI Audit

1. **Ruff Linter:** `ruff check python/rehoboam_calib python/tests/test_calib.py` passed with 0 errors.
2. **Ruff Formatter:** `ruff format --check python/rehoboam_calib python/tests/test_calib.py` passed clean.
3. **Mypy Static Typing:** `mypy --strict python/rehoboam_calib` passed with 0 errors across 6 source files.
4. **CI Integration:** In PR #124 CI (Run 39350354670), all 3 parallel workflows (`core`, `golden`, `stream`) passed with SUCCESS.

---

## 5. Ruling & Handoff

Task T-02 is **APPROVED 1.0.0**. The `rehoboam_calib` package forms the authoritative statistical baseline for Stage B (Task T-03: Ensembles and G1a demography gates).
