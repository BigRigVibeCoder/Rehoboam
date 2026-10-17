"""Exact reference and self-test of M2 F1 mortality (SPR-016; ``mortality_f1.toml`` beside it).

The algorithm is ``[algorithm.mortality_f1]``, steps F0 to F13: Brass at any (alpha, beta)
through the fixed-point ``exp`` and ``ln`` of `05.1` §6 item 4 (v1.10), the infant sub-schedule
below age 1, the old-age tail above 90 and the density modifier. This file implements those steps
with Python integers only, so it reproduces the engine's integers bit for bit; ``decimal`` is only
the oracle that generates the exp/ln constants and the infant table, as in
``specs/numerics/exp_ln_v0.py``, whose procedure is repeated here verbatim so that this file runs
alone under ``python -I``. It checks:

  1. **rung 0 is the beta = 1 special case:** at beta = 1 the odds grid is the exact rational
     one, so K equals the rung-0 K of ``M2.demography.odd.toml``'s check values for every e0
     there, the 71 grid survivors equal rung 0's S3 bit for bit, and l(x) equals rung 0's at every
     minute of age from 1 to 90 years; F1 differs from rung 0 only below age 1 (F7) and above 90
     (F8), and the modifier is exactly 1 at or under the capacity (F11);
  2. the widths the spec states hold at the extremes (k w_i, the F4 numerator, knot products, A_j);
  3. at the Registry's corners (e0 in {20, 85}, beta in {0.6, 1.65}) and the design points: the
     bisection brackets e0, T(k) is non-increasing, the spec's check values (K, a, b) hold, the
     knots are strictly increasing in age and
     non-increasing in survivorship, and the tail's preconditions hold (m2 > m1, mu < 1,
     30 b <= 16);
  4. the tail: continuous at 90, zero at 120, its first month's hazard within 1% of its anchor
     mu(90) (Brass's own interval hazards around 90 are printed beside it), survival from 90 to
     100 reported;
  5. the infant table: recomputed from its formula, normalised, non-decreasing; continuity at age 1;
     the first-month share reported;
  6. the density modifier: exactly 2^16 for N <= K, the slope, the ceiling of 4, q' = q when m = 1,
     q' a Prob32;
  7. the draws (F12 = rung 0's S9-S11): day starts inside the month, re-samples after the input;
  8. the realized e0 of the F1 schedule (trapezoid over its knots) is within 0.1 years of the input
     for e0 <= 60 and within 1 year up to 85 (the tail adds life past Brass's l(100) = 0 where
     l(90) is large), and V-SF-02b's l(65) / l(15) is reported at beta in {0.64, 1, 1.64};
  9. the golden file ``specs/demography/golden/m2_f1_mortality.txt`` is current (``--check``).

Controls that must FAIL (``--control NAME``; bin/check_all.sh runs each): ``beta-one-power``
evaluates the power form at beta = 1 (check 1 fails: the rung-0 schedule is no longer reproduced);
``shares-unnormalised`` ends the infant table at 65535 (check 5 fails: the sub-schedule no longer
meets l(1)); ``--check --golden PATH`` on a stale fixture fails the golden check.

Run: python -I specs/demography/selftest_m2_f1_mortality.py [--check] [--golden PATH]
     [--control NAME]
Exit: 0 ok, 1 stale golden file, 2 a check failed. Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import itertools
import sys
import tomllib
from collections.abc import Callable, Sequence
from dataclasses import dataclass, field
from decimal import ROUND_HALF_EVEN, Decimal, getcontext
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
STANDARD = HERE / "brass_general_standard.toml"
INFANT = HERE / "infant_sub_schedule.toml"
SPEC_F1 = HERE / "mortality_f1.toml"
SPEC_M2 = HERE.parent / "systems" / "M2.demography.odd.toml"
GOLDEN = HERE / "golden" / "m2_f1_mortality.txt"
EXIT_OK, EXIT_STALE, EXIT_FAILED = 0, 1, 2

ONE = 1 << 32  # l, q and Q32_32 carry 32 fraction bits; Prob32's 1
Q16 = 1 << 16
K_HI = 1 << 40
STEPS = 40
SCALE = 10_000
GRID_POINTS = 71
DAY = 1440
YEAR_360, YEAR_365 = 360 * DAY, 365 * DAY
MONTH_360 = 30 * DAY
MONTHS = 12
AGE_1, AGE_90, AGE_120 = 1, 90, 120
TAIL_MONTHS = (AGE_120 - AGE_90) * MONTHS  # 360 monthly knots above 90
GRID_87, GRID_90, GRID_92 = 65, 66, 67  # grid indices of ages 87.5, 90, 92.5
HALF_YEARS_PER_INTERVAL = 5  # 2.5 years between those grid ages, in half-years
MODIFIER_CAP = 4 * Q16
INFANT_KNOTS = 13
E0_TOL = Fraction(1, 10)  # the realized e0 is within 0.1 years of the input for e0 <= 60 ...
E0_TOL_HIGH = Fraction(1)  # ... and within 1 year up to 85, where the tail adds life past 100
E0_TOL_SPLIT = 60
TAIL_HAZARD_TOL = Fraction(1, 100)  # the tail's first month against its own mu(90)
CORNERS = [("20", "0.6"), ("20", "1.65"), ("85", "0.6"), ("85", "1.65")]
DESIGN_POINTS = [
    ("30", "1"),
    ("30", "0.8"),
    ("30", "1.3"),
    ("23.441086", "1"),
    ("60", "1.65"),
    ("45", "1"),
]
SF02B_BETAS = ("0.64", "1", "1.64")
AGE_15, AGE_65 = 15, 65

# ---------------------------------------------------------------------- exp and ln (05.1 §6 item 4)
EXP_FRAC = 32
WORK = 64
WORK_ONE = 1 << WORK
EXP_STEPS = 32
EXP_MAX = 16 * ONE
EXP_ZERO_BELOW = -23 * ONE
LN_OPERAND_BITS = 63
INV_LN2_SHIFT = 62
getcontext().prec = 60


def rhe(n: int, d: int) -> int:
    """Return n / d rounded to the nearest integer, ties to even (F0 = S0); d > 0, n signed."""
    q, r = divmod(n, d)
    if 2 * r > d or (2 * r == d and q % 2 == 1):
        q += 1
    return q


def _round_half_even(value: Decimal) -> int:
    return int(value.to_integral_value(rounding=ROUND_HALF_EVEN))


def _constants() -> tuple[int, int, list[int]]:
    ln2 = Decimal(2).ln()
    table = [
        _round_half_even((Decimal((1 << k) + 1) / Decimal(1 << k)).ln() * WORK_ONE)
        for k in range(1, EXP_STEPS + 1)
    ]
    return (
        _round_half_even(ln2 * WORK_ONE),
        _round_half_even(Decimal(1 << INV_LN2_SHIFT) / ln2),
        table,
    )


LN2_64, INV_LN2_62, C = _constants()


def exp_q32(y: int) -> int:
    """Return e^y as Q32_32 bits (05.1 §6 item 4, E1-E6); refuses y > 16, 0 below -23."""
    if y > EXP_MAX:
        msg = f"ExpDomain: {y}"
        raise ValueError(msg)
    if y < EXP_ZERO_BELOW:
        return 0
    n = (y * INV_LN2_62) >> (INV_LN2_SHIFT + EXP_FRAC)
    r = (y << EXP_FRAC) - n * LN2_64
    p = WORK_ONE
    for k in range(1, EXP_STEPS + 1):
        if r >= C[k - 1]:
            r -= C[k - 1]
            p += p >> k
    p += (p * r) >> WORK
    return rhe(p, 1 << (EXP_FRAC - n))


def ln_ratio(n: int, d: int) -> int:
    """Return ln(n / d) as Q32_32 bits for 0 < n, d < 2^63 (05.1 §6 item 4, L1-L6)."""
    if not (0 < n < (1 << LN_OPERAND_BITS) and 0 < d < (1 << LN_OPERAND_BITS)):
        msg = f"LnDomain: {n}, {d}"
        raise ValueError(msg)
    e = n.bit_length() - d.bit_length()
    if (n >> e if e >= 0 else n << -e) < d:
        e -= 1
    m = (n << WORK) // (d << e) if e >= 0 else (n << (WORK - e)) // d
    p, s = WORK_ONE, 0
    for k in range(1, EXP_STEPS + 1):
        t = p + (p >> k)
        if t <= m:
            p, s = t, s + C[k - 1]
    residual = ((m - p) << WORK) // p
    return rhe(e * LN2_64 + s + residual, 1 << EXP_FRAC)


# ----------------------------------------------------------------------------- F1: the standard
def standard() -> tuple[list[int], list[int]]:
    """Return S1's constants: h_i = 2 a_i and s_i = 10^4 ls_i, both exact."""
    t = tomllib.loads(STANDARD.read_text(encoding="utf-8"), parse_float=Decimal)["standard"]
    h = [int(Decimal(str(a)) * 2) for a in t["ages"]]
    s = [int(v * SCALE) for v in t["l"]]
    assert len(h) == GRID_POINTS and s[0] == SCALE and s[-1] == 0
    assert all(a >= b for a, b in itertools.pairwise(s))
    assert h[GRID_90] == 2 * AGE_90 and h[GRID_87] == 175 and h[GRID_92] == 185  # noqa: PLR2004
    return h, s


