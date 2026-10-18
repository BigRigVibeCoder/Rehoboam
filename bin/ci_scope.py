"""CI scope: which parts of the PR tier, and which path-scoped gates, a change needs.

**Parts** (the director's ruling, 2026-10-17; BCK-002 A-63). A change whose every file is
documentation (under `docs/` or `CODEX/`, or a Markdown file anywhere) cannot move a golden ladder,
a baseline, a performance count, a stream byte or a C3 rule. The golden part (release builds,
golden runs, baselines, the perf gates) and the stream part (the stream, serve, tail, sift, replay,
rungs and twin gates) then skip their work and report green in seconds, to save runner minutes. The
core part always runs in full, whatever the scope: every lint, test and spec check that reads
Markdown is there. `--part` writes `run=true|false` to `$GITHUB_OUTPUT` when it is set.

**Path-scoped gates** (BCK-002 A-64). A few pure-Python spec references of the core part are slow
(population synthesis 25-31 s, fixture worlds 10-12 s) and read a small, known set of files.
`--gate KEY` says whether the gates of `bin/check_all.sh`'s `scope KEY` block must run: they run
when any changed file is one of KEY's declared inputs (`SCOPED_GATES[KEY]`, plus `COMMON_INPUTS`:
the chain, this rule, the workflow and the Python lock), and are skipped only when the changed
files are known and none of them is. check_all reports a skipped gate as SKIP with this reason.
Nearly every sprint edits the chain to add its own gates, so `bin/check_all.sh` counts as changed
for KEY only when KEY's own text in it changed (`scoped_text`: the `scope KEY` block and the
definitions of the functions it names, `run`, `part` and `scope`), compared with the base of the
pull request or the push; any doubt about that base counts it as changed.

`trace KEY -- SCRIPT [ARGS]` runs the Python script SCRIPT in this process under an audit hook
(`sys.addaudithook`) and fails when it read a repository file, or listed a repository directory,
that is not one of KEY's inputs. The scoped gates run through it, so every run of a gate checks
that its input list still covers what its code reads: a reference that starts reading a new file
must change a declared input to do so, which runs the gate, whose trace then names the new file.

Both rules fail closed: anything they cannot classify runs everything.

- `pull_request`: the files are `git diff --name-only <base>...HEAD` (`--base-ref`, default
  `$RH_BASELINE_BASE`). No base, or a git error, runs everything.
- `push`: the files are `git diff --name-only <before> <after>` (`--before`, `--after`, default
  `$RH_PUSH_BEFORE`, `$RH_PUSH_AFTER`). An empty or all-zero `before` (a new branch) runs
  everything.
- `workflow_dispatch` and anything else (`local` included): everything runs. A manual run is a
  request to run everything, and a local `bin/check_all.sh` never skips a gate.
- An empty file list runs everything.
- `--files PATH ...` (with `--gate`) gives the changed files directly, and `--script-base PATH`
  the base's `bin/check_all.sh` (else the chain counts as changed), for the planted controls.

Usage:  python -I bin/ci_scope.py --part core|golden|stream [--event EVENT] [--base-ref REF]
            [--before SHA] [--after SHA]
        python -I bin/ci_scope.py --gate KEY [--files PATH ... [--script-base PATH]]
            [--event EVENT] [--base-ref REF] [--before SHA] [--after SHA]
        python -I bin/ci_scope.py trace KEY -- SCRIPT [ARG ...]
Exit:   0 for --part and --gate (a scope question never fails the tier; the verdict is in the
        output); for trace, the script's own status, or 1 when it passed but read an undeclared
        input; 2 for a usage error.
Standard library only (GOV-003 §3).

Refs: the director's ruling of 2026-10-17 (docs/decisions-log.md); SPR-009 review 2b (the parts);
BCK-002 A-63 (the third part), A-64 (path-scoping).
"""

