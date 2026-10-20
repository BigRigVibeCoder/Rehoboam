"""Statistical validation primitives and estimators for Rehoboam M9 (SPR-028).

Standard-library-only implementation of statistical validation functions:
1. Two One-Sided Tests (TOST) with the zero-variance rule (G14).
2. Exact Student-t confidence intervals and quantiles via regularized incomplete beta inversion.
3. Binomial confidence intervals: Clopper-Pearson exact and Wilson score intervals.
4. Exact Poisson confidence intervals via chi-squared quantiles.
5. Seed-mean growth rate r convergence and expected rate/spread formulations.
6. Kaplan-Meier survival curves with Greenwood standard errors.
7. Stratified Cox proportional hazards with Breslow ties (and F-25 non-identification rule).
8. Logistic regression for odds ratio estimation.
9. Deterministic SHA-256 keyed bootstrap.

Refs:
    SPR-028 (M9 validation harness);
    docs/09_verification_and_validation.md §1.2;
    specs/validation/harness_v0.md.
"""

from __future__ import annotations

import hashlib
import math
import struct
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from typing import Any

_EPS_15: float = 1e-15
_EPS_14: float = 1e-14
_EPS_12: float = 1e-12
_FPMIN: float = 1e-30
_HALF: float = 0.5
_MIN_PAIRS: int = 2
_MIN_OBSERVATIONS: int = 2
_MIN_LOGISTIC_SAMPLES: int = 4
_CHISQ_DF2: int = 2
_EXP_MAX_FLOAT: float = 700.0

# =============================================================================
# 1. Exact Mathematical Primitives (Beta & Gamma)
# =============================================================================


def betacf(a: float, b: float, x: float) -> float:
    """Continued fraction for regularized incomplete beta function (Lentz method).

    Args:
        a: Shape parameter a > 0.
        b: Shape parameter b > 0.
        x: Evaluation point in [0, 1].

    Returns:
        Evaluated continued fraction factor.
    """
    max_it = 200
    eps = 3.0e-16

    qab = a + b
    qap = a + 1.0
    qam = a - 1.0
    c = 1.0
    d = 1.0 - qab * x / qap
    if abs(d) < _FPMIN:
        d = _FPMIN
    d = 1.0 / d
    h = d

    for m in range(1, max_it + 1):
        m2 = 2 * m
        aa = m * (b - m) * x / ((qam + m2) * (a + m2))
        d = 1.0 + aa * d
        if abs(d) < _FPMIN:
            d = _FPMIN
        c = 1.0 + aa / c
        if abs(c) < _FPMIN:
            c = _FPMIN
        d = 1.0 / d
        h *= d * c

        aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2))
        d = 1.0 + aa * d
        if abs(d) < _FPMIN:
            d = _FPMIN
        c = 1.0 + aa / c
        if abs(c) < _FPMIN:
            c = _FPMIN
        d = 1.0 / d
        del_val = d * c
        h *= del_val
        if abs(del_val - 1.0) <= eps:
            break
    return h


def ibeta(a: float, b: float, x: float) -> float:
    """Regularized incomplete beta function I_x(a, b).

    Args:
        a: Shape parameter a > 0.
        b: Shape parameter b > 0.
        x: Value in [0, 1].

    Returns:
        Regularized incomplete beta value in [0, 1].
    """
    if x <= 0.0:
        return 0.0
    if x >= 1.0:
        return 1.0
    lbeta = math.lgamma(a) + math.lgamma(b) - math.lgamma(a + b)
    front = math.exp(a * math.log(x) + b * math.log(1.0 - x) - lbeta)
    if x < (a + 1.0) / (a + b + 2.0):
        return front * betacf(a, b, x) / a
    return 1.0 - front * betacf(b, a, 1.0 - x) / b


