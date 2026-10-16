"""Unit tests for bin/perf_gate.py: the baseline format, the 10% rule and ruling 3 (SPR-009).

The gate's end-to-end controls (a real 15% slowdown, a typed baseline, a speed-up) run in
bin/check_all.sh; these pin the pure parts so a change to the rules shows up in review.
"""

from pathlib import Path

import perf_gate
import pytest
from perf_gate import (
    FAIL,
    FORMAT,
    OK,
    REBASELINE,
    SLOTS,
    Counts,
    GateError,
    band_problems,
    cmd_integrity,
    compare,
    has_reason,
    integrity_problems,
    parse,
    pr_context,
)

TOOLS = {"rustc": "1.97.0 abc", "valgrind": "valgrind-3.22.0"}


def counts(**benches: int) -> Counts:
    """Counts with a fixed toolchain and the real slots."""
    return Counts(toolchain=dict(TOOLS), slots=dict(SLOTS), benches=dict(benches))


def test_render_and_parse_round_trip_byte_for_byte() -> None:
    """The canonical text parses back to the same counts and renders to the same bytes."""
    c = counts(b=2, a=1)
    text = c.render()
    assert parse(text, "x") == c
    assert parse(text, "x").render() == text
    assert text.index("bench a ") < text.index("bench b ")


def test_parse_refuses_anything_else() -> None:
    """A line in no known form, or no format line, is a GateError, never a silent skip."""
    with pytest.raises(GateError, match="not a"):
        parse(f"format {FORMAT}\nbench a 12x\n", "x")
    with pytest.raises(GateError, match="no `format"):
        parse("bench a 1\n", "x")


def test_ten_percent_is_the_limit() -> None:
    """+10% exactly passes; one instruction more fails (docs/08 §9, V-PERF-04)."""
    assert compare(counts(a=1000), counts(a=1100))[0] == OK
    verdict, lines = compare(counts(a=1000), counts(a=1101))
    assert verdict == FAIL
    assert lines == ["REGRESSION a: 1000 -> 1101 (+10.10% > +10%)"]


def test_a_speed_up_passes_and_says_the_baseline_is_kept() -> None:
    """A faster build passes; compare returns a verdict and never a new baseline."""
    verdict, lines = compare(counts(a=1000), counts(a=500))
    assert verdict == OK
    assert lines == ["faster (baseline kept) a: 1000 -> 500 (-50.00%)"]


def test_a_changed_bench_set_fails() -> None:
    """A bench missing from either side fails: a change of the set is a re-baseline."""
    assert compare(counts(a=1), counts(a=1, b=1))[0] == FAIL
    assert compare(counts(a=1, b=1), counts(a=1))[0] == FAIL


def test_a_changed_toolchain_asks_for_a_re_baseline_not_a_verdict() -> None:
    """Ruling 2: another valgrind reports re-baseline, even when the counts would pass."""
    other = counts(a=1000)
    other.toolchain["valgrind"] = "valgrind-3.23.0"
    verdict, lines = compare(counts(a=1000), other)
    assert verdict == REBASELINE
    assert lines[0] == "toolchain changed: re-baseline required (SPR-009 ruling 2)"


def test_the_reason_line_needs_text() -> None:
    """`perf-baseline-reason:` with nothing after it is no reason."""
    assert has_reason("Why:\nperf-baseline-reason: the keyframe writer batches rows\n")
    assert not has_reason("perf-baseline-reason:   \n")
    assert not has_reason("the keyframe writer batches rows")


def test_the_band_is_64_instructions_or_1e5_of_the_tool_count() -> None:
    """R-8: max(64, 1e-5 relative); toolchain lines stay exact."""
    tool = counts(a=1_000_000, b=100_000_000)
    assert band_problems(counts(a=1_000_064, b=100_000_000), tool) == []
    assert len(band_problems(counts(a=1_000_065, b=100_000_000), tool)) == 1
    assert band_problems(counts(a=1_000_000, b=100_001_000), tool) == []
    assert len(band_problems(counts(a=1_000_000, b=100_001_001), tool)) == 1
    other = counts(a=1_000_000, b=100_000_000)
    other.toolchain["valgrind"] = "valgrind-3.23.0"
    assert band_problems(other, tool)[0].startswith("toolchain valgrind:")


