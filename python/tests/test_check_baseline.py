"""Unit tests for bin/check_baseline.py: a golden baseline is regenerated, never typed.

The C3 rule fails closed.

Refs: SPR-008 T-04, ruling 6; VER-SPR-008 MAJ-1; docs/13 §4.4.
"""

import subprocess
from pathlib import Path

import pytest
from check_baseline import (
    EXIT_FAIL,
    EXIT_OK,
    EXIT_UNREADABLE,
    C3Rule,
    check,
    has_c3_tag,
    main,
)

LADDER = "rehoboam-ladder v1\nworld smoke\nrung 1 9 0000-12 00000000000000aa 00000000000000bb\n"
UNCHANGED = C3Rule(changed=False, marked=False, where="base")


def write(path: Path, text: str) -> Path:
    """Write ``text`` to ``path``."""
    path.write_text(text, encoding="utf-8")
    return path


def test_an_unchanged_baseline_that_equals_the_recomputation_holds(tmp_path: Path) -> None:
    """The ordinary PR: nothing in the history moved."""
    base = write(tmp_path / "base.ladder", LADDER)
    runs = [write(tmp_path / f"t{n}.ladder", LADDER) for n in (1, 4)]
    ok, line = check(base, runs, UNCHANGED)
    assert ok and line.startswith("BASELINE OK") and "unchanged against the base" in line


def test_a_hand_edited_baseline_fails_whatever_the_title(tmp_path: Path) -> None:
    """One typed hex digit: the baseline no longer equals what the tool writes."""
    base = write(tmp_path / "base.ladder", LADDER.replace("aa", "ab"))
    run = write(tmp_path / "t1.ladder", LADDER)
    for rule in (UNCHANGED, C3Rule(True, True, "t"), C3Rule(None, False, "no base ref")):
        ok, line = check(base, [run], rule)
        assert not ok and line.startswith("BASELINE MISMATCH") and "line 3" in line


def test_a_changed_baseline_needs_a_c3_mark(tmp_path: Path) -> None:
    """Regenerated (equal to the recomputation) but changed: only with a C3 mark."""
    base = write(tmp_path / "base.ladder", LADDER)
    run = write(tmp_path / "t1.ladder", LADDER)
    ok, line = check(base, [run], C3Rule(True, False, "the title `x`"))
    assert not ok and "not marked [C3]" in line
    ok, line = check(base, [run], C3Rule(True, True, "the title `x [C3]`"))
    assert ok and "regenerated in a [C3] PR" in line


def test_with_no_base_the_rule_is_reported_not_checked_never_unchanged(tmp_path: Path) -> None:
    """MAJ-1 item 3: a local run says what it did not check."""
    base = write(tmp_path / "base.ladder", LADDER)
    run = write(tmp_path / "t1.ladder", LADDER)
    ok, line = check(base, [run], C3Rule(None, False, "no base ref"))
    assert ok and "C3 rule not checked (no base ref)" in line and "unchanged" not in line


@pytest.mark.parametrize(
    ("text", "marked"),
    [
        ("SPR-008: Golden-run CI, the smoke world and the bisect tool [C3]", True),
        ("feat(SPR-008): regenerate the smoke baseline [C3]", True),
        ("x [C1] [C3]", True),
        ("x [C3] [perf-baseline]  ", True),
        ("docs: explain the [C3] rule", False),
        ("[C3] at the front only", False),
        ("x [C1] [perf-baseline]", False),
        ("x [C33]", False),
        ("x C3", False),
    ],
)
def test_the_c3_mark_counts_only_in_the_trailing_tag_group(text: str, marked: bool) -> None:
    """MAJ-1 item 4: `docs: explain the [C3] rule` does not pass."""
    assert has_c3_tag(text) is marked


def test_recomputations_that_differ_by_thread_count_fail(tmp_path: Path) -> None:
    """The 1-thread and N-thread ladders must be the same bytes (V-DET-01)."""
    base = write(tmp_path / "base.ladder", LADDER)
    one = write(tmp_path / "t1.ladder", LADDER)
    four = write(tmp_path / "t4.ladder", LADDER.replace("bb", "bc"))
    ok, line = check(base, [one, four], UNCHANGED)
    assert not ok and line.startswith("RECOMPUTATIONS DIFFER")


def test_a_missing_baseline_fails(tmp_path: Path) -> None:
    """No baseline is not a pass."""
    run = write(tmp_path / "t1.ladder", LADDER)
    ok, line = check(tmp_path / "none.ladder", [run], UNCHANGED)
    assert not ok and line.startswith("NO BASELINE")


