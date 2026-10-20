"""CLI ensemble runner for Rehoboam calibration (rehoboam_calib ensemble).

Governed by:
    SPR-028 (M9 validation harness);
    docs/09_verification_and_validation.md §3.1, §3.7, §4.

Usage:
    python -m rehoboam_calib ensemble --world <w> --seeds 1-32 --years Y --jobs J --out DIR
"""

from __future__ import annotations

import argparse
import concurrent.futures
import shutil
import subprocess
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from rehoboam_calib.bands import evaluate_runs, load_bands
from rehoboam_calib.read import RunData, read_run_dir
from rehoboam_calib.report import format_report_v0, render_markdown

_SPLIT_PARTS: int = 2


def parse_seed_range(spec: str) -> list[int]:
    """Parse a seed specification string into a sorted list of unique integers.

    Supported formats:
        - "1-32" (inclusive range)
        - "1,2,3,4" (comma-separated list)
        - "1-4,7,9-12" (combination of ranges and numbers)
        - "5" (single integer)

    Raises:
        ValueError: On empty or invalid seed specification.
    """
    cleaned = spec.strip()
    if not cleaned:
        raise ValueError("Seed specification cannot be empty")

    seeds: set[int] = set()
    for part in cleaned.split(","):
        p = part.strip()
        if not p:
            continue
        if "-" in p:
            sub = p.split("-", 1)
            if (
                len(sub) != _SPLIT_PARTS
                or not sub[0].strip().isdigit()
                or not sub[1].strip().isdigit()
            ):
                raise ValueError(f"Invalid range in seeds spec: '{p}'")
            start = int(sub[0].strip())
            end = int(sub[1].strip())
            if start > end:
                raise ValueError(f"Invalid range start > end in seeds spec: '{p}'")
            seeds.update(range(start, end + 1))
        else:
            if not p.isdigit():
                raise ValueError(f"Invalid seed number in seeds spec: '{p}'")
            seeds.add(int(p))

    if not seeds:
        raise ValueError(f"No seeds parsed from specification: '{spec}'")
    return sorted(seeds)


def find_rh_cli(candidate: str | Path | None = None) -> Path:
    """Find the rh-cli executable binary.

    Searches:
        1. Explicit candidate path if provided.
        2. target/release/rh-cli under repository root.
        3. target/debug/rh-cli under repository root.
        4. System PATH via shutil.which("rh-cli").

    Raises:
        FileNotFoundError: If rh-cli binary cannot be found.
    """
    if candidate is not None:
        p = Path(candidate).resolve()
        if p.is_file():
            return p
        raise FileNotFoundError(f"Specified rh-cli binary not found: {candidate}")

    root = Path(__file__).resolve().parents[2]
    release_bin = root / "target" / "release" / "rh-cli"
    if release_bin.is_file():
        return release_bin

    debug_bin = root / "target" / "debug" / "rh-cli"
    if debug_bin.is_file():
        return debug_bin

    which_bin = shutil.which("rh-cli")
    if which_bin is not None:
        return Path(which_bin)

    raise FileNotFoundError(
        "Could not find rh-cli binary. Build it with `cargo build -p rh-cli` or specify --rust-bin."
    )


@dataclass(frozen=True)
class SeedResult:
    """Outcome of running a single seed in an ensemble."""

    seed: int
    run_dir: Path
    returncode: int
    stdout: str
    stderr: str
    error: str | None = None


@dataclass(frozen=True)
class SeedRunConfig:
    """Execution parameters for a single seed run."""

    rh_cli_bin: Path
    world: str
    seed: int
    years: int
    run_dir: Path
    extra_flags: Sequence[str] | None = None


def _run_single_seed(cfg: SeedRunConfig) -> SeedResult:
    """Execute rh-cli run for a single seed with vitals and thread count 1."""
    cfg.run_dir.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(cfg.rh_cli_bin),
        "run",
        cfg.world,
        "--seed",
        str(cfg.seed),
        "--threads",
        "1",
        "--years",
        str(cfg.years),
        "--run-dir",
        str(cfg.run_dir),
        "--vitals",
    ]
    if cfg.extra_flags:
        cmd.extend(cfg.extra_flags)

    try:
        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=False,
        )
    except Exception as exc:
        return SeedResult(
            seed=cfg.seed,
            run_dir=cfg.run_dir,
            returncode=1,
            stdout="",
            stderr="",
            error=f"Process launch error: {exc}",
        )

    stdout_file = cfg.run_dir / "stdout.log"
    stderr_file = cfg.run_dir / "stderr.log"
    vitals_file = cfg.run_dir / "vitals.txt"

    stdout_file.write_text(proc.stdout, encoding="utf-8")
    stderr_file.write_text(proc.stderr, encoding="utf-8")

    vitals_lines = [
        line for line in proc.stdout.splitlines() if line.startswith("vitals:") or "pace " in line
    ]
    if vitals_lines:
        vitals_file.write_text("\n".join(vitals_lines) + "\n", encoding="utf-8")

    error_msg = None
    if proc.returncode != 0:
        error_msg = (
            proc.stderr.strip()
            or proc.stdout.strip()
            or f"Process exited with code {proc.returncode}"
        )

    return SeedResult(
        seed=cfg.seed,
        run_dir=cfg.run_dir,
        returncode=proc.returncode,
        stdout=proc.stdout,
        stderr=proc.stderr,
        error=error_msg,
    )


