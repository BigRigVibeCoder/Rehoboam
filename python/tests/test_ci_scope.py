"""bin/ci_scope.py: which changes may skip the golden part of the PR tier (director, 2026-10-17).

Only a change whose every file is documentation skips it; anything the script cannot classify runs
the full part (fail closed). The git cases run in a throwaway repository.

Refs: the director's ruling of 2026-10-17 (docs/decisions-log.md).
"""

import subprocess
from pathlib import Path

import pytest
from ci_scope import changed_files, decide, docs_only, is_doc


@pytest.mark.parametrize(
    "path",
    [
        "docs/03_requirements.md",
        "docs/handoffs/x.md",
        "CODEX/05_PROJECT/SPR-014.md",
        "specs/stream/wire_v1.md",
        "README.md",
    ],
)
def test_documentation_paths(path: str) -> None:
    """docs/, CODEX/ and Markdown anywhere are documentation."""
    assert is_doc(path)


@pytest.mark.parametrize(
    "path",
    [
        "crates/rh-core/src/lib.rs",
        "bin/check_all.sh",
        "worlds/solo/baselines/solo_yearly_200y.ladder",
        "specs/stream/canonical_v0.py",
        ".github/workflows/ci.yml",
        "python/tests/test_ci_scope.py",
        "docsx/a.md.txt",
    ],
)
def test_other_paths(path: str) -> None:
    """Code, specs, baselines, workflows and tests are not."""
    assert not is_doc(path)


def test_docs_only_needs_every_file_and_at_least_one() -> None:
    """Mixed changes and empty lists run the golden part."""
    assert docs_only(["docs/a.md", "CODEX/b.md"])
    assert not docs_only(["docs/a.md", "crates/x/src/lib.rs"])
    assert not docs_only([])
    assert not docs_only(["", "  "])


def test_decide() -> None:
    """Core always runs; golden skips only a known documentation-only change."""
    assert decide("core", ["docs/a.md"])[0] is True
    assert decide("core", None)[0] is True
    assert decide("golden", ["docs/a.md"])[0] is False
    assert decide("golden", ["docs/a.md", "bin/x.py"])[0] is True
    assert decide("golden", None)[0] is True
    assert decide("golden", [])[0] is True


def _git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid", *args],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def test_changed_files_by_event(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """A pull request diffs base...HEAD, a push before..after; others and new branches: unknown."""
    _git(tmp_path, "init", "-q", "-b", "main")
    (tmp_path / "a.rs").write_text("x\n")
    _git(tmp_path, "add", "-A")
    _git(tmp_path, "commit", "-q", "-m", "base")
    base = _git(tmp_path, "rev-parse", "HEAD")
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "n.md").write_text("y\n")
    _git(tmp_path, "add", "-A")
    _git(tmp_path, "commit", "-q", "-m", "docs")
    head = _git(tmp_path, "rev-parse", "HEAD")
    monkeypatch.chdir(tmp_path)
    assert changed_files("pull_request", base, "", "") == ["docs/n.md"]
    assert changed_files("push", "", base, head) == ["docs/n.md"]
    assert changed_files("pull_request", "", "", "") is None
    assert changed_files("push", "", "0" * 40, head) is None
    assert changed_files("push", "", "", head) is None
    assert changed_files("workflow_dispatch", base, base, head) is None
    assert changed_files("pull_request", "no-such-ref", "", "") is None
