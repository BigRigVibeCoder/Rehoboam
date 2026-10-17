"""Exact reference and self-test of M2 F1's fertility and partnering arithmetic (SPR-015).

The algorithms are stated in the M2 F1 spec (``specs/systems/next/M2.demography.odd.toml`` while
it is a draft; ``specs/systems/M2.demography.odd.toml`` once it supersedes the rung-0 slice),
sections [algorithm.fertility_f1] (steps F0 to F10) and [algorithm.partnering_f1] (N1 to N7).
This file implements their arithmetic with Python integers and ``fractions`` only, so it
reproduces the engine's integers bit for bit, and it checks:

  1. the spec's check values: the natural-fertility standard, the nuptiality standard as Prob32
     bits, the reference TFR of the nuptiality schedule (``TFR1``) as Q16_16 bits, the level M
     and the seven conception probabilities at the default ``tfr``, and the sex ratio at birth;
  2. the widths the spec states hold at the Registry extremes (tfr 7.5 with
     elite_fertility_mult 1.6 included), and every conception probability is a Prob32 in
     [0, 2^32], non-decreasing in tfr and in the multiplier;
  3. the level-to-rate identity: the long-run birth rate of a woman drawn monthly at c_i with a
     closed period of G + P months is M n_i per year, exactly, so ``tfr`` means what it says;
  4. the nuptiality schedule's shape: mean age at first marriage and the share ever married by
     50 lie in the design band the spec states (the F1 early-marriage pattern);
  5. the stationary estimate: with the rung-0 mortality schedule at e0 = 30 (the Registry
     default), the net reproduction rate at the default ``tfr`` lies inside V-SF-01's growth
     band under the no-widowhood approximation the spec states;
  6. timers: a birth falls on a day start of the due month, never its pulse instant; a marriage
     on a day start of its own month; a re-sampled birth lies after the input and never drops.

Run: python -I specs/demography/selftest_m2_f1.py
     python -I specs/demography/selftest_m2_f1.py --spec <path>   (a planted control: must FAIL)
"""

from __future__ import annotations

import importlib.util
import math
import sys
import tomllib
from collections.abc import Callable
from decimal import Decimal
from fractions import Fraction
from pathlib import Path
from types import ModuleType
from typing import Any

HERE = Path(__file__).resolve().parent
NEXT_SPEC = HERE.parent / "systems" / "next" / "M2.demography.odd.toml"
MAIN_SPEC = HERE.parent / "systems" / "M2.demography.odd.toml"