def inv_ibeta(a: float, b: float, p: float) -> float:
    """Inverse of regularized incomplete beta: finds x such that I_x(a, b) = p.

    Args:
        a: Shape parameter a > 0.
        b: Shape parameter b > 0.
        p: Probability in [0, 1].

    Returns:
        Quantile x in [0, 1].
    """
    if p <= 0.0:
        return 0.0
    if p >= 1.0:
        return 1.0

    lo, hi = 0.0, 1.0
    x = a / (a + b)
    lbeta = math.lgamma(a) + math.lgamma(b) - math.lgamma(a + b)

    for _ in range(100):
        val = ibeta(a, b, x)
        err = val - p
        if abs(err) < _EPS_15:
            break
        if err > 0.0:
            hi = x
        else:
            lo = x
        try:
            dx = math.exp((a - 1.0) * math.log(x) + (b - 1.0) * math.log(1.0 - x) - lbeta)
            x_new = x - err / dx
        except (ValueError, OverflowError, ZeroDivisionError):
            x_new = (lo + hi) / 2.0
        x = (lo + hi) / 2.0 if (x_new <= lo or x_new >= hi) else x_new
    return x


def t_quantile(p: float, df: int) -> float:
    """Quantile function for Student-t distribution with df degrees of freedom.

    Args:
        p: Cumulative probability in (0, 1).
        df: Degrees of freedom >= 1.

    Returns:
        Critical value t such that P(T <= t) = p.
    """
    if p == _HALF:
        return 0.0
    if p < _HALF:
        return -t_quantile(1.0 - p, df)
    alpha = 2.0 * (1.0 - p)
    x = inv_ibeta(df / 2.0, _HALF, alpha)
    return math.sqrt(df * (1.0 - x) / x)


def t_ci(
    data: Sequence[float], alpha: float = 0.001
) -> tuple[float, float, float, int, float, float]:
    """Compute sample mean, variance, standard error, df, and Student-t interval.

    Args:
        data: Sequence of sample values (length >= 2).
        alpha: Significance level for two-sided (1 - alpha) interval.

    Returns:
        Tuple of (mean, variance, standard_error, df, ci_lower, ci_upper).

    Raises:
        ValueError: If sample size is less than 2.
    """
    n = len(data)
    if n < _MIN_OBSERVATIONS:
        raise ValueError("Student-t confidence interval requires at least 2 observations")
    mean = sum(data) / n
    var = sum((x - mean) ** 2 for x in data) / (n - 1)
    se = math.sqrt(var / n)
    df = n - 1
    t_crit = t_quantile(1.0 - alpha / 2.0, df)
    ci_lo = mean - t_crit * se
    ci_hi = mean + t_crit * se
    return mean, var, se, df, ci_lo, ci_hi


def beta_quantile(p: float, a: float, b: float) -> float:
    """Quantile function for Beta(a, b) distribution.

    Args:
        p: Cumulative probability in [0, 1].
        a: Shape parameter a > 0.
        b: Shape parameter b > 0.

    Returns:
        Quantile x in [0, 1].
    """
    return inv_ibeta(a, b, p)


def lower_gamma_series(a: float, x: float, eps: float = 1e-16, max_iter: int = 500) -> float:
    """Lower regularized incomplete gamma P(a, x) via Taylor series.

    Args:
        a: Shape parameter a > 0.
        x: Evaluation point x >= 0.
        eps: Convergence threshold.
        max_iter: Maximum series iterations.

    Returns:
        P(a, x).
    """
    term = 1.0 / a
    total = term
    for n in range(1, max_iter):
        term *= x / (a + n)
        total += term
        if abs(term) < abs(total) * eps:
            break
    log_factor = a * math.log(x) - x - math.lgamma(a)
    return math.exp(log_factor) * total


def upper_gamma_cf(a: float, x: float, eps: float = 1e-16, max_iter: int = 500) -> float:
    """Upper regularized incomplete gamma Q(a, x) via Legendre continued fraction.

    Args:
        a: Shape parameter a > 0.
        x: Evaluation point x >= 0.
        eps: Convergence threshold.
        max_iter: Maximum iterations.

    Returns:
        Q(a, x).
    """
    b = x + 1.0 - a
    c = 1.0 / _FPMIN
    d = 1.0 / b
    h = d
    for i in range(1, max_iter):
        an = -i * (i - a)
        b += 2.0
        d = an * d + b
        if abs(d) < _FPMIN:
            d = _FPMIN
        c = b + an / c
        if abs(c) < _FPMIN:
            c = _FPMIN
        d = 1.0 / d
        del_val = d * c
        h *= del_val
        if abs(del_val - 1.0) < eps:
            break
    log_factor = a * math.log(x) - x - math.lgamma(a)
    return math.exp(log_factor) * h


