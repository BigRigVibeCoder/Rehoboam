"""The population projection of SPR-030's fixture worlds with the house-member mechanics.

REV-SPR-030 R1: the trajectory of `twin_vales` and `kingdom_10k` over their horizons cannot be
read from the commoner NRR alone, because three mechanics the manifests set act on great-house
members and membership spreads through the population:

  * M7 `on_birth`: a newborn's house_member row, by the membership rule (BCK-002 A-56; the three
    rules below);
  * M2 F3/F4: `elite_fertility_mult` (1.2 here) multiplies the fertility level of every woman
    with a house_member row;
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
    agnatic:              W_e' = h (B_e + B_c)            the father's house, else none
    bounded (M7 0.5.0):   W_e' = h (B_e + B_c)            the father's house, else the mother's
                              + (1 - h) min(W_e, 2 H W_n) (1 - v) NRR_e
                                                          only when she is named-eligible
    W_c' = (everyone born) - W_e'       with h = m (1 - v) / (m (1 - v) + 1 - m)

where m is the member share, v = vocation_holy_orders, H the number of houses and W_n the
named member women of childbearing age per house, whose children by a commoner join under the
bounded rule (a member mother within `dormancy_degrees` of a holder of her house, or first or
second in a line, or holding a seat, office or claim; the member fathers' children are in h's
term already). W_n is measured at genesis by the fixture reference (`selftest_fixture_worlds.py`,
the full reading's named women aged 15-49, per house: twin_vales 5, 5 and 4 over five houses at
seeds 1-3, kingdom_10k 8 over twelve houses at seed 1; ``--derive-caps`` recomputes them) and
pinned below with a doubled sensitivity, since a mid-reign holder is younger than the genesis
holders and has more daughters of that age. The population is proportional to W_e + W_c (a
constant sex ratio; the W's count persons). The capped case (`kingdom_10k` with
`density_capacity` K and slope 1, M2 F1 MR11) multiplies every hazard by
m_d = min(4, 1 + max(0, N / K - 1)), which under proportional hazards turns survival to age T
into l(T)^m_d, so each NRR is scaled by l(T)^(m_d - 1), with l from the same chain's schedule
(rung 0 at e0 30).

Limits (stated, not hidden): generations are discrete (no age structure, no overlap); mating is
random (M2 N4 bounds husbands to the settlement and excludes the named; both make h smaller for
commoner women far from a seat); holy orders is applied as a 20% removal of member women at
majority and of member men from the marriage pool (the vocation draw excludes the first in a
line and seat holders, a handful per house, which under the agnatic and bounded rules slows the
decline of a small house's male line a little); the back-fill's age structure at genesis and
widowhood beyond the chain's are ignored; the capped case's NRR scaling is the
proportional-hazards approximation at one age; W_n is a genesis cross-section held constant.
The model says what the mechanics imply to within tens of per cent, not a golden; the engine's
run reports the truth.

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


# BCK-002 A-56: M7 0.3.4's rule, the back-fill's, and M7 0.5.0's
RULES = ("cognatic", "agnatic", "bounded")


@dataclass(frozen=True)
class World:
    """A world's genesis counts (the fixture-worlds golden at seed 1) and its parameters."""

    id: str
    population: int  # living at tick 0
    members: int  # living great-house members at tick 0
    houses: int  # great houses (H)
    named_women: float  # W_n: named member women aged 15-49 per house at genesis (--derive-caps)
    horizon: int  # years
    mult: str  # elite_fertility_mult
    holy: float  # vocation_holy_orders
    capacity: int | None  # density_capacity, None for the default (never binds)
    rule: str = "cognatic"  # the membership rule of M7's on_birth (RULES)


# W_n: the fixture reference's named member women aged 15-49 at genesis (the full reading), per
# house: twin_vales 5 / 5 houses at seed 1 (5 and 4 at seeds 2 and 3), kingdom_10k 8 / 12 at seed 1.
TWIN_VALES = World("twin_vales", 1500, 56, 5, 1.0, 300, "1.2", 0.2, None)
KINGDOM_10K = World("kingdom_10k", 10000, 117, 12, 8 / 12, 1000, "1.2", 0.2, None)
WORLDS = (TWIN_VALES, KINGDOM_10K, replace(KINGDOM_10K, capacity=10000))
# the reviewer's variants, uncapped, under the cognatic rule
SENSITIVITY = (("1.0", 0.2), ("1.2", 0.0), ("1.0", 0.0))
# the bounded rule with W_n doubled (mid-reign holders are younger than the genesis cross-section)
CAP_SENSITIVITY = 2.0


@dataclass
class Rates:
    """The chain's NRRs, the generation length and survival to it."""

    nrr_c: float
    nrr_e: float
    t_years: float
    l_t: float


def rates(mult: str) -> Rates:
    """NRR of commoner and member women, the mean age of net maternity, and l(T)."""
    st = m2f1.Standards(m2f1.load_spec(m2f1.NEXT_SPEC))
    tfr_bits = m2f1.q16(TFR)
    level = m2f1.level_bits(st, tfr_bits)
    nrr_c, t_years = m2f1.nrr_with_widowhood(st, tfr_bits)
    elite = m2f1.elite_level_bits(level, m2f1.q16(mult))
    chain = m2f1.Chain(st, elite, E0, mortality=True, gap=HUSBAND_GAP)
    nrr_e, _ = m2f1.nrr_and_mean_age(chain.run()[0], st)
    l_t = chain.survival(round(t_years * YEAR_360))
    return Rates(nrr_c, nrr_e, t_years, l_t)


