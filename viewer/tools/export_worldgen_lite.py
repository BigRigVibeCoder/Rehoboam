#!/usr/bin/env python3
"""Export worldgen-lite rasters and metadata for the Bevy voxel viewer (SPR-033 T-03).

Exports worldgen-lite elevation and region rasters as little-endian u16 binary files,
accompanied by a TOML metadata file containing seats, edges, routes, scale constants,
and founder placements.

Usage:
  python -I viewer/tools/export_worldgen_lite.py --world hamlet
  python -I viewer/tools/export_worldgen_lite.py --world hamlet --check
  python -I viewer/tools/export_worldgen_lite.py --world hamlet --check --control shifted-seed
  python -I viewer/tools/export_worldgen_lite.py \
    --manifest specs/manifest/examples/twin_vales.yaml --check
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import sys
import tomllib
from array import array
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]

EXIT_OK = 0
EXIT_FAIL = 1

SPLIT_PARTS = 2
SOLO_POPULATION = 1
HAMLET_POPULATION = 60

SCALE_CONSTANTS = {
    "metres_per_cell": 30,
    "elevation_units_per_metre": 64,
    "metres_per_voxel_num": 7,
    "metres_per_voxel_den": 20,
    "voxels_per_cell_num": 600,
    "voxels_per_cell_den": 7,
    "elevation_units_per_voxel_num": 2240,
    "elevation_units_per_voxel_den": 100,
}


@dataclass(frozen=True)
class ExportContext:
    """Context holding manifest, world, and generator parameters."""

    world_id: str
    manifest_path: Path
    actual_sha: str
    template_name: str
    seed: int
    r: int
    k: int
    world: object
    pop: object | None
    total_pop: int
    golden_case_key: str
    case_key: str


def _load(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SystemExit(f"{path} not found")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def load_golden_cases(golden_path: Path) -> dict[str, dict[str, str]]:
    """Parses specs/worldgen/golden/worldgen_lite_v0.txt into case records."""
    cases: dict[str, dict[str, str]] = {}
    current_case: str | None = None
    lines: dict[str, str] = {}

    for raw_line in golden_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("case "):
            if current_case is not None:
                cases[current_case] = lines
            current_case = line[5:].strip()
            lines = {}
        elif current_case is not None:
            parts = line.split(maxsplit=1)
            if len(parts) == SPLIT_PARTS:
                lines[parts[0]] = parts[1]
            elif len(parts) == 1:
                lines[parts[0]] = ""

    if current_case is not None:
        cases[current_case] = lines

    return cases


def extract_population(manifest: object, wl: ModuleType) -> tuple[object | None, int]:
    """Builds a Population object from a manifest when population is defined."""
    peoples = getattr(manifest, "peoples", None)
    if not peoples:
        return None, 0
    total_pop = sum(getattr(p, "population", 0) for p in peoples)

    cast = getattr(manifest, "cast", [])
    households = getattr(manifest, "households", [])

    if not cast and not households:
        return None, total_pop

    cast_tuple = tuple(getattr(c, "household", None) for c in cast)

    hh_list: list[tuple[str, int, int]] = []
    for h in households:
        hid = h.id
        head_id = h.head
        try:
            head_idx = next(i for i, c in enumerate(cast) if c.id == head_id)
        except StopIteration:
            head_idx = 0
        children = h.children
        sons = children.sons if children else 0
        daughters = children.daughters if children else 0
        servants = h.servants
        hh_list.append((hid, head_idx, sons + daughters + servants))

    named_count = len(cast) + sum(m for _, _, m in hh_list)
    rest = max(0, total_pop - named_count)

    pop = wl.Population(cast=cast_tuple, households=tuple(hh_list), rest=rest)
    return pop, total_pop


def _resolve_manifest(
    world_arg: str | None, manifest_arg: Path | None
) -> tuple[Path, str, str] | None:
    """Resolve manifest path, SHA-256 hash, and world ID."""
    if world_arg:
        world_toml_path = ROOT / "worlds" / world_arg / "world.toml"
        if not world_toml_path.exists():
            print(f"Error: world config not found: {world_toml_path}", file=sys.stderr)
            return None
        w_cfg = tomllib.loads(world_toml_path.read_text(encoding="utf-8"))
        manifest_rel = Path(w_cfg["manifest"])
        manifest_path = ROOT / manifest_rel if not manifest_rel.is_absolute() else manifest_rel
        expected_sha = w_cfg.get("manifest_sha256")
        manifest_bytes = manifest_path.read_bytes()
        actual_sha = hashlib.sha256(manifest_bytes).hexdigest()
        if expected_sha and actual_sha != expected_sha:
            print(
                f"Error: manifest SHA-256 mismatch for world {world_arg}:\n"
                f"  expected: {expected_sha}\n  actual:   {actual_sha}",
                file=sys.stderr,
            )
            return None
        return manifest_path, actual_sha, world_arg

    if manifest_arg:
        manifest_path = manifest_arg if manifest_arg.is_absolute() else ROOT / manifest_arg
        manifest_bytes = manifest_path.read_bytes()
        actual_sha = hashlib.sha256(manifest_bytes).hexdigest()
        return manifest_path, actual_sha, manifest_path.stem

    return None


def _check_golden_lines(
    ctx: ExportContext,
    g_lines: dict[str, str],
    wl: ModuleType,
) -> int:
    """Check computed worldgen results against golden case expectations."""
    edges_val = (
        f"{len(ctx.world.edges)} " + " ".join(f"{a}-{b}" for a, b in ctx.world.edges)
        if ctx.world.edges
        else f"{len(ctx.world.edges)}"
    )
    graph_comp = wl.graph_components(ctx.world.k, ctx.world.edges)
    connected_str = "yes" if graph_comp == 1 else "no"
    checks = [
        ("elevation_sha256", wl.sha(ctx.world.elevation)),
        ("land", str(len(ctx.world.land))),
        ("seeds", " ".join(map(str, ctx.world.seeds))),
        ("regions_sha256", wl.sha(ctx.world.region)),
        ("land_cells", " ".join(map(str, ctx.world.land_count))),
        ("seats", " ".join(map(str, ctx.world.seat))),
        ("edges", edges_val),
        ("routes", " ".join(map(str, ctx.world.route_length))),
        ("graph", f"components {graph_comp} connected {connected_str}"),
    ]
    if ctx.pop is not None:
        place = wl.placement(ctx.world.seed, ctx.world.land_count, ctx.pop)
        hh_str = ",".join(f"{hid}:{head}:{n}" for hid, head, n in ctx.pop.households)
        res_str = f"cast={len(ctx.pop.cast)} households={hh_str} rest={ctx.pop.rest} " + " ".join(
            map(str, place)
        )
        if ctx.world_id == "hamlet" or (
            "residence" in g_lines and g_lines["residence"].startswith(f"cast={len(ctx.pop.cast)} ")
        ):
            checks.append(("residence", res_str))

    for key, computed_val in checks:
        if key not in g_lines:
            print(f"STALE: missing golden line {key}", file=sys.stderr)
            return EXIT_FAIL
        if g_lines[key] != computed_val:
            print(
                f"STALE: {key} mismatch:\n  golden:   {g_lines[key]}\n  computed: {computed_val}",
                file=sys.stderr,
            )
            return EXIT_FAIL

    if ctx.world_id == "solo" and ctx.total_pop != SOLO_POPULATION:
        print(f"STALE: solo population {ctx.total_pop} != {SOLO_POPULATION}", file=sys.stderr)
        return EXIT_FAIL
    if ctx.world_id == "household" and ctx.total_pop not in (5, 6):
        print(f"STALE: household population {ctx.total_pop} not in (5, 6)", file=sys.stderr)
        return EXIT_FAIL
    if ctx.world_id == "hamlet" and ctx.total_pop != HAMLET_POPULATION:
        print(f"STALE: hamlet population {ctx.total_pop} != {HAMLET_POPULATION}", file=sys.stderr)
        return EXIT_FAIL

    print(f"EXPORT-CHECK: PASS ({ctx.world_id} matches golden case {ctx.case_key})")
    return EXIT_OK


def _verify_export(
    ctx: ExportContext,
    wl: ModuleType,
    templates: dict[str, object],
) -> int:
    """Verify export against golden reference file or self-agreement."""
    golden_file = ROOT / "specs" / "worldgen" / "golden" / "worldgen_lite_v0.txt"
    golden_cases = load_golden_cases(golden_file)

    if ctx.golden_case_key in golden_cases:
        return _check_golden_lines(ctx, golden_cases[ctx.golden_case_key], wl)

    # Self-agreement check
    world2 = wl.generate(templates, ctx.template_name, ctx.seed, ctx.r, ctx.k)
    if wl.sha(ctx.world.elevation) != wl.sha(world2.elevation) or wl.sha(
        ctx.world.region
    ) != wl.sha(world2.region):
        print("STALE: self-agreement check failed between repeated runs", file=sys.stderr)
        return EXIT_FAIL
    print(f"EXPORT-CHECK: PASS ({ctx.world_id} self-agreement verified for case {ctx.case_key})")
    return EXIT_OK


def _write_export(
    ctx: ExportContext,
    out_dir: Path,
    sea_level: int,
    templates_version: str,
    wl: ModuleType,
) -> None:
    """Write binary rasters and TOML metadata to out_dir."""
    out_dir.mkdir(parents=True, exist_ok=True)
    toml_path = out_dir / f"{ctx.world_id}.toml"
    elev_path = out_dir / f"{ctx.world_id}_elevation.bin"
    reg_path = out_dir / f"{ctx.world_id}_regions.bin"

    elev_path.write_bytes(array("H", ctx.world.elevation).tobytes())
    reg_path.write_bytes(array("H", ctx.world.region).tobytes())

    edges_str = ", ".join(f"[{a}, {b}]" for a, b in ctx.world.edges)
    seats_str = ", ".join(str(s) for s in ctx.world.seat)
    routes_str = ", ".join(str(r_len) for r_len in ctx.world.route_length)

    toml_lines = [
        'format = "rehoboam-worldgen-lite-export-v0"',
        f'templates_version = "{templates_version}"',
        f'case = "{ctx.case_key}"',
        f'world = "{ctx.world_id}"',
        f'manifest_path = "{ctx.manifest_path}"',
        f'manifest_sha256 = "{ctx.actual_sha}"',
        f"raster_resolution = {ctx.r}",
        f"sea_level = {sea_level}",
        f"land_count = {len(ctx.world.land)}",
        f"seats = [{seats_str}]",
        f"edges = [{edges_str}]",
        f"routes = [{routes_str}]",
        "",
        "[scale]",
        f"metres_per_cell = {SCALE_CONSTANTS['metres_per_cell']}",
        f"elevation_units_per_metre = {SCALE_CONSTANTS['elevation_units_per_metre']}",
        f"metres_per_voxel_num = {SCALE_CONSTANTS['metres_per_voxel_num']}",
        f"metres_per_voxel_den = {SCALE_CONSTANTS['metres_per_voxel_den']}",
        f"voxels_per_cell_num = {SCALE_CONSTANTS['voxels_per_cell_num']}",
        f"voxels_per_cell_den = {SCALE_CONSTANTS['voxels_per_cell_den']}",
        f"elevation_units_per_voxel_num = {SCALE_CONSTANTS['elevation_units_per_voxel_num']}",
        f"elevation_units_per_voxel_den = {SCALE_CONSTANTS['elevation_units_per_voxel_den']}",
    ]

    if ctx.pop is not None:
        place = wl.placement(ctx.world.seed, ctx.world.land_count, ctx.pop)
        toml_lines.append("")
        for k_idx, settlement in enumerate(place):
            pid = wl.founder(ctx.world.seed, k_idx)
            toml_lines.extend(
                [
                    "[[founders]]",
                    f"person_id = {pid}",
                    f"settlement = {settlement}",
                ]
            )

    toml_path.write_text("\n".join(toml_lines) + "\n", encoding="utf-8")
    print(f"EXPORT: wrote {toml_path}, {elev_path}, {reg_path}")


def main() -> int:
    """Run worldgen-lite export tool with parsed CLI arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--world", help="World name (e.g. hamlet, solo, household)")
    parser.add_argument("--manifest", type=Path, help="Manifest file path")
    parser.add_argument(
        "--out",
        type=Path,
        default=ROOT / "target" / "viewer" / "exports",
        help="Output directory (default: target/viewer/exports)",
    )
    parser.add_argument(
        "--check", action="store_true", help="Check outputs against golden reference"
    )
    parser.add_argument(
        "--control",
        choices=["shifted-seed"],
        help="Apply control mutation (must fail check)",
    )
    args = parser.parse_args()

    resolved = _resolve_manifest(args.world, args.manifest)
    if resolved is None:
        print("Error: specify valid --world <name> or --manifest <path>", file=sys.stderr)
        return EXIT_FAIL

    manifest_path, actual_sha, world_id = resolved
    manifest_mod = _load("manifest_v0", ROOT / "specs" / "manifest" / "manifest_v0.py")
    manifest = manifest_mod.load_manifest(manifest_path)

    geo = manifest.geography
    world_meta = manifest.world
    template_name = geo.heightmap_template
    seed = world_meta.seed + (1 if args.control == "shifted-seed" else 0)
    r = geo.raster_resolution
    k = geo.region_count

    wl = _load("selftest_worldgen_lite", ROOT / "specs" / "worldgen" / "selftest_worldgen_lite.py")
    templates = wl.load_templates(wl.TEMPLATES)
    if template_name not in templates:
        print(f"Error: template '{template_name}' not found", file=sys.stderr)
        return EXIT_FAIL
    tpl = templates[template_name]

    world = wl.generate(templates, template_name, seed, r, k)
    _, total_pop = extract_population(manifest, wl)
    ref_pops = {"solo": wl.SOLO, "household": wl.HOUSEHOLD, "hamlet": wl.HAMLET}
    pop = ref_pops.get(world_id) or extract_population(manifest, wl)[0]

    golden_case_key = f"{template_name} {world_meta.seed} {r} {k}"
    case_key = f"{template_name} {seed} {r} {k}"

    ctx = ExportContext(
        world_id=world_id,
        manifest_path=manifest_path,
        actual_sha=actual_sha,
        template_name=template_name,
        seed=seed,
        r=r,
        k=k,
        world=world,
        pop=pop,
        total_pop=total_pop,
        golden_case_key=golden_case_key,
        case_key=case_key,
    )

    if args.check:
        res = _verify_export(ctx, wl, templates)
        if res != EXIT_OK:
            return res

    tpl_ver = str(templates.get("version", "worldgen-lite-templates-v0"))
    _write_export(ctx, args.out, tpl.sea_level, tpl_ver, wl)
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
