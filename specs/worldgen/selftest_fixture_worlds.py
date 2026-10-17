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
  * every house: its members' founder indices (``founder = k``, the annals' labels and the names
    file's keys: SPR-019 B4, B6, B8), generations, rows, members, the living, the title it holds
    with the SPR-019 reference's title and tenure lines (tier, liege, polity, holder, held_since,
    predecessor_exit, every tenure row: REV-SPR-030 R6), the title's line at genesis under its
    law (the effect language's succession laws, as the SPR-019 reference states them) with the
    heir and the second in line as ``(k, kinship distance from the holder)`` and the line's
    length (an empty line is the computable route to an interregnum: REV-SPR-030 R2, R8);
  * the named actors at world start under M7's rule as written (SIM-9; `dormancy`, `on_birth`;
    T1 [initialization]), in two readings: the NARROW one, the holder of each house's title and
    every living house member within ``dormancy_degrees`` kinship steps of a holder of the
    house's title (parent-or-child steps over the life-course log, the dead included: the effect
    language's kin(p, n)); and the FULL one, the gate's, which adds the first and the second in
    each title's line (on_birth's clause) and the ruling house's heir (dormancy's Promote clause;
    claim holders would join, and none exist at genesis: REV-SPR-030 R2). Beside them the adults
    beyond that distance (dormant-eligible at the first pass, which runs at world start) and the
    minors beyond it (named or not is SPR-021's question: companion §2);
  * the digests of the person, marriage, household and house_member rows the SPR-019 reference
    renders, so the genesis of these worlds is bit-pinned.

The companion ``specs/worldgen/fixture_worlds.md`` says which mechanism each gate exercises on
each world and gives the arithmetic. ``python/tests/test_fixture_worlds_cases.py`` keeps the
hardcoded manifests below equal to the YAML files.

Tiers (REV-SPR-030 R10): the PR tier renders the fixture at seeds 1 to 3 and the 10k world at
seed 1 and compares those blocks of the golden file; the nightly tier renders every case. The
altered-golden control runs in the same process as the check, on the one rendering.

Run: python -I specs/worldgen/selftest_fixture_worlds.py                 (every case)
     python -I specs/worldgen/selftest_fixture_worlds.py --tier pr       (the PR tier's cases)
     python -I specs/worldgen/selftest_fixture_worlds.py --write         (regenerate the golden)
     python -I specs/worldgen/selftest_fixture_worlds.py --golden PATH   (check another file)
     python -I specs/worldgen/selftest_fixture_worlds.py --tier pr --altered-golden PATH
         (also require that an altered copy of the golden file is reported STALE)
     python -I specs/worldgen/selftest_fixture_worlds.py --control dormancy-one
         (must FAIL: dormancy_degrees read as 1 moves the named-at-start lines of twin_vales
         seed 1 against the committed golden)
Exit: 0 pass, 1 fail, 2 the reference failed its own checks. Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
from collections import deque
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
LINE_HEAD = 2  # on_birth: the first and the second in a title's line meet the naming criteria
DISTANCE_LIMIT = 32  # a kinship distance beyond this is reported as none (no such pair exists)
SEEDS = {
    "twin_vales": (1, 2, 3),
    "kingdom_10k": (1, 2),
}  # the fixture at 1-3 (the brief), 10k at 1-2
PR_TIER = {"twin_vales": (1, 2, 3), "kingdom_10k": (1,)}  # REV-SPR-030 R10; the rest nightly
CONTROL_CASE = ("twin_vales", 1)  # --control dormancy-one renders this case alone

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


# ----------------------------------------------------------------------------- kinship
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


def kin_distance(gen: Genealogy, a: int, b: int) -> int | None:
    """Return the least number of parent-or-child steps from a to b, or None beyond the limit."""
    if a == b:
        return 0
    seen = {a}
    queue = deque([(a, 0)])
    while queue:
        x, d = queue.popleft()
        if d >= DISTANCE_LIMIT:
            continue
        for y in gen.parents(x, False) + gen.children(x):
            if y == b:
                return d + 1
            if y not in seen:
                seen.add(y)
                queue.append((y, d + 1))
    return None


# ----------------------------------------------------------------------------- one house
@dataclass
class HouseReport:
    """One house at world start: its members, the named under both readings, the line (SIM-9)."""

    house: House
    title: Title
    living: list[int]
    narrow: list[
        int
    ]  # living members within dormancy_degrees of the title's holder, holder included
    full: list[int]  # narrow + the first and second in the title's line (the gate's reading)
    dormant_adults: list[int]  # living adult members beyond that distance and not named (R18)
    minors_far: list[int]  # living minor members beyond it (SPR-021 decides whether they are named)
    line: list[int]  # the title's line at genesis under its law, in succession order
    line_outside: int  # persons in the line who are not members of the house


def house_report(g: Genesis, gen: Genealogy, h: House, degrees: int) -> HouseReport:
    """Classify one house's living members at tick 0 by the naming criterion, both readings."""
    t = next(t for t in g.titles if t.id == h.title)
    members = set(h.members)
    living = [pid for pid in h.members if g.persons[pid].alive]
    near = kin_within(gen, t.holder, degrees) | {t.holder}
    narrow = [pid for pid in living if pid in near]
    line = ps.Law(gen, t.holder, t.tenures, g.majority).line_for(t.law)
    head = set(line[:LINE_HEAD])
    full = [pid for pid in living if pid in near or pid in head]
    far = [pid for pid in living if pid not in near and pid not in head]  # R18: not the named
    adults = [pid for pid in far if -g.persons[pid].birth >= g.majority]
    minors = [pid for pid in far if -g.persons[pid].birth < g.majority]
    outside = sum(1 for pid in line if pid not in members)
    return HouseReport(h, t, living, narrow, full, adults, minors, line, outside)


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


def line_text(g: Genesis, gen: Genealogy, x: HouseReport) -> str:
    """The heir and the second in line as k/distance, and the line's length."""
    parts = []
    for label, pid in zip(("heir", "second"), x.line[:LINE_HEAD], strict=False):
        d = kin_distance(gen, x.title.holder, pid)
        parts.append(f"{label} {g.persons[pid].k}/{'none' if d is None else d}")
    for label in ("heir", "second")[len(x.line) :]:
        parts.append(f"{label} none")
    return f"line {len(x.line)} outside_house {x.line_outside} " + " ".join(parts)


def render_world(m: Manifest, degrees: int) -> tuple[list[str], list[str]]:
    """Build one world; return its golden lines and the failures of the SPR-019 invariants."""
    g = ps.build(m)
    failures = ps.Checker(g).run() + ps.check_widths(g)
    gen = ps.Genealogy(g.persons, 0)
    reports = [house_report(g, gen, h, degrees) for h in g.houses]
    r = g.report
    males, females, young, fertile, old = living_bands(g)
    lines = [
        f"world {m.id} {m.seed}",
        f"population {m.population} living {sum(1 for p in g.persons.values() if p.alive)} "
        f"persons {len(g.persons)} dynasts {r.living_dynasts} synthesised {r.synthesised} "
        f"married {r.married} households {len(g.households)} orphans {r.orphans}",
        f"living_by_sex males {males} females {females} by_age 0-14 {young} 15-49 {fertile} "
        f"50+ {old}",
        f"named_at_start narrow {sum(len(x.narrow) for x in reports)} "
        f"full {sum(len(x.full) for x in reports)} "
        f"dormant_adults {sum(len(x.dormant_adults) for x in reports)} "
        f"minors_beyond {sum(len(x.minors_far) for x in reports)} "
        f"empty_lines {sum(1 for x in reports if not x.line)} "
        f"dormancy_degrees {DORMANCY_DEGREES} majority_age {MAJORITY_AGE}",  # the label, not the
        # degree used: --control dormancy-one must move the counts, not this label (R19)
    ]
    for x in reports:
        h, t = x.house, x.title
        lines += [
            f"house {h.k} generations {h.generations} rows {h.rows} members {len(h.members)} "
            f"living {len(x.living)} named_narrow {len(x.narrow)} named_full {len(x.full)} "
            f"dormant_adults {len(x.dormant_adults)} minors_beyond {len(x.minors_far)} "
            f"title {t.k} law {t.law} holder_k {g.persons[t.holder].k} tenures {len(t.tenures)} "
            f"fallbacks {t.fallbacks} minors_seated {t.minor_fallbacks} {line_text(g, gen, x)}",
            f"members_k {h.k}: " + " ".join(str(g.persons[pid].k) for pid in h.members),
            *ps.render_house(g, h),
        ]
    case_lines = ps.render_case(g)
    lines += [line for line in case_lines if line.endswith("_sha256") or "_sha256 " in line]
    return lines, failures


HEADER = [
    "# fixture worlds golden outputs (SPR-030): GENERATED by",
    "# python -I specs/worldgen/selftest_fixture_worlds.py --write; do not edit.",
    "# Per world and seed: the counts at tick 0; the living by sex and age band; the named",
    "# actors at world start under M7's rule (narrow: holders and living house members within",
    "# dormancy_degrees of the house's holder; full: plus the first and second in each title's",
    "# line, the gate's reading), the dormant-eligible adults and the minors beyond that distance,",
    "# the titles whose line is empty; each house with its generations, rows, members, living,",
    "# named under both readings, title, holder, tenure count, the line's length and its heir and",
    "# second as k/kinship distance, its members' founder indices, and the SPR-019 reference's",
    "# title and tenure lines; the SPR-019 digests of the person, marriage, household and",
    "# house_member rows.",
    "constants fixture-worlds-v0",
]


def cases(tier: str) -> list[tuple[Manifest, int]]:
    """The (manifest, seed) cases of a tier."""
    seeds = PR_TIER if tier == "pr" else SEEDS
    return [(m, seed) for m in WORLDS for seed in seeds[m.id]]


def render(selected: list[tuple[Manifest, int]], degrees: int) -> tuple[dict[str, str], list[str]]:
    """Render the selected cases; return the blocks by 'world id seed' and the failures."""
    blocks: dict[str, str] = {}
    failures: list[str] = []
    for m, seed in selected:
        world_lines, world_failures = render_world(
            ps.Manifest(**{**m.__dict__, "seed": seed}), degrees
        )
        blocks[world_lines[0]] = "\n".join(world_lines) + "\n"
        failures += world_failures
    return blocks, failures


def split_blocks(text: str) -> tuple[str, dict[str, str]]:
    """Split a golden text into its header and its 'world id seed' blocks."""
    header: list[str] = []
    blocks: dict[str, str] = {}
    key = ""
    for line in text.splitlines():
        if line.startswith("world "):
            key = line
            blocks[key] = ""
        if key:
            blocks[key] += line + "\n"
        else:
            header.append(line)
    return "\n".join(header) + ("\n" if header else ""), blocks


def first_difference(current: str, text: str) -> int:
    """Return the 1-based line of the first difference between two texts."""
    a, b = current.splitlines(), text.splitlines()
    pairs = enumerate(zip(a, b, strict=False))
    return next((i for i, (x, z) in pairs if x != z), min(len(a), len(b))) + 1


def compare(golden_text: str, blocks: dict[str, str]) -> str | None:
    """Return a STALE line for the first rendered block that differs from the golden, or None."""
    header, golden_blocks = split_blocks(golden_text)
    expected_header = "\n".join(HEADER) + "\n"
    if header != expected_header:
        return f"STALE: header differs at line {first_difference(header, expected_header)}"
    for key, text in blocks.items():
        current = golden_blocks.get(key, "")
        if current != text:
            offset = golden_text.find(key)
            base = golden_text[:offset].count("\n") if offset >= 0 else 0
            return f"STALE: {key!r} differs at line {base + first_difference(current, text)}"
    return None


def parse(argv: list[str] | None) -> argparse.Namespace:
    """The command line."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--write", action="store_true", help="regenerate the golden file")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the golden file to check")
    parser.add_argument("--tier", choices=("pr", "nightly"), default="nightly")
    parser.add_argument("--altered-golden", type=Path, help="an altered copy that must be STALE")
    parser.add_argument("--control", choices=("dormancy-one",), help="a control that must fail")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Check or regenerate the golden file; run the controls."""
    args = parse(argv)
    degrees = 1 if args.control == "dormancy-one" else DORMANCY_DEGREES
    if args.control:
        selected = [(m, s) for m, s in cases("nightly") if (m.id, s) == CONTROL_CASE]
    else:
        selected = cases("nightly" if args.write else args.tier)
    try:
        blocks, failures = render(selected, degrees)
    except AssertionError as e:
        blocks, failures = {}, [f"construction error: {e}"]
    if failures:
        for x in failures[:20]:
            print(f"REFERENCE FAILED: {x}")
        print(f"FIXTURE WORLDS: FAIL (the reference's own checks: {len(failures)} failures)")
        return EXIT_REFERENCE
    text = "\n".join(HEADER) + "\n" + "".join(blocks.values())
    if args.write:
        args.golden.parent.mkdir(parents=True, exist_ok=True)
        args.golden.write_text(text, encoding="utf-8")
        print(f"wrote {args.golden}")
        return EXIT_OK
    current = args.golden.read_text(encoding="utf-8") if args.golden.exists() else ""
    stale = compare(current, blocks)
    if stale:
        print(stale)
        print("FIXTURE WORLDS: FAIL")
        return EXIT_FAIL
    if args.control:
        print(f"CONTROL INERT: {args.control} left the golden lines unchanged")
        return EXIT_OK
    if args.altered_golden is not None:
        altered = compare(args.altered_golden.read_text(encoding="utf-8"), blocks)
        if altered is None:
            print(f"ALTERED GOLDEN CONTROL FAILED: {args.altered_golden} equals the reference")
            print("FIXTURE WORLDS: FAIL")
            return EXIT_FAIL
        print(f"altered golden control: {altered} (as required)")
    n = text.count("\n")
    print(f"FIXTURE WORLDS: PASS ({args.tier} tier: {len(selected)} cases, {n} lines rendered)")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
