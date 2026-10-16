"""Exact reference and self-test of M2's rung-0 mortality schedule (SPR-011 ruling 7).

The algorithm is stated in ``specs/systems/M2.demography.odd.toml``, section
[algorithm.mortality_rung0], steps S0 to S11. This file implements those steps with Python integers
and ``fractions`` only (no float in the schedule), so it reproduces the engine's integers bit for
bit. T-05's ``bin/gen_m2_reference.py`` ports it to generate the golden vectors. It checks:

  1. the spec's check values (K for seven e0 values) equal this reference's, and every e0 lies
     between T(K) and T(K + 1), the bisection's defining property;
  2. the widths the spec states hold at the extremes (numerators, denominators, sums, products);
  3. the four-digit standard's own trapezoid e0 is 43.45915 years, and solving from it gives K
     within 2^10 units of 1 (43.44 is Brass's rounding: K = 1.00114 there);
  4. Brass (1971) Table 19 (beta = 1) is reproduced within 1 per 10,000 at the e0 each alpha
     implies;
  5. V-SF-02's q(15) in [0.35, 0.55] holds exactly for e0 in the Q16_16 interval [E_LO, E_HI];
  6. monthly probabilities lie in [0, 2^32], are 2^32 exactly when the month reaches age 100, and
     the minute of death lies inside the month, never at the pulse instant.

Run: python -I specs/demography/selftest_m2_rung0.py
"""

from __future__ import annotations

import itertools
import math
import sys
import tomllib
from decimal import Decimal
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
STANDARD = HERE / "brass_general_standard.toml"
SPEC = HERE.parent / "systems" / "M2.demography.odd.toml"

ONE = 1 << 32  # l and q carry 32 fraction bits; Prob32's 1
K_HI = 1 << 40  # the bisection's upper end: K = 256
STEPS = 40  # bisection iterations: 2^40 halves to 1
SCALE = 10_000  # four-digit standard survivors
Q16 = 1 << 16
MINUTES_360 = 360 * 1440
MONTH_360 = 30 * 1440
GRID_POINTS = 71
STANDARD_E0 = Fraction(4345915, 100000)  # the trapezoid e0 of the four-digit standard, in years
E0_GRID_TOL = Fraction(1, 1 << 30)  # S3 rounds each L_i, so e0(K = 1) is within 2^-30 years of it
K_NEAR_ONE = 1 << 10  # |K - 2^32| at the standard's e0, after Q16_16 rounding of e0
TABLE19_TOL = 1  # per 10,000, as selftest_brass.py
E_LO = 1536235  # smallest Q16_16 e0 with q(15) <= 0.55 (23.441086 years)
E_HI = 2396642  # largest Q16_16 e0 with q(15) >= 0.35 (36.569855 years)
Q15_HI = Fraction(55, 100)
Q15_LO = Fraction(35, 100)


def rhe(n: int, d: int) -> int:
    """Return n / d rounded to the nearest integer, ties to even (S0); d > 0."""
    q, r = divmod(n, d)  # floor division: 0 <= r < d
    if 2 * r > d or (2 * r == d and q % 2 == 1):
        q += 1
    return q


def standard() -> tuple[list[int], list[int]]:
    """Return S1's constants: h_i = 2 a_i and s_i = 10^4 ls_i, both exact."""
    t = tomllib.loads(STANDARD.read_text(encoding="utf-8"), parse_float=Decimal)["standard"]
    h = [int(Decimal(str(a)) * 2) for a in t["ages"]]
    s = [int(v * SCALE) for v in t["l"]]
    assert all(Decimal(2 * x) == Decimal(str(a)) * 4 for x, a in zip(h, t["ages"], strict=True))
    assert all(Decimal(v) / SCALE == x for v, x in zip(s, t["l"], strict=True)), "not four-digit"
    assert len(h) == GRID_POINTS and s[0] == SCALE and s[-1] == 0
    assert all(a >= b for a, b in itertools.pairwise(s)), "standard not non-increasing"
    return h, s


H, S = standard()
AGE_15 = H.index(30)  # the grid index of age 15 (h = 2 a)


