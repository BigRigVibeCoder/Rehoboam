"""Exact reference of the fixed-point `exp` and `ln` of plane A (SPR-016; `05.1` §6 item 4, v1.10).

Plane A has no transcendental except through this procedure. `rh_core::fixed` (SPR-016) must
reproduce it bit for bit, and `bin/gen_fixed_vectors.py` turns the golden file into Rust vectors.
Every quantity is an integer; the only non-integer arithmetic in this file is the
``decimal`` oracle at 60 digits, which generates the constants once and measures the error of
the integer procedure. It never enters a result.

The format is **Q32_32**: an ``i64`` whose value is ``bits / 2^32`` (32 integer bits, 32 fraction
bits), the wide intermediate between the ``Q16_16`` parameters of a system spec and the ``Prob32``
a draw is compared against. Inputs widen exactly (``Q16_16`` bits ``<< 16``); a ``Prob32`` out is
formed by one exact integer division or subtraction (:func:`prob_from_hazard`).

The algorithm is shift-and-add (De Lugish's pseudo-division, as in CORDIC's hyperbolic mode) with
one table of 32 constants ``C_k = round(ln(1 + 2^-k) 2^64)``, ``LN2_64 = round(ln 2 2^64)`` and
``INV_LN2_62 = round(2^62 / ln 2)``; the working precision is 64 fraction bits in 128-bit integers.
Steps and refusals are stated in `05.1` §6 item 4 (the normative text); this file is the same
procedure in Python.

Checks (``python -I specs/numerics/exp_ln_v0.py``):
  1. the constants equal the 60-digit oracle rounded half to even at their scale;
  2. the convergence condition ``C_k <= sum_{j > k} C_j`` for k = 1..31, which bounds the residual
     of both pseudo-divisions by ``2^-32`` (proof in the contract text);
  3. over the edge cases, the Brass standard's 69 odds and 4,096 pseudo-random inputs: every
     result is within 1 ulp of the true value (faithful), equals the correctly rounded value
     unless the true value lies within ``2^-24`` ulp of a rounding midpoint (for exp, plus the
     relative budget below, in ulps), and the internal value before the final rounding is within
     the budget the contract text proves: ``164 2^-65`` (below ``2^-57.6``) absolute for ln,
     ``133 2^-65`` (below ``2^-57.9``) relative for exp;
  4. the refusals: ``exp`` above 16, ``ln`` of 0 or of an operand of 63 bits or more,
     ``prob_from_hazard`` of a negative hazard; and ``exp(y) = 0`` for every ``y < -23``;
  5. ``prob_from_hazard`` is a ``Prob32`` (bits in [0, 2^32]), non-decreasing, 0 at 0, and
     within 1 ulp of ``1 - e^-h``;
  6. the golden file ``specs/numerics/golden/exp_ln_v0.txt`` (constants and vectors) is current
     (``--check``; ``--check --golden PATH`` checks another file: the positive control plants a
     stale one).

``--exhaustive`` (nightly tier, ``bin/golden_nightly.sh``): check 3 over every ``Q16_16``-granular
``y`` from -23 to 16 (2,555,905 inputs: the whole computed range of exp), every ``ln(n / 2^16)`` for
n in [1, 2^20], and 2^16 pseudo-random ``Q32_32`` inputs of each function (the Q32_32 space itself,
2^63 values, is sampled, never swept).

Exit: 0 ok, 1 stale golden file, 2 a check failed. Standard library only (GOV-003 §3).
Refs: `05.1` §6 item 4 (v1.10); SPR-010 (``rhe``, ``Q16_16``); SPR-011 ruling 4 (``Prob32``);
DET-3; V-DET-19.
"""

from __future__ import annotations

import argparse
import itertools
import sys
import tomllib
from collections.abc import Callable, Iterator, Sequence
from dataclasses import dataclass
from decimal import ROUND_HALF_EVEN, Decimal, getcontext
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GOLDEN = HERE / "golden" / "exp_ln_v0.txt"
STANDARD = ROOT / "specs" / "demography" / "brass_general_standard.toml"
VERSION = "rehoboam/fixed/exp-ln/v0"

