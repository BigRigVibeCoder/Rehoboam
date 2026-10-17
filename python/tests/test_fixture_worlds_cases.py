"""The fixture-worlds reference's hardcoded manifests mirror the example YAML files (SPR-030).

``specs/worldgen/selftest_fixture_worlds.py`` runs under ``python -I`` with the standard library
only, so it cannot parse YAML; its ``TWIN_VALES`` and ``KINGDOM_10K`` restate the fields of
``specs/manifest/examples/twin_vales.yaml`` and ``kingdom_10k.yaml`` that genesis reads, and its
``DORMANCY_DEGREES`` and ``MAJORITY_AGE`` restate the two M7 parameters the naming criterion reads.
This test reads the YAML (the test suite has PyYAML) and compares, so an edit to either file that
the other does not follow turns the suite red. The CRN pairing reference
(``specs/determinism/selftest_crn_pairing.py``) builds the same ``TWIN_VALES``, so it is covered
too.
"""

from __future__ import annotations

import importlib.util
import sys
import tomllib
from pathlib import Path
from types import ModuleType
from typing import Any

import yaml  # type: ignore[import-untyped]  # PyYAML ships no stubs (as test_ci_workflow.py)

REPO = Path(__file__).resolve().parents[2]
REFERENCE = REPO / "specs" / "worldgen" / "selftest_fixture_worlds.py"
EXAMPLES = REPO / "specs" / "manifest" / "examples"
WORLDS = ("twin_vales", "kingdom_10k")


def _reference() -> ModuleType:
    spec = importlib.util.spec_from_file_location("selftest_fixture_worlds", REFERENCE)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules["selftest_fixture_worlds"] = module
    spec.loader.exec_module(module)
    return module


def _docs() -> dict[str, dict[str, Any]]:
    return {
        name: yaml.safe_load((EXAMPLES / f"{name}.yaml").read_text(encoding="utf-8"))
        for name in WORLDS
    }


def _check(case: Any, doc: dict[str, Any], ref: ModuleType) -> None:  # noqa: ANN401
    human = next(p for p in doc["peoples"] if p["archetype"] == "human")
    m2 = doc["systems"]["M2.demography"]
    m7 = doc["systems"]["M7.dynasty"]
    ps = ref.ps
    assert case.id == doc["world"]["id"]
    assert case.seed == doc["world"]["seed"]
    assert case.days_per_year == doc["world"]["calendar"]["days_per_year"]
    assert case.template == doc["geography"]["heightmap_template"]
    assert case.raster == doc["geography"]["raster_resolution"]
    assert case.regions == doc["geography"]["region_count"]
    assert case.population == human["population"]
    assert case.manifest_founders == 0 and "cast" not in doc and "households" not in doc
    assert str(case.e0) == str(m2["e0"])
    assert str(case.tfr) == str(m2["tfr"])
    assert str(case.elite_fertility_mult) == str(m2["elite_fertility_mult"])
    assert str(case.brass_beta) == str(m2.get("brass_beta", ps.BETA_DEFAULT))
    assert case.majority_age == str(m7["majority_age"]) == ref.MAJORITY_AGE
    assert m7["dormancy_degrees"] == ref.DORMANCY_DEGREES
    polities = [
        (p["id"], p["succession_law"], [(d["id"], d["generations"]) for d in p["dynasties"]])
        for p in doc["polities"]
    ]
    mirrored = [
        (p.id, p.succession_law, [(d.id, d.generations) for d in p.dynasties])
        for p in case.polities
    ]
    assert mirrored == polities


def test_fixture_worlds_mirror_their_yaml() -> None:
    """TWIN_VALES and KINGDOM_10K restate twin_vales.yaml and kingdom_10k.yaml field for field."""
    ref = _reference()
    docs = _docs()
    for case in ref.WORLDS:
        _check(case, docs[case.id], ref)
    assert {case.id for case in ref.WORLDS} == set(WORLDS)


def test_fixture_seeds_cover_the_brief() -> None:
    """The fixture is pinned at seeds 1 to 3 and the 10k world at seeds 1 and 2 (the brief)."""
    ref = _reference()
    assert ref.SEEDS["twin_vales"] == (1, 2, 3)
    assert ref.SEEDS["kingdom_10k"] == (1, 2)


def test_both_worlds_set_every_m7_and_t1_parameter() -> None:
    """Neither M7 nor T1 has Registry defaults yet, so both manifests set all of their params."""
    registry = (REPO / "specs" / "manifest" / "registry_v0.toml").read_text(encoding="utf-8")
    params = tomllib.loads(registry)["systems"]
    for doc in _docs().values():
        for system in ("M7.dynasty", "T1.minds"):
            bounded = set(params[system]["params"])
            assert set(doc["systems"][system]) == bounded, system
