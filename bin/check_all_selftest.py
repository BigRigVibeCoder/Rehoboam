"""Self-test of bin/check_all.sh's own verdict and chain (VER-SPR-008-TEST GAP-3).

`check_all.sh` once exited with the status of its chain's last command, so a gate line ending in
`|| true` stayed green. Two guards close that, and this script checks both:

- `verdict [--or-true]`: runs check_all.sh's real `run()` function and its real verdict code (cut
  from the script, not copied) on a planted two-gate chain in a temporary directory, and prints
  the planted report. The first gate fails; with `--or-true` its line ends in `|| true`, so the
  chain goes on and its last command succeeds. The verdict must still be FAIL (exit 1). Without
  `--or-true`, both planted gates pass and the verdict is PASS (exit 0).
- `lint FILE [--plant-or-true]`: refuses a `run` line of the chain that ends in `|| true` or
  `|| :` (exit 1, naming the line). `--plant-or-true` appends `|| true` to the chain's first `run`
  line in memory, as the planted control.

Usage: python -I bin/check_all_selftest.py verdict [--or-true]
       python -I bin/check_all_selftest.py lint bin/check_all.sh [--plant-or-true]
"""

from __future__ import annotations

import re
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "check_all.sh"
OR_TRUE = re.compile(r"\|\|\s*(true|:)\s*(&&)?\s*$")


def chain(lines: list[str]) -> list[tuple[int, str]]:
    """Return the chain's lines (1-based), from the first top-level `run` to `status=$?`."""
    start = next(i for i, line in enumerate(lines) if line.startswith("run "))
    end = lines.index("status=$?")
    return [(i + 1, lines[i]) for i in range(start, end)]


def lint(text: str, *, plant: bool = False) -> list[str]:
    """Return one finding per chain line that ends in `|| true` or `|| :`."""
    lines = text.splitlines()
    if plant:
        first = chain(lines)[0][0] - 1
        lines[first] = lines[first].removesuffix(" &&") + " || true &&"
    return [
        f"line {n}: {line.strip()[:100]}"
        for n, line in chain(lines)
        if not line.lstrip().startswith("#") and OR_TRUE.search(line)
    ]


def _cut(text: str) -> tuple[str, str]:
    """Return check_all.sh's `run()` definition and its verdict code (from `status=$?`)."""
    lines = text.splitlines()
    start = lines.index("run() {")
    end = lines.index("}", start)
    tail = lines[lines.index("status=$?") :]
    return "\n".join(lines[start : end + 1]), "\n".join(tail)


def verdict(*, or_true: bool) -> int:
    """Run the planted chain through check_all.sh's own `run()` and verdict; return its status."""
    run_fn, tail = _cut(SCRIPT.read_text(encoding="utf-8"))
    if or_true:
        planted = (
            'run "planted failing gate" pass false || true &&\nrun "planted later gate" pass true'
        )
    else:
        planted = 'run "planted gate" pass true &&\nrun "planted later gate" pass true'
    with tempfile.TemporaryDirectory() as tmp:
        report = Path(tmp) / "report.md"
        script = "\n".join(
            [
                "set -u",
                f"REPORT='{report}'",
                ': > "$REPORT"',
                'n=0; failed=""',
                run_fn,
                planted,
                tail,
            ]
        )
        path = Path(tmp) / "planted_check_all.sh"
        path.write_text(script + "\n", encoding="utf-8")
        done = subprocess.run(["bash", str(path)], capture_output=True, text=True, check=False)
        print(done.stdout, end="")
        print(report.read_text(encoding="utf-8"), end="")
    return done.returncode


def main(argv: list[str]) -> int:
    """Run the self-test named by `argv`."""
    match argv:
        case ["verdict"]:
            return verdict(or_true=False)
        case ["verdict", "--or-true"]:
            return verdict(or_true=True)
        case ["lint", file, *rest] if rest in ([], ["--plant-or-true"]):
            found = lint(Path(file).read_text(encoding="utf-8"), plant=bool(rest))
            for f in found:
                print(f"OR-TRUE ON A RUN LINE: {f}")
            if found:
                return 1
            print(f"NO OR-TRUE: no run line of {file} ends in || true or || :")
            return 0
    print(__doc__.split("Usage: ", 1)[1], file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