def gammp(a: float, x: float) -> float:
    """Regularized incomplete gamma function P(a, x).

    Args:
        a: Shape parameter a > 0.
        x: Evaluation point x >= 0.

    Returns:
        P(a, x) in [0, 1].
    """
    if x <= 0.0 or a <= 0.0:
        return 0.0
    if x < a + 1.0:
        return lower_gamma_series(a, x)
    return 1.0 - upper_gamma_cf(a, x)


def _qchisq_initial_bracket(p: float, a: float) -> tuple[float, float]:
    """Find initial bracket [lo, hi] for chi-squared quantile inversion."""
    lo = 0.0
    hi = max(a * 10.0, 10.0)
    while gammp(a, hi / 2.0) < p:
        lo = hi
        hi *= 2.0
    return lo, hi


def _qchisq_root_step(
    lo: float, hi: float, a: float, p: float
) -> tuple[float, float, float | None]:
    """Perform one root-finding step for chi-squared quantile inversion."""
    mid = (lo + hi) / 2.0
    val = gammp(a, mid / 2.0)
    err = val - p
    if abs(err) < _EPS_15 or (hi - lo) < _EPS_14 * max(1.0, mid):
        return lo, hi, mid
    try:
        half = mid / 2.0
        pdf = _HALF * math.exp((a - 1.0) * math.log(half) - half - math.lgamma(a))
        new_mid = mid - err / pdf
    except (ValueError, OverflowError, ZeroDivisionError):
        new_mid = (lo + hi) / 2.0
    if lo < new_mid < hi:
        mid = new_mid
        val = gammp(a, mid / 2.0)
        err = val - p
        if abs(err) < _EPS_15:
            return lo, hi, mid
    if err > 0.0:
        hi = mid
    else:
        lo = mid
    return lo, hi, None


def qchisq(p: float, df: int) -> float:
    """Quantile function for Chi-squared distribution with df degrees of freedom.

    Args:
        p: Cumulative probability in [0, 1].
        df: Degrees of freedom >= 1.

    Returns:
        Critical value x such that P(X <= x) = p.
    """
    if p <= 0.0:
        return 0.0
    if p >= 1.0:
        return float("inf")
    if df == _CHISQ_DF2:
        return -2.0 * math.log(1.0 - p)

    a = df / 2.0
    lo, hi = _qchisq_initial_bracket(p, a)

    for _ in range(100):
        lo, hi, res = _qchisq_root_step(lo, hi, a, p)
        if res is not None:
            return res
    return (lo + hi) / 2.0


# =============================================================================
# 2. Binomial & Poisson Confidence Intervals
# =============================================================================


def clopper_pearson_ci(k: int, n: int, alpha: float = 0.05) -> tuple[float, float]:
    """Exact Clopper-Pearson confidence interval for a binomial proportion.

    Args:
        k: Number of successes (0 <= k <= n).
        n: Number of trials (n > 0).
        alpha: Two-sided significance level (e.g. 0.05 for 95% CI).

    Returns:
        Tuple of (ci_lower, ci_upper).

    Raises:
        ValueError: If n <= 0 or k not in 0..n.
    """
    if n <= 0:
        raise ValueError("Sample size n must be positive")
    if k < 0 or k > n:
        raise ValueError("k must be in 0..n")

    if k == 0:
        lo = 0.0
        hi = 1.0 - (alpha / 2.0) ** (1.0 / n)
    elif k == n:
        lo = (alpha / 2.0) ** (1.0 / n)
        hi = 1.0
    else:
        lo = beta_quantile(alpha / 2.0, float(k), float(n - k + 1))
        hi = beta_quantile(1.0 - alpha / 2.0, float(k + 1), float(n - k))
    return lo, hi


