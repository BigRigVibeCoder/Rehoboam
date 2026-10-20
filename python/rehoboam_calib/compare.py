"""V-GOLD-03 statistical comparison tool for Rehoboam calibration (rehoboam_calib compare).

Governed by:
    SPR-028 (M9 validation harness);
    docs/09_verification_and_validation.md §1.2, §3.1, §4 (V-GOLD-03);
    specs/validation/harness_v0.md §3.2 (Zero-Variance Rule, Ruling G14).

Usage:
    python -m rehoboam_calib compare --before DIR --after DIR [--out DIR]
"""

from __future__ import annotations

import argparse
import math
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from rehoboam_calib.read import RunData, read_run_dir
from rehoboam_calib.stats import tost_paired

_AGE_CHILD_LIMIT: float = 15.0
_AGE_ADULT_LIMIT: float = 65.0
_MIN_LIVING_RATIO_FLOOR: float = 1e-12
_MIN_PAIRED_SEEDS: int = 2

# Standard equivalence margins delta = 10% of band width (docs/09 §1.2, V-GOLD-03):
# V-SF-01.r: band [-0.001, 0.003], width = 0.004 -> delta = 0.0004
# V-SF-02:   band [0.35, 0.55],   width = 0.20  -> delta = 0.02
# V-SF-02b:  target 0.3146, tol 0.03, band [0.2846, 0.3446], width = 0.06 -> delta = 0.006
# V-SF-01.pop: band [0.3, 5.0],  width = 4.7   -> delta = 0.47
DEFAULT_DELTAS: dict[str, float] = {
    "V-SF-01.r": 0.0004,
    "V-SF-02": 0.02,
    "V-SF-02b": 0.006,
    "V-SF-01.pop": 0.47,
}


@dataclass(frozen=True)
class ComparisonResult:
    """Outcome of TOST equivalence test for a single metric across paired seeds."""

    metric: str
    delta: float
    verdict: str  # EQUIVALENT, NOT_EQUIVALENT
    mean_diff: float
    se_diff: float
    ci_lo: float
    ci_hi: float
    n_pairs: int


def _discover_runs(root_dir: Path) -> dict[int, RunData]:
    """Find all simulation runs in directory, indexed by seed.

    Checks:
        1. If root_dir contains run.toml, returns {seed: read_run_dir(root_dir)}.
        2. Else scans subdirectories for run.toml and extracts seed.
    """
    runs: dict[int, RunData] = {}
    if (root_dir / "run.toml").is_file():
        run = read_run_dir(root_dir)
        runs[run.metadata.seed] = run
        return runs

    for child in sorted(root_dir.iterdir()):
        if child.is_dir() and (child / "run.toml").is_file():
            try:
                run = read_run_dir(child)
                runs[run.metadata.seed] = run
            except Exception:
                pass

    return runs


def _extract_metric_values(runs: Sequence[RunData], metric: str) -> list[float]:
    """Extract metric value for each run in sequence."""
    values: list[float] = []
    for r in runs:
        if metric == "V-SF-01.r":
            ratio = max(
                r.demographics.living_end / max(r.demographics.living_start, 1),
                _MIN_LIVING_RATIO_FLOOR,
            )
            years = max(r.metadata.years, 1)
            values.append(math.log(ratio) / years)
        elif metric == "V-SF-02":
            n = len(r.demographics.ages_at_death) + r.demographics.living_end
            if n > 0:
                d15 = sum(1 for a in r.demographics.ages_at_death if a < _AGE_CHILD_LIMIT)
                values.append(d15 / n)
            else:
                values.append(0.0)
        elif metric == "V-SF-02b":
            n15 = sum(1 for a in r.demographics.ages_at_death if a >= _AGE_CHILD_LIMIT)
            n65 = sum(1 for a in r.demographics.ages_at_death if a >= _AGE_ADULT_LIMIT)
            if n15 > 0:
                values.append(n65 / n15)
            else:
                values.append(0.0)
        elif metric == "V-SF-01.pop":
            start = max(r.demographics.living_start, 1)
            values.append(r.demographics.living_end / start)
        else:
            raise ValueError(f"Unknown comparison metric: {metric}")
    return values


def compare_ensembles(
    before_dir: Path,
    after_dir: Path,
    deltas: dict[str, float] | None = None,
    alpha: float = 0.05,
) -> tuple[list[ComparisonResult], bool]:
    """Perform V-GOLD-03 TOST equivalence comparison between two run ensembles.

    Enforces the Zero-Variance Rule (Ruling G14):
        Identical runs produce identically zero differences, returning EQUIVALENT with CI [0, 0].

    Returns:
        (results, is_equivalent_overall)
    """
    before_runs = _discover_runs(before_dir)
    after_runs = _discover_runs(after_dir)

    common_seeds = sorted(before_runs.keys() & after_runs.keys())
    if len(common_seeds) < _MIN_PAIRED_SEEDS:
        raise ValueError(
            f"V-GOLD-03 requires at least 2 common paired seeds. Found: {common_seeds}"
        )

    paired_before = [before_runs[s] for s in common_seeds]
    paired_after = [after_runs[s] for s in common_seeds]

    active_deltas = dict(DEFAULT_DELTAS)
    if deltas:
        active_deltas.update(deltas)

    results: list[ComparisonResult] = []
    overall_equivalent = True

    for metric, delta in active_deltas.items():
        vals_before = _extract_metric_values(paired_before, metric)
        vals_after = _extract_metric_values(paired_after, metric)

        verdict, mean_diff, se_diff, ci_lo, ci_hi = tost_paired(
            vals_after, vals_before, delta=delta, alpha=alpha
        )

        if verdict != "EQUIVALENT":
            overall_equivalent = False

        results.append(
            ComparisonResult(
                metric=metric,
                delta=delta,
                verdict=verdict,
                mean_diff=mean_diff,
                se_diff=se_diff,
                ci_lo=ci_lo,
                ci_hi=ci_hi,
                n_pairs=len(common_seeds),
            )
        )

    return results, overall_equivalent


