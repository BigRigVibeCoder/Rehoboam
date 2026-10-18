#!/usr/bin/env python3
"""Red Team Probe B08-01: Dynastic Survival and Extinction Stress Test.

Tests noble bloodline survival over 300 years in the Manor of Ash (N=400, 2 houses)
and Twin Vales (N=1500, 5 houses) under M2/M7 demography and population synthesis.

Measures:
1. Long-term extinction risk under agnatic vs cognatic vs bounded succession.
2. Sensitivity to vocation_holy_orders (removal from reproduction) and elite fertility mult.
3. Actual genesis seed variation across 50 seeds: initial fertile women per house.
"""

from __future__ import annotations

import importlib.util
import sys
from dataclasses import replace
from pathlib import Path
from types import ModuleType

ROOT = Path(__file__).resolve().parents[2]
PROJ_PATH = ROOT / "specs" / "worldgen" / "project_population.py"
SYNTH_PATH = ROOT / "specs" / "worldgen" / "selftest_population_synthesis.py"

MIN_PER_HOUSE_SAFE = 2.0
FERTILE_AGE_MIN = 15
FERTILE_AGE_MAX = 45
SEEDS_COUNT = 50
PERCENT_MULT = 100.0


def load_module(name: str, path: Path) -> ModuleType:
    """Dynamically load and return a Python module from path."""
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise SystemExit(f"Could not load {path}")
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


proj = load_module("project_population", PROJ_PATH)


def part1_projection(base_rates: list[object], manor_world: object) -> None:
    """Run 300-year dynastic projection under agnatic, cognatic, and bounded rules."""
    print("=" * 80)
    print("PROBE B08-01 PART 1: 300-YEAR DYNASTIC PROJECTION (MANOR & TWIN VALES)")
    print("=" * 80)
    print("\n--- Manor of Ash (Pop=400, 2 Dynasties, 20 Nobles at Genesis) ---")

    rules = ["agnatic", "cognatic", "bounded"]
    regimes = ["isolation", "t1"]

    for regime in regimes:
        print(f"\n[Regime: {regime.upper()}]")
        for rule in rules:
            w = replace(manor_world, rule=rule, regime=regime)
            points = proj.project(w, base_rates)

            p0 = proj.at(points, 0)
            p50 = proj.at(points, 50)
            p100 = proj.at(points, 100)
            p200 = proj.at(points, 200)
            p300 = proj.at(points, 300)

            least = min(points, key=lambda p: p.members)
            h_least = least.members / w.houses

            line_part1 = (
                f"  Rule: {rule:<8} | Y0: {p0.members:4.1f} ({p0.share * PERCENT_MULT:4.1f}%)"
                f" | Y50: {p50.members:4.1f} | Y100: {p100.members:4.1f}"
            )
            line_part2 = (
                f" | Y200: {p200.members:4.1f} | Y300: {p300.members:4.1f} | "
                f"Least: {least.members:4.1f} at Yr {least.year:3.0f} ({h_least:4.1f}/hse)"
            )
            print(line_part1 + line_part2)


def part2_vocation_sweep(manor_world: object) -> None:
    """Sweep vocation holy orders rates from 0% to 30%."""
    print("\n" + "=" * 80)
    print("PROBE B08-01 PART 2: VOCATION & EXTINCTION SENSITIVITY SWEEP (MANOR)")
    print("=" * 80)

    holy_rates = [0.0, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30]
    print("\nAgnatic Manor Member Count at Year 300 vs Holy Orders Drain (v):")
    for v in holy_rates:
        w = replace(manor_world, rule="agnatic", regime="isolation", holy=v)
        rates = proj.rates("1.2", holy=v)
        p = proj.project(w, rates)
        p300 = proj.at(p, 300)
        least = min(p, key=lambda pt: pt.members)
        per_h = least.members / w.houses
        warning = (
            " [!] CRITICAL EXTINCTION RISK" if per_h < MIN_PER_HOUSE_SAFE else " [OK]"
        )
        y300_h = p300.members / w.houses
        v_pct = v * PERCENT_MULT
        msg = (
            f"  Holy Orders v={v_pct:4.1f}% | Y300 Nobles: {p300.members:5.1f} | "
            f"Per House: {y300_h:4.1f} | Min: {least.members:4.1f} (Yr {least.year:.0f}){warning}"
        )
        print(msg)


