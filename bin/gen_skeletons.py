"""Writes or checks the generated system skeletons that rh-core compiles against (SPR-005 T-03).

``specs/systems/lint_odd.py --rust <id>`` turns each system spec into a Rust skeleton (system id,
schedule, RNG-purpose enum, parameter struct; ADR-0033 §3). The skeletons are committed under
``crates/rh-core/tests/generated/<id with dots as underscores>.rs`` and compiled by
``crates/rh-core/tests/skeletons.rs`` against the ``SystemModel`` contract. Generated code is never
hand-edited (GOV-003 §2): ``--check`` regenerates every skeleton and fails on any difference, so a
spec change that is not regenerated, or a hand edit, turns the PR tier red.

Usage:  python -I bin/gen_skeletons.py           write every skeleton
        python -I bin/gen_skeletons.py --check   compare; exit 1 and print STALE/MISSING/EXTRA
        python -I bin/gen_skeletons.py --check --out <dir>   compare against another directory
Exit:   0 up to date (or written), 1 differences, 2 the generator failed.
Standard library only (GOV-003 §3).

Refs: SPR-005 T-03; ADR-0033 §3; GOV-003 §2 (generated code: CI regenerate-and-diff).
"""

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPECS = ROOT / "specs" / "systems"
LINT_ODD = SPECS / "lint_odd.py"
DEFAULT_OUT = ROOT / "crates" / "rh-core" / "tests" / "generated"

EXIT_OK = 0
EXIT_DIFFERENT = 1
EXIT_GENERATOR = 2


def spec_ids() -> list[str]:
    """Return the system ids of every spec (``*.odd.toml``, templates excluded), in file order."""
    return sorted(
        p.name.removesuffix(".odd.toml")
        for p in SPECS.glob("*.odd.toml")
        if not p.name.startswith("_")
    )


def file_name(system_id: str) -> str:
    """Return the skeleton's file name: dots become underscores (``M2.demography`` -> ...)."""
    return system_id.replace(".", "_") + ".rs"


def generate(system_id: str) -> str:
    """Return ``lint_odd.py --rust <id>`` output, with a trailing newline."""
    done = subprocess.run(
        [sys.executable, "-I", str(LINT_ODD), "--rust", system_id],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if done.returncode != 0:
        raise RuntimeError(f"lint_odd.py --rust {system_id} failed: {done.stdout}{done.stderr}")
    return done.stdout.rstrip("\n") + "\n"


def rel(path: Path) -> str:
    """Return ``path`` relative to the repository root when it is inside it."""
    try:
        return str(path.resolve().relative_to(ROOT))
    except ValueError:
        return str(path)


def check(out: Path, expected: dict[str, str]) -> list[str]:
    """Return one line per difference between ``out`` and the expected files."""
    problems = []
    for name, text in sorted(expected.items()):
        path = out / name
        if not path.is_file():
            problems.append(f"MISSING: {rel(path)} (run python -I bin/gen_skeletons.py)")
        elif path.read_text(encoding="utf-8") != text:
            problems.append(
                f"STALE: {rel(path)} differs from lint_odd.py --rust "
                "(regenerate it; never hand-edit generated code)"
            )
    for path in sorted(out.glob("*.rs")):
        if path.name not in expected:
            problems.append(f"EXTRA: {rel(path)} has no spec in specs/systems")
    return problems


def main(argv: list[str] | None = None) -> int:
    """Write or check the skeletons; see the module docstring."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT, help="skeleton directory")
    args = parser.parse_args(argv)
    try:
        expected = {file_name(sid): generate(sid) for sid in spec_ids()}
    except RuntimeError as e:
        print(e)
        return EXIT_GENERATOR
    out: Path = args.out
    if args.check:
        problems = check(out, expected)
        for line in problems:
            print(line)
        if problems:
            return EXIT_DIFFERENT
        print(f"SKELETONS: {len(expected)} up to date in {rel(out)}")
        return EXIT_OK
    out.mkdir(parents=True, exist_ok=True)
    for name, text in sorted(expected.items()):
        (out / name).write_text(text, encoding="utf-8")
    print(f"SKELETONS: wrote {len(expected)} to {rel(out)}")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