EXIT_OK = 0
EXIT_STALE = 1
EXIT_FAILED = 2

FRAC = 32  # Q32_32: 32 fraction bits
ONE = 1 << FRAC
WORK = 64  # working precision: 64 fraction bits
WORK_ONE = 1 << WORK
STEPS = 32  # pseudo-division steps, k = 1..32
EXP_MAX = 16 * ONE  # exp's domain: y <= 16 (e^16 2^32 < 2^56, so the error stays below 1 ulp)
EXP_ZERO_BELOW = -23 * ONE  # for y < -23, e^y 2^32 < 1/2: the correctly rounded result is 0
LN_OPERAND_BITS = 63  # ln operands are positive and below 2^63
INV_LN2_SHIFT = 62
I64_MIN, I64_MAX = -(1 << 63), (1 << 63) - 1
ORACLE_DIGITS = 60
NEAR_TIE = Fraction(1, 1 << 24)  # ulps: a true value this close to a midpoint may round either way
BUDGET_LN = Fraction(
    164, 1 << 65
)  # the proven pre-rounding budget of ln, absolute (05.1 §6 item 4)
BUDGET_EXP = Fraction(133, 1 << 65)  # the proven pre-rounding budget of exp, relative
EXHAUSTIVE_RANDOM = 1 << 16  # pseudo-random Q32_32 inputs per function in the nightly sweep
SEED = 0x5EED_E8_1A  # SplitMix64 seed of the pseudo-random inputs
RANDOM_CASES = 4096
MASK64 = (1 << 64) - 1
Q16_ONE = 1 << 16
EXHAUSTIVE_LN_MAX = 1 << 20

getcontext().prec = ORACLE_DIGITS


# --------------------------------------------------------------------------- the one rounding rule
def rhe(n: int, d: int) -> int:
    """Return n / d rounded to the nearest integer, ties to the even integer (SPR-010); d > 0."""
    q, r = divmod(n, d)
    if 2 * r > d or (2 * r == d and q % 2 == 1):
        q += 1
    return q


def shr_rhe(value: int, shift: int) -> int:
    """Return value / 2^shift rounded half to even (``fixed::round::shr_round_half_even``)."""
    return rhe(value, 1 << shift)


# ----------------------------------------------------------------------------- the constants
def oracle_ln(n: int, d: int = 1) -> Decimal:
    """Return ln(n / d) at 60 digits (the oracle; never part of a result)."""
    return (Decimal(n) / Decimal(d)).ln()


def oracle_exp(y: Fraction) -> Decimal:
    """Return e^y at 60 digits for an exact rational y."""
    return (Decimal(y.numerator) / Decimal(y.denominator)).exp()


def round_half_even(value: Decimal) -> int:
    """Return the integer nearest ``value``, ties to even (the constants' rounding)."""
    return int(value.to_integral_value(rounding=ROUND_HALF_EVEN))


def make_constants() -> tuple[int, int, list[int]]:
    """Return (LN2_64, INV_LN2_62, [C_1 .. C_32]) from the oracle, rounded half to even."""
    ln2 = oracle_ln(2)
    ln2_64 = round_half_even(ln2 * WORK_ONE)
    inv_ln2 = round_half_even(Decimal(1 << INV_LN2_SHIFT) / ln2)
    table = [
        round_half_even(oracle_ln((1 << k) + 1, 1 << k) * WORK_ONE) for k in range(1, STEPS + 1)
    ]
    return ln2_64, inv_ln2, table


LN2_64, INV_LN2_62, C = make_constants()


# ----------------------------------------------------------------------------- the procedure
class RefusedError(ValueError):
    """A refusal by name: the caller's input is outside the function's domain."""


