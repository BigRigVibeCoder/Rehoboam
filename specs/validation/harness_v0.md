# M9 validation harness v0: statistical validation contract (SPR-028)

*Status: draft for SPR-028 Task T-01 (Stage A; `CODEX/05_PROJECT/SPR-028_ValidationHarness.md`). Companion of `specs/validation/harness_v0.toml`, `specs/validation/selftest_validation_harness.py`, `specs/validation/golden/harness_v0.txt`, and `specs/validation/golden/r_reference_v0.txt`. Binding governance: `CLAUDE.md`, `AGENTS.md`, `docs/09_verification_and_validation.md` §1.2, §3.1, §4, `docs/11_roadmap.md` P1.10, ADR-0024, ADR-0026, ADR-0033, ADR-0035, and Director Decisions D41, D58, D61, D62, D63.*

> **BLUF:** Defines the executable statistical validation contract for Rehoboam M9 (`rh-calib` / `rehoboam_calib`). It formalizes the conventions of `docs/09` §1.2 into standard-library Python reference mathematics: Two One-Sided Tests (TOST) with the zero-variance rule (G14); exact Student-t confidence intervals computed directly via incomplete beta regularized functions; exact Clopper–Pearson and Wilson score intervals for binomial proportions; exact Poisson confidence intervals via chi-squared quantiles; seed-mean growth rate $r$ convergence testing with a two-sided 99.9% Student-t interval; expected rate $r_{\text{exp}}$ and spread $s(y)$ formulations; right-censored Kaplan–Meier survival curves with Greenwood standard errors; stratified Cox proportional hazards with Breslow ties; logistic regression for regicide odds ratios; keyed deterministic bootstrap (SHA-256); held-out band sealing (D61 item 4); and the canonical report format `rehoboam/m9/report/v0`.

---

## 1. Scope & Purpose

The M9 validation harness bridges the gap between raw plane-A simulation output (chronicles, parquet stream segments, vitals lines) and scientific acceptance evidence for Gate G1a, Gate G1b, and subsequent phase gates. Under ADR-0024, the harness operates strictly outside plane A (authoritative simulation state): it reads sealed simulation outputs at rest, computes deterministic statistics, and renders structured audit reports.

This document serves as the binding contract for:
1. **The reference implementation** (`specs/validation/selftest_validation_harness.py`): a pure Python standard-library reference that verifies all statistical primitives against closed-form solutions and R's `survival` package outputs (`specs/validation/golden/r_reference_v0.txt`) to within $10^{-9}$ relative precision.
2. **The machine-readable catalog** (`specs/validation/harness_v0.toml`): formal declarations for Stylized Facts V-SF-01 through V-SF-25.
3. **The production calibration package** (`python/rehoboam_calib/`): implemented in Task T-02.

---

## 2. Core Statistical Conventions

### 2.1 The Seed as the Fundamental Unit
In accordance with `docs/09` §1.2 rule 5, **the unit of statistical analysis across ensembles is the seed**.
- For an ensemble of size $S$ (standard $S = 32$ for gate evaluations), a run metric $X_s$ is derived from each seed $s \in \{1, \dots, S\}$.
- The ensemble statistic is the seed-mean $\bar{X} = \frac{1}{S}\sum_{s=1}^S X_s$ or median across seeds.
- Within-seed variances and between-seed variances are computed separately to report non-ergodicity (`docs/09` §3.1).
- Ensembles run on pinned seeds (e.g. seeds 1–32) with fixed salts so that all evaluations are bit-deterministic functions of the codebase.

### 2.2 Confidence Levels: Gated vs Reported
To eliminate flaky gates while maintaining rigorous scientific standards (SPR-012, SPR-028 S1):
- **Gated bands** (e.g. V-SF-01 growth rate $r$, V-SF-02 child mortality $q(15)$, V-SF-02b adult survival $l(65)/l(15)$, and M2 `[isolation]` expect (1)): evaluate **two-sided 99.9% confidence intervals** ($\alpha = 0.001$, $t(0.9995; S-1)$).
- **Reported bands** (exploratory or descriptive): evaluate **90% confidence intervals** ($\alpha = 0.10$, $t(0.95; S-1)$) or 95% confidence intervals as specified in `harness_v0.toml`.