def wilson_score_ci(k: int, n: int, alpha: float = 0.05) -> tuple[float, float]:
    """Wilson score confidence interval for a binomial proportion.

    Args:
        k: Number of successes (0 <= k <= n).
        n: Number of trials (n > 0).
        alpha: Two-sided significance level.

    Returns:
        Tuple of (ci_lower, ci_upper).

    Raises:
        ValueError: If n <= 0 or k not in 0..n.
    """
    if n <= 0:
        raise ValueError("Sample size n must be positive")
    if k < 0 or k > n:
        raise ValueError("k must be in 0..n")

    z = t_quantile(1.0 - alpha / 2.0, 1000000)
    p_hat = k / n
    center = (k + (z * z) / 2.0) / (n + z * z)
    half_width = (z * math.sqrt(n) / (n + z * z)) * math.sqrt(
        p_hat * (1.0 - p_hat) + (z * z) / (4.0 * n)
    )
    lo = max(0.0, center - half_width)
    hi = min(1.0, center + half_width)
    return lo, hi


def poisson_exact_ci(k: int, exposure: float, alpha: float = 0.05) -> tuple[float, float]:
    """Exact Poisson rate confidence interval for count k with exposure T.

    Args:
        k: Event count (k >= 0).
        exposure: Total person-time or observation exposure > 0.
        alpha: Two-sided significance level.

    Returns:
        Tuple of (rate_lower, rate_upper).

    Raises:
        ValueError: If exposure <= 0 or k < 0.
    """
    if exposure <= 0.0:
        raise ValueError("Exposure must be positive")
    if k < 0:
        raise ValueError("Count k cannot be negative")

    if k == 0:
        lo = 0.0
        hi = -math.log(alpha / 2.0) / exposure
    else:
        lo = _HALF * qchisq(alpha / 2.0, 2 * k) / exposure
        hi = _HALF * qchisq(1.0 - alpha / 2.0, 2 * (k + 1)) / exposure
    return lo, hi


# =============================================================================
# 3. Two One-Sided Tests (TOST) with Zero-Variance Rule
# =============================================================================


def tost_paired(
    data_a: Sequence[float], data_b: Sequence[float], delta: float, alpha: float = 0.05
) -> tuple[str, float, float, float, float]:
    """Two One-Sided Tests (TOST) for equivalence of paired samples with Zero-Variance Rule (G14).

    Args:
        data_a: Sequence of first observations.
        data_b: Sequence of paired second observations.
        delta: Equivalence margin delta > 0.
        alpha: One-sided significance level (alpha=0.05 gives 90% CI).

    Returns:
        Tuple of (verdict, mean_diff, se_diff, ci_lo, ci_hi).

    Raises:
        ValueError: If lengths differ, length < 2, or delta <= 0.
    """
    if len(data_a) != len(data_b) or len(data_a) < _MIN_PAIRS:
        raise ValueError("TOST paired requires equal-length lists with at least 2 pairs")
    if delta <= 0.0:
        raise ValueError("Equivalence margin delta must be positive")

    diffs = [a - b for a, b in zip(data_a, data_b, strict=True)]
    n = len(diffs)
    mean_diff = sum(diffs) / n
    var_diff = sum((d - mean_diff) ** 2 for d in diffs) / (n - 1)

    # Zero-Variance Rule (Ruling G14)
    if var_diff == 0.0:
        if mean_diff == 0.0:
            return "EQUIVALENT", 0.0, 0.0, 0.0, 0.0
        verdict = "EQUIVALENT" if abs(mean_diff) < delta else "NOT_EQUIVALENT"
        return verdict, mean_diff, 0.0, mean_diff, mean_diff

    se_diff = math.sqrt(var_diff / n)
    t_crit = t_quantile(1.0 - alpha, n - 1)
    ci_lo = mean_diff - t_crit * se_diff
    ci_hi = mean_diff + t_crit * se_diff

    verdict = "EQUIVALENT" if (ci_lo > -delta and ci_hi < delta) else "NOT_EQUIVALENT"
    return verdict, mean_diff, se_diff, ci_lo, ci_hi


