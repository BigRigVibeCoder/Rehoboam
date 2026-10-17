"""Self-test of bin/check_all.sh's own verdict and chain (VER-SPR-008-TEST GAP-3, GAP-7).

`check_all.sh` once exited with the status of its chain's last command, so a gate line ending in
`|| true` stayed green (GAP-3), and `|| exit 0` ended it with status 0 and no verdict (GAP-7). The
verdict is now computed in an EXIT trap (`finish`), and a lint refuses those forms; this script
checks both:

- `verdict [--or-true | --or-exit | --early-exit]`: runs check_all.sh's real `run()` and `finish()`
  functions and its EXIT trap and chain end (cut from the script, not copied) on a planted chain
  in a temporary directory, and prints the planted report. Plain, both planted gates pass and the
  verdict is PASS (exit 0). With `--or-true` the first gate fails and its line ends in `|| true`;
  with `--or-exit` it fails and its line ends in `|| exit 0`; with `--early-exit` a passing gate
  is followed by `exit 0` before the chain's end. Each must be FAIL (exit 1).
  `--part-core` and `--part-golden` (SPR-009 review 2) plant a two-part chain whose golden gate
  fails, and run it with `--part core` (PASS, 1 check: the golden part is skipped and says so) and
  `--part golden` (FAIL at the golden gate): the verdict works per part.
- `lint FILE [--plant-or-true | --plant-or-exit]`: refuses a `run` line of the chain that ends in
  `|| true`, `|| :` or `|| exit ...` (exit 1, naming the line). The plant flags append `|| true`
  or `|| exit 0` to the chain's first `run` line in memory, as the planted controls.
- `abs-target [--plant-pwd-join]` (VER-SPR-008-TEST DEF-3): runs check_all.sh's real
  `baseline_push_control` and its real `GOLDEN` definition (cut from the script) with an absolute
  `TARGET_DIR` in a temporary directory, the committed baseline standing in for the recomputed
  ladder. Without `[C3]` the control must fail with the C3 message, with `[C3]` it must pass; any
  other outcome (a missing file, say) exits 1. `--plant-pwd-join` restores the old
  `$PWD/$GOLDEN` join in memory, which must then fail for the wrong reason.

Usage: python -I bin/check_all_selftest.py verdict [--or-true | --or-exit | --early-exit |
                                                    --part-core | --part-golden]
       python -I bin/check_all_selftest.py lint bin/check_all.sh [--plant-or-true | --plant-or-exit]
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
OR_TRUE = re.compile(r"\|\|\s*(true|:|exit(\s+\$?\w+)?)\s*(&&)?\s*$")
LATER = '\nrun "planted later gate" pass true'
PARTS = (
    'part core &&\nrun "planted core gate" pass true &&\n'
    'part golden &&\nrun "planted failing golden gate" pass false'
)
PLANTED_CHAINS = {
    "": 'run "planted gate" pass true &&' + LATER,
    "--or-true": 'run "planted failing gate" pass false || true &&' + LATER,
    "--or-exit": 'run "planted failing gate" pass false || exit 0 &&' + LATER,
    "--early-exit": 'run "planted gate" pass true && exit 0' + LATER,
    "--part-core": PARTS,
    "--part-golden": PARTS,
}
PLANTS = {"--plant-or-true": " || true &&", "--plant-or-exit": " || exit 0 &&"}


def chain(lines: list[str]) -> list[tuple[int, str]]:
    """Return the chain's lines (1-based), from the first top-level `run` or `part` to the end."""
    start = next(i for i, line in enumerate(lines) if line.startswith(("run ", "part ")))
    end = lines.index("status=$?")
    return [(i + 1, lines[i]) for i in range(start, end)]


def lint(text: str, *, plant: str = "") -> list[str]:
    """Return one finding per chain line that ends in `|| true`, `|| :` or `|| exit ...`.

    `plant` (a `PLANTS` suffix) is first appended to the chain's first `run` line.
    """
    lines = text.splitlines()
    if plant:
        first = chain(lines)[0][0] - 1
        lines[first] = lines[first].removesuffix(" &&") + plant
    return [
        f"line {n}: {line.strip()[:100]}"
        for n, line in chain(lines)
        if not line.lstrip().startswith("#") and OR_TRUE.search(line)
    ]


def _function(lines: list[str], name: str) -> list[str]:
    start = lines.index(f"{name}() {{")
    return lines[start : lines.index("}", start) + 1]


def _cut(text: str) -> tuple[str, str]:
    """Return check_all.sh's `run()`, `finish()`, the trap's lines, and the chain's end."""
    lines = text.splitlines()
    trap = [line for line in lines if line in ('chain_done=""', "trap finish EXIT")]
    head = _function(lines, "run") + _function(lines, "part") + _function(lines, "finish") + trap
    return "\n".join(head), "\n".join(lines[lines.index("status=$?") :])


def verdict(mode: str = "") -> int:
    """Run a planted chain through check_all.sh's own verdict code; return its status.

    The chain is `PLANTED_CHAINS[mode]`, between check_all.sh's `run()`, `finish()`, EXIT trap
    and chain end.
    """
    run_fn, tail = _cut(SCRIPT.read_text(encoding="utf-8"))
    planted = PLANTED_CHAINS[mode]
    with tempfile.TemporaryDirectory() as tmp:
        report = Path(tmp) / "report.md"
        script = "\n".join(
            [
                "set -u",
                f"REPORT='{report}'",
                f"PART='{mode.removeprefix('--part-') if mode.startswith('--part-') else ''}'",
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


def _lint(file: str, *, plant: str) -> int:
    found = lint(Path(file).read_text(encoding="utf-8"), plant=plant)
    for f in found:
        print(f"OR-TRUE ON A RUN LINE: {f}")
    if found:
        return 1
    print(f"NO OR-TRUE: no run line of {file} ends in || true, || : or || exit")
    return 0


def main(argv: list[str]) -> int:
    """Run the self-test named by `argv`."""
    match argv:
        case ["verdict", *flag] if flag in ([], *([m] for m in PLANTED_CHAINS if m)):
            return verdict(flag[0] if flag else "")
        case ["lint", file, *flag] if flag in ([], *([p] for p in PLANTS)):
            return _lint(file, plant=PLANTS[flag[0]] if flag else "")
        case ["abs-target", *flag] if flag in ([], ["--plant-pwd-join"]):
            return abs_target(plant=bool(flag))
    print(__doc__.split("Usage: ", 1)[1], file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
