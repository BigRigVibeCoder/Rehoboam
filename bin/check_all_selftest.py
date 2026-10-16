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
- `abs-target [--plant-pwd-join]` (VER-SPR-008-TEST DEF-3): runs check_all.sh's real
  `baseline_push_control` and its real `GOLDEN` definition (cut from the script) with an absolute
  `TARGET_DIR` in a temporary directory, the committed baseline standing in for the recomputed
  ladder. Without `[C3]` the control must fail with the C3 message, with `[C3]` it must pass; any
  other outcome (a missing file, say) exits 1. `--plant-pwd-join` restores the old
  `$PWD/$GOLDEN` join in memory, which must then fail for the wrong reason.

Usage: python -I bin/check_all_selftest.py verdict [--or-true]
       python -I bin/check_all_selftest.py lint bin/check_all.sh [--plant-or-true]
       python -I bin/check_all_selftest.py abs-target [--plant-pwd-join]
"""

from __future__ import annotations

import re
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "check_all.sh"
REPO = SCRIPT.parent.parent
JOINED = 'ladder="$GOLDEN/smoke_yearly_t1.ladder"'
C3_MISSING = "BASELINE CHANGED in a PR not marked [C3]"
C3_PRESENT = "regenerated in a [C3] PR"
BASELINE = "worlds/smoke/baselines/smoke_yearly_1000y.ladder"
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


def abs_target(*, plant: bool) -> int:
    """Run the two baseline push controls with an absolute TARGET_DIR; 0 when both behave."""
    text = SCRIPT.read_text(encoding="utf-8")
    if plant:
        text = text.replace(JOINED, JOINED.replace('"$GOLDEN', '"$PWD/$GOLDEN'), 1)
    lines = text.splitlines()
    start = lines.index("baseline_push_control() {")
    function = "\n".join(lines[start : lines.index("}", start) + 1])
    golden = [
        line for line in lines if line.startswith(('mkdir -p "$TARGET_DIR/golden"', "GOLDEN="))
    ]
    outcomes = []
    with tempfile.TemporaryDirectory() as tmp:
        script = "\n".join(
            [
                "set -u",
                f"TARGET_DIR='{Path(tmp).resolve() / 'target'}'",
                f"PY='{sys.executable}'",
                *golden,
                f'cp {BASELINE} "$GOLDEN/smoke_yearly_t1.ladder"',
                function,
                'baseline_push_control "$1"',
            ]
        )
        path = Path(tmp) / "abs_target.sh"
        path.write_text(script + "\n", encoding="utf-8")
        for subject, expect_rc, expect in (
            ("feat: regenerate the baseline", 1, C3_MISSING),
            ("feat: regenerate the baseline [C3]", 0, C3_PRESENT),
        ):
            done = subprocess.run(
                ["bash", str(path), subject], cwd=REPO, capture_output=True, text=True, check=False
            )
            out = done.stdout + done.stderr
            ok = (done.returncode == 0) == (expect_rc == 0) and expect in out
            outcomes.append(ok)
            tail = out.strip()[-200:]
            print(f"{'OK' if ok else 'WRONG'}: {subject!r} -> exit {done.returncode}: {tail}")
    if all(outcomes):
        print("ABSOLUTE TARGET DIR: both baseline push controls fail and pass for the right reason")
        return 0
    print("ABSOLUTE TARGET DIR: a baseline push control behaved wrongly (VER-SPR-008-TEST DEF-3)")
    return 1


def _lint(file: str, *, plant: bool) -> int:
    found = lint(Path(file).read_text(encoding="utf-8"), plant=plant)
    for f in found:
        print(f"OR-TRUE ON A RUN LINE: {f}")
    if found:
        return 1
    print(f"NO OR-TRUE: no run line of {file} ends in || true or || :")
    return 0


def main(argv: list[str]) -> int:
    """Run the self-test named by `argv`."""
    match argv:
        case ["verdict", *flag] if flag in ([], ["--or-true"]):
            return verdict(or_true=bool(flag))
        case ["lint", file, *flag] if flag in ([], ["--plant-or-true"]):
            return _lint(file, plant=bool(flag))
        case ["abs-target", *flag] if flag in ([], ["--plant-pwd-join"]):
            return abs_target(plant=bool(flag))
    print(__doc__.split("Usage: ", 1)[1], file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
