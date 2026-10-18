"""Planted Registries of envelope_v0.md §8 through lint_patterns.py (SPR-020 T-01).

REV-SPR-020 R12 NIT-4 and R22 NIT-3.
Each plant in ``tests/lint_fixtures/registry_envelope/plants.toml`` is one text edit of the real
Registry (``specs/manifest/registry_v0.toml``). rh-manifest's ``Registry::problems`` refuses every
plant (its Rust test, ``crates/rh-manifest/tests/registry_envelope.rs``); this gate checks that
``specs/sifting/lint_patterns.py``, which reads the same ``[event_roles]`` and ``[role_domains]``
sections, refuses every plant that carries a ``python`` message, with that message, and that it
accepts the real Registry. A plant whose ``find`` text does not occur exactly once is an error of
the fixture, not a pass.

Usage:  python -I specs/manifest/registry_envelope_plants.py
Exit:   0 every plant refused by name and the real Registry accepted; 1 otherwise.
Standard library only (GOV-003 §3).
"""

import importlib.util
import sys
import tomllib
from pathlib import Path
from types import ModuleType

ROOT = Path(__file__).resolve().parents[2]
REGISTRY = ROOT / "specs/manifest/registry_v0.toml"
PLANTS = ROOT / "tests/lint_fixtures/registry_envelope/plants.toml"
PATTERNS = ROOT / "specs/sifting/patterns.example.toml"
LINT = ROOT / "specs/sifting/lint_patterns.py"


def _lint_module() -> ModuleType:
    spec = importlib.util.spec_from_file_location("lint_patterns", LINT)
    if spec is None or spec.loader is None:
        raise SystemExit(f"cannot load {LINT}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> int:
    """Run every plant with a ``python`` message through lint_patterns.lint."""
    lint = _lint_module().lint
    text = REGISTRY.read_text(encoding="utf-8")
    patterns = tomllib.loads(PATTERNS.read_text(encoding="utf-8"))
    plants = tomllib.loads(PLANTS.read_text(encoding="utf-8"))["plant"]
    failures = []
    if lint(patterns, tomllib.loads(text)):
        failures.append("the real Registry is refused")
    checked = 0
    for p in plants:
        if "python" not in p:
            continue
        if text.count(p["find"]) != 1:
            failures.append(f"{p['name']}: the edit's text does not occur exactly once")
            continue
        errs = lint(patterns, tomllib.loads(text.replace(p["find"], p["replace"], 1)))
        if not any(p["python"] in e for e in errs):
            failures.append(f"{p['name']}: not refused with {p['python']!r}: {errs[:3]}")
            continue
        checked += 1
        print(f"refused {p['name']}: {next(e for e in errs if p['python'] in e)}")
    for f in failures:
        print(f"FAIL: {f}")
    if failures:
        return 1
    print(f"OK: {checked} planted Registries refused by lint_patterns by name; the Registry passes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