### 2.3 Verdict Definitions
For a band with specified bounds $[L, U]$ and estimated $(1-\alpha)$ confidence interval $[C_{\text{lo}}, C_{\text{hi}}]$:
1. **`PASS`**: The confidence interval lies entirely within the target band:
   $$L \le C_{\text{lo}} \quad \text{and} \quad C_{\text{hi}} \le U$$
2. **`FAIL`**: The confidence interval lies entirely outside the target band:
   $$C_{\text{hi}} < L \quad \text{or} \quad C_{\text{lo}} > U$$
3. **`MARGINAL`**: The interval overlaps an edge of the band, or an endpoint is within $10^{-9}$ relative distance of the boundary:
   $$\frac{|C_{\text{lo}} - L|}{\max(|L|, 1)} \le 10^{-9} \quad \text{or} \quad \frac{|C_{\text{hi}} - U|}{\max(|U|, 1)} \le 10^{-9}$$
4. **`INSUFFICIENT`**: The sample size (e.g. total births, events, or completed reigns) is below the minimum threshold $N_{\min}$ specified in `harness_v0.toml`.
5. **`SEALED`**: Applicable to held-out bands (V-SF-06c, V-SF-07; D61 item 4, GOV-002 law 11). The evaluator emits `SEALED` and prints **no statistic, interval, or sample size**. Any report printing numerical values for a sealed band fails verification.

---

## 3. Mathematical Formulations & Estimators

### 3.1 Student-t Distribution and Quantiles
Given sample values $X = [x_1, x_2, \dots, x_n]$:
$$\bar{x} = \frac{1}{n}\sum_{i=1}^n x_i, \quad s^2 = \frac{1}{n-1}\sum_{i=1}^n (x_i - \bar{x})^2, \quad \text{SE} = \frac{s}{\sqrt{n}}$$
Degrees of freedom $\nu = n - 1$. The two-sided $(1-\alpha)$ confidence interval is:
$$\left[ \bar{x} - t(1 - \alpha/2; \nu)\,\text{SE}, \;\; \bar{x} + t(1 - \alpha/2; \nu)\,\text{SE} \right]$$

