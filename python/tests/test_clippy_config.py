"""clippy.toml keeps the libm-only rule complete: every std float transcendental is disallowed.

The SPR-003 Architect ruling (2026-10-16), item 3, names 22 std float transcendentals, for f32 and
f64: 44 disallowed-methods entries. The check_all controls call only f64::ln and f64::exp, so this
test pins the whole list: deleting or misspelling any one entry fails here (VER-SPR-003-TEST GAP-2).
`sqrt` stays allowed (IEEE 754 rounds it correctly) and `mul_add` stays disallowed.

Refs: SPR-003 ruling item 3; ADR-0006 item 4; V-DET-19.
"""

import tomllib
from pathlib import Path

CLIPPY = Path(__file__).resolve().parents[2] / "clippy.toml"
# Ruling item 3, verbatim and in its order.
TRANSCENDENTALS = [
    "ln",
    "log",
    "log2",
    "log10",
    "ln_1p",
    "exp",
    "exp2",
    "exp_m1",
    "powf",
    "powi",
    "sin",
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "atan2",
    "sinh",
    "cosh",
    "tanh",
    "cbrt",
    "hypot",
]
FLOATS = ("f32", "f64")


def disallowed_methods() -> list[dict[str, object]]:
    """Return clippy.toml's disallowed-methods entries."""
    entries = tomllib.loads(CLIPPY.read_text(encoding="utf-8"))["disallowed-methods"]
    assert isinstance(entries, list)
    return entries


def float_paths() -> set[str]:
    """Return the disallowed paths that name an f32 or f64 method."""
    return {str(e["path"]) for e in disallowed_methods() if str(e["path"]).startswith(FLOATS)}


def test_the_ruling_names_22_transcendentals() -> None:
    """The list above is the ruling's: 22 distinct names, so 44 entries."""
    assert len(set(TRANSCENDENTALS)) == len(TRANSCENDENTALS) == 22


def test_every_std_transcendental_is_disallowed_for_f32_and_f64() -> None:
    """Exactly the 44 ruled entries, plus mul_add for both types, and nothing else on floats."""
    expected = {f"{t}::{name}" for t in FLOATS for name in TRANSCENDENTALS}
    assert float_paths() == expected | {"f32::mul_add", "f64::mul_add"}


def test_sqrt_stays_allowed() -> None:
    """IEEE 754 rounds sqrt correctly (ruling item 3)."""
    assert not {"f32::sqrt", "f64::sqrt"} & float_paths()


def test_each_transcendental_entry_points_to_libm() -> None:
    """Each entry's reason tells the reader what to use instead."""
    for entry in disallowed_methods():
        path = str(entry["path"])
        if path.startswith(FLOATS) and not path.endswith("mul_add"):
            assert "libm" in str(entry["reason"]), path
