"""Exact reference and self-test of worldgen-lite v0 and residence at genesis (SPR-018; S3 F0).

The algorithm is stated in ``specs/systems/M1.regions.odd.toml``, section [algorithm.worldgen_lite],
steps W0 to W10, with the templates of ``specs/worldgen/templates_v0.toml``; residence at genesis
is ``specs/systems/S3.migration.odd.toml`` [algorithm.residence_f0], steps R0 to R4. This file
implements those steps with Python integers only (no float anywhere), on the keyed RNG of
``bin/gen_rng_golden.py`` (the independent rh-ids reference), so it reproduces the engine's integers
bit for bit. It checks:

  1. the committed golden outputs (``specs/worldgen/golden/worldgen_lite_v0.txt``) equal this
     reference's, for seven templates, several seeds and resolutions: elevation and region rasters
     (SHA-256), land counts, seeds, seats, region sizes, edges, route lengths and the founders'
     settlements; ``--write`` regenerates the file, never by hand;
  2. the invariants of the M1 spec hold on every case: every land cell reached from a seed has one
     region and every sea cell none; regions are 4-connected and non-empty; one seat per region,
     inside it; the region graph is exactly the 4-adjacency of regions; one route per edge;
  3. the widths the spec states hold (the accumulator, the step cost, the draw-to-index product);
  4. residence intervals tile the recorded lifetime of founders and of children born later
     (V-DATA-04's P1a variant), on a small scripted sequence of births;
  5. the bench 4 fixture (``docs/08`` §9, ``bench/raster_8x512``): the 512-square elevation raster
     of ``continental_plains`` at seed 1 and the digest of one smoothing pass over its eight layers.

Run: python -I specs/worldgen/selftest_worldgen_lite.py            (check; exit 1 on any failure)
     python -I specs/worldgen/selftest_worldgen_lite.py --write    (regenerate the golden file)
     python -I specs/worldgen/selftest_worldgen_lite.py --golden PATH   (check another golden file)
     python -I specs/worldgen/selftest_worldgen_lite.py --templates PATH   (another template file)
     The positive controls pass an altered copy of each to the matching option.
Exit: 0 pass, 1 fail, 2 the reference failed its own checks. Standard library only (GOV-003 §3).
"""

from __future__ import annotations

import argparse
import hashlib
import heapq
import math
import sys
import tomllib
from array import array
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "bin"))

import gen_rng_golden as rng  # noqa: E402  (the independent rh-ids reference; SPR-002)

TEMPLATES = HERE / "templates_v0.toml"
GOLDEN = HERE / "golden" / "worldgen_lite_v0.txt"
EXIT_OK, EXIT_FAIL, EXIT_REFERENCE = 0, 1, 2

MASK64 = (1 << 64) - 1
U16_MAX = 65535
PER_MILLE = 1000
# W0: the reserved rh-worldgen system id (SPR-011 ruling 10) and the canonical timeline's salt.
SYSTEM_WORLDGEN = rng.system_id("rh-worldgen")
SALT_CANONICAL = rng.branch_salt([], 0)
GENESIS_TICK = 0
EPOCH_GENESIS = 0
PURPOSE_BLOB = "heightmap_blob"
PURPOSE_REGION_SEED = "region_seed"
PURPOSE_RESIDENCE = "founder_residence"
# W7: the Dijkstra step cost is 1 + (|dE| >> SLOPE_SHIFT): at most 1 + 255 per step.
SLOPE_SHIFT = 8
MAX_STEP_COST = 1 + (U16_MAX >> SLOPE_SHIFT)
MAX_RESOLUTION = 4096
SPEC_MAX_STEP_COST = 256
SPEC_DISTANCE_BITS = 33
SMOOTH_CENTRE_WEIGHT = 4
SMOOTH_DIVISOR = 8
MIN_RADIUS_CELLS = 1
BENCH_LAYERS = 8
BENCH_RESOLUTION = 512
BENCH_TEMPLATE = "continental_plains"
BENCH_SEED = 1
BENCH_SHIFT_X, BENCH_SHIFT_Y = 61, 37
DRAWS_PER_DOME, DRAWS_PER_RING = 4, 5
RANGE_FIELDS = 2
MAX_OP_COUNT = 64
MAX_SMOOTH_PASSES = 4
REGION_NONE = 0
I64_MAX = (1 << 63) - 1


