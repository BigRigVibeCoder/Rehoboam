"""Exact reference and self-test of M2 F1's fertility and partnering arithmetic (SPR-015; round 2).

The algorithms are stated in the M2 F1 spec (``specs/systems/next/M2.demography.odd.toml`` while
it is a draft; ``specs/systems/M2.demography.odd.toml`` once it supersedes the rung-0 slice),
sections [algorithm.fertility_f1] (steps F0 to F10) and [algorithm.partnering_f1] (N1 to N7).
This file implements their arithmetic with Python integers and ``fractions`` only, so it
reproduces the engine's integers bit for bit. The standards it needs (n(a), the nuptiality
thousandths, the gestation and postpartum months, the sex ratio) are READ FROM THE SPEC's
``constants`` tables, so a change to the spec moves the reference (REV-SPR-015 NIT-4). It checks:

  1. the spec's check values: the nuptiality standard as Prob32 bits, the reference TFR of the
     nuptiality schedule (``TFR1``) as Q16_16 bits, the level M and the seven conception
     probabilities at the default ``tfr``, and the sex ratio at birth;
  2. the widths the spec states hold at the Registry extremes (tfr 7.5 with
     elite_fertility_mult 1.6 included), every conception probability is a Prob32 in [0, 2^32],
     non-decreasing in tfr and in the multiplier, and F0's band formula maps every age in
     [15, 50) to a band in 0..6 (REV-SPR-015 R3);
  3. the level-to-rate identity: the long-run birth rate of a woman drawn monthly at c_i with a
     closed period of G + P months is M n_i per year, exactly, so ``tfr`` means what it says;
  4. the nuptiality schedule's shape: mean age at first marriage and the share ever married by
     50 lie in the design band the spec states (the F1 early-marriage pattern);
  5. the stationary estimate WITH WIDOWHOOD (REV-SPR-015 R7): the 42-state chain of one woman
     (married or not, 21 closed-period states, the husband a fixed number of years older and
     dying on the same rung-0 schedule at e0 = 30, a widow re-entering at her band's probability)
     puts the default ``tfr`` inside V-SF-01's growth band; the no-widowhood figure is printed for
     comparison, and the tfr at which NRR = 1 is reported;
  6. timers: a birth falls on a day start of the due month, never its pulse instant; a marriage
     on a day start of its own month; a re-sampled birth lies after the input and never drops;
  7. V-SF-24's report values: births per married woman-year by the mother's age at birth, from
     the no-mortality chain, are NOT M n(a) (REV-SPR-015 MAJ-3); the per-band ratios are printed
     and pinned against the spec's stated figures, and conceptions per susceptible woman-month
     equal c_i exactly (the identity V-SF-24 tests).

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
YEAR_365 = 365 * DAY
TFR_BOUNDS = (Decimal("1.2"), Decimal("7.5"))  # Registry [systems."M2.demography"].params.tfr
MULT_BOUNDS = (Decimal("1.0"), Decimal("1.6"))  # elite_fertility_mult
MARRIAGE_AGE_BAND = (18.0, 22.0)  # design band: mean age at first marriage, years (check 4)
EVER_MARRIED_MIN = Fraction(90, 100)  # by 50 (check 4)
GROWTH_BAND = (Fraction(-1, 1000), Fraction(3, 1000))  # V-SF-01: r in [-0.1%, +0.3%] a year
HUSBAND_OLDER_YEARS = 3  # check 5's widowhood chain: the husband's age gap (the spec's fact)
NUM_BITS_MAX = 60  # F4 numerator width the spec states
DEN_BITS_MAX = 30  # F4 denominator width the spec states
REPORT_TOL = Fraction(1, 100)  # check 7: the spec's two-digit ratios, within a unit (floats)
SECANT_STEPS = 3  # check 5: NRR is almost linear in tfr, so few steps find NRR = 1


class Standards:
    """The spec's compiled constants (`constants` of the two algorithm tables)."""

    def __init__(self, spec: dict[str, Any]) -> None:
        """Read the standards from ``spec``."""
        fert = spec["algorithm"]["fertility_f1"]["constants"]
        part = spec["algorithm"]["partnering_f1"]["constants"]
        self.natural: list[int] = [int(n) for n in fert["natural"]]
        self.gestation: int = int(fert["gestation_months"])
        self.postpartum: int = int(fert["postpartum_months"])
        self.sex_ratio: tuple[int, int] = (int(fert["sex_ratio"][0]), int(fert["sex_ratio"][1]))
        self.nuptiality: list[int] = [int(t) for t in part["nuptiality_thousandths"]]
        if len(self.natural) != BANDS or len(self.nuptiality) != BANDS:
            raise SystemExit("the spec's standards must have seven bands")

    @property
    def closed(self) -> int:
        """The closed period after a conception, in pulses (G + P)."""
        return self.gestation + self.postpartum

    @property
    def female_share(self) -> Fraction:
        """The share of births that are girls."""
        return Fraction(self.sex_ratio[1] - self.sex_ratio[0], self.sex_ratio[1])