def exp_q32(y: int) -> int:
    """Return e^y as Q32_32 bits for the Q32_32 bits ``y`` (`05.1` §6 item 4, exp steps E1-E6).

    Refuses ``y > 16 2^32`` (``ExpDomain``). Returns 0 for ``y < -23 2^32``, the correctly rounded
    value there (``e^-23 2^32 < 1/2``; the steps below give 0 there too). Otherwise:
    ``n = floor(y INV_LN2_62 / 2^94)``; ``r = y 2^32 - n LN2_64`` (64 fraction bits); 32
    pseudo-division steps on ``r`` building ``p = prod (1 + 2^-k)^{a_k}`` at 64 fraction bits with
    truncation; ``p += floor(p r / 2^64)``; the result is ``p 2^n / 2^32`` rounded half to even.
    """
    if not I64_MIN <= y <= I64_MAX:
        msg = f"ExpDomain: {y} is not a Q32_32"
        raise RefusedError(msg)
    if y > EXP_MAX:
        msg = f"ExpDomain: y = {y} / 2^32 > 16"
        raise RefusedError(msg)
    if y < EXP_ZERO_BELOW:
        return 0
    n = (y * INV_LN2_62) >> (INV_LN2_SHIFT + FRAC)  # floor; n in [-34, 23]
    r = (y << FRAC) - n * LN2_64  # 64 fraction bits; in [-2^-59, ln 2 + 2^-59]
    p = WORK_ONE
    for k in range(1, STEPS + 1):
        if r >= C[k - 1]:
            r -= C[k - 1]
            p += p >> k
    p += (p * r) >> WORK  # first-order term; floor
    return shr_rhe(p, FRAC - n)  # shift in [9, 66]


def ln_ratio(n: int, d: int) -> int:
    """Return ln(n / d) as Q32_32 bits for integers 0 < n, d < 2^63 (ln steps L1-L6).

    ``e`` is the exact binary exponent (``2^e <= n / d < 2^(e+1)``); ``m = floor(n 2^(64-e) / d)``
    is the mantissa at 64 fraction bits; 32 pseudo-division steps build ``p <= m`` and the sum
    ``s`` of the accepted ``C_k``; the residual is ``floor((m - p) 2^64 / p)``; the result is
    ``(e LN2_64 + s + residual) / 2^32`` rounded half to even.
    """
    if not (0 < n < (1 << LN_OPERAND_BITS) and 0 < d < (1 << LN_OPERAND_BITS)):
        msg = f"LnDomain: operands {n}, {d} must be positive and below 2^63"
        raise RefusedError(msg)
    e = n.bit_length() - d.bit_length()
    if (n >> e if e >= 0 else n << -e) < d:  # 2^e > n / d: step down once
        e -= 1
    m = (n << WORK) // (d << e) if e >= 0 else (n << (WORK - e)) // d
    p, s = WORK_ONE, 0
    for k in range(1, STEPS + 1):
        t = p + (p >> k)
        if t <= m:
            p, s = t, s + C[k - 1]
    residual = ((m - p) << WORK) // p
    return shr_rhe(e * LN2_64 + s + residual, FRAC)


def prob_from_hazard(h: int) -> int:
    """Return the Prob32 bits of ``1 - e^-h`` for a cumulative hazard ``h`` (Q32_32 bits, h >= 0).

    ``2^32 - exp_q32(-h)``: exactly 0 at h = 0, exactly 2^32 for h > 23, non-decreasing.
    Refuses a negative hazard (``HazardDomain``).
    """
    if h < 0:
        msg = f"HazardDomain: hazard {h} < 0"
        raise RefusedError(msg)
    return ONE - exp_q32(-h)


# ----------------------------------------------------------------------------- the checks
class SplitMix64:
    """SplitMix64: a fixed, version-independent input stream (as bin/gen_fixed_vectors.py)."""

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

    def below(self, bound: int) -> int:
        """Return a value in [0, bound) (rejection-free modulo: bias is irrelevant for inputs)."""
        return self.next_u64() % bound


