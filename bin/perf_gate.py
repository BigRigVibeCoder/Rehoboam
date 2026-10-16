"""The performance gate of the PR tier: instruction counts against a committed baseline (SPR-009).

V-PERF-04 (`docs/09`): more than 10% more instructions than the committed baseline fails the
build; a speed-up never re-baselines automatically (`docs/08` §9). Instruction counts come from
callgrind through iai-callgrind (valgrind as an outside tool), which makes them deterministic: two
runs of one build on one toolchain give the same counts, on a hosted runner or a laptop (T-02a).

Subcommands:

    measure    Run the instruction-count benches (`cargo bench -p rh-bench`) and write their counts
               in the baseline format (below) to --out. Every run starts from an empty
               iai-callgrind home, so no earlier run is compared or kept.
    check      Compare measured counts with a baseline: FAIL on any bench more than 10% above its
               baseline, on a bench missing from either side, or (exit 3) when the toolchain
               differs, which reports "toolchain changed: re-baseline required" instead of a false
               pass or fail (ruling 2). It never writes the baseline.
    gate       measure, then check.
    same       Two measured files must be identical (T-02 acceptance: two runs on one commit).
    integrity  Ruling 3: a baseline is regenerated, never typed. If the committed baseline differs
               from the base revision's (or is new), it must equal the tool's output: toolchain
               lines exactly, each count within max(64, 1e-5 relative) (R-8), every difference
               printed,
               and the pull request's title must carry [perf-baseline] and its body a line
               `perf-baseline-reason: <why>`. The title and body come from RH_PR_TITLE and
               RH_PR_BODY; outside a pull request (no RH_PR_TITLE, GITHUB_EVENT_NAME not
               pull_request) they cannot be checked and the report says so: CI checks them.
    record     Measure twice, require identical counts, and write the baseline. The only way a
               baseline file is written.

Baseline format `rehoboam/rh-bench/instructions/v1`, one record per line, `#` lines are comments:

    format rehoboam/rh-bench/instructions/v1
    toolchain <name> <version text>     rustc, llvm, host, valgrind, libc, iai-callgrind, bmi2
    slot <bench> <note>                 a named, empty slot of docs/08 §9 (bench 4, ruling 1)
    bench <file>/<function>/<case> <Ir> one per measured case, in byte order of keys

Usage:  python -I bin/perf_gate.py gate --baseline crates/rh-bench/baselines/instructions_v1.txt \
            --out artifacts/perf/instructions.txt
Exit:   0 pass, 1 fail, 2 usage or a tool failed, 3 toolchain changed: re-baseline required.
Standard library only (GOV-003 §3).

REF: docs/08_performance_and_capacity.md §9; docs/09 V-PERF-04; docs/13 §4.3, §4.4;
CODEX/05_PROJECT/SPR-009_Benchmarks.md T-01, T-02, rulings 1-3.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass, field
from pathlib import Path

FORMAT = "rehoboam/rh-bench/instructions/v1"
ROOT = Path(__file__).resolve().parents[1]
BASELINE = Path("crates/rh-bench/baselines/instructions_v1.txt")
PACKAGE = "rh-bench"

# REF: docs/08 §9 regression policy, V-PERF-04: "a slowdown of more than 10% fails the build".
LIMIT_PERCENT = 10

# The benches of docs/08 §9 that P0 owns, by number. Bench 4 is a named, empty slot by the
# director's ruling at G0 (SPR-009 ruling 1): stencil fields do not exist before P1a.
BENCH_TARGETS = ("keyframe_write_read", "plane_hashes")
SLOTS = (
    (
        "raster_8x512",
        "bench 4: no baseline until P1a, the first sprint with a spatial field (M1 or M3)",
    ),
)

# R-8 (Architect, 2026-10-16), amending ruling 3: a committed baseline equals the tool's output
# within max(64 instructions, 1e-5 of the tool's count). Within one checkout two runs are identical;
# across checkouts the executable's path still moves the stack, by up to 48 instructions in 66
# million on the arena (SPR-009, measured). Far below anything that could loosen a 10% gate.
BAND_INSTRUCTIONS = 64
BAND_RELATIVE_DIVISOR = 100_000

PR_TAG = "[perf-baseline]"
# GAP-2: on a push, the commit that changes a baseline carries this line in its body.
PUSH_REASON_KEY = "Reason:"
NO_COMMIT = re.compile(r"^0*$")
REASON_KEY = "perf-baseline-reason:"

OK, FAIL, USAGE, REBASELINE = 0, 1, 2, 3


@dataclass
class Counts:
    """A baseline or measured file: its toolchain, slots and per-case counts."""

    toolchain: dict[str, str] = field(default_factory=dict)
    slots: dict[str, str] = field(default_factory=dict)
    benches: dict[str, int] = field(default_factory=dict)

    def render(self) -> str:
        """The canonical text: every value in a fixed order, so equal counts are equal bytes."""
        lines = [
            f"# {FORMAT}: callgrind instruction counts (Ir) of the PR-tier benches (SPR-009).",
            "# Written by `python -I bin/perf_gate.py record`, never by hand (SPR-009 ruling 3).",
            f"format {FORMAT}",
        ]
        lines += [f"toolchain {k} {v}" for k, v in self.toolchain.items()]
        lines += [f"slot {k} {v}" for k, v in sorted(self.slots.items())]
        lines += [f"bench {k} {v}" for k, v in sorted(self.benches.items())]
        return "\n".join(lines) + "\n"


class GateError(Exception):
    """A tool failed or a file is not in the baseline format (exit 2)."""


def parse(text: str, origin: str) -> Counts:
    """Parses the baseline format; anything else is a GateError naming the line."""
    counts = Counts()
    seen_format = False
    for number, raw in enumerate(text.splitlines(), 1):
        if not raw or raw.startswith("#"):
            continue
        kind, _, rest = raw.partition(" ")
        key, _, value = rest.partition(" ")
        if kind == "format" and rest == FORMAT:
            seen_format = True
        elif kind == "toolchain" and value:
            counts.toolchain[key] = value
        elif kind == "slot" and value:
            counts.slots[key] = value
        elif kind == "bench" and value.isdigit():
            counts.benches[key] = int(value)
        else:
            raise GateError(f"{origin}:{number}: not a {FORMAT} line: {raw!r}")
    if not seen_format:
        raise GateError(f"{origin}: no `format {FORMAT}` line")
    return counts


def read_counts(path: Path) -> Counts:
    """Reads and parses a baseline or measured file."""
    try:
        return parse(path.read_text(encoding="utf-8"), str(path))
    except OSError as e:
        raise GateError(f"cannot read {path}: {e}") from e


def _first_lines(cmd: Sequence[str]) -> list[str]:
    """The output lines of a version command, run in the root (rustup reads rust-toolchain.toml)."""
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=ROOT).stdout
    except (OSError, subprocess.CalledProcessError) as e:
        raise GateError(f"`{' '.join(cmd)}` failed: {e}") from e
    return out.splitlines()


def _first_line(cmd: Sequence[str]) -> str:
    lines = _first_lines(cmd)
    return lines[0].strip() if lines else ""


def locked_version(package: str) -> str:
    """A package's version in Cargo.lock.

    iai-callgrind's runner refuses to run a library of another version, so the library's locked
    version is the runner's too.
    """
    try:
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as e:
        raise GateError(f"cannot read Cargo.lock: {e}") from e
    versions = [str(p["version"]) for p in lock.get("package", []) if p.get("name") == package]
    if len(versions) != 1:
        raise GateError(f"Cargo.lock holds {len(versions)} versions of {package}, not one")
    return versions[0]


def cpu_flag(flag: str) -> str:
    """Whether the host CPU reports `flag` in /proc/cpuinfo.

    R-9: zstd picks its BMI2 code path at run time, and valgrind passes the host's BMI2 through,
    so a host without it counts other instructions.
    """
    try:
        text = Path("/proc/cpuinfo").read_text(encoding="utf-8")
    except OSError:
        return "unknown"
    flags = next((line for line in text.splitlines() if line.startswith("flags")), "")
    return "yes" if flag in flags.split() else "no"


def toolchain() -> dict[str, str]:
    """The versions an instruction count depends on besides the code (ruling 2), in fixed order."""
    rustc = dict(line.split(": ", 1) for line in _first_lines(["rustc", "-vV"]) if ": " in line)
    return {
        "rustc": f"{rustc.get('release', '?')} {rustc.get('commit-hash', '?')}",
        "llvm": rustc.get("LLVM version", "?"),
        "host": rustc.get("host", "?"),
        "valgrind": _first_line(["valgrind", "--version"]),
        "libc": os.confstr("CS_GNU_LIBC_VERSION") or "?",
        "iai-callgrind": locked_version("iai-callgrind"),
        "bmi2": cpu_flag("bmi2"),
    }


def instructions(record: dict[str, object]) -> tuple[str, int]:
    """The key and Ir of one iai-callgrind JSON summary line (a fresh home: only `Left`)."""
    try:
        exe = Path(str(record["benchmark_file"])).stem
        key = f"{exe}/{record['function_name']}/{record['id']}"
        profiles = record["profiles"]
        if not isinstance(profiles, list):
            raise TypeError("profiles is not a list")
        ir = profiles[0]["summaries"]["total"]["summary"]["Callgrind"]["Ir"]["metrics"]["Left"]
        return key, int(ir["Int"])
    except (KeyError, IndexError, TypeError, ValueError) as e:
        raise GateError(f"unexpected iai-callgrind summary: {e!r}") from e


def measure(features: Sequence[str]) -> Counts:
    """Runs the benches once from an empty iai-callgrind home and returns their counts."""
    cmd = ["cargo", "bench", "--locked", "--quiet", "-p", PACKAGE]
    for target in BENCH_TARGETS:
        cmd += ["--bench", target]
    # The instruction-count targets run on the deterministic arena (crates/rh-bench, arena-alloc).
    cmd += ["--features", ",".join(["arena-alloc", *features])]
    cmd += ["--", "--output-format=json"]
    with tempfile.TemporaryDirectory(prefix="rh-iai-") as home:
        env = dict(os.environ, IAI_CALLGRIND_HOME=home)
        run = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, env=env, check=False)
    if run.returncode != 0:
        tail = "\n".join(run.stderr.splitlines()[-30:])
        raise GateError(f"`{' '.join(cmd)}` failed ({run.returncode}):\n{tail}")
    counts = Counts(toolchain=toolchain(), slots=dict(SLOTS))
    for line in run.stdout.splitlines():
        if line.strip():
            key, ir = instructions(json.loads(line))
            counts.benches[key] = ir
    if not counts.benches:
        raise GateError(f"`{' '.join(cmd)}` reported no benchmark")
    return counts


def write(path: Path, counts: Counts) -> None:
    """Writes counts in the canonical format, creating the directory."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(counts.render(), encoding="utf-8")


