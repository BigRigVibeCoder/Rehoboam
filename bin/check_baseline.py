"""Golden baseline integrity: a baseline can only be regenerated, never typed (SPR-008 T-04).

CI recomputes the smoke ladder on every PR head (`rh-cli run smoke`, at 1 thread and at N), and
this gate compares the committed baseline with the recomputation (SPR-008 ruling 6):

- the recomputed ladders must equal each other byte for byte (thread-count invariance, V-DET-01);
- whether or not the baseline changed, it must equal the recomputation byte for byte, so a
  hand-edited baseline fails, and so does a baseline left stale by a change in history (run the
  bisect: `docs/13` §4.5);
- the **C3 rule**: a baseline that changed may only come from a C3 change. There is no merge
  queue (GitHub Free), so the C3 PR's own final commit writes the baseline with the golden-run
  tool (`bin/regen_baseline.sh`; the fallback of `docs/13` §4.4).

The C3 rule fails closed (VER-SPR-008 MAJ-1). By event (`--event`, default `$RH_EVENT`, else
`local`):

- `pull_request`: the base (`--base-ref`, default `$RH_BASELINE_BASE`) is required; without one
  the gate exits 2. The baseline changed if `git diff --quiet <base> -- <baseline>` says so
  (against the working tree, which CI checks out as the PR's merge commit). Then the PR title
  (`--title`, default `$RH_PR_TITLE`) must carry `[C3]` in its trailing tag group.
- `push`: the range is `--before` (default `$RH_PUSH_BEFORE`) to `--after` (default
  `$RH_PUSH_AFTER`, else `HEAD`). An empty or all-zero `before` (a new branch) leaves the rule
  "not checked: no base", said so. If the baseline changed in the range, at least one commit
  subject in it must carry `[C3]` in its trailing tag group: the commit `bin/regen_baseline.sh`
  asks for.
- `local` (and `workflow_dispatch`, a manual CI run): with `--base-ref`, as for a pull request;
  without one, the rule is reported "not checked (no base ref)", never "unchanged".

`--changed yes|no` overrides the change detection (with the title rule), for the planted controls.

A trailing tag group is the run of `[...]` tags that ends the title or subject: `SPR-008: golden
CI [C3]` and `x [C3] [perf]` carry it; `docs: explain the [C3] rule` does not.

Usage:  python -I bin/check_baseline.py --baseline B --recomputed R [R ...]
            [--event pull_request|push|local] [--base-ref REF] [--before SHA] [--after SHA]
            [--changed yes|no] [--title TITLE]
Exit:   0 the baseline holds; 1 it does not; 2 a file, a ref or a needed base is missing.
Standard library only (GOV-003 §3).

Refs: SPR-008 T-04, ruling 6; VER-SPR-008 MAJ-1; docs/13 §4.4, §4.5; docs/09 V-DET-01.
"""

import argparse
import os
import re
import subprocess
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

EXIT_OK = 0
EXIT_FAIL = 1
EXIT_UNREADABLE = 2
C3_MARK = "[C3]"
TRAILING_TAGS = re.compile(r"(?:\s*\[[^\[\]]+\])+\s*$")
NO_COMMIT = re.compile(r"^0*$")


def has_c3_tag(text: str) -> bool:
    """Return whether ``text`` ends with a tag group that holds ``[C3]``."""
    match = TRAILING_TAGS.search(text.strip())
    return match is not None and C3_MARK in re.findall(r"\[[^\[\]]+\]", match.group(0))


@dataclass(frozen=True)
class C3Rule:
    """What the C3 rule found: whether the baseline changed, and whether a C3 mark covers it.

    ``where`` says where the mark was looked for; with ``changed`` None, the rule could not be
    checked and ``where`` says why.
    """

    changed: bool | None
    marked: bool
    where: str


class UnreadableError(Exception):
    """A file or the base ref could not be read."""


def first_difference(a: bytes, b: bytes) -> str:
    """Return the 1-based line where two texts first differ, and the two lines."""
    la, lb = a.decode("utf-8", "replace").split("\n"), b.decode("utf-8", "replace").split("\n")
    for i, (x, y) in enumerate(zip(la, lb, strict=False)):
        if x != y:
            return f"line {i + 1}: `{x}` vs `{y}`"
    return f"line {min(len(la), len(lb)) + 1}: one file ends first"