def pick(u: int, n: int) -> int:
    """W0: the index in [0, n) a uniform u64 draw stands for: floor(u * n / 2^64), n >= 1."""
    if not (0 <= u <= MASK64 and n >= 1):
        raise ValueError("pick needs a u64 draw and n >= 1")
    return (u * n) >> 64


def draw(world_seed: int, purpose: str, entity: int, k: int) -> int:
    """W0: H(world_seed, canonical salt, rh-worldgen, epoch 0, purpose, entity, t = 0, k)."""
    key: rng.RngKey = (
        world_seed,
        SALT_CANONICAL,
        SYSTEM_WORLDGEN,
        rng.purpose_id(purpose),
        entity,
        EPOCH_GENESIS,
        GENESIS_TICK,
    )
    return rng.h(key, k)


@dataclass(frozen=True)
class Op:
    """One template operation (W1): ``count`` blobs of one kind drawn inside the given ranges."""

    kind: str
    count: int
    height: tuple[int, int]
    radius: tuple[int, int]
    x: tuple[int, int]
    y: tuple[int, int]
    width: tuple[int, int] | None


@dataclass(frozen=True)
class Template:
    """A heightmap template: a base level, a sea level, smoothing passes and operations."""

    name: str
    base: int
    sea_level: int
    smooth_passes: int
    ops: tuple[Op, ...]


def _pair(v: object, lo: int, hi: int, what: str) -> tuple[int, int]:
    """Validate a [lo, hi] integer range inside [lo, hi] bounds."""
    if not (isinstance(v, list) and len(v) == RANGE_FIELDS and all(isinstance(x, int) for x in v)):
        raise ValueError(f"{what} must be [lo, hi] integers")
    a, b = int(v[0]), int(v[1])
    if not (lo <= a <= b <= hi):
        raise ValueError(f"{what} = [{a}, {b}] must satisfy {lo} <= lo <= hi <= {hi}")
    return a, b


def _op(raw: dict[str, object], where: str) -> Op:
    """Parse and validate one operation table."""
    kind = raw.get("kind")
    if kind not in ("dome", "ring"):
        raise ValueError(f"{where}: kind must be dome or ring")
    count = raw.get("count")
    if not isinstance(count, int) or not (1 <= count <= MAX_OP_COUNT):
        raise ValueError(f"{where}: count must be an integer in [1, {MAX_OP_COUNT}]")
    width = _pair(raw["width"], 1, PER_MILLE, f"{where}.width") if kind == "ring" else None
    if kind == "ring" and "width" not in raw:
        raise ValueError(f"{where}: a ring needs a width")
    return Op(
        kind=str(kind),
        count=count,
        height=_pair(raw.get("height"), -U16_MAX, U16_MAX, f"{where}.height"),
        radius=_pair(raw.get("radius"), 1, PER_MILLE, f"{where}.radius"),
        x=_pair(raw.get("x"), 0, PER_MILLE, f"{where}.x"),
        y=_pair(raw.get("y"), 0, PER_MILLE, f"{where}.y"),
        width=width,
    )


def load_templates(path: Path) -> dict[str, Template]:
    """Read and validate every template of the data file."""
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    out: dict[str, Template] = {}
    for name, t in data["templates"].items():
        for key in ("base", "sea_level", "smooth_passes"):
            if not isinstance(t.get(key), int) or not (0 <= t[key] <= U16_MAX):
                raise ValueError(f"{name}.{key} must be an integer in [0, {U16_MAX}]")
        if t["smooth_passes"] > MAX_SMOOTH_PASSES:
            raise ValueError(f"{name}.smooth_passes must be at most {MAX_SMOOTH_PASSES}")
        ops = tuple(_op(op, f"{name}.ops[{i}]") for i, op in enumerate(t.get("ops", [])))
        out[name] = Template(name, t["base"], t["sea_level"], t["smooth_passes"], ops)
    return out


def pm_to_coord(v: int, r: int) -> int:
    """W1: a per-mille position to a cell coordinate: floor(v (R - 1) / 1000)."""
    return v * (r - 1) // PER_MILLE


