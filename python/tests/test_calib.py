"""Tests for Rehoboam M9 calibration package (rehoboam_calib).

Governed by:
    SPR-028 (M9 validation harness);
    docs/09_verification_and_validation.md §1.2, §3.1, §3.7, §4;
    specs/validation/harness_v0.md.
"""

from __future__ import annotations

import math
import os
import subprocess
from pathlib import Path

import duckdb
import pytest

from rehoboam_calib.bands import evaluate_band, evaluate_runs, load_bands
from rehoboam_calib.read import parse_vitals_text, read_run_dir
from rehoboam_calib.report import (
    format_report_v0,
    generate_synthetic_reference_report,
)
from rehoboam_calib.stats import (
    beta_quantile,
    clopper_pearson_ci,
    coxph_breslow,
    kaplan_meier,
    logistic_regression,
    poisson_exact_ci,
    qchisq,
    sha256_bootstrap,
    t_quantile,
    tost_paired,
    wilson_score_ci,
)

REPO_ROOT = Path(__file__).resolve().parents[2]
VENV_PYTHON = REPO_ROOT / "python" / ".venv" / "bin" / "python"
GOLDEN_HARNESS_V0 = REPO_ROOT / "specs" / "validation" / "golden" / "harness_v0.txt"


# -----------------------------------------------------------------------------
# 1. Closed-Form Statistical Primitives & Known Answers
# -----------------------------------------------------------------------------


def test_clopper_pearson_closed_form() -> None:
    """Clopper-Pearson 0/20 at 95% has exact closed-form upper bound 1 - 0.05^(1/20)."""
    k = 0
    n = 20
    alpha = 0.05
    lo, hi = clopper_pearson_ci(k, n, alpha=alpha)
    expected_hi = 1.0 - (alpha / 2.0) ** (1.0 / n)
    assert lo == 0.0
    assert abs(hi - expected_hi) < 1e-15


def test_wilson_score_properties() -> None:
    """Wilson score interval is bounded within [0, 1] and symmetric around 0.5."""
    lo, hi = wilson_score_ci(50, 100, alpha=0.05)
    assert abs((lo + hi) / 2.0 - 0.5) < 1e-12
    assert 0.0 <= lo < hi <= 1.0


def test_poisson_exact_closed_form() -> None:
    """Poisson exact 0 count at 95% has exact two-sided upper bound -ln(0.025)."""
    k = 0
    alpha = 0.05
    lo, hi = poisson_exact_ci(k, exposure=1.0, alpha=alpha)
    assert lo == 0.0
    assert abs(hi - (-math.log(alpha / 2.0))) < 1e-15


def test_tost_zero_variance_rule() -> None:
    """Ruling G14: TOST handles identical arrays without zero-division error."""
    identical = [1.0, 1.0, 1.0, 1.0, 1.0]
    verdict, _, _, lo, hi = tost_paired(identical, identical, delta=0.05)
    assert verdict == "EQUIVALENT"
    assert lo == 0.0
    assert hi == 0.0

    # Constant shift d = 0.02 < 0.05 -> EQUIVALENT
    shifted_pass = [x + 0.02 for x in identical]
    v_pass, _, _, lo_p, hi_p = tost_paired(shifted_pass, identical, delta=0.05)
    assert v_pass == "EQUIVALENT"
    assert abs(lo_p - 0.02) < 1e-15
    assert abs(hi_p - 0.02) < 1e-15

    # Constant shift d = 0.06 >= 0.05 -> NOT_EQUIVALENT
    shifted_fail = [x + 0.06 for x in identical]
    v_fail, _, _, lo_f, hi_f = tost_paired(shifted_fail, identical, delta=0.05)
    assert v_fail == "NOT_EQUIVALENT"
    assert abs(lo_f - 0.06) < 1e-15
    assert abs(hi_f - 0.06) < 1e-15


def test_quantile_functions() -> None:
    """Student-t, Beta, and Chi-squared quantiles match standard reference values."""
    # Student-t
    assert abs(t_quantile(0.975, 7) - 2.364624251592785) < 1e-9
    assert abs(t_quantile(0.9995, 31) - 3.633456349758372) < 1e-9

    # Beta
    assert abs(beta_quantile(0.5, 2.0, 2.0) - 0.5) < 1e-9

    # Chi-squared
    assert abs(qchisq(0.95, 2) - 5.991464547107979) < 1e-9


