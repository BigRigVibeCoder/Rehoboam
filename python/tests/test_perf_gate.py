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
    Base,
    Counts,
    GateError,
    alloc_compare,
    band_problems,
    cmd_integrity,
    compare,
    has_reason,
    has_tag,
    integrity_problems,
    parse,
    pr_context,
    push_problems,
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
    good = ("perf: rows batched [perf-baseline]", "perf-baseline-reason: batching")
    assert integrity_problems(typed, tool, good)[0].startswith("a: the baseline's 1065 is +65")
    assert integrity_problems(tool, tool, ("rows batched", good[1])) == [
        "a changed baseline needs [perf-baseline] in the pull request's title, in its trailing "
        "tag group: 'rows batched'"
    ]
    mid = ("docs: explain the [perf-baseline] rule", good[1])
    assert len(integrity_problems(tool, tool, mid)) == 1
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
    with pytest.raises(GateError, match="without RH_PUSH_BEFORE"):
        pr_context()
    monkeypatch.setenv("RH_PUSH_BEFORE", "a" * 40)
    monkeypatch.setenv("RH_PUSH_AFTER", "b" * 40)
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
    monkeypatch.setattr(perf_gate, "base_text", lambda *_: None)
    monkeypatch.setenv("RH_PR_TITLE", "perf: first baseline [C1] [perf-baseline]")
    monkeypatch.setenv("RH_PR_BODY", "perf-baseline-reason: the first baseline")
    tool = _write(tmp_path, "tool.txt", counts(a=1000))
    assert cmd_integrity(_write(tmp_path, "regen.txt", counts(a=1000)), tool, Base("x")) == OK
    assert "(new): equals the tool's output" in capsys.readouterr().out
    assert cmd_integrity(_write(tmp_path, "typed.txt", counts(a=1065)), tool, Base("x")) == FAIL
    assert "INTEGRITY" in capsys.readouterr().out
    monkeypatch.setenv("RH_PR_TITLE", "first baseline")
    assert cmd_integrity(_write(tmp_path, "untagged.txt", counts(a=1000)), tool, Base("x")) == FAIL


def test_a_push_needs_a_tagged_commit_with_a_reason() -> None:
    """GAP-2: on a push, a changed baseline needs a [perf-baseline] commit with a Reason: line."""
    good = ("SPR-009: re-record the baseline [perf-baseline]", "Why:\nReason: the arena\n")
    assert push_problems([("x", ""), good]) == []
    assert push_problems([("SPR-009: re-record [perf-baseline]", "no reason")]) != []
    assert push_problems([("SPR-009: re-record", "Reason: the arena")]) != []
    assert push_problems([("docs: explain [perf-baseline] use", "Reason: x")]) != []
    assert has_tag("x [C1] [perf-baseline]")
    assert has_tag("x [perf-baseline] [C1]")
    assert not has_tag("x [perf-baseline] y")


