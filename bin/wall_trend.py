"""Wall-clock trends of benches 7 and 8 from criterion's sample data (SPR-009 T-02b, T-04).

criterion's `wall` bench target (crates/rh-bench/benches/wall.rs) writes each case's samples to
`target/criterion/<group>/<case>/<rows>/new/sample.json` (iteration counts and total times). This
script takes those samples, not criterion's bootstrap summary (Architect ruling, 2026-10-16):

    summarize  Per case: the median time per iteration over its samples (at least 5; RUN-002 §5)
               and the spread, (max - min) / median of the per-iteration times. Writes them with
               the run conditions (RUN-002 §5; SPR-009 T-04: machine, CPU model, logical CPUs,
               kernel, rustc, power profile, temperature) in the trend format below.
    check      Compares a summary with a committed trend file: a median more than 10% above the
               trend's is an ALERT (docs/08 §9). Without --gate it only reports; with --gate an
               alert fails (exit 1). Wall-clock results gate only on adjutant, once T-04 has
               measured its spread (SPR-009 T-02b; RUN-002 §5). Another machine, CPU or rustc is
               never compared silently: exit 3, "machine changed: new trend baseline required".
               A faster run is reported, never written back.
    combine    Merges the summaries of N separate runs of one session (SPR-009 T-04; RUN-002 §5):
               per case, the median of the run medians and the run-to-run spread, (max - min) /
               median of the run medians. That spread, not the within-run one, decides whether
               a wall-clock gate is possible on the machine (well under 10%). The runs must share
               their identity conditions.
    record     Writes a summary as the trend file (the only way one is written).

Trend format `rehoboam/rh-bench/wall/v1`, one record per line, `#` lines are comments:

    format rehoboam/rh-bench/wall/v1
    condition <name> <value>              identity: machine, cpu, cpus, rustc; the rest are notes
    bench <group>/<case>/<rows> <median ns> <spread %> <samples>

Usage:  python -I bin/wall_trend.py summarize --machine adjutant --out artifacts/perf/wall.txt
        python -I bin/wall_trend.py check --trend crates/rh-bench/baselines/wall_adjutant_v1.txt \
            --measured artifacts/perf/wall.txt [--gate]
Exit:   0 pass, 1 an alert with --gate, 2 usage or bad input, 3 machine changed.
Standard library only (GOV-003 §3).

REF: docs/08_performance_and_capacity.md §9; RUN-001 §6; RUN-002 §5; docs/09 V-PERF-04;
CODEX/05_PROJECT/SPR-009_Benchmarks.md T-02b, T-04.
"""

import argparse
import json
import os
import platform
import shutil
import statistics
import subprocess
import sys
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path

FORMAT = "rehoboam/rh-bench/wall/v1"
ROOT = Path(__file__).resolve().parents[1]
# REF: docs/08 §9 and SPR-009 T-02b: alert above 10%.
LIMIT_PERCENT = 10
# REF: RUN-002 §5: at least 5 repeated runs per wall-clock bench.
MIN_SAMPLES = 5
# The conditions that make two runs comparable; the others are recorded as notes (RUN-002 §5).
IDENTITY = ("machine", "cpu", "cpus", "rustc")
# Fields of a `condition` line (at least) and of a `bench` line (exactly).
CONDITION_FIELDS, BENCH_FIELDS = 3, 5

OK, ALERT, USAGE, MACHINE = 0, 1, 2, 3


class TrendError(Exception):
    """Bad input (exit 2)."""


@dataclass
class Trend:
    """A summary or trend file: run conditions and per-case medians."""

    conditions: dict[str, str] = field(default_factory=dict)
    # key -> (median ns per iteration, spread %, samples)
    benches: dict[str, tuple[float, float, int]] = field(default_factory=dict)

    def render(self) -> str:
        """The canonical text."""
        lines = [
            f"# {FORMAT}: wall-clock medians of the criterion benches (SPR-009 T-02b).",
            "# Written by `python -I bin/wall_trend.py`, never by hand.",
            f"format {FORMAT}",
        ]
        lines += [f"condition {k} {v}" for k, v in self.conditions.items()]
        lines += [f"bench {k} {m:.1f} {s:.2f} {n}" for k, (m, s, n) in sorted(self.benches.items())]
        return "\n".join(lines) + "\n"