# -----------------------------------------------------------------------------
# 2. Survival Analysis & Regression Parity
# -----------------------------------------------------------------------------


def test_kaplan_meier_uncensored_empirical() -> None:
    """Kaplan-Meier survival curve on uncensored data matches 1 - empirical CDF."""
    times = [1.0, 2.0, 3.0, 4.0, 5.0]
    status = [1, 1, 1, 1, 1]
    res = kaplan_meier(times, status)
    for i, pt in enumerate(res):
        expected_s = 1.0 - (i + 1) / 5.0
        assert abs(pt.survival - expected_s) < 1e-15


def test_gehan_cox_regression_r_parity() -> None:
    """Gehan 6-MP Cox regression reproduces R survival package reference."""
    gehan_6mp_times = [
        6,
        6,
        6,
        6,
        7,
        9,
        10,
        10,
        11,
        13,
        16,
        17,
        19,
        20,
        22,
        23,
        25,
        32,
        32,
        34,
        35,
    ]
    gehan_6mp_status = [1, 1, 1, 0, 1, 0, 1, 0, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0]
    gehan_ctrl_times = [
        1,
        1,
        2,
        2,
        3,
        4,
        4,
        5,
        5,
        8,
        8,
        8,
        8,
        11,
        11,
        12,
        12,
        15,
        17,
        22,
        23,
    ]
    gehan_ctrl_status = [1] * 21

    all_times = gehan_ctrl_times + gehan_6mp_times
    all_status = gehan_ctrl_status + gehan_6mp_status
    treat = [0.0] * 21 + [1.0] * 21

    beta, se, _, _, _ = coxph_breslow(all_times, all_status, treat)
    r_beta = -1.509191412585879
    r_se = 0.409564406366741

    assert abs(beta - r_beta) / abs(r_beta) < 1e-9
    assert abs(se - r_se) / r_se < 1e-9


def test_stratified_cox_f25_unidentifiable() -> None:
    """Finding F-25: within-stratum constant law raises ValueError (unidentifiable)."""
    times = [1.0, 2.0, 3.0, 4.0]
    status = [1, 1, 1, 1]
    # Stratum 1 has all x=0.0; stratum 2 has all x=1.0
    x = [0.0, 0.0, 1.0, 1.0]
    strata = [1, 1, 2, 2]

    with pytest.raises(ValueError, match="covariate unidentified"):
        coxph_breslow(times, status, x, strata=strata)


def test_logistic_regression() -> None:
    """Logistic regression converges on data with realistic overlap."""
    x = [-2.0, -1.0, -0.5, 0.5, 1.0, 2.0]
    y = [0, 0, 1, 0, 1, 1]
    _b0, b1, se1, odds_ratio, ci_lo, ci_hi = logistic_regression(y, x)
    assert b1 > 0.0
    assert se1 > 0.0
    assert odds_ratio > 1.0
    assert 0.0 <= ci_lo <= odds_ratio <= ci_hi


def test_keyed_deterministic_bootstrap() -> None:
    """SHA-256 keyed bootstrap is bit-identical across runs."""
    data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
    key = "deterministic_key_seed_123"
    ci1 = sha256_bootstrap(data, lambda d: sum(d) / len(d), seed_tag=key, n_resamples=200)
    ci2 = sha256_bootstrap(data, lambda d: sum(d) / len(d), seed_tag=key, n_resamples=200)
    assert ci1 == ci2


# -----------------------------------------------------------------------------
# 3. DuckDB Channel A Reading & Parity
# -----------------------------------------------------------------------------


