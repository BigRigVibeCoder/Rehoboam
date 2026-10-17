"""Unit tests for bin/wall_trend.py: medians from samples, the 10% alert, machine identity.

Refs: SPR-009 T-02b, T-04; RUN-002 §5; docs/08 §9.
"""

import json
from pathlib import Path

import pytest
from wall_trend import ALERT, MACHINE, OK, Trend, TrendError, case, combine, compare, parse

IDENTITY = {"machine": "adjutant", "cpu": "AMD EPYC 9B14", "cpus": "16", "rustc": "rustc 1.97.0"}


def trend(**medians: float) -> Trend:
    """A trend with fixed conditions and the given medians."""
    return Trend(dict(IDENTITY), {k: (v, 1.0, 10) for k, v in medians.items()})


def test_round_trip() -> None:
    """The canonical text parses back to the same values."""
    t = trend(a=1234.5)
    assert parse(t.render(), "x") == t


def test_median_and_spread_come_from_the_samples(tmp_path: Path) -> None:
    """Per-iteration times are times / iters; the median and the spread are over them."""
    d = tmp_path / "g" / "c" / "1000" / "new"
    d.mkdir(parents=True)
    (d / "benchmark.json").write_text(json.dumps({"full_id": "g/c/1000"}))
    iters = [1.0, 2.0, 3.0, 4.0, 5.0]
    times = [100.0, 220.0, 300.0, 360.0, 500.0]  # per iteration 100, 110, 100, 90, 100
    (d / "sample.json").write_text(json.dumps({"iters": iters, "times": times}))
    key, (median, spread, n) = case(d / "sample.json")
    assert (key, median, n) == ("g/c/1000", 100.0, 5)
    assert spread == pytest.approx(20.0)


def test_fewer_than_five_samples_is_refused(tmp_path: Path) -> None:
    """RUN-002 §5: at least 5 runs per wall-clock bench."""
    d = tmp_path / "new"
    d.mkdir()
    (d / "benchmark.json").write_text(json.dumps({"full_id": "x"}))
    (d / "sample.json").write_text(json.dumps({"iters": [1.0] * 4, "times": [1.0] * 4}))
    with pytest.raises(TrendError, match="fewer than 5"):
        case(d / "sample.json")


def test_the_alert_is_above_ten_percent() -> None:
    """+10% is within the trend; more is an ALERT; faster is reported, never written."""
    assert compare(trend(a=100.0), trend(a=110.0))[0] == OK
    assert compare(trend(a=100.0), trend(a=110.5))[0] == ALERT
    verdict, lines = compare(trend(a=100.0), trend(a=50.0))
    assert verdict == OK
    assert lines[0].startswith("faster (trend kept) a:")


def test_another_machine_is_never_compared() -> None:
    """RUN-002 §5: a resized or migrated VM gets a new baseline, never a silent comparison."""
    other = trend(a=100.0)
    other.conditions["cpu"] = "Intel Xeon"
    assert compare(trend(a=100.0), other)[0] == MACHINE
    noted = trend(a=100.0)
    noted.conditions["temperature"] = "71.0C"
    assert compare(trend(a=100.0), noted)[0] == OK


def test_combine_takes_the_median_of_run_medians_and_the_run_to_run_spread() -> None:
    """T-04: five separate runs; the spread across runs decides whether a gate is possible."""
    runs = [trend(a=m) for m in (100.0, 104.0, 98.0, 102.0, 96.0)]
    median, spread, n = combine(runs).benches["a"]
    assert (median, n) == (100.0, 5)
    assert spread == pytest.approx(8.0)
    with pytest.raises(TrendError, match="at least 5"):
        combine(runs[:4])
    other = trend(a=100.0)
    other.conditions["machine"] = "laptop"
    with pytest.raises(TrendError, match="different machines"):
        combine([*runs[:4], other])


@pytest.mark.parametrize(
    "median, spread",
    [
        ("nan", "1.0"),
        ("inf", "1.0"),
        ("0.0", "1.0"),
        ("-5", "1"),
        ("100.0", "nan"),
        ("100.0", "-1"),
    ],
)
def test_non_finite_or_non_positive_values_are_refused(median: str, spread: str) -> None:
    """DEF-3: a NaN median makes every comparison false, so a slowdown would pass; refuse it."""
    text = trend(a=1.0).render().replace("bench a 1.0 1.00 10", f"bench a {median} {spread} 10")
    with pytest.raises(TrendError, match="not a"):
        parse(text, "x")


def test_zero_samples_are_refused_and_a_zero_spread_is_allowed() -> None:
    """D4: a bench line with 0 samples is refused; identical samples (spread 0) are not."""
    with pytest.raises(TrendError, match="not a"):
        parse(trend(a=1.0).render().replace("bench a 1.0 1.00 10", "bench a 1.0 1.00 0"), "x")
    same = trend(a=1.0).render().replace("bench a 1.0 1.00 10", "bench a 1.0 0.00 10")
    assert parse(same, "x").benches["a"] == (1.0, 0.0, 10)
