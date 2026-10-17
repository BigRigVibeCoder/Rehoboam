"""The population projection of SPR-030's fixture worlds with the house-member mechanics.

REV-SPR-030 R1: the trajectory of `twin_vales` and `kingdom_10k` over their horizons cannot be
read from the commoner NRR alone, because three mechanics the manifests set act on great-house
members and membership spreads through the population:

  * M7 `on_birth`: a newborn joins the father's house, else the mother's (cognatic), so a child
    of a member and a commoner is a member;
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
(members in holy orders excluded), and her daughters are then members (the father's house):

    W_e' = W_e (1 - v) NRR_e + W_c NRR_c h
    W_c' = W_c NRR_c (1 - h)                   with h = m (1 - v) / (m (1 - v) + 1 - m)

where m is the member share and v = vocation_holy_orders. The population is proportional to
W_e + W_c (a constant sex ratio). The capped case (`kingdom_10k` with `density_capacity` K and
slope 1, M2 F1 MR11) multiplies every hazard by m_d = min(4, 1 + max(0, N / K - 1)), which
under proportional hazards turns survival to age T into l(T)^m_d, so each NRR is scaled by
l(T)^(m_d - 1), with l from the same chain's schedule (rung 0 at e0 30).

Limits (stated, not hidden): generations are discrete (no age structure, no overlap); mating is
random (M2 N4 bounds husbands to the settlement and excludes the named; both make h smaller for
commoner women far from a seat); holy orders is applied as a 20% removal of member women at
majority (the vocation draw excludes the first in a line and seat holders, a handful); the
back-fill's age structure at genesis and widowhood beyond the chain's are ignored; the capped
case's NRR scaling is the proportional-hazards approximation at one age. The model says what the
mechanics imply to within tens of per cent, not a golden; the engine's run reports the truth.

Run: python -I specs/worldgen/project_population.py            (print the tables)
     python -I specs/worldgen/project_population.py --check    (compare with the golden file)
     python -I specs/worldgen/project_population.py --write    (regenerate the golden file)
Golden: specs/worldgen/golden/population_projection_v0.txt. Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import importlib.util
import itertools
import math
import sys
from dataclasses import dataclass
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


@dataclass(frozen=True)
class World:
    """A world's genesis counts (the fixture-worlds golden at seed 1) and its parameters."""

    id: str
    population: int  # living at tick 0
    members: int  # living great-house members at tick 0
    horizon: int  # years
    mult: str  # elite_fertility_mult
    holy: float  # vocation_holy_orders
    capacity: int | None  # density_capacity, None for the default (never binds)


WORLDS = (
    World("twin_vales", 1500, 56, 300, "1.2", 0.2, None),
    World("kingdom_10k", 10000, 117, 1000, "1.2", 0.2, None),
    World("kingdom_10k", 10000, 117, 1000, "1.2", 0.2, 10000),
)
SENSITIVITY = (("1.0", 0.2), ("1.2", 0.0), ("1.0", 0.0))  # the reviewer's variants, uncapped


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
    """The population and the member share at one year."""

    year: float
    n: float
    share: float


def project(w: World, r: Rates) -> list[Point]:
    """The generational recursion over the horizon, one point per generation."""
    w_e, w_c = float(w.members), float(w.population - w.members)
    points = [Point(0.0, w.population, w.members / w.population)]
    year = 0.0
    while year < w.horizon:
        n = w_e + w_c
        m_d = 1.0
        if w.capacity is not None and n > w.capacity:
            m_d = min(MODIFIER_CAP, 1.0 + (n / w.capacity - 1.0))
        scale = r.l_t ** (m_d - 1.0)
        m = w_e / n
        h = m * (1 - w.holy) / (m * (1 - w.holy) + 1 - m)
        w_e, w_c = (
            w_e * (1 - w.holy) * r.nrr_e * scale + w_c * r.nrr_c * scale * h,
            w_c * r.nrr_c * scale * (1 - h),
        )
        year += r.t_years
        points.append(Point(year, w_e + w_c, w_e / (w_e + w_c)))
    return points


def at(points: list[Point], year: float) -> Point:
    """The point at ``year`` by geometric interpolation between generations."""
    for p0, p1 in itertools.pairwise(points):
        if p0.year <= year <= p1.year:
            f = (year - p0.year) / (p1.year - p0.year)
            n = p0.n * (p1.n / p0.n) ** f
            return Point(year, n, p0.share + (p1.share - p0.share) * f)
    return points[-1]


def render_world(w: World, r: Rates) -> list[str]:
    """The golden lines of one world."""
    points = project(w, r)
    cap = "default" if w.capacity is None else str(w.capacity)
    lines = [f"world {w.id} horizon {w.horizon} mult {w.mult} holy {w.holy} capacity {cap}"]
    for year in CHECKPOINTS:
        if year <= w.horizon:
            p = at(points, year)
            lines.append(f"year {year} population {p.n:.0f} member_share {p.share:.3f}")
    peak = max(points, key=lambda p: p.n)
    years = range(w.horizon + 1)
    mean = sum(at(points, y).n for y in years) / len(years)
    lines.append(f"peak population {peak.n:.0f} at year {peak.year:.0f} mean {mean:.0f}")
    for level in MEMBER_MILESTONES:
        hit = next((p for p in points if p.share >= level), None)
        when = "never" if hit is None else f"{hit.year:.0f}"
        lines.append(f"member_share_reaches {level} year {when}")
    return lines


def render() -> str:
    """The whole golden text."""
    base = rates("1.2")
    lines = [
        "# population projection of the SPR-030 fixture worlds (REV-SPR-030 R1): GENERATED by",
        "# python -I specs/worldgen/project_population.py --write; do not edit.",
        "constants population-projection-v0",
        f"rates mult 1.2 nrr_c {base.nrr_c:.4f} nrr_e {base.nrr_e:.4f} "
        f"r_c {100 * math.log(base.nrr_c) / base.t_years:+.3f}%/yr "
        f"generation_years {base.t_years:.2f} l_T {base.l_t:.4f}",
    ]
    for w in WORLDS:
        lines += render_world(w, base)
    for mult, holy in SENSITIVITY:
        r = rates(mult)
        w = World("kingdom_10k", 10000, 117, 1000, mult, holy, None)
        p = project(w, r)
        final, peak = p[-1], max(p, key=lambda q: q.n)
        lines.append(
            f"sensitivity mult {mult} holy {holy} nrr_e {r.nrr_e:.4f} "
            f"final population {final.n:.0f} member_share {final.share:.3f} "
            f"peak {peak.n:.0f} at year {peak.year:.0f}"
        )
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    """Print, check or write the golden file."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the golden file")
    parser.add_argument("--write", action="store_true", help="regenerate the golden file")
    args = parser.parse_args(argv)
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
        print(f"POPULATION PROJECTION: PASS ({len(WORLDS)} cases, {text.count(chr(10))} lines)")
        return 0
    print(text, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