import argparse
import os
import re
import runpy
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PARTS = ("core", "golden", "stream")
DOC_PREFIXES = ("docs/", "CODEX/")
DOC_SUFFIXES = (".md",)
SHOWN = 5  # files named in a verdict
PATH_TYPES = (str, bytes, os.PathLike)
FUNCTION = re.compile(r"^([A-Za-z_]\w*)\(\) \{$")
WORD = re.compile(r"[A-Za-z_]\w*")
LISTING_EVENTS = ("os.listdir", "os.scandir")

# Every path-scoped gate also runs when one of these changes: the chain that runs it (its own
# text in it: `scoped_text`), this rule, the workflow, and the Python environment.
CHAIN = "bin/check_all.sh"
COMMON_INPUTS = (
    "bin/check_all.sh",
    "bin/ci_scope.py",
    ".github/workflows/ci.yml",
    "python/pyproject.toml",
    "python/uv.lock",
)
# The SPR-019 population-synthesis reference and what it loads (worldgen-lite, M2 F1, the RNG).
_SYNTHESIS = (
    "bin/gen_rng_golden.py",
    "specs/demography/brass_general_standard.toml",
    "specs/demography/infant_sub_schedule.toml",
    "specs/demography/selftest_m2_f1.py",
    "specs/demography/selftest_m2_f1_mortality.py",
    "specs/demography/selftest_m2_rung0.py",
    "specs/systems/next/M2.demography.odd.toml",
    "specs/worldgen/selftest_population_synthesis.py",
    "specs/worldgen/selftest_worldgen_lite.py",
    "specs/worldgen/templates_v0.toml",
)
# KEY -> the repository files its gates read (found by `trace`; a path ending in / is a directory).
SCOPED_GATES: dict[str, tuple[str, ...]] = {
    "population-synthesis": (
        *_SYNTHESIS,
        "specs/registry/effect_language_golden_v0.toml",
        "specs/registry/effect_ref.py",
        "specs/worldgen/golden/population_synthesis_v0.txt",
        "specs/worldgen/population_synthesis.toml",
    ),
    "fixture-worlds": (
        *_SYNTHESIS,
        "specs/worldgen/golden/fixture_worlds_v0.txt",
        "specs/worldgen/selftest_fixture_worlds.py",
    ),
}


def is_doc(path: str) -> bool:
    """A file that cannot change what the golden and stream parts check."""
    return path.startswith(DOC_PREFIXES) or path.endswith(DOC_SUFFIXES)


def docs_only(paths: Sequence[str]) -> bool:
    """Every path is documentation, and there is at least one."""
    files = [p for p in paths if p.strip()]
    return bool(files) and all(is_doc(p) for p in files)


def is_input(path: str, key: str) -> bool:
    """Whether ``path`` is one of the declared inputs of the scoped gate ``key``."""
    for entry in (*COMMON_INPUTS, *SCOPED_GATES[key]):
        if path == entry or (entry.endswith("/") and path.startswith(entry)):
            return True
    return False


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


def _shown(paths: Sequence[str]) -> str:
    return ", ".join(paths[:SHOWN]) + (" …" if len(paths) > SHOWN else "")


def decide(part: str, files: list[str] | None) -> tuple[bool, str]:
    """Whether the part runs, and why."""
    if part == "core":
        return True, f"part {part}: always runs in full"
    if files is None:
        return True, f"part {part}: the scope is unknown, so it runs in full (fail closed)"
    if not any(p.strip() for p in files):
        return True, f"part {part}: no changed file found, so it runs in full (fail closed)"
    if docs_only(files):
        return False, f"part {part}: skipped, {len(files)} changed file(s), all documentation"
    others = [p for p in files if p.strip() and not is_doc(p)]
    return True, f"part {part}: runs in full ({len(others)} other file(s): {_shown(others)})"


