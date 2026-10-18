#!/usr/bin/env python3
"""Export worldgen-lite rasters and metadata for the Bevy voxel viewer (SPR-033 T-03).

Exports worldgen-lite elevation and region rasters as little-endian u16 binary files,
accompanied by a TOML metadata file containing seats, edges, routes, scale constants,
and founder placements.

Usage:
  python -I viewer/tools/export_worldgen_lite.py --world hamlet
  python -I viewer/tools/export_worldgen_lite.py --world hamlet --check
  python -I viewer/tools/export_worldgen_lite.py --world hamlet --check --control shifted-seed
  python -I viewer/tools/export_worldgen_lite.py --manifest specs/manifest/examples/twin_vales.yaml --check
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import sys
import tomllib
from array import array
from pathlib import Path
from types import ModuleType

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]

EXIT_OK = 0
EXIT_FAIL = 1

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
            if len(parts) == 2:
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--world", help="World name (e.g. hamlet, solo, household)")
    parser.add_argument("--manifest", type=Path, help="Manifest file path")
    parser.add_argument(
        "--out",
        type=Path,
        default=ROOT / "target" / "viewer" / "exports",
        help="Output directory (default: target/viewer/exports)",
    )
    parser.add_argument("--check", action="store_true", help="Check outputs against golden reference")
    parser.add_argument(
        "--control",
        choices=["shifted-seed"],
        help="Apply control mutation (must fail check)",
    )
    args = parser.parse_args()

    if not args.world and not args.manifest:
        print("Error: either --world <name> or --manifest <path> must be specified", file=sys.stderr)
        return EXIT_FAIL

    # 1. Resolve manifest and verify SHA-256 pin if world.toml exists
    if args.world:
        world_toml_path = ROOT / "worlds" / args.world / "world.toml"
        if not world_toml_path.exists():
            print(f"Error: world config not found: {world_toml_path}", file=sys.stderr)
            return EXIT_FAIL
        w_cfg = tomllib.loads(world_toml_path.read_text(encoding="utf-8"))
        manifest_rel = Path(w_cfg["manifest"])
        manifest_path = ROOT / manifest_rel if not manifest_rel.is_absolute() else manifest_rel
        expected_sha = w_cfg.get("manifest_sha256")
        manifest_bytes = manifest_path.read_bytes()
        actual_sha = hashlib.sha256(manifest_bytes).hexdigest()
        if expected_sha and actual_sha != expected_sha:
            print(
                f"Error: manifest SHA-256 mismatch for world {args.world}:\n"
                f"  expected: {expected_sha}\n  actual:   {actual_sha}",
                file=sys.stderr,
            )
            return EXIT_FAIL
        world_id = args.world
    else:
        manifest_path = args.manifest if args.manifest.is_absolute() else ROOT / args.manifest
        manifest_bytes = manifest_path.read_bytes()
        actual_sha = hashlib.sha256(manifest_bytes).hexdigest()
        world_id = manifest_path.stem

    # 2. Load manifest through manifest_v0
    manifest_mod = _load("manifest_v0", ROOT / "specs" / "manifest" / "manifest_v0.py")
    manifest = manifest_mod.load_manifest(manifest_path)

    geo = manifest.geography
    world_meta = manifest.world

    template_name = geo.heightmap_template
    seed = world_meta.seed
    r = geo.raster_resolution
    k = geo.region_count

    if args.control == "shifted-seed":
        seed += 1

    # 3. Load worldgen-lite reference and generate case
    wl = _load("selftest_worldgen_lite", ROOT / "specs" / "worldgen" / "selftest_worldgen_lite.py")
    templates = wl.load_templates(wl.TEMPLATES)
    if template_name not in templates:
        print(f"Error: template '{template_name}' not found in templates", file=sys.stderr)
        return EXIT_FAIL
    tpl = templates[template_name]

    world = wl.generate(templates, template_name, seed, r, k)
    _, total_pop = extract_population(manifest, wl)
    ref_pops = {
        "solo": wl.SOLO,
        "household": wl.HOUSEHOLD,
        "hamlet": wl.HAMLET,
    }
    if world_id in ref_pops:
        pop = ref_pops[world_id]
    else:
        pop, _ = extract_population(manifest, wl)

    golden_case_key = f"{template_name} {world_meta.seed} {r} {k}"
    case_key = f"{template_name} {seed} {r} {k}"

    # 4. Check mode
    if args.check:
        golden_file = ROOT / "specs" / "worldgen" / "golden" / "worldgen_lite_v0.txt"
        golden_cases = load_golden_cases(golden_file)

        if golden_case_key in golden_cases:
            g_lines = golden_cases[golden_case_key]
            # Compare every golden line
            edges_val = (
                f"{len(world.edges)} " + " ".join(f"{a}-{b}" for a, b in world.edges)
                if world.edges
                else f"{len(world.edges)}"
            )
            checks = [
                ("elevation_sha256", wl.sha(world.elevation)),
                ("land", str(len(world.land))),
                ("seeds", " ".join(map(str, world.seeds))),
                ("regions_sha256", wl.sha(world.region)),
                ("land_cells", " ".join(map(str, world.land_count))),
                ("seats", " ".join(map(str, world.seat))),
                ("edges", edges_val),
                ("routes", " ".join(map(str, world.route_length))),
                (
                    "graph",
                    (
                        f"components {wl.graph_components(world.k, world.edges)} "
                        f"connected {'yes' if wl.graph_components(world.k, world.edges) == 1 else 'no'}"
                    ),
                ),
            ]
            if pop is not None:
                place = wl.placement(world.seed, world.land_count, pop)
                hh_str = ",".join(f"{hid}:{head}:{n}" for hid, head, n in pop.households)
                res_str = (
                    f"cast={len(pop.cast)} households={hh_str} rest={pop.rest} "
                    + " ".join(map(str, place))
                )
                if world_id == "hamlet" or (
                    "residence" in g_lines and g_lines["residence"].startswith(f"cast={len(pop.cast)} ")
                ):
                    checks.append(("residence", res_str))

            for key, computed_val in checks:
                if key not in g_lines:
                    print(f"STALE: missing golden line {key}", file=sys.stderr)
                    return EXIT_FAIL
                if g_lines[key] != computed_val:
                    print(f"STALE: {key} mismatch:", file=sys.stderr)
                    print(f"  golden:   {g_lines[key]}", file=sys.stderr)
                    print(f"  computed: {computed_val}", file=sys.stderr)
                    return EXIT_FAIL

            # Check population matches reference constant for rung worlds
            if world_id == "solo" and total_pop != 1:
                print(f"STALE: solo population {total_pop} != 1", file=sys.stderr)
                return EXIT_FAIL
            if world_id == "household" and total_pop not in (5, 6):
                print(f"STALE: household population {total_pop} not in (5, 6)", file=sys.stderr)
                return EXIT_FAIL
            if world_id == "hamlet" and total_pop != 60:
                print(f"STALE: hamlet population {total_pop} != 60", file=sys.stderr)
                return EXIT_FAIL

            print(f"EXPORT-CHECK: PASS ({world_id} matches golden case {case_key})")
        else:
            # Self-agreement check (e.g. twin_vales)
            world2 = wl.generate(templates, template_name, seed, r, k)
            if wl.sha(world.elevation) != wl.sha(world2.elevation) or wl.sha(world.region) != wl.sha(world2.region):
                print("STALE: self-agreement check failed between repeated runs", file=sys.stderr)
                return EXIT_FAIL
            print(f"EXPORT-CHECK: PASS ({world_id} self-agreement verified for case {case_key})")

    # 5. Write export files
    out_dir = args.out
    out_dir.mkdir(parents=True, exist_ok=True)

    toml_path = out_dir / f"{world_id}.toml"
    elev_path = out_dir / f"{world_id}_elevation.bin"
    reg_path = out_dir / f"{world_id}_regions.bin"

    # Write little-endian u16 binary rasters
    elev_path.write_bytes(array("H", world.elevation).tobytes())
    reg_path.write_bytes(array("H", world.region).tobytes())

    # Build TOML metadata
    edges_str = ", ".join(f"[{a}, {b}]" for a, b in world.edges)
    seats_str = ", ".join(str(s) for s in world.seat)
    routes_str = ", ".join(str(r_len) for r_len in world.route_length)

    toml_lines = [
        'format = "rehoboam-worldgen-lite-export-v0"',
        f'templates_version = "{templates.get("version", "worldgen-lite-templates-v0")}"',
        f'case = "{case_key}"',
        f'world = "{world_id}"',
        f'manifest_path = "{manifest_path}"',
        f'manifest_sha256 = "{actual_sha}"',
        f"raster_resolution = {r}",
        f"sea_level = {tpl.sea_level}",
        f"land_count = {len(world.land)}",
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

    if pop is not None:
        place = wl.placement(world.seed, world.land_count, pop)
        toml_lines.append("")
        for k_idx, settlement in enumerate(place):
            pid = wl.founder(world.seed, k_idx)
            toml_lines.extend([
                "[[founders]]",
                f"person_id = {pid}",
                f"settlement = {settlement}",
            ])

    toml_content = "\n".join(toml_lines) + "\n"
    toml_path.write_text(toml_content, encoding="utf-8")

    print(f"EXPORT: wrote {toml_path}, {elev_path}, {reg_path}")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