def parse(text: str, origin: str) -> Trend:
    """Parses the trend format."""
    trend, seen = Trend(), False
    for number, raw in enumerate(text.splitlines(), 1):
        if not raw or raw.startswith("#"):
            continue
        parts = raw.split(" ")
        try:
            if parts[0] == "format" and parts[1:] == [FORMAT]:
                seen = True
            elif parts[0] == "condition" and len(parts) >= CONDITION_FIELDS:
                trend.conditions[parts[1]] = " ".join(parts[2:])
            elif parts[0] == "bench" and len(parts) == BENCH_FIELDS:
                trend.benches[parts[1]] = (float(parts[2]), float(parts[3]), int(parts[4]))
            else:
                raise ValueError(raw)
        except ValueError as e:
            raise TrendError(f"{origin}:{number}: not a {FORMAT} line: {raw!r}") from e
    if not seen:
        raise TrendError(f"{origin}: no `format {FORMAT}` line")
    return trend


def read_trend(path: Path) -> Trend:
    """Reads a trend or summary file."""
    try:
        return parse(path.read_text(encoding="utf-8"), str(path))
    except OSError as e:
        raise TrendError(f"cannot read {path}: {e}") from e


def per_iteration(sample: dict[str, list[float]]) -> list[float]:
    """Nanoseconds per iteration of each sample."""
    iters, times = sample["iters"], sample["times"]
    if len(iters) != len(times) or not iters or min(iters) <= 0:
        raise TrendError("a sample.json with unequal or empty iters and times")
    return [t / i for t, i in zip(times, iters, strict=True)]


def case(sample_path: Path) -> tuple[str, tuple[float, float, int]]:
    """The key and (median, spread %, samples) of one criterion case."""
    try:
        meta = json.loads((sample_path.parent / "benchmark.json").read_text(encoding="utf-8"))
        times = per_iteration(json.loads(sample_path.read_text(encoding="utf-8")))
    except (OSError, ValueError, KeyError) as e:
        raise TrendError(f"{sample_path}: {e}") from e
    if len(times) < MIN_SAMPLES:
        raise TrendError(f"{sample_path}: {len(times)} samples, fewer than {MIN_SAMPLES}")
    median = statistics.median(times)
    spread = (max(times) - min(times)) * 100 / median
    return str(meta["full_id"]), (median, spread, len(times))


def _read(path: str) -> str | None:
    try:
        return Path(path).read_text(encoding="utf-8").strip()
    except OSError:
        return None


def _command(cmd: list[str]) -> str:
    if shutil.which(cmd[0]) is None:
        return "unavailable"
    out = subprocess.run(cmd, capture_output=True, text=True, check=False, cwd=ROOT)
    return out.stdout.strip().splitlines()[0] if out.returncode == 0 and out.stdout.strip() else "?"


def cpu_model() -> str:
    """The CPU model from /proc/cpuinfo (lscpu's model name)."""
    for line in (_read("/proc/cpuinfo") or "").splitlines():
        if line.startswith("model name"):
            return line.split(":", 1)[1].strip()
    return platform.processor() or "?"


def max_temperature() -> str:
    """The hottest thermal zone now, in degrees C, or `unavailable` (a VM has none)."""
    zones = sorted(Path("/sys/class/thermal").glob("thermal_zone*/temp"))
    readings = [int(t) for z in zones if (t := _read(str(z))) and t.lstrip("-").isdigit()]
    return f"{max(readings) / 1000:.1f}C" if readings else "unavailable"


def conditions(machine: str) -> dict[str, str]:
    """The run conditions of RUN-002 §5 and SPR-009 T-04, identity first."""
    product = _read("/sys/class/dmi/id/product_name") or "?"
    return {
        "machine": machine,
        "cpu": cpu_model(),
        "cpus": str(os.cpu_count() or "?"),
        "rustc": _command(["rustc", "-V"]),
        "product": product,
        "kernel": platform.release(),
        "power_profile": _command(["powerprofilesctl", "get"]),
        "temperature": max_temperature(),
    }


def summarize(criterion_dir: Path, machine: str) -> Trend:
    """Every case under criterion_dir with its median and spread, and the run conditions."""
    samples = sorted(criterion_dir.glob("**/new/sample.json"))
    if not samples:
        raise TrendError(f"no criterion samples under {criterion_dir}: run the wall bench first")
    trend = Trend(conditions=conditions(machine))
    for path in samples:
        key, value = case(path)
        trend.benches[key] = value
    return trend