def test_duckdb_hamlet_reading_parity() -> None:
    """DuckDB reader over Channel A sealed Parquet matches independent DuckDB query."""
    hamlet_dir = REPO_ROOT / "target" / "golden" / "hamlet_run"
    if not hamlet_dir.exists():
        pytest.skip("target/golden/hamlet_run not present")

    run_data = read_run_dir(hamlet_dir)
    assert run_data.metadata.world == "hamlet"
    assert run_data.metadata.years == 200
    assert run_data.demographics.total_entries == 60
    assert run_data.demographics.births_count == 0
    assert run_data.demographics.founders_count == 60
    assert run_data.demographics.deaths_count == 60
    assert run_data.demographics.living_start == 60
    assert run_data.demographics.living_end == 0
    assert len(run_data.demographics.ages_at_death) == 60

    # Independent DuckDB verification query over the exact same parquet segments
    entries_glob = str(hamlet_dir / "stream" / "t*" / "A" / "y*" / "life_course_entry.parquet")
    exits_glob = str(hamlet_dir / "stream" / "t*" / "A" / "y*" / "life_course_exit.parquet")
    con = duckdb.connect()
    row_entries = con.execute(f"SELECT count(*) FROM '{entries_glob}'").fetchone()
    row_deaths = con.execute(f"SELECT count(*) FROM '{exits_glob}'").fetchone()
    assert row_entries is not None
    assert row_deaths is not None
    indep_entries = int(row_entries[0])
    indep_deaths = int(row_deaths[0])

    assert run_data.demographics.total_entries == indep_entries
    assert run_data.demographics.deaths_count == indep_deaths


def test_vitals_parser_order_invariance() -> None:
    """Ruling G11 / G14: vitals lines are parsed by key, invariant to column order."""
    line_normal = "vitals: tip=5184000 pace=100.0 ticks_per_sec=51840 serial_pct=0.0 wall_ms=120"
    line_reordered = "vitals: wall_ms=120 ticks_per_sec=51840 tip=5184000 serial_pct=0.0 pace=100.0"

    v1 = parse_vitals_text(line_normal)
    v2 = parse_vitals_text(line_reordered)

    assert v1.tip == v2.tip == "5184000"
    assert v1.pace == v2.pace == 100.0
    assert v1.wall_ms == v2.wall_ms == 120.0


# -----------------------------------------------------------------------------
# 4. Sealed Rule Enforcement (A-04, D61 item 4)
# -----------------------------------------------------------------------------


def test_sealed_rule_zero_numerical_leakage() -> None:
    """Held-out bands (V-SF-06c, V-SF-07) emit SEALED with zero numerical leakage."""
    catalog = load_bands()
    held_out_ids = ["V-SF-06c", "V-SF-07"]

    for bid in held_out_ids:
        assert bid in catalog, f"Missing held-out band {bid} in catalog"
        bdef = catalog[bid]
        verdict = evaluate_band(bdef, sample_stat=0.42, ci=(0.35, 0.49), n_samples=50)

        assert verdict.verdict == "SEALED"
        assert verdict.stat is None
        assert verdict.ci_lo is None
        assert verdict.ci_hi is None
        assert "A-04" in verdict.note


def test_sealed_rule_in_report_output() -> None:
    """Grep verification: no number follows SEALED in formatted report."""
    catalog = load_bands()
    v6c = evaluate_band(catalog["V-SF-06c"], None, (None, None), 10)
    v07 = evaluate_band(catalog["V-SF-07"], None, (None, None), 10)

    hamlet_dir = REPO_ROOT / "target" / "golden" / "hamlet_run"
    run = read_run_dir(hamlet_dir)
    report_text = format_report_v0(run.metadata, (1, 1), 1, [v6c, v07])

    for line in report_text.splitlines():
        if "BAND V-SF-06c:" in line or "BAND V-SF-07:" in line:
            assert line.endswith("SEALED (A-04; held-out, docs/09 §3.1)")
            assert "stat=" not in line
            assert "ci_lo=" not in line
            assert "[" not in line


# -----------------------------------------------------------------------------
# 5. V-SCALE-02 Small-N Scale Verification
# -----------------------------------------------------------------------------


