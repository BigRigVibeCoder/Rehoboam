"""Generate the golden vectors of M2's rung-0 schedule from its exact reference (SPR-012 T-05).

The algorithm is ``[algorithm.mortality_rung0]`` of ``specs/systems/M2.demography.odd.toml``. Its
exact reference is ``specs/demography/selftest_m2_rung0.py`` (Python integers and ``fractions``,
no float in the schedule); this tool loads that file's functions, so there is one reference, and
writes every value the engine must reproduce bit for bit:

* ``standard i h s``: S1's compiled-in constants, from ``brass_general_standard.toml``;
* ``schedule E K T`` and ``grid E L_0 ... L_70``: S5's K, S4's T(K) and S6's schedule for each
  ``Q16_16`` e0 input E: the seven of SPR-012 T-05, the spec's check values, V-SF-02's band edges
  and their neighbours, the e0 each Brass Table 19 row implies, and pseudo-random ones;
* ``table19.ages ...`` and ``table19 key alpha E golden_e0_hundredths l_per_10k...``: the Table 19
  comparison. E is the trapezoid e0 of the row's beta = 1 schedule on M2's grid, rounded to
  ``Q16_16`` half to even, computed exactly as the reference's check 4 does (Architect ruling,
  2026-10-17); the published two-decimal e0 only bounds the recomputed one (0.06 years);
* ``ell E Y x l``: S7's l(x) on both calendars (Y minutes per year);
* ``q E Y x0 x1 q``: the probability of dying between ages x0 and x1 (S8 with x1 = x + D; S11);
* ``day t_m D d1 tau`` and ``resample t_in t_end d1 tau|none``: S9's and S11's day draws.

Inputs beyond the edge cases come from a SplitMix64 with a fixed seed (defined by its arithmetic,
not by a Python version). Read by ``crates/rh-sys-demography/tests/schedule_golden.rs``.

Usage:  python -I bin/gen_m2_reference.py           (write the file)
        python -I bin/gen_m2_reference.py --check   (exit 1 if the committed file is stale)
        --output <file> checks or writes another file (the positive control:
        tests/lint_fixtures/golden/altered_m2_schedule_v1.txt must be STALE)
Standard library only (GOV-003 §3). Refs: SPR-012 T-05; SPR-011 ruling 7; R12 §9.
"""

from __future__ import annotations

import argparse
import importlib.util
import math
import sys
import tomllib
from collections.abc import Iterator, Sequence
from fractions import Fraction
from pathlib import Path
from types import ModuleType

REPO = Path(__file__).resolve().parents[1]
REFERENCE = REPO / "specs" / "demography" / "selftest_m2_rung0.py"
OUTPUT = REPO / "crates" / "rh-sys-demography" / "tests" / "golden" / "m2_schedule_v1.txt"
SEED = 0x5EED_0002_0005
MASK64 = (1 << 64) - 1
DAY = 1440
YEARS = (360 * DAY, 365 * DAY)  # minutes per year of the two calendars
MONTHS = (30 * DAY, 28 * DAY, 29 * DAY, 31 * DAY)  # month lengths of the two calendars
T05_E0 = ("20", "23.441086", "30", "36.569855", "43.45915", "60", "85")
RANDOM_E0 = 8
RANDOM_AGES = 24
RANDOM_DRAWS = 16


def load_reference() -> ModuleType:
    """Return the exact reference module (one copy of the algorithm)."""
    spec = importlib.util.spec_from_file_location("selftest_m2_rung0", REFERENCE)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {REFERENCE}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class SplitMix64:
    """SplitMix64 (Steele, Lea and Flood 2014): a fixed, version-independent input stream."""

    def __init__(self, seed: int) -> None:
        """Start the stream at ``seed``."""
        self.state = seed & MASK64

    def next_u64(self) -> int:
        """Return the next 64-bit output."""
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK64
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK64
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK64
        return z ^ (z >> 31)

    def below(self, n: int) -> int:
        """Return an integer in [0, n) (multiply-shift; n < 2^63)."""
        return (self.next_u64() * n) >> 64