def test_a_pull_request_run_without_a_base_fails_closed(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """MAJ-1 item 1: exit 2, never a pass."""
    base = write(tmp_path / "base.ladder", LADDER)
    run = write(tmp_path / "t1.ladder", LADDER)
    args = ["--baseline", str(base), "--recomputed", str(run), "--event", "pull_request"]
    assert main([*args, "--base-ref", ""]) == EXIT_UNREADABLE
    assert "needs its base" in capsys.readouterr().err


def test_the_command_line_exit_codes(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """0 holds, 1 does not, 2 unreadable."""
    base = write(tmp_path / "base.ladder", LADDER)
    run = write(tmp_path / "t1.ladder", LADDER)
    args = ["--baseline", str(base), "--recomputed", str(run), "--event", "local"]
    assert main([*args, "--changed", "no"]) == EXIT_OK
    assert main([*args, "--changed", "yes", "--title", "plain"]) == EXIT_FAIL
    assert main(args) == EXIT_OK
    assert "C3 rule not checked (no base ref)" in capsys.readouterr().out
    missing = ["--baseline", str(base), "--recomputed", str(tmp_path / "nope")]
    assert main(missing) == EXIT_UNREADABLE
    capsys.readouterr()


def repo_with_baseline(tmp_path: Path, subject: str) -> tuple[Path, str, str]:
    """A repository whose second commit (``subject``) adds the baseline: (dir, before, after)."""
    git = ["git", "-C", str(tmp_path), "-c", "user.name=t", "-c", "user.email=t@t"]
    subprocess.run([*git, "init", "-q"], check=True)
    write(tmp_path / "other.txt", "x")
    subprocess.run([*git, "add", "."], check=True)
    subprocess.run([*git, "commit", "-qm", "base"], check=True)
    before = subprocess.run(
        [*git, "rev-parse", "HEAD"], capture_output=True, text=True, check=True
    ).stdout.strip()
    write(tmp_path / "base.ladder", LADDER)
    subprocess.run([*git, "add", "."], check=True)
    subprocess.run([*git, "commit", "-qm", subject], check=True)
    after = subprocess.run(
        [*git, "rev-parse", "HEAD"], capture_output=True, text=True, check=True
    ).stdout.strip()
    return tmp_path, before, after


@pytest.mark.parametrize(
    ("subject", "expected"),
    [
        ("feat(SPR-008): regenerate the smoke baseline [C3]", EXIT_OK),
        ("feat(SPR-008): regenerate the smoke baseline", EXIT_FAIL),
        ("docs: explain the [C3] rule", EXIT_FAIL),
    ],
)
def test_a_push_that_changes_the_baseline_needs_a_c3_commit(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, subject: str, expected: int
) -> None:
    """MAJ-1 item 2: on push, a commit in before..after must carry the trailing [C3]."""
    repo, before, after = repo_with_baseline(tmp_path, subject)
    monkeypatch.chdir(repo)
    run = write(repo / "t1.ladder", LADDER)
    args = ["--baseline", "base.ladder", "--recomputed", str(run), "--event", "push"]
    assert main([*args, "--before", before, "--after", after]) == expected
    # The range after the baseline's commit does not touch it: unchanged.
    assert main([*args, "--before", after, "--after", after]) == EXIT_OK


def test_a_push_with_no_previous_commit_is_not_checked(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """An all-zero `before` (a new branch) says the rule was not checked."""
    repo, _, after = repo_with_baseline(tmp_path, "no mark")
    monkeypatch.chdir(repo)
    run = write(repo / "t1.ladder", LADDER)
    args = ["--baseline", "base.ladder", "--recomputed", str(run), "--event", "push"]
    assert main([*args, "--before", "0" * 40, "--after", after]) == EXIT_OK
    assert "C3 rule not checked (no base: the push has no previous commit)" in (
        capsys.readouterr().out
    )


def test_a_pull_request_reads_changed_from_git_against_the_base_ref(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """An added baseline counts as changed against the base; the title then needs [C3]."""
    repo, before, _ = repo_with_baseline(tmp_path, "regenerate")
    monkeypatch.chdir(repo)
    run = write(repo / "t1.ladder", LADDER)
    common = [
        "--baseline",
        "base.ladder",
        "--recomputed",
        str(run),
        "--event",
        "pull_request",
        "--base-ref",
        before,
    ]
    assert main([*common, "--title", "SPR-008: no mark"]) == EXIT_FAIL
    assert main([*common, "--title", "SPR-008: golden CI [C3]"]) == EXIT_OK
    assert main([*common[:-1], "no-such-ref", "--title", "x [C3]"]) == EXIT_UNREADABLE
