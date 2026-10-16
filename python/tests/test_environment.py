"""The Python environment matches GOV-008 §4 and the committed lock (SPR-001 T-02).

Refs: SPR-001 T-02; GOV-008 §4 (Python 3.12, uv, DuckDB >= 1.5); GOV-002 §1 law 1.
"""

import importlib
import sys
from importlib import metadata

import pytest

# REF: GOV-008 §4. CI and the laptop run 3.12; newer interpreters must keep working.
MIN_PYTHON = (3, 12)
# REF: GOV-008 §4 (DuckDB >= 1.5).
MIN_DUCKDB = (1, 5)


def _major_minor(version: str) -> tuple[int, int]:
    """Return the (major, minor) pair of a dotted version string such as ``1.5.6``."""
    major, minor = version.split(".")[:2]
    return int(major), int(minor)


def test_interpreter_is_at_least_3_12() -> None:
    """The interpreter meets ``requires-python``."""
    assert sys.version_info[:2] >= MIN_PYTHON


@pytest.mark.parametrize("module", ["pydantic", "yaml", "duckdb"])
def test_spec_check_dependencies_import(module: str) -> None:
    """Every runtime dependency of the spec checks imports."""
    assert importlib.import_module(module) is not None


def test_duckdb_meets_the_minimum_version() -> None:
    """DuckDB is at least the version GOV-008 §4 requires."""
    assert _major_minor(metadata.version("duckdb")) >= MIN_DUCKDB