H, S = standard()
LAMBDA = [0, *(ln_ratio(SCALE - s, s) for s in S[1:-1]), 0]  # F1: ln O_i at the interior grid ages


def infant_shares() -> list[int]:
    """Return F7's cumulative shares c_j (per 2^16) from the data file."""
    t = tomllib.loads(INFANT.read_text(encoding="utf-8"))["infant_sub_schedule"]
    shares: list[int] = list(t["cumulative_share_q16"])
    return shares


def q16(literal: str) -> int:
    """Return the Q16_16 bits of a decimal literal, rounded half to even (SPR-011 ruling 9)."""
    return round(Fraction(Decimal(literal)) * Q16)


# ------------------------------------------------------------------------ F3-F6: the Brass schedule
@dataclass
class Options:
    """The controls that must fail, and the knobs of the reference."""

    power_at_beta_one: bool = False  # control beta-one-power
    shares: list[int] = field(default_factory=infant_shares)


def odds_grid(beta_bits: int, opt: Options) -> list[int | None]:
    """Return F3's w_i (Q32_32 bits) for i = 1..69, or None when the rational form applies."""
    if beta_bits == Q16 and not opt.power_at_beta_one:
        return [None] * GRID_POINTS
    return [0, *(exp_q32(rhe(beta_bits * lam, Q16)) for lam in LAMBDA[1:-1]), 0]