@dataclass
class Point:
    """The population, the member share and the member count at one year."""

    year: float
    n: float
    share: float
    members: float


def step(w: World, r: Rates, w_e: float, w_c: float) -> tuple[float, float]:
    """One generation under the world's membership rule (the docstring's recursion)."""
    n = w_e + w_c
    m_d = 1.0
    if w.capacity is not None and n > w.capacity:
        m_d = min(MODIFIER_CAP, 1.0 + (n / w.capacity - 1.0))
    scale = r.l_t ** (m_d - 1.0)
    m = w_e / n
    h = m * (1 - w.holy) / (m * (1 - w.holy) + 1 - m)
    b_e = w_e * (1 - w.holy) * r.nrr_e * scale  # born of member mothers
    b_c = w_c * r.nrr_c * scale  # born of commoner mothers
    if w.rule == "cognatic":
        members = b_e + h * b_c
    elif w.rule == "agnatic":
        members = h * (b_e + b_c)
    elif w.rule == "bounded":
        named = min(w_e, 2 * w.houses * w.named_women)  # the named member women, as persons
        members = h * (b_e + b_c) + (1 - h) * named * (1 - w.holy) * r.nrr_e * scale
    else:
        raise SystemExit(f"unknown membership rule {w.rule!r}")
    return members, b_e + b_c - members


def project(w: World, r: Rates) -> list[Point]:
    """The generational recursion over the horizon, one point per generation."""
    w_e, w_c = float(w.members), float(w.population - w.members)
    points = [Point(0.0, w.population, w.members / w.population, w.members)]
    year = 0.0
    while year < w.horizon:
        w_e, w_c = step(w, r, w_e, w_c)
        year += r.t_years
        points.append(Point(year, w_e + w_c, w_e / (w_e + w_c), w_e))
    return points


def at(points: list[Point], year: float) -> Point:
    """The point at ``year`` by geometric interpolation between generations."""
    for p0, p1 in itertools.pairwise(points):
        if p0.year <= year <= p1.year:
            f = (year - p0.year) / (p1.year - p0.year)
            n = p0.n * (p1.n / p0.n) ** f
            share = p0.share + (p1.share - p0.share) * f
            return Point(year, n, share, p0.members + (p1.members - p0.members) * f)
    return points[-1]


def render_world(w: World, r: Rates) -> list[str]:
    """The golden lines of one world under one rule."""
    points = project(w, r)
    cap = "default" if w.capacity is None else str(w.capacity)
    lines = [
        f"world {w.id} rule {w.rule} horizon {w.horizon} mult {w.mult} holy {w.holy} "
        f"capacity {cap} houses {w.houses} named_women_per_house {w.named_women:.2f}"
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


def render() -> str:
    """The whole golden text."""
    base = rates("1.2")
    lines = [
        "# population projection of the SPR-030 fixture worlds (REV-SPR-030 R1) under the three",
        "# membership rules of BCK-002 A-56 (specs/systems/next/M7_membership.md): GENERATED by",
        "# python -I specs/worldgen/project_population.py --write; do not edit.",
        "constants population-projection-v1",
        f"rates mult 1.2 nrr_c {base.nrr_c:.4f} nrr_e {base.nrr_e:.4f} "
        f"r_c {100 * math.log(base.nrr_c) / base.t_years:+.3f}%/yr "
        f"generation_years {base.t_years:.2f} l_T {base.l_t:.4f}",
    ]
    for rule in RULES:
        for w in WORLDS:
            lines += render_world(replace(w, rule=rule), base)
    for mult, holy in SENSITIVITY:
        r = rates(mult)
        w = replace(KINGDOM_10K, mult=mult, holy=holy)
        p = project(w, r)
        final = at(p, w.horizon)
        peak = max([*(q for q in p if q.year <= w.horizon), final], key=lambda q: q.n)
        lines.append(
            f"sensitivity rule cognatic mult {mult} holy {holy} nrr_e {r.nrr_e:.4f} "
            f"final population {final.n:.0f} member_share {final.share:.3f} "
            f"peak {peak.n:.0f} at year {peak.year:.0f}"
        )
    for base_world in (TWIN_VALES, KINGDOM_10K):
        w_n = base_world.named_women * CAP_SENSITIVITY
        w = replace(base_world, rule="bounded", named_women=w_n)
        final = at(project(w, base), w.horizon)
        lines.append(
            f"sensitivity rule bounded named_women_per_house {w.named_women:.2f} world {w.id} "
            f"final population {final.n:.0f} member_share {final.share:.3f} "
            f"members {final.members:.0f}"
        )
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
        cases = len(WORLDS) * len(RULES)
        print(f"POPULATION PROJECTION: PASS ({cases} cases, {text.count(chr(10))} lines)")
        return 0
    print(text, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