def format_comparison_report(
    before_dir: Path,
    after_dir: Path,
    results: Sequence[ComparisonResult],
    overall_equivalent: bool,
) -> str:
    """Format ASCII report of V-GOLD-03 comparison."""
    header = (
        f"{'Statistic':<14} {'Delta':<10} {'Mean Diff':<12} "
        f"{'SE Diff':<10} {'90% CI':<24} {'Verdict'}"
    )
    lines: list[str] = [
        "=" * 80,
        "V-GOLD-03 Statistical Equivalence Report (TOST)",
        "=" * 80,
        f"Before: {before_dir}",
        f"After:  {after_dir}",
        f"Seeds:  {results[0].n_pairs if results else 0} paired seeds",
        "",
        header,
        "-" * 80,
    ]

    all_diffs_zero = True
    for r in results:
        if r.mean_diff != 0.0 or r.se_diff != 0.0:
            all_diffs_zero = False
        ci_str = f"[{r.ci_lo:+.6f}, {r.ci_hi:+.6f}]"
        lines.append(
            f"{r.metric:<14} {r.delta:<10.6f} {r.mean_diff:<+12.6f} "
            f"{r.se_diff:<10.6f} {ci_str:<24} {r.verdict}"
        )

    lines.append("-" * 80)
    verdict_str = "EQUIVALENT" if overall_equivalent else "NOT EQUIVALENT"
    lines.append(f"OVERALL VERDICT: {verdict_str}")
    if all_diffs_zero and overall_equivalent:
        lines.append("Note: Zero-variance rule applies; all paired differences identically 0.0.")
    elif not overall_equivalent:
        failing = [r.metric for r in results if r.verdict != "EQUIVALENT"]
        lines.append(f"Note: Equivalence rejected on: {', '.join(failing)}.")
    lines.append("=" * 80)
    return "\n".join(lines) + "\n"


def render_comparison_markdown(
    before_dir: Path,
    after_dir: Path,
    results: Sequence[ComparisonResult],
    overall_equivalent: bool,
) -> str:
    """Format Markdown report of V-GOLD-03 comparison."""
    verdict_str = "EQUIVALENT" if overall_equivalent else "NOT EQUIVALENT"
    lines = [
        f"# V-GOLD-03 Equivalence Comparison: {verdict_str}",
        "",
        f"- **Before:** `{before_dir}`",
        f"- **After:** `{after_dir}`",
        f"- **Paired Seeds:** {results[0].n_pairs if results else 0}",
        "",
        "| Statistic | Delta (Margin) | Mean Diff | SE Diff | 90% CI | Verdict |",
        "| :--- | :--- | :--- | :--- | :--- | :--- |",
    ]
    for r in results:
        ci_str = f"[{r.ci_lo:+.6f}, {r.ci_hi:+.6f}]"
        row = (
            f"| `{r.metric}` | `{r.delta:.6f}` | `{r.mean_diff:+.6f}` | "
            f"`{r.se_diff:.6f}` | `{ci_str}` | **{r.verdict}** |"
        )
        lines.append(row)
    lines.append("")
    return "\n".join(lines) + "\n"


def add_compare_subparser(subparsers: argparse._SubParsersAction[Any]) -> None:
    """Register 'compare' subcommand into CLI argument parser."""
    p = subparsers.add_parser(
        "compare",
        help="Compare two run ensembles for statistical equivalence (V-GOLD-03)",
    )
    p.add_argument("--before", required=True, help="Baseline/before ensemble directory")
    p.add_argument("--after", required=True, help="Candidate/after ensemble directory")
    p.add_argument("--out", help="Optional output directory to write comparison reports")
    p.add_argument(
        "--alpha",
        type=float,
        default=0.05,
        help="TOST significance level alpha (default: 0.05)",
    )


def handle_compare(
    before_str: str,
    after_str: str,
    out_dir_str: str | None = None,
    alpha: float = 0.05,
) -> int:
    """Execute compare subcommand and print reports."""
    before_dir = Path(before_str).resolve()
    after_dir = Path(after_str).resolve()

    if not before_dir.exists():
        sys.stderr.write(f"Error: --before directory does not exist: {before_dir}\n")
        return 1
    if not after_dir.exists():
        sys.stderr.write(f"Error: --after directory does not exist: {after_dir}\n")
        return 1

    results, is_equivalent = compare_ensembles(before_dir, after_dir, alpha=alpha)
    report_txt = format_comparison_report(before_dir, after_dir, results, is_equivalent)
    sys.stdout.write(report_txt)

    if out_dir_str:
        out_dir = Path(out_dir_str)
        out_dir.mkdir(parents=True, exist_ok=True)
        (out_dir / "comparison.txt").write_text(report_txt, encoding="utf-8")
        report_md = render_comparison_markdown(before_dir, after_dir, results, is_equivalent)
        (out_dir / "comparison.md").write_text(report_md, encoding="utf-8")

    return 0 if is_equivalent else 1