def table19_rows(ref: ModuleType) -> list[tuple[str, str, int, int, list[int]]]:
    """Return each Table 19 row: key, alpha, implied Q16_16 e0, golden e0 (hundredths), l values.

    The implied e0 is computed as the reference's check 4 does: the trapezoid e0 of the row's
    beta = 1 schedule on M2's grid (tool-side floats), rounded to Q16_16 half to even.
    """
    t = tomllib.loads(ref.STANDARD.read_text(encoding="utf-8").replace("nan", "-1"))
    golden_e0 = {Fraction(str(a)): Fraction(str(e)) for a, e in t["golden_e0"]["values"]}
    rows = []
    for key, row in t["golden_table19"].items():
        if key == "ages":
            continue
        odds = math.exp(2 * row["alpha"])
        lx = [
            1.0 if s == ref.SCALE else 0.0 if s == 0 else s / (s + odds * (ref.SCALE - s))
            for s in ref.S
        ]
        e0 = sum(
            (ref.H[i + 1] - ref.H[i]) * (lx[i] + lx[i + 1]) / 4 for i in range(ref.GRID_POINTS - 1)
        )
        bits = round(Fraction(e0) * ref.Q16)
        published = golden_e0[Fraction(str(row["alpha"]))] * 100
        assert published.denominator == 1
        rows.append((key, str(row["alpha"]), bits, int(published), row["l_per_10k"]))
    return rows


def e0_inputs(ref: ModuleType, rng: SplitMix64, table19: Sequence[int]) -> list[int]:
    """Return the Q16_16 e0 inputs, ascending and distinct."""
    spec = tomllib.loads(ref.SPEC.read_text(encoding="utf-8"))
    checks = [row["e0"] for row in spec["algorithm"]["mortality_rung0"]["check_values"]]
    bits = {ref.q16(e) for e in (*T05_E0, *checks)}
    bits |= {ref.E_LO - 1, ref.E_LO, ref.E_HI, ref.E_HI + 1, 20 << 16, 85 << 16}
    bits |= set(table19)
    lo, hi = 20 << 16, 85 << 16
    bits |= {lo + rng.below(hi - lo + 1) for _ in range(RANDOM_E0)}
    return sorted(bits)


