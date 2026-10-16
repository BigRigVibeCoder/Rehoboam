"""Unit tests for bin/manifest_parity.py's report parser (SPR-004 T-02, V-WC-04)."""

from manifest_parity import Result, parse


def test_registry_checked_report() -> None:
    """A VALID/INVALID line gives the verdict and both counts."""
    out = "[ERROR] x: y\nINVALID: m.yaml (8 errors, 3 warnings) — next gate: liveness burn-in\n"
    assert parse(out, 1) == Result("INVALID", 1, 8, 3)


def test_schema_report() -> None:
    """Pydantic's header gives the structural error count, singular or plural."""
    assert parse("SCHEMA INVALID: m.yaml\n1 validation error for Manifest\n", 2) == Result(
        "SCHEMA INVALID", 2, 1, 0
    )
    assert parse("SCHEMA INVALID: m.yaml\n17 validation errors for Manifest\n", 2).errors == 17


def test_anything_else_is_a_load_error() -> None:
    """A traceback (reference) or LOAD ERROR (Rust) is a load error with its exit code."""
    assert parse("Traceback (most recent call last):\n", 1) == Result("LOAD ERROR", 1, 1, 0)
