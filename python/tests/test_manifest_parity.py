"""Unit tests for bin/manifest_parity.py's report parser (SPR-004 T-02, GAP-3; V-WC-04)."""

from manifest_parity import DIVERGENCE, Result, parse


def test_registry_checked_report() -> None:
    """A VALID/INVALID line gives the verdict and counts; the issue lines are kept in order."""
    out = "[ERROR] x: y\n[WARNING] z: w\nINVALID: m.yaml (1 errors, 1 warnings) — next gate\n"
    assert parse(out, 1) == Result("INVALID", 1, 1, 1, ("[ERROR] x: y", "[WARNING] z: w"))


def test_schema_report_pairs() -> None:
    """Pydantic's report gives (loc, type) pairs; a root error has no location line."""
    out = (
        "SCHEMA INVALID: m.yaml\n2 validation errors for Manifest\n"
        "world.seed\n  Input should be ... [type=less_than_equal, input_value=1]\n"
        "    For further information visit https://errors.pydantic.dev\n"
        "  Input should be a valid dictionary [type=model_type]\n"
    )
    assert parse(out, 2) == Result(
        "SCHEMA INVALID", 2, 2, 0, ("world.seed | less_than_equal", " | model_type")
    )
    assert parse("SCHEMA INVALID: m\n1 validation error for Manifest\n", 2).errors == 1


def test_anything_else_is_a_load_error() -> None:
    """A traceback (reference) or LOAD ERROR (Rust) is a load error with its exit code."""
    assert parse("Traceback (most recent call last):\n", 1) == Result("LOAD ERROR", 1, 1, 0)


def test_divergence_header() -> None:
    """A ruled divergence is declared as two verdicts in a header line."""
    m = DIVERGENCE.search("# note\n# parity-divergence: VALID / LOAD ERROR\nworld: {}\n")
    assert m is not None
    assert (m[1], m[2]) == ("VALID", "LOAD ERROR")
