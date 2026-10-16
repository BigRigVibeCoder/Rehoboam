"""Structural regression tests for bin/check_all.sh.

DEF-2: a function definition placed inside the `&&` chain split it in two, so a failing first check
did not stop the run. These tests keep the chain whole.

Refs: SPR-001 DEF-2; GOV-002 §1 laws 5 (fail fast) and 9 (regression first), §4.
"""

import re
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "bin" / "check_all.sh"
FUNCTION_DEFINITION = re.compile(r"^\s*[A-Za-z_][A-Za-z0-9_]*\s*\(\)\s*\{")


def _chain() -> list[str]:
    """Return the logical lines from the first top-level `run` to `status=$?`, without comments."""
    lines = SCRIPT.read_text(encoding="utf-8").splitlines()
    start = next(i for i, line in enumerate(lines) if line.startswith("run "))
    end = lines.index("status=$?")
    logical: list[str] = []
    pending = ""
    for line in lines[start:end]:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        pending += line
        if pending.endswith("\\"):
            pending = pending[:-1]
            continue
        logical.append(pending)
        pending = ""
    return logical


def test_the_chain_is_one_fail_fast_sequence() -> None:
    """Every check but the last ends with `&&`, so the first failure stops the run."""
    chain = _chain()
    assert len(chain) > 1
    assert [line for line in chain[:-1] if not line.rstrip().endswith("&&")] == []
    assert not chain[-1].rstrip().endswith("&&")


def test_no_function_is_defined_inside_the_chain() -> None:
    """Functions are defined above the chain (the DEF-2 regression)."""
    assert [line for line in _chain() if FUNCTION_DEFINITION.match(line)] == []