def toolchain_diff(baseline: Counts, measured: Counts) -> list[str]:
    """Every toolchain entry that differs, as `name: baseline -> measured`."""
    names = sorted(set(baseline.toolchain) | set(measured.toolchain))
    return [
        f"{n}: {baseline.toolchain.get(n, '(none)')} -> {measured.toolchain.get(n, '(none)')}"
        for n in names
        if baseline.toolchain.get(n) != measured.toolchain.get(n)
    ]


def percent(base: int, now: int) -> str:
    """The change from base to now, signed, two decimals."""
    return f"{(now - base) * 100 / base:+.2f}%" if base else "n/a"


def compare(baseline: Counts, measured: Counts) -> tuple[int, list[str]]:
    """The verdict of measured against baseline, and one report line per case."""
    changed = toolchain_diff(baseline, measured)
    if changed:
        lines = ["toolchain changed: re-baseline required (SPR-009 ruling 2)"]
        return REBASELINE, lines + [f"  {c}" for c in changed]
    verdict, lines = OK, []
    for key in sorted(set(baseline.benches) | set(measured.benches)):
        base, now = baseline.benches.get(key), measured.benches.get(key)
        if base is None or now is None:
            side = "baseline" if base is None else "this run"
            lines.append(f"MISSING {key}: not in {side}; a bench set change is a re-baseline")
            verdict = FAIL
        elif now * 100 > base * (100 + LIMIT_PERCENT):
            lines.append(
                f"REGRESSION {key}: {base} -> {now} ({percent(base, now)} > +{LIMIT_PERCENT}%)"
            )
            verdict = FAIL
        else:
            word = "faster (baseline kept)" if now < base else "ok"
            lines.append(f"{word} {key}: {base} -> {now} ({percent(base, now)})")
    if baseline.slots != measured.slots:
        lines.append(f"SLOTS differ: {baseline.slots} -> {measured.slots}")
        verdict = FAIL
    return verdict, lines


