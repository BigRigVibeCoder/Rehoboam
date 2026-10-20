"""The population projection of SPR-030's fixture worlds with the house-member mechanics.

REV-SPR-030 R1: the trajectory of `twin_vales` and `kingdom_10k` over their horizons cannot be
read from the commoner NRR alone, because three mechanics the manifests set act on great-house
members and membership spreads through the population:

  * M7 `on_birth`: a newborn's house_member row, by the membership rule (BCK-002 A-56; the three
    rules below);
  * M2 F3/F4/F6: `elite_fertility_mult` (1.2 here) multiplies the fertility level of every woman
    with a house_member row and, under D56 (e) (the read on the couple, the M2 0.3.0 draft), of
    every woman whose husband has one;
  * M7 `vocation`: a member reaching majority with no seat, office, claim or first place in a
    line draws holy orders with probability `vocation_holy_orders` (0.2 here), which removes the
    member from marriage and conception (M2 N3/N4/F5).

This is a two-type generational model (the reviewer's sketch, `elite_growth.py`, restated):
women of two types, commoner (c) and member (e); each generation of T years (the mean age of net
maternity of the M2 F1 widowhood chain, ``specs/demography/selftest_m2_f1.py`` check 5) a
commoner woman has NRR_c daughters and a member woman not in holy orders NRR_e = NRR at the elite
level; a commoner woman's husband is a member with the share h of members among marriageable men
(members in holy orders excluded). Which children are members is the **membership rule** of M7's
`on_birth` (BCK-002 A-56; `specs/systems/next/M7_membership.md`):

    births of member mothers   B_e = W_e (1 - v) NRR_e
    births of commoner mothers B_c = W_c NRR_c
    cognatic (M7 0.3.4):  W_e' = B_e + h B_c              the father's house, else the mother's
    agnatic (M7 0.5.0):   W_e' = h (B_e + B_c)            the father's house, else none
    bounded (PR #79's first draft): W_e' = h (B_e + B_c)  the father's house, else the mother's
                              + (1 - h) min(W_e, 2 H W_n) (1 - v) NRR_e
                                                          only when she is named-eligible
    W_c' = (everyone born) - W_e'       with h = m (1 - v) / (m (1 - v) + 1 - m)

where m is the member share, v = vocation_holy_orders, H the number of houses and W_n the
named member women of childbearing age per house, whose children by a commoner join under the
bounded rule (a member mother within `dormancy_degrees` of a holder of her house, or first or
second in a line, or holding a seat, office or claim; the member fathers' children are in h's
term already). W_n is measured at genesis by the fixture reference (`selftest_fixture_worlds.py`,
the full reading's named women aged 15-49, per house: twin_vales 4, 2 and 3 over five houses at
seeds 1-3, kingdom_10k 11 over twelve houses at seed 1; ``--derive-caps`` recomputes them) and
pinned below with a doubled sensitivity, since a mid-reign holder is younger than the genesis
holders and has more daughters of that age. The population is proportional to W_e + W_c (a
constant sex ratio; the W's count persons). The capped case (`kingdom_10k` with
`density_capacity` K and slope 1, M2 F1 MR11) multiplies every hazard by
m_d = min(4, 1 + max(0, N / K - 1)), which under proportional hazards turns survival to age T
into l(T)^m_d, so each NRR is scaled by l(T)^(m_d - 1), with l from the same chain's schedule
(rung 0 at e0 30).

Limits (stated, not hidden): generations are discrete (no age structure, no overlap); mating is
random (M2 N4 bounds husbands to the settlement and excludes the named; both make h smaller for
commoner women far from a seat); holy orders is applied as a removal of the share v of member
women at majority and of member men from the marriage pool (0.2 under the frozen reading; under
the later-child reading v is derived per mother from the chain, so it ignores a father's
children by other wives and splits a widower's families by mother: an independent Monte Carlo
split by husband gives v = 0.111 / 0.122 against 0.116 / 0.128, and remarriage pushes the other
way; seat holders and the first and second in a line, a handful per house, are left out); the
back-fill's age structure at genesis and widowhood beyond the chain's are ignored; the capped
case's NRR scaling is the proportional-hazards approximation at one age; W_n is a genesis
cross-section held constant. The model says what the mechanics imply to within tens of per cent,
not a golden; the engine's run reports the truth.

The baseline reading is the configuration the director adopted in D56 (2026-10-17): the T1
regime, agnatic membership, vocations for later children (d) and the elite read on the couple
(e). The golden renders it first, as ``baseline`` blocks and lines; every other configuration (the
frozen readings under each rule and regime, each lever alone or combined, hypergamy) is rendered
as an ``alternative``, and the reviewer's variants as ``sensitivity`` lines.

Run: python -I specs/worldgen/project_population.py            (print the tables)
     python -I specs/worldgen/project_population.py --check    (compare with the golden file)
     python -I specs/worldgen/project_population.py --write    (regenerate the golden file)
     python -I specs/worldgen/project_population.py --derive-caps   (W_n from the fixture reference)
Golden: specs/worldgen/golden/population_projection_v0.txt. Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import importlib.util
import itertools
import math
import sys
from dataclasses import dataclass, replace
from pathlib import Path
from types import ModuleType
from typing import Protocol, TypedDict

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GOLDEN = HERE / "golden" / "population_projection_v0.txt"


def _load(name: str, path: Path) -> ModuleType:
    """Load a reference by path (python -I adds no script directory to sys.path)."""
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SystemExit(f"{path} not found")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


m2f1 = _load("selftest_m2_f1", ROOT / "specs" / "demography" / "selftest_m2_f1.py")

E0 = "30"
TFR = "4.9"
HUSBAND_GAP = int(m2f1.HUSBAND_OLDER_YEARS)
YEAR_360 = int(m2f1.YEAR_360)
MODIFIER_CAP = 4.0  # MR11: the density modifier's ceiling
CHECKPOINTS = (0, 50, 100, 150, 200, 300, 400, 500, 750, 1000)
MEMBER_MILESTONES = (0.5, 0.9, 0.98)


# BCK-002 A-56: M7 0.3.4's rule, the back-fill's (M7 0.5.0's), and PR #79's first draft's
RULES = ("cognatic", "agnatic", "bounded")
# REV of PR #79 R1: the regime decides who marries whom. "isolation": M7's isolation run, T1
# stubbed, nobody named, everyone on M2's market (N3/N4: random within the settlement). "t1": T1
# registered, as both fixture worlds are: the named core (M7's criteria; a bounded set) marries
# within itself through T1's marry decision (T1 0.3.3: unmarried named adults in the actor's court
# or relation set) and M2's N3/N4 exclude the named; the un-named members, beyond the degrees,
# marry on M2's market.
REGIMES = ("isolation", "t1")
# the vocation reading (R4): "all" draws holy orders for every member at majority (the frozen
# submodel's "not first in any title's line"), "later" for later children only (the first-born
# exempt: the frozen [[params]] meaning, the frozen schedule step and RT-2026-10-16 item 12), the
# primary reading
VOCATION_READINGS = ("all", "later", "literal")


@dataclass(frozen=True)
class World:
    """A world's genesis counts (the fixture-worlds golden at seed 1), parameters and levers."""

    id: str
    population: int  # living at tick 0
    members: int  # living great-house members at tick 0
    houses: int  # great houses (H)
    named_women: float  # W_n: named member women aged 15-49 per house at genesis (--derive-caps)
    horizon: int  # years
    mult: str  # elite_fertility_mult
    holy: float  # vocation_holy_orders
    capacity: int | None  # density_capacity, None for the default (never binds)
    # the defaults are the baseline reading, D56's (BASELINE below)
    rule: str = "agnatic"  # the membership rule of M7's on_birth (RULES)
    regime: str = "t1"  # REGIMES
    vocation: str = "later"  # VOCATION_READINGS
    elite_read: str = (
        "couple"  # M2 F3/F4/F6's elite read: "couple" (either spouse's row; lever e, D56), "woman"
    )
    hypergamy: float = (
        0.0  # share of the named core marrying unnamed persons of the polity (lever f)
    )

    def label(self) -> str:
        """The levers in force, for a golden line."""
        return (
            f"regime {self.regime} rule {self.rule} vocation {self.vocation} "
            f"elite_read {self.elite_read} hypergamy {self.hypergamy}"
        )

    def reading(self) -> str:
        """``baseline`` for D56's adopted configuration, else ``alternative``."""
        same = all(getattr(self, key) == value for key, value in BASELINE.items())
        return "baseline" if same else "alternative"


