"""Exact reference implementation of terrain_v0 voxelization (SPR-033 T-03).

Loads worldgen-lite for `hamlet` (desert_fringe 42 256 4), computes the terrain_v0
columns for 400 SplitMix64-chosen voxel positions, and verifies bit-for-bit against
`specs/viewer/golden/terrain_v0_columns.txt`.

Usage:
  python -I specs/viewer/terrain_ref.py --check
  python -I specs/viewer/terrain_ref.py --write
  python -I specs/viewer/terrain_ref.py --control route-order
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GOLDEN_COLUMNS = HERE / "golden" / "terrain_v0_columns.txt"

EXIT_OK = 0
EXIT_FAIL = 1

# Scale constants from terrain_v0.md:
METRES_PER_CELL = 30
ELEVATION_UNITS_PER_METRE = 64
METRES_PER_VOXEL_NUM = 7
METRES_PER_VOXEL_DEN = 20
VOXELS_PER_CELL_NUM = 600
VOXELS_PER_CELL_DEN = 7
ELEVATION_UNITS_PER_VOXEL_NUM = 2240
ELEVATION_UNITS_PER_VOXEL_DEN = 100
SHELL_DEPTH = 6
PLAZA_SIZE = 34
PLAZA_HALF_EXTENT = 17
ROUTE_WIDTH = 4

CELL_HALF_VOXEL_OFFSET = 300
CLIFF_HEIGHT_DIFF = 3
SAND_BEACH_MAX_ELEVATION_DIFF = 5
CLIFF_SUB_COUNT = 5

# Material palette IDs:
AIR = 0
WATER = 1
SAND = 2
GRASS = 3
DIRT = 4
ROCK = 5
PACKED_EARTH = 6
STONE = 7

# SplitMix64 seed for deterministic column selection:
SAMPLE_SEED = 0x7E22_A100_0033_0001
SAMPLE_COUNT = 400


class SplitMix64:
    """SplitMix64 deterministic pseudo-random number generator."""

    def __init__(self, seed: int) -> None:
        """Initialize generator state from 64-bit seed."""
        self.state = seed & 0xFFFF_FFFF_FFFF_FFFF

    def next_u64(self) -> int:
        """Advance generator state and return next 64-bit unsigned integer."""
        self.state = (self.state + 0x9E37_79B9_7F4A_7C15) & 0xFFFF_FFFF_FFFF_FFFF
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58_476D_1CE4_E5B9) & 0xFFFF_FFFF_FFFF_FFFF
        z = ((z ^ (z >> 27)) * 0x94D0_49BB_1331_11EB) & 0xFFFF_FFFF_FFFF_FFFF
        return (z ^ (z >> 31)) & 0xFFFF_FFFF_FFFF_FFFF

    def below(self, n: int) -> int:
        """Return pseudo-random integer in range [0, n)."""
        return self.next_u64() % n


def _load_worldgen_lite(root: Path) -> ModuleType:
    path = root / "specs" / "worldgen" / "selftest_worldgen_lite.py"
    spec = importlib.util.spec_from_file_location("selftest_worldgen_lite", path)
    if spec is None or spec.loader is None:
        raise SystemExit(f"{path} not found")
    module = importlib.util.module_from_spec(spec)
    sys.modules["selftest_worldgen_lite"] = module
    spec.loader.exec_module(module)
    return module


def interpolate_elevation(raster: list[int], r: int, vx: int, vz: int) -> int:
    """Bilinear interpolation over the 4 nearest cell centres in scaled integer math."""
    vx_scaled = 7 * vx
    vz_scaled = 7 * vz

    # X axis
    if vx_scaled <= CELL_HALF_VOXEL_OFFSET:
        i0, i1 = 0, 0
        wx0, wx1 = 600, 0
    elif vx_scaled >= 600 * (r - 1) + CELL_HALF_VOXEL_OFFSET:
        i0, i1 = r - 1, r - 1
        wx0, wx1 = 600, 0
    else:
        x_prime = vx_scaled - CELL_HALF_VOXEL_OFFSET
        i0 = x_prime // 600
        i1 = i0 + 1
        wx1 = x_prime % 600
        wx0 = 600 - wx1

    # Z axis
    if vz_scaled <= CELL_HALF_VOXEL_OFFSET:
        j0, j1 = 0, 0
        wz0, wz1 = 600, 0
    elif vz_scaled >= 600 * (r - 1) + CELL_HALF_VOXEL_OFFSET:
        j0, j1 = r - 1, r - 1
        wz0, wz1 = 600, 0
    else:
        z_prime = vz_scaled - CELL_HALF_VOXEL_OFFSET
        j0 = z_prime // 600
        j1 = j0 + 1
        wz1 = z_prime % 600
        wz0 = 600 - wz1

    e00 = raster[j0 * r + i0]
    e10 = raster[j0 * r + i1]
    e01 = raster[j1 * r + i0]
    e11 = raster[j1 * r + i1]

    num = e00 * wx0 * wz0 + e10 * wx1 * wz0 + e01 * wx0 * wz1 + e11 * wx1 * wz1
    den = 360_000
    return num // den


def height_from_elevation(e: int) -> int:
    """Integer height in voxels: floor(e * 100 / 2240)."""
    return (e * 100) // 2240


def bresenham_line(x0: int, z0: int, x1: int, z1: int) -> list[tuple[int, int]]:
    """Standard integer 2D Bresenham line algorithm."""
    pts: list[tuple[int, int]] = []
    dx = abs(x1 - x0)
    sx = 1 if x0 < x1 else -1
    dz = -abs(z1 - z0)
    sz = 1 if z0 < z1 else -1
    err = dx + dz
    x, z = x0, z0
    while True:
        pts.append((x, z))
        if x == x1 and z == z1:
            break
        e2 = 2 * err
        if e2 >= dz:
            err += dz
            x += sx
        if e2 <= dx:
            err += dx
            z += sz
    return pts


def choose_sample_columns(
    seats: list[tuple[int, int]],
    edges: list[tuple[int, int]],
    max_voxels: int,
) -> list[tuple[int, int]]:
    """Selects 400 positions deterministically with SplitMix64."""
    rng = SplitMix64(SAMPLE_SEED)
    cols: list[tuple[int, int]] = []

    # 1. 100 across broad terrain
    for _ in range(100):
        cols.append((rng.below(max_voxels), rng.below(max_voxels)))

    # 2. 100 in and around plazas
    for _ in range(100):
        s = seats[rng.below(len(seats))]
        dx = rng.below(51) - 25
        dz = rng.below(51) - 25
        cols.append((s[0] + dx, s[1] + dz))

    # 3. 100 near junction seats where routes meet
    for _ in range(100):
        s = seats[rng.below(len(seats))]
        dx = rng.below(7) - 3
        dz = rng.below(7) - 3
        cols.append((s[0] + dx, s[1] + dz))

    # 4. 100 along route paths
    for _ in range(100):
        e = rng.below(len(edges))
        a, b = edges[e]
        sa, sb = seats[a - 1], seats[b - 1]
        t = rng.below(1000) / 1000.0
        x = int(sa[0] + t * (sb[0] - sa[0])) + (rng.below(9) - 4)
        z = int(sa[1] + t * (sb[1] - sa[1])) + (rng.below(9) - 4)
        cols.append((x, z))

    return cols


@dataclass(frozen=True)
class ColumnData:
    """Evaluated voxel column properties."""

    vx: int
    vz: int
    e: int
    h: int
    surface: int
    stack: list[int]
    flag: str

    def line(self, idx: int) -> str:
        """Format the column as a deterministic text line for comparison."""
        stk_str = ",".join(str(m) for m in self.stack)
        return (
            f"col i={idx} vx={self.vx} vz={self.vz} e={self.e} h={self.h} "
            f"surface={self.surface} stack={stk_str} flag={self.flag}"
        )


@dataclass(frozen=True)
class TerrainContext:
    """World raster and layout context for column evaluation."""

    raster: list[int]
    r: int
    sea_level: int
    max_voxels: int
    plaza_map: dict[tuple[int, int], tuple[int, int]]
    route_map: dict[tuple[int, int], int]


def evaluate_column(ctx: TerrainContext, vx: int, vz: int) -> ColumnData:
    """Computes terrain properties for column (vx, vz) according to terrain_v0."""
    e = interpolate_elevation(ctx.raster, ctx.r, vx, vz)
    h = height_from_elevation(e)
    h_sea = height_from_elevation(ctx.sea_level)

    # 1. Cliff detection against 4-neighbours
    cliff = False
    for dx, dz in ((-1, 0), (1, 0), (0, -1), (0, 1)):
        nb_x = max(0, min(ctx.max_voxels - 1, vx + dx))
        nb_z = max(0, min(ctx.max_voxels - 1, vz + dz))
        nb_e = interpolate_elevation(ctx.raster, ctx.r, nb_x, nb_z)
        nb_h = height_from_elevation(nb_e)
        if h - nb_h > CLIFF_HEIGHT_DIFF:
            cliff = True
            break

    # 2. Base terrain material and shell
    if e < ctx.sea_level or h - h_sea <= SAND_BEACH_MAX_ELEVATION_DIFF:
        surf = SAND
        sub = DIRT
        sub_count = 3
    elif cliff:
        surf = ROCK
        sub = ROCK
        sub_count = CLIFF_SUB_COUNT
    else:
        surf = GRASS
        sub = DIRT
        sub_count = 3

    flag = "none"

    # 3. Plazas: levelled height, STONE surface
    if (vx, vz) in ctx.plaza_map:
        h_plaza, seat_idx = ctx.plaza_map[(vx, vz)]
        h = h_plaza
        surf = STONE
        sub = DIRT
        sub_count = 3
        flag = f"plaza:{seat_idx}"

    # 4. Routes: draped over surface, PACKED_EARTH
    if (vx, vz) in ctx.route_map:
        edge_idx = ctx.route_map[(vx, vz)]
        surf = PACKED_EARTH
        flag = f"route:{edge_idx}"

    # Build 6-voxel solid shell [y=h down to y=h-5]
    if sub == ROCK and sub_count == CLIFF_SUB_COUNT:
        # All rock cliff
        stack = [surf] + [ROCK] * 5
    else:
        # Top voxel, 3 dirt, 2 rock
        stack = [surf, DIRT, DIRT, DIRT, ROCK, ROCK]

    return ColumnData(vx, vz, e, h, surf, stack, flag)


def _build_plaza_map(
    elevation: list[int], r: int, seats: list[tuple[int, int]]
) -> dict[tuple[int, int], tuple[int, int]]:
    """Build coordinate lookup map for plazas."""
    plaza_map: dict[tuple[int, int], tuple[int, int]] = {}
    for seat_idx, (svx, svz) in enumerate(seats):
        e_seat = interpolate_elevation(elevation, r, svx, svz)
        h_plaza = height_from_elevation(e_seat)
        for dx in range(-PLAZA_HALF_EXTENT, PLAZA_HALF_EXTENT):
            for dz in range(-PLAZA_HALF_EXTENT, PLAZA_HALF_EXTENT):
                plaza_map[(svx + dx, svz + dz)] = (h_plaza, seat_idx)
    return plaza_map


def _build_route_map(
    seats: list[tuple[int, int]], edges: list[tuple[int, int]], reverse_routes: bool
) -> dict[tuple[int, int], int]:
    """Build coordinate lookup map for draped routes."""
    edge_list = list(reversed(edges)) if reverse_routes else edges
    route_map: dict[tuple[int, int], int] = {}
    for edge_idx, (a, b) in enumerate(edge_list):
        actual_idx = (len(edges) - 1 - edge_idx) if reverse_routes else edge_idx
        sa = seats[a - 1]
        sb = seats[b - 1]
        dx = abs(sb[0] - sa[0])
        dz = -abs(sb[1] - sa[1])
        line = bresenham_line(sa[0], sa[1], sb[0], sb[1])
        for lx, lz in line:
            if dx >= -dz:
                for k in (-1, 0, 1, 2):
                    route_map[(lx, lz + k)] = actual_idx
            else:
                for k in (-1, 0, 1, 2):
                    route_map[(lx + k, lz)] = actual_idx
    return route_map


def generate_columns(reverse_routes: bool = False) -> list[str]:
    """Generates all 400 sample column lines for hamlet."""
    wl = _load_worldgen_lite(ROOT)
    templates = wl.load_templates(wl.TEMPLATES)
    tpl = templates["desert_fringe"]
    world = wl.generate(templates, "desert_fringe", 42, 256, 4)

    r = world.r
    sea_level = tpl.sea_level
    max_voxels = (600 * r) // 7

    # Calculate seat centres in voxel coordinates
    seats: list[tuple[int, int]] = []
    for s_cell in world.seat:
        scx = s_cell % r
        scz = s_cell // r
        svx = (600 * scx + CELL_HALF_VOXEL_OFFSET) // 7
        svz = (600 * scz + CELL_HALF_VOXEL_OFFSET) // 7
        seats.append((svx, svz))

    plaza_map = _build_plaza_map(world.elevation, r, seats)
    route_map = _build_route_map(seats, world.edges, reverse_routes)
    ctx = TerrainContext(world.elevation, r, sea_level, max_voxels, plaza_map, route_map)

    # Sample the 400 columns
    sample_cols = choose_sample_columns(seats, world.edges, max_voxels)
    lines: list[str] = []
    for idx, (vx, vz) in enumerate(sample_cols):
        col = evaluate_column(ctx, vx, vz)
        lines.append(col.line(idx))

    return lines


def main() -> int:
    """Run reference column generator or golden comparator."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="Write golden columns file")
    parser.add_argument("--check", action="store_true", help="Check against golden columns file")
    parser.add_argument(
        "--control",
        choices=["route-order"],
        help="Run control mutation (must fail check)",
    )
    args = parser.parse_args()

    reverse = args.control == "route-order"
    lines = generate_columns(reverse_routes=reverse)

    if args.write:
        GOLDEN_COLUMNS.parent.mkdir(parents=True, exist_ok=True)
        content = "\n".join(lines) + "\n"
        GOLDEN_COLUMNS.write_text(content, encoding="utf-8")
        print(f"TERRAIN-REF: wrote {len(lines)} columns to {GOLDEN_COLUMNS}")
        return EXIT_OK

    # Default to check
    if not GOLDEN_COLUMNS.exists():
        print(f"FAIL: golden file {GOLDEN_COLUMNS} does not exist", file=sys.stderr)
        return EXIT_FAIL

    golden_lines = [
        ln.strip() for ln in GOLDEN_COLUMNS.read_text(encoding="utf-8").splitlines() if ln.strip()
    ]
    if len(golden_lines) != len(lines):
        msg = f"STALE: count mismatch: golden has {len(golden_lines)}, computed {len(lines)}"
        print(msg, file=sys.stderr)
        return EXIT_FAIL

    for i, (g_ln, c_ln) in enumerate(zip(golden_lines, lines, strict=True)):
        if g_ln != c_ln:
            print(f"STALE: col {i} mismatch:", file=sys.stderr)
            print(f"  golden:   {g_ln}", file=sys.stderr)
            print(f"  computed: {c_ln}", file=sys.stderr)
            return EXIT_FAIL

    print(f"TERRAIN-REF: PASS ({len(lines)} columns verified)")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