def cmd_check(baseline: Path, measured: Path) -> int:
    """Prints the comparison and returns its verdict."""
    verdict, lines = compare(read_counts(baseline), read_counts(measured))
    print("\n".join(lines))
    word = {OK: "PASS", FAIL: "FAIL", REBASELINE: "RE-BASELINE REQUIRED"}[verdict]
    print(f"V-PERF-04 {word}: {len(read_counts(measured).benches)} cases against {baseline}")
    return verdict


def cmd_same(first: Path, second: Path) -> int:
    """Two runs on one commit must give identical counts (T-02 acceptance)."""
    a, b = read_counts(first), read_counts(second)
    diffs = [
        f"{k}: {a.benches.get(k)} != {b.benches.get(k)}"
        for k in sorted(set(a.benches) | set(b.benches))
        if a.benches.get(k) != b.benches.get(k)
    ]
    if diffs or a.toolchain != b.toolchain:
        print("NOT REPRODUCIBLE: two runs on one commit differ\n" + "\n".join(diffs))
        return FAIL
    print(f"identical counts in two runs: {len(a.benches)} cases")
    return OK


def git(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    """Runs git in `repo` (the repository root, or a control's scratch repository)."""
    return subprocess.run(["git", *args], capture_output=True, text=True, cwd=repo, check=False)


def base_text(base_ref: str, path: Path, repo: Path = ROOT) -> str | None:
    """The baseline at the merge base of HEAD and base_ref, or None if it did not exist there."""
    merge_base = git(repo, "merge-base", "HEAD", base_ref)
    if merge_base.returncode != 0:
        raise GateError(f"no merge base of HEAD and {base_ref}: {merge_base.stderr.strip()}")
    shown = git(repo, "show", f"{merge_base.stdout.strip()}:{path.as_posix()}")
    return shown.stdout if shown.returncode == 0 else None


@dataclass(frozen=True)
class Context:
    """Where the change comes from (GAP-2: never fail open, as SPR-008's MAJ-1).

    `pull_request` carries the title and body; `push` the range before..after; `local` (a
    developer's run, `workflow_dispatch`) carries nothing, and the tag and reason go unchecked,
    said so.
    """

    event: str
    title: str = ""
    body: str = ""
    before: str = ""
    after: str = ""


def change_context() -> Context:
    """The context from the environment CI sets (RH_EVENT, RH_PR_TITLE, RH_PR_BODY, RH_PUSH_*).

    An explicit RH_PR_TITLE is a pull request (the controls set one). A pull_request event without a
    title, or a push without its range, is an error, never "no context".
    """
    event = os.environ.get("RH_EVENT") or os.environ.get("GITHUB_EVENT_NAME") or "local"
    # A workflow passes an empty title outside pull_request events: an empty title is no title.
    title = os.environ.get("RH_PR_TITLE") or None
    if title is not None:
        return Context("pull_request", title=title, body=os.environ.get("RH_PR_BODY", ""))
    if event == "pull_request":
        raise GateError("a pull_request run without RH_PR_TITLE: the workflow must pass it")
    if event == "push":
        before, after = os.environ.get("RH_PUSH_BEFORE", ""), os.environ.get("RH_PUSH_AFTER", "")
        if not before or not after:
            raise GateError(
                "a push run without RH_PUSH_BEFORE and RH_PUSH_AFTER: the workflow must pass them"
            )
        return Context("push", before=before, after=after)
    return Context("local")


def pr_context() -> tuple[str, str] | None:
    """The pull request's title and body, or None outside a pull request."""
    c = change_context()
    return (c.title, c.body) if c.event == "pull_request" else None


def has_reason(body: str, key: str = REASON_KEY) -> bool:
    """Whether the body has a `<key>` line (`perf-baseline-reason:`) with text after the key."""
    return any(
        line.strip().lower().startswith(key.lower()) and line.strip()[len(key) :].strip()
        for line in body.splitlines()
    )


def has_tag(subject: str) -> bool:
    """Whether a commit subject's trailing tag group holds [perf-baseline] (as SPR-008's [C3])."""
    return re.search(r"\[perf-baseline\](?:\s*\[[^\[\]]+\])*\s*$", subject) is not None


def push_problems(commits: Sequence[tuple[str, str]]) -> list[str]:
    """GAP-2: a push that changes a baseline needs one commit tagged and with a reason.

    The commit's subject ends in [perf-baseline] and its body has a `Reason:` line.
    """
    if any(has_tag(subject) and has_reason(body, PUSH_REASON_KEY) for subject, body in commits):
        return []
    return [
        f"a baseline changed in this push, but no commit in the range has a subject ending in "
        f"{PR_TAG} with a `{PUSH_REASON_KEY} <why>` line in its body"
    ]


def push_commits(repo: Path, before: str, after: str) -> list[tuple[str, str]]:
    """(subject, body) of every commit in before..after."""
    done = git(repo, "log", "--format=%s%x1f%b%x1e", f"{before}..{after}")
    if done.returncode != 0:
        raise GateError(f"git log {before}..{after} failed: {done.stderr.strip()}")
    commits = []
    for record in done.stdout.split("\x1e"):
        if record.strip():
            subject, _, body = record.strip("\n").partition("\x1f")
            commits.append((subject, body))
    return commits


def within_band(tool: int, committed: int) -> bool:
    """R-8: |committed - tool| <= max(BAND_INSTRUCTIONS, tool / BAND_RELATIVE_DIVISOR)."""
    diff = abs(committed - tool)
    return diff <= BAND_INSTRUCTIONS or diff * BAND_RELATIVE_DIVISOR <= tool


def band_problems(committed: Counts, measured: Counts) -> list[str]:
    """Ruling 3 as amended by R-8: the baseline equals the tool's output within the band.

    Toolchain and slot lines must be equal exactly, and so must the set of cases. Every count that
    differs is printed, inside the band or not.
    """
    problems = [
        f"toolchain {n}: the baseline says {committed.toolchain.get(n)!r}, the tool "
        f"{measured.toolchain.get(n)!r}; toolchain lines are exact"
        for n in sorted(set(committed.toolchain) | set(measured.toolchain))
        if committed.toolchain.get(n) != measured.toolchain.get(n)
    ]
    if committed.slots != measured.slots or set(committed.benches) != set(measured.benches):
        problems.append("the baseline's slots or cases differ from the tool's output")
    for key in sorted(set(committed.benches) & set(measured.benches)):
        base, now = committed.benches[key], measured.benches[key]
        if base == now:
            continue
        inside = within_band(now, base)
        print(
            f"{'within band' if inside else 'OUTSIDE BAND'} {key}: baseline {base}, tool {now} "
            f"({base - now:+d})"
        )
        if not inside:
            problems.append(
                f"{key}: the baseline's {base} is {base - now:+d} from the tool's {now}, outside "
                f"max({BAND_INSTRUCTIONS} instructions, 1/{BAND_RELATIVE_DIVISOR}); a baseline is "
                "regenerated with `python -I bin/perf_gate.py record`, never typed (SPR-009 "
                "ruling 3, R-8)"
            )
    return problems


def tag_problems(context: Context, repo: Path) -> list[str]:
    """Why the change's tag or reason is missing; local runs say they cannot check."""
    if context.event == "pull_request":
        problems = []
        if PR_TAG not in context.title:
            problems.append(
                f"a changed baseline needs {PR_TAG} in the pull request's title: {context.title!r}"
            )
        if not has_reason(context.body):
            problems.append(f"a changed baseline needs a `{REASON_KEY} <why>` line in the PR body")
        return problems
    if context.event == "push":
        return push_problems(push_commits(repo, context.before, context.after))
    return []


def integrity_problems(
    committed: str, measured: str, pr: tuple[str, str] | None, exact: bool = False
) -> list[str]:
    """Why a changed baseline is not acceptable (ruling 3); empty if it is.

    Instruction counts compare within the R-8 band; allocation counts (exact) byte for byte.
    """
    if exact:
        problems = (
            []
            if committed == measured
            else [
                "the changed allocation baseline does not equal the tool's output: allocation "
                "counts are exact (R-8); regenerate it with `python -I bin/perf_gate.py "
                "alloc-record`, never type it"
            ]
        )
    else:
        problems = band_problems(
            parse(committed, "the baseline"), parse(measured, "the tool's output")
        )
    if pr is not None:
        problems += tag_problems(Context("pull_request", title=pr[0], body=pr[1]), ROOT)
    return problems


def base_for(context: Context, baseline: Path, repo: Path, base_ref: str | None) -> str | None:
    """The baseline before the change: at the push's `before`, else at the merge base."""
    path = baseline.resolve().relative_to(repo.resolve())
    if context.event == "push":
        shown = git(repo, "show", f"{context.before}:{path.as_posix()}")
        return shown.stdout if shown.returncode == 0 else None
    ref = base_ref or os.environ.get("RH_BASELINE_BASE") or "origin/main"
    return base_text(ref, path, repo)


@dataclass(frozen=True)
class Base:
    """Where the baseline's previous version comes from: a ref, or a file (the controls)."""

    ref: str | None = None
    file: Path | None = None


def cmd_integrity(
    baseline: Path, measured: Path, base: Base, *, exact: bool = False, repo: Path = ROOT
) -> int:
    """Ruling 3 for one baseline file (exact for allocation baselines)."""
    context = change_context()
    if context.event == "push" and NO_COMMIT.match(context.before):
        print(f"{baseline}: not checked: the push has no base (a new branch)")
        return OK
    committed = baseline.read_text(encoding="utf-8")
    if base.file is not None:
        before: str | None = base.file.read_text(encoding="utf-8")
    else:
        before = base_for(context, baseline, repo, base.ref)
    if before == committed:
        print(f"{baseline}: unchanged from the base; the regression check governs it")
        return OK
    state = "new" if before is None else "changed from the base"
    problems = integrity_problems(committed, measured.read_text(encoding="utf-8"), None, exact)
    problems += tag_problems(context, repo)
    for p in problems:
        print(f"INTEGRITY {baseline} ({state}): {p}")
    if problems:
        return FAIL
    if context.event == "local":
        print(
            f"{baseline} ({state}): equals the tool's output; tag and reason not checked here "
            "(no pull-request or push context): CI checks them"
        )
    else:
        print(f"{baseline} ({state}): equals the tool's output, with the {PR_TAG} tag and a reason")
    return OK


def cmd_record(baseline: Path, features: Sequence[str]) -> int:
    """Measures twice, requires equal counts, and writes the baseline."""
    first, second = measure(features), measure(features)
    if first.render() != second.render():
        print("NOT REPRODUCIBLE: two runs differ; nothing written")
        return FAIL
    write(baseline, first)
    print(f"wrote {baseline}: {len(first.benches)} cases; commit it in a {PR_TAG} pull request")
    return OK


# V-PERF-11 (SPR-009 T-05): the smoke world's heap allocations per steady-state tick.
ALLOC_BASELINE = Path("crates/rh-bench/baselines/allocations_v1.txt")
ALLOC_FORMAT = "rehoboam/rh-bench/allocations/v1"
# Gated fields of a `budget` line: any increase fails (ruling 5). Shrinks are reported, not gated.
ALLOC_GATED = ("allocations", "growths", "max_per_tick")


def alloc_measure(control: bool) -> str:
    """Runs the alloc_budget target and returns its report (the baseline format)."""
    with tempfile.TemporaryDirectory(prefix="rh-alloc-") as tmp:
        out = Path(tmp) / "allocations.txt"
        cmd = ["cargo", "bench", "--locked", "--quiet", "-p", PACKAGE, "--features", "count-alloc"]
        cmd += ["--bench", "alloc_budget", "--", "--out", str(out)]
        env = dict(os.environ, RH_ALLOC_CONTROL="1" if control else "0")
        run = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, env=env, check=False)
        if run.returncode != 0 or not out.exists():
            tail = "\n".join((run.stdout + run.stderr).splitlines()[-20:])
            raise GateError(f"`{' '.join(cmd)}` failed ({run.returncode}):\n{tail}")
        return out.read_text(encoding="utf-8")