def survivor(i: int, k: int, w: list[int | None]) -> int:
    """Return F4's L_i(k): 2^32 / (1 + K W_i) rounded once, in the rational or the power form."""
    if i == 0:
        return ONE
    if i == GRID_POINTS - 1:
        return 0
    wi = w[i]
    if wi is None:
        return rhe(S[i] << 64, (S[i] << 32) + k * (SCALE - S[i]))  # rung 0's S3, verbatim
    return rhe(1 << 96, (1 << 64) + k * wi)


def grid(k: int, w: list[int | None]) -> list[int]:
    """Return L_i(k) for the 71 grid ages."""
    return [survivor(i, k, w) for i in range(GRID_POINTS)]


def trapezoid(lx: Sequence[int]) -> int:
    """Return F5's T (S4): e0 in units of 2^-34 years over the standard grid."""
    return sum((H[i + 1] - H[i]) * (lx[i] + lx[i + 1]) for i in range(GRID_POINTS - 1))


def solve_k(e_bits: int, w: list[int | None]) -> int:
    """Return F5's K (S5): 40 bisection steps on the trapezoid e0 of the standard grid."""
    target = e_bits << 18
    lo, hi = 0, K_HI
    for _ in range(STEPS):
        mid = (lo + hi) >> 1
        if trapezoid(grid(mid, w)) >= target:
            lo = mid
        else:
            hi = mid
    return lo


# ----------------------------------------------------------------------------- F7-F9: the F1 knots
def infant_knots(l1: int, shares: Sequence[int]) -> list[int]:
    """Return F7's 13 survivors at ages j / 12 years: 2^32 - rhe(q0 c_j / 2^16), q0 = 2^32 - l1."""
    q0 = ONE - l1
    return [ONE - rhe(q0 * c, Q16) for c in shares]


@dataclass(frozen=True)
class Tail:
    """F8's derived logistic (Beard's Perks logistic, R12 §9 p. 59): mu at 90 and its log-slope."""

    m1: int  # Brass's per-year hazard over [87.5, 90], Q32_32
    m2: int  # over [90, 92.5]
    mu: int  # the hazard at 90: (m1 + m2) / 2
    a: int  # mu / (1 - mu)
    b: int  # g / (1 - mu), g = ln(m2 / m1) / 2.5