@dataclass
class Accuracy:
    """What check 3 measures over a set of inputs."""

    count: int = 0
    unfaithful: int = 0
    not_correctly_rounded: int = 0
    near_ties: int = 0
    worst_internal: Fraction = Fraction(0)
    relative: bool = False  # exp's budget is relative to the true value; ln's is absolute

    @property
    def budget(self) -> Fraction:
        """Return the proven pre-rounding budget of this function."""
        return BUDGET_EXP if self.relative else BUDGET_LN

    def ok(self) -> bool:
        """Return whether the bound holds: faithful, correct away from ties, budget met."""
        return (
            self.unfaithful == 0
            and self.not_correctly_rounded == 0
            and self.worst_internal < self.budget
            and self.count > 0
        )

    def record(self, result: int, truth: Decimal, internal: Fraction | None) -> None:
        """Compare one result (bits) with the true value (as a 60-digit Decimal)."""
        exact = Fraction(truth) * ONE
        floor = exact.numerator // exact.denominator
        self.count += 1
        if result not in (floor, floor + 1):
            self.unfaithful += 1
        correct = round(exact)
        window = NEAR_TIE + (self.budget * exact if self.relative else 0)  # the budget, in ulps
        if result != correct:
            if abs(exact - floor - Fraction(1, 2)) < window:
                self.near_ties += 1
            else:
                self.not_correctly_rounded += 1
        if internal is not None:
            error = abs(internal - Fraction(truth))
            if self.relative:
                error /= Fraction(truth)
            self.worst_internal = max(self.worst_internal, error)


def exp_internal(y: int) -> Fraction:
    """Return exp's value before the final rounding (p 2^n / 2^64), for the budget check."""
    n = (y * INV_LN2_62) >> (INV_LN2_SHIFT + FRAC)
    r = (y << FRAC) - n * LN2_64
    p = WORK_ONE
    for k in range(1, STEPS + 1):
        if r >= C[k - 1]:
            r -= C[k - 1]
            p += p >> k
    p += (p * r) >> WORK
    return Fraction(p, WORK_ONE) * Fraction(2) ** n


def ln_internal(n: int, d: int) -> Fraction:
    """Return ln's value before the final rounding, for the budget check."""
    e = n.bit_length() - d.bit_length()
    if (n >> e if e >= 0 else n << -e) < d:
        e -= 1
    m = (n << WORK) // (d << e) if e >= 0 else (n << (WORK - e)) // d
    p, s = WORK_ONE, 0
    for k in range(1, STEPS + 1):
        t = p + (p >> k)
        if t <= m:
            p, s = t, s + C[k - 1]
    residual = ((m - p) << WORK) // p
    return Fraction(e * LN2_64 + s + residual, WORK_ONE)


def check_constants() -> bool:
    """Check 1: the constants are the oracle's, rounded half to even; print them."""
    ln2_64, inv, table = make_constants()
    ok = (ln2_64, inv, table) == (LN2_64, INV_LN2_62, C) and len(C) == STEPS
    ok &= abs(Fraction(LN2_64, WORK_ONE) - Fraction(oracle_ln(2))) <= Fraction(1, 1 << (WORK + 1))
    ok &= abs(Fraction(INV_LN2_62, 1 << INV_LN2_SHIFT) - 1 / Fraction(oracle_ln(2))) <= Fraction(
        1, 1 << (INV_LN2_SHIFT + 1)
    )
    print(f"   LN2_64 = {LN2_64}, INV_LN2_62 = {INV_LN2_62}, C_1 = {C[0]}, C_32 = {C[-1]}")
    return ok