def parse_budget(text: str, origin: str) -> tuple[str, dict[str, dict[str, int]]]:
    """The window line and every `budget threads N field value ...` line of an allocation file."""
    window, budgets, seen = "", {}, False
    for raw in text.splitlines():
        if raw.startswith("format "):
            seen = raw == f"format {ALLOC_FORMAT}"
        elif raw.startswith("window "):
            window = raw
        elif raw.startswith("budget threads "):
            words = raw.split()
            try:
                budgets[words[2]] = {
                    k: int(v) for k, v in zip(words[3::2], words[4::2], strict=True)
                }
            except ValueError as e:
                raise GateError(f"{origin}: not a budget line: {raw!r}") from e
    if not seen or not window or not budgets:
        raise GateError(f"{origin}: not a {ALLOC_FORMAT} file")
    return window, budgets


def alloc_compare(baseline: str, measured: str) -> tuple[int, list[str]]:
    """V-PERF-11: any increase over the recorded budget fails; a decrease passes and is kept."""
    base_window, base = parse_budget(baseline, "the baseline")
    now_window, now = parse_budget(measured, "this run")
    if base_window != now_window or set(base) != set(now):
        return REBASELINE, [
            f"window changed: re-baseline required ({base_window!r} -> {now_window!r})"
        ]
    verdict, lines = OK, []
    for threads in sorted(base, key=int):
        if base[threads].get("ticks") != now[threads].get("ticks"):
            return REBASELINE, [f"window changed: the ticks at {threads} thread(s) differ"]
        for field_name in ALLOC_GATED:
            b, n = base[threads][field_name], now[threads][field_name]
            if n > b:
                lines.append(f"OVER BUDGET threads {threads} {field_name}: {b} -> {n} (+{n - b})")
                verdict = FAIL
            else:
                word = "below budget (kept)" if n < b else "ok"
                lines.append(f"{word} threads {threads} {field_name}: {b} -> {n}")
    return verdict, lines


