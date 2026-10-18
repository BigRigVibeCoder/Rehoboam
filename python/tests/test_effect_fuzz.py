"""SPR-017 T-08: the differential fuzz driver's own tests (bin/effect_fuzz.py).

One seed gives byte-identical files, the files are golden-format TOML with every case kind, the
coverage counters count what they name and report what is missing, and the comparison is exact
except on character mutants, where only accept or reject is compared. Nothing here runs either
evaluator: the PR tier of bin/check_all.sh does.
"""

import tomllib
from pathlib import Path

import effect_fuzz

REPO = Path(__file__).resolve().parents[2]
GOLDEN = REPO / "specs" / "registry" / "effect_language_golden_v0.toml"


def _slice() -> effect_fuzz.Slice:
    return effect_fuzz.Slice(GOLDEN.read_text(encoding="utf-8"))


def test_one_seed_gives_byte_identical_files() -> None:
    """The file is a function of the seed alone."""
    sl = _slice()
    a, meta_a = effect_fuzz.build(sl, 7, 48)
    b, meta_b = effect_fuzz.build(sl, 7, 48)
    assert a == b
    assert meta_a == meta_b
    assert effect_fuzz.build(sl, 8, 48)[0] != a


def test_the_file_is_golden_format_with_every_kind() -> None:
    """It parses, its case ids are unique, its laws exist, and every case kind occurs."""
    body, meta = effect_fuzz.build(_slice(), 3, 80)
    data = tomllib.loads(body)
    ids = [c["id"] for c in data["case"]]
    assert len(ids) == len(set(ids)) == len(meta)
    assert {c["kind"] for c in data["case"]} == set(effect_fuzz.KINDS)
    laws = data["registry"]["programs"]["succession_laws"]
    for case in data["case"]:
        assert case["world"] in data["worlds"]
        if case["kind"] == "succession":
            assert set(laws[case["law"]]) == {"candidates", "eligible", "order"}


def test_even_seeds_build_a_tree_over_the_lineage_bound() -> None:
    """A multiple of 4 keeps its leaves (E_TRUNCATED after pruning); 2 mod 4 has dead leaves."""
    for seed, dead in ((4, False), (6, True)):
        fan = tomllib.loads(effect_fuzz.build(_slice(), seed, 1)[0])["worlds"]["fan"]["fan"]
        size = sum(fan["fanout"] ** g for g in range(fan["depth"] + 1))
        assert size > 4096
        assert fan["dead_leaves"] is dead


def test_coverage_counts_what_it_names() -> None:
    """Each counter moves for its own result, diagnostic or call, and the rest are missing."""
    meta = {
        "a": {"kind": "expr", "text": "count(y in kin(holder, 6))", "class": "typed", "world": "w"},
        "b": {"kind": "order", "text": "abs(1)", "class": "typed", "world": "w"},
        "c": {"kind": "expr", "text": "x", "class": "typed", "world": "w"},
    }
    results = {
        "a": ("int:256", {"truncated": 1}),
        "b": ("error:E_TYPE", {}),
        "c": ("invariant:E_TRUNCATED", {}),
    }
    cov: dict[str, int] = {}
    effect_fuzz.coverage_of(meta, results, cov)
    for key in (
        "kind:expr",
        "kind:order",
        "code:E_TYPE",
        "invariant:E_TRUNCATED",
        "truncated_relations>0",
        "call:kin",
        "call:abs",
        "aggregate:count",
    ):
        assert cov.get(key, 0) > 0, key
    assert cov["kind:expr"] == 2
    missing = [k for k in effect_fuzz.required_coverage() if cov.get(k, 0) == 0]
    assert "code:E_SYNTAX" in missing
    assert "invariant:E_RNG_REUSE" in missing
    assert "call:kin" not in missing


def test_the_comparison_is_exact_except_on_character_mutants() -> None:
    """A character mutant compares accept or reject; everything else compares exactly."""
    rejected_a: effect_fuzz.Result = ("error:E_SYNTAX", {})
    rejected_b: effect_fuzz.Result = ("error:E_TYPE", {})
    assert effect_fuzz.agree("chars", rejected_a, rejected_b)
    assert not effect_fuzz.agree("typed", rejected_a, rejected_b)
    assert not effect_fuzz.agree("fault:E_TYPE", rejected_a, rejected_b)
    value: effect_fuzz.Result = ("int:1", {"missing_reads": 0})
    assert not effect_fuzz.agree("chars", value, rejected_a)
    assert not effect_fuzz.agree("chars", value, ("int:1", {"missing_reads": 1}))
    assert effect_fuzz.agree("typed", value, ("int:1", {"missing_reads": 0}))


def test_emitted_lines_parse() -> None:
    """`id result` with or without the diagnostics, a result holding spaces included."""
    text = (
        "a int:3 missing_reads=0 dropped_actions=0 neutral_reads=1 truncated=2\n"
        "b error:E_TYPE\n"
        "c line:[1, 2] missing_reads=0 dropped_actions=0 neutral_reads=0 truncated=0\n"
    )
    got = effect_fuzz.parse_emit(text)
    assert got["a"] == (
        "int:3",
        {"missing_reads": 0, "dropped_actions": 0, "neutral_reads": 1, "truncated": 2},
    )
    assert got["b"] == ("error:E_TYPE", {})
    assert got["c"][0] == "line:[1, 2]"


def test_seed_lists_and_the_stream() -> None:
    """Seed ranges expand, and SplitMix64 gives its published first output for seed 0."""
    assert effect_fuzz.seeds_of("1-3,7") == [1, 2, 3, 7]
    assert effect_fuzz.splitmix64(0) == 0xE220A8397B1DCDAF