def ages(rng: SplitMix64, year: int) -> list[int]:
    """Return ages in minutes: every grid age and its neighbours, the top, and random ones."""
    grid = [h * year // 2 for h in (0, 1, 2, 30, 99, 100, 101, 105, 199, 200)]
    edge = {a + d for a in grid for d in (-1, 0, 1) if a + d >= 0}
    edge |= {rng.below(100 * year + year) for _ in range(RANDOM_AGES)}
    return sorted(edge)


HEADER = [
    "# Golden vectors for M2's rung-0 schedule. GENERATED by bin/gen_m2_reference.py; never edit.",
    "# Exact reference: specs/demography/selftest_m2_rung0.py ([algorithm.mortality_rung0]).",
    "# Integers only; E = Q16_16 e0 bits; L, l and q carry 32 fraction bits; T is 2^-34 years.",
]


def schedule_lines(
    ref: ModuleType, rng: SplitMix64, t19: list[tuple[str, str, int, int, list[int]]]
) -> tuple[list[str], dict[int, list[int]]]:
    """Return the standard, schedule, grid and Table 19 lines, and each input's grid."""
    lines = [f"standard {i} {h} {s}" for i, (h, s) in enumerate(zip(ref.H, ref.S, strict=True))]
    schedules = {}
    for e in e0_inputs(ref, rng, [row[2] for row in t19]):
        k = ref.solve_k(e)
        lx = ref.survivors(k)
        schedules[e] = lx
        lines.append(f"schedule {e} {k} {ref.trapezoid(lx)}")
        lines.append(f"grid {e} " + " ".join(map(str, lx)))
    std = tomllib.loads(ref.STANDARD.read_text(encoding="utf-8").replace("nan", "-1"))
    lines.append("table19.ages " + " ".join(map(str, std["golden_table19"]["ages"])))
    for key, alpha, e, published, want in t19:
        lines.append(f"table19 {key} {alpha} {e} {published} " + " ".join(map(str, want)))
    return lines, schedules


def age_lines(ref: ModuleType, rng: SplitMix64, e: int, lx: list[int], year: int) -> list[str]:
    """Return the ell and q lines of one schedule on one calendar."""
    lines = []
    for x in ages(rng, year):
        lines.append(f"ell {e} {year} {x} {ref.ell(lx, x, year)}")
        lines += [
            f"q {e} {year} {x} {x + d} {ref.month_q(lx, x, d, year)}"
            for d in MONTHS
            if x + d <= 101 * year
        ]
    for _ in range(RANDOM_AGES):  # S11 intervals: an input inside the month
        x0 = rng.below(100 * year)
        x1 = x0 + 1 + rng.below(31 * DAY)
        now = ref.ell(lx, x0, year)
        q = ref.ONE if now == 0 else ref.rhe(ref.ONE * (now - ref.ell(lx, x1, year)), now)
        lines.append(f"q {e} {year} {x0} {x1} {q}")
    return lines


def day_lines(ref: ModuleType, rng: SplitMix64) -> list[str]:
    """Return the day and resample lines."""
    lines = []
    for d1 in draws(rng):
        for t_m in (0, 30 * DAY, 365 * DAY * 7 + 59 * DAY, -(30 * DAY)):
            lines += [f"day {t_m} {d} {d1} {ref.death_day(t_m, d, d1)}" for d in MONTHS]
        for t_in, t_end in resample_cases(rng):
            r = ref.resample_day(t_in, t_end, d1)
            lines.append(f"resample {t_in} {t_end} {d1} {'none' if r is None else r}")
    return lines


def render() -> str:
    """Return the vector file's text."""
    ref = load_reference()
    rng = SplitMix64(SEED)
    lines, schedules = schedule_lines(ref, rng, table19_rows(ref))
    for e in (ref.q16("30"), ref.q16("60"), ref.E_LO, 85 << 16):
        for year in YEARS:
            lines += age_lines(ref, rng, e, schedules[e], year)
    lines += day_lines(ref, rng)
    return "\n".join(HEADER + lines) + "\n"


def draws(rng: SplitMix64) -> Iterator[int]:
    """Yield the u64 draws of the day vectors: the extremes, halves, and random ones."""
    yield from (0, 1, (1 << 63) - 1, 1 << 63, MASK64)
    for _ in range(RANDOM_DRAWS):
        yield rng.next_u64()


def resample_cases(rng: SplitMix64) -> list[tuple[int, int]]:
    """Return (t_in, t_end) pairs: day starts, the month's last day, its last minute, random."""
    month = 30 * DAY
    cases = [(0, month), (5, month), (DAY, month), (month - DAY, month), (month - 1, month)]
    cases += [(month - DAY - 1, month), (13 * DAY + 7, month), (-DAY - 3, 0)]
    cases += [(t, 28 * DAY) for t in (0, 27 * DAY - 1, 27 * DAY)]
    cases += [(rng.below(month), month) for _ in range(4)]
    return cases


def vector_count(text: str) -> int:
    """Return the number of vector lines."""
    return sum(1 for line in text.splitlines() if line and not line.startswith("#"))


def main(argv: Sequence[str] | None = None) -> int:
    """Write the vector file, or with ``--check`` verify that it is current."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    parser.add_argument("--output", type=Path, default=OUTPUT, help="the vector file")
    args = parser.parse_args(argv)
    output: Path = args.output
    text = render()
    if args.check:
        current = output.read_text(encoding="utf-8") if output.exists() else ""
        if current != text:
            print(f"STALE: {output} differs; run python -I bin/gen_m2_reference.py")
            return 1
        print(f"gen_m2_reference: current ({vector_count(text)} vectors)")
        return 0
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(text, encoding="utf-8")
    print(f"gen_m2_reference: wrote {output} ({vector_count(text)} vectors)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