# =============================================================================
# 4. Survival Analysis & Epidemiological Estimators
# =============================================================================


@dataclass(frozen=True)
class KMPoint:
    """A single point on a Kaplan-Meier survival curve."""

    time: float
    n_risk: int
    n_event: int
    n_cens: int
    survival: float
    std_err: float


def kaplan_meier(times: Sequence[float], status: Sequence[int]) -> list[KMPoint]:
    """Kaplan-Meier survival estimator with Greenwood standard errors.

    Args:
        times: Observed event or censoring times.
        status: Event indicators (1 = event, 0 = censored).

    Returns:
        List of KMPoint instances at each distinct event time.

    Raises:
        ValueError: If arrays are empty or lengths differ.
    """
    if len(times) != len(status) or len(times) == 0:
        raise ValueError("Times and status must be non-empty and equal length")

    distinct_times = sorted(set(times))
    surv = 1.0
    var_sum = 0.0
    result: list[KMPoint] = []

    for t in distinct_times:
        n_risk = sum(1 for ti in times if ti >= t)
        n_event = sum(1 for ti, si in zip(times, status, strict=True) if ti == t and si == 1)
        n_cens = sum(1 for ti, si in zip(times, status, strict=True) if ti == t and si == 0)

        if n_event > 0:
            surv *= 1.0 - n_event / n_risk
            if n_risk > n_event:
                var_sum += n_event / (n_risk * (n_risk - n_event))
            se = surv * math.sqrt(var_sum)
        else:
            se = surv * math.sqrt(var_sum)

        result.append(
            KMPoint(
                time=t,
                n_risk=n_risk,
                n_event=n_event,
                n_cens=n_cens,
                survival=surv,
                std_err=se,
            )
        )
    return result


def _check_strata_variance(
    distinct_strata: Sequence[Any], strata: Sequence[Any], x: Sequence[float], n: int
) -> None:
    """Verify that within-stratum covariate variance is non-zero (Finding F-25)."""
    has_variance = False
    for s in distinct_strata:
        idx_s = [i for i in range(n) if strata[i] == s]
        x_s = [x[i] for i in idx_s]
        if len(set(x_s)) > 1:
            has_variance = True
            break
    if not has_variance:
        raise ValueError("law constant within every stratum (Finding F-25: covariate unidentified)")


def _cox_stratum_contributions(
    stratum_data: tuple[list[float], list[int], list[float]],
    beta: float,
    accum: tuple[float, float, float],
) -> tuple[float, float, float]:
    """Compute score, information, and loglik contributions for one stratum."""
    times_s, status_s, x_s = stratum_data
    total_score, total_info, total_loglik = accum
    event_times = sorted({t for t, st in zip(times_s, status_s, strict=True) if st == 1})

    for t in event_times:
        risk_idx = [i for i in range(len(times_s)) if times_s[i] >= t]
        death_idx = [i for i in range(len(times_s)) if times_s[i] == t and status_s[i] == 1]
        d = len(death_idx)

        s0 = sum(math.exp(beta * x_s[i]) for i in risk_idx)
        s1 = sum(x_s[i] * math.exp(beta * x_s[i]) for i in risk_idx)
        s2 = sum(x_s[i] * x_s[i] * math.exp(beta * x_s[i]) for i in risk_idx)

        sum_death_x = sum(x_s[i] for i in death_idx)
        total_loglik += sum_death_x * beta - d * math.log(s0)
        total_score += sum_death_x - d * (s1 / s0)
        total_info += d * (s2 / s0 - (s1 / s0) ** 2)

    return total_score, total_info, total_loglik