def _load_rung0() -> ModuleType:
    """Load the rung-0 reference beside this file (python -I adds no script dir to sys.path)."""
    spec = importlib.util.spec_from_file_location(
        "selftest_m2_rung0", HERE / "selftest_m2_rung0.py"
    )
    if spec is None or spec.loader is None:
        raise SystemExit("selftest_m2_rung0.py not found beside selftest_m2_f1.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# S0 rhe, q16, the rung-0 schedule (survivors, solve_k, ell) and the day draws.
rung0 = _load_rung0()

ONE = 1 << 32  # Prob32's 1
Q16 = 1 << 16
THOUSAND = 1000
BANDS = 7  # five-year bands [15, 20) .. [45, 50)
BAND_YEARS = 5
BAND_MONTHS = 60
FIRST_AGE = 15  # years: the first fertile and marriageable band starts here
LAST_AGE = 50
MONTHS_PER_YEAR = 12
DAY = 1440
MONTH_360 = 30 * DAY
YEAR_360 = 360 * DAY
# F1: Coale & Trussell (1974) n(a), thousandths of births per married woman-year, bands 15-19..45-49
NATURAL = [411, 460, 431, 395, 322, 167, 24]
# N1 design values: monthly first-marriage probability of an unmarried woman, thousandths, by band
NUPTIALITY = [18, 35, 24, 11, 5, 2, 2]
GESTATION = 9  # months (F2)
POSTPARTUM = 11  # months (F2; Barrett's mean non-susceptible period, R12 §9)
CLOSED = GESTATION + POSTPARTUM
SEX_RATIO = (105, 205)  # male births per 205 births (F2)
TFR_BOUNDS = (Decimal("1.2"), Decimal("7.5"))  # Registry [systems."M2.demography"].params.tfr
MULT_BOUNDS = (Decimal("1.0"), Decimal("1.6"))  # elite_fertility_mult
MARRIAGE_AGE_BAND = (18.0, 22.0)  # design band: mean age at first marriage, years (check 4)
EVER_MARRIED_MIN = Fraction(90, 100)  # by 50 (check 4)
GROWTH_BAND = (Fraction(-1, 1000), Fraction(3, 1000))  # V-SF-01: r in [-0.1%, +0.3%] a year
FEMALE_SHARE = Fraction(SEX_RATIO[1] - SEX_RATIO[0], SEX_RATIO[1])
NUM_BITS_MAX = 60  # F4 numerator width the spec states
DEN_BITS_MAX = 30  # F4 denominator width the spec states


def rhe(n: int, d: int) -> int:
    """Return n / d rounded to the nearest integer, ties to even (S0 / F0); d > 0."""
    return int(rung0.rhe(n, d))


def q16(literal: str) -> int:
    """Return the Q16_16 bits of a decimal literal, rounded half to even (SPR-011 ruling 9)."""
    return int(rung0.q16(literal))


def nuptiality_bits() -> list[int]:
    """Return N1's monthly marriage probabilities as Prob32 bits."""
    return [rhe(ONE * m, THOUSAND) for m in NUPTIALITY]


def married_share_by_month() -> list[Fraction]:
    """Return S(a), the share of a no-mortality cohort ever married at the start of month a.

    a counts months from exact age 15 (a = 0) to the start of the month before 50 (a = 419).
    """
    bits = nuptiality_bits()
    unmarried = Fraction(1)
    shares = []
    for a in range(BANDS * BAND_MONTHS):
        shares.append(1 - unmarried)
        unmarried *= 1 - Fraction(bits[a // BAND_MONTHS], ONE)
    return shares


def band_married_shares() -> list[Fraction]:
    """Return F3's pi_i: the mean married share over the 60 months of each band."""
    s = married_share_by_month()
    return [
        sum(s[i * BAND_MONTHS : (i + 1) * BAND_MONTHS], Fraction(0)) / BAND_MONTHS
        for i in range(BANDS)
    ]


def tfr1_exact() -> Fraction:
    """Return F3's reference TFR at M = 1: sum over bands of 5 n_i pi_i, exactly."""
    return sum(
        (
            BAND_YEARS * Fraction(n, THOUSAND) * pi
            for n, pi in zip(NATURAL, band_married_shares(), strict=True)
        ),
        Fraction(0),
    )


def tfr1_bits() -> int:
    """Return TFR1 as Q16_16 bits, rounded half to even (the compiled constant)."""
    return round(tfr1_exact() * Q16)


def level_bits(tfr_bits: int) -> int:
    """Return F3's level M (Q16_16 bits) for the manifest's tfr (Q16_16 bits)."""
    return rhe(tfr_bits * Q16, tfr1_bits())


def elite_level_bits(m_bits: int, mult_bits: int) -> int:
    """Return F4's M_e = M x elite_fertility_mult in Q16_16, rounded half to even."""
    return rhe(m_bits * mult_bits, Q16)


def conception_bits(m_bits: int, band: int) -> int:
    """Return F4's monthly conception probability c_i as Prob32 bits for level ``m_bits``."""
    b = m_bits * NATURAL[band]  # births per woman-year in units of 2^-16 / 1000
    den = MONTHS_PER_YEAR * Q16 * THOUSAND - CLOSED * b
    if den <= 0:
        return ONE
    return min(ONE, rhe(ONE * b, den))


def sex_male_bits() -> int:
    """Return F9's probability that a birth is male, as Prob32 bits."""
    return rhe(ONE * SEX_RATIO[0], SEX_RATIO[1])


def birth_day(t_due_month: int, month: int, draw: int) -> int:
    """Return F6's birth instant: a day start of the due month other than its pulse instant."""
    return int(rung0.death_day(t_due_month, month, draw))


def marriage_day(t_m: int, month: int, draw: int) -> int:
    """Return N4's marriage instant: a day start of this month other than its pulse instant."""
    return int(rung0.death_day(t_m, month, draw))


def resample_birth_day(t_in: int, t_due: int, t_due_end: int, draw: int) -> int:
    """Return F8's re-drawn birth instant: after the input in the due month, else the next day."""
    start = max(t_in, t_due)
    first = (start // DAY + 1) * DAY  # the first day start strictly after start
    c = (t_due_end - first) // DAY if first < t_due_end else 0
    if c == 0:
        return t_due_end + DAY
    return first + DAY * ((draw * c) >> 64)


def load_spec(path: Path) -> dict[str, Any]:
    """Return the parsed spec at ``path``."""
    return tomllib.loads(path.read_text(encoding="utf-8"))


def spec_path(argv: list[str]) -> Path:
    """Return the spec to check: --spec PATH, else the next-version draft, else the main spec."""
    if len(argv) == 3 and argv[1] == "--spec":  # noqa: PLR2004
        return Path(argv[2])
    if len(argv) != 1:
        raise SystemExit("usage: selftest_m2_f1.py [--spec PATH]")
    return NEXT_SPEC if NEXT_SPEC.exists() else MAIN_SPEC


def check_values(spec: dict[str, Any]) -> bool:
    """Check 1: every check value the spec states equals this reference's."""
    alg = spec["algorithm"]
    fert = alg["fertility_f1"]
    part = alg["partnering_f1"]
    cv = fert["check_values"]
    default = str(cv["tfr_default"])
    m = level_bits(q16(default))
    conceptions = [conception_bits(m, i) for i in range(BANDS)]
    got: dict[str, Any] = {
        "natural": NATURAL,
        "tfr1_bits": tfr1_bits(),
        "m_bits_at_default": m,
        "conception_bits_at_default": conceptions,
        "sex_male_bits": sex_male_bits(),
    }
    want = {k: cv[k] for k in got}
    ok = got == want
    for k, value in got.items():
        if value != want[k]:
            print(f"   {k}: spec {want[k]} != reference {value}")
    nupt_ok = part["check_values"]["nuptiality_bits"] == nuptiality_bits()
    if not nupt_ok:
        print(
            f"   nuptiality_bits: spec {part['check_values']['nuptiality_bits']} != "
            f"{nuptiality_bits()}"
        )
    print(
        f"   tfr default {default}: TFR1 = {float(tfr1_exact()):.4f} ({tfr1_bits()} bits), "
        f"M = {m / Q16:.4f}, c = {[round(c / ONE, 5) for c in conceptions]}, "
        f"P(male) = {sex_male_bits() / ONE:.6f}"
    )
    return ok and nupt_ok


def check_widths() -> bool:
    """Check 2: widths at the extremes; Prob32 range; monotone in tfr and in the multiplier."""
    m_hi = elite_level_bits(level_bits(q16(str(TFR_BOUNDS[1]))), q16(str(MULT_BOUNDS[1])))
    num = max(ONE * m_hi * n for n in NATURAL).bit_length()
    den = (MONTHS_PER_YEAR * Q16 * THOUSAND).bit_length()
    ok = num <= NUM_BITS_MAX and den <= DEN_BITS_MAX
    levels = [level_bits(q16(str(TFR_BOUNDS[0] + Decimal(k) / 10))) for k in range(0, 64)]
    for i in range(BANDS):
        cs = [conception_bits(lv, i) for lv in levels]
        ok &= all(0 <= c <= ONE for c in cs) and cs == sorted(cs)
        ok &= conception_bits(m_hi, i) >= conception_bits(level_bits(q16(str(TFR_BOUNDS[1]))), i)
    worst = max(conception_bits(m_hi, i) for i in range(BANDS))
    print(
        f"   widths: F4 numerator {num} bits (<= {NUM_BITS_MAX}), denominator {den} "
        f"(<= {DEN_BITS_MAX}); largest c at tfr 7.5 x 1.6 = {worst / ONE:.4f}"
    )
    return ok


def check_rate_identity() -> bool:
    """Check 3: 12 c / (1 + (G + P) c) = M n_i exactly before rounding, for the default level."""
    ok = True
    for i, n in enumerate(NATURAL):
        b = Fraction(q16("1.0") * n, Q16 * THOUSAND)  # M = 1 exactly
        c = b / (MONTHS_PER_YEAR - CLOSED * b)
        ok &= MONTHS_PER_YEAR * c / (1 + CLOSED * c) == b
        assert i < BANDS
    print(f"   rate identity: 12 c / (1 + {CLOSED} c) = n_i for every band (M = 1)")
    return ok


def check_nuptiality() -> bool:
    """Check 4: mean age at first marriage and the share ever married by 50."""
    s = married_share_by_month()
    bits = nuptiality_bits()
    unmarried = Fraction(1)
    mean_months = Fraction(0)
    for a in range(BANDS * BAND_MONTHS):
        p = unmarried * Fraction(bits[a // BAND_MONTHS], ONE)
        mean_months += p * (a + Fraction(1, 2))
        unmarried -= p
    ever = 1 - unmarried
    mean_age = FIRST_AGE + float(mean_months / ever) / MONTHS_PER_YEAR
    ok = MARRIAGE_AGE_BAND[0] <= mean_age <= MARRIAGE_AGE_BAND[1] and ever >= EVER_MARRIED_MIN
    print(
        f"   nuptiality: mean age at first marriage {mean_age:.2f} (band {MARRIAGE_AGE_BAND}), "
        f"ever married by 50: {float(ever):.4f}; married share at 20: {float(s[BAND_MONTHS]):.3f}, "
        f"at 25: {float(s[2 * BAND_MONTHS]):.3f}"
    )
    return ok


def net_reproduction(m_bits: int, e0: str) -> tuple[Fraction, Fraction]:
    """Return (NRR, mean age of net maternity in years) under the no-widowhood approximation.

    Births per married woman-month are M n_i / 12 (the long-run rate, check 3); the married share
    is S(a) (check 4); survival is the rung-0 schedule at ``e0`` on the 360-day calendar; the
    daughter share is 100/205. Widowhood and remarriage gaps are ignored, which overstates NRR.
    """
    lx: list[int] = [int(v) for v in rung0.survivors(rung0.solve_k(q16(e0)))]
    s = married_share_by_month()
    nrr = Fraction(0)
    moment = Fraction(0)
    for a in range(BANDS * BAND_MONTHS):
        rate = Fraction(m_bits * NATURAL[a // BAND_MONTHS], Q16 * THOUSAND * MONTHS_PER_YEAR)
        age_min = FIRST_AGE * YEAR_360 + a * MONTH_360 + MONTH_360 // 2
        surv = Fraction(int(rung0.ell(lx, age_min, YEAR_360)), ONE)
        term = rate * s[a] * surv * FEMALE_SHARE
        nrr += term
        moment += term * Fraction(age_min, YEAR_360)
    return nrr, moment / nrr


def check_stationary(spec: dict[str, Any]) -> bool:
    """Check 5: the default tfr's growth lies in V-SF-01's band at e0 = 30 (approximation)."""
    cv = spec["algorithm"]["fertility_f1"]["check_values"]
    m = level_bits(q16(str(cv["tfr_default"])))
    nrr, t_mean = net_reproduction(m, "30")
    # r = ln(NRR) / T: tool-side float, information only; the band test is on NRR's exact bounds.
    r = math.log(nrr) / float(t_mean)
    lo = math.exp(float(GROWTH_BAND[0] * t_mean))
    hi = math.exp(float(GROWTH_BAND[1] * t_mean))
    ok = lo <= float(nrr) <= hi
    nrr1, _ = net_reproduction(q16("1.0"), "30")
    print(
        f"   stationary estimate at e0 = 30: NRR = {float(nrr):.4f} (band [{lo:.4f}, {hi:.4f}]), "
        f"r = {100 * r:+.3f}%/yr, mean age of net maternity {float(t_mean):.1f}; "
        f"NRR = 1 at tfr = {float(tfr1_exact() / nrr1):.3f} (no widowhood)"
    )
    return ok


def check_timers() -> bool:
    """Check 6: birth and marriage days are day starts inside their month; re-samples never drop."""
    ok = True
    due = GESTATION * MONTH_360
    for d in (0, 1, 1 << 63, (1 << 64) - 1):
        tau = birth_day(due, MONTH_360, d)
        ok &= tau % DAY == 0 and due + DAY <= tau <= due + MONTH_360 - DAY
        mu = marriage_day(0, MONTH_360, d)
        ok &= mu % DAY == 0 and DAY <= mu <= MONTH_360 - DAY
        for t_in in (0, due - 5, due, due + 7, due + 13 * DAY, due + MONTH_360 - DAY - 1):
            r = resample_birth_day(t_in, due, due + MONTH_360, d)
            ok &= r % DAY == 0 and r > t_in and due < r < due + MONTH_360
        ok &= (
            resample_birth_day(due + MONTH_360 - DAY, due, due + MONTH_360, d)
            == due + MONTH_360 + DAY
        )
        ok &= (
            resample_birth_day(due + MONTH_360 - 1, due, due + MONTH_360, d)
            == due + MONTH_360 + DAY
        )
    ok &= birth_day(due, 28 * DAY, (1 << 64) - 1) == due + 27 * DAY
    print("   timers: births on days 1..n-1 of the due month, marriages on days 1..n-1, no drop")
    return ok


def main() -> int:
    """Run every check and print PASS or FAIL."""
    spec = load_spec(spec_path(sys.argv))
    checks: list[tuple[str, Callable[[], bool]]] = [
        ("1. spec check values", lambda: check_values(spec)),
        ("2. widths and monotonicity", check_widths),
        ("3. rate identity", check_rate_identity),
        ("4. nuptiality shape", check_nuptiality),
        ("5. stationary estimate", lambda: check_stationary(spec)),
        ("6. timers", check_timers),
    ]
    ok = True
    for name, fn in checks:
        good = fn()
        ok &= good
        print(f"{name}: {'PASS' if good else 'FAIL'}")
    print("M2 F1 FERTILITY AND PARTNERING:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