def compare(trend: Trend, measured: Trend) -> tuple[int, list[str]]:
    """The verdict of measured against trend, and one report line per case."""
    changed = [
        f"  {k}: {trend.conditions.get(k)} -> {measured.conditions.get(k)}"
        for k in IDENTITY
        if trend.conditions.get(k) != measured.conditions.get(k)
    ]
    if changed:
        return MACHINE, ["machine changed: new trend baseline required (RUN-002 §5)", *changed]
    verdict, lines = OK, []
    for key in sorted(set(trend.benches) | set(measured.benches)):
        if key not in trend.benches or key not in measured.benches:
            lines.append(f"MISSING {key}: in only one of the two files")
            verdict = ALERT
            continue
        base, now = trend.benches[key][0], measured.benches[key][0]
        pct = (now - base) * 100 / base
        spread = measured.benches[key][1]
        if now * 100 > base * (100 + LIMIT_PERCENT):
            lines.append(f"ALERT {key}: median {base:.0f} -> {now:.0f} ns ({pct:+.2f}%)")
            verdict = ALERT
        else:
            word = "faster (trend kept)" if now < base else "ok"
            lines.append(
                f"{word} {key}: {base:.0f} -> {now:.0f} ns ({pct:+.2f}%, spread {spread:.1f}%)"
            )
    return verdict, lines


def cmd_check(trend_path: Path, measured_path: Path, gate: bool) -> int:
    """Prints the comparison; an alert fails only with --gate."""
    verdict, lines = compare(read_trend(trend_path), read_trend(measured_path))
    print("\n".join(lines))
    if verdict == MACHINE:
        return MACHINE
    if verdict == ALERT:
        print(f"wall-clock ALERT above +{LIMIT_PERCENT}% ({'gating' if gate else 'trend only'})")
        return ALERT if gate else OK
    print(f"wall-clock trend: every median within +{LIMIT_PERCENT}% of {trend_path}")
    return OK


def combine(runs: list[Trend]) -> Trend:
    """One session's runs as one summary: median of medians, run-to-run spread, run count."""
    if len(runs) < MIN_SAMPLES:
        raise TrendError(f"{len(runs)} runs; RUN-002 §5 asks for at least {MIN_SAMPLES}")
    first = runs[0]
    for r in runs[1:]:
        if any(r.conditions.get(k) != first.conditions.get(k) for k in IDENTITY):
            raise TrendError("the runs come from different machines (identity conditions differ)")
        if set(r.benches) != set(first.benches):
            raise TrendError("the runs measured different cases")
    out = Trend(conditions=dict(first.conditions))
    for key in sorted(first.benches):
        medians = [r.benches[key][0] for r in runs]
        mid = statistics.median(medians)
        out.benches[key] = (mid, (max(medians) - min(medians)) * 100 / mid, len(runs))
    return out


def build_parser() -> argparse.ArgumentParser:
    """The command-line interface."""
    p = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    sub = p.add_subparsers(dest="command", required=True)
    s = sub.add_parser("summarize")
    s.add_argument("--criterion-dir", type=Path, default=ROOT / "target" / "criterion")
    s.add_argument("--machine", required=True, help="adjutant, laptop or runner")
    s.add_argument("--out", type=Path, required=True)
    s = sub.add_parser("check")
    s.add_argument("--trend", type=Path, required=True)
    s.add_argument("--measured", type=Path, required=True)
    s.add_argument("--gate", action="store_true")
    s = sub.add_parser("combine")
    s.add_argument("runs", type=Path, nargs="+")
    s.add_argument("--out", type=Path, required=True)
    s = sub.add_parser("record")
    s.add_argument("--measured", type=Path, required=True)
    s.add_argument("--trend", type=Path, required=True)
    return p


def main(argv: Iterable[str] | None = None) -> int:
    """Entry point."""
    a = build_parser().parse_args(None if argv is None else list(argv))
    try:
        if a.command == "summarize":
            trend = summarize(a.criterion_dir, a.machine)
            a.out.parent.mkdir(parents=True, exist_ok=True)
            a.out.write_text(trend.render(), encoding="utf-8")
            print(f"wrote {a.out}: {len(trend.benches)} cases on {a.machine}")
            return OK
        if a.command == "check":
            return cmd_check(a.trend, a.measured, a.gate)
        if a.command == "combine":
            session = combine([read_trend(r) for r in a.runs])
            a.out.parent.mkdir(parents=True, exist_ok=True)
            a.out.write_text(session.render(), encoding="utf-8")
            worst = max(spread for _, spread, _ in session.benches.values())
            print(f"wrote {a.out}: {len(a.runs)} runs; worst run-to-run spread {worst:.2f}%")
            return OK
        trend = read_trend(a.measured)
        a.trend.parent.mkdir(parents=True, exist_ok=True)
        a.trend.write_text(trend.render(), encoding="utf-8")
        print(f"wrote {a.trend}: {len(trend.benches)} cases")
        return OK
    except TrendError as e:
        print(f"wall_trend: {e}", file=sys.stderr)
        return USAGE


if __name__ == "__main__":
    sys.exit(main())
