"""
Golden-vector self-test for the Brass relational logit (M2 mortality; ADR-0027 era hooks).

Checks, against Brass (1971) as transcribed in brass_general_standard.toml:
  1. logit(l(x)) of the General Standard matches the published Ys to within 0.0006;
  2. Table 19 schedules (beta = 1) are reproduced to within 1 per 10,000;
  3. e0 by trapezoid on the standard grid is within 0.06 years of Table 19;
  4. the child-mortality band V-SF-02 (q15 in [0.35, 0.55]) maps to alpha and e0 ranges at beta = 1.
Run: python -I specs/demography/selftest_brass.py
"""
from __future__ import annotations

import math
import sys
import tomllib
from pathlib import Path

T = tomllib.loads(Path(__file__).with_name("brass_general_standard.toml").read_text(encoding="utf-8"))
AGES, L = T["standard"]["ages"], T["standard"]["l"]
YS = [0.5 * math.log((1 - p) / p) if 0 < p < 1 else None for p in L]


def schedule(alpha: float, beta: float) -> list[float]:
    return [p if p in (0.0, 1.0) else 1 / (1 + math.exp(2 * (alpha + beta * y))) for p, y in zip(L, YS)]


def at(lx: list[float], x: float) -> float:
    return lx[AGES.index(x)]


def e0(lx: list[float]) -> float:
    return sum((AGES[i + 1] - AGES[i]) * (lx[i] + lx[i + 1]) / 2 for i in range(len(AGES) - 1))


def solve(f, target: float, lo: float, hi: float) -> float:
    for _ in range(200):
        mid = (lo + hi) / 2
        lo, hi = (lo, mid) if (f(mid) - target) * (f(lo) - target) <= 0 else (mid, hi)
    return (lo + hi) / 2


def main() -> int:
    ok = True
    worst = max(abs(y - p) for y, p in zip(YS, T["standard"]["ys_published"]) if y is not None and not math.isnan(p))
    ok &= worst <= 0.0006
    print(f"1. standard logits vs published: max |diff| = {worst:.5f} {'PASS' if worst <= 0.0006 else 'FAIL'}")

    ages19 = T["golden_table19"]["ages"]
    worst = 0
    for key, row in T["golden_table19"].items():
        if key == "ages":
            continue
        lx = schedule(row["alpha"], 1.0)
        worst = max(worst, *(abs(round(at(lx, x) * 10000) - v) for x, v in zip(ages19, row["l_per_10k"])))
    ok &= worst <= 1
    print(f"2. Table 19 schedules: max |diff| = {worst} per 10,000 {'PASS' if worst <= 1 else 'FAIL'}")

    worst = max(abs(e0(schedule(a, 1.0)) - v) for a, v in T["golden_e0"]["values"])
    ok &= worst <= 0.06
    print(f"3. e0 trapezoid: max |diff| = {worst:.3f} years {'PASS' if worst <= 0.06 else 'FAIL'}")

    a_lo = solve(lambda a: 1 - at(schedule(a, 1.0), 15), 0.35, -4, 3)
    a_hi = solve(lambda a: 1 - at(schedule(a, 1.0), 15), 0.55, -4, 3)
    print(f"4. V-SF-02 q15 in [0.35, 0.55] at beta = 1  <=>  alpha in [{a_lo:.2f}, {a_hi:.2f}],"
          f" e0 in [{e0(schedule(a_hi, 1.0)):.1f}, {e0(schedule(a_lo, 1.0)):.1f}]")
    print("BRASS GOLDEN VECTORS:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