def scoped_text(script: str, key: str) -> str:
    """KEY's own text in check_all.sh: its `scope KEY` block and the functions it relies on.

    The block runs from `scope KEY &&` to the next `scope` or `part` line; the functions are `run`,
    `part`, `scope` and every function whose name is a word of the block. Empty without a block.
    """
    lines = script.splitlines()
    marker = f"scope {key} &&"
    if marker not in lines:
        return ""
    block = [marker]
    for line in lines[lines.index(marker) + 1 :]:
        if line.startswith(("scope ", "part ")):
            break
        block.append(line)
    names = {"run", "part", "scope", *WORD.findall("\n".join(block))}
    defined = []
    for at, line in enumerate(lines):
        if (m := FUNCTION.match(line)) and m.group(1) in names and "}" in lines[at:]:
            defined += lines[at : lines.index("}", at) + 1]
    return "\n".join(block + defined)


def decide_gate(key: str, files: list[str] | None, *, chain_moved: bool = True) -> tuple[bool, str]:
    """Whether the path-scoped gates of ``key`` run, and why.

    ``chain_moved`` False says that a changed bin/check_all.sh left KEY's own text unchanged.
    """
    if key not in SCOPED_GATES:
        return True, f"gate {key}: not a path-scoped gate, so it runs (fail closed)"
    if files is None:
        return True, f"gate {key}: the scope is unknown, so it runs (fail closed)"
    changed = [p.strip() for p in files if p.strip()]
    if not changed:
        return True, f"gate {key}: no changed file found, so it runs (fail closed)"
    hits = [p for p in changed if is_input(p, key) and (p != CHAIN or chain_moved)]
    if hits:
        return True, f"gate {key}: runs ({len(hits)} of its inputs changed: {_shown(hits)})"
    inputs = len(COMMON_INPUTS) + len(SCOPED_GATES[key])
    chain = f" ({CHAIN} changed, but not the gate's own text in it)" if CHAIN in changed else ""
    return False, (
        f"gate {key}: skipped, {len(changed)} changed file(s), none of its {inputs} declared "
        f"inputs (bin/ci_scope.py SCOPED_GATES){chain}"
    )