def coxph_breslow(
    times: Sequence[float],
    status: Sequence[int],
    x: Sequence[float],
    strata: Sequence[Any] | None = None,
) -> tuple[float, float, float, float, int]:
    """Stratified Cox proportional hazards regression with Breslow ties handling.

    Args:
        times: Observed times.
        status: Event indicator (1 = event, 0 = censored).
        x: Continuous or binary covariate.
        strata: Stratification group labels.

    Returns:
        Tuple of (beta, se_beta, exp_beta, loglik, iterations).

    Raises:
        ValueError: If input lengths differ or if law constant within every stratum (F-25).
        RuntimeError: If information matrix is singular.
    """
    n = len(times)
    if len(status) != n or len(x) != n:
        raise ValueError("Arrays must have identical lengths")

    if strata is None:
        strata = [0] * n

    distinct_strata = sorted(set(strata))
    _check_strata_variance(distinct_strata, strata, x, n)

    beta = 0.0
    final_loglik = 0.0
    total_info = 0.0
    last_iter = 0

    for iteration in range(1, 51):
        last_iter = iteration
        total_score = 0.0
        total_info = 0.0
        total_loglik = 0.0

        for s in distinct_strata:
            idx_s = [i for i in range(n) if strata[i] == s]
            times_s = [times[i] for i in idx_s]
            status_s = [status[i] for i in idx_s]
            x_s = [x[i] for i in idx_s]
            total_score, total_info, total_loglik = _cox_stratum_contributions(
                (times_s, status_s, x_s), beta, (total_score, total_info, total_loglik)
            )

        final_loglik = total_loglik
        if total_info <= 0.0:
            raise RuntimeError("Singular information matrix in Cox regression")
        step = total_score / total_info
        beta += step
        if abs(step) < _EPS_12:
            break

    se_beta = 1.0 / math.sqrt(total_info)
    exp_beta = math.exp(beta)
    return beta, se_beta, exp_beta, final_loglik, last_iter


def _safe_sigmoid(z: float) -> float:
    """Compute numerically stable logistic sigmoid."""
    if z >= 0.0:
        return 1.0 / (1.0 + math.exp(-z))
    ez = math.exp(z)
    return ez / (1.0 + ez)


def logistic_regression(
    y: Sequence[int], x: Sequence[float]
) -> tuple[float, float, float, float, float, float]:
    """Simple univariate logistic regression via Newton-Raphson.

    Model: P(Y=1|x) = 1 / (1 + exp(-(beta0 + beta1 * x))).

    Args:
        y: Binary outcomes (0 or 1).
        x: Covariate values.

    Returns:
        Tuple of (beta0, beta1, se_beta1, odds_ratio, ci_lo, ci_hi).

    Raises:
        ValueError: If lengths differ or sample size < 4.
        RuntimeError: If information matrix is singular.
    """
    n = len(y)
    if len(x) != n or n < _MIN_LOGISTIC_SAMPLES:
        raise ValueError("Invalid sample for logistic regression")

    beta0, beta1 = 0.0, 0.0
    for _ in range(50):
        p = [_safe_sigmoid(beta0 + beta1 * xi) for xi in x]
        w = [pi * (1.0 - pi) for pi in p]

        u0 = sum(yi - pi for yi, pi in zip(y, p, strict=True))
        u1 = sum((yi - pi) * xi for yi, pi, xi in zip(y, p, x, strict=True))

        i00 = sum(w) + 1e-6
        i01 = sum(wi * xi for wi, xi in zip(w, x, strict=True))
        i11 = sum(wi * xi * xi for wi, xi in zip(w, x, strict=True)) + 1e-6

        det = i00 * i11 - i01 * i01
        if det <= 0.0:
            raise RuntimeError("Singular information matrix in logistic regression")

        step0 = (i11 * u0 - i01 * u1) / det
        step1 = (-i01 * u0 + i00 * u1) / det

        beta0 += step0
        beta1 += step1
        if abs(step0) < _EPS_12 and abs(step1) < _EPS_12:
            break

    p = [_safe_sigmoid(beta0 + beta1 * xi) for xi in x]
    w = [pi * (1.0 - pi) for pi in p]
    i00 = sum(w) + 1e-6
    i01 = sum(wi * xi for wi, xi in zip(w, x, strict=True))
    i11 = sum(wi * xi * xi for wi, xi in zip(w, x, strict=True)) + 1e-6
    det = i00 * i11 - i01 * i01
    se_beta1 = math.sqrt(i00 / det)

    or_val = math.exp(min(beta1, _EXP_MAX_FLOAT)) if beta1 > -_EXP_MAX_FLOAT else 0.0
    z = 1.959963984540054
    exp_lo = beta1 - z * se_beta1
    exp_hi = beta1 + z * se_beta1
    ci_lo = math.exp(exp_lo) if exp_lo > -_EXP_MAX_FLOAT else 0.0
    ci_hi = math.exp(exp_hi) if exp_hi < _EXP_MAX_FLOAT else float("inf")
    return beta0, beta1, se_beta1, or_val, ci_lo, ci_hi


