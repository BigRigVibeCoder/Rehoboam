"""Unit tests for bin/manifest_parity.py's report parser (SPR-004 T-02, GAP-3; V-WC-04)."""

from manifest_parity import CORPUS, Divergence, Result, declared_divergence, divergence_holds, parse


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


def _fixture(header: str, lines_before: int = 1) -> str:
    return "# comment\n" * lines_before + header + "\nworld: {}\n"


GOOD = '# parity-divergence: VALID / LOAD ERROR "folded block"'


def test_divergence_header_is_read() -> None:
    """A well-formed header in the corpus gives both verdicts and the Rust diagnostic."""
    got = declared_divergence(CORPUS / "x.yaml", _fixture(GOOD))
    assert got == Divergence("VALID", "LOAD ERROR", "folded block")


def test_divergence_header_only_in_the_corpus() -> None:
    """(a) Outside tests/manifest_parity/ the header is not honoured."""
    assert declared_divergence(CORPUS.parent / "x.yaml", _fixture(GOOD)) is None
    assert declared_divergence(CORPUS / ".." / "x.yaml", _fixture(GOOD)) is None


def test_divergence_header_rejects_equal_verdicts() -> None:
    """(b) Two equal verdicts are not a divergence."""
    header = '# parity-divergence: VALID / VALID "x"'
    assert isinstance(declared_divergence(CORPUS / "x.yaml", _fixture(header)), str)


def test_divergence_needs_the_rust_diagnostic() -> None:
    """(c) The header must name a diagnostic, and the Rust output must contain it."""
    bare = "# parity-divergence: VALID / LOAD ERROR"
    assert isinstance(declared_divergence(CORPUS / "x.yaml", _fixture(bare)), str)
    declared = Divergence("VALID", "LOAD ERROR", "folded block")
    python = Result("VALID", 0, 0, 0)
    right = Result("LOAD ERROR", 1, 1, 0, (), "LOAD ERROR: m: ... folded block ...")
    other = Result("LOAD ERROR", 1, 1, 0, (), "LOAD ERROR: m: found a tab")
    assert divergence_holds(declared, python, right)
    assert not divergence_holds(declared, python, other)


def test_divergence_header_only_in_the_first_five_lines() -> None:
    """(d) A header on line 6 or later is ignored."""
    assert declared_divergence(CORPUS / "x.yaml", _fixture(GOOD, lines_before=4)) is not None
    assert declared_divergence(CORPUS / "x.yaml", _fixture(GOOD, lines_before=5)) is None
