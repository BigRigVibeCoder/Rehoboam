"""The gate inventory of bin/check_all.sh: deleting a gate's line must turn this test red.

Every check a sprint adds to bin/check_all.sh is listed here by its exact name, and must appear as a
`run "<name>"` line inside the fail-fast chain (not in a comment, not after `status=$?`). Without
this list, removing a line leaves the tier green: nothing else notices that a gate stopped running
(SPR-010 GAP-4, the golden `--check` line; VER-SPR-001-TEST §4 item 2).

EVERY SPRINT ADDS ITS GATES HERE, in the same commit that adds them to bin/check_all.sh. Keep each
list alphabetical. Renaming a gate means renaming it here too, which is the point: a reviewer sees
the change.

Refs: SPR-010 GAP-4 (VER-SPR-010-TEST); GOV-002 §1 laws 4 and 9, §4.
"""

import re
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "bin" / "check_all.sh"
FIXED_POINT_FIXTURE = (
    Path(__file__).resolve().parents[2] / "tests" / "lint_fixtures" / "compile_fail" / "fixed_point"
)
RUN_NAME = re.compile(r'^\s*run "(?P<name>[^"]*)"')

# Every gate run directly by the chain, by its exact `run` name. Alphabetical (Python's str order).
REQUIRED_GATES = (
    "Architecture: crate boundaries and planes (lint_arch; V-INSP-03, V-PLN-01)",
    "CI workflows parse (YAML)",
    "Chronicle SQL-twin equivalence",
    "Clippy float backstop positive control: unnamed float arithmetic in plane A must FAIL "
    "(SPR-010 GAP-3)",
    "Doctest count gate positive control: one typed-ID case missing must FAIL",
    "Doctest count gate: every expected compile-fail case and twin ran (SPR-002 T-01; DEF-2)",
    "Document index builds (gen_manifest --check)",
    "Forbidden APIs in plane A (lint_forbidden; V-DET-19)",
    "Golden vectors: Brass (M2)",
    "Golden vectors: M4 basket and price rule",
    "Golden vectors: rh-core::fixed are the exact reference's (gen_fixed_vectors --check)",
    "Golden vectors: rh-ids reference matches the committed file "
    "(gen_rng_golden --check; V-DET-09)",
    "Licence ledger control: the GPL fixture is listed as GPL-3.0-only",
    "Licence ledger, not a gate (artifacts/licence_ledger.txt; ADR-0021)",
    "Manifest: aethelgard",
    "Manifest: invalid_draft_style must FAIL",
    "Manifest: solo",
    "Manifest: uz_job",
    "Manifest: val_khor",
    "Python: format (ruff format --check)",
    "Python: lint (ruff check)",
    "Python: types (mypy --strict)",
    "Python: unit tests (pytest)",
    "Python: uv.lock matches pyproject.toml (uv lock --check)",
    "Required tools present",
    "Rust: bans and sources (cargo deny: crates.io only, no wildcards)",
    "Rust: build the workspace (cargo build)",
    "Rust: doctests (cargo test --doc)",
    "Rust: format (cargo fmt --check)",
    "Rust: lint (cargo clippy -D warnings)",
    "Rust: unit tests (cargo nextest)",
    "Sifting patterns (lint_patterns)",
    "System specs (lint_odd)",
    "V-PLN-01 compile-fail control: plane A naming a plane-B type must FAIL",
    "V-PLN-01 compile-fail control: plane A naming a plane-C type must FAIL",
    "V-PLN-01 compile-fail fixture (plane C) has a clean dependency graph",
    "V-PLN-01 compile-fail fixture has a clean dependency graph",
    "Virtual telemetry (S5 prototype)",
    "cargo deny positive control: git source must FAIL",
    "cargo deny positive control: wildcard version must FAIL",
    "gen_rng_golden positive control: stale golden file must FAIL",
    "lint_arch positive control: kind contract outside rh-contract-* must FAIL",
    "lint_arch positive control: missing plane/kind must FAIL",
    "lint_arch positive control: plane A -> plane B must FAIL",
    "lint_arch positive control: plane A -> plane C must FAIL",
    "lint_arch positive control: plane A -> rand must FAIL",
    "lint_arch positive control: rand_chacha outside rh-ids must FAIL",
    "lint_arch positive control: relabelled rh-snapshot must FAIL",
    "lint_arch positive control: system -> rh-snapshot aliased as rh-core must FAIL",
    "lint_arch positive control: system -> rh-snapshot as a build-dependency must FAIL",
    "lint_arch positive control: system -> rh-snapshot must FAIL",
    "lint_arch positive control: system -> system must FAIL",
    "lint_forbidden control: a declared f32_field escape is accepted and counted (SPR-010 T-04)",
    "lint_forbidden positive control: float threshold in a plane-A crate must FAIL (SPR-010 T-04)",
    "lint_forbidden positive control: workspace plane-A crate must FAIL",
    "rh-ids gates by name: V-DET-16 fork trees, V-DET-20 stream independence, PR tier (SPR-002)",
    "uv lock --check positive control: drifted lock must FAIL",
)

# Functions the chain calls, each running a loop of controls. Their lists are pinned below
# (fixed point) and in test_lint_forbidden.py (FORBIDDEN_API_CONTROLS). Alphabetical.
REQUIRED_CHAIN_FUNCTIONS = (
    "fixed_point_compile_controls",
    "forbidden_api_controls",
)


def _chain_lines() -> list[str]:
    """Return the non-comment lines from the first top-level `run` to `status=$?`."""
    lines = SCRIPT.read_text(encoding="utf-8").splitlines()
    start = next(i for i, line in enumerate(lines) if line.startswith("run "))
    end = lines.index("status=$?")
    return [line for line in lines[start:end] if not line.lstrip().startswith("#")]


def test_the_inventory_lists_are_alphabetical_and_unique() -> None:
    """Sorted lists make a missing or duplicated entry easy to see in review."""
    for listed in (REQUIRED_GATES, REQUIRED_CHAIN_FUNCTIONS):
        assert list(listed) == sorted(set(listed))


def test_every_required_gate_runs_in_the_chain() -> None:
    """Deleting, commenting out or moving a gate out of the chain fails here."""
    chained = {m.group("name") for line in _chain_lines() if (m := RUN_NAME.match(line))}
    assert sorted(set(REQUIRED_GATES) - chained) == []


def test_every_chained_gate_is_in_the_inventory() -> None:
    """A new gate must be added to the inventory in the same change, or it is unguarded."""
    chained = {m.group("name") for line in _chain_lines() if (m := RUN_NAME.match(line))}
    assert sorted(chained - set(REQUIRED_GATES)) == []


def test_every_control_loop_is_called_from_the_chain() -> None:
    """The loop functions run many controls; each must still be called by the chain."""
    calls = {line.strip().removesuffix("&&").strip() for line in _chain_lines()}
    assert sorted(set(REQUIRED_CHAIN_FUNCTIONS) - calls) == []


def test_the_fixed_point_loop_covers_every_compile_fail_crate() -> None:
    """Each fixture crate of tests/lint_fixtures/compile_fail/fixed_point/ has its control."""
    script = SCRIPT.read_text(encoding="utf-8")
    listed = script.split("FIXED_POINT_COMPILE_FAIL='", 1)[1].split("'", 1)[0]
    crates = {line.split("|", 1)[0] for line in listed.splitlines() if line.strip()}
    on_disk = {p.parent.name for p in FIXED_POINT_FIXTURE.glob("*/Cargo.toml")}
    assert crates == on_disk - {"fixed-ok"}
    assert "-p fixed-ok --target-dir" in script, "the well-typed control must still compile"
