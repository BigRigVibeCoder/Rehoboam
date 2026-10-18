"""BCK-002 A-52: every `mutant-*` Cargo feature is visible to the build guard.

`rh-cli run` and `bin/regen_baseline.sh` refuse a build with a mutant compiled in by asking
`rh_golden::run::mutant_features()`, which gathers each crate's `MUTANTS` constant. A mutant feature
missing from its crate's `MUTANTS`, or a crate with mutants that `mutant_features()` does not read,
would let a mutant build make a baseline unseen; this test makes either a red check.
"""

import re
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CRATES = REPO / "crates"
GUARD = CRATES / "rh-golden" / "src" / "run.rs"


def _mutant_features() -> dict[str, list[str]]:
    """Return, per crate directory name, its `mutant-*` features."""
    found: dict[str, list[str]] = {}
    for manifest in sorted(CRATES.glob("*/Cargo.toml")):
        features = tomllib.loads(manifest.read_text(encoding="utf-8")).get("features", {})
        names = sorted(f for f in features if f.startswith("mutant-"))
        if names:
            found[manifest.parent.name] = names
    return found


def test_there_are_mutant_features_to_guard() -> None:
    """SPR-012's and SPR-017's planted switches exist; if they move, this test moves with them."""
    assert _mutant_features() == {
        "rh-manifest": ["mutant-effect"],
        "rh-registry": ["mutant-effect"],
        "rh-sys-demography": ["mutant-resample-day-k0", "mutant-resample-hit-k1"],
    }


def test_every_mutant_feature_is_listed_in_its_crates_mutants() -> None:
    """Each feature appears in `MUTANTS` as a cfg-gated entry naming itself."""
    for crate, names in _mutant_features().items():
        source = "\n".join(
            p.read_text(encoding="utf-8") for p in sorted((CRATES / crate / "src").rglob("*.rs"))
        )
        block = re.search(r"pub const MUTANTS: &\[&str\] = &\[(?P<body>.*?)\];", source, re.S)
        assert block is not None, f"{crate} has mutant features but no MUTANTS constant"
        listed = re.findall(
            r'#\[cfg\(feature = "(?P<f>[^"]+)"\)\]\s*"(?P<n>[^"]+)",', block.group("body")
        )
        assert sorted(f for f, n in listed if f == n) == names, (crate, listed)


def test_the_guard_reads_every_crate_with_mutants() -> None:
    """`mutant_features()` names each such crate's `MUTANTS`."""
    guard = GUARD.read_text(encoding="utf-8")
    body = guard[guard.index("pub fn mutant_features()") :].split("\n}\n", 1)[0]
    for crate in _mutant_features():
        assert f"{crate.replace('-', '_')}::MUTANTS" in body, crate
