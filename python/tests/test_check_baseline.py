"""Unit tests for bin/check_baseline.py: a golden baseline is regenerated, never typed.

Refs: SPR-008 T-04, ruling 6; docs/13 §4.4.
"""

import subprocess
from pathlib import Path

import pytest
from check_baseline import EXIT_FAIL, EXIT_OK, EXIT_UNREADABLE, check, main

LADDER = "rehoboam-ladder v1\nworld smoke\nrung 1 9 0000-12 00000000000000aa 00000000000000bb\n"


def write(path: Path, text: str) -> Path:
    """Write ``text`` to ``path``."""
    path.write_text(text, encoding="utf-8")
    return path


def test_an_unchanged_baseline_that_equals_the_recomputation_holds(tmp_path: Path) -> None:
    """The ordinary PR: nothing in the history moved."""
    base = write(tmp_path / "base.ladder", LADDER)
    runs = [write(tmp_path / f"t{n}.ladder", LADDER) for n in (1, 4)]
    ok, line = check(base, runs, changed=False, title="SPR-009: something")
    assert ok and line.startswith("BASELINE OK") and "unchanged" in line


def test_a_hand_edited_baseline_fails_whatever_the_title(tmp_path: Path) -> None:
    """One typed hex digit: the baseline no longer equals what the tool writes."""
    base = write(tmp_path / "base.ladder", LADDER.replace("aa", "ab"))
    run = write(tmp_path / "t1.ladder", LADDER)
    for changed in (False, True):
        ok, line = check(base, [run], changed=changed, title="SPR-008: x [C3]")
        assert not ok and line.startswith("BASELINE MISMATCH") and "line 3" in line


def test_a_regenerated_baseline_needs_a_c3_title(tmp_path: Path) -> None:
    """Regenerated (equal to the recomputation) but changed against the base: only in [C3]."""
    base = write(tmp_path / "base.ladder", LADDER)
    run = write(tmp_path / "t1.ladder", LADDER)
    ok, line = check(base, [run], changed=True, title="SPR-008: new systems")
    assert not ok and "not marked [C3]" in line
    ok, line = check(base, [run], changed=True, title="SPR-008: new systems [C3]")
    assert ok and "regenerated in a [C3] PR" in line


def test_recomputations_that_differ_by_thread_count_fail(tmp_path: Path) -> None:
    """The 1-thread and N-thread ladders must be the same bytes (V-DET-01)."""
    base = write(tmp_path / "base.ladder", LADDER)
    one = write(tmp_path / "t1.ladder", LADDER)
    four = write(tmp_path / "t4.ladder", LADDER.replace("bb", "bc"))
    ok, line = check(base, [one, four], changed=False, title="")
    assert not ok and line.startswith("RECOMPUTATIONS DIFFER")


def test_a_missing_baseline_fails(tmp_path: Path) -> None:
    """No baseline is not a pass."""
    run = write(tmp_path / "t1.ladder", LADDER)
    ok, line = check(tmp_path / "none.ladder", [run], changed=False, title="")
    assert not ok and line.startswith("NO BASELINE")


def test_the_command_line_exit_codes(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """0 holds, 1 does not, 2 unreadable."""
    base = write(tmp_path / "base.ladder", LADDER)
    run = write(tmp_path / "t1.ladder", LADDER)
    args = ["--baseline", str(base), "--recomputed", str(run)]
    assert main([*args, "--changed", "no"]) == EXIT_OK
    assert main([*args, "--changed", "yes", "--title", "plain"]) == EXIT_FAIL
    missing = ["--baseline", str(base), "--recomputed", str(tmp_path / "nope")]
    assert main(missing) == EXIT_UNREADABLE
    capsys.readouterr()


def test_changed_is_read_from_git_against_the_base_ref(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """An added or modified baseline counts as changed; an untouched one does not."""
    git = ["git", "-C", str(tmp_path), "-c", "user.name=t", "-c", "user.email=t@t"]
    subprocess.run([*git, "init", "-q"], check=True)
    write(tmp_path / "other.txt", "x")
    subprocess.run([*git, "add", "."], check=True)
    subprocess.run([*git, "commit", "-qm", "base"], check=True)
    base = write(tmp_path / "base.ladder", LADDER)
    run = write(tmp_path / "t1.ladder", LADDER)
    common = ["--baseline", str(base), "--recomputed", str(run), "--base-ref", "HEAD"]
    monkeypatch.chdir(tmp_path)
    subprocess.run([*git, "add", "-N", "base.ladder"], check=True)
    assert main([*common, "--title", "no mark"]) == EXIT_FAIL, "added: changed"
    subprocess.run([*git, "add", "."], check=True)
    subprocess.run([*git, "commit", "-qm", "baseline"], check=True)
    assert main([*common, "--title", "no mark"]) == EXIT_OK, "committed: unchanged"
    assert main([*common[:-1], "no-such-ref"]) == EXIT_UNREADABLE