#### Exact Computation of $t(p; \nu)$
Quantiles are calculated via the inverse regularized incomplete beta function, avoiding approximation tables:
For $p > 0.5$, let $a = 2(1 - p)$. We find $x \in (0, 1)$ such that:
$$I_x\left(\frac{\nu}{2}, \frac{1}{2}\right) = a$$
where $I_x(a, b) = \frac{B(x; a, b)}{B(a, b)}$ is the regularized incomplete beta function computed via continued fractions (Lentz's method).
The Student-t critical value is then:
$$t(p; \nu) = \sqrt{\nu \, \frac{1 - x}{x}}$$
For $p < 0.5$, $t(p; \nu) = -t(1 - p; \nu)$; for $p = 0.5$, $t(0.5; \nu) = 0$.

### 3.2 TOST Equivalence Testing & Zero-Variance Rule (G14)
To demonstrate that two configurations (e.g. CRN-paired before/after models) agree within tolerance $\delta > 0$, TOST tests the two composite null hypotheses:
$$H_{01}: \mu_A - \mu_B \le -\delta \quad \text{vs} \quad H_{11}: \mu_A - \mu_B > -\delta$$
$$H_{02}: \mu_A - \mu_B \ge +\delta \quad \text{vs} \quad H_{12}: \mu_A - \mu_B < +\delta$$
Rejecting both at significance level $\alpha = 0.05$ is mathematically equivalent to the 90% confidence interval of the paired difference $\bar{d}$ lying strictly within $(-\delta, +\delta)$.

#### The Zero-Variance Rule (Ruling G14)
When comparing identical configurations or deterministic CRN rollouts, paired differences may have zero variance ($s_d = 0$):
1. **Identical samples ($D_i = 0$ for all $i$):**
   $\bar{d} = 0$, $s_d = 0$, $\text{SE}_d = 0$.
   The confidence interval is defined as $[0.0, 0.0]$. The verdict is **`EQUIVALENT`**. No division by zero is permitted.
2. **Constant non-zero shift ($D_i = d \ne 0$ for all $i$):**
   $\bar{d} = d$, $s_d = 0$.
   The interval is defined as $[d, d]$. The verdict is **`EQUIVALENT`** if and only if $|d| < \delta$; otherwise **`NOT_EQUIVALENT`**.
3. **Non-zero variance ($s_d > 0$):**
   The standard 90% Student-t interval $[\bar{d} - t(0.95; n-1)\text{SE}_d, \; \bar{d} + t(0.95; n-1)\text{SE}_d]$ is evaluated against $(-\delta, +\delta)$.

### 3.3 Binomial Confidence Intervals

#### Clopper–Pearson Exact Interval
For $k$ successes in $n$ independent trials at significance level $\alpha$:
- If $k = 0$: Lower bound is $0.0$; Upper bound is $1 - (\alpha/2)^{1/n}$.
- If $k = n$: Lower bound is $(\alpha/2)^{1/n}$; Upper bound is $1.0$.
- If $0 < k < n$:
  $$\text{Lower} = B(\alpha/2; k, n - k + 1)$$
  $$\text{Upper} = B(1 - \alpha/2; k + 1, n - k)$$
  where $B(p; a, b)$ is the $p$-th quantile of the $\text{Beta}(a, b)$ distribution.

#### Wilson Score Interval
Center: $\tilde{p} = \frac{k + z^2/2}{n + z^2}$ where $z = z_{1-\alpha/2}$.
$$\text{Half-width} = \frac{z \sqrt{n}}{n + z^2} \sqrt{\hat{p}(1 - \hat{p}) + \frac{z^2}{4n}}, \quad \hat{p} = \frac{k}{n}$$
$$\text{CI} = \left[ \max\left(0, \tilde{p} - \text{half-width}\right), \;\; \min\left(1, \tilde{p} + \text{half-width}\right) \right]$$

### 3.4 Exact Poisson Confidence Interval
For observed count $k$ over total exposure $T$:
- If $k = 0$: Lower bound is $0.0$; Upper bound is $-\ln(\alpha/2) / T$.
- If $k > 0$:
  $$\text{Lower} = \frac{1}{2T} \chi^2(2k; \alpha/2)$$
  $$\text{Upper} = \frac{1}{2T} \chi^2(2(k + 1); 1 - \alpha/2)$$
where $\chi^2(\nu; p)$ is the $p$-th quantile of the $\chi^2(\nu)$ distribution computed via incomplete gamma inversion.

### 3.5 Seed-Mean Growth Rate $r$, Expected Rate $r_{\text{exp}}$, and Spread $s(y)$
For seed $s$ over a simulation horizon of $y$ years with initial population $N_0$ and final population $N_y$:
$$r_s = \frac{\ln(N_y / N_0)}{y}$$
The ensemble mean $\bar{r} = \frac{1}{S}\sum r_s$ is evaluated via a two-sided 99.9% Student-t interval against V-SF-01's target band $[-0.001, +0.003]$ ($-0.1\%$ to $+0.3\%$/yr).

#### Expected Rate Formulation ($r_{\text{exp}}$)
$$r_{\text{exp}}(\text{start}, y) = \frac{\ln(E[N_y] / N_0) - s(y)^2 / 2}{y}$$
where:
1. $E[N_y]$ is the mean projection under the compiled F1 demography schedule. For a stationary start, the transient term is zero and $\ln(E[N_y]/N_0)/y = r_{\text{stable}}$. For a uniform start $[0, 60)$ years (M2 isolation), $E[N_y]$ incorporates the initial demographic restructuring transient.
2. The term $-s(y)^2 / 2$ accounts for the Jensen's inequality difference between the mean of the log and the log of the mean ($\mathbb{E}[\ln X] \approx \ln \mathbb{E}[X] - \text{Var}(X) / (2 \mathbb{E}[X]^2)$).

#### Expected Relative Spread ($s(y)$)
Derived from the variance of a linear birth-and-death process:
$$s(y)^2 = \frac{(b + d)(1 - e^{-r y})}{r F_0}$$
where $F_0$ is the initial population of reproductive females (e.g. 500 for a 1,000-person founder cohort), $b$ is the crude birth rate, and $d$ is the crude death rate. The expected relative standard deviation of population size is:
$$s(y) = \sqrt{s(y)^2}$$

### 3.6 Kaplan–Meier Survival Estimator
For lifetimes $T_i$ with event indicators $\delta_i \in \{0, 1\}$ (where $\delta_i = 1$ denotes an uncensored event and $\delta_i = 0$ denotes right-censoring):
Let distinct event times be $t_1 < t_2 < \dots < t_K$. At time $t_k$:
- $Y_k$: number of subjects at risk ($T_i \ge t_k$).
- $d_k$: number of events observed at $t_k$.
The Kaplan–Meier survival estimate is:
$$\hat{S}(t) = \prod_{t_k \le t} \left( 1 - \frac{d_k}{Y_k} \right)$$
Greenwood's estimator of variance is:
$$\widehat{\text{Var}}(\hat{S}(t)) = \hat{S}(t)^2 \sum_{t_k \le t} \frac{d_k}{Y_k (Y_k - d_k)}$$
$$\text{SE}(\hat{S}(t)) = \sqrt{\widehat{\text{Var}}(\hat{S}(t))}$$

### 3.7 Stratified Cox Proportional Hazards Model (Breslow Ties)
Subjects belong to strata $s \in \{1, \dots, S\}$ (e.g. polities), with hazard:
$$h_{is}(t) = h_{0s}(t) \exp(\boldsymbol{\beta}^T \mathbf{z}_i)$$
With Breslow handling of tied event times, the log partial likelihood across all strata is:
$$\ln L(\boldsymbol{\beta}) = \sum_{s=1}^S \sum_{k=1}^{K_s} \left[ \boldsymbol{\beta}^T \sum_{i \in D_{sk}} \mathbf{z}_i - d_{sk} \ln\left( \sum_{j \in R_{sk}} \exp(\boldsymbol{\beta}^T \mathbf{z}_j) \right) \right]$$
where $R_{sk}$ is the risk set in stratum $s$ at event time $t_{sk}$, and $D_{sk}$ is the set of events ($d_{sk} = |D_{sk}|$).
The score vector $\mathbf{U}(\boldsymbol{\beta})$ and observed information matrix $\mathbf{I}(\boldsymbol{\beta})$ are summed across strata. Newton–Raphson iteration obtains $\hat{\boldsymbol{\beta}}$, with asymptotic covariance $\widehat{\text{Var}}(\hat{\boldsymbol{\beta}}) = \mathbf{I}(\hat{\boldsymbol{\beta}})^{-1}$.

#### Stratification Non-Identification Rule (Finding F-25)
In P1a worlds (such as `twin_vales`), each polity operates under a single immutable succession law for the entire run (e.g. Polity 1: primogeniture; Polity 2: tanistry).
Consequently, within each stratum $s$, the covariate vector $\mathbf{z}_i$ is constant across all subjects:
$$\sum_{j \in R_{sk}} (\mathbf{z}_j - \bar{\mathbf{z}}_{sk})^{\otimes 2} = 0 \implies \mathbf{I}(\boldsymbol{\beta}) = 0$$
When within-stratum variance of covariates is zero across all strata, $\boldsymbol{\beta}$ is **unidentified**.
- The harness reports V-SF-06b's hazard ratio as **`INSUFFICIENT`** with the diagnostic note:
  `"law constant within every stratum"`.
- Deposition shares by law (Clopper–Pearson CIs) serve as P1a empirical evidence.
- An unstratified descriptive Cox HR is printed without a confidence interval, labelled:
  `"descriptive; not docs/09's method"`.

### 3.8 Deterministic Keyed Bootstrap
Where analytical confidence intervals are not defined (e.g. complex turning-point detection in V-SF-04), a seed-level bootstrap of $B = 10,000$ replicates is used.
- To maintain bit-level determinism across platforms and independent runs, the pseudorandom resample indices are generated using **SHA-256 stream expansion**:
  $$K = \text{SHA-256}\left(\texttt{b"rehoboam/calib/bootstrap/v0:"} \,\|\, \text{seed\_tag} \,\|\, \text{band\_id} \,\|\, \text{resample\_idx}\right)$$
- No dependence on system clocks, unseeded libraries, or `PYTHONHASHSEED` is permitted.

---

## 4. Stylized Facts Catalog (Summary)

The machine-readable definitions in `specs/validation/harness_v0.toml` declare bounds, sample requirements, and gate tiers for all stylized facts:

| ID | Name | Kind | Role | Status | Test Method | Gated Bounds / Targets |
|:---|:---|:---|:---|:---|:---|:---|
| **V-SF-01.r** | Long-run growth rate $r$ | Identity | Calibration | **gate_g1a** | Student-t (99.9%) | $[-0.001, +0.003]$/yr ($[-0.1\%, +0.3\%]$) |
| **V-SF-01.pop** | Population range | Identity | Calibration | report | Range Check | $[0.3, 5.0] \cdot N_0 e^{ry}$ or $[0.3, 5.0] \cdot K$ |
| **V-SF-02** | Child mortality $q(15)$ | Identity | Calibration | **gate_g1a** | Student-t (99.9%) | $[0.35, 0.55]$ (min 400 births) |
| **V-SF-02b** | Adult mortality shape $l(65)/l(15)$ | Identity | Calibration | **gate_g1a** | Student-t (99.9%) | Manifest target $\pm 0.03$ |
| **V-SF-03** | Famine mortality crises | Emergent | Calibration | gate_g1b | Bootstrap | Crises/century: any 2–5, med 1–3, major 0.1–0.5 |
| **V-SF-03a** | Harvest shortfalls | Identity | Calibration | gate_g1b | Clopper–Pearson | CV $\in [0.09, 0.13]$, shortfall frequencies |
| **V-SF-03b** | Famine price amplitude | Emergent | Calibration | report | Bootstrap | Peak/normal price ratio in worst famine $\times 2.0$--$3.5$ |
| **V-SF-03c** | Famine triggers | Emergent | Held-out | gate_g1b | Clopper–Pearson | $\ge 70\%$ follow $\ge 2$ shortfalls or siege |
| **V-SF-03d** | Entitlement over stock | Emergent | Held-out | gate_g4 | CRN Paired | Wages rise post-shock; partial $\rho > 0$ |
| **V-SF-03e** | General famines | Emergent | Held-out | soft | Poisson Exact | 2–5 per polity-century |
| **V-SF-04** | Secular cycles | Emergent | Calibration | gate_g4 | Bootstrap | Median period 200–330y, $\ge 80\%$ in 150–450y |
| **V-SF-04b** | Instability contrast | Emergent | Held-out | gate_g4 | Wilcoxon | Events/decade decline/growth $> 1.0$; median 3–10 |
| **V-SF-05** | Violence waves | Emergent | Held-out | gate_g4 | Bootstrap | Peak intervals median 40–60y, IQR in 30–75y |
| **V-SF-05b** | Demographic lead-lag | Emergent | Calibration | soft | Cross-Corr | Wage downturn leads elite rise by 10–30y |
| **V-SF-06** | Ruler murder rate | Identity | Calibration | gate_g1b | Poisson Exact | Murders/1000 ruler-years by institutional state |
| **V-SF-06b** | Deposition by law | Identity | Calibration | gate_g1b | Cox Stratified | Primogeniture vs elective HR in $[0.17, 0.63]$ |
| **V-SF-06c** | Reign length by institutions | Emergent | Held-out | **sealed** | Kaplan–Meier | Unconstrained 8–15y, constrained 17–22y (SEALED) |
| **V-SF-06d** | Regicide contagion | Identity | Calibration | soft | Logistic Reg | Predecessor murder odds ratio $\in [1.5, 4.0]$ |
| **V-SF-07** | Polity lifespans | Emergent | Held-out | **sealed** | Weibull Censored| Shape $k \in [0.8, 1.7]$; empire mean 150–320y (SEALED) |
| **V-SF-08** | City sizes | Emergent | Calibration | soft | CSN Power-Law | Tail exponent $\hat{\alpha} \in [1.8, 2.6]$ |
| **V-SF-09** | War severity | Emergent | Calibration | gate_g4 | CSN Power-Law | Heavy tail $\hat{\alpha} \in [1.3, 1.9]$ ($p > 0.1$) |
| **V-SF-09b** | Memoryless war onsets | Emergent | Held-out | gate_g4 | Bootstrap | Onset gaps geometric-plausible; dispersion $[0.7, 1.5]$ |
| **V-SF-09c** | War tail produced | Emergent | Calibration | gate_g4 | CRN Paired | Ablation raises $\hat{\alpha}$ by $\ge 0.3$ |
| **V-SF-10** | Malthusian test | Identity | Calibration | report | TOST Paired | Income change equivalent to 0 ($\delta = \pm 5\%$) |
| **V-SF-10b** | Wage pressure & escape | Emergent | Calibration | report | CRN Paired | Frozen tech lowers $WR_A$; matching tech stable |
| **V-SF-11** | Post-shock wage rise | Identity | Calibration | report | Sign Test | Mean $WR_A(+10..+50) > WR_A(-25..-1)$ |
| **V-SF-11b** | Post-plague levelling | Emergent | Held-out | report | Sign Test | Top-10% wealth share falls within 50y |
| **V-SF-11c** | Wage shock magnitude | Emergent | Held-out | report | Bootstrap | Ratio $WR_A(+40..+60)/WR_A(-25..-1) \in [1.2, 2.5]$ |
| **V-SF-12** | Hive false positives | Identity | Calibration | gate_g4 | Clopper–Pearson | At prevalence $< 1\%$, false positive share $> 50\%$ |
| **V-SF-13** | Sensitivity screening | Identity | Calibration | report | Morris / Sobol | Screening on labelled PRs |
| **V-SF-14** | Structural crises | Emergent | Held-out | report | Clopper–Pearson | Breakdown frequency by population growth class |
| **V-SF-15** | Industrial damping | Emergent | Calibration | report | Cox PH | Breakdown HR with damping $< 1.0$ |
| **V-SF-16** | Income inequality | Emergent | Calibration | report | Gini Bootstrap | Person-weighted Gini in $[24, 64]$; median in $[33, 58]$ |
| **V-SF-17** | Wealth inequality | Emergent | Calibration | report | Sign Test | Wealth Gini $>$ Income Gini |
| **V-SF-18** | Household structure | Emergent | Calibration | soft | Bootstrap | Mean household size within cultural bounds |
| **V-SF-19** | Polity consolidation | Emergent | Calibration | soft | CRN Paired | Power projection reduces polity count |
| **V-SF-20** | Agrarian welfare ratios | Emergent | Calibration | report | Bootstrap | Labourer $WR_A \in [0.35, 1.7]$; craftsman $[0.65, 2.9]$ |
| **V-SF-20b** | Subsistence coupling | Identity | Calibration | gate_g1b | Invariant | $WR_{\text{kcal}} < 0.44$ for $> 1$y raises hunger hazard |
| **V-SF-21** | Price dispersion | Emergent | Calibration | report | Spearman | Ratio dearest/cheapest in $[1.8, 4.0]$; falls with integration |
| **V-SF-22** | Isolation harness | Identity | Calibration | **gate_g1a** | Isolation Expect | Runs under synthetic forcing match spec `expect` |
| **V-SF-23** | Coupling strength sweep| Emergent | Calibration | report | Sweep Liveness | Runs at $\alpha \in \{0, 0.25, 0.5, 0.75, 1\}$ survive |
| **V-SF-24** | Conception rate $c_i$ | Identity | Calibration | report | Binomial Exact | Conception rate per band matches $c_i$ (99.9% CI) |
| **V-SF-25** | Nuptiality & paternity | Identity | Calibration | report | Student-t | Mean age at marriage in $[18, 22]$y; 100% have father |

---

## 5. Report Canonical Format: `rehoboam/m9/report/v0`

Reports generated by the harness follow the strict ASCII line-oriented schema:
```text
format: rehoboam/m9/report/v0
world: <world_name>
seeds: <seed_start>-<seed_end> (count: <S>)
horizon_years: <years>
engine_build: <sha>
registry_version: <version>
manifest_hash: <hash>
---
BAND V-SF-01.r: PASS [stat=0.001140, ci_lo=-0.000120, ci_hi=0.002400, band=[-0.001, 0.003], unit=%/yr, n=32]
BAND V-SF-01.pop: REPORT [stat=1.120000, spread_exp=0.312000, range=[0.300, 5.000], outliers=0, n=32]
BAND V-SF-02: PASS [stat=0.444300, ci_lo=0.412100, ci_hi=0.476500, band=[0.350, 0.550], unit=prob, n=32]
BAND V-SF-02b: PASS [stat=0.314600, ci_lo=0.298000, ci_hi=0.331200, band=[0.2846, 0.3446], unit=ratio, n=32]
...
BAND V-SF-06c: SEALED (A-04; held-out, docs/09 §3.1)
BAND V-SF-07: SEALED (A-04; held-out, docs/09 §3.1)
---
SUMMARY: 4/4 GATED PASS (100.0%), 0 FAIL, 0 MARGINAL, 0 INSUFFICIENT
digest: sha256:<hex_digest>
```
The terminal line `digest: sha256:<hex>` is the SHA-256 digest of all preceding lines in canonical ASCII encoding, ensuring tamper-evidence in CI audit artifacts.