@dataclass(frozen=True)
class EnsembleConfig:
    """Configuration for an ensemble run."""

    world: str
    seeds: Sequence[int]
    years: int
    jobs: int
    out_dir: Path
    rh_cli_bin: Path | None = None
    bands: Sequence[str] | None = None
    bands_config: str | Path | None = None
    extra_flags: Sequence[str] | None = None


def run_ensemble(cfg: EnsembleConfig) -> int:
    """Run simulation ensemble across seeds and generate calibration report.

    Fail-fast rule:
        Any failed seed immediately aborts the ensemble and names the failing seed and error.
    """
    cli_bin = find_rh_cli(cfg.rh_cli_bin)
    cfg.out_dir.mkdir(parents=True, exist_ok=True)

    max_workers = max(1, cfg.jobs)
    runs: list[RunData] = []
    catalog = load_bands(cfg.bands_config)

    with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures_to_seed = {
            executor.submit(
                _run_single_seed,
                SeedRunConfig(
                    rh_cli_bin=cli_bin,
                    world=cfg.world,
                    seed=s,
                    years=cfg.years,
                    run_dir=cfg.out_dir / f"seed-{s}",
                    extra_flags=cfg.extra_flags,
                ),
            ): s
            for s in cfg.seeds
        }

        for future in concurrent.futures.as_completed(futures_to_seed):
            res = future.result()
            if res.returncode != 0:
                for f in futures_to_seed:
                    f.cancel()
                sys.stderr.write(
                    f"\n[FAIL-FAST] Ensemble aborted! Seed {res.seed} failed with error:\n"
                    f"{res.error}\n"
                )
                return 1

    for s in cfg.seeds:
        seed_dir = cfg.out_dir / f"seed-{s}"
        runs.append(read_run_dir(seed_dir))

    filter_ids = list(cfg.bands) if cfg.bands else None
    verdicts = evaluate_runs(runs, bands_catalog=catalog, filter_band_ids=filter_ids)
    seed_bounds = (min(cfg.seeds), max(cfg.seeds))
    meta = runs[0].metadata

    report_txt = format_report_v0(meta, seed_bounds, len(cfg.seeds), verdicts)
    report_md = render_markdown(meta, verdicts, runs[0].demographics)

    (cfg.out_dir / "report.txt").write_text(report_txt, encoding="utf-8")
    (cfg.out_dir / "report.md").write_text(report_md, encoding="utf-8")

    sys.stdout.write(report_txt)
    has_failure = any(v.verdict == "FAIL" for v in verdicts)
    return 1 if has_failure else 0


def add_ensemble_subparser(subparsers: argparse._SubParsersAction[Any]) -> None:
    """Register 'ensemble' subcommand into CLI argument parser."""
    p = subparsers.add_parser("ensemble", help="Run simulation ensemble across seeds")
    p.add_argument(
        "--world",
        required=True,
        help="World name or fixture (e.g. fixture-demography_1000)",
    )
    p.add_argument("--seeds", required=True, help="Seed range or list (e.g. 1-32 or 1,2,3)")
    p.add_argument("--years", type=int, required=True, help="Simulation duration in years")
    p.add_argument(
        "--jobs",
        type=int,
        default=4,
        help="Number of concurrent simulation jobs (default: 4)",
    )
    p.add_argument("--out", required=True, help="Output directory for seed runs and report")
    p.add_argument("--rust-bin", help="Path to rh-cli executable binary")
    p.add_argument(
        "--bands",
        nargs="+",
        help="Filter band IDs (e.g. V-SF-01.r,V-SF-02,V-SF-02b)",
    )
    p.add_argument("--bands-config", help="Path to bands TOML catalog")
