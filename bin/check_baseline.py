"""Golden baseline integrity: a baseline can only be regenerated, never typed (SPR-008 T-04).

CI recomputes the smoke ladder on every PR head (`rh-cli run smoke`, at 1 thread and at N), and
this gate compares the committed baseline with the recomputation (SPR-008 ruling 6):

- the recomputed ladders must equal each other byte for byte (thread-count invariance, V-DET-01);
- whether or not the baseline changed in this PR, it must equal the recomputation byte for byte,
  so a hand-edited baseline fails, and so does a baseline left stale by a change in history (run
  the bisect: `docs/13` §4.5);
- if the baseline changed against the base branch, the PR title must carry `[C3]`: only a C3 PR
  may record a new history, and its own final commit writes the baseline with the golden-run tool
  (`bin/regen_baseline.sh`; the fallback of `docs/13` §4.4, there being no merge queue).

Whether the baseline changed is `git diff --quiet <base> -- <baseline>` (`--base-ref`, default
`$RH_BASELINE_BASE`); with no base (a local run, a push to main) it counts as unchanged.
`--changed yes|no` overrides it, for the planted controls. The comparison is with the working
tree, so a regenerated but uncommitted baseline counts too, provided git tracks the file (CI
checks out the PR head, where it is committed). The title is `--title`, default
`$RH_PR_TITLE`.

Usage:  python -I bin/check_baseline.py --baseline B --recomputed R [R ...]
            [--base-ref REF | --changed yes|no] [--title TITLE]
Exit:   0 the baseline holds; 1 it does not; 2 a file or the base ref cannot be read.
Standard library only (GOV-003 §3).

Refs: SPR-008 T-04, ruling 6; docs/13 §4.4, §4.5; docs/09 V-DET-01.
"""

import argparse
import os
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path

EXIT_OK = 0
EXIT_FAIL = 1
EXIT_UNREADABLE = 2
C3_MARK = "[C3]"


class UnreadableError(Exception):
    """A file or the base ref could not be read."""


def first_difference(a: bytes, b: bytes) -> str:
    """Return the 1-based line where two texts first differ, and the two lines."""
    la, lb = a.decode("utf-8", "replace").split("\n"), b.decode("utf-8", "replace").split("\n")
    for i, (x, y) in enumerate(zip(la, lb, strict=False)):
        if x != y:
            return f"line {i + 1}: `{x}` vs `{y}`"
    return f"line {min(len(la), len(lb)) + 1}: one file ends first"


def changed_against(base: str, baseline: Path) -> bool:
    """Return whether ``baseline`` differs from its version at ``base`` (absent there counts)."""
    try:
        done = subprocess.run(
            ["git", "diff", "--quiet", base, "--", str(baseline)],
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError as err:
        raise UnreadableError("git is not on PATH") from err
    if done.returncode not in (0, 1):
        raise UnreadableError(f"git diff against {base} failed: {done.stderr.strip()}")
    return done.returncode == 1


def read(path: Path) -> bytes:
    """Return a file's bytes."""
    try:
        return path.read_bytes()
    except OSError as err:
        raise UnreadableError(f"{path}: {err}") from err


def check(
    baseline: Path, recomputed: Sequence[Path], changed: bool, title: str
) -> tuple[bool, str]:
    """Return whether the baseline holds, and the report line."""
    runs = [(p, read(p)) for p in recomputed]
    first_path, first = runs[0]
    for path, text in runs[1:]:
        if text != first:
            return False, (
                f"RECOMPUTATIONS DIFFER: {path} vs {first_path}, {first_difference(first, text)}: "
                "the thread count reached the history (V-DET-01)"
            )
    if not baseline.is_file():
        return False, (
            f"NO BASELINE: {baseline} is missing; regenerate it with bin/regen_baseline.sh "
            "in a [C3] PR"
        )
    committed = read(baseline)
    if committed != first:
        return False, (
            f"BASELINE MISMATCH: {baseline} differs from the recomputation {first_path}, "
            f"{first_difference(committed, first)}. A baseline is regenerated, never typed; if "
            "the history changed, run rh-cli bisect and explain it (docs/13 §4.5)"
        )
    if changed and C3_MARK not in title:
        return False, (
            f"BASELINE CHANGED in a PR not marked {C3_MARK}: {baseline} differs from the base "
            f"branch, and the title is `{title}`. Only a C3 PR records a new history "
            "(SPR-008 ruling 6)"
        )
    how = f"regenerated in a {C3_MARK} PR" if changed else "unchanged"
    return True, (
        f"BASELINE OK: {baseline} equals the recomputation ({len(runs)} run(s) agree; {how})"
    )


def main(argv: Sequence[str] | None = None) -> int:
    """Run the gate and return the process exit status."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--recomputed", type=Path, nargs="+", required=True)
    parser.add_argument("--base-ref", default=os.environ.get("RH_BASELINE_BASE", ""))
    parser.add_argument("--changed", choices=("yes", "no"))
    parser.add_argument("--title", default=os.environ.get("RH_PR_TITLE", ""))
    args = parser.parse_args(argv)
    try:
        if args.changed is not None:
            changed = args.changed == "yes"
        elif args.base_ref:
            changed = changed_against(args.base_ref, args.baseline)
        else:
            changed = False
        ok, line = check(args.baseline, args.recomputed, changed, args.title)
    except UnreadableError as err:
        print(f"check_baseline: {err}", file=sys.stderr)
        return EXIT_UNREADABLE
    print(line)
    return EXIT_OK if ok else EXIT_FAIL


if __name__ == "__main__":
    sys.exit(main())
