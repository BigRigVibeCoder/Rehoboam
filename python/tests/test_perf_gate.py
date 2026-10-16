"""Unit tests for bin/perf_gate.py: the baseline format, the 10% rule and ruling 3 (SPR-009).

The gate's end-to-end controls (a real 15% slowdown, a typed baseline, a speed-up) run in
bin/check_all.sh; these pin the pure parts so a change to the rules shows up in review.
"""

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
    compare,
    has_reason,
    integrity_problems,
    parse,
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