def test_a_local_run_says_it_did_not_check_the_tag(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """GAP-2: no pull-request or push context: the equality is checked, the tag is not, said so."""
    tmp_path = perf_gate.ROOT / "target" / "pytest-perf-gate" / tmp_path.name
    tmp_path.mkdir(parents=True, exist_ok=True)
    for var in ("RH_PR_TITLE", "RH_EVENT", "GITHUB_EVENT_NAME"):
        monkeypatch.delenv(var, raising=False)
    monkeypatch.setattr(perf_gate, "base_text", lambda *_: None)
    tool = _write(tmp_path, "tool.txt", counts(a=1000))
    assert cmd_integrity(_write(tmp_path, "regen.txt", counts(a=1000)), tool, Base("x")) == OK
    out = capsys.readouterr().out
    assert "tag and reason not checked here" in out
    assert "with the [perf-baseline] tag" not in out


def test_allocation_baselines_are_exact() -> None:
    """R-8: allocation counts stay exact; one allocation more is a typed baseline."""
    assert integrity_problems("a 1\n", "a 1\n", None, exact=True) == []
    assert integrity_problems("a 2\n", "a 1\n", None, exact=True) != []


@pytest.mark.parametrize("event", ["local", "schedule", "pull_request_target"])
def test_an_unknown_event_inside_actions_is_an_error(
    monkeypatch: pytest.MonkeyPatch, event: str
) -> None:
    """Review 6: inside Actions only pull_request, push and workflow_dispatch are known."""
    monkeypatch.setenv("GITHUB_ACTIONS", "true")
    monkeypatch.setenv("RH_EVENT", event)
    with pytest.raises(GateError, match="inside GitHub Actions"):
        pr_context()
    monkeypatch.setenv("RH_EVENT", "workflow_dispatch")
    assert pr_context() is None
    monkeypatch.delenv("GITHUB_ACTIONS")
    monkeypatch.setenv("RH_EVENT", event)
    assert pr_context() is None


ALLOC = (
    "format rehoboam/rh-bench/allocations/v1\n"
    "toolchain rustc 1.97.0 abc\ntoolchain libc glibc 2.39\n"
    "window smoke\nbudget threads 1 ticks 10 allocations 100 growths 5 shrinks 0 max_per_tick 9\n"
)


def test_an_allocation_toolchain_change_asks_for_a_re_baseline() -> None:
    """Review 7: another rustc reports re-baseline (exit 3), not OVER BUDGET or a pass."""
    assert alloc_compare(ALLOC, ALLOC)[0] == OK
    over = ALLOC.replace("allocations 100", "allocations 101")
    assert alloc_compare(ALLOC, over)[0] == FAIL
    other = over.replace("1.97.0 abc", "1.98.0 def")
    verdict, lines = alloc_compare(ALLOC, other)
    assert verdict == REBASELINE
    assert lines == [
        "toolchain changed: re-baseline required (SPR-009 review 7, as R-9)",
        "  rustc: 1.97.0 abc -> 1.98.0 def",
    ]


@pytest.mark.parametrize("field_name", ["growths", "max_per_tick"])
def test_each_gated_allocation_field_fails_on_any_increase(field_name: str) -> None:
    """GAP-15: growths and the most in a tick are gated like allocations; a decrease is kept."""
    value = {"growths": 5, "max_per_tick": 9}[field_name]
    more = ALLOC.replace(f"{field_name} {value}", f"{field_name} {value + 1}")
    verdict, lines = alloc_compare(ALLOC, more)
    assert verdict == FAIL
    assert f"OVER BUDGET threads 1 {field_name}: {value} -> {value + 1} (+1)" in lines
    fewer = ALLOC.replace(f"{field_name} {value}", f"{field_name} {value - 1}")
    verdict, lines = alloc_compare(ALLOC, fewer)
    assert verdict == OK
    assert f"below budget (kept) threads 1 {field_name}: {value} -> {value - 1}" in lines


def test_shrinks_are_reported_not_gated() -> None:
    """Ruling 5: shrinks are not a gated field."""
    assert alloc_compare(ALLOC, ALLOC.replace("shrinks 0", "shrinks 50"))[0] == OK


def test_a_changed_window_or_tick_count_asks_for_a_re_baseline() -> None:
    """GAP-15: another window, other thread counts or other ticks: exit 3, not a verdict."""
    window = ALLOC.replace("window smoke", "window smoke seed 2")
    verdict, lines = alloc_compare(ALLOC, window)
    assert verdict == REBASELINE
    assert lines[0].startswith("window changed: re-baseline required")
    threads = ALLOC.replace("budget threads 1 ", "budget threads 4 ")
    assert alloc_compare(ALLOC, threads)[0] == REBASELINE
    ticks = ALLOC.replace("ticks 10", "ticks 11").replace("allocations 100", "allocations 999")
    verdict, lines = alloc_compare(ALLOC, ticks)
    assert verdict == REBASELINE
    assert lines == ["window changed: the ticks at 1 thread(s) differ"]


RUNGS = ALLOC + (
    "window-rungs solo,hamlet warm-up 2 years\n"
    "rung solo seed 1 threads 1 ticks 12 allocations 1068 growths 85 shrinks 0 max_per_tick 89\n"
    "rung hamlet seed 42 threads 1 ticks 14 allocations 1502 growths 157 shrinks 0 "
    "max_per_tick 113\n"
)


def test_each_rung_is_gated_like_the_smoke_world() -> None:
    """SPR-013 T-13: a rung line is a budget of its own; an increase in one rung fails by name."""
    assert alloc_compare(RUNGS, RUNGS)[0] == OK
    verdict, lines = alloc_compare(RUNGS, RUNGS.replace("allocations 1502", "allocations 1503"))
    assert verdict == FAIL
    assert "OVER BUDGET rung hamlet allocations: 1502 -> 1503 (+1)" in lines
    assert "ok rung solo allocations: 1068 -> 1068" in lines
    verdict, lines = alloc_compare(RUNGS, RUNGS.replace("ticks 12", "ticks 13"))
    assert (verdict, lines) == (REBASELINE, ["window changed: the ticks of rung solo differ"])
    assert alloc_compare(RUNGS, RUNGS.replace("seed 42", "seed 43"))[0] == REBASELINE
    rung_threads = RUNGS.replace("rung solo seed 1 threads 1", "rung solo seed 1 threads 2")
    assert alloc_compare(RUNGS, rung_threads) == (
        REBASELINE,
        ["window changed: the ticks of rung solo differ"],
    )
    assert alloc_compare(ALLOC, RUNGS)[0] == REBASELINE, "a rung added is a new window"


def test_a_push_with_no_base_says_it_did_not_check(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """G3: a push whose before is all zeros (a new branch) has no base: said so, never a pass."""
    tmp_path = perf_gate.ROOT / "target" / "pytest-perf-gate" / tmp_path.name
    tmp_path.mkdir(parents=True, exist_ok=True)
    monkeypatch.setenv("RH_EVENT", "push")
    monkeypatch.setenv("RH_PUSH_BEFORE", "0" * 40)
    monkeypatch.setenv("RH_PUSH_AFTER", "b" * 40)
    tool = _write(tmp_path, "tool.txt", counts(a=1000))
    typed = _write(tmp_path, "typed.txt", counts(a=1065))
    assert cmd_integrity(typed, tool, Base("x")) == OK
    assert "not checked: the push has no base (a new branch)" in capsys.readouterr().out