def tail_parameters(lx: Sequence[int]) -> Tail:
    """Return F8's (a, b) from the Brass survivors at 87.5, 90 and 92.5."""
    l87, l90, l92 = lx[GRID_87], lx[GRID_90], lx[GRID_92]
    assert l87 > l90 > l92 > 0, "F8 precondition: strictly decreasing survivors around 90"
    m1 = rhe(2 * ln_ratio(l87, l90), HALF_YEARS_PER_INTERVAL)
    m2 = rhe(2 * ln_ratio(l90, l92), HALF_YEARS_PER_INTERVAL)
    assert m2 > m1 > 0, "F8 precondition: the hazard rises through 90"
    mu = rhe(m1 + m2, 2)
    assert mu < ONE, "F8 precondition: the hazard at 90 is below one per year"
    a = rhe(mu * ONE, ONE - mu)
    g = rhe(2 * ln_ratio(m2, m1), HALF_YEARS_PER_INTERVAL)
    b = rhe(g * ONE, ONE - mu)
    assert b > 0 and b * (AGE_120 - AGE_90) <= EXP_MAX, "F8 precondition: 30 b <= 16"
    return Tail(m1, m2, mu, a, b)


def tail_knots(l90: int, tail: Tail) -> list[int]:
    """Return F8's 361 survivors at ages 90 + j / 12 years, j = 0..360 (the last is 0)."""
    knots = [l90]
    for j in range(1, TAIL_MONTHS):
        e_j = exp_q32(rhe(tail.b * j, MONTHS))
        a_j = ONE + rhe(tail.a * e_j, ONE)
        r_j = ln_ratio(ONE + tail.a, a_j)
        s_j = exp_q32(rhe(r_j * ONE, tail.b))
        knots.append(min(knots[-1], rhe(l90 * s_j, ONE)))
    knots.append(0)
    return knots


