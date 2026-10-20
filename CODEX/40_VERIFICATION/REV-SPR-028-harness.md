---
id: REV-SPR-028-harness
title: "Independent review of PR #124: specs/validation/harness_v0.md, the M9 validation harness contract and reference (SPR-028 T-01)"
type: record
status: APPROVED
owner: architect
agents: [architect]
tags: [verification, validation]
related: [SPR-028, PRJ-002, GOV-002, GOV-003, ADR-0024, ADR-0026, ADR-0033]
created: 2026-10-19
updated: 2026-10-19
version: 1.0.0
---

> **BLUF: APPROVED by the Lead Architect on 2026-10-19 (Commit `730bac2` / PR #124). Task T-01 deliverables meet all contract, mathematical, and acceptance criteria.**
> - Statistical contract `specs/validation/harness_v0.md` formalizes `docs/09` §1.2, §3.1, and §4 conventions into rigorous, verifiable specifications.
> - Pure standard-library Python reference implementation `specs/validation/selftest_validation_harness.py` passes all closed-form proofs (machine epsilon $10^{-12}$), reproduces R 4.4.1 `survival 3.7-0` outputs to $\le 4.9 \times 10^{-13}$ (far exceeding the $10^{-9}$ threshold), and satisfies all five planted positive controls.
> - Stylized facts catalog `specs/validation/harness_v0.toml` (and `bands_v0.toml`) accurately codifies bounds, minimum sample requirements, units, and gate tiers for V-SF-01 through V-SF-25, correctly splitting V-SF-01 into gated $r$ and reported population spread under Ruling G10.
> - Typed synthetic forcing table `specs/validation/forcing_v0.toml` and bidirectional linter `bin/lint_forcing.py` enforce exact spec alignment and catch all 4 planted mutant controls.
> - **Upstream CI blocker noted:** PR #124 stream CI job failure at gate 8 (`the_display_form_matches_its_snapshots`) is caused by an upstream snapshot drift on `main` following PR #107 (SPR-015: Births & Households) marriage event introduction. PR #124's own code (C2 spec/tool plane) is clean and non-regressive.

---

# REV-SPR-028-harness: Independent Architect Review

## 1. Scope & Verification Target

- **Sprint:** `CODEX/05_PROJECT/SPR-028_ValidationHarness.md` (Stage A: Task T-01).
- **Target Branch / PR:** `spec/SPR-028-validation-contract` / PR #124.
- **Reviewed Head Commit:** `730bac2` (`spec(SPR-028): T-01 validation harness contract and reference [C2]`), merged with `origin/main` at `032e86d`.
- **Change Class:** C2 (specification contract, reference implementation, typed forcing declarations; tool/spec plane only; plane A untouched).
- **Reviewer:** Lead Architect (`AGT-001`).

---

## 2. Deliverables Audit

| Artifact | Purpose | Status | Audit Findings |
|:---|:---|:---|:---|
| [`specs/validation/harness_v0.md`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/harness_v0.md) | Statistical validation contract | **PASS** | Formalizes TOST zero-variance rule (G14), regularized incomplete beta Student-t quantiles, Clopper–Pearson, Wilson score, exact Poisson via $\chi^2$, seed-mean growth rate $r$ (99.9% CI), $r_{\text{exp}}$, spread $s(y)$, Kaplan–Meier with Greenwood SE, stratified Cox with Breslow ties, F-25 non-identification rule, SHA-256 keyed bootstrap, held-out band sealing, and canonical report format `rehoboam/m9/report/v0`. |
| [`specs/validation/harness_v0.toml`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/harness_v0.toml) / [`bands_v0.toml`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/bands_v0.toml) | Machine-readable stylized facts catalog | **PASS** | Declarations for V-SF-01 through V-SF-25. V-SF-01 correctly split into `V-SF-01.r` (gated) and `V-SF-01.pop` (reported) per Ruling G10. Retaining both filename aliases guarantees backwards/forwards compatibility across doc references. |
| [`specs/validation/forcing_v0.toml`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/forcing_v0.toml) | Typed synthetic forcing declarations | **PASS** | Codifies system isolation runs for M2, M7, S3, S5, S6, S7, and T1. Restricts series heads to `constant`, `step`, and `historical`; forbids `sine` and `ramp` in v0. |
| [`specs/validation/forcing/fixture_regicides.toml`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/forcing/fixture_regicides.toml) | M7 historical regicide fixture | **PASS** | Scripted historical murders on title holders at years 50, 100, 150. Valid schema. |
| [`specs/validation/forcing/court_deaths.toml`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/forcing/court_deaths.toml) | T1 historical court death fixture | **PASS** | Scripted advisor, marshall, and sovereign deaths for cascade testing (V-SAGA-16b). |
| [`bin/lint_forcing.py`](file:///home/bdavidriggins/Documents/Rehoboam/bin/lint_forcing.py) | Bidirectional forcing linter | **PASS** | Verifies bidirectional alignment between system odd TOML specs and `forcing_v0.toml`. Includes 4 planted mutant controls. |
| [`specs/validation/selftest_validation_harness.py`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/selftest_validation_harness.py) | Python stdlib reference implementation | **PASS** | Pure standard library (GOV-003 §3). Implements complete mathematical library, test suite, and positive control harness. |
| [`specs/validation/golden/r_reference_v0.txt`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/golden/r_reference_v0.txt) | Committed R 4.4.1 transcript | **PASS** | Committed R 4.4.1 `survival 3.7-0` reference run with `options(digits = 15)`. |
| [`specs/validation/golden/harness_v0.txt`](file:///home/bdavidriggins/Documents/Rehoboam/specs/validation/golden/harness_v0.txt) | Deterministic test suite golden | **PASS** | Exact canonical output snapshot of the self-test execution. |

---

## 3. Mathematical Verification & Precision Analysis

Independent evaluation against textbook closed forms and R 4.4.1 survival analysis reference standards:

### 3.1 Closed-Form Known Answers
1. **Clopper–Pearson Exact Interval (0 events in 20 trials, 95%):**
   - Expected: $1 - (0.025)^{1/20} = 0.168433497427909...$
   - Computed: $0.168433497427909...$
   - Absolute Difference: $0.000 \times 10^0$ (**PASS**; exact to machine precision).
2. **Poisson Rate Exact Interval (0 count, 95%):**
   - Expected: $-\ln(0.025) / T = 3.688879454113936...$
   - Computed: $3.688879454113936...$
   - Absolute Difference: $0.000 \times 10^0$ (**PASS**; exact to machine precision).
3. **Kaplan–Meier Uncensored Empirical Parity:**
   - Absolute Maximum Difference: $1.110 \times 10^{-16}$ (**PASS**; machine epsilon).
4. **TOST Zero-Variance Rule (Ruling G14):**
   - Identical samples ($\Delta = 0$): Evaluates to $[0.0, 0.0]$, verdict `EQUIVALENT` without division-by-zero panic (**PASS**).
   - Constant shift $d = 0.02 < 0.05$: Evaluates to $[0.02, 0.02]$, verdict `EQUIVALENT` (**PASS**).
   - Constant shift $d = 0.06 \ge 0.05$: Evaluates to $[0.06, 0.06]$, verdict `NOT_EQUIVALENT` (**PASS**).

### 3.2 R 4.4.1 Parity
Comparison of `selftest_validation_harness.py` against committed `golden/r_reference_v0.txt`:

| Estimator / Statistic | R 4.4.1 / survival 3.7-0 Value | Reference Implementation | Relative Difference | Acceptance Criterion | Verdict |
|:---|:---|:---|:---|:---|:---|
| **Gehan KM Survival Curve** | 7 discrete evaluation points | Identical to 14 decimals | $\le 9.218 \times 10^{-15}$ | $\le 10^{-9}$ | **PASS** |
| **Gehan Cox $\beta$ (Breslow)** | $-1.509191412585879$ | $-1.509191412585879$ | $2.943 \times 10^{-16}$ | $\le 10^{-9}$ | **PASS** |
| **Gehan Cox SE($\beta$)** | $0.409564406366741$ | $0.409564406366741$ | $5.421 \times 10^{-16}$ | $\le 10^{-9}$ | **PASS** |
| **Stratified Cox $\beta$ (Breslow)**| $-0.343131957478944$ | $-0.343131957478944$ | $1.132 \times 10^{-15}$ | $\le 10^{-9}$ | **PASS** |
| **Stratified Cox SE($\beta$)** | $0.718386740446407$ | $0.718386740446407$ | $3.091 \times 10^{-16}$ | $\le 10^{-9}$ | **PASS** |
| **Student-t Quantiles** ($p \in \{0.95, 0.975, 0.9995\}$) | $\nu \in \{7, 31\}$ | Incomplete beta continued fraction | $\le 2.177 \times 10^{-16}$ | $\le 10^{-9}$ | **PASS** |
| **Beta Quantiles** ($a=10, b=15$) | $p \in \{0.0005, 0.025, 0.975, 0.9995\}$ | Newton-Raphson regularized beta | $\le 3.604 \times 10^{-15}$ | $\le 10^{-9}$ | **PASS** |
| **$\chi^2$ Quantiles** ($\nu \in \{2, 10, 31\}$) | $p \in \{0.0005, 0.025, 0.975, 0.9995\}$ | Incomplete gamma inversion | $\le 4.856 \times 10^{-13}$ | $\le 10^{-9}$ | **PASS** |

### 3.3 Rate Convergence & Spread Formulations
- **Stationary start:** $r_{\text{exp}} = 0.001099$ ($r_{\text{stable}} = 0.001140$, transient $= 0.000000$, log-correction $= -0.000041$). Transient is zero to printed precision, matching `selftest_m2_f1.py` check 5.
- **Uniform start $[0, 60)$:** $r_{\text{exp}} = 0.001057$ ($r_{\text{stable}} = 0.001140$, transient $= -0.000042$, log-correction $= -0.000041$).
- **Spread formula vs Leslie simulation (400 replicates):**
  - $y = 100$: formula $= 0.1141$, simulation $= 0.1192$, ratio $= 0.957$ (diff $4.3\% \le 25\%$).
  - $y = 300$: formula $= 0.1871$, simulation $= 0.1985$, ratio $= 0.942$ (diff $5.8\% \le 25\%$).
  - $y = 1000$: formula $= 0.2867$, simulation $= 0.3104$, ratio $= 0.923$ (diff $7.7\% \le 25\%$).

---

## 4. Planted Controls & Failure Mode Verification

All required negative and positive controls verified independently:

1. **`km-ignores-censoring`**: Caught survival drop $0.448 \to 0.000$ when right-censored subjects are counted as survivors (**PASS**).
2. **`minimum-ignored`**: Emits `INSUFFICIENT` when event counts fall below $N_{\min}$ threshold (**PASS**).
3. **`heldout-printed`**: Grep audit catches and fails on any numeric output inside held-out bands V-SF-06c or V-SF-07 (**PASS**).
4. **`bootstrap-unkeyed`**: Verifies exact byte-identical resamples across different execution environments via SHA-256 expansion (**PASS**).
5. **`range-gated` (Ruling G10)**: Demonstrates that gating per-seed population range $[0.3, 5.0]$ fails $3/3$ correct synthetic ensembles over 32 seeds, confirming the mathematical rationale for reporting range rather than gating it (**PASS**).
6. **`lint_forcing.py --controls`**:
   - `isolation-no-typed-entry`: Caught missing typed forcing declaration (**PASS**).
   - `head-word-differs`: Caught mismatched series keyword between spec and forcing table (**PASS**).
   - `rate-read-unforced`: Caught unforced read lacking neutral value (**PASS**).
   - `sine-series-refused`: Caught forbidden `sine` series keyword in v0 (**PASS**).

---

## 5. Architectural Findings & Rulings

### Ruling H-01: Acceptance of Task T-01
Task T-01 of Stage A (`SPR-028a`) is **ACCEPTED and APPROVED**. The deliverables establish an airtight, mathematically verified foundation for Task T-02 (`rehoboam_calib` implementation).

### Ruling H-02: Machine-Readable Catalog Path Alias
Both `specs/validation/harness_v0.toml` and `specs/validation/bands_v0.toml` exist with byte-identical content. `SPR-028` mentions both names across various sections. Both shall be maintained as identical artifacts until Stage B consolidation.

### Ruling H-03: Finding F-25 Codification
The codification of Finding F-25 (stratified Cox unidentifiability under constant-law strata in P1a worlds) is correctly resolved in `harness_v0.md` §3.7 and in `selftest_validation_harness.py`. Emitting `INSUFFICIENT` with `"law constant within every stratum"`, accompanied by descriptive unstratified HR without confidence intervals, accurately reflects the information limits of P1a historical setups without generating spurious failures.

### Ruling H-04: Upstream Stream CI Failure Remediation
During PR #124 CI execution, the `PR tier (bin/check_all.sh --part stream)` job failed at gate 8:
```text
thread 'the_display_form_matches_its_snapshots' panicked at crates/rh-golden/tests/tail.rs:250:9:
SNAPSHOT hamlet_min_tellability_100 differs at line 1:
-     76 0001-04-04 00:00 DEATH target Job's servant 5 (natural) [event #0]
+     61 0000-01-20 00:00 MARRIAGE actor #d8521cbfd69fb647 target Job's servant 6 [event #0]
rerun with RH_UPDATE_SNAPSHOTS=1 and review the diff
```
**Architect Analysis:**
1. This failure is an **upstream regression on `main`**, introduced when PR #107 (SPR-015: Births & Households) merged into `main`. The partnering and marriage system generated a valid new `MARRIAGE` event at tick 61 in `hamlet`.
2. PR #123 (merged subsequently) did not update this snapshot, leaving `main`'s stream tier red.
3. PR #124 is a C2 change touching only `specs/validation/` and `bin/lint_forcing.py`. It introduced zero engine or golden changes.
4. **Action:** The developer or Architect shall update `hamlet_min_tellability_100.txt` using `RH_UPDATE_SNAPSHOTS=1` so that PR #124 CI turns green before final merge commit pinning.

---

## 6. Next Steps

1. Mark Task T-01 complete in `CODEX/05_PROJECT/SPR-028_ValidationHarness.md`.
2. Address the upstream snapshot drift to restore green CI on the branch.
3. Authorize developer `AGT-002-TOOLS` to proceed to Task T-02 (`rehoboam_calib` package implementation).