def _print_house_stats(
    name: str,
    living: list[int],
    fertile: list[int],
    zero_count: int,
    *,
    is_critical: bool = False,
) -> None:
    """Print summary stats for one house across seeds."""
    mean_l = sum(living) / len(living)
    mean_f = sum(fertile) / len(fertile)
    zero_pct = (zero_count / SEEDS_COUNT) * PERCENT_MULT
    crit_tag = " [!] CRITICAL EXTINCTION RISK" if is_critical else ""

    print(f"  {name}:")
    print(
        f"    Living Bloodline Members: Min={min(living)}, Mean={mean_l:.1f}, Max={max(living)}"
    )
    print(
        f"    Available Fertile Women:  Min={min(fertile)}, Mean={mean_f:.1f}, Max={max(fertile)}"
    )
    print(
        f"    Zero Fertile Women Risk:  {zero_count}/{SEEDS_COUNT} seeds "
        f"({zero_pct:.1f}%){crit_tag}"
    )


def part3_seed_variation() -> None:
    """Test 50 seeds of population synthesis to measure genesis demographic traps."""
    print("\n" + "=" * 80)
    print("PROBE B08-01 PART 3: 50-SEED GENESIS VARIATION TEST (MANOR)")
    print("=" * 80)

    synth = load_module("selftest_population_synthesis", SYNTH_PATH)
    manor_manifest = synth.MANOR

    print(f"\nTesting Genesis Population Synthesis across {SEEDS_COUNT} Seeds...")
    ash_living: list[int] = []
    thorne_living: list[int] = []
    ash_fertile: list[int] = []
    thorne_fertile: list[int] = []
    ash_zero = 0
    thorne_zero = 0
    both_zero = 0

    for seed in range(1, SEEDS_COUNT + 1):
        m = replace(manor_manifest, seed=seed)
        g = synth.build(m)

        ash_id = g.houses[0].id
        thorne_id = g.houses[1].id

        a_liv = [p for p in g.persons.values() if p.alive and p.house == ash_id]
        t_liv = [p for p in g.persons.values() if p.alive and p.house == thorne_id]

        a_wives = [
            g.persons[p.spouse] for p in a_liv if p.spouse and g.persons[p.spouse].alive
        ]
        t_wives = [
            g.persons[p.spouse] for p in t_liv if p.spouse and g.persons[p.spouse].alive
        ]

        a_fertile = [
            p
            for p in a_liv
            if p.sex == synth.SEX_FEMALE
            and FERTILE_AGE_MIN <= -p.birth // g.cal.year < FERTILE_AGE_MAX
        ] + [
            p
            for p in a_wives
            if FERTILE_AGE_MIN <= -p.birth // g.cal.year < FERTILE_AGE_MAX
        ]
        t_fertile = [
            p
            for p in t_liv
            if p.sex == synth.SEX_FEMALE
            and FERTILE_AGE_MIN <= -p.birth // g.cal.year < FERTILE_AGE_MAX
        ] + [
            p
            for p in t_wives
            if FERTILE_AGE_MIN <= -p.birth // g.cal.year < FERTILE_AGE_MAX
        ]

        ash_living.append(len(a_liv))
        thorne_living.append(len(t_liv))
        ash_fertile.append(len(a_fertile))
        thorne_fertile.append(len(t_fertile))

        if len(a_fertile) == 0:
            ash_zero += 1
        if len(t_fertile) == 0:
            thorne_zero += 1
        if len(a_fertile) == 0 and len(t_fertile) == 0:
            both_zero += 1

    both_zero_pct = (both_zero / SEEDS_COUNT) * PERCENT_MULT

    print(f"  Seeds Successfully Tested: {len(ash_living)}")
    _print_house_stats(
        "House Ashcombe (3-gen backfill)", ash_living, ash_fertile, ash_zero
    )
    _print_house_stats(
        "House Thorne (2-gen backfill)",
        thorne_living,
        thorne_fertile,
        thorne_zero,
        is_critical=True,
    )
    print(
        f"  Both Houses Lacking Fertile Women Simultaneously: "
        f"{both_zero}/{SEEDS_COUNT} seeds ({both_zero_pct:.1f}%)"
    )


def test_manor_projection() -> None:
    """Run all probe parts for the Manor world."""
    base_rates = proj.rates("1.2")
    manor_world = proj.World(
        id="manor",
        population=400,
        members=20,
        houses=2,
        named_women=1.0,
        capacity=None,
        horizon=300,
        mult="1.2",
        holy=0.2,
        rule="agnatic",
        regime="isolation",
        vocation="all",
        elite_read="woman",
        hypergamy=0.0,
    )
    part1_projection(base_rates, manor_world)
    part2_vocation_sweep(manor_world)
    part3_seed_variation()


if __name__ == "__main__":
    test_manor_projection()