def pm_to_cells(v: int, r: int) -> int:
    """W1: a per-mille length to cells: max(1, floor(v R / 1000))."""
    return max(MIN_RADIUS_CELLS, v * r // PER_MILLE)


@dataclass(frozen=True)
class Blob:
    """One drawn blob (W1): centre, signed height, radius and (rings) width, in cells."""

    kind: str
    cx: int
    cy: int
    height: int
    radius: int
    width: int


def _ranged(world_seed: int, b: int, k: int, lo: int, hi: int) -> int:
    """One keyed draw of blob b mapped onto the inclusive integer range [lo, hi]."""
    return lo + pick(draw(world_seed, PURPOSE_BLOB, b, k), hi - lo + 1)


def blobs(tpl: Template, world_seed: int, r: int) -> list[Blob]:
    """W1: every blob of the template, in op order, with its four or five keyed draws."""
    out: list[Blob] = []
    b = 0
    for op in tpl.ops:
        for _ in range(op.count):
            cx = _ranged(world_seed, b, 0, pm_to_coord(op.x[0], r), pm_to_coord(op.x[1], r))
            cy = _ranged(world_seed, b, 1, pm_to_coord(op.y[0], r), pm_to_coord(op.y[1], r))
            h = _ranged(world_seed, b, 2, op.height[0], op.height[1])
            rad = _ranged(world_seed, b, 3, *(pm_to_cells(v, r) for v in op.radius))
            w = 0
            if op.width is not None:
                w = _ranged(world_seed, b, 4, *(pm_to_cells(v, r) for v in op.width))
            out.append(Blob(op.kind, cx, cy, h, rad, w))
            b += 1
    return out


def _contribution(blob: Blob, d2: int) -> int:
    """W2: a blob's signed contribution at squared distance d2 from its centre (0 outside)."""
    mag = abs(blob.height)
    sign = -1 if blob.height < 0 else 1
    if blob.kind == "dome":
        r2 = blob.radius * blob.radius
        return sign * (mag * (r2 - d2) // r2) if d2 < r2 else 0
    q = abs(math.isqrt(d2) - blob.radius)
    return sign * (mag * (blob.width - q) // blob.width) if q < blob.width else 0


def apply_blob(acc: list[int], blob: Blob, r: int) -> None:
    """W2: add one blob to the accumulator over its bounding box."""
    reach = blob.radius + (blob.width if blob.kind == "ring" else 0)
    for y in range(max(0, blob.cy - reach), min(r - 1, blob.cy + reach) + 1):
        dy2 = (y - blob.cy) ** 2
        row = y * r
        for x in range(max(0, blob.cx - reach), min(r - 1, blob.cx + reach) + 1):
            acc[row + x] += _contribution(blob, (x - blob.cx) ** 2 + dy2)


def smooth(e: list[int], r: int) -> list[int]:
    """W4: one 5-point pass, floor((4 c + n + s + e + w) / 8), edge cells using themselves."""
    out = [0] * (r * r)
    for y in range(r):
        row = y * r
        up = (y - 1 if y > 0 else y) * r
        down = (y + 1 if y < r - 1 else y) * r
        for x in range(r):
            left = x - 1 if x > 0 else x
            right = x + 1 if x < r - 1 else x
            s = SMOOTH_CENTRE_WEIGHT * e[row + x] + e[up + x] + e[down + x] + e[row + left]
            out[row + x] = (s + e[row + right]) // SMOOTH_DIVISOR
    return out


def heightmap(tpl: Template, world_seed: int, r: int) -> list[int]:
    """W1-W4: the u16 elevation raster of a template at a seed and resolution."""
    acc = [tpl.base] * (r * r)
    for blob in blobs(tpl, world_seed, r):
        apply_blob(acc, blob, r)
    e = [min(U16_MAX, max(0, v)) for v in acc]
    for _ in range(tpl.smooth_passes):
        e = smooth(e, r)
    return e


def land_cells(e: list[int], sea_level: int) -> list[int]:
    """W5: the land cells (elevation >= sea level), in index order."""
    return [i for i, v in enumerate(e) if v >= sea_level]


def region_seeds(land: list[int], world_seed: int, k: int, r: int) -> list[int]:
    """W6: the first seed by a keyed draw, the rest by farthest-point sampling (ties: low index)."""
    if len(land) < k:
        raise ValueError(f"too few land cells for {k} regions: {len(land)}")
    first = land[pick(draw(world_seed, PURPOSE_REGION_SEED, 0, 0), len(land))]
    seeds = [first]
    dmin = {i: (i % r - first % r) ** 2 + (i // r - first // r) ** 2 for i in land}
    for _ in range(1, k):
        best = max(land, key=lambda i: (dmin[i], -i))
        seeds.append(best)
        bx, by = best % r, best // r
        for i in land:
            d = (i % r - bx) ** 2 + (i // r - by) ** 2
            dmin[i] = min(dmin[i], d)
    return seeds


def _neighbours(i: int, r: int) -> list[int]:
    """The 4-neighbours of a cell, in the fixed order up, left, right, down."""
    x, y = i % r, i // r
    out = []
    if y > 0:
        out.append(i - r)
    if x > 0:
        out.append(i - 1)
    if x < r - 1:
        out.append(i + 1)
    if y < r - 1:
        out.append(i + r)
    return out


def regions(e: list[int], land: list[int], seeds: list[int], r: int) -> list[int]:
    """W7: multi-source Dijkstra over land; priority (dist, cell, region); 0 = sea or unreached."""
    is_land = bytearray(r * r)
    for i in land:
        is_land[i] = 1
    region = [REGION_NONE] * (r * r)
    heap = [(0, s, j + 1) for j, s in enumerate(seeds)]
    heapq.heapify(heap)
    while heap:
        dist, cell, reg = heapq.heappop(heap)
        if region[cell] != REGION_NONE:
            continue
        region[cell] = reg
        for nb in _neighbours(cell, r):
            if is_land[nb] and region[nb] == REGION_NONE:
                cost = 1 + (abs(e[cell] - e[nb]) >> SLOPE_SHIFT)
                heapq.heappush(heap, (dist + cost, nb, reg))
    return region


def seats(region: list[int], k: int, r: int) -> tuple[list[int], list[int]]:
    """W8: per region its land-cell count and seat, the cell nearest the floor centroid."""
    sx, sy, n = [0] * (k + 1), [0] * (k + 1), [0] * (k + 1)
    for i, reg in enumerate(region):
        if reg != REGION_NONE:
            sx[reg] += i % r
            sy[reg] += i // r
            n[reg] += 1
    best: list[tuple[int, int] | None] = [None] * (k + 1)
    for i, reg in enumerate(region):
        if reg == REGION_NONE:
            continue
        cx, cy = sx[reg] // n[reg], sy[reg] // n[reg]
        key = ((i % r - cx) ** 2 + (i // r - cy) ** 2, i)
        if best[reg] is None or key < best[reg]:  # type: ignore[operator]
            best[reg] = key
    seat = [b[1] if b is not None else -1 for b in best[1:]]
    return n[1:], seat


def edges(region: list[int], r: int) -> list[tuple[int, int]]:
    """W9: the region graph: (a, b) with a < b iff a land cell of a is 4-adjacent to one of b."""
    found: set[tuple[int, int]] = set()
    for i, a in enumerate(region):
        if a == REGION_NONE:
            continue
        for nb in (i + 1 if i % r < r - 1 else -1, i + r if i // r < r - 1 else -1):
            if nb >= 0 and region[nb] not in (REGION_NONE, a):
                b = region[nb]
                found.add((min(a, b), max(a, b)))
    return sorted(found)


def routes(edge_list: list[tuple[int, int]], seat: list[int], r: int) -> list[int]:
    """W9: one route per edge, length = isqrt of the squared distance between the two seats."""
    out = []
    for a, b in edge_list:
        sa, sb = seat[a - 1], seat[b - 1]
        out.append(math.isqrt((sa % r - sb % r) ** 2 + (sa // r - sb // r) ** 2))
    return out


@dataclass(frozen=True)
class World:
    """The outputs of worldgen-lite for one (template, seed, resolution, region count)."""

    template: str
    seed: int
    r: int
    k: int
    elevation: list[int]
    land: list[int]
    seeds: list[int]
    region: list[int]
    land_count: list[int]
    seat: list[int]
    edges: list[tuple[int, int]]
    route_length: list[int]


def generate(templates: dict[str, Template], name: str, seed: int, r: int, k: int) -> World:
    """W1-W9 for one case."""
    tpl = templates[name]
    e = heightmap(tpl, seed, r)
    land = land_cells(e, tpl.sea_level)
    sd = region_seeds(land, seed, k, r)
    reg = regions(e, land, sd, r)
    counts, seat = seats(reg, k, r)
    ed = edges(reg, r)
    return World(name, seed, r, k, e, land, sd, reg, counts, seat, ed, routes(ed, seat, r))


# ------------------------------------------------------------------- S3 F0: residence at genesis
TAG_FOUNDER = rng.TAG_FOUNDER


def founder(world_seed: int, k: int) -> int:
    """The PersonId of founder k (rh-ids person/founder/v1; no rehash fires in these cases)."""
    return rng.h64_words(TAG_FOUNDER, [world_seed, k])


def placement(world_seed: int, weights: list[int], households: list[int], loose: int) -> list[int]:
    """R1-R3: the settlement (1-based) of every founder, in founder order.

    ``weights`` are the settlements' land-cell counts in settlement order. Founders are numbered as
    SPR-011 ruling 10 orders them: the heads of the manifest households first (one cast member per
    household here), then each household's other members in household order, then the loose rest.
    A household takes one draw, keyed by its head's PersonId; a loose founder its own.
    """
    cum, total = [], 0
    for w in weights:
        total += w
        cum.append(total)
    if total <= 0:
        raise ValueError("no land cell to place anyone on")

    def settle(entity: int) -> int:
        x = pick(draw(world_seed, PURPOSE_RESIDENCE, entity, 0), total)
        return 1 + next(i for i, c in enumerate(cum) if c > x)

    heads = [settle(founder(world_seed, h)) for h in range(len(households))]
    out = list(heads)
    for h, size in enumerate(households):
        out += [heads[h]] * (size - 1)
    n = len(out)
    out += [settle(founder(world_seed, n + j)) for j in range(loose)]
    return out


@dataclass(order=True)
class Interval:
    """A residence interval (R4): its end is implicit, the next interval's start or the exit."""

    person: int
    settlement: int
    from_tick: int


def birth_sequence() -> list[str]:
    """Check 4: residence at birth copies the mother's; intervals tile [entry, exit) (V-DATA-04)."""
    failures = []
    residence = {1: 3, 2: 5}  # two founders in settlements 3 and 5
    intervals = [Interval(1, 3, 0), Interval(2, 5, 0)]
    exits: dict[int, int] = {}
    births = [(10, 1, 1440 * 30), (11, 2, 1440 * 61), (12, 10, 1440 * 400)]  # (child, mother, tick)
    for child, mother, tick in births:
        if residence.get(mother, REGION_NONE) == REGION_NONE:
            failures.append(f"birth of {child}: mother {mother} has no residence (INV-S3-02)")
            continue
        residence[child] = residence[mother]
        intervals.append(Interval(child, residence[mother], tick))
    exits[1] = 1440 * 500
    entry = {1: 0, 2: 0, 10: 1440 * 30, 11: 1440 * 61, 12: 1440 * 400}
    for person, start in entry.items():
        mine = sorted(iv for iv in intervals if iv.person == person)
        end = exits.get(person, I64_MAX)
        tiled = len(mine) == 1 and mine[0].from_tick == start and start < end
        if not tiled:
            failures.append(f"person {person}: intervals {mine} do not tile [{start}, {end})")
    if residence[12] != residence[1]:
        failures.append("a grandchild does not inherit the lineage's residence at F0")
    return failures


# ----------------------------------------------------------------------------- checks
def check_widths() -> list[str]:
    """Check 3: the widths the spec states."""
    failures = []
    if pick(MASK64, 1 << 32) != (1 << 32) - 1 or pick(0, 7) != 0:
        failures.append("pick does not map the draw extremes to 0 and n - 1")
    worst = 64 * U16_MAX + U16_MAX  # 64 blobs of one op at full height, plus the base
    if worst >= 1 << 31:
        failures.append("the accumulator needs more than i32: the spec says i64, fine, but note it")
    if MAX_STEP_COST != SPEC_MAX_STEP_COST:
        failures.append(
            f"the step cost bound is {MAX_STEP_COST}, the spec says {SPEC_MAX_STEP_COST}"
        )
    if MAX_RESOLUTION * MAX_RESOLUTION * MAX_STEP_COST >= 1 << SPEC_DISTANCE_BITS:
        failures.append(
            "a Dijkstra distance can reach 2^33 at the largest raster; the spec says below"
        )
    return failures


def _connected(cells: list[int], r: int) -> bool:
    """Whether a set of cells is 4-connected."""
    if not cells:
        return False
    members = set(cells)
    seen = {cells[0]}
    stack = [cells[0]]
    while stack:
        c = stack.pop()
        for nb in _neighbours(c, r):
            if nb in members and nb not in seen:
                seen.add(nb)
                stack.append(nb)
    return len(seen) == len(members)


def _check_regions(w: World, by_region: dict[int, list[int]]) -> list[str]:
    """INV-M1-02 and INV-M1-03 per region."""
    failures = []
    tpl = f"{w.template}/{w.seed}/{w.r}"
    for reg in range(1, w.k + 1):
        cells = by_region.get(reg, [])
        if not cells or not _connected(cells, w.r):
            failures.append(f"{tpl}: region {reg} is empty or not 4-connected (INV-M1-02)")
        if w.region[w.seeds[reg - 1]] != reg:
            failures.append(f"{tpl}: seed of region {reg} lies outside it (INV-M1-02)")
        if w.region[w.seat[reg - 1]] != reg or len(cells) != w.land_count[reg - 1]:
            failures.append(f"{tpl}: seat or count of region {reg} is wrong (INV-M1-03)")
    return failures


def check_invariants(w: World) -> list[str]:
    """Check 2: INV-M1-01 to INV-M1-05 on one case."""
    failures = []
    tpl = f"{w.template}/{w.seed}/{w.r}"
    land = set(w.land)
    if any(reg != REGION_NONE and i not in land for i, reg in enumerate(w.region)):
        failures.append(f"{tpl}: a sea cell has a region (INV-M1-01)")
    by_region: dict[int, list[int]] = {}
    for i, reg in enumerate(w.region):
        if reg != REGION_NONE:
            by_region.setdefault(reg, []).append(i)
    failures += _check_regions(w, by_region)
    if len(set(w.seat)) != w.k:
        failures.append(f"{tpl}: two regions share a seat (INV-M1-03)")
    if w.edges != sorted(set(w.edges)) or any(a >= b for a, b in w.edges):
        failures.append(f"{tpl}: the edge list is not sorted, unique and a < b (INV-M1-04)")
    if len(w.route_length) != len(w.edges):
        failures.append(f"{tpl}: routes and edges differ in count (INV-M1-05)")
    return failures


# ----------------------------------------------------------------------------- golden file
CASES: tuple[tuple[str, int, int, int, list[int], int], ...] = (
    # (template, seed, resolution, regions, household sizes, loose founders)
    ("flat", 1, 64, 1, [1], 0),  # solo: Ada alone, one valley
    ("flat", 7, 64, 1, [5], 0),  # household: a couple and three children
    ("desert_fringe", 42, 128, 4, [42], 18),  # hamlet: Job's house of 42, and 18 others
    ("desert_fringe", 7, 128, 4, [3, 4], 10),
    ("archipelago", 7, 96, 6, [2, 2, 2], 12),
    ("mountain_valleys", 7, 128, 5, [6], 30),
    ("caldera_with_passes", 1, 128, 3, [4, 4], 8),
    ("continental_plains", 3, 64, 8, [5, 5, 5, 5], 40),
    ("river_delta_city", 11, 128, 4, [7], 25),
)


def sha(values: list[int]) -> str:
    """SHA-256 of a list of u16 values, little-endian, in index order."""
    return hashlib.sha256(array("H", values).tobytes()).hexdigest()


def render_case(w: World, households: list[int], loose: int) -> list[str]:
    """The golden lines of one case."""
    place = placement(w.seed, w.land_count, households, loose)
    lines = [
        f"case {w.template} {w.seed} {w.r} {w.k}",
        f"elevation_sha256 {sha(w.elevation)}",
        f"land {len(w.land)}",
        "seeds " + " ".join(map(str, w.seeds)),
        f"regions_sha256 {sha(w.region)}",
        "land_cells " + " ".join(map(str, w.land_count)),
        "seats " + " ".join(map(str, w.seat)),
        f"edges {len(w.edges)} " + " ".join(f"{a}-{b}" for a, b in w.edges),
        "routes " + " ".join(map(str, w.route_length)),
        f"residence households={','.join(map(str, households))} loose={loose} "
        + " ".join(map(str, place)),
    ]
    return lines


def bench_fixture(templates: dict[str, Template]) -> list[str]:
    """Check 5: the bench 4 fixture lines (docs/08 §9 bench/raster_8x512)."""
    e = heightmap(templates[BENCH_TEMPLATE], BENCH_SEED, BENCH_RESOLUTION)
    r = BENCH_RESOLUTION
    digest = hashlib.sha256()
    for layer in range(BENCH_LAYERS):
        dx, dy = (BENCH_SHIFT_X * layer) % r, (BENCH_SHIFT_Y * layer) % r
        shifted = [e[((y + dy) % r) * r + (x + dx) % r] for y in range(r) for x in range(r)]
        digest.update(array("H", smooth(shifted, r)).tobytes())
    return [
        f"bench4 {BENCH_TEMPLATE} {BENCH_SEED} {r} layers {BENCH_LAYERS}",
        f"bench4_elevation_sha256 {sha(e)}",
        f"bench4_smoothed_layers_sha256 {digest.hexdigest()}",
    ]


def render(templates: dict[str, Template], version: str) -> tuple[str, list[str]]:
    """The whole golden file and the invariant failures found while generating it."""
    header = [
        "# worldgen-lite v0 golden outputs (SPR-018): GENERATED by",
        "# python -I specs/worldgen/selftest_worldgen_lite.py --write; do not edit.",
        "# Each case: template, seed, raster resolution, region count; SHA-256 of the u16 rasters",
        "# (little-endian, index order); cells as row-major indices; settlements 1-based.",
        f"templates {version}",
    ]
    body, failures = [], []
    for name, seed, r, k, households, loose in CASES:
        w = generate(templates, name, seed, r, k)
        failures += check_invariants(w)
        body += render_case(w, households, loose)
    body += bench_fixture(templates)
    return "\n".join([*header, *body]) + "\n", failures


def templates_version(path: Path) -> str:
    """The version line of the template data file."""
    return str(tomllib.loads(path.read_text(encoding="utf-8"))["version"])


def main(argv: list[str] | None = None) -> int:
    """Check or regenerate the golden file; run the reference's own checks."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--write", action="store_true", help="regenerate the golden file")
    parser.add_argument("--golden", type=Path, default=GOLDEN, help="the golden file to check")
    parser.add_argument("--templates", type=Path, default=TEMPLATES, help="the template data file")
    args = parser.parse_args(argv)
    templates = load_templates(args.templates)
    text, failures = render(templates, templates_version(args.templates))
    failures += check_widths() + birth_sequence()
    if failures:
        for f in failures:
            print(f"REFERENCE FAILED: {f}")
        print("WORLDGEN-LITE: FAIL (the reference's own checks)")
        return EXIT_REFERENCE
    if args.write:
        args.golden.parent.mkdir(parents=True, exist_ok=True)
        args.golden.write_text(text, encoding="utf-8")
        print(f"wrote {args.golden}")
        return EXIT_OK
    current = args.golden.read_text(encoding="utf-8") if args.golden.exists() else ""
    if current != text:
        first = next(
            (
                i
                for i, (a, b) in enumerate(
                    zip(current.splitlines(), text.splitlines(), strict=False)
                )
                if a != b
            ),
            min(len(current.splitlines()), len(text.splitlines())),
        )
        print(f"STALE: {args.golden} differs from the reference at line {first + 1}")
        print("WORLDGEN-LITE: FAIL")
        return EXIT_FAIL
    print(f"WORLDGEN-LITE: PASS ({len(CASES)} cases, {text.count(chr(10))} lines)")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