def check_convergence() -> bool:
    """Check 2: c_k <= B_k, the infinite tail sum_{j > k} c_j, for k = 1..32, with the oracle.

    With that condition the residual of a pseudo-division after step k is below B_k, so after
    step 32 it is below B_32 < 2^-32 (`05.1` §6 item 4). The tail is summed to j = 160; the rest
    is below 2^-160 and is added to the left side.
    """
    exact = [oracle_ln((1 << k) + 1, 1 << k) for k in range(1, 5 * STEPS + 1)]
    tails = [sum(exact[k:], Decimal(0)) for k in range(STEPS + 1)]
    slack = Decimal(2) ** -(5 * STEPS)
    ok = all(exact[k] + slack <= tails[k + 1] for k in range(STEPS))
    ok &= tails[STEPS] < Decimal(2) ** -STEPS  # B_32 < 2^-32
    ok &= Decimal(LN2_64) / WORK_ONE < tails[0]  # ln 2 < B_0: the whole of [0, ln 2] is reachable
    ok &= Fraction(C[-1], WORK_ONE) <= Fraction(1, 1 << STEPS)  # C_32 rounds to exactly 2^-32
    print(
        f"   B_0 = {tails[0]:.6f} > ln 2; B_32 = {tails[STEPS]:.3e} < 2^-32; c_k <= B_k for k <= 32"
    )
    return ok


def standard_odds() -> list[tuple[int, int]]:
    """Return the Brass General Standard's 69 interior odds (10^4 - s_i, s_i) as integer pairs."""
    t = tomllib.loads(STANDARD.read_text(encoding="utf-8"), parse_float=Decimal)["standard"]
    s = [int(v * 10_000) for v in t["l"]]
    return [(10_000 - v, v) for v in s if 0 < v < 10_000]  # noqa: PLR2004