class Reading(TypedDict, total=False):
    """A configuration's fields of World, as keyword changes for ``replace``."""

    regime: str
    rule: str
    vocation: str
    elite_read: str
    hypergamy: float


# the baseline reading: the configuration D56 adopted (the director, 2026-10-17), T1 registered,
# agnatic membership, (d) vocations for later children, (e) the elite read on the couple
BASELINE: Reading = {
    "regime": "t1",
    "rule": "agnatic",
    "vocation": "later",
    "elite_read": "couple",
    "hypergamy": 0.0,
}
# the frozen texts' readings (every member draws holy orders, the elite read on the woman's row,
# no hypergamy): the alternatives' blocks and the levers start from them
FROZEN_READINGS: Reading = {"vocation": "all", "elite_read": "woman", "hypergamy": 0.0}
# W_n: the fixture reference's named member women aged 15-49 at genesis (the full reading), per
# house: twin_vales 4 / 5 houses at seed 1 (2 and 3 at seeds 2 and 3), kingdom_10k 11 / 12 at
# seed 1; members: the living house members at genesis, seed 1 (golden/fixture_worlds_v0.txt)
TWIN_VALES = World("twin_vales", 1500, 53, 5, 4 / 5, 300, "1.2", 0.2, None)
KINGDOM_10K = World("kingdom_10k", 10000, 185, 12, 11 / 12, 1000, "1.2", 0.2, None)
WORLDS = (TWIN_VALES, KINGDOM_10K, replace(KINGDOM_10K, capacity=10000))
# the reviewer's variants, uncapped, under the cognatic rule in isolation
SENSITIVITY = (("1.0", 0.2), ("1.2", 0.0), ("1.0", 0.0))
# the bounded rule with W_n doubled (mid-reign holders are younger than the genesis cross-section)
CAP_SENSITIVITY = 2.0
HYPERGAMY = 0.5  # lever f: half of the named core's marriages reach outside it
# the levers compared under the T1 regime for the agnatic and the bounded rule (R1): (d)
# later-children vocations, (e) the elite read on the couple, (f) hypergamy, (g) their
# combinations
LEVERS: tuple[tuple[str, Reading], ...] = (
    ("d", {"vocation": "later"}),
    ("e", {"elite_read": "couple"}),
    ("f", {"hypergamy": HYPERGAMY}),
    ("d+e", {"vocation": "later", "elite_read": "couple"}),
    ("d+f", {"vocation": "later", "hypergamy": HYPERGAMY}),
    ("d+e+f", {"vocation": "later", "elite_read": "couple", "hypergamy": HYPERGAMY}),
)