def git(*args: str) -> subprocess.CompletedProcess[str]:
    """Run git with ``args``."""
    try:
        return subprocess.run(["git", *args], capture_output=True, text=True, check=False)
    except FileNotFoundError as err:
        raise UnreadableError("git is not on PATH") from err


def changed_against(base: str, baseline: Path, after: str | None = None) -> bool:
    """Return whether ``baseline`` differs from its version at ``base`` (absent there counts).

    The comparison is with the working tree, or with commit ``after`` when it is given.
    """
    span = [base] if after is None else [base, after]
    done = git("diff", "--quiet", *span, "--", str(baseline))
    if done.returncode not in (0, 1):
        raise UnreadableError(f"git diff {' '.join(span)} failed: {done.stderr.strip()}")
    return done.returncode == 1


def subjects(before: str, after: str) -> list[str]:
    """Return the commit subjects of ``before..after``."""
    done = git("log", "--format=%s", f"{before}..{after}")
    if done.returncode != 0:
        raise UnreadableError(f"git log {before}..{after} failed: {done.stderr.strip()}")
    return done.stdout.splitlines()


def c3_rule(args: argparse.Namespace) -> C3Rule:
    """Apply the C3 rule for the event (module docs)."""
    if args.changed is not None:
        return C3Rule(args.changed == "yes", has_c3_tag(args.title), f"the title `{args.title}`")
    if args.event == "push":
        before = args.before.strip()
        if NO_COMMIT.match(before):
            return C3Rule(None, False, "no base: the push has no previous commit")
        after = args.after or "HEAD"
        if not changed_against(before, args.baseline, after):
            return C3Rule(False, False, f"{before[:12]}..{after[:12]}")
        marked = any(has_c3_tag(s) for s in subjects(before, after))
        return C3Rule(True, marked, f"the commit subjects of {before[:12]}..{after[:12]}")
    if not args.base_ref:
        if args.event == "pull_request":
            raise UnreadableError(
                "a pull_request run needs its base (--base-ref or $RH_BASELINE_BASE): the C3 "
                "rule fails closed (VER-SPR-008 MAJ-1)"
            )
        return C3Rule(None, False, "no base ref")
    changed = changed_against(args.base_ref, args.baseline)
    return C3Rule(changed, has_c3_tag(args.title), f"the title `{args.title}`")


def read(path: Path) -> bytes:
    """Return a file's bytes."""
    try:
        return path.read_bytes()
    except OSError as err:
        raise UnreadableError(f"{path}: {err}") from err


def check(baseline: Path, recomputed: Sequence[Path], rule: C3Rule) -> tuple[bool, str]:
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
    if rule.changed and not rule.marked:
        return False, (
            f"BASELINE CHANGED in a PR not marked {C3_MARK}: {baseline} changed, but no "
            f"trailing {C3_MARK} tag is in {rule.where}. Only a C3 change records a new history "
            "(SPR-008 ruling 6)"
        )
    if rule.changed is None:
        how = f"C3 rule not checked ({rule.where})"
    elif rule.changed:
        how = f"regenerated in a {C3_MARK} PR ({rule.where})"
    else:
        how = f"unchanged against the base ({rule.where})"
    return True, (
        f"BASELINE OK: {baseline} equals the recomputation ({len(runs)} run(s) agree); {how}"
    )


def main(argv: Sequence[str] | None = None) -> int:
    """Run the gate and return the process exit status."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0] if __doc__ else None)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--recomputed", type=Path, nargs="+", required=True)
    parser.add_argument(
        "--event",
        # workflow_dispatch: ci.yml passes the real event name; a manual run is a local one.
        choices=("pull_request", "push", "local", "workflow_dispatch"),
        default=os.environ.get("RH_EVENT") or "local",
    )
    parser.add_argument("--base-ref", default=os.environ.get("RH_BASELINE_BASE", ""))
    parser.add_argument("--before", default=os.environ.get("RH_PUSH_BEFORE", ""))
    parser.add_argument("--after", default=os.environ.get("RH_PUSH_AFTER", ""))
    parser.add_argument("--changed", choices=("yes", "no"))
    parser.add_argument("--title", default=os.environ.get("RH_PR_TITLE", ""))
    args = parser.parse_args(argv)
    try:
        ok, line = check(args.baseline, args.recomputed, c3_rule(args))
    except UnreadableError as err:
        print(f"check_baseline: {err}", file=sys.stderr)
        return EXIT_UNREADABLE
    print(line)
    return EXIT_OK if ok else EXIT_FAIL


if __name__ == "__main__":
    sys.exit(main())
