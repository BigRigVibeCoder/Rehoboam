"""Genesis of SPR-030's fixture worlds, pinned by the exact reference (SPR-030; PRJ-002 §6).

The two manifests `specs/manifest/examples/twin_vales.yaml` (the dynastic fixture world: 1,500
persons, two polities, five great houses) and `kingdom_10k.yaml` (the G1a performance world:
10,000 persons, three polities, twelve great houses) are built here, the fixture at seeds 1 to 3
and the 10k world at seeds 1 and 2, by the exact reference of population synthesis and the
dynastic back-fill (``specs/worldgen/selftest_population_synthesis.py``, SPR-019), and what the
G1a gates and the annals need from genesis is pinned in
``specs/worldgen/golden/fixture_worlds_v0.txt``:

  * the counts: persons, living, back-filled dynasts, synthesised, married, households, orphans;
    the living by sex and by age band (0-14, 15-49, 50 and over);
  * every house: its founder indices (``founder = k``, the annals' labels and the names file's
    keys: SPR-019 B4, B6, B8), generations, rows, members, the living, the title it holds and
    the holder's k, its tenure log's length and the stand-in fallbacks;
  * the named actors at world start under M7's criterion (SIM-9; T1 [initialization]): the
    holder of each house's title and every living house member within ``dormancy_degrees``
    kinship steps of a holder of the house's title (parent-or-child steps over the life-course
    log, the dead included: the effect language's kin(p, n)); beside them the adults beyond that
    distance (dormant-eligible at the first dormancy pass) and the minors beyond it (named or not
    is SPR-021's question: companion §2);
  * the digests of the person, marriage, household and house_member rows the SPR-019 reference
    renders, so the genesis of these worlds is bit-pinned.

The companion ``specs/worldgen/fixture_worlds.md`` says which mechanism each gate exercises on
each world and gives the arithmetic. ``python/tests/test_fixture_worlds_cases.py`` keeps the
hardcoded manifests below equal to the YAML files.

Run: python -I specs/worldgen/selftest_fixture_worlds.py            (check; exit 1 on failure)
     python -I specs/worldgen/selftest_fixture_worlds.py --write    (regenerate the golden)
     python -I specs/worldgen/selftest_fixture_worlds.py --golden PATH   (check another file)
Exit: 0 pass, 1 fail, 2 the reference failed its own checks. Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType
from typing import Any

HERE = Path(__file__).resolve().parent
GOLDEN = HERE / "golden" / "fixture_worlds_v0.txt"
EXIT_OK, EXIT_FAIL, EXIT_REFERENCE = 0, 1, 2


def _load(name: str, path: Path) -> ModuleType:
    """Load a sibling reference by path (python -I adds no script directory to sys.path)."""
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SystemExit(f"{path} not found")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


ps = _load("selftest_population_synthesis", HERE / "selftest_population_synthesis.py")
# The reference's types, loaded by path (mypy sees a module object, so they are aliases of Any).
Manifest = Any
Genesis = Any
Genealogy = Any
House = Any
Title = Any

# M7 parameters the manifests set and genesis reads (SPR-019 B0: majority_age; SIM-9:
# dormancy_degrees).
MAJORITY_AGE = "16"
DORMANCY_DEGREES = 2
CHILD_MAX = 15
FERTILE_END = 50
SEEDS = {
    "twin_vales": (1, 2, 3),
    "kingdom_10k": (1, 2),
}  # the fixture at seeds 1-3 (the brief), 10k at 1-2

# The manifests, as specs/manifest/examples/twin_vales.yaml and kingdom_10k.yaml give them
# (python/tests/test_fixture_worlds_cases.py checks these mirrors against the YAML).
TWIN_VALES = ps.Manifest(
    "twin_vales",
    1,
    360,
    "mountain_valleys",
    128,
    6,
    1500,
    0,
    "30",
    "1",
    "4.9",
    "1.2",
    MAJORITY_AGE,
    (
        ps.Polity(
            "vale_of_ash",
            "primogeniture",
            (ps.Dynasty("ashcombe", 5), ps.Dynasty("thorne", 4), ps.Dynasty("redmane", 3)),
        ),
        ps.Polity("high_fells", "tanistry", (ps.Dynasty("fell", 6), ps.Dynasty("carrow", 3))),
    ),
)
KINGDOM_10K = ps.Manifest(
    "kingdom_10k",
    1,
    360,
    "continental_plains",
    256,
    40,
    10000,
    0,
    "30",
    "1",
    "4.9",
    "1.2",
    MAJORITY_AGE,
    (
        ps.Polity(
            "crownlands",
            "primogeniture",
            (
                ps.Dynasty("aldric", 6),
                ps.Dynasty("beaumont", 5),
                ps.Dynasty("corvay", 4),
                ps.Dynasty("dunmere", 4),
                ps.Dynasty("estwick", 3),
            ),
        ),
        ps.Polity(
            "marches",
            "seniority",
            (
                ps.Dynasty("ferrand", 5),
                ps.Dynasty("gault", 4),
                ps.Dynasty("hollin", 3),
                ps.Dynasty("ingram", 3),
            ),
        ),
        ps.Polity(
            "high_fells",
            "tanistry",
            (ps.Dynasty("fell", 6), ps.Dynasty("carrow", 4), ps.Dynasty("moss", 3)),
        ),
    ),
)
WORLDS: tuple[Manifest, ...] = (TWIN_VALES, KINGDOM_10K)


def kin_within(gen: Genealogy, p: int, n: int) -> set[int]:
    """Return kin(p, n): every other person within n parent-or-child steps, the dead included."""
    seen = {p}
    frontier = [p]
    for _ in range(n):
        nxt: list[int] = []
        for y in frontier:
            for z in gen.parents(y, False) + gen.children(y):
                if z not in seen:
                    seen.add(z)
                    nxt.append(z)
        frontier = nxt
    return seen - {p}


@dataclass
class HouseReport:
    """One house at world start: its members, the named and the dormant-eligible (SIM-9)."""

    house: House
    title: Title
    living: list[int]
    named: list[
        int
    ]  # living members within dormancy_degrees of the title's holder, holder included
    dormant_adults: list[int]  # living adult members beyond that distance: Demote at the first pass
    minors_far: list[int]  # living minor members beyond it (SPR-021 decides whether they are named)
    k_lo: int
    k_hi: int


def house_report(g: Genesis, gen: Genealogy, h: House) -> HouseReport:
    """Classify one house's living members at tick 0 by the naming criterion."""
    t = next(t for t in g.titles if t.id == h.title)
    living = [pid for pid in h.members if g.persons[pid].alive]
    near = kin_within(gen, t.holder, DORMANCY_DEGREES) | {t.holder}
    named = [pid for pid in living if pid in near]
    far = [pid for pid in living if pid not in near]
    adults = [pid for pid in far if -g.persons[pid].birth >= g.majority]
    minors = [pid for pid in far if -g.persons[pid].birth < g.majority]
    ks = sorted(g.persons[pid].k for pid in h.members)
    return HouseReport(h, t, living, named, adults, minors, ks[0], ks[-1])