@dataclass(frozen=True)
class Schedule:
    """One compiled F1 schedule: the level, the grid, the tail and the knots in years."""

    e_bits: int
    beta_bits: int
    k: int
    grid: list[int]
    tail: Tail
    knot_twelfths: list[int]  # knot ages in twelfths of a year
    knot_l: list[int]

    def ages(self, year: int) -> list[int]:
        """Return the knot ages in minutes for a calendar of ``year`` minutes (exact: 12 | year)."""
        return [t * year // MONTHS for t in self.knot_twelfths]


def compile_schedule(e0: str, beta: str, opt: Options | None = None) -> Schedule:
    """Return the F1 schedule of the manifest literals ``e0`` and ``beta`` (F2-F9)."""
    opt = opt or Options()
    e_bits, beta_bits = q16(e0), q16(beta)
    w = odds_grid(beta_bits, opt)
    k = solve_k(e_bits, w)
    lx = grid(k, w)
    tail = tail_parameters(lx)
    infant = infant_knots(lx[1], opt.shares)
    above = tail_knots(lx[GRID_90], tail)
    twelfths = list(range(INFANT_KNOTS))  # 0 .. 12 months
    survivors = list(infant)
    for i in range(2, GRID_90 + 1):  # ages 2 .. 90 (age 1 is the infant knot 12)
        twelfths.append(H[i] * MONTHS // 2)
        survivors.append(lx[i])
    twelfths += [AGE_90 * MONTHS + j for j in range(1, TAIL_MONTHS + 1)]
    survivors += above[1:]
    return Schedule(e_bits, beta_bits, k, lx, tail, twelfths, survivors)


# ----------------------------------------------------------------------------- F9-F13: the month
def ell(sched: Schedule, x: int, year: int) -> int:
    """Return F9's l(x) for an age of ``x`` minutes: linear between knots (S7), 0 from 120."""
    assert x >= 0, "a negative age is an error"
    ages = sched.ages(year)
    if x >= ages[-1]:
        return 0
    i = max(j for j in range(len(ages)) if ages[j] <= x)
    lo, hi = sched.knot_l[i], sched.knot_l[i + 1]
    return lo + rhe((hi - lo) * (x - ages[i]), ages[i + 1] - ages[i])


def month_q(sched: Schedule, x: int, month: int, year: int) -> int:
    """Return F10's Prob32 bits of dying within ``month`` minutes from age ``x`` (S8)."""
    now = ell(sched, x, year)
    return ONE if now == 0 else rhe(ONE * (now - ell(sched, x + month, year)), now)


def density_multiplier(living: int, capacity: int, slope_bits: int) -> int:
    """Return F11's m (Q16_16 bits): 1 at or under the capacity, 1 + slope (N / K - 1) above."""
    if living <= capacity:
        return Q16
    return min(MODIFIER_CAP, Q16 + rhe(slope_bits * (living - capacity), capacity))


def modified_q(q: int, m: int) -> int:
    """Return F11's q' = min(2^32, rhe(q m / 2^16)); exactly q when m = 2^16."""
    return min(ONE, rhe(q * m, Q16))


def hits(draw: int, q: int) -> bool:
    """Return the Prob32 mapping: hits iff q = 2^32 or draw < q 2^32."""
    return q == ONE or draw < q << 32


def death_day(t_m: int, month: int, draw: int) -> int:
    """Return S9's death instant: t_m + 1440 j, j = 1 + floor(draw (n - 1) / 2^64)."""
    n = month // DAY
    return t_m + DAY * (1 + ((draw * (n - 1)) >> 64))


def resample_day(t_in: int, t_end: int, draw: int) -> int | None:
    """Return S11's re-drawn death instant, or None when no day start is left after ``t_in``."""
    first = (t_in // DAY + 1) * DAY
    c = (t_end - first) // DAY if first < t_end else 0
    return None if c == 0 else first + DAY * ((draw * c) >> 64)


def realized_e0(sched: Schedule) -> Fraction:
    """Return the trapezoid e0 over the F1 knots, in years (check 8; not part of the algorithm)."""
    t, lx = sched.knot_twelfths, sched.knot_l
    area = sum((t[i + 1] - t[i]) * (lx[i] + lx[i + 1]) for i in range(len(t) - 1))
    return Fraction(area, 2 * MONTHS * ONE)


# ----------------------------------------------------------------------------- the checks
def rung0_survivors(k: int) -> list[int]:
    """Return rung 0's S3 for a level k, written as selftest_m2_rung0.py writes it (check 1)."""
    return [0 if s == 0 else rhe(s << 64, (s << 32) + k * (SCALE - s)) for s in S]


def check_beta_one(opt: Options) -> bool:
    """Check 1: at beta = 1, K, the grid and l(x) on [1, 90] years equal rung 0's."""
    rows = tomllib.loads(SPEC_M2.read_text(encoding="utf-8"))["algorithm"]["mortality_rung0"]
    ok = True
    for row in rows["check_values"]:
        sched = compile_schedule(row["e0"], "1", opt)
        same_grid = sched.grid == rung0_survivors(sched.k)
        ok &= sched.k == row["K"] and same_grid
        print(f"   e0 {row['e0']:>6}: K = {sched.k} (rung 0: {row['K']}), grid equal: {same_grid}")
    sched = compile_schedule("30", "1", opt)
    rung0 = rung0_survivors(sched.k)
    for year in (YEAR_360, YEAR_365):
        ages = [h * year // 2 for h in H]
        for x in range(year, AGE_90 * year + 1, MONTH_360 * 5 + 7):
            i = max(j for j in range(GRID_POINTS) if ages[j] <= x)
            want = rung0[i] + rhe((rung0[i + 1] - rung0[i]) * (x - ages[i]), ages[i + 1] - ages[i])
            ok &= ell(sched, x, year) == want
    ok &= density_multiplier(60, 10_000_000, Q16) == Q16
    print("   l(x) on [1, 90] years equals rung 0's S7 at every sampled minute, both calendars")
    return ok


def check_widths(opt: Options) -> bool:
    """Check 2: the widths the spec states, at the extremes of the Registry bounds."""
    w_max = max(x for x in odds_grid(q16("1.65"), opt) if x)
    worst_kw = (K_HI * w_max).bit_length()
    year = YEAR_365
    sched = compile_schedule("85", "0.6", opt)
    ages = sched.ages(year)
    worst_gap = max(b - a for a, b in itertools.pairwise(ages))
    worst_knot = (ONE * worst_gap).bit_length()
    tails = [compile_schedule(e, b, opt).tail for e, b in CORNERS]
    worst_b = max(t.b for t in tails)
    worst_a = max(t.a for t in tails)
    a_j = ONE + rhe(worst_a * exp_q32(rhe(worst_b * (TAIL_MONTHS - 1), MONTHS)), ONE)
    print(
        f"   bit lengths: k w_i {worst_kw} (<= 94), F4 numerator 97 (2^96), knot product "
        f"{worst_knot} (<= 53), A_j {a_j.bit_length()} (< 63); w_max = {w_max / ONE:.4e}"
    )
    narrow = a_j.bit_length() < LN_OPERAND_BITS
    return worst_kw <= 94 and worst_knot <= 53 and narrow  # noqa: PLR2004


def check_corners(opt: Options) -> bool:
    """Check 3: bracketing, monotone T, ordered knots, the tail's preconditions at the corners."""
    ok = True
    for e0, beta in CORNERS + DESIGN_POINTS:
        sched = compile_schedule(e0, beta, opt)
        w = odds_grid(sched.beta_bits, opt)
        target = sched.e_bits << 18
        ok &= trapezoid(grid(sched.k, w)) >= target > trapezoid(grid(sched.k + 1, w))
        ok &= trapezoid(grid(1, w)) > target > trapezoid(grid(K_HI, w))
        ok &= all(a < b for a, b in itertools.pairwise(sched.knot_twelfths))
        ok &= all(a >= b for a, b in itertools.pairwise(sched.knot_l))
        ok &= sched.tail.m2 > sched.tail.m1 and sched.tail.mu < ONE
        print(
            f"   e0 {e0:>9}, beta {beta:>4}: K = {sched.k / ONE:.6f}, mu(90) = "
            f"{sched.tail.mu / ONE:.4f}/yr, b = {sched.tail.b / ONE:.4f}/yr, "
            f"l(90) = {sched.grid[GRID_90] / ONE:.2e}"
        )
    samples = [trapezoid(grid(k, odds_grid(q16("1.3"), opt))) for k in range(1, K_HI, K_HI // 97)]
    ok &= all(a >= b for a, b in itertools.pairwise(samples))
    rows = tomllib.loads(SPEC_F1.read_text(encoding="utf-8"))["algorithm"]["mortality_f1"]
    for row in rows["check_values"]:
        sched = compile_schedule(row["e0"], row["beta"], opt)
        ok &= (sched.k, sched.tail.a, sched.tail.b) == (row["K"], row["tail_a"], row["tail_b"])
    print(f"   the spec's {len(rows['check_values'])} check values (K, a, b) hold")
    return ok


def check_tail(opt: Options) -> bool:
    """Check 4: continuity at 90, zero at 120, hazard continuity, survival 90 to 100."""
    ok = True
    for e0, beta in DESIGN_POINTS:
        sched = compile_schedule(e0, beta, opt)
        year = YEAR_360
        l90 = ell(sched, AGE_90 * year, year)
        ok &= l90 == sched.grid[GRID_90] and ell(sched, AGE_120 * year, year) == 0
        ok &= month_q(sched, AGE_120 * year - MONTH_360, MONTH_360, year) == ONE
        first = Fraction(ONE - month_q(sched, AGE_90 * year, MONTH_360, year), ONE)
        hazard = -Fraction(ln_ratio(first.numerator, first.denominator), ONE) * MONTHS
        mu = Fraction(sched.tail.mu, ONE)
        ok &= abs(hazard / mu - 1) <= TAIL_HAZARD_TOL
        s100 = Fraction(ell(sched, 100 * year, year), l90)
        print(
            f"   e0 {e0:>9}, beta {beta:>4}: tail hazard in the first month {float(hazard):.4f}/yr "
            f"(mu(90) {float(mu):.4f}; Brass's 87.5-90 {sched.tail.m1 / ONE:.4f}, 90-92.5 "
            f"{sched.tail.m2 / ONE:.4f}), survival 90->100 {float(s100):.4f}, l(110)/l(90) "
            f"{ell(sched, 110 * year, year) / l90:.2e}"
        )
    return ok


def check_infant(opt: Options) -> bool:
    """Check 5: the share table recomputed, normalised, non-decreasing; continuity at age 1."""
    t = tomllib.loads(INFANT.read_text(encoding="utf-8"))["infant_sub_schedule"]
    eps = Decimal(t["endogenous_share"])
    den = Decimal(366).ln() ** 3
    want = [0]
    for j in range(1, INFANT_KNOTS):
        c = eps + (1 - eps) * (Decimal(1) + Decimal(365 * j) / MONTHS).ln() ** 3 / den
        want.append(_round_half_even(c * Q16))
    shares = opt.shares
    ok = shares[0] == 0 and shares[-1] == Q16 and len(shares) == INFANT_KNOTS
    ok &= all(a <= b for a, b in itertools.pairwise(shares))
    ok &= want == list(t["cumulative_share_q16"])
    sched = compile_schedule("30", "1", opt)
    ok &= sched.knot_l[INFANT_KNOTS - 1] == sched.grid[1] and sched.knot_l[0] == ONE
    q0 = ONE - sched.grid[1]
    q_first = month_q(sched, 0, MONTH_360, YEAR_360)
    print(
        f"   shares recomputed: {want == list(t['cumulative_share_q16'])}; first-month share "
        f"{shares[1] / Q16:.4f}; e0 30: q(0) = {q0 / ONE:.4f}, first month q = {q_first / ONE:.4f}"
    )
    return ok


def check_density() -> bool:
    """Check 6: the modifier is exactly 1 at or under the capacity, linear above, capped at 4."""
    slope = Q16
    ok = all(density_multiplier(n, 100, slope) == Q16 for n in (0, 1, 50, 100))
    ok &= density_multiplier(150, 100, slope) == Q16 + Q16 // 2
    ok &= density_multiplier(200, 100, q16("0.5")) == Q16 + Q16 // 2
    ok &= density_multiplier(10**7, 100, slope) == MODIFIER_CAP
    ok &= density_multiplier(10_001, 10_000, q16("3")) == Q16 + rhe(q16("3"), 10_000)
    sched = compile_schedule("30", "1")
    for x in range(0, AGE_120 * YEAR_360, MONTH_360 * 11 + 3):
        q = month_q(sched, x, MONTH_360, YEAR_360)
        ok &= modified_q(q, Q16) == q and 0 <= modified_q(q, MODIFIER_CAP) <= ONE
        ok &= modified_q(q, Q16 + Q16 // 2) >= q
    print(
        "   m = 1 exactly for N <= K; m = 1 + slope (N / K - 1) above; ceiling 4; q' = q at m = 1"
    )
    return ok


def check_draws() -> bool:
    """Check 7: F12 = S9-S11, unchanged from rung 0 (its check 6, repeated)."""
    ok = True
    for d in (0, 1, (1 << 63), (1 << 64) - 1):
        tau = death_day(0, MONTH_360, d)
        ok &= tau % DAY == 0 and DAY <= tau <= MONTH_360 - DAY
        ok &= death_day(0, 28 * DAY, d) <= 27 * DAY
        for t_in in (0, 5, DAY, 13 * DAY + 7, MONTH_360 - DAY - 1):
            r = resample_day(t_in, MONTH_360, d)
            ok &= r is not None and r % DAY == 0 and t_in < r < MONTH_360
        ok &= resample_day(MONTH_360 - DAY, MONTH_360, d) is None
    ok &= hits((1 << 64) - 1, ONE) and not hits(0, 0) and hits(0, 1) and not hits(1 << 32, 1)
    print("   death days 1..29 inside the month; re-samples after the input; the Prob32 mapping")
    return ok


def check_e0(opt: Options) -> bool:
    """Check 8: the realized e0 within 0.1 years of the input; V-SF-02b's l(65) / l(15)."""
    ok = True
    for e0, beta in DESIGN_POINTS:
        sched = compile_schedule(e0, beta, opt)
        got = realized_e0(sched)
        tol = E0_TOL if Fraction(Decimal(e0)) <= E0_TOL_SPLIT else E0_TOL_HIGH
        ok &= abs(got - Fraction(Decimal(e0))) <= tol
        print(f"   e0 {e0:>9}, beta {beta:>4}: realized e0 {float(got):.4f} years")
    for beta in SF02B_BETAS:
        sched = compile_schedule("30", beta, opt)
        l15, l65 = sched.grid[H.index(2 * AGE_15)], sched.grid[H.index(2 * AGE_65)]
        print(f"   V-SF-02b at e0 30, beta {beta}: l(65) / l(15) = {l65 / l15:.4f}")
    return ok


# ----------------------------------------------------------------------------- the golden file
def render(opt: Options) -> str:
    """Return the golden file: per design point the level, the tail, every knot, sample months."""
    lines = [
        "# Golden vectors of M2 F1 mortality (specs/demography/mortality_f1.toml). GENERATED by",
        "# specs/demography/selftest_m2_f1_mortality.py; never edit. Q32_32 and Prob32 as bits.",
        "# schedule <e0> <beta> <K> <a> <b>; knot <twelfths> <l>; q <year> <age_minutes> <q>;",
        "# modifier <living> <capacity> <slope_q16> <m>; qmod <q> <m> <q'>",
    ]
    for e0, beta in DESIGN_POINTS + CORNERS:
        sched = compile_schedule(e0, beta, opt)
        lines.append(f"schedule {e0} {beta} {sched.k} {sched.tail.a} {sched.tail.b}")
        lines += [f"knot {t} {lx}" for t, lx in zip(sched.knot_twelfths, sched.knot_l, strict=True)]
        for year, month in ((YEAR_360, MONTH_360), (YEAR_365, 31 * DAY), (YEAR_365, 28 * DAY)):
            for x in (0, DAY, MONTH_360, year // 2, year, 10 * year + 7, 50 * year, 89 * year):
                lines.append(f"q {year} {x} {month_q(sched, x, month, year)}")
            for x in (
                AGE_90 * year,
                95 * year + 11,
                100 * year,
                110 * year,
                AGE_120 * year - month,
            ):
                lines.append(f"q {year} {x} {month_q(sched, x, month, year)}")
    for living, cap, slope in (
        (60, 10_000_000, Q16),
        (150, 100, Q16),
        (10**7, 100, Q16),
        (10_001, 10_000, q16("3")),
    ):
        lines.append(f"modifier {living} {cap} {slope} {density_multiplier(living, cap, slope)}")
    for q, m in ((131070, Q16), (131070, Q16 + Q16 // 2), (ONE - 1, MODIFIER_CAP), (ONE, Q16)):
        lines.append(f"qmod {q} {m} {modified_q(q, m)}")
    return "\n".join(lines) + "\n"


def check_golden(golden: Path, opt: Options) -> int:
    """Return EXIT_OK if ``golden`` equals the rendered text, else EXIT_STALE."""
    text = render(opt)
    try:
        current = golden.read_text(encoding="utf-8")
    except OSError as e:
        print(f"STALE: cannot read {golden}: {e}", file=sys.stderr)
        return EXIT_STALE
    if current != text:
        print(f"STALE: {golden} differs from the reference", file=sys.stderr)
        return EXIT_STALE
    print(f"OK: {golden} matches the reference ({text.count(chr(10))} lines)")
    return EXIT_OK


def selftest(opt: Options) -> bool:
    """Run checks 1 to 8 and print PASS or FAIL per check."""
    checks: list[tuple[str, Callable[[], bool]]] = [
        ("1. rung 0 = beta 1", lambda: check_beta_one(opt)),
        ("2. widths", lambda: check_widths(opt)),
        ("3. corners", lambda: check_corners(opt)),
        ("4. tail", lambda: check_tail(opt)),
        ("5. infant sub-schedule", lambda: check_infant(opt)),
        ("6. density modifier", check_density),
        ("7. draws", check_draws),
        ("8. realized e0", lambda: check_e0(opt)),
    ]
    ok = True
    for name, fn in checks:
        good = fn()
        ok &= good
        print(f"{name}: {'PASS' if good else 'FAIL'}")
    print("M2 F1 MORTALITY:", "PASS" if ok else "FAIL")
    return ok


def options(control: str | None) -> Options:
    """Return the options of a control that must fail, or the plain ones."""
    opt = Options()
    if control == "beta-one-power":
        opt.power_at_beta_one = True
    elif control == "shares-unnormalised":
        opt.shares = [*opt.shares[:-1], Q16 - 1]
    elif control is not None:
        msg = f"unknown control {control}"
        raise SystemExit(msg)
    return opt


def main(argv: Sequence[str] | None = None) -> int:
    """Self-test, then write or check the golden file."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("--check", action="store_true", help="fail if the golden file is stale")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the file to write or check")
    parser.add_argument("--control", help="a planted violation that must FAIL a check")
    args = parser.parse_args(argv)
    opt = options(args.control)
    if not selftest(opt):
        return EXIT_FAILED
    if args.check:
        return check_golden(args.golden, opt)
    args.golden.parent.mkdir(parents=True, exist_ok=True)
    text = render(opt)
    args.golden.write_text(text, encoding="utf-8")
    print(f"wrote {args.golden} ({text.count(chr(10))} lines)")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