def _git_text(rev: str, path: str) -> str | None:
    """``path`` at ``rev``, or None when git cannot give it."""
    try:
        done = subprocess.run(
            ["git", "show", f"{rev}:{path}"], check=True, capture_output=True, text=True
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return done.stdout


def base_chain(event: str, base_ref: str, before: str) -> str | None:
    """bin/check_all.sh at the base of this change, or None when it cannot be known."""
    if event == "push" and before and set(before) != {"0"}:
        return _git_text(before, CHAIN)
    if event != "pull_request" or not base_ref:
        return None
    try:
        merge_base = subprocess.run(
            ["git", "merge-base", base_ref, "HEAD"], check=True, capture_output=True, text=True
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return None
    return _git_text(merge_base, CHAIN) if merge_base else None


def chain_moved(key: str, base: str | None) -> bool:
    """Whether KEY's own text in bin/check_all.sh differs from ``base`` (None: assume it does)."""
    if base is None:
        return True
    head = (ROOT / CHAIN).read_text(encoding="utf-8")
    return scoped_text(base, key) != scoped_text(head, key)


def _repo_path(raw: object) -> str | None:
    """The repository-relative path an audited path names, or None when it is not one to check.

    Outside the repository, under target/ or .git/ (planted copies, build output) is not checked;
    a compiled module under __pycache__ stands for its source, and other __pycache__ files (the
    interpreter's temporary writes) are skipped.
    """
    if not isinstance(raw, PATH_TYPES):
        return None
    path = Path(os.fsdecode(raw)).resolve()
    try:
        rel = path.relative_to(ROOT)
    except ValueError:
        return None
    if not rel.parts or rel.parts[0] in {"target", ".git"}:
        return None
    if "__pycache__" in rel.parts:
        if path.suffix != ".pyc":
            return None
        return (rel.parent.parent / f"{path.name.split('.', 1)[0]}.py").as_posix()
    return rel.as_posix()


def _run_script(script: str, args: Sequence[str]) -> int:
    """Run ``script`` as __main__ in this process; its exit status."""
    sys.argv = [script, *args]
    try:
        runpy.run_path(script, run_name="__main__")
    except SystemExit as done:
        if done.code is None or isinstance(done.code, int):
            return done.code or 0
        print(done.code, file=sys.stderr)
        return 1
    return 0


def trace(key: str, script: str, args: Sequence[str]) -> int:
    """Run ``script`` and check that every repository file it read is an input of ``key``."""
    if key not in SCOPED_GATES:
        print(f"ci_scope trace: {key!r} is not a path-scoped gate", file=sys.stderr)
        return 2
    read: set[str] = set()
    listed: set[str] = set()

    def hook(event: str, hook_args: tuple[object, ...]) -> None:
        if event == "open" and hook_args and (rel := _repo_path(hook_args[0])):
            read.add(rel)
        elif event in LISTING_EVENTS and hook_args and isinstance(hook_args[0], PATH_TYPES):
            listed.add(str(Path(os.fsdecode(hook_args[0])).resolve()))

    sys.addaudithook(hook)
    status = _run_script(script, args)
    sys.stdout.flush()
    # A directory on sys.path is listed by the import system, not by the script.
    on_path = {str(Path(p).resolve()) for p in sys.path if p}
    dirs = {rel + "/" for d in listed - on_path if (rel := _repo_path(d))}
    undeclared = sorted(p for p in read | dirs if not is_input(p, key))
    for path in undeclared:
        print(
            f"SCOPE INCOMPLETE: gate {key} read {path}, which is not among its declared inputs "
            "(bin/ci_scope.py SCOPED_GATES): add it, or the gate would be skipped when it changes"
        )
    if not undeclared:
        print(f"SCOPE TRACE: gate {key}: {len(read)} repository file(s) read, all declared inputs")
    return status if status or not undeclared else 1


def _trace_main(argv: Sequence[str]) -> int:
    """`trace KEY -- SCRIPT [ARG ...]`."""
    if len(argv) < len(("trace", "KEY", "--", "SCRIPT")) or argv[2] != "--":
        print("usage: ci_scope.py trace KEY -- SCRIPT [ARG ...]", file=sys.stderr)
        return 2
    return trace(argv[1], argv[3], argv[4:])


def main(argv: Sequence[str] | None = None) -> int:
    """Print the verdict, and write `run=` to $GITHUB_OUTPUT for a part when it is set."""
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv[:1] == ["trace"]:
        return _trace_main(argv)
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    which = ap.add_mutually_exclusive_group(required=True)
    which.add_argument("--part", choices=PARTS)
    which.add_argument("--gate", metavar="KEY", help=f"one of {', '.join(sorted(SCOPED_GATES))}")
    ap.add_argument("--files", nargs="+", help="the changed files (with --gate; for controls)")
    ap.add_argument("--script-base", type=Path, help="the base's check_all.sh (with --files)")
    ap.add_argument("--event", default=os.environ.get("RH_EVENT", "local"))
    ap.add_argument("--base-ref", default=os.environ.get("RH_BASELINE_BASE", ""))
    ap.add_argument("--before", default=os.environ.get("RH_PUSH_BEFORE", ""))
    ap.add_argument("--after", default=os.environ.get("RH_PUSH_AFTER", ""))
    args = ap.parse_args(argv)
    if (args.files is not None and args.part) or (args.script_base and args.files is None):
        ap.error("--files goes with --gate, and --script-base with --files")
    files = args.files
    if files is None:
        files = changed_files(args.event, args.base_ref, args.before, args.after)
    if args.gate:
        moved = True
        if files and CHAIN in files:
            if args.files is not None:
                base = args.script_base.read_text(encoding="utf-8") if args.script_base else None
            else:
                base = base_chain(args.event, args.base_ref, args.before)
            moved = chain_moved(args.gate, base)
        print(f"CI SCOPE: {decide_gate(args.gate, files, chain_moved=moved)[1]}")
        return 0
    run, why = decide(args.part, files)
    print(f"CI SCOPE: {why}")
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with Path(output).open("a", encoding="utf-8") as fh:
            fh.write(f"run={'true' if run else 'false'}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