def living_bands(g: Genesis) -> tuple[int, int, int, int, int]:
    """Return the living by sex (male, female) and by age band (0-14, 15-49, 50+) at tick 0."""
    y = g.cal.year
    males = females = young = fertile = old = 0
    for p in g.persons.values():
        if not p.alive:
            continue
        males += p.sex == ps.SEX_MALE
        females += p.sex == ps.SEX_FEMALE
        age = -p.birth // y
        if age < CHILD_MAX:
            young += 1
        elif age < FERTILE_END:
            fertile += 1
        else:
            old += 1
    return males, females, young, fertile, old


def render_world(m: Manifest) -> tuple[list[str], list[str]]:
    """Build one world; return its golden lines and the failures of the SPR-019 invariants."""
    g = ps.build(m)
    failures = ps.Checker(g).run() + ps.check_widths(g)
    gen = ps.Genealogy(g.persons, 0)
    reports = [house_report(g, gen, h) for h in g.houses]
    r = g.report
    males, females, young, fertile, old = living_bands(g)
    lines = [
        f"world {m.id} {m.seed}",
        f"population {m.population} living {sum(1 for p in g.persons.values() if p.alive)} "
        f"persons {len(g.persons)} dynasts {r.living_dynasts} synthesised {r.synthesised} "
        f"married {r.married} households {len(g.households)} orphans {r.orphans}",
        f"living_by_sex males {males} females {females} by_age 0-14 {young} 15-49 {fertile} "
        f"50+ {old}",
        f"named_at_start {sum(len(x.named) for x in reports)} "
        f"dormant_adults {sum(len(x.dormant_adults) for x in reports)} "
        f"minors_beyond {sum(len(x.minors_far) for x in reports)} "
        f"dormancy_degrees {DORMANCY_DEGREES} majority_age {MAJORITY_AGE}",
    ]
    for x in reports:
        h, t = x.house, x.title
        lines.append(
            f"house {h.k} generations {h.generations} founders {x.k_lo}..{x.k_hi} rows {h.rows} "
            f"members {len(h.members)} living {len(x.living)} named {len(x.named)} "
            f"dormant_adults {len(x.dormant_adults)} minors_beyond {len(x.minors_far)} "
            f"title {t.k} law {t.law} holder_k {g.persons[t.holder].k} tenures {len(t.tenures)} "
            f"fallbacks {t.fallbacks} minors_seated {t.minor_fallbacks}"
        )
    case_lines = ps.render_case(g)
    lines += [line for line in case_lines if line.endswith("_sha256") or "_sha256 " in line]
    return lines, failures


