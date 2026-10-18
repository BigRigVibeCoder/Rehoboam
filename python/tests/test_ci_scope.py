"""bin/ci_scope.py: which changes may skip a part, or a path-scoped gate, of the PR tier.

Only a change whose every file is documentation skips the golden and stream parts (director,
2026-10-17; BCK-002 A-63); a path-scoped gate is skipped only by a known change to none of its
declared inputs (A-64), and `trace` fails a scoped gate that reads an undeclared input. Anything the
script cannot classify runs everything (fail closed). The git cases run in a throwaway repository,
the traces in a child process (an audit hook cannot be removed).

Refs: the director's ruling of 2026-10-17 (docs/decisions-log.md); BCK-002 A-63, A-64.
"""

import subprocess
import sys
from pathlib import Path

import pytest
from ci_scope import (
    CHAIN,
    COMMON_INPUTS,
    ROOT,
    SCOPED_GATES,
    chain_moved,
    changed_files,
    decide,
    decide_gate,
    docs_only,
    is_doc,
    main,
    scoped_text,
)


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


def test_decide_stream_part_like_golden() -> None:
    """BCK-002 A-63: the stream part skips a documentation-only change, like the golden part."""
    assert decide("stream", ["docs/a.md"])[0] is False
    assert decide("stream", ["docs/a.md", "crates/rh-stream/src/lib.rs"])[0] is True
    assert decide("stream", None)[0] is True
    assert decide("stream", [])[0] is True


def test_every_declared_input_exists() -> None:
    """A-64: a scoped gate's inputs name real files (a path ending in / a directory)."""
    for key, inputs in SCOPED_GATES.items():
        for entry in (*COMMON_INPUTS, *inputs):
            path = ROOT / entry
            assert path.is_dir() if entry.endswith("/") else path.is_file(), (key, entry)


@pytest.mark.parametrize("key", sorted(SCOPED_GATES))
def test_a_change_to_any_one_input_runs_the_gate(key: str) -> None:
    """A-64: each declared input, changed alone or beside unrelated files, runs its gate."""
    for entry in (*COMMON_INPUTS, *SCOPED_GATES[key]):
        assert decide_gate(key, [entry])[0] is True, entry
        assert decide_gate(key, ["docs/a.md", entry, "crates/x/src/lib.rs"])[0] is True, entry


def test_a_gate_skips_only_known_changes_outside_its_inputs() -> None:
    """A-64: unrelated or documentation changes skip; anything unknown runs (fail closed)."""
    for key in SCOPED_GATES:
        ran, why = decide_gate(key, ["crates/rh-core/src/lib.rs", "docs/a.md"])
        assert ran is False
        assert why.startswith(f"gate {key}: skipped, 2 changed file(s), none of its")
        assert decide_gate(key, None)[0] is True
        assert decide_gate(key, [])[0] is True
        assert decide_gate(key, ["", " "])[0] is True
    assert decide_gate("no-such-gate", ["docs/a.md"])[0] is True
    # Each gate's own list: population synthesis's golden is no input of fixture worlds.
    golden = "specs/worldgen/golden/population_synthesis_v0.txt"
    assert decide_gate("population-synthesis", [golden])[0] is True
    assert decide_gate("fixture-worlds", [golden])[0] is False


def test_the_gate_verdict_in_ci_and_locally(capsys: pytest.CaptureFixture[str]) -> None:
    """`--gate`: an explicit file list decides; a local run (no event) always runs."""
    main(["--gate", "fixture-worlds", "--files", "crates/x.rs"])
    assert capsys.readouterr().out.startswith("CI SCOPE: gate fixture-worlds: skipped")
    main(["--gate", "fixture-worlds", "--event", "local"])
    assert "the scope is unknown, so it runs" in capsys.readouterr().out


def _trace(tmp_path: Path, body: str) -> subprocess.CompletedProcess[str]:
    script = tmp_path / "probe.py"
    script.write_text(body, encoding="utf-8")
    return subprocess.run(
        [sys.executable, "-I", "bin/ci_scope.py", "trace", "fixture-worlds", "--", str(script)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


def test_trace_passes_a_script_that_reads_only_its_inputs(tmp_path: Path) -> None:
    """`trace`: reading declared inputs (and anything outside the repository) passes."""
    done = _trace(
        tmp_path,
        "from pathlib import Path\n"
        "Path('specs/worldgen/golden/fixture_worlds_v0.txt').read_text()\n"
        f"Path({str(tmp_path / 'probe.py')!r}).read_text()\n",
    )
    assert done.returncode == 0, done.stdout + done.stderr
    assert "SCOPE TRACE: gate fixture-worlds:" in done.stdout


def test_trace_fails_an_undeclared_read_or_listing(tmp_path: Path) -> None:
    """`trace`: an undeclared file read or a directory listed fails, naming it."""
    done = _trace(tmp_path, "open('specs/stream/canonical_v0.py').read()\n")
    assert done.returncode == 1
    assert "SCOPE INCOMPLETE: gate fixture-worlds read specs/stream/canonical_v0.py" in done.stdout
    done = _trace(tmp_path, "import os\nos.listdir('specs/manifest/examples')\n")
    assert done.returncode == 1
    assert "read specs/manifest/examples/, which" in done.stdout


def test_trace_keeps_the_script_status(tmp_path: Path) -> None:
    """`trace`: a failing script fails with its own status, whatever it read."""
    done = _trace(tmp_path, "import sys\nprint('STALE: x')\nsys.exit(3)\n")
    assert done.returncode == 3
    assert "STALE: x" in done.stdout


def test_the_chain_counts_only_through_a_gate_s_own_text() -> None:
    """A-64: check_all.sh moves a scoped gate only where its block or functions changed."""
    script = (ROOT / CHAIN).read_text(encoding="utf-8")
    for key in SCOPED_GATES:
        own = scoped_text(script, key)
        assert own.startswith(f"scope {key} &&\n")
        assert "\nrun() {\n" in f"\n{own}" and "\nscope() {\n" in f"\n{own}"
        outside = script.replace(
            'run "CI workflows parse (YAML)" pass ', 'run "CI workflows" pass '
        )
        assert outside != script
        assert scoped_text(outside, key) == own
        assert chain_moved(key, outside) is False
        assert chain_moved(key, None) is True
        assert chain_moved(key, script.replace(f"scope {key} &&\n", "")) is True
    command = "selftest_fixture_worlds.py --tier pr --altered-golden"
    inside = script.replace(command, f"{command} ", 1)
    assert chain_moved("fixture-worlds", inside) is True
    assert chain_moved("population-synthesis", inside) is False
    assert scoped_text("part core &&\n", "fixture-worlds") == ""


def test_a_changed_chain_runs_the_gate_unless_its_own_text_is_known_unchanged() -> None:
    """A-64: the chain alone runs a gate by default (fail closed), and skips it when unmoved."""
    for key in SCOPED_GATES:
        assert decide_gate(key, [CHAIN])[0] is True
        ran, why = decide_gate(key, [CHAIN], chain_moved=False)
        assert ran is False
        assert why.endswith(f"({CHAIN} changed, but not the gate's own text in it)")
        assert decide_gate(key, [CHAIN, "bin/ci_scope.py"], chain_moved=False)[0] is True


def test_files_without_a_base_chain_run_the_gate(capsys: pytest.CaptureFixture[str]) -> None:
    """`--files bin/check_all.sh` with no `--script-base` cannot know the base: it runs."""
    main(["--gate", "fixture-worlds", "--files", CHAIN])
    assert "runs (1 of its inputs changed: bin/check_all.sh)" in capsys.readouterr().out