def exp_inputs(rng: SplitMix64) -> Iterator[int]:
    """Yield exp's edge inputs, the Q16_16 edges and pseudo-random Q32_32 values in the domain."""
    yield from (0, 1, -1, ONE, -ONE, 2 * ONE, -2 * ONE, EXP_MAX, EXP_MAX - 1, EXP_ZERO_BELOW)
    yield from (EXP_ZERO_BELOW + 1, -22 * ONE, ONE // 2, -ONE // 2, 3 * ONE // 2, LN2_64 >> FRAC)
    yield from ((LN2_64 >> FRAC) + 1, (LN2_64 >> FRAC) - 1, 10 * ONE, -10 * ONE, 15 * ONE)
    for i in range(RANDOM_CASES):  # mostly |y| < 1, every third over the whole domain
        if i % 3 == 0:
            yield EXP_ZERO_BELOW + rng.below(EXP_MAX - EXP_ZERO_BELOW + 1)
        else:
            yield rng.below(2 * ONE) - ONE


def ln_inputs(rng: SplitMix64) -> Iterator[tuple[int, int]]:
    """Yield ln's edge operands, the standard's odds, Q16_16 edges and pseudo-random pairs."""
    top = (1 << LN_OPERAND_BITS) - 1
    yield from ((1, 1), (2, 1), (1, 2), (3, 1), (1, 3), (top, 1), (1, top), (top, top), (top, 2))
    yield from ((Q16_ONE, Q16_ONE), (1, Q16_ONE), (Q16_ONE + 1, Q16_ONE), (Q16_ONE - 1, Q16_ONE))
    yield from ((ONE, ONE), (ONE + 1, ONE), (ONE - 1, ONE), (10_000, 1), (1, 10_000))
    yield from standard_odds()
    for i in range(RANDOM_CASES):
        if i % 4 == 0:
            yield rng.below(top) + 1, rng.below(top) + 1
        elif i % 4 == 1:
            yield rng.below(1 << 31) + 1, Q16_ONE  # a Q16_16 value
        elif i % 4 == 2:  # noqa: PLR2004
            yield rng.below(1 << 40) + 1, ONE  # a Q32_32 value
        else:
            yield rng.below(10_000) + 1, rng.below(10_000) + 1  # a rational like the standard's


def hazard_inputs(rng: SplitMix64) -> Iterator[int]:
    """Yield cumulative hazards: 0, the smallest steps, monthly hazards, and beyond 16."""
    yield from (0, 1, 2, ONE >> 20, ONE >> 15, ONE >> 8, ONE >> 4, ONE, 16 * ONE, 23 * ONE + 1)
    yield from (rng.below(ONE >> 6) for _ in range(RANDOM_CASES // 4))


def check_accuracy(exhaustive: bool = False) -> bool:
    """Check 3: faithful rounding, correctness away from ties, the pre-rounding budget."""
    rng = SplitMix64(SEED)
    acc_exp, acc_ln = Accuracy(relative=True), Accuracy()
    ys: Iterator[int] = exp_inputs(rng)
    pairs: Iterator[tuple[int, int]] = ln_inputs(rng)
    if exhaustive:  # the Q16_16 grid of both, then a Q32_32 sample of both
        span = EXP_MAX - EXP_ZERO_BELOW
        ys = itertools.chain(
            range(EXP_ZERO_BELOW, EXP_MAX + 1, Q16_ONE),
            (EXP_ZERO_BELOW + rng.below(span + 1) for _ in range(EXHAUSTIVE_RANDOM)),
        )
        pairs = itertools.chain(
            ((n, Q16_ONE) for n in range(1, EXHAUSTIVE_LN_MAX + 1)),
            ((rng.below(1 << 48) + 1, ONE) for _ in range(EXHAUSTIVE_RANDOM)),
        )
    for y in ys:
        acc_exp.record(exp_q32(y), oracle_exp(Fraction(y, ONE)), exp_internal(y))
    for n, d in pairs:
        acc_ln.record(ln_ratio(n, d), oracle_ln(n, d), ln_internal(n, d))
    for name, acc in (("exp", acc_exp), ("ln", acc_ln)):
        print(
            f"   {name}: {acc.count} inputs, unfaithful {acc.unfaithful}, not correctly rounded "
            f"away from a tie {acc.not_correctly_rounded}, near ties {acc.near_ties}, worst "
            f"pre-rounding error {float(acc.worst_internal):.2e} "
            f"{'relative' if acc.relative else 'absolute'} (budget {float(acc.budget):.2e})"
        )
    return acc_exp.ok() and acc_ln.ok()


def refused(fn: Callable[[], int]) -> bool:
    """Return whether ``fn`` raises :class:`RefusedError`."""
    try:
        fn()
    except RefusedError:
        return True
    return False


def check_refusals() -> bool:
    """Check 4: the domain refusals by name, and exp's exact 0 below -16."""
    ok = refused(lambda: exp_q32(EXP_MAX + 1)) and refused(lambda: exp_q32(I64_MAX))
    ok &= refused(lambda: ln_ratio(0, 1)) and refused(lambda: ln_ratio(1, 0))
    ok &= refused(lambda: ln_ratio(1 << LN_OPERAND_BITS, 1))
    ok &= refused(lambda: ln_ratio(1, 1 << LN_OPERAND_BITS))
    ok &= refused(lambda: ln_ratio(-1, 1)) and refused(lambda: prob_from_hazard(-1))
    ok &= exp_q32(EXP_ZERO_BELOW - 1) == 0 and exp_q32(I64_MIN) == 0
    ok &= exp_q32(EXP_ZERO_BELOW) == 0 and exp_q32(0) == ONE and ln_ratio(1, 1) == 0
    ok &= exp_q32(-22 * ONE) > 0 and exp_q32(EXP_ZERO_BELOW + 1) == 0
    ok &= ln_ratio(2, 1) == -ln_ratio(1, 2) == shr_rhe(LN2_64, FRAC)
    print("   exp refuses above 16 and is 0 below -23; ln refuses 0 and 63-bit operands")
    return ok


def check_hazard() -> bool:
    """Check 5: prob_from_hazard is a Prob32, monotone, exact at 0, and within 1 ulp."""
    rng = SplitMix64(SEED ^ 1)
    ok = prob_from_hazard(0) == 0 and prob_from_hazard(23 * ONE + 1) == ONE
    last = -1
    for h in sorted(set(hazard_inputs(rng))):
        q = prob_from_hazard(h)
        truth = Fraction(1) - Fraction(oracle_exp(Fraction(-h, ONE)))
        ok &= 0 <= q <= ONE and q >= last and abs(Fraction(q, ONE) - truth) <= Fraction(1, ONE)
        last = q
    tiny = prob_from_hazard(ONE >> 15)  # a monthly hazard of 3e-5
    truth = 1 - Fraction(oracle_exp(Fraction(-1, 1 << 15)))
    print(f"   hazard 2^-15: q = {tiny} bits = {tiny / ONE:.6e} (1 - e^-h = {float(truth):.6e})")
    return ok


# ----------------------------------------------------------------------------- the golden file
def fmt(fn: Callable[..., int], *args: int) -> str:
    """Return ``fn(*args)`` as text: the bits, or ``refused``."""
    try:
        return str(fn(*args))
    except RefusedError:
        return "refused"


def render() -> str:
    """Return the golden file: constants, then every vector of exp, ln and prob_from_hazard."""
    rng = SplitMix64(SEED)
    lines = [
        f"# Golden vectors of {VERSION}. GENERATED by specs/numerics/exp_ln_v0.py; never edit.",
        "# Constants at 64 (LN2_64, C_k) and 62 (INV_LN2_62) fraction bits; vectors: Q32_32 bits.",
        "# exp <y> <bits|refused>; ln <n> <d> <bits|refused>; hazard <h> <Prob32 bits|refused>.",
        f"const LN2_64 {LN2_64}",
        f"const INV_LN2_62 {INV_LN2_62}",
        *(f"const C {k} {c}" for k, c in enumerate(C, start=1)),
    ]
    lines += [f"exp {y} {fmt(exp_q32, y)}" for y in exp_inputs(rng)]
    lines += [f"exp {y} {fmt(exp_q32, y)}" for y in (EXP_MAX + 1, I64_MIN, I64_MAX)]
    lines += [f"ln {n} {d} {fmt(ln_ratio, n, d)}" for n, d in ln_inputs(rng)]
    lines += [f"ln {n} {d} {fmt(ln_ratio, n, d)}" for n, d in ((0, 1), (1 << 63, 1))]
    lines += [f"hazard {h} {fmt(prob_from_hazard, h)}" for h in hazard_inputs(rng)]
    lines.append("hazard -1 refused")
    return "\n".join(lines) + "\n"


def check_golden(golden: Path) -> int:
    """Return EXIT_OK if ``golden`` equals :func:`render`'s text, else EXIT_STALE."""
    text = render()
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


def selftest(exhaustive: bool) -> bool:
    """Run checks 1 to 5 and print PASS or FAIL per check."""
    checks: list[tuple[str, Callable[[], bool]]] = [
        ("1. constants", check_constants),
        ("2. convergence", check_convergence),
        (
            "3. accuracy" + (" (exhaustive)" if exhaustive else ""),
            lambda: check_accuracy(exhaustive),
        ),
        ("4. refusals", check_refusals),
        ("5. prob_from_hazard", check_hazard),
    ]
    ok = True
    for name, fn in checks:
        good = fn()
        ok &= good
        print(f"{name}: {'PASS' if good else 'FAIL'}")
    print("EXP/LN v0:", "PASS" if ok else "FAIL")
    return ok


def main(argv: Sequence[str] | None = None) -> int:
    """Self-test, then write or check the golden file."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("--check", action="store_true", help="fail if the golden file is stale")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the file to write or check")
    parser.add_argument("--exhaustive", action="store_true", help="the nightly sweep of check 3")
    args = parser.parse_args(argv)
    if not selftest(args.exhaustive):
        return EXIT_FAILED
    if args.check:
        return check_golden(args.golden)
    args.golden.parent.mkdir(parents=True, exist_ok=True)
    text = render()
    args.golden.write_text(text, encoding="utf-8")
    print(f"wrote {args.golden} ({text.count(chr(10))} lines)")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