def test_vscale02_on_small_worlds() -> None:
    """V-SCALE-02: solo, household, hamlet report INSUFFICIENT (N < 200 required)."""
    solo_dir = REPO_ROOT / "target" / "golden" / "solo_run"
    household_dir = REPO_ROOT / "target" / "golden" / "household_run"
    hamlet_dir = REPO_ROOT / "target" / "golden" / "hamlet_run"

    for r_dir in [solo_dir, household_dir, hamlet_dir]:
        if not r_dir.exists():
            continue
        run = read_run_dir(r_dir)
        verdicts = evaluate_runs([run], filter_band_ids=["V-SCALE-02"])
        assert len(verdicts) == 1
        assert verdicts[0].band_id == "V-SCALE-02"
        assert verdicts[0].verdict == "INSUFFICIENT"
        assert verdicts[0].n_samples < 200
        assert "samples" in verdicts[0].note


# -----------------------------------------------------------------------------
# 6. Reference Synthetic Report Parity
# -----------------------------------------------------------------------------


def test_synthetic_reference_report_byte_parity() -> None:
    """generate_synthetic_reference_report() matches golden harness_v0.txt byte-for-byte."""
    assert GOLDEN_HARNESS_V0.exists(), "Missing golden harness_v0.txt"
    expected = GOLDEN_HARNESS_V0.read_bytes()
    actual = generate_synthetic_reference_report().encode("utf-8")
    assert actual == expected


# -----------------------------------------------------------------------------
# 7. CLI Subprocess Tests & Determinism
# -----------------------------------------------------------------------------


def test_cli_report_synthetic_golden() -> None:
    """CLI python -I -m rehoboam_calib report --synthetic emits exact golden bytes."""
    cmd = [str(VENV_PYTHON), "-I", "-m", "rehoboam_calib", "report", "--synthetic"]
    res = subprocess.run(cmd, cwd=str(REPO_ROOT), capture_output=True, check=True)
    expected = GOLDEN_HARNESS_V0.read_bytes()
    assert res.stdout == expected


def test_cli_report_runs_vscale02() -> None:
    """CLI report on solo, household, hamlet with V-SCALE-02 prints INSUFFICIENT."""
    cmd = [
        str(VENV_PYTHON),
        "-I",
        "-m",
        "rehoboam_calib",
        "report",
        "--runs",
        "target/golden/solo_run",
        "target/golden/household_run",
        "target/golden/hamlet_run",
        "--bands",
        "V-SCALE-02",
    ]
    res = subprocess.run(cmd, cwd=str(REPO_ROOT), capture_output=True, text=True, check=True)
    assert res.returncode == 0
    # Every rung reports INSUFFICIENT
    assert res.stdout.count("BAND V-SCALE-02: INSUFFICIENT") == 3
    assert "samples 1 < 200" in res.stdout
    assert "samples 6 < 200" in res.stdout
    assert "samples 60 < 200" in res.stdout


def test_cli_determinism_across_pythonhashseed() -> None:
    """Report output is byte-identical across processes under differing PYTHONHASHSEED."""
    cmd = [
        str(VENV_PYTHON),
        "-P",
        "-s",
        "-m",
        "rehoboam_calib",
        "report",
        "--runs",
        "target/golden/hamlet_run",
        "--bands",
        "V-SCALE-02",
    ]
    env1 = {**os.environ, "PYTHONHASHSEED": "0"}
    env2 = {**os.environ, "PYTHONHASHSEED": "42"}

    res1 = subprocess.run(cmd, cwd=str(REPO_ROOT), env=env1, capture_output=True, check=True)
    res2 = subprocess.run(cmd, cwd=str(REPO_ROOT), env=env2, capture_output=True, check=True)

    assert res1.stdout == res2.stdout
    assert res1.returncode == res2.returncode == 0


def test_no_environment_or_clock_reads() -> None:
    """Validation harness reads no environment variables and no system clock."""
    calib_pkg = REPO_ROOT / "python" / "rehoboam_calib"
    for py_file in calib_pkg.glob("*.py"):
        text = py_file.read_text(encoding="utf-8")
        assert "os.environ" not in text, f"Illegal os.environ in {py_file.name}"
        assert "os.getenv" not in text, f"Illegal os.getenv in {py_file.name}"
        assert "time.time" not in text, f"Illegal time.time in {py_file.name}"
        assert "datetime.now" not in text, f"Illegal datetime.now in {py_file.name}"