def cmd_alloc_check(baseline: Path, measured: str) -> int:
    """Prints the allocation comparison and returns its verdict."""
    verdict, lines = alloc_compare(baseline.read_text(encoding="utf-8"), measured)
    print("\n".join(lines))
    word = {OK: "PASS", FAIL: "FAIL", REBASELINE: "RE-BASELINE REQUIRED"}[verdict]
    print(f"V-PERF-11 {word}: per-tick allocations against {baseline}")
    return verdict


def cmd_alloc_record(baseline: Path) -> int:
    """Measures twice, requires identical reports, and writes the allocation baseline."""
    first, second = alloc_measure(control=False), alloc_measure(control=False)
    if first != second:
        print("NOT REPRODUCIBLE: two allocation runs differ; nothing written")
        return FAIL
    baseline.parent.mkdir(parents=True, exist_ok=True)
    baseline.write_text(first, encoding="utf-8")
    print(f"wrote {baseline}; commit it in a {PR_TAG} change with a reason")
    return OK


def build_parser() -> argparse.ArgumentParser:
    """The command-line interface."""
    p = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    sub = p.add_subparsers(dest="command", required=True)
    for name in ("measure", "gate", "record"):
        s = sub.add_parser(name)
        s.add_argument("--features", action="append", default=[])
        if name != "record":
            s.add_argument("--out", type=Path, required=True)
        if name != "measure":
            s.add_argument("--baseline", type=Path, default=BASELINE)
    s = sub.add_parser("check")
    s.add_argument("--baseline", type=Path, default=BASELINE)
    s.add_argument("--measured", type=Path, required=True)
    s = sub.add_parser("same")
    s.add_argument("first", type=Path)
    s.add_argument("second", type=Path)
    s = sub.add_parser("integrity")
    s.add_argument("--baseline", type=Path, default=BASELINE)
    s.add_argument("--measured", type=Path, required=True)
    s.add_argument("--base-ref", default=None, help="default origin/main")
    s.add_argument("--base-file", type=Path, default=None, help="the base revision's baseline")
    s.add_argument("--repo", type=Path, default=ROOT, help="the git repository (controls)")
    s.add_argument("--exact", action="store_true", help="allocation baselines: byte for byte")
    s = sub.add_parser("alloc-measure")
    s.add_argument("--out", type=Path, required=True)
    s.add_argument("--control", action="store_true", help="V-PERF-11's allocating system")
    s = sub.add_parser("alloc-gate")
    s.add_argument("--out", type=Path, required=True)
    s.add_argument("--control", action="store_true", help="V-PERF-11's allocating system")
    s.add_argument("--baseline", type=Path, default=ALLOC_BASELINE)
    s = sub.add_parser("alloc-check")
    s.add_argument("--baseline", type=Path, default=ALLOC_BASELINE)
    s.add_argument("--measured", type=Path, required=True)
    s = sub.add_parser("alloc-record")
    s.add_argument("--baseline", type=Path, default=ALLOC_BASELINE)
    return p


