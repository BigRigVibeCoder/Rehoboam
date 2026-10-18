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
  `--empty-fail` (VER-SPR-012-TEST GAP-6) plants a failing gate whose expectation is `fail:` with
  no text, which would match any failure: `run` refuses it as a harness error, and the verdict is
  FAIL (exit 1). `--blank-fail` (GAP-9, BCK-002 A-53) does the same with `fail:` followed by
  spaces only, which is empty once trimmed. `--scope-incomplete` (PR #89's review C2) plants a
  control that fails with its own expected text but also prints `SCOPE INCOMPLETE` (it read an
  undeclared input of a path-scoped gate): `run` fails it, and the verdict is FAIL (exit 1).
  `--part-core`, `--part-golden` and `--part-stream` (SPR-009 review 2; BCK-002 A-63) plant a
  three-part chain whose golden gate fails, and run it with `--part core` and `--part stream`
  (PASS, 1 check each: the other parts are skipped and say so) and `--part golden` (FAIL at the
  golden gate): the verdict works per part. `--all-parts` plants a three-part chain whose only
  failing gate is the stream part's and runs it with no `--part`: FAIL at that gate, so a run with
  no flag reaches every part.
- `lint FILE [--plant-or-true | --plant-or-exit]`: refuses a `run` line of the chain that ends in
  `|| true`, `|| :` or `|| exit ...` (exit 1, naming the line). The plant flags append `|| true`
  or `|| exit 0` to the chain's first `run` line in memory, as the planted controls.
- `abs-target [--plant-pwd-join]` (VER-SPR-008-TEST DEF-3): runs check_all.sh's real
  `baseline_push_control` and its real `GOLDEN` definition (cut from the script) with an absolute
  `TARGET_DIR` in a temporary directory, the committed baseline standing in for the recomputed
  ladder. Without `[C3]` the control must fail with the C3 message, with `[C3]` it must pass; any
  other outcome (a missing file, say) exits 1. `--plant-pwd-join` restores the old
  `$PWD/$GOLDEN` join in memory, which must then fail for the wrong reason.
- `baseline-controls [--plant-committed]` (BCK-002 A-66): runs every `Golden baseline` control of
  the chain that calls `baseline_control`, `baseline_hand_edited_control` or
  `baseline_push_control` (their real definitions, cut from the script) with its own expectation,
  on a stale stand-in: each world's recomputation is its committed baseline with the last rung's
  hash changed, as on a C3 head before its regeneration commit. Each must still fail or pass for
  its own reason (exit 0, `BASELINE CONTROLS: all ...`). `--plant-committed` feeds the controls the
  committed baseline in memory, as before A-66: they then fail with BASELINE MISMATCH, not their
  own message (exit 1, `BASELINE CONTROLS: WRONG ...`).
- `inventory [--plant-unlisted | --plant-dropped]` (BCK-002 A-65): the gate inventory both ways.
  Every `run` gate of the chain must be listed in python/tests/test_check_all_inventory.py's
  REQUIRED_GATES and every listed gate chained, each once; every other chain call must be a
  listed control loop (REQUIRED_CHAIN_FUNCTIONS) and every listed loop called; and no `run` line
  may sit outside the chain and those loops. The lists are read with `ast`, not imported.
  `--plant-unlisted` adds a gate to the chain in memory; `--plant-dropped` deletes, in memory, the
  lines of the wrapped inventory entry a three-way merge once dropped (SPR-026's merge of PR #79).
  Each must FAIL with `UNLISTED:`.

Usage: python -I bin/check_all_selftest.py verdict [--or-true | --or-exit | --early-exit |
                                                    --empty-fail | --blank-fail | --part-core |
                                                    --part-golden | --part-stream | --all-parts |
                                                    --scope-incomplete]
       python -I bin/check_all_selftest.py lint bin/check_all.sh [--plant-or-true | --plant-or-exit]
       python -I bin/check_all_selftest.py abs-target [--plant-pwd-join]
       python -I bin/check_all_selftest.py baseline-controls [--plant-committed]
       python -I bin/check_all_selftest.py inventory [--plant-unlisted | --plant-dropped]
"""

from __future__ import annotations

import ast
import re
import shlex
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
    'part golden &&\nrun "planted failing golden gate" pass false &&\n'
    'part stream &&\nrun "planted stream gate" pass true'
)
STREAM_FAILS = (
    'part core &&\nrun "planted core gate" pass true &&\n'
    'part golden &&\nrun "planted golden gate" pass true &&\n'
    'part stream &&\nrun "planted failing stream gate" pass false'
)
PLANTED_CHAINS = {
    "": 'run "planted gate" pass true &&' + LATER,
    "--or-true": 'run "planted failing gate" pass false || true &&' + LATER,
    "--or-exit": 'run "planted failing gate" pass false || exit 0 &&' + LATER,
    "--early-exit": 'run "planted gate" pass true && exit 0' + LATER,
    "--empty-fail": 'run "planted gate with an empty expectation" "fail:" false &&' + LATER,
    "--blank-fail": 'run "planted gate with a blank expectation" "fail:   " false &&' + LATER,
    "--scope-incomplete": (
        'run "planted control whose output says SCOPE INCOMPLETE" "fail:STALE:" '
        "sh -c 'echo STALE: x; echo SCOPE INCOMPLETE: y; exit 1' &&" + LATER
    ),
    "--part-core": PARTS,
    "--part-golden": PARTS,
    "--part-stream": PARTS,
    "--all-parts": STREAM_FAILS,
}
PLANTS = {"--plant-or-true": " || true &&", "--plant-or-exit": " || exit 0 &&"}
RUN_NAME = re.compile(r'^\s*run "(?P<name>[^"]*)"')
INVENTORY = REPO / "python" / "tests" / "test_check_all_inventory.py"
PLANTED_UNLISTED = 'run "planted unlisted gate (BCK-002 A-65)" pass true &&'
# The wrapped entry SPR-026's three-way merge of PR #79 dropped from the inventory (BCK-002 A-65).
DROPPED = (
    "lint_odd next positive control: an amendment whose text changed since the committed lock "
    "must FAIL (SPR-015)"
)
# Chain lines that open a part or a path scope (BCK-002 A-64) rather than call a control loop.
OPENERS = ("part", "scope")
# BCK-002 A-66: the functions behind the Golden baseline controls (the first gives their input),
# the line that derives that input from the recomputation, and the plant that restores the
# committed baseline in its place; and the worlds whose stale stand-ins the controls run on.
BASELINE_FUNCTIONS = (
    "baseline_control_input",
    "baseline_control",
    "baseline_hand_edited_control",
    "baseline_push_control",
)
BASELINE_INPUT = 'cp "$GOLDEN/$1_yearly_t1.ladder" "$copy"'
BASELINE_COMMITTED = 'cp worlds/"$1"/baselines/*.ladder "$copy"'
STALE_WORLDS = {
    "smoke": "worlds/smoke/baselines/smoke_yearly_1000y.ladder",
    "solo": "worlds/solo/baselines/solo_yearly_200y.ladder",
}


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
    function = "\n".join(
        _function(lines, "baseline_control_input") + _function(lines, "baseline_push_control")
    )
    golden = _golden_lines(lines)
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


def _golden_lines(lines: list[str]) -> list[str]:
    """check_all.sh's own lines that define $GOLDEN (under $TARGET_DIR)."""
    return [line for line in lines if line.startswith(('mkdir -p "$TARGET_DIR/golden"', "GOLDEN="))]


def _expectation_met(expect: str, status: int, out: str) -> bool:
    """check_all.sh's `run` rule: pass | fail | pass:<text> | fail:<text>."""
    kind, _, text = expect.partition(":")
    return (status == 0) == (kind == "pass") and text in out


def _stale(baseline: str) -> str:
    """A committed baseline with the last rung's state hash changed: a recomputation it misses."""
    lines = baseline.splitlines(keepends=True)
    last = max(i for i, line in enumerate(lines) if line.startswith("rung "))
    words = lines[last].rstrip("\n").split(" ")
    words[-1] = words[-1][:-1] + ("0" if words[-1][-1] != "0" else "1")
    lines[last] = " ".join(words) + "\n"
    return "".join(lines)


def _baseline_control_calls(lines: list[str]) -> list[tuple[str, str, list[str]]]:
    """The chain's Golden baseline controls: (name, expectation, function call words)."""
    controls = []
    for _, line in chain(lines):
        if not line.startswith("run "):
            continue
        words = shlex.split(line.strip().removesuffix("&&"))
        if len(words) > len(("run", "NAME", "EXPECT")) and words[3] in BASELINE_FUNCTIONS[1:]:
            controls.append((words[1], words[2], words[3:]))
    return controls


def baseline_controls(*, plant: bool) -> int:
    """Run the chain's Golden baseline controls on a stale baseline; 0 when each behaves."""
    text = SCRIPT.read_text(encoding="utf-8")
    if plant:
        if BASELINE_INPUT not in text:
            print(f"HARNESS ERROR: {BASELINE_INPUT!r} is not in check_all.sh")
            return 2
        text = text.replace(BASELINE_INPUT, BASELINE_COMMITTED, 1)
    lines = text.splitlines()
    functions = [line for name in BASELINE_FUNCTIONS for line in _function(lines, name)]
    controls = _baseline_control_calls(lines)
    wrong = 0
    with tempfile.TemporaryDirectory() as tmp:
        target = Path(tmp).resolve() / "target"
        (target / "golden").mkdir(parents=True)
        for world, baseline in STALE_WORLDS.items():
            stale = _stale((REPO / baseline).read_text(encoding="utf-8"))
            (target / "golden" / f"{world}_yearly_t1.ladder").write_text(stale, encoding="utf-8")
        script = Path(tmp) / "baseline_controls.sh"
        head = ["set -u", f"TARGET_DIR='{target}'", f"PY='{sys.executable}'"]
        body = [*head, *_golden_lines(lines), *functions, '"$@"']
        script.write_text("\n".join(body) + "\n", encoding="utf-8")
        for name, expect, call in controls:
            done = subprocess.run(
                ["bash", str(script), *call], cwd=REPO, capture_output=True, text=True, check=False
            )
            out = done.stdout + done.stderr
            ok = _expectation_met(expect, done.returncode, out)
            wrong += not ok
            print(
                f"{'OK' if ok else 'WRONG'}: {name[:90]} -> exit {done.returncode}: {out[-160:]!r}"
            )
    if not controls or wrong:
        print(f"BASELINE CONTROLS: WRONG: {wrong} of {len(controls)} wrong on a stale baseline")
        return 1
    print(f"BASELINE CONTROLS: all {len(controls)} behave on a stale baseline, each for its reason")
    return 0


def listed(source: str) -> tuple[tuple[str, ...], tuple[str, ...]]:
    """REQUIRED_GATES and REQUIRED_CHAIN_FUNCTIONS of the inventory test's source, by `ast`."""
    found: dict[str, tuple[str, ...]] = {}
    for node in ast.parse(source).body:
        if (
            isinstance(node, ast.Assign)
            and len(node.targets) == 1
            and isinstance(target := node.targets[0], ast.Name)
            and target.id in ("REQUIRED_GATES", "REQUIRED_CHAIN_FUNCTIONS")
        ):
            found[target.id] = tuple(ast.literal_eval(node.value))
    return found.get("REQUIRED_GATES", ()), found.get("REQUIRED_CHAIN_FUNCTIONS", ())


def _bodies(lines: list[str], names: tuple[str, ...]) -> set[int]:
    """0-based line numbers of the named functions' definitions (absent ones give none)."""
    at: set[int] = set()
    for name in names:
        if f"{name}() {{" in lines:
            start = lines.index(f"{name}() {{")
            at |= set(range(start, lines.index("}", start) + 1))
    return at


def _chain_findings(names: list[str], gates: tuple[str, ...]) -> list[str]:
    """The run gates of the chain against the listed gates, both ways, and duplicates."""
    found = [f"UNLISTED: {g}" for g in sorted(set(names) - set(gates))]
    found += [f"NOT IN THE CHAIN: {g}" for g in sorted(set(gates) - set(names))]
    found += [f"TWICE IN THE CHAIN: {g}" for g in sorted({g for g in names if names.count(g) > 1})]
    found += [f"LISTED TWICE: {g}" for g in sorted({g for g in gates if gates.count(g) > 1})]
    return found


def inventory(text: str, gates: tuple[str, ...], loops: tuple[str, ...]) -> list[str]:
    """Every way the chain and the inventory disagree, one finding each (empty: they agree)."""
    lines = text.splitlines()
    body = [(n, line) for n, line in chain(lines) if not line.lstrip().startswith("#")]
    runs = [(n, line) for n, line in body if line.lstrip().startswith("run ")]
    names = [m.group("name") for _, line in runs if (m := RUN_NAME.match(line))]
    calls = [
        line.strip().removesuffix("&&").strip()
        for _, line in body
        if line.strip() and not line.lstrip().startswith("run ")
    ]
    found = _chain_findings(names, gates)
    found += [
        f"UNPARSED RUN LINE: line {n}: {line.strip()[:80]}"
        for n, line in runs
        if not RUN_NAME.match(line)
    ]
    found += [
        f"UNLISTED CALL: {c}" for c in calls if c not in loops and c.split()[0] not in OPENERS
    ]
    found += [f"LOOP NOT CALLED: {loop}" for loop in loops if loop not in calls]
    inside = {n - 1 for n, _ in body} | _bodies(lines, loops)
    found += [
        f"RUN OUTSIDE THE CHAIN: line {i + 1}: {line.strip()[:80]}"
        for i, line in enumerate(lines)
        if RUN_NAME.match(line) and i not in inside
    ]
    return found


def _drop_entry(source: str, entry: str) -> str:
    """``source`` with the physical lines of the string constant ``entry`` deleted."""
    for node in ast.walk(ast.parse(source)):
        if isinstance(node, ast.Constant) and node.value == entry and node.end_lineno:
            lines = source.splitlines(keepends=True)
            del lines[node.lineno - 1 : node.end_lineno]
            return "".join(lines)
    return source


def _inventory(plant: str) -> int:
    text = SCRIPT.read_text(encoding="utf-8")
    source = INVENTORY.read_text(encoding="utf-8")
    if plant == "--plant-unlisted":
        text = text.replace("part core &&\n", f"part core &&\n{PLANTED_UNLISTED}\n", 1)
    elif plant == "--plant-dropped":
        dropped = _drop_entry(source, DROPPED)
        if dropped == source:
            print(f"HARNESS ERROR: the entry {DROPPED!r} is not in {INVENTORY.name}")
            return 2
        source = dropped
    gates, loops = listed(source)
    found = inventory(text, gates, loops)
    for f in found:
        print(f)
    if found:
        print(f"INVENTORY: {len(found)} finding(s) between {SCRIPT.name} and {INVENTORY.name}")
        return 1
    print(
        f"INVENTORY OK: {len(gates)} gates chained and listed, each once, both ways; "
        f"{len(loops)} control loop(s) called; no run line outside the chain and its loops"
    )
    return 0


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
        case ["baseline-controls", *flag] if flag in ([], ["--plant-committed"]):
            return baseline_controls(plant=bool(flag))
        case ["inventory", *flag] if flag in ([], ["--plant-unlisted"], ["--plant-dropped"]):
            return _inventory(flag[0] if flag else "")
    print(__doc__.split("Usage: ", 1)[1], file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