def test_integrity_both_ways() -> None:
    """Ruling 3: typed fails, untagged fails, no reason fails, regenerated with both passes."""
    tool = counts(a=1000).render()
    typed = counts(a=1065).render()
    good = ("[perf-baseline] rows batched", "perf-baseline-reason: batching")
    assert integrity_problems(typed, tool, good)[0].startswith("a: the baseline's 1065 is +65")
    assert integrity_problems(tool, tool, ("rows batched", good[1])) == [
        "a changed baseline needs [perf-baseline] in the pull request's title: 'rows batched'"
    ]
    assert len(integrity_problems(tool, tool, (good[0], ""))) == 1
    assert integrity_problems(tool, tool, good) == []


def test_a_bmi2_change_asks_for_a_re_baseline() -> None:
    """R-9: zstd's BMI2 path changes the counts; a host without it must not be compared."""
    other = counts(a=1000)
    other.toolchain["bmi2"] = "no"
    base = counts(a=1000)
    base.toolchain["bmi2"] = "yes"
    verdict, lines = compare(base, other)
    assert verdict == REBASELINE
    assert "  bmi2: yes -> no" in lines


def test_bench_4_s_empty_slot_must_match() -> None:
    """Ruling 1: the slot line of bench 4 is part of the baseline; a changed slot fails."""
    moved = counts(a=1000)
    moved.slots = {"raster_8x512": "bench 4: something else"}
    assert compare(counts(a=1000), moved)[0] == FAIL
    assert band_problems(moved, counts(a=1000)) == [
        "the baseline's slots or cases differ from the tool's output"
    ]


def test_a_pull_request_run_without_a_title_is_an_error(monkeypatch: pytest.MonkeyPatch) -> None:
    """CI must pass the PR title: a pull_request run without one never skips the title check."""
    monkeypatch.setenv("GITHUB_EVENT_NAME", "pull_request")
    monkeypatch.delenv("RH_PR_TITLE", raising=False)
    with pytest.raises(GateError, match="without RH_PR_TITLE"):
        pr_context()
    monkeypatch.setenv("RH_PR_TITLE", "")
    with pytest.raises(GateError, match="without RH_PR_TITLE"):
        pr_context()
    monkeypatch.setenv("GITHUB_EVENT_NAME", "push")
    assert pr_context() is None


def _write(tmp_path: Path, name: str, c: Counts) -> Path:
    path = tmp_path / name
    path.write_text(c.render(), encoding="utf-8")
    return path


def test_a_new_baseline_is_checked_both_ways(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """Ruling 3 for a baseline that did not exist at the base: regenerated passes, typed fails."""
    # cmd_integrity names a baseline by its path in the repository, so the files live under target/.
    tmp_path = perf_gate.ROOT / "target" / "pytest-perf-gate" / tmp_path.name
    tmp_path.mkdir(parents=True, exist_ok=True)
    monkeypatch.setattr(perf_gate, "base_text", lambda _ref, _path: None)
    monkeypatch.setenv("RH_PR_TITLE", "[perf-baseline] first baseline")
    monkeypatch.setenv("RH_PR_BODY", "perf-baseline-reason: the first baseline")
    tool = _write(tmp_path, "tool.txt", counts(a=1000))
    assert cmd_integrity(_write(tmp_path, "regen.txt", counts(a=1000)), tool, "x", None) == OK
    assert "(new): equals the tool's output" in capsys.readouterr().out
    assert cmd_integrity(_write(tmp_path, "typed.txt", counts(a=1065)), tool, "x", None) == FAIL
    assert "INTEGRITY" in capsys.readouterr().out
    monkeypatch.setenv("RH_PR_TITLE", "first baseline")
    assert cmd_integrity(_write(tmp_path, "untagged.txt", counts(a=1000)), tool, "x", None) == FAIL