def dispatch_alloc(a: argparse.Namespace) -> int:
    """Runs one allocation subcommand (V-PERF-11)."""
    if a.command == "alloc-record":
        return cmd_alloc_record(a.baseline)
    if a.command == "alloc-check":
        return cmd_alloc_check(a.baseline, a.measured.read_text(encoding="utf-8"))
    text = alloc_measure(a.control)
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(text, encoding="utf-8")
    if a.command == "alloc-measure":
        print(f"wrote {a.out}")
        return OK
    return cmd_alloc_check(a.baseline, text)


def cmd_measure(a: argparse.Namespace) -> int:
    """`measure` and `gate`: count, write, and (gate) compare with the baseline."""
    write(a.out, measure(a.features))
    if a.command == "gate":
        return cmd_check(a.baseline, a.out)
    print(f"wrote {a.out}")
    return OK


HANDLERS: dict[str, Callable[[argparse.Namespace], int]] = {
    "measure": cmd_measure,
    "gate": cmd_measure,
    "check": lambda a: cmd_check(a.baseline, a.measured),
    "same": lambda a: cmd_same(a.first, a.second),
    "integrity": lambda a: cmd_integrity(
        a.baseline, a.measured, Base(a.base_ref, a.base_file), exact=a.exact, repo=a.repo
    ),
    "record": lambda a: cmd_record(a.baseline, a.features),
}


def dispatch(a: argparse.Namespace) -> int:
    """Runs one subcommand."""
    if a.command.startswith("alloc-"):
        return dispatch_alloc(a)
    return HANDLERS[a.command](a)


def main(argv: Iterable[str] | None = None) -> int:
    """Entry point."""
    args = build_parser().parse_args(None if argv is None else list(argv))
    for tool in ("cargo", "valgrind", "iai-callgrind-runner"):
        if args.command in ("measure", "gate", "record") and shutil.which(tool) is None:
            print(f"perf_gate: {tool} not found (RUN-001 §3)", file=sys.stderr)
            return USAGE
    try:
        return dispatch(args)
    except GateError as e:
        print(f"perf_gate: {e}", file=sys.stderr)
        return USAGE


if __name__ == "__main__":
    sys.exit(main())
