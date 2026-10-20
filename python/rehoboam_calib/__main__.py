"""CLI entry point for Rehoboam M9 calibration package (rehoboam_calib).

Governed by:
    SPR-028 (M9 validation harness);
    docs/09_verification_and_validation.md §3.1, §3.7, §4.

Usage:
    python -I -m rehoboam_calib report --synthetic [--out <dir>]
    python -I -m rehoboam_calib report --runs <dir>... --bands <ids> [--out <dir>]
"""

from __future__ import annotations

import argparse
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Any

from rehoboam_calib.bands import evaluate_runs, load_bands
from rehoboam_calib.read import RunData, read_run_dir
from rehoboam_calib.report import (
    format_report_v0,
    generate_synthetic_reference_report,
    render_markdown,
)


def _parse_band_ids(bands_args: Sequence[str] | None) -> list[str]:
    """Parse and clean comma- or whitespace-separated band IDs."""
    if not bands_args:
        return []
    result: list[str] = []
    for item in bands_args:
        for part in item.split(","):
            cleaned = part.strip()
            if cleaned and cleaned not in result:
                result.append(cleaned)
    return result


def _handle_synthetic(out_dir: str | None) -> int:
    """Handle generation of synthetic reference harness report."""
    report_text = generate_synthetic_reference_report()
    sys.stdout.write(report_text)

    if out_dir:
        out_path = Path(out_dir)
        out_path.mkdir(parents=True, exist_ok=True)
        (out_path / "report.txt").write_text(report_text, encoding="utf-8")
        md_text = f"# M9 Validation Report: Synthetic Reference\n\n```\n{report_text}\n```\n"
        (out_path / "report.md").write_text(md_text, encoding="utf-8")

    return 0 if "VALIDATION HARNESS: PASS" in report_text else 1


def _evaluate_world_runs(
    world_runs: list[RunData],
    bands_catalog: dict[str, dict[str, Any]],
    band_ids: list[str],
) -> tuple[str, str, bool]:
    """Evaluate runs for a single world and format reports.

    Returns:
        (report_txt, report_md, has_failure)
    """
    filter_ids = band_ids if band_ids else None
    verdicts = evaluate_runs(world_runs, bands_catalog=bands_catalog, filter_band_ids=filter_ids)
    seeds = (
        min(r.metadata.seed for r in world_runs),
        max(r.metadata.seed for r in world_runs),
    )
    seed_count = len(world_runs)
    meta = world_runs[0].metadata

    report_txt = format_report_v0(meta, seeds, seed_count, verdicts)
    report_md = render_markdown(meta, verdicts, world_runs[0].demographics)
    has_failure = any(v.verdict == "FAIL" for v in verdicts)

    return report_txt, report_md, has_failure


def _handle_runs(
    run_paths: Sequence[str],
    bands_args: Sequence[str] | None,
    bands_config: str | None,
    out_dir: str | None,
) -> int:
    """Handle evaluation and reporting for observed simulation runs."""
    band_ids = _parse_band_ids(bands_args)
    catalog = load_bands(bands_config)

    runs = [read_run_dir(p) for p in run_paths]
    runs_by_world: dict[str, list[RunData]] = {}
    for r in runs:
        runs_by_world.setdefault(r.metadata.world, []).append(r)

    all_txt_reports: list[str] = []
    any_failure = False

    for world, world_runs in runs_by_world.items():
        txt, md, has_fail = _evaluate_world_runs(world_runs, catalog, band_ids)
        all_txt_reports.append(txt)
        if has_fail:
            any_failure = True

        sys.stdout.write(txt)

        if out_dir:
            out_path = Path(out_dir)
            out_path.mkdir(parents=True, exist_ok=True)
            if len(runs_by_world) > 1:
                (out_path / f"report_{world}.txt").write_text(txt, encoding="utf-8")
                (out_path / f"report_{world}.md").write_text(md, encoding="utf-8")
            else:
                (out_path / "report.txt").write_text(txt, encoding="utf-8")
                (out_path / "report.md").write_text(md, encoding="utf-8")

    if out_dir and len(runs_by_world) > 1:
        combined_txt = "\n".join(all_txt_reports)
        out_path = Path(out_dir)
        (out_path / "report.txt").write_text(combined_txt, encoding="utf-8")

    return 1 if any_failure else 0


def build_parser() -> argparse.ArgumentParser:
    """Build command line argument parser for rehoboam_calib."""
    parser = argparse.ArgumentParser(
        prog="rehoboam_calib",
        description="Rehoboam M9 Calibration and Validation Harness (SPR-028)",
    )
    subparsers = parser.add_subparsers(dest="subcommand")

    report_p = subparsers.add_parser("report", help="Generate calibration report")
    report_p.add_argument(
        "--synthetic",
        action="store_true",
        help="Generate synthetic reference validation harness report",
    )
    report_p.add_argument(
        "--runs",
        nargs="+",
        help="One or more simulation run directories to evaluate",
    )
    report_p.add_argument(
        "--bands",
        nargs="+",
        help="Band IDs or prefixes (e.g. V-SCALE-02 or V-SF-01,V-SF-02,V-SF-03)",
    )
    report_p.add_argument(
        "--bands-config",
        help="Path to bands TOML catalog (defaults to harness_v0.toml)",
    )
    report_p.add_argument(
        "--out",
        help="Directory to write output report.txt and report.md",
    )

    return parser


def main(argv: Sequence[str] | None = None) -> int:
    """CLI main entry point for rehoboam_calib."""
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.subcommand != "report":
        parser.print_help()
        return 0

    if args.synthetic:
        return _handle_synthetic(args.out)

    if not args.runs:
        sys.stderr.write("Error: --runs <dir>... or --synthetic must be specified\n")
        return 1

    return _handle_runs(args.runs, args.bands, args.bands_config, args.out)


if __name__ == "__main__":
    sys.exit(main())