@dataclass
class Rates:
    """The chain's NRRs, the generation length, survival to it, and the holy-orders shares."""

    nrr_c: float
    nrr_e: float
    t_years: float
    l_t: float
    births_c: float = 0.0  # E[B]: births per woman born, commoner level (the counter chain)
    births_e: float = 0.0  # and at the elite level
    mothers_c: float = 0.0  # P(B >= 1), commoner level
    mothers_e: float = 0.0
    v_later_c: float = 0.0  # holy orders' share under the ruled reading (N2), commoner mothers
    v_later_e: float = 0.0  # and member mothers
    v_literal_c: float = 0.0  # under the literal reading (the eldest by birth tick, dead included)
    v_literal_e: float = 0.0


class ChainLike(Protocol):
    """What the extensions below need of ``selftest_m2_f1.Chain`` (loaded by path, so untyped)."""

    st: object

    def survival(self, age_min: int) -> float:
        """l(age) of the rung-0 schedule."""
        ...

    def husband_death(self, x: int) -> float:
        """The husband's probability of dying this month at the woman's age x."""
        ...

    def step(
        self, p: dict[tuple[int, int], float], band: int, q_h: float
    ) -> dict[tuple[int, int], float]:
        """The next month's state distribution."""
        ...


def gestation(chain: ChainLike) -> int:
    """The chain's gestation pulse count (the Standards' G)."""
    return int(getattr(chain.st, "gestation"))  # noqa: B009  (st is a Standards, typed object here)


BIRTHS_MAX = 12  # the counter chain's cap on births per woman (the mass beyond it is negligible)
MAJORITY_MONTHS = 16 * 12  # majority_age 16: the vocation draw's age, in months of the 360-day year
GAP_MAX = 40 * 12  # an inter-birth gap longer than this cannot happen inside the fertile span