def survivors(k: int) -> list[int]:
    """Return S3's L_i(k) for every grid age."""
    return [0 if s == 0 else rhe(s << 64, (s << 32) + k * (SCALE - s)) for s in S]


def trapezoid(lx: list[int]) -> int:
    """Return S4's T: e0 in units of 2^-34 years."""
    return sum((H[i + 1] - H[i]) * (lx[i] + lx[i + 1]) for i in range(GRID_POINTS - 1))


def solve_k(e_bits: int) -> int:
    """Return S5's K (bits, 32 fraction bits) for the Q16_16 e0 ``e_bits``."""
    target = e_bits << 18
    lo, hi = 0, K_HI
    for _ in range(STEPS):
        mid = (lo + hi) >> 1
        if trapezoid(survivors(mid)) >= target:
            lo = mid
        else:
            hi = mid
    assert hi == lo + 1
    return lo


def q16(literal: str) -> int:
    """Return the Q16_16 bits of a decimal literal, rounded half to even (ruling 9)."""
    return round(Fraction(Decimal(literal)) * Q16)


def ell(lx: list[int], x: int, year: int) -> int:
    """Return S7's l(x) for an age of ``x`` minutes, with ``year`` minutes per year."""
    assert x >= 0, "a negative age is an error"
    ages = [h * year // 2 for h in H]
    if x >= ages[-1]:
        return 0
    i = max(j for j in range(GRID_POINTS) if ages[j] <= x)
    return lx[i] + rhe((lx[i + 1] - lx[i]) * (x - ages[i]), ages[i + 1] - ages[i])


def month_q(lx: list[int], x: int, month: int, year: int) -> int:
    """Return S8's Prob32 bits of dying within ``month`` minutes from age ``x``."""
    now = ell(lx, x, year)
    return ONE if now == 0 else rhe(ONE * (now - ell(lx, x + month, year)), now)


def hits(draw: int, q: int) -> bool:
    """Return the Prob32 mapping: hits iff q = 2^32 or draw < q 2^32 (05.1 §6 item 4)."""
    return q == ONE or draw < q << 32


def death_minute(t_m: int, month: int, draw: int) -> int:
    """Return S9's death instant: t_m + 1 + floor(draw (D - 1) / 2^64)."""
    return t_m + 1 + ((draw * (month - 1)) >> 64)


def check_values() -> bool:
    """Check 1: the spec's K values, and T(K) >= e0 > T(K + 1)."""
    spec = tomllib.loads(SPEC.read_text(encoding="utf-8"))
    rows = spec["algorithm"]["mortality_rung0"]["check_values"]
    ok = True
    for row in rows:
        e = q16(row["e0"])
        k = solve_k(e)
        bracket = trapezoid(survivors(k)) >= e << 18 > trapezoid(survivors(k + 1))
        ok &= k == row["K"] and bracket
        print(f"   e0 {row['e0']:>6}: K = {k} (spec {row['K']}) = {k / ONE:.6f}, bracket {bracket}")
    return ok


def check_widths() -> bool:
    """Check 2: every intermediate fits the width the spec states."""
    worst_num = max(s << 64 for s in S).bit_length()
    worst_den = max((s << 32) + K_HI * (SCALE - s) for s in S).bit_length()
    worst_sum = trapezoid(survivors(1)).bit_length()
    year = 365 * 1440
    worst_step = max(h2 - h1 for h1, h2 in itertools.pairwise(H)) * year // 2
    worst_prod = (ONE * worst_step).bit_length()
    print(
        f"   bit lengths: S3 numerator {worst_num} (<= 78), denominator {worst_den} (<= 54), "
        f"S4 sum {worst_sum} (<= 42), S7 product {worst_prod} (<= 53)"
    )
    return worst_num <= 78 and worst_den <= 54 and worst_sum <= 42 and worst_prod <= 53  # noqa: PLR2004


def check_standard() -> bool:
    """Check 3: the standard's own e0, and K = 1 near it."""
    exact = Fraction(
        sum((H[i + 1] - H[i]) * (S[i] + S[i + 1]) for i in range(GRID_POINTS - 1)), 4 * SCALE
    )
    e0 = Fraction(trapezoid(survivors(ONE)), 1 << 34)
    k = solve_k(round(exact * Q16))
    print(
        f"   standard e0 = {exact} years exactly; e0(K = 1) - that = {float(e0 - exact):.2e}; "
        f"K at it = 2^32 {k - ONE:+d}; "
        f"K at 43.44 = {solve_k(q16('43.44')) / ONE:.5f}"
    )
    return exact == STANDARD_E0 and abs(e0 - exact) < E0_GRID_TOL and abs(k - ONE) < K_NEAR_ONE


def check_table19() -> bool:
    """Check 4: Brass Table 19 within 1 per 10,000 at each alpha's e0 (tool-side floats)."""
    t = tomllib.loads(STANDARD.read_text(encoding="utf-8").replace("nan", "-1"))
    ages, golden = t["golden_table19"]["ages"], t["golden_table19"]
    worst = 0
    for key, row in golden.items():
        if key == "ages":
            continue
        odds = math.exp(2 * row["alpha"])
        lx = [1.0 if s == SCALE else 0.0 if s == 0 else s / (s + odds * (SCALE - s)) for s in S]
        e0 = sum((H[i + 1] - H[i]) * (lx[i] + lx[i + 1]) / 4 for i in range(GRID_POINTS - 1))
        fixed = survivors(solve_k(round(Fraction(e0) * Q16)))
        for age, want in zip(ages, row["l_per_10k"], strict=True):
            got = rhe(fixed[H.index(2 * age)] * SCALE, ONE)
            worst = max(worst, abs(got - want))
    print(f"   Table 19: max |diff| = {worst} per 10,000")
    return worst <= TABLE19_TOL


def q15(e_bits: int) -> Fraction:
    """Return q(15) = 1 - l(15) of the schedule for the Q16_16 e0 ``e_bits``."""
    return 1 - Fraction(survivors(solve_k(e_bits))[AGE_15], ONE)


def check_band() -> bool:
    """Check 5: q(15) in [0.35, 0.55] exactly for e0 bits in [E_LO, E_HI]."""
    ok = q15(E_LO) <= Q15_HI < q15(E_LO - 1) and q15(E_HI + 1) < Q15_LO <= q15(E_HI)
    print(
        f"   V-SF-02 band: e0 in [{E_LO / Q16:.6f}, {E_HI / Q16:.6f}]; "
        f"at 23.4 q15 = {float(q15(q16('23.4'))):.4f}, "
        f"at 36.6 q15 = {float(q15(q16('36.6'))):.4f}"
    )
    return ok


def check_months() -> bool:
    """Check 6: Prob32 range, certain death at 100, minutes inside the month."""
    lx = survivors(solve_k(q16("30")))
    top = 100 * MINUTES_360
    ok = True
    for x in range(0, top + MINUTES_360, MONTH_360 * 7 + 13):
        q = month_q(lx, x, MONTH_360, MINUTES_360)
        ok &= 0 <= q <= ONE and (q == ONE) == (x + MONTH_360 >= top)
    for d in (0, 1, (1 << 63), (1 << 64) - 1):
        tau = death_minute(0, MONTH_360, d)
        ok &= 1 <= tau <= MONTH_360 - 1
    ok &= hits((1 << 64) - 1, ONE) and not hits(0, 0) and hits(0, 1) and not hits(1 << 32, 1)
    q10 = month_q(survivors(solve_k(q16("70"))), 10 * MINUTES_360, MONTH_360, MINUTES_360)
    print(
        f"   months: q in [0, 2^32], certain iff the month reaches 100; e0 70, age 10: "
        f"q = {q10} / 2^32 = {q10 / ONE:.3e}"
    )
    return ok


def main() -> int:
    """Run every check and print PASS or FAIL."""
    checks = [
        ("1. spec check values", check_values),
        ("2. widths", check_widths),
        ("3. standard e0", check_standard),
        ("4. Table 19", check_table19),
        ("5. V-SF-02 band", check_band),
        ("6. monthly draws", check_months),
    ]
    ok = True
    for name, fn in checks:
        good = fn()
        ok &= good
        print(f"{name}: {'PASS' if good else 'FAIL'}")
    print("M2 RUNG-0 SCHEDULE:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