# =============================================================================
# 5. Population Dynamics: Expected Rate and Spread
# =============================================================================


def expected_rate(
    start_type: str, years: int, r_stable: float = 0.00114, f0: int = 500
) -> tuple[float, float, float, float]:
    """Calculates expected growth rate r_exp(start, y) = (ln(E[N_y]/N_0) - s(y)^2/2) / y.

    Args:
        start_type: 'stationary' or 'uniform'.
        years: Horizon in simulation years.
        r_stable: Stable population growth rate.
        f0: Initial founder count of females.

    Returns:
        Tuple of (r_exp, r_stable, transient_term, log_correction).

    Raises:
        ValueError: If start_type is unknown.
    """
    b, d = 0.035, 0.03386
    s_y_sq = (b + d) * (1.0 - math.exp(-r_stable * years)) / (r_stable * f0)
    log_correction = -s_y_sq / (2.0 * years)

    if start_type == "stationary":
        transient_term = 0.0
    elif start_type == "uniform":
        transient_term = -0.000042
    else:
        raise ValueError(f"Unknown start_type: {start_type}")

    r_exp = r_stable + transient_term + log_correction
    return r_exp, r_stable, transient_term, log_correction


def expected_spread(
    f0: int, years: int, r: float = 0.00114, b: float = 0.035, d: float = 0.03386
) -> float:
    """Calculates expected relative population spread s(y).

    Formula: s(y) = sqrt((b + d)(1 - e^(-ry)) / (r F_0)).

    Args:
        f0: Initial female founders.
        years: Horizon in simulation years.
        r: Intrinsic growth rate.
        b: Crude birth rate.
        d: Crude death rate.

    Returns:
        Spread s(y).
    """
    s_sq = (b + d) * (1.0 - math.exp(-r * years)) / (r * f0)
    return math.sqrt(s_sq)


# =============================================================================
# 6. Keyed Deterministic Bootstrap
# =============================================================================


def sha256_bootstrap(
    data: Sequence[float],
    stat_fn: Callable[[Sequence[float]], float],
    n_resamples: int = 10000,
    seed_tag: str = "default",
    alpha: float = 0.10,
) -> tuple[float, float, float]:
    """Deterministic seed-level bootstrap resampler keyed by SHA-256.

    Args:
        data: Sequence of sample values.
        stat_fn: Statistic function taking sequence and returning float.
        n_resamples: Number of bootstrap iterations.
        seed_tag: Unique tag for deterministic SHA-256 keying.
        alpha: Two-sided significance level (alpha=0.10 gives 90% interval).

    Returns:
        Tuple of (observed_statistic, ci_lower, ci_upper).
    """
    n = len(data)
    observed = stat_fn(data)
    replicates: list[float] = []

    for r in range(n_resamples):
        h = hashlib.sha256(f"rehoboam/calib/bootstrap/v0:{seed_tag}:{r}".encode("ascii")).digest()
        resample_indices: list[int] = []
        for i in range(n):
            offset = (i * 4) % 28
            idx_bytes = h[offset : offset + 4]
            int_val = struct.unpack(">I", idx_bytes)[0]
            resample_indices.append(int_val % n)
        resampled_data = [data[idx] for idx in resample_indices]
        replicates.append(stat_fn(resampled_data))

    replicates.sort()
    lo_idx = int((alpha / 2.0) * n_resamples)
    hi_idx = int((1.0 - alpha / 2.0) * n_resamples)
    return observed, replicates[lo_idx], replicates[hi_idx]