def counter_chain(chain: ChainLike) -> list[list[float]]:
    """The widowhood chain with a birth counter.

    Returns births_by_order[j][a], the mass of a woman's j-th birth at month a of her fertile
    span (j from 1), weighted by her survival as the chain weights every birth (the mother alive
    at the birth).
    """
    x15 = m2f1.FIRST_AGE * m2f1.YEAR_360 + m2f1.MONTH_360 // 2
    p: dict[tuple[int, int, int], float] = {(0, 0, 0): 1.0}
    by_order = [[0.0] * (m2f1.BANDS * m2f1.BAND_MONTHS) for _ in range(BIRTHS_MAX + 2)]
    for a in range(m2f1.BANDS * m2f1.BAND_MONTHS):
        x = x15 + a * m2f1.MONTH_360
        alive = chain.survival(x + m2f1.MONTH_360 // 2)
        counted: dict[tuple[int, int, int], float] = {}
        for (mm, k, b), v in p.items():  # a state at k = G is a birth this month: the counter moves
            if k == gestation(chain) and b < BIRTHS_MAX:
                by_order[b + 1][a] += v * alive
                counted[(mm, k, b + 1)] = counted.get((mm, k, b + 1), 0.0) + v
            else:
                counted[(mm, k, b)] = counted.get((mm, k, b), 0.0) + v
        q_h = chain.husband_death(x)
        nxt: dict[tuple[int, int, int], float] = {}
        for b in range(BIRTHS_MAX + 1):
            sub = {(mm, k): v for (mm, k, bb), v in counted.items() if bb == b}
            if sub:
                for (mm, k), v in chain.step(sub, a // m2f1.BAND_MONTHS, q_h).items():
                    nxt[(mm, k, b)] = nxt.get((mm, k, b), 0.0) + v
        p = nxt
    return by_order


def gap_distribution(chain: ChainLike, by_order: list[list[float]]) -> list[float]:
    """The inter-birth gap in months, g[d].

    From the state just after a birth (married, at the gestation pulse) at each birth age, the
    next birth's month, weighted by the mother's survival across the gap and mixed over the ages
    at which births happen.
    """
    x15 = m2f1.FIRST_AGE * m2f1.YEAR_360 + m2f1.MONTH_360 // 2
    span = m2f1.BANDS * m2f1.BAND_MONTHS
    g = [0.0] * (GAP_MAX + 1)
    # one start per five-year band, at its middle month, weighted by the band's births (a start
    # per month gives the same shares to the third decimal at sixty times the cost)
    weights = {
        band * m2f1.BAND_MONTHS + m2f1.BAND_MONTHS // 2: sum(
            by_order[j][a]
            for j in range(1, BIRTHS_MAX + 1)
            for a in range(band * m2f1.BAND_MONTHS, (band + 1) * m2f1.BAND_MONTHS)
        )
        for band in range(m2f1.BANDS)
    }
    for a0, w0 in weights.items():
        if w0 == 0:
            continue
        x0 = x15 + a0 * m2f1.MONTH_360
        s0 = chain.survival(x0 + m2f1.MONTH_360 // 2)
        p: dict[tuple[int, int], float] = {(1, gestation(chain)): 1.0}
        for a in range(a0, span):
            x = x15 + a * m2f1.MONTH_360
            p = chain.step(p, a // m2f1.BAND_MONTHS, chain.husband_death(x))
            d = a + 1 - a0
            due = sum(v for (_, k), v in p.items() if k == gestation(chain))
            if due and d <= GAP_MAX:
                survived = chain.survival(x + m2f1.MONTH_360 + m2f1.MONTH_360 // 2) / s0
                g[d] += w0 * due * survived
            p = {key: v for key, v in p.items() if key[1] != gestation(chain)}  # absorb
    total = sum(g)
    return [v / total for v in g] if total else g


def exempt_share(chain: ChainLike, by_order: list[list[float]], g: list[float]) -> float:
    """The share of births exempt under the ruled reading.

    The j-th child is exempt when every elder sibling is dead at its majority. With the gaps
    between consecutive births drawn from g (independent, the approximation), the elder sibling
    i steps back is alive at the child's majority with probability l(majority + the cumulative
    gap), and the expectation of the product over the elder siblings is a convolution over the
    cumulative gaps.
    """
    span = m2f1.BANDS * m2f1.BAND_MONTHS
    orders = [sum(by_order[j][a] for a in range(span)) for j in range(BIRTHS_MAX + 1)]
    total = sum(orders[1:])
    majority = MAJORITY_MONTHS * m2f1.MONTH_360
    l_at = [chain.survival(majority + d * m2f1.MONTH_360) for d in range(GAP_MAX * 2 + 1)]
    # f[s]: the mass of cumulative gap s with the product of (1 - l) over the elder siblings so far
    f = [1.0] + [0.0] * (GAP_MAX * 2)
    exempt = orders[1]
    for j in range(2, BIRTHS_MAX + 1):
        nf = [0.0] * len(f)
        for s0, m in enumerate(f):
            if m == 0:
                continue
            for d, pg in enumerate(g):
                if pg == 0 or s0 + d >= len(nf):
                    continue
                nf[s0 + d] += m * pg * (1 - l_at[s0 + d])
        f = nf
        exempt += orders[j] * sum(f)
    return exempt / total


def vocation_shares(chain: ChainLike, holy: float) -> tuple[float, float, float, float]:
    """(E[B], P(B >= 1), v under the ruled reading, v under the literal reading) for one chain."""
    by_order = counter_chain(chain)
    span = m2f1.BANDS * m2f1.BAND_MONTHS
    births = sum(by_order[j][a] for j in range(1, BIRTHS_MAX + 1) for a in range(span))
    mothers = sum(by_order[1])
    g = gap_distribution(chain, by_order)
    v_later = holy * (1 - exempt_share(chain, by_order, g))
    v_literal = holy * (1 - mothers / births)
    return births, mothers, v_later, v_literal


def rates(mult: str, holy: float = 0.2) -> Rates:
    """NRR of commoner and member women, the mean age of net maternity, l(T) and the shares."""
    spec_path = m2f1.NEXT_SPEC if m2f1.NEXT_SPEC.exists() else m2f1.MAIN_SPEC
    st = m2f1.Standards(m2f1.load_spec(spec_path))
    tfr_bits = m2f1.q16(TFR)
    level = m2f1.level_bits(st, tfr_bits)
    nrr_c, t_years = m2f1.nrr_with_widowhood(st, tfr_bits)
    elite = m2f1.elite_level_bits(level, m2f1.q16(mult))
    chain_c = m2f1.Chain(st, level, E0, mortality=True, gap=HUSBAND_GAP)
    chain = m2f1.Chain(st, elite, E0, mortality=True, gap=HUSBAND_GAP)
    nrr_e, _ = m2f1.nrr_and_mean_age(chain.run()[0], st)
    l_t = chain.survival(round(t_years * YEAR_360))
    b_c, m_c, vl_c, vlit_c = vocation_shares(chain_c, holy)
    b_e, m_e, vl_e, vlit_e = vocation_shares(chain, holy)
    return Rates(nrr_c, nrr_e, t_years, l_t, b_c, b_e, m_c, m_e, vl_c, vl_e, vlit_c, vlit_e)


def holy_share(w: World, r: Rates, mothers: str) -> float:
    """The share a member cohort born of ``mothers`` loses to holy orders at majority.

    By the vocation reading: "all" (the frozen submodel: every member not first in a line
    draws, v = vocation_holy_orders), "later" (the ruled reading, N2 of PR #79's round 2: a
    later child is one with an elder child of the same father alive at the draw, so the
    father's eldest living child is exempt; a child first or second in a line is exempt too, a
    handful the model ignores), "literal" (the eldest by birth tick, dead included: the
    first-born only). ``mothers`` is "commoner" or "member", the chain the cohort was born of.
    """
    if w.vocation == "all":
        return w.holy
    if w.vocation == "later":
        return r.v_later_c if mothers == "commoner" else r.v_later_e
    return r.v_literal_c if mothers == "commoner" else r.v_literal_e


@dataclass
class Point:
    """The population, the member share and the member count at one year."""

    year: float
    n: float
    share: float
    members: float


def step(w: World, r: Rates, w_e: float, w_c: float, s_c: float) -> tuple[float, float, float]:
    """One generation under the world's regime, rule and levers (the docstring's recursion).

    Counts are persons with a constant sex ratio: a group of P persons bears P x NRR persons in a
    generation (its P/2 women, 2 NRR children each). ``s_c`` is the share of the members born of
    commoner mothers, whose holy-orders share is the commoner chain's; the rest is the member
    chain's. Returns the members, the commoners and the next ``s_c``.
    """
    n = w_e + w_c
    m_d = 1.0
    if w.capacity is not None and n > w.capacity:
        m_d = min(MODIFIER_CAP, 1.0 + (n / w.capacity - 1.0))
    s = r.l_t ** (m_d - 1.0)
    v_e, v_c = holy_share(w, r, "member"), holy_share(w, r, "commoner")
    v = (1 - s_c) * v_e + s_c * v_c  # the members' own share, by their mothers
    couple = w.elite_read == "couple"
    nrr_w = r.nrr_e if couple else r.nrr_c  # a member man's commoner wife
    if w.regime == "isolation":
        core, u = 0.0, w_e  # nobody is named: every member is on M2's market
    else:
        core = min(w_e, 2 * w.houses * w.named_women)  # the named core, persons
        u = w_e - core  # the un-named members, on M2's market
    h = u * (1 - v) / (u * (1 - v) + w_c) if u + w_c > 0 else 0.0  # member share, M2's husbands
    q = w.hypergamy if w.regime == "t1" else 0.0
    # births, in persons: the core within itself, the core's women and men marrying out, the
    # un-named member women (M2 husbands), the commoner women (member husbands with share h)
    b_core = core * (1 - q) * (1 - v) * r.nrr_e * s
    b_core_women_out = (core / 2) * q * (1 - v) * r.nrr_e * s
    b_core_men_out = (core / 2) * q * (1 - v) * nrr_w * s
    b_u = u * (1 - v) * r.nrr_e * s
    b_c = w_c * ((1 - h) * r.nrr_c + h * nrr_w) * s
    b_c_member_father = w_c * h * nrr_w * s
    births = b_core + b_core_women_out + b_core_men_out + b_u + b_c
    of_commoner_mothers = b_core_men_out + b_c_member_father  # member children of commoner wives
    if w.rule == "cognatic":
        members = b_core + b_core_women_out + b_core_men_out + b_u + b_c_member_father
    elif w.rule == "agnatic":
        members = b_core + b_core_men_out + h * b_u + b_c_member_father
    elif w.rule == "bounded":
        # the mother branch: a named-eligible woman and a houseless father. In isolation the named
        # women (M7's own criteria, not T1's column) bear on M2's market; under T1 they marry
        # within the core, so the branch fires only for a named woman married out (hypergamy) or
        # for one who became eligible after an M2 marriage, which the model puts at zero
        if w.regime == "isolation":
            named = min(w_e, 2 * w.houses * w.named_women)
            extra = (1 - h) * named * (1 - v) * r.nrr_e * s
        else:
            extra = b_core_women_out
        members = b_core + b_core_men_out + h * b_u + b_c_member_father + extra
    else:
        raise SystemExit(f"unknown membership rule {w.rule!r}")
    # under the couple read a commoner wife bears at the elite level, so her children carry the
    # member chain's sibling structure
    next_s_c = 0.0 if couple or members <= 0 else of_commoner_mothers / members
    return members, births - members, next_s_c


def project(w: World, r: Rates) -> list[Point]:
    """The generational recursion over the horizon, one point per generation."""
    w_e, w_c, s_c = float(w.members), float(w.population - w.members), 0.0
    points = [Point(0.0, w.population, w.members / w.population, w.members)]
    year = 0.0
    while year < w.horizon:
        w_e, w_c, s_c = step(w, r, w_e, w_c, s_c)
        year += r.t_years
        points.append(Point(year, w_e + w_c, w_e / (w_e + w_c), w_e))
    return points


def at(points: list[Point], year: float) -> Point:
    """The point at ``year`` by geometric interpolation between generations."""
    for p0, p1 in itertools.pairwise(points):
        if p0.year <= year <= p1.year:
            f = (year - p0.year) / (p1.year - p0.year)
            n = p0.n * (p1.n / p0.n) ** f
            if p0.members > 0 and p1.members > 0:
                members = p0.members * (p1.members / p0.members) ** f
            else:
                members = p0.members + (p1.members - p0.members) * f
            return Point(year, n, members / n, members)
    return points[-1]


def summary(w: World, r: Rates) -> str:
    """One line: the horizon's population and members, the least members, the peak population."""
    points = project(w, r)
    inside = [*(p for p in points if p.year <= w.horizon), at(points, w.horizon)]
    final = inside[-1]
    least = min(inside, key=lambda p: p.members)
    peak = max(inside, key=lambda p: p.n)
    cap = "default" if w.capacity is None else str(w.capacity)
    return (
        f"line {w.id} capacity {cap} {w.label()}: year {w.horizon} population {final.n:.0f} "
        f"members {final.members:.0f} ({final.members / w.houses:.1f} per house) "
        f"share {final.share:.3f} least {least.members:.0f} at year {least.year:.0f} "
        f"peak {peak.n:.0f} at year {peak.year:.0f}"
    )


def render_world(w: World, r: Rates) -> list[str]:
    """The golden block of one world under one regime and rule."""
    points = project(w, r)
    cap = "default" if w.capacity is None else str(w.capacity)
    lines = [
        f"{w.reading()} world {w.id} capacity {cap} horizon {w.horizon} mult {w.mult} "
        f"holy {w.holy} houses {w.houses} named_women_per_house {w.named_women:.2f} {w.label()}"
    ]
    for year in CHECKPOINTS:
        if year <= w.horizon:
            p = at(points, year)
            lines.append(
                f"year {year} population {p.n:.0f} member_share {p.share:.3f} "
                f"members {p.members:.0f}"
            )
    inside = [*(p for p in points if p.year <= w.horizon), at(points, w.horizon)]
    peak = max(inside, key=lambda p: p.n)
    years = range(w.horizon + 1)
    mean = sum(at(points, y).n for y in years) / len(years)
    lines.append(f"peak population {peak.n:.0f} at year {peak.year:.0f} mean {mean:.0f}")
    least = min(inside, key=lambda p: p.members)
    lines.append(
        f"least members {least.members:.0f} at year {least.year:.0f} "
        f"({least.members / w.houses:.1f} per house)"
    )
    for level in MEMBER_MILESTONES:
        hit = next((p for p in points if p.share >= level), None)
        when = "never" if hit is None else f"{hit.year:.0f}"
        lines.append(f"member_share_reaches {level} year {when}")
    return lines


def lever_lines(base: Rates) -> list[str]:
    """The levers (R1): one line each for the agnatic and the bounded rule, under both regimes.

    Each lever is counted from the frozen readings; the line whose configuration is D56's is
    labelled ``baseline`` (the T1 regime, the agnatic rule, d+e), every other ``alternative``.
    """
    out = []
    for regime in REGIMES:
        for rule in ("agnatic", "bounded"):
            for name, levers in LEVERS:
                if regime == "isolation" and "hypergamy" in levers:
                    continue  # nobody is named in isolation: the lever has no subject
                reading: Reading = {**FROZEN_READINGS, **levers, "rule": rule, "regime": regime}
                for w in WORLDS:
                    world = replace(w, **reading)
                    out.append(f"{world.reading()} lever {name} " + summary(world, base))
    # the cognatic rule under lever d (R4: the knife edge does not survive the later-children
    # reading)
    for regime in REGIMES:
        cognatic: Reading = {
            **FROZEN_READINGS,
            "vocation": "later",
            "rule": "cognatic",
            "regime": regime,
        }
        for w in WORLDS:
            world = replace(w, **cognatic)
            out.append(f"{world.reading()} lever d " + summary(world, base))
    # the literal reading (the eldest by birth tick, dead included) as a sensitivity (N2): the
    # agnatic rule under T1 with (d) and with (d)+(e)
    for read in ("woman", "couple"):
        literal: Reading = {**BASELINE, "vocation": "literal", "elite_read": read}
        for w in WORLDS:
            world = replace(w, **literal)
            out.append("sensitivity literal-reading " + summary(world, base))
    return out


def sensitivity_lines(base: Rates) -> list[str]:
    """The reviewer's cognatic variants in isolation, and the bounded rule with W_n doubled."""
    out = []
    cognatic: Reading = {**FROZEN_READINGS, "rule": "cognatic", "regime": "isolation"}
    for mult, holy in SENSITIVITY:
        r = rates(mult)
        w = replace(KINGDOM_10K, mult=mult, holy=holy, **cognatic)
        p = project(w, r)
        final = at(p, w.horizon)
        peak = max([*(q for q in p if q.year <= w.horizon), final], key=lambda q: q.n)
        out.append(
            f"sensitivity regime isolation rule cognatic mult {mult} holy {holy} "
            f"nrr_e {r.nrr_e:.4f} final population {final.n:.0f} member_share {final.share:.3f} "
            f"peak {peak.n:.0f} at year {peak.year:.0f}"
        )
    for regime in REGIMES:
        bounded: Reading = {**FROZEN_READINGS, "rule": "bounded", "regime": regime}
        for base_world in (TWIN_VALES, KINGDOM_10K):
            w_n = base_world.named_women * CAP_SENSITIVITY
            w = replace(base_world, named_women=w_n, **bounded)
            final = at(project(w, base), w.horizon)
            out.append(
                f"sensitivity regime {regime} rule bounded named_women_per_house "
                f"{w.named_women:.2f} world {w.id} final population {final.n:.0f} "
                f"member_share {final.share:.3f} members {final.members:.0f}"
            )
    return out


def render() -> str:
    """The whole golden text."""
    base = rates("1.2")
    lines = [
        "# population projection of the SPR-030 fixture worlds (REV-SPR-030 R1) under the three",
        "# membership rules of BCK-002 A-56, in M7's isolation regime and the T1-registered",
        "# regime, with the levers of PR #79's review R1 (specs/systems/next/M7_membership.md).",
        "# baseline: D56's adopted reading (T1, agnatic, (d) later children, (e) the elite read",
        "# on the couple); alternative: every other configuration; sensitivity: the variants.",
        "# GENERATED by python -I specs/worldgen/project_population.py --write; do not edit.",
        "constants population-projection-v3",
        f"rates mult 1.2 nrr_c {base.nrr_c:.4f} nrr_e {base.nrr_e:.4f} "
        f"r_c {100 * math.log(base.nrr_c) / base.t_years:+.3f}%/yr "
        f"generation_years {base.t_years:.2f} l_T {base.l_t:.4f}",
        f"vocation holy {TWIN_VALES.holy} births_per_woman commoner {base.births_c:.3f} "
        f"member {base.births_e:.3f} mothers commoner {base.mothers_c:.3f} "
        f"member {base.mothers_e:.3f} holy_later commoner {base.v_later_c:.4f} "
        f"member {base.v_later_e:.4f} holy_literal commoner {base.v_literal_c:.4f} "
        f"member {base.v_literal_e:.4f}",
    ]
    # the baseline blocks: D56's reading on every world
    for w in WORLDS:
        lines += render_world(replace(w, **BASELINE), base)
    # the alternatives' blocks: every regime and rule on every world, at the frozen readings
    for regime in REGIMES:
        for rule in RULES:
            frozen: Reading = {**FROZEN_READINGS, "rule": rule, "regime": regime}
            for w in WORLDS:
                lines += render_world(replace(w, **frozen), base)
    lines += lever_lines(base)
    lines += sensitivity_lines(base)
    return "\n".join(lines) + "\n"


def derive_caps() -> str:
    """W_n from the fixture reference: the full reading's named member women aged 15-49."""
    fw = _load("selftest_fixture_worlds", HERE / "selftest_fixture_worlds.py")
    ps = fw.ps
    out = []
    for m, seed in fw.cases("pr"):
        g = ps.build(replace(m, seed=seed))
        gen = ps.Genealogy(g.persons, 0)
        y = g.cal.year
        named = 0
        for h in g.houses:
            x = fw.house_report(g, gen, h, fw.DORMANCY_DEGREES)
            named += sum(
                1
                for pid in x.full
                if g.persons[pid].sex == ps.SEX_FEMALE
                and fw.CHILD_MAX <= -g.persons[pid].birth // y < fw.FERTILE_END
            )
        out.append(
            f"{m.id} seed {seed}: named member women aged 15-49: {named} over "
            f"{len(g.houses)} houses = {named / len(g.houses):.2f} per house"
        )
    return "\n".join(out) + "\n"


def main(argv: list[str] | None = None) -> int:
    """Print, check or write the golden file."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the golden file")
    parser.add_argument("--write", action="store_true", help="regenerate the golden file")
    parser.add_argument("--derive-caps", action="store_true", help="W_n from the fixture reference")
    args = parser.parse_args(argv)
    if args.derive_caps:
        print(derive_caps(), end="")
        return 0
    text = render()
    if args.write:
        GOLDEN.write_text(text, encoding="utf-8")
        print(f"wrote {GOLDEN}")
        return 0
    if args.check:
        current = GOLDEN.read_text(encoding="utf-8") if GOLDEN.exists() else ""
        if current != text:
            print(text, end="")
            print(f"STALE: {GOLDEN} differs from the projection")
            print("POPULATION PROJECTION: FAIL")
            return 1
        cases = len(WORLDS) * (1 + len(RULES) * len(REGIMES))
        print(f"POPULATION PROJECTION: PASS ({cases} cases, {text.count(chr(10))} lines)")
        return 0
    print(text, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