def rhe(n: int, d: int) -> int:
    """Return n / d rounded to the nearest integer, ties to even (S0 / F0); d > 0."""
    return int(rung0.rhe(n, d))


def q16(literal: str) -> int:
    """Return the Q16_16 bits of a decimal literal, rounded half to even (SPR-011 ruling 9)."""
    return int(rung0.q16(literal))


def band_of(age_minutes: int, year: int) -> int | None:
    """Return F0's band i = floor((floor(x / Y) - 15) / 5) for 15 Y <= x < 50 Y, else None."""
    if age_minutes < FIRST_AGE * year or age_minutes >= LAST_AGE * year:
        return None
    return (age_minutes // year - FIRST_AGE) // BAND_YEARS


def nuptiality_bits(st: Standards) -> list[int]:
    """Return N1's monthly marriage probabilities as Prob32 bits."""
    return [rhe(ONE * m, THOUSAND) for m in st.nuptiality]


def married_share_by_month(st: Standards) -> list[Fraction]:
    """Return S(a), the share of a no-mortality cohort ever married at the start of month a (F3).

    a counts months from exact age 15 (a = 0) to the start of the month before 50 (a = 419).
    """
    bits = nuptiality_bits(st)
    unmarried = Fraction(1)
    shares = []
    for a in range(BANDS * BAND_MONTHS):
        shares.append(1 - unmarried)
        unmarried *= 1 - Fraction(bits[a // BAND_MONTHS], ONE)
    return shares


def band_married_shares(st: Standards) -> list[Fraction]:
    """Return F3's pi_i: the mean married share over the 60 months of each band."""
    s = married_share_by_month(st)
    return [
        sum(s[i * BAND_MONTHS : (i + 1) * BAND_MONTHS], Fraction(0)) / BAND_MONTHS
        for i in range(BANDS)
    ]


def tfr1_exact(st: Standards) -> Fraction:
    """Return F3's reference TFR at M = 1: sum over bands of 5 n_i pi_i, exactly."""
    return sum(
        (
            BAND_YEARS * Fraction(n, THOUSAND) * pi
            for n, pi in zip(st.natural, band_married_shares(st), strict=True)
        ),
        Fraction(0),
    )


def tfr1_bits(st: Standards) -> int:
    """Return TFR1 as Q16_16 bits, rounded half to even (the compiled constant)."""
    return round(tfr1_exact(st) * Q16)


def level_bits(st: Standards, tfr_bits: int) -> int:
    """Return F3's level M (Q16_16 bits) for the manifest's tfr (Q16_16 bits)."""
    return rhe(tfr_bits * Q16, tfr1_bits(st))


def elite_level_bits(m_bits: int, mult_bits: int) -> int:
    """Return F3's M_e = M x elite_fertility_mult in Q16_16, rounded half to even."""
    return rhe(m_bits * mult_bits, Q16)


def conception_bits(st: Standards, m_bits: int, band: int) -> int:
    """Return F4's monthly conception probability c_i as Prob32 bits for level ``m_bits``."""
    b = m_bits * st.natural[band]  # births per woman-year in units of 2^-16 / 1000
    den = MONTHS_PER_YEAR * Q16 * THOUSAND - st.closed * b
    if den <= 0:
        return ONE
    return min(ONE, rhe(ONE * b, den))


def sex_male_bits(st: Standards) -> int:
    """Return F9's probability that a birth is male, as Prob32 bits."""
    return rhe(ONE * st.sex_ratio[0], st.sex_ratio[1])


def birth_day(t_due_month: int, month: int, draw: int) -> int:
    """Return F6's birth instant: a day start of the due month other than its pulse instant."""
    return int(rung0.death_day(t_due_month, month, draw))


def marriage_day(t_m: int, month: int, draw: int) -> int:
    """Return N4's marriage instant: a day start of this month other than its pulse instant."""
    return int(rung0.death_day(t_m, month, draw))


def resample_birth_day(t_in: int, t_due: int, t_due_end: int, draw: int) -> int:
    """Return F8's re-drawn birth instant: after the input in the due month, else the next day.

    ``t_due`` and ``t_due_end`` are the due month's bounds, which F8 derives from the timer's
    payload (the conception pulse), never from state.
    """
    start = max(t_in, t_due)
    first = (start // DAY + 1) * DAY  # the first day start strictly after start
    c = (t_due_end - first) // DAY if first < t_due_end else 0
    if c == 0:
        after_input = -(-t_in // DAY) * DAY  # the first day start at or after the input
        if after_input == t_due_end:  # a Month pulse instant is never an M2 timer instant
            after_input += DAY
        return max(t_due_end + DAY, after_input)
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


def default_tfr_bits(spec: dict[str, Any]) -> int:
    """Return the Q16_16 bits of the spec's default tfr."""
    return q16(str(spec["algorithm"]["fertility_f1"]["check_values"]["tfr_default"]))


def check_values(st: Standards, spec: dict[str, Any]) -> bool:
    """Check 1: every check value the spec states equals this reference's."""
    fert = spec["algorithm"]["fertility_f1"]
    part = spec["algorithm"]["partnering_f1"]
    cv = fert["check_values"]
    m = level_bits(st, default_tfr_bits(spec))
    conceptions = [conception_bits(st, m, i) for i in range(BANDS)]
    got: dict[str, Any] = {
        "tfr1_bits": tfr1_bits(st),
        "m_bits_at_default": m,
        "conception_bits_at_default": conceptions,
        "sex_male_bits": sex_male_bits(st),
    }
    want = {k: cv[k] for k in got}
    ok = got == want
    for k, value in got.items():
        if value != want[k]:
            print(f"   {k}: spec {want[k]} != reference {value}")
    nupt_ok = part["check_values"]["nuptiality_bits"] == nuptiality_bits(st)
    if not nupt_ok:
        print(
            f"   nuptiality_bits: spec {part['check_values']['nuptiality_bits']} != "
            f"{nuptiality_bits(st)}"
        )
    print(
        f"   tfr default {cv['tfr_default']}: TFR1 = {float(tfr1_exact(st)):.4f} "
        f"({tfr1_bits(st)} bits), M = {m / Q16:.4f}, "
        f"c = {[round(c / ONE, 5) for c in conceptions]}, P(male) = {sex_male_bits(st) / ONE:.6f}"
    )
    return ok and nupt_ok


def bands_cover(year: int) -> bool:
    """Return whether F0's band formula maps [15 Y, 50 Y) onto 0..6 and nothing else."""
    sampled = {band_of(x, year) for x in range(0, 60 * year, 7 * DAY + 13)}
    ok = sampled == {None, *range(BANDS)}
    ok &= band_of(FIRST_AGE * year, year) == 0 and band_of(LAST_AGE * year - 1, year) == BANDS - 1
    ok &= band_of(FIRST_AGE * year - 1, year) is None and band_of(LAST_AGE * year, year) is None
    return ok and band_of(20 * year, year) == 1 and band_of(20 * year - 1, year) == 0


def check_widths(st: Standards) -> bool:
    """Check 2: widths at the extremes; Prob32 range; monotone; F0's band formula."""
    hi_tfr = level_bits(st, q16(str(TFR_BOUNDS[1])))
    m_hi = elite_level_bits(hi_tfr, q16(str(MULT_BOUNDS[1])))
    num = max(ONE * m_hi * n for n in st.natural).bit_length()
    den = (MONTHS_PER_YEAR * Q16 * THOUSAND).bit_length()
    ok = num <= NUM_BITS_MAX and den <= DEN_BITS_MAX
    levels = [level_bits(st, q16(str(TFR_BOUNDS[0] + Decimal(k) / 10))) for k in range(64)]
    for i in range(BANDS):
        cs = [conception_bits(st, lv, i) for lv in levels]
        ok &= all(0 <= c <= ONE for c in cs) and cs == sorted(cs)
        ok &= conception_bits(st, m_hi, i) >= conception_bits(st, hi_tfr, i)
    worst = max(conception_bits(st, m_hi, i) for i in range(BANDS))
    ok &= bands_cover(YEAR_360) and bands_cover(YEAR_365)
    print(
        f"   widths: F4 numerator {num} bits (<= {NUM_BITS_MAX}), denominator {den} "
        f"(<= {DEN_BITS_MAX}); largest c at tfr 7.5 x 1.6 = {worst / ONE:.4f}; "
        f"F0 bands 0..6 over ages 15..49 on both calendars"
    )
    return ok


def check_rate_identity(st: Standards) -> bool:
    """Check 3: 12 c / (1 + (G + P) c) = M n_i exactly before rounding, at M = 1."""
    ok = True
    for n in st.natural:
        b = Fraction(q16("1.0") * n, Q16 * THOUSAND)  # M = 1 exactly
        c = b / (MONTHS_PER_YEAR - st.closed * b)
        ok &= MONTHS_PER_YEAR * c / (1 + st.closed * c) == b
    print(f"   rate identity: 12 c / (1 + {st.closed} c) = n_i for every band (M = 1)")
    return ok


def check_nuptiality(st: Standards) -> bool:
    """Check 4: mean age at first marriage and the share ever married by 50."""
    s = married_share_by_month(st)
    bits = nuptiality_bits(st)
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


class Chain:
    """The month-by-month chain of one woman under F5 and N3 (tool side; floats, a model).

    States: (married, k) with k the pulses since the conception (0 = susceptible, 1..G-1 pregnant,
    G..closed in the postpartum period, closed = G + P skipped pulses; a birth happens when k
    reaches G). With ``mortality`` the woman and her husband (``gap`` years older) die on the
    rung-0 schedule at ``e0`` on the 360-day calendar, and a widow re-enters the unmarried state
    and marries again at N1's probability; without it the chain is the no-widowhood approximation
    of round 1. The probabilities come from the exact integers; the chain itself is a float model
    of the spec text, not engine arithmetic, and its outputs are bands and reports.
    """

    def __init__(self, st: Standards, m_bits: int, e0: str, *, mortality: bool, gap: int) -> None:
        """Build the chain for level ``m_bits`` and the husband ``gap`` years older."""
        self.st = st
        self.c = [conception_bits(st, m_bits, i) / ONE for i in range(BANDS)]
        self.mu = [b / ONE for b in nuptiality_bits(st)]
        self.lx: list[int] = [int(v) for v in rung0.survivors(rung0.solve_k(q16(e0)))]
        self.mortality = mortality
        self.gap = gap

    def survival(self, age_min: int) -> float:
        """Return l(age) of the rung-0 schedule, or 1 without mortality."""
        if not self.mortality:
            return 1.0
        return int(rung0.ell(self.lx, age_min, YEAR_360)) / ONE

    def husband_death(self, x: int) -> float:
        """Return the husband's probability of dying this month at the woman's age ``x``."""
        if not self.mortality:
            return 0.0
        now = self.survival(x + self.gap * YEAR_360)
        if now == 0:
            return 1.0
        return 1 - self.survival(x + self.gap * YEAR_360 + MONTH_360) / now

    def step(
        self, p: dict[tuple[int, int], float], band: int, q_h: float
    ) -> dict[tuple[int, int], float]:
        """Return the next month's state distribution."""
        closed = self.st.closed
        new: dict[tuple[int, int], float] = {}

        def add(key: tuple[int, int], v: float) -> None:
            new[key] = new.get(key, 0.0) + v

        for (mm, k), v in p.items():
            if v == 0:
                continue
            nk = 0 if k == closed else (k + 1 if k >= 1 else 0)  # pulses 1..closed are skipped
            if mm == 0:
                add((1, nk), v * self.mu[band])
                add((0, nk), v * (1 - self.mu[band]))
                continue
            pc = self.c[band] if k == 0 else 0.0
            for still, w in ((1, 1 - q_h), (0, q_h)):
                if k == 0:
                    add((still, 1), v * w * pc)
                    add((still, 0), v * w * (1 - pc))
                else:
                    add((still, nk), v * w)
        return new

    def run(self) -> tuple[list[float], list[float], list[float]]:
        """Return per-month (births, married share, susceptible share) from age 15 to 49."""
        p: dict[tuple[int, int], float] = {(0, 0): 1.0}
        births, married, susceptible = [], [], []
        x15 = FIRST_AGE * YEAR_360 + MONTH_360 // 2
        for a in range(BANDS * BAND_MONTHS):
            x = x15 + a * MONTH_360
            married.append(sum(v for (mm, _), v in p.items() if mm == 1))
            susceptible.append(p.get((1, 0), 0.0))
            due = sum(v for (_, k), v in p.items() if k == self.st.gestation)
            births.append(due * self.survival(x + MONTH_360 // 2))  # the mother alive at the birth
            p = self.step(p, a // BAND_MONTHS, self.husband_death(x))
        return births, married, susceptible


def nrr_and_mean_age(births: list[float], st: Standards) -> tuple[float, float]:
    """Return (NRR, mean age of net maternity in years) from per-month births."""
    total = sum(births)
    moment = sum(b * (FIRST_AGE + a / MONTHS_PER_YEAR) for a, b in enumerate(births))
    return total * float(st.female_share), moment / total


def nrr_with_widowhood(st: Standards, tfr_bits: int) -> tuple[float, float]:
    """Return (NRR, mean age) of the widowhood chain at e0 = 30 for ``tfr_bits``."""
    chain = Chain(st, level_bits(st, tfr_bits), "30", mortality=True, gap=HUSBAND_OLDER_YEARS)
    return nrr_and_mean_age(chain.run()[0], st)


def check_stationary(st: Standards, spec: dict[str, Any]) -> bool:
    """Check 5: the default tfr's growth with widowhood lies in V-SF-01's band at e0 = 30."""
    tfr_bits = default_tfr_bits(spec)
    nrr, t_mean = nrr_with_widowhood(st, tfr_bits)
    lo = math.exp(float(GROWTH_BAND[0]) * t_mean)
    hi = math.exp(float(GROWTH_BAND[1]) * t_mean)
    ok = lo <= nrr <= hi
    r = math.log(nrr) / t_mean
    no_widow = Chain(st, level_bits(st, tfr_bits), "30", mortality=True, gap=0)
    nrr_no, _ = nrr_and_mean_age(no_widow.run()[0], st)
    guess = tfr_bits / Q16 / nrr  # NRR is almost linear in tfr: a few secant steps
    for _ in range(SECANT_STEPS):
        nrr_g, _ = nrr_with_widowhood(st, round(guess * Q16))
        guess = guess / nrr_g
    print(
        f"   stationary estimate at e0 = 30 with widowhood (husband {HUSBAND_OLDER_YEARS} y older):"
        f" NRR = {nrr:.4f} (band [{lo:.4f}, {hi:.4f}]), r = {100 * r:+.3f}%/yr, "
        f"mean age of net maternity {t_mean:.1f}; NRR = 1 at tfr = {guess:.3f}; "
        f"with a same-age husband: NRR = {nrr_no:.4f}"
    )
    return ok


def check_timers(st: Standards) -> bool:
    """Check 6: birth and marriage days are day starts inside their month; re-samples never drop."""
    ok = True
    due = st.gestation * MONTH_360
    end = due + MONTH_360
    for d in (0, 1, 1 << 63, (1 << 64) - 1):
        tau = birth_day(due, MONTH_360, d)
        ok &= tau % DAY == 0 and due + DAY <= tau <= end - DAY
        mu = marriage_day(0, MONTH_360, d)
        ok &= mu % DAY == 0 and DAY <= mu <= MONTH_360 - DAY
        for t_in in (0, due - 5, due, due + 7, due + 13 * DAY, end - DAY - 1):
            r = resample_birth_day(t_in, due, end, d)
            ok &= r % DAY == 0 and r > t_in and due < r < end
        ok &= resample_birth_day(end - DAY, due, end, d) == end + DAY
        ok &= resample_birth_day(end - 1, due, end, d) == end + DAY
        ok &= resample_birth_day(end, due, end, d) == end + DAY
        ok &= resample_birth_day(end + DAY, due, end, d) == end + DAY  # an input at the moved birth
    ok &= birth_day(due, 28 * DAY, (1 << 64) - 1) == due + 27 * DAY
    print("   timers: births on days 1..n-1 of the due month, marriages on days 1..n-1, no drop")
    return ok


def check_report(st: Standards, spec: dict[str, Any]) -> bool:
    """Check 7: V-SF-24's report values from the no-mortality chain (REV-SPR-015 MAJ-3)."""
    m = level_bits(st, default_tfr_bits(spec))
    births, married, _ = Chain(st, m, "30", mortality=False, gap=0).run()
    ratios = []
    for i in range(BANDS):
        lo, hi = i * BAND_MONTHS, (i + 1) * BAND_MONTHS
        married_years = sum(married[lo:hi]) / MONTHS_PER_YEAR
        ratios.append(sum(births[lo:hi]) / married_years / (m * st.natural[i] / (Q16 * THOUSAND)))
    stated = [float(x) for x in spec["algorithm"]["fertility_f1"]["report_ratios"]]
    ok = all(abs(r - x) <= float(REPORT_TOL) for r, x in zip(ratios, stated, strict=True))
    print(
        "   V-SF-24 report: births per married woman-year / M n(a) by band at birth = "
        + ", ".join(f"{r:.2f}" for r in ratios)
        + f" (spec: {', '.join(f'{x:.2f}' for x in stated)}); the chain draws every conception "
        "at c_i, the identity V-SF-24 tests"
    )
    return ok


def main() -> int:
    """Run every check and print PASS or FAIL."""
    spec = load_spec(spec_path(sys.argv))
    st = Standards(spec)
    checks: list[tuple[str, Callable[[], bool]]] = [
        ("1. spec check values", lambda: check_values(st, spec)),
        ("2. widths, monotonicity and bands", lambda: check_widths(st)),
        ("3. rate identity", lambda: check_rate_identity(st)),
        ("4. nuptiality shape", lambda: check_nuptiality(st)),
        ("5. stationary estimate with widowhood", lambda: check_stationary(st, spec)),
        ("6. timers", lambda: check_timers(st)),
        ("7. V-SF-24 report values", lambda: check_report(st, spec)),
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
