"""lint_odd.py's frozen-default self-check probe (REV-SPR-015 R6; the re-check's R3, 2026-10-18).

The ``--next`` self-check plants a moved parameter default in each next version and requires the
next-version lint to reject it. A draft may add defaults that are not frozen: M7 0.6.0 adds sixteen,
carried by ``specs/systems/next/registry_delta.toml`` and mirrored in its ``[[params]]`` (R3). A
change planted there is not a frozen default, nothing rejects it, and the self-check would fail on
a correct spec; so the probe picks a parameter whose frozen entry declares a default, and plants
nothing when there is none. These tests pin both cases on the committed specs.
"""

from __future__ import annotations

import copy
import importlib.util
import sys
import tomllib
from decimal import Decimal
from pathlib import Path
from types import ModuleType
from typing import Any

REPO = Path(__file__).resolve().parents[2]
SPECS = REPO / "specs" / "systems"
NEXT = SPECS / "next"


def _lint() -> ModuleType:
    spec = importlib.util.spec_from_file_location("lint_odd", SPECS / "lint_odd.py")
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules["lint_odd"] = module
    spec.loader.exec_module(module)
    return module


def _spec(path: Path) -> dict[str, Any]:
    return tomllib.loads(path.read_text(encoding="utf-8"), parse_float=Decimal)


def _next_errors(
    lint: ModuleType, sid: str, frozen: dict[str, Any], nxt: dict[str, Any]
) -> list[str]:
    errs: list[str] = []
    lint.lint_next(
        {sid: (Path("frozen.odd.toml"), frozen)}, {sid: (Path("next.odd.toml"), nxt)}, errs
    )
    return errs


def test_the_probe_moves_a_frozen_default_and_the_next_lint_rejects_it() -> None:
    """M2: e0 has a frozen default, so the probe moves it and lint_next names the change."""
    lint = _lint()
    frozen = _spec(SPECS / "M2.demography.odd.toml")
    nxt = _spec(NEXT / "M2.demography.odd.toml")
    bad = lint.frozen_default_probe(frozen, nxt)
    assert bad is not None
    errs = _next_errors(lint, "M2.demography", frozen, bad)
    assert [e for e in errs if "frozen default of e0 changed" in e], errs
    assert _next_errors(lint, "M2.demography", frozen, nxt) == []


def test_the_probe_plants_nothing_where_no_frozen_parameter_has_a_default() -> None:
    """M7 0.6.0: sixteen draft defaults and none frozen, so there is no frozen default to move."""
    lint = _lint()
    frozen = _spec(SPECS / "M7.dynasty.odd.toml")
    nxt = _spec(NEXT / "M7.dynasty.odd.toml")
    assert not [x for x in frozen["params"] if "default" in x]
    assert len([x for x in nxt["params"] if "default" in x]) == 16
    assert lint.frozen_default_probe(frozen, nxt) is None


def test_a_moved_draft_default_is_not_a_frozen_default() -> None:
    """Why the probe changed: moving M7's first draft default is not caught as a frozen default."""
    lint = _lint()
    frozen = _spec(SPECS / "M7.dynasty.odd.toml")
    bad = copy.deepcopy(_spec(NEXT / "M7.dynasty.odd.toml"))
    first = next(x for x in bad["params"] if "default" in x)
    first["default"] += 1
    errs = _next_errors(lint, "M7.dynasty", frozen, bad)
    assert not [e for e in errs if "frozen default of" in e], errs