HEADER = [
    "# fixture worlds golden outputs (SPR-030): GENERATED by",
    "# python -I specs/worldgen/selftest_fixture_worlds.py --write; do not edit.",
    "# Per world and seed: the counts at tick 0; the living by sex and age band; the named",
    "# actors at world start (M7 SIM-9: holders and living house members within dormancy_degrees",
    "# of the house's holder), the dormant-eligible adults and the minors beyond that distance;",
    "# each house with its founder indices, generations, rows, members, living, named, title,",
    "# holder and tenure log; the SPR-019 digests of the person, marriage, household and",
    "# house_member rows.",
    "constants fixture-worlds-v0",
]


def render() -> tuple[str, list[str]]:
    """Render every world at every seed."""
    lines = HEADER[:]
    failures: list[str] = []
    for m in WORLDS:
        for seed in SEEDS[m.id]:
            world_lines, world_failures = render_world(ps.Manifest(**{**m.__dict__, "seed": seed}))
            lines += world_lines
            failures += world_failures
    return "\n".join(lines) + "\n", failures


def first_difference(current: str, text: str) -> int:
    """Return the 1-based line of the first difference between two texts."""
    a, b = current.splitlines(), text.splitlines()
    pairs = enumerate(zip(a, b, strict=False))
    return next((i for i, (x, z) in pairs if x != z), min(len(a), len(b))) + 1


def main(argv: list[str] | None = None) -> int:
    """Check or regenerate the golden file."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--write", action="store_true", help="regenerate the golden file")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the golden file to check")
    args = parser.parse_args(argv)
    try:
        text, failures = render()
    except AssertionError as e:
        text, failures = "", [f"construction error: {e}"]
    if failures:
        for x in failures[:20]:
            print(f"REFERENCE FAILED: {x}")
        print(f"FIXTURE WORLDS: FAIL (the reference's own checks: {len(failures)} failures)")
        return EXIT_REFERENCE
    if args.write:
        args.golden.parent.mkdir(parents=True, exist_ok=True)
        args.golden.write_text(text, encoding="utf-8")
        print(f"wrote {args.golden}")
        return EXIT_OK
    current = args.golden.read_text(encoding="utf-8") if args.golden.exists() else ""
    if current != text:
        line = first_difference(current, text)
        print(f"STALE: {args.golden} differs from the reference at line {line}")
        print("FIXTURE WORLDS: FAIL")
        return EXIT_FAIL
    n, cases = text.count("\n"), sum(len(v) for v in SEEDS.values())
    print(f"FIXTURE WORLDS: PASS ({len(WORLDS)} worlds, {cases} cases, {n} lines)")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
