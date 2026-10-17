"""The exact reference's hardcoded manifests mirror the example YAML files (SPR-019).

``specs/worldgen/selftest_population_synthesis.py`` runs under ``python -I`` with the standard
library only, so it cannot parse YAML; its ``HAMLET`` and ``MANOR`` cases restate the fields of
``specs/manifest/examples/hamlet.yaml`` and ``manor.yaml`` it reads. This test reads the YAML (the
test suite has PyYAML) and compares, so an edit to either file that the other does not follow turns
the suite red.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from types import ModuleType
from typing import Any

import yaml  # type: ignore[import-untyped]  # PyYAML ships no stubs (as test_ci_workflow.py)

REPO = Path(__file__).resolve().parents[2]
REFERENCE = REPO / "specs" / "worldgen" / "selftest_population_synthesis.py"
EXAMPLES = REPO / "specs" / "manifest" / "examples"


def _reference() -> ModuleType:
    spec = importlib.util.spec_from_file_location("selftest_population_synthesis", REFERENCE)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules["selftest_population_synthesis"] = module
    spec.loader.exec_module(module)
    return module


def _manifest_founders(doc: dict[str, Any]) -> int:
    households = doc.get("households", [])
    members = 0
    for h in households:
        children = h.get("children", {})
        members += int(children.get("sons", 0)) + int(children.get("daughters", 0))
        members += int(h.get("servants", 0))
    return len(doc.get("cast", [])) + members


def _check(case: Any, doc: dict[str, Any], ref: ModuleType) -> None:  # noqa: ANN401
    human = next(p for p in doc["peoples"] if p["archetype"] == "human")
    m2 = doc.get("systems", {}).get("M2.demography", {})
    assert case.id == doc["world"]["id"]
    assert case.seed == doc["world"]["seed"]
    assert case.days_per_year == doc["world"]["calendar"]["days_per_year"]
    assert case.template == doc["geography"]["heightmap_template"]
    assert case.raster == doc["geography"]["raster_resolution"]
    assert case.regions == doc["geography"]["region_count"]
    assert case.population == human["population"]
    assert case.manifest_founders == _manifest_founders(doc)
    assert str(case.e0) == str(m2.get("e0", ref.E0_DEFAULT))
    assert str(case.tfr) == str(m2.get("tfr", ref.TFR_DEFAULT))
    mult = m2.get("elite_fertility_mult", ref.MULT_DEFAULT)
    assert str(case.elite_fertility_mult) == str(mult)
    assert str(case.brass_beta) == str(m2.get("brass_beta", ref.BETA_DEFAULT))
    m7 = doc.get("systems", {}).get("M7.dynasty")
    majority = str(m7["majority_age"]) if m7 and "majority_age" in m7 else None
    assert case.majority_age == majority
    polities = [
        (
            p["id"],
            p["succession_law"],
            [(d["id"], d.get("generations", 3)) for d in p.get("dynasties", [])],
        )
        for p in doc.get("polities", [])
    ]
    mirrored = [
        (p.id, p.succession_law, [(d.id, d.generations) for d in p.dynasties])
        for p in case.polities
    ]
    assert mirrored == polities


def test_hamlet_and_manor_mirror_their_yaml() -> None:
    """HAMLET and MANOR restate hamlet.yaml and manor.yaml field for field."""
    ref = _reference()
    for case, name in ((ref.HAMLET, "hamlet"), (ref.MANOR, "manor")):
        doc = yaml.safe_load((EXAMPLES / f"{name}.yaml").read_text(encoding="utf-8"))
        _check(case, doc, ref)


def test_cases_cover_two_manifests_and_two_seeds() -> None:
    """The golden cases cover at least two manifests at two seeds each (the brief's minimum)."""
    ref = _reference()
    seeds: dict[str, set[int]] = {}
    for case in ref.CASES:
        seeds.setdefault(case.id, set()).add(case.seed)
    assert len(seeds) >= 2
    assert all(len(s) >= 2 for s in seeds.values())
