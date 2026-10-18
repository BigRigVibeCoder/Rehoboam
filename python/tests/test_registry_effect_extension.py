"""SPR-017 T-09: V-WC-10's data-only extension stays what it claims to be.

`tests/registry_effect/registry_absolute_primogeniture.toml` is the real Registry with two
edits: `absolute_primogeniture` appended to `[succession_laws] ids`, and its programs. Its case file
pins the law's lines on the golden file's worlds, so its slice must be the golden file's, and the
law's three programs must be the same text in the fixture Registry, the case file and the golden
file. When the real Registry or the golden file changes, these tests say the fixtures need
regenerating.
"""

import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
REGISTRY = REPO / "specs" / "manifest" / "registry_v0.toml"
GOLDEN = REPO / "specs" / "registry" / "effect_language_golden_v0.toml"
FIXTURE = REPO / "tests" / "registry_effect" / "registry_absolute_primogeniture.toml"
CASES = REPO / "tests" / "registry_effect" / "absolute_primogeniture_cases.toml"
LAW = "absolute_primogeniture"
PROGRAMS = ("candidates", "eligible", "order")


def _slice(text: str) -> str:
    """A golden-format file's `[registry]` slice, as text."""
    lines = text.splitlines(keepends=True)
    start = next(i for i, ln in enumerate(lines) if ln.strip() == "[registry]")
    end = next(i for i, ln in enumerate(lines) if ln.startswith(("[worlds.", "[[worlds.")))
    return "".join(lines[start:end])


def test_the_fixture_is_the_registry_with_two_edits() -> None:
    """Removing the law's table and its id gives the real Registry back, byte for byte."""
    fixture = FIXTURE.read_text(encoding="utf-8")
    start = fixture.index("\n# SPR-017 T-09 (V-WC-10, EVO-1)")
    end = fixture.index("\n", fixture.index("order      =", start)) + 1
    unedited = (fixture[:start] + fixture[end:]).replace(f', "{LAW}"]', "]", 1)
    assert unedited == REGISTRY.read_text(encoding="utf-8")


def test_the_law_has_one_text_everywhere() -> None:
    """The fixture Registry's programs equal the case file's slice and the golden file's."""
    fixture = tomllib.loads(FIXTURE.read_text(encoding="utf-8"))
    assert fixture["succession_laws"]["ids"][-1] == LAW
    ours = fixture["succession_laws"][LAW]
    for path in (CASES, GOLDEN):
        theirs = tomllib.loads(path.read_text(encoding="utf-8"))["registry"]["programs"]
        assert {p: theirs["succession_laws"][LAW][p] for p in PROGRAMS} == {
            p: ours[p] for p in PROGRAMS
        }, path.name


def test_the_case_file_uses_the_golden_slice_and_the_law() -> None:
    """Its `[registry]` slice is the golden file's, and its cases run the law on two worlds."""
    cases = CASES.read_text(encoding="utf-8")
    assert _slice(cases) == _slice(GOLDEN.read_text(encoding="utf-8"))
    data = tomllib.loads(cases)
    assert {c["law"] for c in data["case"]} == {LAW}
    assert {c["world"] for c in data["case"]} == {"dynasty", "deep"}
