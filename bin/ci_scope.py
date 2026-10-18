"""CI scope: does this change need the golden part of the PR tier? (director ruling, 2026-10-17).

A change whose every file is documentation (under `docs/` or `CODEX/`, or a Markdown file
anywhere) cannot move a golden ladder, a baseline, a performance count or a C3 rule. The golden
part (release builds, golden runs, the perf gates) then skips its work and reports green in
seconds, to save runner minutes. The core part always runs in full, whatever the scope: every
lint, test and spec check that reads Markdown is there.

The rule fails closed: anything it cannot classify runs the full part.

- `pull_request`: the files are `git diff --name-only <base>...HEAD` (`--base-ref`, default
  `$RH_BASELINE_BASE`). No base, or a git error, runs the full part.
- `push`: the files are `git diff --name-only <before> <after>` (`--before`, `--after`, default
  `$RH_PUSH_BEFORE`, `$RH_PUSH_AFTER`). An empty or all-zero `before` (a new branch) runs the full
  part.
- `workflow_dispatch` and anything else (`local` included): the full part. A manual run is a
  request to run everything.
- An empty file list runs the full part.

The verdict goes to stdout and, when `$GITHUB_OUTPUT` is set, as `run=true|false` to that file.

Usage:  python -I bin/ci_scope.py --part core|golden [--event EVENT] [--base-ref REF]
            [--before SHA] [--after SHA]
Exit:   0 always (a scope question never fails the tier; the verdict is in the output).
Standard library only (GOV-003 §3).

Refs: the director's ruling of 2026-10-17 (docs/decisions-log.md); SPR-009 review 2b (the parts).
"""

import argparse
import os
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path

DOC_PREFIXES = ("docs/", "CODEX/")
DOC_SUFFIXES = (".md",)
SHOWN = 5  # non-documentation files named in the verdict


def is_doc(path: str) -> bool:
    """A file that cannot change what the golden part checks."""
    return path.startswith(DOC_PREFIXES) or path.endswith(DOC_SUFFIXES)


def docs_only(paths: Sequence[str]) -> bool:
    """Every path is documentation, and there is at least one."""
    files = [p for p in paths if p.strip()]
    return bool(files) and all(is_doc(p) for p in files)


def changed_files(event: str, base_ref: str, before: str, after: str) -> list[str] | None:
    """The changed files for this event, or None when the scope cannot be known."""
    if event == "pull_request":
        if not base_ref:
            return None
        cmd = ["git", "diff", "--name-only", f"{base_ref}...HEAD"]
    elif event == "push":
        if not before or set(before) == {"0"}:
            return None
        cmd = ["git", "diff", "--name-only", before, after or "HEAD"]
    else:
        return None
    try:
        out = subprocess.run(cmd, check=True, capture_output=True, text=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return None
    return out.splitlines()


def decide(part: str, files: list[str] | None) -> tuple[bool, str]:
    """Whether the part runs, and why."""
    if part != "golden":
        return True, f"part {part}: always runs in full"
    if files is None:
        return True, "part golden: the scope is unknown, so it runs in full (fail closed)"
    if not any(p.strip() for p in files):
        return True, "part golden: no changed file found, so it runs in full (fail closed)"
    if docs_only(files):
        return False, f"part golden: skipped, {len(files)} changed file(s), all documentation"
    others = [p for p in files if p.strip() and not is_doc(p)]
    shown = ", ".join(others[:SHOWN]) + (" …" if len(others) > SHOWN else "")
    return True, f"part golden: runs in full ({len(others)} non-documentation file(s): {shown})"


def main(argv: Sequence[str] | None = None) -> int:
    """Print the verdict and write `run=` to $GITHUB_OUTPUT when set."""
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument("--part", required=True, choices=["core", "golden"])
    ap.add_argument("--event", default=os.environ.get("RH_EVENT", "local"))
    ap.add_argument("--base-ref", default=os.environ.get("RH_BASELINE_BASE", ""))
    ap.add_argument("--before", default=os.environ.get("RH_PUSH_BEFORE", ""))
    ap.add_argument("--after", default=os.environ.get("RH_PUSH_AFTER", ""))
    args = ap.parse_args(argv)
    files = changed_files(args.event, args.base_ref, args.before, args.after)
    run, why = decide(args.part, files)
    print(f"CI SCOPE: {why}")
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with Path(output).open("a", encoding="utf-8") as fh:
            fh.write(f"run={'true' if run else 'false'}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
